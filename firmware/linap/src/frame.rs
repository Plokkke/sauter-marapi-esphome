use core::fmt;

use crate::byte::has_odd_parity;

/// Header bytes, with their parity bit, as read on the wire.
pub mod header {
    /// Poll for the command board state, and header of its answer.
    pub const STATE: u8 = 0x46;
    /// Poll sent about every 30 s instead of `STATE` after a `CYCLE`.
    pub const PERIODIC: u8 = 0x70;
    /// Power board state.
    pub const POWER: u8 = 0xC7;
    pub const CYCLE: u8 = 0x75;
    pub const BURST_END: u8 = 0x76;

    pub const fn is_poll(byte: u8) -> bool {
        byte == STATE || byte == PERIODIC
    }
}

/// Header, 3 data bytes, then the XOR of the 3 data bytes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Frame(pub [u8; 5]);

impl Frame {
    pub const fn new(header: u8, data: [u8; 3]) -> Self {
        Self([header, data[0], data[1], data[2], data[0] ^ data[1] ^ data[2]])
    }

    pub const fn header(&self) -> u8 {
        self.0[0]
    }

    pub fn data(&self) -> [u8; 3] {
        [self.0[1], self.0[2], self.0[3]]
    }

    /// What follows the header: the answer to a poll byte.
    pub fn body(&self) -> [u8; 4] {
        [self.0[1], self.0[2], self.0[3], self.0[4]]
    }

    /// Checksum and parity of every byte (the XOR of three odd bytes is odd too).
    pub fn is_valid(&self) -> bool {
        self.0[1] ^ self.0[2] ^ self.0[3] == self.0[4] && self.0.iter().all(|&byte| has_odd_parity(byte))
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d, e] = self.0;
        write!(f, "{a:02X} {b:02X} {c:02X} {d:02X} {e:02X}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_the_checksum() {
        assert_eq!(Frame::new(0xC7, [0x83, 0xE0, 0x98]).0, [0xC7, 0x83, 0xE0, 0x98, 0xFB]);
    }

    #[test]
    fn checks_checksum_and_parity() {
        assert!(Frame::new(0x46, [0xB0, 0x9D, 0x80]).is_valid());
        assert!(!Frame([0x46, 0xB0, 0x9D, 0x80, 0x00]).is_valid());
        assert!(!Frame::new(0x46, [0x30, 0x9D, 0x80]).is_valid());
    }

    #[test]
    fn prints_as_hex() {
        assert_eq!(Frame::new(0x70, [0x83, 0x80, 0x89]).to_string(), "70 83 80 89 8A");
    }
}
