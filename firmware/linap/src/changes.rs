use crate::frame::Frame;

/// Remembers the last frame of each header heard on each half of DATA, to keep only the ones that change.
#[derive(Default)]
pub struct Changes {
    last: [Option<(u8, Frame)>; 32],
}

impl Changes {
    /// True when this frame differs from the last one of its header on this half (`side`), or is the first.
    pub fn is_new(&mut self, side: u8, frame: Frame) -> bool {
        let same_kind =
            |entry: &Option<(u8, Frame)>| entry.is_some_and(|(s, last)| s == side && last.header() == frame.header());
        let slot = match self.last.iter().position(same_kind) {
            Some(index) => index,
            None => match self.last.iter().position(Option::is_none) {
                Some(index) => index,
                None => return true,
            },
        };
        let new = self.last[slot] != Some((side, frame));
        self.last[slot] = Some((side, frame));
        new
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_frames_that_change() {
        let mut changes = Changes::default();
        let idle = Frame::new(0xC7, [0x80, 0xE0, 0x38]);
        assert!(changes.is_new(0, idle));
        assert!(!changes.is_new(0, idle));
        assert!(changes.is_new(1, idle));
        assert!(changes.is_new(0, Frame::new(0xC7, [0x01, 0xE0, 0x38])));
        assert!(changes.is_new(0, idle));
    }

    #[test]
    fn headers_are_tracked_apart() {
        let mut changes = Changes::default();
        assert!(changes.is_new(0, Frame::new(0x75, [0x83, 0xB5, 0x02])));
        assert!(changes.is_new(0, Frame::new(0x61, [0x1C, 0x4C, 0x80])));
        assert!(!changes.is_new(0, Frame::new(0x75, [0x83, 0xB5, 0x02])));
    }
}
