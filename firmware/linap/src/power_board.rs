use crate::byte::BIT_US;
use crate::frame::{Frame, header};

/// One slot of the power board cycle: 5 slots make the ≈ 305 ms cycle measured on the radiator.
pub const SLOT_MS: u64 = 61;
/// From the end of a 75 or C7 frame to the poll byte, as measured on the radiator.
pub const POLL_DELAY_MS: u64 = 13;
const PERIODIC_MS: u64 = 30_000;

const CYCLE: Frame = Frame::new(header::CYCLE, [0x83, 0xB5, 0x02]);
const IDLE: Frame = Frame::new(header::POWER, [0x80, 0xE0, 0x38]);
const TAIL: Frame = Frame::new(0x61, [0x1C, 0x4C, 0x80]);

/// Idle bits the power board leaves after the header, then between the other bytes (capture of 07/11/2026).
const AFTER_HEADER_BITS: u64 = 7;
const BETWEEN_BYTES_BITS: u64 = 2;

/// Plays the power board in front of the command board, on its own clock (`discovery/2-power-emulator`, in Rust):
/// 75, nothing, C7, nothing, 61, and the poll byte after 75 and C7. The command board ignores what these frames
/// say (`knowledge/linap.md`): they are constant, whatever the real power board does.
#[derive(Default)]
pub struct PowerBoard {
    slot: u8,
    last_periodic_ms: Option<u64>,
}

impl PowerBoard {
    /// The frame of the next slot, if any.
    pub fn next_slot(&mut self) -> Option<Frame> {
        let frame = match self.slot {
            0 => Some(CYCLE),
            2 => Some(IDLE),
            4 => Some(TAIL),
            _ => None,
        };
        self.slot = (self.slot + 1) % 5;
        frame
    }

    /// The poll byte to send after this frame: 70 after a 75 about every 30 s, 46 otherwise.
    pub fn poll_after(&mut self, frame: &Frame, now_ms: u64) -> Option<u8> {
        match frame.header() {
            header::CYCLE if self.last_periodic_ms.is_none_or(|last| now_ms - last >= PERIODIC_MS) => {
                self.last_periodic_ms = Some(now_ms);
                Some(header::PERIODIC)
            }
            header::CYCLE | header::POWER => Some(header::STATE),
            _ => None,
        }
    }
}

/// Wait after writing byte `index` of a frame before writing the next one: the byte itself (10 bits) and the
/// idle bits the power board leaves.
pub const fn byte_delay_us(index: usize) -> u64 {
    let idle = if index == 0 { AFTER_HEADER_BITS } else { BETWEEN_BYTES_BITS };
    (10 + idle) * BIT_US
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plays_the_five_slot_cycle() {
        let mut board = PowerBoard::default();
        let headers: Vec<_> = (0..10).map(|_| board.next_slot().map(|frame| frame.header())).collect();
        let cycle = [Some(0x75), None, Some(0xC7), None, Some(0x61)];
        assert_eq!(headers, [cycle, cycle].concat());
    }

    #[test]
    fn polls_after_75_and_c7_with_70_every_30_s() {
        let mut board = PowerBoard::default();
        assert_eq!(board.poll_after(&CYCLE, 0), Some(0x70));
        assert_eq!(board.poll_after(&CYCLE, 300), Some(0x46));
        assert_eq!(board.poll_after(&IDLE, 600), Some(0x46));
        assert_eq!(board.poll_after(&TAIL, 900), None);
        assert_eq!(board.poll_after(&CYCLE, 30_000), Some(0x70));
    }

    #[test]
    fn frames_are_valid() {
        assert!([CYCLE, IDLE, TAIL].iter().all(Frame::is_valid));
    }
}
