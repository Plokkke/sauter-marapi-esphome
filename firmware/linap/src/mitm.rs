use crate::frame::{Frame, header};
use crate::setpoint::Setpoint;
use crate::state::CommandState;

/// A Boost ordered by Home Assistant stops by itself after this long, unless it is ordered again.
pub const DEFAULT_BOOST_US: u64 = 30 * 60 * 1_000_000;

/// The state at start-up, until the saved state, the keys or Home Assistant give one: off, at 15 °C.
pub const DEFAULT_SETPOINT: Setpoint = match Setpoint::from_halves(30) {
    Some(setpoint) => setpoint,
    None => panic!("default setpoint out of range"),
};

/// The constant answer of the command board to the `70` poll (meaning unknown).
const PERIODIC: Frame = Frame::new(header::PERIODIC, [0x83, 0x80, 0x89]);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    On,
    Boost,
    Off,
}

/// The radiator's state, owned by the board (`knowledge/architecture.md`): the keys, heard on the command board
/// half, and Home Assistant change it, the last change wins; the power board receives it at each poll.
pub struct Mitm {
    mode: Mode,
    setpoint: Setpoint,
    /// The command board's last answer: only a change of it is an order from the keys.
    keys: Option<Frame>,
    /// While a Boost ordered by Home Assistant lasts: the mode to go back to, and when it started.
    boost: Option<(Mode, u64)>,
    boost_us: u64,
    /// Home Assistant or the keys changed the state since start-up: the saved state is then older.
    ordered: bool,
}

impl Default for Mitm {
    fn default() -> Self {
        Self::new()
    }
}

impl Mitm {
    pub const fn new() -> Self {
        Self {
            mode: Mode::Off,
            setpoint: DEFAULT_SETPOINT,
            keys: None,
            boost: None,
            boost_us: DEFAULT_BOOST_US,
            ordered: false,
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The setpoint of the On mode, kept while the radiator is off or in Boost.
    pub fn setpoint(&self) -> Setpoint {
        self.setpoint
    }

    pub fn state(&self) -> CommandState {
        match self.mode {
            Mode::On => CommandState::On(self.setpoint),
            Mode::Boost => CommandState::Boost,
            Mode::Off => CommandState::Off,
        }
    }

    /// An order from Home Assistant. Ordering a Boost again restarts its clock: that is how it is extended.
    pub fn set_mode(&mut self, mode: Mode, now_us: u64) {
        self.boost = match mode {
            Mode::Boost => Some((self.boost.map_or(self.mode, |(back, _)| back), now_us)),
            Mode::On | Mode::Off => None,
        };
        self.mode = mode;
        self.ordered = true;
    }

    /// An order from Home Assistant: this setpoint, on.
    pub fn set_setpoint(&mut self, setpoint: Setpoint) {
        self.apply(CommandState::On(setpoint));
        self.ordered = true;
    }

    /// The state saved before a restart (the broker keeps the last one published), unless an order came since.
    /// A Boost does not outlive a restart. True when the state was restored.
    pub fn restore(&mut self, mode: Mode, setpoint: Setpoint) -> bool {
        if self.ordered {
            return false;
        }
        self.setpoint = setpoint;
        self.mode = if mode == Mode::Boost { Mode::On } else { mode };
        true
    }

    pub fn boost_us(&self) -> u64 {
        self.boost_us
    }

    pub fn set_boost_us(&mut self, boost_us: u64) {
        self.boost_us = boost_us;
    }

    /// Ends a Boost ordered by Home Assistant once its time is over; true when that just happened.
    pub fn expire(&mut self, now_us: u64) -> bool {
        let Some((back, since_us)) = self.boost else { return false };
        let over = now_us.wrapping_sub(since_us) > self.boost_us;
        if over {
            self.mode = back;
            self.boost = None;
        }
        over
    }

    /// A frame completed by the command board; true when the keys changed the state. Its first answer is taken
    /// as it is, until the saved state comes; only its changes are orders.
    pub fn from_command_board(&mut self, frame: Frame) -> bool {
        if frame.header() != header::STATE || !frame.is_valid() || self.keys == Some(frame) {
            return false;
        }
        let first = self.keys.replace(frame).is_none();
        let Some(state) = CommandState::decode(&frame) else { return false };
        let before = self.state();
        self.apply(state);
        self.ordered |= !first;
        self.state() != before
    }

    /// The command board ends its own Boost: the clock is only for a Boost ordered by Home Assistant.
    fn apply(&mut self, state: CommandState) {
        match state {
            CommandState::On(setpoint) => {
                self.setpoint = setpoint;
                self.mode = Mode::On;
            }
            CommandState::Boost => self.mode = Mode::Boost,
            CommandState::Off => self.mode = Mode::Off,
        }
        self.boost = None;
    }

    /// The 4 bytes that complete the frame after this poll byte.
    pub fn reply_for(&self, poll: u8) -> Option<[u8; 4]> {
        match poll {
            header::STATE => Some(self.state().frame().body()),
            header::PERIODIC => Some(PERIODIC.body()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(halves: u8) -> CommandState {
        CommandState::On(Setpoint::from_halves(halves).unwrap())
    }

    #[test]
    fn answers_its_own_state_without_a_command_board() {
        let mitm = Mitm::new();
        assert_eq!(mitm.reply_for(header::STATE), Some(CommandState::Off.frame().body()));
        assert_eq!(mitm.reply_for(header::PERIODIC), Some(PERIODIC.body()));
        assert_eq!(mitm.reply_for(0x75), None);
    }

    #[test]
    fn takes_the_keys_when_they_change() {
        let mut mitm = Mitm::new();
        assert!(mitm.from_command_board(on(29).frame()));
        assert_eq!(mitm.state(), on(29));
        assert!(!mitm.from_command_board(on(29).frame()));
        assert!(mitm.from_command_board(CommandState::Off.frame()));
        assert_eq!(mitm.state(), CommandState::Off);
    }

    #[test]
    fn the_last_order_wins() {
        let mut mitm = Mitm::new();
        mitm.from_command_board(on(29).frame());
        mitm.set_mode(Mode::Off, 0);
        assert!(!mitm.from_command_board(on(29).frame()));
        assert_eq!(mitm.state(), CommandState::Off);
        assert!(mitm.from_command_board(on(30).frame()));
        assert_eq!(mitm.state(), on(30));
    }

    #[test]
    fn keeps_the_setpoint_while_off() {
        let mut mitm = Mitm::new();
        mitm.set_setpoint(Setpoint::from_halves(44).unwrap());
        mitm.set_mode(Mode::Off, 0);
        mitm.set_mode(Mode::On, 0);
        assert_eq!(mitm.state(), on(44));
    }

    #[test]
    fn stops_an_ordered_boost_unless_it_is_ordered_again() {
        let mut mitm = Mitm::new();
        mitm.set_boost_us(1_000);
        mitm.set_mode(Mode::Off, 0);
        mitm.set_mode(Mode::Boost, 0);
        assert!(!mitm.expire(900));
        mitm.set_mode(Mode::Boost, 900);
        assert!(!mitm.expire(1_800));
        assert!(mitm.expire(1_901));
        assert_eq!(mitm.state(), CommandState::Off);
    }

    #[test]
    fn leaves_a_boost_from_the_keys_to_the_command_board() {
        let mut mitm = Mitm::new();
        mitm.set_boost_us(1_000);
        mitm.set_mode(Mode::Boost, 0);
        mitm.from_command_board(CommandState::Boost.frame());
        assert!(!mitm.expire(5_000));
        assert_eq!(mitm.state(), CommandState::Boost);
    }

    #[test]
    fn the_saved_state_replaces_the_first_answer_of_the_keys() {
        let mut mitm = Mitm::new();
        mitm.from_command_board(on(29).frame());
        assert!(mitm.restore(Mode::Off, Setpoint::from_halves(40).unwrap()));
        assert_eq!(mitm.state(), CommandState::Off);
        mitm.set_mode(Mode::On, 0);
        assert_eq!(mitm.state(), on(40));
    }

    #[test]
    fn an_order_since_start_up_beats_the_saved_state() {
        let mut mitm = Mitm::new();
        mitm.from_command_board(on(29).frame());
        mitm.from_command_board(on(31).frame());
        assert!(!mitm.restore(Mode::Off, Setpoint::from_halves(40).unwrap()));
        assert_eq!(mitm.state(), on(31));
    }

    #[test]
    fn a_saved_boost_restores_on() {
        let mut mitm = Mitm::new();
        mitm.restore(Mode::Boost, Setpoint::from_halves(40).unwrap());
        assert_eq!(mitm.state(), on(40));
    }

    #[test]
    fn ignores_corrupted_and_unknown_answers() {
        let mut mitm = Mitm::new();
        mitm.set_mode(Mode::Boost, 0);
        assert!(!mitm.from_command_board(Frame([0x46, 0xB0, 0x9D, 0x80, 0x00])));
        assert!(!mitm.from_command_board(Frame::new(header::STATE, [0x01, 0x02, 0x04])));
        assert_eq!(mitm.state(), CommandState::Boost);
    }
}
