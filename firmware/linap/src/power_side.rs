use crate::echoes::Echoes;
use crate::frame::Frame;
use crate::mitm::Mitm;
use crate::wire::Wire;

/// What to do with one byte heard on the power board half of DATA.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Actions {
    /// To write at once on the power board half: the answer to its poll byte.
    pub reply: Option<[u8; 4]>,
    /// To write on the command board half: everything the power board says, its poll bytes included.
    pub forward: Option<u8>,
    /// A frame completed on this half (power board frames, and our answers).
    pub frame: Option<Frame>,
}

/// We play the command board in front of the power board.
#[derive(Default)]
pub struct PowerSide {
    wire: Wire,
    echoes: Echoes,
    pub polls: u32,
    pub answered: u32,
}

impl PowerSide {
    pub fn on_byte(&mut self, byte: u8, now_us: u64, mitm: &Mitm) -> Actions {
        let echo = self.echoes.is_echo(byte);
        let heard = self.wire.push(byte, now_us);
        let reply = heard.poll.and_then(|poll| mitm.reply_for(poll));
        if heard.poll.is_some() {
            self.polls = self.polls.wrapping_add(1);
        }
        if let Some(reply) = reply {
            self.answered = self.answered.wrapping_add(1);
            self.echoes.expect(&reply);
        }
        Actions { reply, forward: (!echo).then_some(byte), frame: heard.frame }
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

    fn relaying_off() -> Mitm {
        let mut mitm = Mitm::new();
        mitm.from_command_board(CommandState::Off.frame());
        mitm
    }

    #[test]
    fn forwards_the_power_board_frames() {
        let (mut side, mitm) = (PowerSide::default(), relaying_off());
        let frame = Frame::new(0xC7, [0x83, 0xE0, 0x98]);
        let actions = feed(&mut side, &mitm, &frame.0, 0);
        assert_eq!(actions.iter().map(|a| a.forward.unwrap()).collect::<Vec<_>>(), frame.0);
        assert_eq!(actions[4].frame.as_ref().and_then(PowerState::decode), Some(PowerState::Boost));
    }

    #[test]
    fn answers_the_poll_and_does_not_forward_its_own_answer() {
        let (mut side, mitm) = (PowerSide::default(), relaying_off());
        feed(&mut side, &mitm, &Frame::new(0xC7, [0x80, 0xE0, 0x38]).0, 0);
        let answer = CommandState::Off.frame();
        let actions = feed(&mut side, &mitm, &answer.0, 20_000);
        assert_eq!(actions[0].reply, Some(answer.body()));
        assert_eq!(actions[0].forward, Some(0x46));
        assert!(actions[1..].iter().all(|a| a.reply.is_none() && a.forward.is_none()));
        assert_eq!(actions[4].frame, Some(answer));
        assert_eq!((side.polls, side.answered), (1, 1));
    }

    #[test]
    fn counts_the_polls_it_cannot_answer() {
        let (mut side, mitm) = (PowerSide::default(), Mitm::new());
        let actions = feed(&mut side, &mitm, &[0x46], 0);
        assert_eq!(actions[0].reply, None);
        assert_eq!((side.polls, side.answered), (1, 0));
    }
}
