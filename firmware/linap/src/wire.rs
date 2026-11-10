use crate::frame::{Frame, header};

/// Silence that separates two bursts: bytes of a frame are at most 7 idle bits apart (≈ 1.8 ms from one byte
/// to the next), the poll byte comes ≈ 13 ms after the frame before it.
pub const BURST_GAP_US: u64 = 3_000;

/// What one byte heard on a wire means.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Heard {
    /// A lone `46` or `70` after a silence: the power board asks the command board to complete the frame.
    pub poll: Option<u8>,
    /// The fifth byte of a burst: the frame it completes, valid or not.
    pub frame: Option<Frame>,
}

/// Splits everything heard on one half of DATA (both boards, our echoes included) into frames.
#[derive(Default)]
pub struct Wire {
    last_us: Option<u64>,
    bytes: [u8; 5],
    len: usize,
}

impl Wire {
    pub fn push(&mut self, byte: u8, now_us: u64) -> Heard {
        let starts_burst = self.last_us.is_none_or(|last| now_us.wrapping_sub(last) > BURST_GAP_US);
        self.last_us = Some(now_us);
        if starts_burst {
            self.len = 0;
        }
        if self.len == self.bytes.len() {
            return Heard::default();
        }
        self.bytes[self.len] = byte;
        self.len += 1;
        Heard {
            poll: (starts_burst && header::is_poll(byte)).then_some(byte),
            frame: (self.len == self.bytes.len()).then_some(Frame(self.bytes)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(wire: &mut Wire, bytes: &[u8], start_us: u64) -> Vec<Heard> {
        let byte_us = 1_250;
        (0..bytes.len()).map(|i| wire.push(bytes[i], start_us + i as u64 * byte_us)).collect()
    }

    #[test]
    fn assembles_a_frame_from_one_burst() {
        let mut wire = Wire::default();
        let heard = feed(&mut wire, &[0xC7, 0x80, 0xE0, 0x38, 0x58], 0);
        assert_eq!(heard[4].frame, Some(Frame::new(0xC7, [0x80, 0xE0, 0x38])));
        assert!(heard.iter().all(|h| h.poll.is_none()));
    }

    #[test]
    fn sees_the_poll_byte_after_a_silence_and_the_answer_completing_it() {
        let mut wire = Wire::default();
        feed(&mut wire, &[0x75, 0x83, 0xB5, 0x02, 0x36], 0);
        let heard = feed(&mut wire, &[0x46, 0xB0, 0x9D, 0x80, 0xAD], 18_000);
        assert_eq!(heard[0].poll, Some(0x46));
        assert_eq!(heard[4].frame, Some(Frame::new(0x46, [0xB0, 0x9D, 0x80])));
    }

    #[test]
    fn a_46_inside_a_frame_is_not_a_poll() {
        let mut wire = Wire::default();
        let heard = feed(&mut wire, &[0x75, 0x46, 0x46, 0x02, 0x02], 0);
        assert!(heard.iter().all(|h| h.poll.is_none()));
    }

    #[test]
    fn an_unanswered_poll_is_dropped_at_the_next_burst() {
        let mut wire = Wire::default();
        feed(&mut wire, &[0x46], 0);
        let heard = feed(&mut wire, &[0x61, 0x1C, 0x4C, 0x80, 0xD0], 40_000);
        assert_eq!(heard[4].frame.map(|f| f.header()), Some(0x61));
    }

    #[test]
    fn ignores_bytes_past_the_fifth() {
        let mut wire = Wire::default();
        let heard = feed(&mut wire, &[0x75, 0x83, 0xB5, 0x02, 0x36, 0x99], 0);
        assert_eq!(heard[5], Heard::default());
    }
}
