use crate::frame::{Frame, header};
use crate::mitm::Mitm;
use crate::wire::Wire;

/// What to do with one byte heard on the power board half of DATA.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Actions {
    /// To write at once on the power board half: the answer to its poll byte.
    pub reply: Option<[u8; 4]>,
    /// A frame completed on this half (power board frames, and our answers).
    pub frame: Option<Frame>,
}

/// We play the command board in front of the power board.
#[derive(Default)]
pub struct PowerSide {
    wire: Wire,
    pub polls: u32,
    /// Our answers heard back whole on DATA: the transmit path works.
    pub answered: u32,
}

impl PowerSide {
    pub fn on_byte(&mut self, byte: u8, now_us: u64, mitm: &Mitm) -> Actions {
        let heard = self.wire.push(byte, now_us);
        let reply = heard.poll.and_then(|poll| mitm.reply_for(poll));
        if heard.poll.is_some() {
            self.polls = self.polls.wrapping_add(1);
        }
        if heard.frame.is_some_and(|frame| header::is_poll(frame.header()) && frame.is_valid()) {
            self.answered = self.answered.wrapping_add(1);
        }
        Actions { reply, frame: heard.frame }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CommandState, PowerState};

    const BYTE_US: u64 = 1_250;

    fn feed(side: &mut PowerSide, mitm: &Mitm, bytes: &[u8], start_us: u64) -> Vec<Actions> {
        (0..bytes.len()).map(|i| side.on_byte(bytes[i], start_us + i as u64 * BYTE_US, mitm)).collect()
    }

    fn off() -> Mitm {
        let mut mitm = Mitm::new();
        mitm.from_command_board(CommandState::Off.frame());
        mitm
    }

    #[test]
    fn reads_the_power_board_frames() {
        let (mut side, mitm) = (PowerSide::default(), off());
        let actions = feed(&mut side, &mitm, &Frame::new(0xC7, [0x83, 0xE0, 0x98]).0, 0);
        assert!(actions.iter().all(|a| a.reply.is_none()));
        assert_eq!(actions[4].frame.as_ref().and_then(PowerState::decode), Some(PowerState::Boost));
    }

    #[test]
    fn answers_the_poll_once() {
        let (mut side, mitm) = (PowerSide::default(), off());
        feed(&mut side, &mitm, &Frame::new(0xC7, [0x80, 0xE0, 0x38]).0, 0);
        let answer = CommandState::Off.frame();
        let actions = feed(&mut side, &mitm, &answer.0, 20_000);
        assert_eq!(actions[0].reply, Some(answer.body()));
        assert!(actions[1..].iter().all(|a| a.reply.is_none()));
        assert_eq!(actions[4].frame, Some(answer));
        assert_eq!((side.polls, side.answered), (1, 1));
    }

    #[test]
    fn counts_only_the_answers_heard_back() {
        let (mut side, mitm) = (PowerSide::default(), Mitm::new());
        let actions = feed(&mut side, &mitm, &[0x46], 0);
        assert!(actions[0].reply.is_some());
        assert_eq!((side.polls, side.answered), (1, 0));
    }
}
