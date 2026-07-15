use crate::{Document, EditError, Selection, TextRange, Transaction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorCommand {
    InsertText(String),
    DeleteBackward,
    DeleteWordBackward,
    DeleteLineBackward,
    DeleteForward,
}

impl EditorCommand {
    pub fn insert_text(text: impl Into<String>) -> Self {
        Self::InsertText(text.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    Edit(EditError),
    MultipleSelectionsUnsupported,
}

impl From<EditError> for CommandError {
    fn from(error: EditError) -> Self {
        Self::Edit(error)
    }
}

impl Document {
    pub fn execute(&mut self, command: EditorCommand) -> Result<bool, CommandError> {
        match command {
            EditorCommand::InsertText(text) => self.insert_text(text),
            EditorCommand::DeleteBackward => self.delete_backward(),
            EditorCommand::DeleteWordBackward => self.delete_word_backward(),
            EditorCommand::DeleteLineBackward => self.delete_line_backward(),
            EditorCommand::DeleteForward => self.delete_forward(),
        }
    }

    fn insert_text(&mut self, text: String) -> Result<bool, CommandError> {
        let range = single_selection_range(self)?;
        let caret = range.start + text.len();

        self.apply(
            Transaction::replace(range, text).with_selection_after(Selection::caret(caret)),
        )?;

        Ok(true)
    }

    fn delete_backward(&mut self) -> Result<bool, CommandError> {
        let range = single_selection_range(self)?;

        if !range.is_empty() {
            return self.delete_range(range);
        }

        let Some(previous_offset) = self.previous_grapheme_offset(range.start)? else {
            return Ok(false);
        };

        self.delete_range(TextRange::new(previous_offset, range.start))
    }

    fn delete_word_backward(&mut self) -> Result<bool, CommandError> {
        let range = single_selection_range(self)?;

        if !range.is_empty() {
            return self.delete_range(range);
        }

        let line_index = self.line_index_at_offset(range.start)?;
        let line_start = self
            .line_range(line_index)
            .ok_or(EditError::InvalidRange)?
            .start;
        let previous_offset = self
            .previous_word_offset(range.start)?
            .map_or(line_start, |offset| offset.max(line_start));

        self.delete_range(TextRange::new(previous_offset, range.start))
    }

    fn delete_line_backward(&mut self) -> Result<bool, CommandError> {
        let range = single_selection_range(self)?;

        if !range.is_empty() {
            return self.delete_range(range);
        }

        let line_index = self.line_index_at_offset(range.start)?;
        let line_start = self
            .line_range(line_index)
            .ok_or(EditError::InvalidRange)?
            .start;

        self.delete_range(TextRange::new(line_start, range.start))
    }

    fn delete_range(&mut self, range: TextRange) -> Result<bool, CommandError> {
        if range.is_empty() {
            return Ok(false);
        }

        let caret = range.start;
        self.apply(Transaction::replace(range, "").with_selection_after(Selection::caret(caret)))?;

        Ok(true)
    }

    fn delete_forward(&mut self) -> Result<bool, CommandError> {
        let range = single_selection_range(self)?;

        if !range.is_empty() {
            return self.delete_range(range);
        }

        let Some(next_offset) = self.next_grapheme_offset(range.start)? else {
            return Ok(false);
        };

        self.delete_range(TextRange::new(range.start, next_offset))
    }
}

fn single_selection_range(document: &Document) -> Result<TextRange, CommandError> {
    let ranges = document.selection().ranges();

    if ranges.len() != 1 {
        return Err(CommandError::MultipleSelectionsUnsupported);
    }

    Ok(ranges[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_text_at_caret() {
        let mut document = Document::new("Hanji");

        assert_eq!(
            document
                .execute(EditorCommand::insert_text(" notes"))
                .unwrap(),
            true
        );

        assert_eq!(document.text(), " notesHanji");
        assert_eq!(document.selection().primary(), TextRange::caret(6));
    }

    #[test]
    fn replaces_selected_text_when_inserting() {
        let mut document = Document::new("Hanji draft");
        document
            .set_selection(Selection::single(TextRange::new(6, 11)))
            .unwrap();

        document
            .execute(EditorCommand::insert_text("notes"))
            .unwrap();

        assert_eq!(document.text(), "Hanji notes");
        assert_eq!(document.selection().primary(), TextRange::caret(11));
    }

    #[test]
    fn delete_backward_removes_previous_character() {
        let mut document = Document::new("한지");
        document
            .set_selection(Selection::caret("한지".len()))
            .unwrap();

        document.execute(EditorCommand::DeleteBackward).unwrap();

        assert_eq!(document.text(), "한");
        assert_eq!(document.selection().primary(), TextRange::caret("한".len()));
    }

    #[test]
    fn delete_backward_removes_previous_grapheme_cluster() {
        let mut document = Document::new("A🇰🇷");
        document
            .set_selection(Selection::caret("A🇰🇷".len()))
            .unwrap();

        document.execute(EditorCommand::DeleteBackward).unwrap();

        assert_eq!(document.text(), "A");
        assert_eq!(document.selection().primary(), TextRange::caret("A".len()));
    }

    #[test]
    fn delete_word_backward_uses_existing_word_boundary() {
        let mut document = Document::new("Capture quick note");
        document
            .set_selection(Selection::caret("Capture quick note".len()))
            .unwrap();

        document.execute(EditorCommand::DeleteWordBackward).unwrap();

        assert_eq!(document.text(), "Capture quick ");
        assert_eq!(
            document.selection().primary(),
            TextRange::caret("Capture quick ".len())
        );
    }

    #[test]
    fn delete_word_backward_stays_within_current_line() {
        let mut document = Document::new("First line\n  Second line");
        let caret = "First line\n  ".len();
        document.set_selection(Selection::caret(caret)).unwrap();

        document.execute(EditorCommand::DeleteWordBackward).unwrap();

        assert_eq!(document.text(), "First line\nSecond line");
        assert_eq!(
            document.selection().primary(),
            TextRange::caret("First line\n".len())
        );
    }

    #[test]
    fn delete_word_backward_preserves_grapheme_boundaries() {
        let mut document = Document::new("A🇰🇷B");
        document
            .set_selection(Selection::caret("A🇰🇷B".len()))
            .unwrap();

        document.execute(EditorCommand::DeleteWordBackward).unwrap();

        assert_eq!(document.text(), "A🇰🇷");
        assert_eq!(
            document.selection().primary(),
            TextRange::caret("A🇰🇷".len())
        );
    }

    #[test]
    fn delete_line_backward_removes_current_line_prefix() {
        let mut document = Document::new("First line\nSecond line");
        let caret = "First line\nSecond ".len();
        document.set_selection(Selection::caret(caret)).unwrap();

        document.execute(EditorCommand::DeleteLineBackward).unwrap();

        assert_eq!(document.text(), "First line\nline");
        assert_eq!(
            document.selection().primary(),
            TextRange::caret("First line\n".len())
        );
    }

    #[test]
    fn backward_unit_deletion_noops_at_line_start() {
        for command in [
            EditorCommand::DeleteWordBackward,
            EditorCommand::DeleteLineBackward,
        ] {
            let mut document = Document::new("First line\nSecond line");
            let caret = "First line\n".len();
            document.set_selection(Selection::caret(caret)).unwrap();

            assert!(!document.execute(command).unwrap());
            assert_eq!(document.text(), "First line\nSecond line");
            assert_eq!(document.selection().primary(), TextRange::caret(caret));
        }
    }

    #[test]
    fn backward_unit_deletion_removes_selection() {
        for command in [
            EditorCommand::DeleteWordBackward,
            EditorCommand::DeleteLineBackward,
        ] {
            let mut document = Document::new("Hanji notes");
            document
                .set_selection(Selection::single(TextRange::new(6, 11)))
                .unwrap();

            document.execute(command).unwrap();

            assert_eq!(document.text(), "Hanji ");
            assert_eq!(document.selection().primary(), TextRange::caret(6));
        }
    }

    #[test]
    fn delete_forward_removes_next_character() {
        let mut document = Document::new("한지");
        document.set_selection(Selection::caret(0)).unwrap();

        document.execute(EditorCommand::DeleteForward).unwrap();

        assert_eq!(document.text(), "지");
        assert_eq!(document.selection().primary(), TextRange::caret(0));
    }

    #[test]
    fn delete_forward_removes_next_grapheme_cluster() {
        let mut document = Document::new("🇰🇷B");
        document.set_selection(Selection::caret(0)).unwrap();

        document.execute(EditorCommand::DeleteForward).unwrap();

        assert_eq!(document.text(), "B");
        assert_eq!(document.selection().primary(), TextRange::caret(0));
    }

    #[test]
    fn deletion_noops_at_document_edges() {
        let mut document = Document::new("Hanji");

        assert!(!document.execute(EditorCommand::DeleteBackward).unwrap());
        assert_eq!(document.text(), "Hanji");

        document.set_selection(Selection::caret(5)).unwrap();

        assert!(!document.execute(EditorCommand::DeleteForward).unwrap());
        assert_eq!(document.text(), "Hanji");
    }

    #[test]
    fn deleting_selection_collapses_to_selection_start() {
        let mut document = Document::new("Hanji notes");
        document
            .set_selection(Selection::single(TextRange::new(5, 11)))
            .unwrap();

        document.execute(EditorCommand::DeleteBackward).unwrap();

        assert_eq!(document.text(), "Hanji");
        assert_eq!(document.selection().primary(), TextRange::caret(5));
    }

    #[test]
    fn command_edits_are_undoable() {
        let mut document = Document::new("Hanji");

        document
            .execute(EditorCommand::insert_text(" notes"))
            .unwrap();
        document.undo();

        assert_eq!(document.text(), "Hanji");
        assert_eq!(document.selection().primary(), TextRange::caret(0));
    }

    #[test]
    fn rejects_multiple_selections_for_now() {
        let mut document = Document::new("Hanji");
        document
            .set_selection(
                Selection::new(vec![TextRange::caret(0), TextRange::caret(5)], 0).unwrap(),
            )
            .unwrap();

        let error = document
            .execute(EditorCommand::insert_text("x"))
            .unwrap_err();

        assert_eq!(error, CommandError::MultipleSelectionsUnsupported);
        assert_eq!(document.text(), "Hanji");
    }
}
