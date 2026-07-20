use crate::TextRange;

/// A single directional source selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    anchor: usize,
    head: usize,
}

impl TextSelection {
    pub const fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    pub const fn caret(offset: usize) -> Self {
        Self::new(offset, offset)
    }

    pub const fn anchor(self) -> usize {
        self.anchor
    }

    pub const fn head(self) -> usize {
        self.head
    }

    pub fn range(self) -> TextRange {
        TextRange::new(
            if self.anchor < self.head {
                self.anchor
            } else {
                self.head
            },
            if self.anchor > self.head {
                self.anchor
            } else {
                self.head
            },
        )
    }

    pub const fn is_reversed(self) -> bool {
        self.head < self.anchor
    }
}
