#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBuffer {
    text: String,
}

impl TextBuffer {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn line_count(&self) -> usize {
        self.text.bytes().filter(|byte| *byte == b'\n').count() + 1
    }

    pub fn line_range(&self, line_index: usize) -> Option<TextRange> {
        let mut current_line = 0;
        let mut line_start = 0;

        for (offset, character) in self.text.char_indices() {
            if character == '\n' {
                if current_line == line_index {
                    return Some(TextRange::new(line_start, offset));
                }

                current_line += 1;
                line_start = offset + character.len_utf8();
            }
        }

        (current_line == line_index).then_some(TextRange::new(line_start, self.text.len()))
    }

    pub fn slice(&self, range: TextRange) -> Result<&str, EditError> {
        self.validate_range(range)?;
        Ok(&self.text[range.start..range.end])
    }

    pub fn apply_edits(&mut self, edits: &[TextEdit]) -> Result<(), EditError> {
        validate_edits(&self.text, edits)?;

        let mut edits = edits.to_vec();
        edits.sort_by_key(|edit| edit.range.start);

        for edit in edits.into_iter().rev() {
            self.text
                .replace_range(edit.range.start..edit.range.end, &edit.text);
        }

        Ok(())
    }

    pub fn validate_range(&self, range: TextRange) -> Result<(), EditError> {
        validate_range(&self.text, range)
    }

    pub fn replace_all(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn caret(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    pub fn len(self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: TextRange,
    pub text: String,
}

impl TextEdit {
    pub fn insert(offset: usize, text: impl Into<String>) -> Self {
        Self {
            range: TextRange::caret(offset),
            text: text.into(),
        }
    }

    pub fn replace(range: TextRange, text: impl Into<String>) -> Self {
        Self {
            range,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditError {
    EmptySelection,
    InvalidBoundary,
    InvalidRange,
    OverlappingEdits,
}

fn validate_edits(text: &str, edits: &[TextEdit]) -> Result<(), EditError> {
    let mut ranges: Vec<TextRange> = edits.iter().map(|edit| edit.range).collect();

    for range in &ranges {
        validate_range(text, *range)?;
    }

    ranges.sort_by_key(|range| range.start);

    for pair in ranges.windows(2) {
        if pair[1].start < pair[0].end {
            return Err(EditError::OverlappingEdits);
        }
    }

    Ok(())
}

fn validate_range(text: &str, range: TextRange) -> Result<(), EditError> {
    if range.start > range.end || range.end > text.len() {
        return Err(EditError::InvalidRange);
    }

    if !text.is_char_boundary(range.start) || !text.is_char_boundary(range.end) {
        return Err(EditError::InvalidBoundary);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_line_ranges_without_newlines() {
        let buffer = TextBuffer::new("one\ntwo\nthree");

        assert_eq!(buffer.line_count(), 3);
        assert_eq!(buffer.line_range(0), Some(TextRange::new(0, 3)));
        assert_eq!(buffer.line_range(1), Some(TextRange::new(4, 7)));
        assert_eq!(buffer.line_range(2), Some(TextRange::new(8, 13)));
        assert_eq!(buffer.line_range(3), None);
    }

    #[test]
    fn applies_multiple_edits_against_original_offsets() {
        let mut buffer = TextBuffer::new("alpha beta gamma");

        buffer
            .apply_edits(&[
                TextEdit::replace(TextRange::new(0, 5), "one"),
                TextEdit::replace(TextRange::new(11, 16), "three"),
            ])
            .unwrap();

        assert_eq!(buffer.as_str(), "one beta three");
    }

    #[test]
    fn rejects_overlapping_edits() {
        let mut buffer = TextBuffer::new("Hanji");

        let error = buffer
            .apply_edits(&[
                TextEdit::replace(TextRange::new(0, 3), "A"),
                TextEdit::replace(TextRange::new(2, 5), "B"),
            ])
            .unwrap_err();

        assert_eq!(error, EditError::OverlappingEdits);
        assert_eq!(buffer.as_str(), "Hanji");
    }
}
