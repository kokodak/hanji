#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBuffer {
    text: String,
    line_index: LineIndex,
}

impl TextBuffer {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let line_index = LineIndex::new(&text);

        Self { text, line_index }
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
        self.line_index.line_count()
    }

    pub fn line_range(&self, line_index: usize) -> Option<TextRange> {
        self.line_index.line_range(self.text.len(), line_index)
    }

    pub fn line_index_at_offset(&self, offset: usize) -> Result<usize, EditError> {
        self.validate_range(TextRange::caret(offset))?;
        Ok(self.line_index.line_index_at_offset(offset))
    }

    pub fn position_at_offset(&self, offset: usize) -> Result<TextPosition, EditError> {
        self.validate_range(TextRange::caret(offset))?;

        let line = self.line_index.line_index_at_offset(offset);
        let line_start = self
            .line_index
            .line_start(line)
            .ok_or(EditError::InvalidRange)?;
        let column = self.text[line_start..offset].chars().count();

        Ok(TextPosition::new(line, column))
    }

    pub fn offset_at_position(&self, position: TextPosition) -> Result<usize, EditError> {
        let line_range = self
            .line_range(position.line)
            .ok_or(EditError::InvalidRange)?;
        let line = &self.text[line_range.start..line_range.end];

        if position.column == 0 {
            return Ok(line_range.start);
        }

        for (column, (offset, _)) in line.char_indices().enumerate() {
            if column == position.column {
                return Ok(line_range.start + offset);
            }
        }

        if position.column == line.chars().count() {
            return Ok(line_range.end);
        }

        Err(EditError::InvalidRange)
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

        self.rebuild_line_index();

        Ok(())
    }

    pub fn validate_range(&self, range: TextRange) -> Result<(), EditError> {
        validate_range(&self.text, range)
    }

    pub fn replace_all(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.rebuild_line_index();
    }

    fn rebuild_line_index(&mut self) {
        self.line_index = LineIndex::new(&self.text);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    pub line: usize,
    pub column: usize,
}

impl TextPosition {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    fn new(text: &str) -> Self {
        let mut line_starts = vec![0];

        for (offset, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset + 1);
            }
        }

        Self { line_starts }
    }

    fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    fn line_range(&self, text_len: usize, line_index: usize) -> Option<TextRange> {
        let start = *self.line_starts.get(line_index)?;
        let end = self
            .line_starts
            .get(line_index + 1)
            .map(|next_start| next_start - 1)
            .unwrap_or(text_len);

        Some(TextRange::new(start, end))
    }

    fn line_index_at_offset(&self, offset: usize) -> usize {
        self.line_starts.partition_point(|start| *start <= offset) - 1
    }

    fn line_start(&self, line_index: usize) -> Option<usize> {
        self.line_starts.get(line_index).copied()
    }
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
    fn reports_empty_trailing_line() {
        let buffer = TextBuffer::new("one\n");

        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.line_range(0), Some(TextRange::new(0, 3)));
        assert_eq!(buffer.line_range(1), Some(TextRange::new(4, 4)));
        assert_eq!(buffer.line_index_at_offset(4), Ok(1));
    }

    #[test]
    fn finds_line_index_for_mixed_utf8_offsets() {
        let buffer = TextBuffer::new("안녕 abc\n\ndfg");

        assert_eq!(buffer.line_index_at_offset(0), Ok(0));
        assert_eq!(buffer.line_index_at_offset("안녕 abc".len()), Ok(0));
        assert_eq!(buffer.line_index_at_offset("안녕 abc\n".len()), Ok(1));
        assert_eq!(buffer.line_index_at_offset("안녕 abc\n\n".len()), Ok(2));
        assert_eq!(buffer.line_index_at_offset("안녕 abc\n\ndfg".len()), Ok(2));
    }

    #[test]
    fn rejects_line_index_lookup_inside_utf8_character() {
        let buffer = TextBuffer::new("한지");

        assert_eq!(
            buffer.line_index_at_offset(1),
            Err(EditError::InvalidBoundary)
        );
    }

    #[test]
    fn rebuilds_line_index_after_edits() {
        let mut buffer = TextBuffer::new("one");

        buffer.apply_edits(&[TextEdit::insert(3, "\ntwo")]).unwrap();

        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.line_range(1), Some(TextRange::new(4, 7)));
    }

    #[test]
    fn converts_offsets_to_char_positions() {
        let buffer = TextBuffer::new("안녕 abc\n\ndfg");

        assert_eq!(buffer.position_at_offset(0), Ok(TextPosition::new(0, 0)));
        assert_eq!(
            buffer.position_at_offset("안".len()),
            Ok(TextPosition::new(0, 1))
        );
        assert_eq!(
            buffer.position_at_offset("안녕 abc".len()),
            Ok(TextPosition::new(0, 6))
        );
        assert_eq!(
            buffer.position_at_offset("안녕 abc\n".len()),
            Ok(TextPosition::new(1, 0))
        );
        assert_eq!(
            buffer.position_at_offset("안녕 abc\n\nd".len()),
            Ok(TextPosition::new(2, 1))
        );
    }

    #[test]
    fn converts_char_positions_to_offsets() {
        let buffer = TextBuffer::new("안녕 abc\n\ndfg");

        assert_eq!(buffer.offset_at_position(TextPosition::new(0, 0)), Ok(0));
        assert_eq!(
            buffer.offset_at_position(TextPosition::new(0, 1)),
            Ok("안".len())
        );
        assert_eq!(
            buffer.offset_at_position(TextPosition::new(0, 6)),
            Ok("안녕 abc".len())
        );
        assert_eq!(
            buffer.offset_at_position(TextPosition::new(1, 0)),
            Ok("안녕 abc\n".len())
        );
        assert_eq!(
            buffer.offset_at_position(TextPosition::new(2, 3)),
            Ok("안녕 abc\n\ndfg".len())
        );
    }

    #[test]
    fn rejects_positions_outside_existing_lines() {
        let buffer = TextBuffer::new("Hanji");

        assert_eq!(
            buffer.offset_at_position(TextPosition::new(1, 0)),
            Err(EditError::InvalidRange)
        );
        assert_eq!(
            buffer.offset_at_position(TextPosition::new(0, 6)),
            Err(EditError::InvalidRange)
        );
    }

    #[test]
    fn rejects_position_lookup_inside_utf8_character() {
        let buffer = TextBuffer::new("한지");

        assert_eq!(
            buffer.position_at_offset(1),
            Err(EditError::InvalidBoundary)
        );
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
