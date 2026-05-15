use crate::{Document, EditError, Selection, TextRange, Transaction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorCommand {
    InsertText(String),
    DeleteBackward,
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
            self.apply(
                Transaction::replace(range, "").with_selection_after(Selection::caret(range.start)),
            )?;
            return Ok(true);
        }

        let Some(previous_offset) = previous_char_offset(self.text(), range.start) else {
            return Ok(false);
        };

        let delete_range = TextRange::new(previous_offset, range.start);
        self.apply(
            Transaction::replace(delete_range, "")
                .with_selection_after(Selection::caret(previous_offset)),
        )?;

        Ok(true)
    }

    fn delete_forward(&mut self) -> Result<bool, CommandError> {
        let range = single_selection_range(self)?;

        if !range.is_empty() {
            self.apply(
                Transaction::replace(range, "").with_selection_after(Selection::caret(range.start)),
            )?;
            return Ok(true);
        }

        let Some(next_offset) = next_char_offset(self.text(), range.start) else {
            return Ok(false);
        };

        let delete_range = TextRange::new(range.start, next_offset);
        self.apply(
            Transaction::replace(delete_range, "")
                .with_selection_after(Selection::caret(range.start)),
        )?;

        Ok(true)
    }
}

fn single_selection_range(document: &Document) -> Result<TextRange, CommandError> {
    let ranges = document.selection().ranges();

    if ranges.len() != 1 {
        return Err(CommandError::MultipleSelectionsUnsupported);
    }

    Ok(ranges[0])
}

fn previous_char_offset(text: &str, offset: usize) -> Option<usize> {
    if offset == 0 {
        return None;
    }

    text[..offset]
        .char_indices()
        .last()
        .map(|(offset, _)| offset)
}

fn next_char_offset(text: &str, offset: usize) -> Option<usize> {
    if offset >= text.len() {
        return None;
    }

    text[offset..]
        .char_indices()
        .nth(1)
        .map(|(next_offset, _)| offset + next_offset)
        .or(Some(text.len()))
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
    fn delete_forward_removes_next_character() {
        let mut document = Document::new("한지");
        document.set_selection(Selection::caret(0)).unwrap();

        document.execute(EditorCommand::DeleteForward).unwrap();

        assert_eq!(document.text(), "지");
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
