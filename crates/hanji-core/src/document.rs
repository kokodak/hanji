use crate::{EditError, Selection, TextBuffer, TextPosition, TextRange, Transaction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    buffer: TextBuffer,
    selection: Selection,
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
}

impl Document {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            buffer: TextBuffer::new(text),
            selection: Selection::caret(0),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn text(&self) -> &str {
        self.buffer.as_str()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn line_count(&self) -> usize {
        self.buffer.line_count()
    }

    pub fn line_range(&self, line_index: usize) -> Option<TextRange> {
        self.buffer.line_range(line_index)
    }

    pub fn line_index_at_offset(&self, offset: usize) -> Result<usize, EditError> {
        self.buffer.line_index_at_offset(offset)
    }

    pub fn position_at_offset(&self, offset: usize) -> Result<TextPosition, EditError> {
        self.buffer.position_at_offset(offset)
    }

    pub fn offset_at_position(&self, position: TextPosition) -> Result<usize, EditError> {
        self.buffer.offset_at_position(position)
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn set_selection(&mut self, selection: Selection) -> Result<(), EditError> {
        selection.validate_for_buffer(&self.buffer)?;
        self.selection = selection;
        Ok(())
    }

    pub fn replace_all(&mut self, text: impl Into<String>) {
        self.buffer.replace_all(text);
        self.selection = Selection::caret(0);
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn apply(&mut self, transaction: Transaction) -> Result<DocumentChange, EditError> {
        let (edits, selection_after) = transaction.into_parts();
        let before_text = self.buffer.clone();
        let before_selection = self.selection.clone();
        let mut after_text = self.buffer.clone();

        after_text.apply_edits(&edits)?;

        let selection_after = selection_after.unwrap_or_else(|| self.selection.clone());
        selection_after.validate_for_buffer(&after_text)?;

        self.buffer = after_text;
        self.selection = selection_after;
        self.undo_stack.push(HistoryEntry {
            before_text,
            before_selection,
            after_text: self.buffer.clone(),
            after_selection: self.selection.clone(),
        });
        self.redo_stack.clear();

        Ok(DocumentChange::Applied)
    }

    pub fn undo(&mut self) -> Option<DocumentChange> {
        let entry = self.undo_stack.pop()?;

        self.buffer = entry.before_text.clone();
        self.selection = entry.before_selection.clone();
        self.redo_stack.push(entry);

        Some(DocumentChange::Undone)
    }

    pub fn redo(&mut self) -> Option<DocumentChange> {
        let entry = self.redo_stack.pop()?;

        self.buffer = entry.after_text.clone();
        self.selection = entry.after_selection.clone();
        self.undo_stack.push(entry);

        Some(DocumentChange::Redone)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentChange {
    Applied,
    Undone,
    Redone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HistoryEntry {
    before_text: TextBuffer,
    before_selection: Selection,
    after_text: TextBuffer,
    after_selection: Selection,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TextRange;

    #[test]
    fn applies_insert_transaction_and_moves_selection() {
        let mut document = Document::new("Hanji");

        let change = document.apply(Transaction::insert(5, " notes")).unwrap();

        assert_eq!(change, DocumentChange::Applied);
        assert_eq!(document.text(), "Hanji notes");
        assert_eq!(document.selection().primary(), TextRange::caret(11));
    }

    #[test]
    fn rejects_non_char_boundary_edits_without_changing_document() {
        let mut document = Document::new("한지");

        let error = document
            .apply(Transaction::replace(TextRange::new(1, 1), "x"))
            .unwrap_err();

        assert_eq!(error, EditError::InvalidBoundary);
        assert_eq!(document.text(), "한지");
        assert!(!document.can_undo());
    }

    #[test]
    fn rejects_invalid_selection_without_changing_document() {
        let mut document = Document::new("Hanji");

        let error = document
            .apply(Transaction::insert(5, " notes").with_selection_after(Selection::caret(99)))
            .unwrap_err();

        assert_eq!(error, EditError::InvalidRange);
        assert_eq!(document.text(), "Hanji");
        assert_eq!(document.selection().primary(), TextRange::caret(0));
        assert!(!document.can_undo());
    }

    #[test]
    fn undo_and_redo_restore_text_and_selection() {
        let mut document = Document::new("Hanji");

        document.set_selection(Selection::caret(5)).unwrap();
        document.apply(Transaction::insert(5, " notes")).unwrap();

        assert_eq!(document.undo(), Some(DocumentChange::Undone));
        assert_eq!(document.text(), "Hanji");
        assert_eq!(document.selection().primary(), TextRange::caret(5));
        assert!(document.can_redo());

        assert_eq!(document.redo(), Some(DocumentChange::Redone));
        assert_eq!(document.text(), "Hanji notes");
        assert_eq!(document.selection().primary(), TextRange::caret(11));
    }

    #[test]
    fn new_transaction_clears_redo_history() {
        let mut document = Document::new("Hanji");

        document.apply(Transaction::insert(5, " notes")).unwrap();
        document.undo();
        document.apply(Transaction::insert(5, " draft")).unwrap();

        assert_eq!(document.text(), "Hanji draft");
        assert!(!document.can_redo());
    }

    #[test]
    fn exposes_line_lookup() {
        let document = Document::new("Hanji\nnotes");

        assert_eq!(document.line_count(), 2);
        assert_eq!(document.line_range(1), Some(TextRange::new(6, 11)));
        assert_eq!(document.line_index_at_offset(6), Ok(1));
    }

    #[test]
    fn exposes_position_conversion() {
        let document = Document::new("한지\nnotes");

        assert_eq!(
            document.position_at_offset("한지\nno".len()),
            Ok(TextPosition::new(1, 2))
        );
        assert_eq!(
            document.offset_at_position(TextPosition::new(0, 2)),
            Ok("한지".len())
        );
    }
}
