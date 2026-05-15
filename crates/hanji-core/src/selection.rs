use crate::{EditError, TextBuffer, TextRange};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    ranges: Vec<TextRange>,
    primary_index: usize,
}

impl Selection {
    pub fn new(ranges: Vec<TextRange>, primary_index: usize) -> Result<Self, SelectionError> {
        if ranges.is_empty() {
            return Err(SelectionError::Empty);
        }

        if primary_index >= ranges.len() {
            return Err(SelectionError::InvalidPrimaryIndex);
        }

        Ok(Self {
            ranges,
            primary_index,
        })
    }

    pub fn single(range: TextRange) -> Self {
        Self {
            ranges: vec![range],
            primary_index: 0,
        }
    }

    pub fn caret(offset: usize) -> Self {
        Self::single(TextRange::caret(offset))
    }

    pub fn ranges(&self) -> &[TextRange] {
        &self.ranges
    }

    pub fn primary(&self) -> TextRange {
        self.ranges[self.primary_index]
    }

    pub fn primary_index(&self) -> usize {
        self.primary_index
    }

    pub fn is_caret(&self) -> bool {
        self.ranges.len() == 1 && self.ranges[0].is_empty()
    }

    pub(crate) fn validate_for_buffer(&self, buffer: &TextBuffer) -> Result<(), EditError> {
        if self.ranges.is_empty() {
            return Err(EditError::EmptySelection);
        }

        for range in &self.ranges {
            buffer.validate_range(*range)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionError {
    Empty,
    InvalidPrimaryIndex,
}
