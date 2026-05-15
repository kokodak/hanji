use crate::{Selection, TextEdit, TextRange};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    edits: Vec<TextEdit>,
    selection_after: Option<Selection>,
}

impl Transaction {
    pub fn new(edits: Vec<TextEdit>, selection_after: Option<Selection>) -> Self {
        Self {
            edits,
            selection_after,
        }
    }

    pub fn single(edit: TextEdit) -> Self {
        let selection_after = selection_after_for_edit(&edit);

        Self {
            edits: vec![edit],
            selection_after: Some(selection_after),
        }
    }

    pub fn insert(offset: usize, text: impl Into<String>) -> Self {
        Self::single(TextEdit::insert(offset, text))
    }

    pub fn replace(range: TextRange, text: impl Into<String>) -> Self {
        Self::single(TextEdit::replace(range, text))
    }

    pub fn with_selection_after(mut self, selection: Selection) -> Self {
        self.selection_after = Some(selection);
        self
    }

    pub fn edits(&self) -> &[TextEdit] {
        &self.edits
    }

    pub fn selection_after(&self) -> Option<&Selection> {
        self.selection_after.as_ref()
    }

    pub fn into_parts(self) -> (Vec<TextEdit>, Option<Selection>) {
        (self.edits, self.selection_after)
    }
}

fn selection_after_for_edit(edit: &TextEdit) -> Selection {
    Selection::caret(edit.range.start + edit.text.len())
}
