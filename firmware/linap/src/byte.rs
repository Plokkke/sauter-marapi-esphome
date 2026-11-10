/// Duration of one bit at 9600 baud, in microseconds (rounded).
pub const BIT_US: u64 = 104;

/// 7 data bits plus the odd parity bit as bit 7: the bus is 7O1, read and written as 8N1.
pub const fn with_parity(data: u8) -> u8 {
    let data = data & 0x7F;
    if data.count_ones() % 2 == 1 { data } else { data | 0x80 }
}

pub const fn has_odd_parity(byte: u8) -> bool {
    byte.count_ones() % 2 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_the_parity_bit_only_when_needed() {
        assert_eq!(with_parity(0x46), 0x46);
        assert_eq!(with_parity(0x1D), 0x9D);
        assert_eq!(with_parity(0x7E), 0xFE);
    }

    #[test]
    fn every_byte_with_parity_is_odd() {
        assert!((0..=0x7F).all(|data| has_odd_parity(with_parity(data))));
    }
}
