#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    text: String,
}

impl Document {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn replace_all(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn apply(&mut self, transaction: Transaction) -> Result<(), EditError> {
        for edit in transaction.edits {
            self.replace_range(edit.range, &edit.text)?;
        }

        Ok(())
    }

    fn replace_range(&mut self, range: TextRange, text: &str) -> Result<(), EditError> {
        if range.start > range.end || range.end > self.text.len() {
            return Err(EditError::InvalidRange);
        }

        if !self.text.is_char_boundary(range.start) || !self.text.is_char_boundary(range.end) {
            return Err(EditError::InvalidBoundary);
        }

        self.text.replace_range(range.start..range.end, text);
        Ok(())
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub range: TextRange,
}

impl Selection {
    pub fn caret(offset: usize) -> Self {
        Self {
            range: TextRange::caret(offset),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    edits: Vec<TextEdit>,
}

impl Transaction {
    pub fn single(edit: TextEdit) -> Self {
        Self { edits: vec![edit] }
    }

    pub fn insert(offset: usize, text: impl Into<String>) -> Self {
        Self::single(TextEdit {
            range: TextRange::caret(offset),
            text: text.into(),
        })
    }

    pub fn replace(range: TextRange, text: impl Into<String>) -> Self {
        Self::single(TextEdit {
            range,
            text: text.into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: TextRange,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditError {
    InvalidBoundary,
    InvalidRange,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_insert_transaction() {
        let mut document = Document::new("Hanji");

        document.apply(Transaction::insert(5, " notes")).unwrap();

        assert_eq!(document.text(), "Hanji notes");
    }

    #[test]
    fn rejects_non_char_boundary_edits() {
        let mut document = Document::new("한지");

        let error = document
            .apply(Transaction::replace(TextRange::new(1, 1), "x"))
            .unwrap_err();

        assert_eq!(error, EditError::InvalidBoundary);
    }
}
