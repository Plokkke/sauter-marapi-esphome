use crate::byte::with_parity;
use crate::frame::{Frame, header};
use crate::setpoint::Setpoint;

/// What the command board answers to the `46` poll: what the user set on its keys.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommandState {
    Off,
    Boost,
    /// Effective setpoint (the Eco one in Eco mode).
    On(Setpoint),
}

const OFF: [u8; 3] = [0x80, 0x80, 0x80];
const BOOST: [u8; 3] = [0x38, 0x80, 0x80];
const ON: u8 = 0xB0;
const ON_TRAILER: u8 = 0x80;

impl CommandState {
    pub fn frame(self) -> Frame {
        Frame::new(
            header::STATE,
            match self {
                Self::Off => OFF,
                Self::Boost => BOOST,
                Self::On(setpoint) => [ON, with_parity(setpoint.halves()), ON_TRAILER],
            },
        )
    }

    /// None for a frame of another header or data never seen.
    pub fn decode(frame: &Frame) -> Option<Self> {
        if frame.header() != header::STATE || !frame.is_valid() {
            return None;
        }
        match frame.data() {
            OFF => Some(Self::Off),
            BOOST => Some(Self::Boost),
            [ON, setpoint, ON_TRAILER] => Setpoint::from_halves(setpoint & 0x7F).map(Self::On),
            _ => None,
        }
    }
}

/// What the power board reports in `C7`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PowerState {
    Idle,
    /// Heats the fluid: the setpoint is above what its own sensor reads.
    Heating,
    Boost,
    /// Briefly, at the end of a Boost.
    BoostEnding,
    Off,
    Unknown([u8; 3]),
}

impl PowerState {
    pub fn decode(frame: &Frame) -> Option<Self> {
        if frame.header() != header::POWER || !frame.is_valid() {
            return None;
        }
        Some(match frame.data() {
            [0x80, 0xE0, 0x38] => Self::Idle,
            [0x01, 0xE0, 0x38] => Self::Heating,
            [0x83, 0xE0, 0x98] => Self::Boost,
            [0x83, 0xE0, 0x38] => Self::BoostEnding,
            [0x80, 0xE0, 0x40] => Self::Off,
            data => Self::Unknown(data),
        })
    }

    /// The resistance or the fan is on.
    pub fn is_heating(self) -> bool {
        matches!(self, Self::Heating | Self::Boost)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Heating => "heating",
            Self::Boost => "boost",
            Self::BoostEnding => "boost_ending",
            Self::Off => "off",
            Self::Unknown(_) => "unknown",
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
    fn encodes_the_captured_frames() {
        assert_eq!(on(29).frame(), Frame::new(0x46, [0xB0, 0x9D, 0x80]));
        assert_eq!(on(41).frame(), Frame::new(0x46, [0xB0, 0x29, 0x80]));
        assert_eq!(CommandState::Boost.frame(), Frame::new(0x46, [0x38, 0x80, 0x80]));
    }

    #[test]
    fn decodes_what_it_encodes() {
        for state in [CommandState::Off, CommandState::Boost, on(10), on(34), on(60)] {
            assert_eq!(CommandState::decode(&state.frame()), Some(state));
        }
    }

    #[test]
    fn does_not_decode_other_frames() {
        assert_eq!(CommandState::decode(&Frame::new(0x70, [0x83, 0x80, 0x89])), None);
        assert_eq!(CommandState::decode(&Frame::new(0x46, [0x83, 0x80, 0x89])), None);
    }

    #[test]
    fn decodes_the_power_board_state() {
        let boost = Frame::new(0xC7, [0x83, 0xE0, 0x98]);
        assert_eq!(PowerState::decode(&boost), Some(PowerState::Boost));
        assert!(PowerState::decode(&Frame::new(0xC7, [0x01, 0xE0, 0x38])).unwrap().is_heating());
        assert_eq!(PowerState::decode(&Frame::new(0x75, [0x83, 0xB5, 0x02])), None);
    }
}
