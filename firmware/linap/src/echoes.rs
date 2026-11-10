/// Bytes we wrote on a wire and will hear back, in order: DATA is shared, our receiver hears our transmitter.
#[derive(Default)]
pub struct Echoes {
    bytes: [u8; 8],
    len: usize,
}

impl Echoes {
    pub fn expect(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if self.len < self.bytes.len() {
                self.bytes[self.len] = byte;
                self.len += 1;
            }
        }
    }

    /// True when the byte is the next one we sent; any other byte means a collision, forget the rest.
    pub fn is_echo(&mut self, byte: u8) -> bool {
        if self.len == 0 {
            return false;
        }
        if self.bytes[0] != byte {
            self.len = 0;
            return false;
        }
        self.bytes.copy_within(1..self.len, 0);
        self.len -= 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_our_bytes_in_order() {
        let mut echoes = Echoes::default();
        echoes.expect(&[0xB0, 0x9D]);
        assert!(echoes.is_echo(0xB0));
        assert!(echoes.is_echo(0x9D));
        assert!(!echoes.is_echo(0x9D));
    }

    #[test]
    fn forgets_everything_after_a_collision() {
        let mut echoes = Echoes::default();
        echoes.expect(&[0xB0, 0x9D]);
        assert!(!echoes.is_echo(0x75));
        assert!(!echoes.is_echo(0x9D));
    }
}
