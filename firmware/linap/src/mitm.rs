use crate::frame::{Frame, header};
use crate::state::CommandState;

/// Without news from Home Assistant for this long, an imposed state is dropped (`knowledge/architecture.md`).
pub const HOME_ASSISTANT_TIMEOUT_US: u64 = 30 * 60 * 1_000_000;

/// An imposed Boost stops by itself after this long, unless Home Assistant imposes it again.
pub const DEFAULT_BOOST_US: u64 = 30 * 60 * 1_000_000;

/// The constant answer of the command board to the `70` poll (meaning unknown).
const PERIODIC: Frame = Frame::new(header::PERIODIC, [0x83, 0x80, 0x89]);

/// Who decides what the power board receives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Control {
    /// The command board, as if we were not there.
    Relay,
    /// Home Assistant, until the next change made on the keys.
    Impose(CommandState),
}

/// Answers the power board with the command board's last answer, or with what Home Assistant imposes.
pub struct Mitm {
    keys: Option<Frame>,
    periodic: Option<Frame>,
    imposed: Option<CommandState>,
    imposed_at_us: u64,
    last_contact_us: u64,
    boost_us: u64,
}

impl Default for Mitm {
    fn default() -> Self {
        Self::new()
    }
}

impl Mitm {
    pub const fn new() -> Self {
        Self {
            keys: None,
            periodic: None,
            imposed: None,
            imposed_at_us: 0,
            last_contact_us: 0,
            boost_us: DEFAULT_BOOST_US,
        }
    }

    pub fn control(&self) -> Control {
        self.imposed.map_or(Control::Relay, Control::Impose)
    }

    /// What the user set on the keys, when the command board answered with a known state.
    pub fn keys(&self) -> Option<CommandState> {
        self.keys.as_ref().and_then(CommandState::decode)
    }

    /// Imposing a state again restarts its clock: that is how Home Assistant extends a Boost.
    pub fn set_control(&mut self, control: Control, now_us: u64) {
        self.imposed = match control {
            Control::Relay => None,
            Control::Impose(state) => Some(state),
        };
        self.imposed_at_us = now_us;
        self.last_contact_us = now_us;
    }

    pub fn boost_us(&self) -> u64 {
        self.boost_us
    }

    pub fn set_boost_us(&mut self, boost_us: u64) {
        self.boost_us = boost_us;
    }

    pub fn home_assistant_seen(&mut self, now_us: u64) {
        self.last_contact_us = now_us;
    }

    /// Back to relaying when Home Assistant has been silent too long, or when an imposed Boost is over; true
    /// when that just happened.
    pub fn expire(&mut self, now_us: u64) -> bool {
        let silent = now_us.wrapping_sub(self.last_contact_us) > HOME_ASSISTANT_TIMEOUT_US;
        let boost_over =
            self.imposed == Some(CommandState::Boost) && now_us.wrapping_sub(self.imposed_at_us) > self.boost_us;
        let expired = (silent || boost_over) && self.imposed.is_some();
        if expired {
            self.imposed = None;
        }
        expired
    }

    /// A frame completed by the command board; true when its state changed, which ends an imposed state.
    pub fn from_command_board(&mut self, frame: Frame) -> bool {
        if !frame.is_valid() {
            return false;
        }
        let last = match frame.header() {
            header::STATE => &mut self.keys,
            header::PERIODIC => &mut self.periodic,
            _ => return false,
        };
        let changed = last.is_some_and(|last| last != frame);
        *last = Some(frame);
        if changed && frame.header() == header::STATE {
            self.imposed = None;
        }
        changed && frame.header() == header::STATE
    }

    /// The 4 bytes that complete the frame after this poll byte; None when there is nothing to say yet.
    pub fn reply_for(&self, poll: u8) -> Option<[u8; 4]> {
        let state = self.imposed.map(CommandState::frame).or(self.keys)?;
        match poll {
            header::STATE => Some(state.body()),
            header::PERIODIC => Some(self.periodic.unwrap_or(PERIODIC).body()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setpoint::Setpoint;

    fn on(halves: u8) -> CommandState {
        CommandState::On(Setpoint::from_halves(halves).unwrap())
    }

    #[test]
    fn says_nothing_before_the_command_board_answers() {
        assert_eq!(Mitm::new().reply_for(header::STATE), None);
    }

    #[test]
    fn relays_the_command_board_by_default() {
        let mut mitm = Mitm::new();
        mitm.from_command_board(on(29).frame());
        assert_eq!(mitm.reply_for(header::STATE), Some(on(29).frame().body()));
        assert_eq!(mitm.reply_for(header::PERIODIC), Some(PERIODIC.body()));
        assert_eq!(mitm.reply_for(0x75), None);
    }

    #[test]
    fn imposes_home_assistant_until_the_keys_change() {
        let mut mitm = Mitm::new();
        mitm.from_command_board(on(29).frame());
        mitm.set_control(Control::Impose(CommandState::Boost), 0);
        assert!(!mitm.from_command_board(on(29).frame()));
        assert_eq!(mitm.reply_for(header::STATE), Some(CommandState::Boost.frame().body()));
        assert!(mitm.from_command_board(on(30).frame()));
        assert_eq!(mitm.control(), Control::Relay);
        assert_eq!(mitm.reply_for(header::STATE), Some(on(30).frame().body()));
    }

    #[test]
    fn imposes_even_without_a_command_board() {
        let mut mitm = Mitm::new();
        mitm.set_control(Control::Impose(on(40)), 0);
        assert_eq!(mitm.reply_for(header::STATE), Some(on(40).frame().body()));
    }

    #[test]
    fn goes_back_to_relaying_when_home_assistant_is_silent() {
        let mut mitm = Mitm::new();
        mitm.set_control(Control::Impose(CommandState::Off), 1_000);
        mitm.home_assistant_seen(HOME_ASSISTANT_TIMEOUT_US);
        assert!(!mitm.expire(HOME_ASSISTANT_TIMEOUT_US + 2_000));
        assert!(mitm.expire(2 * HOME_ASSISTANT_TIMEOUT_US + 1));
        assert_eq!(mitm.control(), Control::Relay);
    }

    #[test]
    fn stops_an_imposed_boost_unless_it_is_imposed_again() {
        let mut mitm = Mitm::new();
        mitm.set_boost_us(1_000);
        mitm.set_control(Control::Impose(CommandState::Boost), 0);
        assert!(!mitm.expire(900));
        mitm.set_control(Control::Impose(CommandState::Boost), 900);
        assert!(!mitm.expire(1_800));
        assert!(mitm.expire(1_901));
        assert_eq!(mitm.control(), Control::Relay);
    }

    #[test]
    fn does_not_time_out_an_imposed_setpoint() {
        let mut mitm = Mitm::new();
        mitm.set_boost_us(1_000);
        mitm.set_control(Control::Impose(on(40)), 0);
        mitm.home_assistant_seen(5_000);
        assert!(!mitm.expire(5_000));
    }

    #[test]
    fn ignores_corrupted_answers() {
        let mut mitm = Mitm::new();
        mitm.from_command_board(on(29).frame());
        mitm.set_control(Control::Impose(CommandState::Boost), 0);
        assert!(!mitm.from_command_board(Frame([0x46, 0xB0, 0x9D, 0x80, 0x00])));
        assert_eq!(mitm.control(), Control::Impose(CommandState::Boost));
    }
}
