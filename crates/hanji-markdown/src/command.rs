use hanji_core::{Document, EditError, Selection, TextEdit, TextRange, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownCommand {
    ToggleStrong,
    ToggleEmphasis,
    ToggleCode,
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
        MarkdownCommand::ToggleEmphasis => toggle_emphasis(document),
        MarkdownCommand::ToggleCode => toggle_code(document),
    }
}

pub fn toggle_strong(document: &mut Document) -> Result<bool, MarkdownCommandError> {
    toggle_delimited(document, "**")
}

pub fn toggle_emphasis(document: &mut Document) -> Result<bool, MarkdownCommandError> {
    toggle_delimited(document, "*")
}

pub fn toggle_code(document: &mut Document) -> Result<bool, MarkdownCommandError> {
    toggle_delimited(document, "`")
}

fn toggle_delimited(
    document: &mut Document,
    marker: &'static str,
) -> Result<bool, MarkdownCommandError> {
    let range = single_selection_range(document)?;

    if range.is_empty() {
        return Ok(false);
    }

    let marker_len = marker.len();

    if let Some(unwrapped_range) = delimited_range(document.text(), range, marker) {
        document.apply(Transaction::new(
            vec![
                TextEdit::replace(
                    TextRange::new(unwrapped_range.end, unwrapped_range.end + marker_len),
                    "",
                ),
                TextEdit::replace(
                    TextRange::new(unwrapped_range.start - marker_len, unwrapped_range.start),
                    "",
                ),
            ],
            Some(Selection::single(TextRange::new(
                unwrapped_range.start - marker_len,
                unwrapped_range.end - marker_len,
            ))),
        ))?;
    } else {
        document.apply(Transaction::new(
            vec![
                TextEdit::insert(range.end, marker),
                TextEdit::insert(range.start, marker),
            ],
            Some(Selection::single(TextRange::new(
                range.start + marker_len,
                range.end + marker_len,
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

fn delimited_range(text: &str, range: TextRange, marker: &str) -> Option<TextRange> {
    let bytes = text.as_bytes();
    let marker = marker.as_bytes();
    let marker_len = marker.len();

    if range.start >= marker_len
        && range.end + marker_len <= text.len()
        && bytes.get(range.start - marker_len..range.start) == Some(marker)
        && bytes.get(range.end..range.end + marker_len) == Some(marker)
    {
        return Some(range);
    }

    if range.len() >= marker_len * 2
        && bytes.get(range.start..range.start + marker_len) == Some(marker)
        && bytes.get(range.end - marker_len..range.end) == Some(marker)
    {
        return Some(TextRange::new(
            range.start + marker_len,
            range.end - marker_len,
        ));
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
    fn toggle_emphasis_wraps_selection() {
        let mut document = Document::new("Hanji notes");
        document
            .set_selection(Selection::single(TextRange::new(6, 11)))
            .unwrap();

        toggle_emphasis(&mut document).unwrap();

        assert_eq!(document.text(), "Hanji *notes*");
        assert_eq!(document.selection().primary(), TextRange::new(7, 12));
    }

    #[test]
    fn toggle_emphasis_unwraps_when_selection_is_inside_markers() {
        let mut document = Document::new("Hanji *notes*");
        document
            .set_selection(Selection::single(TextRange::new(7, 12)))
            .unwrap();

        toggle_emphasis(&mut document).unwrap();

        assert_eq!(document.text(), "Hanji notes");
        assert_eq!(document.selection().primary(), TextRange::new(6, 11));
    }

    #[test]
    fn toggle_emphasis_unwraps_when_selection_includes_markers() {
        let mut document = Document::new("Hanji *notes*");
        document
            .set_selection(Selection::single(TextRange::new(6, 13)))
            .unwrap();

        toggle_emphasis(&mut document).unwrap();

        assert_eq!(document.text(), "Hanji notes");
        assert_eq!(document.selection().primary(), TextRange::new(6, 11));
    }

    #[test]
    fn execute_markdown_command_dispatches_toggle_emphasis() {
        let mut document = Document::new("Hanji notes");
        document
            .set_selection(Selection::single(TextRange::new(6, 11)))
            .unwrap();

        execute_markdown_command(&mut document, MarkdownCommand::ToggleEmphasis).unwrap();

        assert_eq!(document.text(), "Hanji *notes*");
    }

    #[test]
    fn toggle_code_wraps_selection() {
        let mut document = Document::new("Use code");
        document
            .set_selection(Selection::single(TextRange::new(4, 8)))
            .unwrap();

        toggle_code(&mut document).unwrap();

        assert_eq!(document.text(), "Use `code`");
        assert_eq!(document.selection().primary(), TextRange::new(5, 9));
    }

    #[test]
    fn toggle_code_unwraps_when_selection_is_inside_markers() {
        let mut document = Document::new("Use `code`");
        document
            .set_selection(Selection::single(TextRange::new(5, 9)))
            .unwrap();

        toggle_code(&mut document).unwrap();

        assert_eq!(document.text(), "Use code");
        assert_eq!(document.selection().primary(), TextRange::new(4, 8));
    }

    #[test]
    fn toggle_code_unwraps_when_selection_includes_markers() {
        let mut document = Document::new("Use `code`");
        document
            .set_selection(Selection::single(TextRange::new(4, 10)))
            .unwrap();

        toggle_code(&mut document).unwrap();

        assert_eq!(document.text(), "Use code");
        assert_eq!(document.selection().primary(), TextRange::new(4, 8));
    }

    #[test]
    fn toggle_code_handles_korean_selection() {
        let mut document = Document::new("한지 코드");
        document
            .set_selection(Selection::single(TextRange::new(
                "한지 ".len(),
                "한지 코드".len(),
            )))
            .unwrap();

        toggle_code(&mut document).unwrap();

        assert_eq!(document.text(), "한지 `코드`");
    }

    #[test]
    fn toggle_code_handles_emoji_selection() {
        let text = "Flag 🇰🇷";
        let mut document = Document::new(text);
        let selection_start = "Flag ".len();
        let selection_end = text.len();
        document
            .set_selection(Selection::single(TextRange::new(
                selection_start,
                selection_end,
            )))
            .unwrap();

        toggle_code(&mut document).unwrap();

        assert_eq!(document.text(), "Flag `🇰🇷`");
        assert_eq!(
            document.selection().primary(),
            TextRange::new(selection_start + 1, selection_end + 1)
        );
    }

    #[test]
    fn toggle_code_noops_without_selection() {
        let mut document = Document::new("Hanji");

        assert!(!toggle_code(&mut document).unwrap());
        assert_eq!(document.text(), "Hanji");
        assert_eq!(document.selection().primary(), TextRange::caret(0));
    }

    #[test]
    fn execute_markdown_command_dispatches_toggle_code() {
        let mut document = Document::new("Use code");
        document
            .set_selection(Selection::single(TextRange::new(4, 8)))
            .unwrap();

        execute_markdown_command(&mut document, MarkdownCommand::ToggleCode).unwrap();

        assert_eq!(document.text(), "Use `code`");
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
