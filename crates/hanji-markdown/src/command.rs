use hanji_core::{Document, EditError, Selection, TextEdit, TextRange, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownCommand {
    ToggleStrong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownCommandError {
    Edit(EditError),
    MultipleSelectionsUnsupported,
}

impl From<EditError> for MarkdownCommandError {
    fn from(error: EditError) -> Self {
        Self::Edit(error)
    }
}

pub fn execute_markdown_command(
    document: &mut Document,
    command: MarkdownCommand,
) -> Result<bool, MarkdownCommandError> {
    match command {
        MarkdownCommand::ToggleStrong => toggle_strong(document),
    }
}

pub fn toggle_strong(document: &mut Document) -> Result<bool, MarkdownCommandError> {
    let range = single_selection_range(document)?;

    if range.is_empty() {
        return Ok(false);
    }

    if let Some(unwrapped_range) = strong_range(document.text(), range) {
        document.apply(Transaction::new(
            vec![
                TextEdit::replace(
                    TextRange::new(unwrapped_range.end, unwrapped_range.end + 2),
                    "",
                ),
                TextEdit::replace(
                    TextRange::new(unwrapped_range.start - 2, unwrapped_range.start),
                    "",
                ),
            ],
            Some(Selection::single(TextRange::new(
                unwrapped_range.start - 2,
                unwrapped_range.end - 2,
            ))),
        ))?;
    } else {
        document.apply(Transaction::new(
            vec![
                TextEdit::insert(range.end, "**"),
                TextEdit::insert(range.start, "**"),
            ],
            Some(Selection::single(TextRange::new(
                range.start + 2,
                range.end + 2,
            ))),
        ))?;
    }

    Ok(true)
}

fn single_selection_range(document: &Document) -> Result<TextRange, MarkdownCommandError> {
    let ranges = document.selection().ranges();

    if ranges.len() != 1 {
        return Err(MarkdownCommandError::MultipleSelectionsUnsupported);
    }

    Ok(ranges[0])
}

fn strong_range(text: &str, range: TextRange) -> Option<TextRange> {
    let bytes = text.as_bytes();

    if range.start >= 2
        && range.end + 2 <= text.len()
        && bytes.get(range.start - 2..range.start) == Some(b"**")
        && bytes.get(range.end..range.end + 2) == Some(b"**")
    {
        return Some(range);
    }

    if range.len() >= 4
        && bytes.get(range.start..range.start + 2) == Some(b"**")
        && bytes.get(range.end - 2..range.end) == Some(b"**")
    {
        return Some(TextRange::new(range.start + 2, range.end - 2));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_strong_wraps_selection() {
        let mut document = Document::new("Hanji notes");
        document
            .set_selection(Selection::single(TextRange::new(6, 11)))
            .unwrap();

        toggle_strong(&mut document).unwrap();

        assert_eq!(document.text(), "Hanji **notes**");
        assert_eq!(document.selection().primary(), TextRange::new(8, 13));
    }

    #[test]
    fn toggle_strong_unwraps_when_selection_is_inside_markers() {
        let mut document = Document::new("Hanji **notes**");
        document
            .set_selection(Selection::single(TextRange::new(8, 13)))
            .unwrap();

        toggle_strong(&mut document).unwrap();

        assert_eq!(document.text(), "Hanji notes");
        assert_eq!(document.selection().primary(), TextRange::new(6, 11));
    }

    #[test]
    fn toggle_strong_unwraps_when_selection_includes_markers() {
        let mut document = Document::new("Hanji **notes**");
        document
            .set_selection(Selection::single(TextRange::new(6, 15)))
            .unwrap();

        toggle_strong(&mut document).unwrap();

        assert_eq!(document.text(), "Hanji notes");
        assert_eq!(document.selection().primary(), TextRange::new(6, 11));
    }

    #[test]
    fn toggle_strong_handles_korean_selection() {
        let mut document = Document::new("한지 노트");
        document
            .set_selection(Selection::single(TextRange::new(
                "한지 ".len(),
                "한지 노트".len(),
            )))
            .unwrap();

        toggle_strong(&mut document).unwrap();

        assert_eq!(document.text(), "한지 **노트**");
    }

    #[test]
    fn toggle_strong_noops_without_selection() {
        let mut document = Document::new("Hanji");

        assert!(!toggle_strong(&mut document).unwrap());
        assert_eq!(document.text(), "Hanji");
        assert_eq!(document.selection().primary(), TextRange::caret(0));
    }

    #[test]
    fn execute_markdown_command_dispatches_toggle_strong() {
        let mut document = Document::new("Hanji notes");
        document
            .set_selection(Selection::single(TextRange::new(6, 11)))
            .unwrap();

        execute_markdown_command(&mut document, MarkdownCommand::ToggleStrong).unwrap();

        assert_eq!(document.text(), "Hanji **notes**");
    }

    #[test]
    fn rejects_multiple_selections_for_now() {
        let mut document = Document::new("Hanji");
        document
            .set_selection(
                Selection::new(vec![TextRange::caret(0), TextRange::caret(5)], 0).unwrap(),
            )
            .unwrap();

        let error = toggle_strong(&mut document).unwrap_err();

        assert_eq!(error, MarkdownCommandError::MultipleSelectionsUnsupported);
        assert_eq!(document.text(), "Hanji");
    }
}
