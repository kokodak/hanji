use hanji_core::{Document, EditError, Selection, TextEdit, TextRange, Transaction};

use crate::projection::{MarkdownInline, MarkdownMarkerRanges, project_document};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownCommand {
    ToggleStrong,
    ToggleEmphasis,
    ToggleCode,
    InsertLink,
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
        MarkdownCommand::InsertLink => insert_link(document),
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

pub fn insert_link(document: &mut Document) -> Result<bool, MarkdownCommandError> {
    let range = single_selection_range(document)?;

    if let Some(destination) = existing_link_destination_range(document.text(), range) {
        document.set_selection(Selection::single(destination))?;
        return Ok(true);
    }

    if range.is_empty() {
        return Ok(false);
    }

    let Some((replacement, selection_after)) = link_replacement(document.text(), range) else {
        return Ok(false);
    };
    document.apply(Transaction::new(
        vec![TextEdit::replace(range, replacement)],
        Some(Selection::single(selection_after)),
    ))?;

    Ok(true)
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
        let Some((replacement, selection_after)) =
            wrapped_selection_replacement(document.text(), range, marker)
        else {
            return Ok(false);
        };
        document.apply(Transaction::new(
            vec![TextEdit::replace(range, replacement)],
            Some(Selection::single(selection_after)),
        ))?;
    }

    Ok(true)
}

fn wrapped_selection_replacement(
    text: &str,
    range: TextRange,
    marker: &str,
) -> Option<(String, TextRange)> {
    if range.is_empty() {
        return None;
    }

    let selected = text.get(range.start..range.end)?;
    let content = if should_flatten_inner_markers(marker) {
        flatten_inner_markers(selected, marker)
    } else {
        selected.to_string()
    };
    let replacement = format!("{marker}{content}{marker}");
    let selection_start = range.start + marker.len();
    let selection_end = selection_start + content.len();

    Some((replacement, TextRange::new(selection_start, selection_end)))
}

fn should_flatten_inner_markers(marker: &str) -> bool {
    matches!(marker, "*" | "**" | "***" | "~~")
}

fn flatten_inner_markers(source: &str, marker: &str) -> String {
    let document = Document::new(source);
    let projection = project_document(&document);
    let mut ranges = Vec::new();

    for line in projection.lines() {
        for markers in line.style_marker_ranges() {
            push_matching_marker_ranges(source, marker, markers, &mut ranges);
        }

        for inline in &line.inlines {
            if let Some(markers) = inline_marker_ranges(inline.kind, marker) {
                push_matching_marker_ranges(source, marker, markers, &mut ranges);
            }

            if let Some(markers) = inline.style_markers {
                push_matching_marker_ranges(source, marker, markers, &mut ranges);
            }
        }
    }

    remove_ranges(source, ranges)
}

fn inline_marker_ranges(kind: MarkdownInline, marker: &str) -> Option<MarkdownMarkerRanges> {
    match (marker, kind) {
        ("*", MarkdownInline::Emphasis { markers })
        | ("**", MarkdownInline::Strong { markers })
        | ("~~", MarkdownInline::Strikethrough { markers }) => Some(markers),
        _ => None,
    }
}

fn push_matching_marker_ranges(
    source: &str,
    marker: &str,
    markers: MarkdownMarkerRanges,
    ranges: &mut Vec<TextRange>,
) {
    if source.get(markers.opening.start..markers.opening.end) == Some(marker)
        && source.get(markers.closing.start..markers.closing.end) == Some(marker)
    {
        ranges.push(markers.opening);
        ranges.push(markers.closing);
    }
}

fn remove_ranges(source: &str, mut ranges: Vec<TextRange>) -> String {
    if ranges.is_empty() {
        return source.to_string();
    }

    ranges.sort_by_key(|range| (range.start, range.end));
    ranges.dedup();

    let mut text = String::with_capacity(source.len());
    let mut copied_until = 0;
    for range in ranges {
        if range.start < copied_until {
            continue;
        }

        text.push_str(&source[copied_until..range.start]);
        copied_until = range.end;
    }
    text.push_str(&source[copied_until..]);

    text
}

fn existing_link_destination_range(text: &str, range: TextRange) -> Option<TextRange> {
    let document = Document::new(text);
    let projection = project_document(&document);

    for line in projection.lines() {
        for inline in &line.inlines {
            let MarkdownInline::Link { markers } = inline.kind else {
                continue;
            };
            let link_range = TextRange::new(markers.opening.start, markers.closing.end);
            if range_targets_source(range, link_range) {
                return Some(markers.destination);
            }
        }
    }

    None
}

fn range_targets_source(range: TextRange, source: TextRange) -> bool {
    if range.is_empty() {
        source.start <= range.start && range.start <= source.end
    } else {
        source.start <= range.start && range.end <= source.end
    }
}

fn link_replacement(text: &str, range: TextRange) -> Option<(String, TextRange)> {
    const PLACEHOLDER_DESTINATION: &str = "https://";

    let selected = text.get(range.start..range.end)?;
    if selected.contains('\n') {
        return None;
    }

    let replacement = format!("[{selected}]({PLACEHOLDER_DESTINATION})");
    let destination_start = range.start + "[".len() + selected.len() + "](".len();
    let destination_end = destination_start + PLACEHOLDER_DESTINATION.len();

    Some((
        replacement,
        TextRange::new(destination_start, destination_end),
    ))
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
    fn toggle_strong_flattens_inner_strong_spans() {
        let source = "Capture **the** **thought** with Hanji.";
        let expected = "**Capture the thought with Hanji.**";
        let mut document = Document::new(source);
        document
            .set_selection(Selection::single(TextRange::new(0, source.len())))
            .unwrap();

        toggle_strong(&mut document).unwrap();

        assert_eq!(document.text(), expected);
        assert_eq!(
            document.selection().primary(),
            TextRange::new("**".len(), expected.len() - "**".len())
        );
    }

    #[test]
    fn toggle_strong_flattens_inner_strong_spans_without_touching_code() {
        let source = "Capture **the** **thought** with `Hanji` and `**literal**`.";
        let expected = "**Capture the thought with `Hanji` and `**literal**`.**";
        let mut document = Document::new(source);
        document
            .set_selection(Selection::single(TextRange::new(0, source.len())))
            .unwrap();

        toggle_strong(&mut document).unwrap();

        assert_eq!(document.text(), expected);
        assert_eq!(
            document.selection().primary(),
            TextRange::new("**".len(), expected.len() - "**".len())
        );
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
    fn toggle_emphasis_flattens_inner_emphasis_spans() {
        let source = "Capture *the* *thought* with Hanji.";
        let expected = "*Capture the thought with Hanji.*";
        let mut document = Document::new(source);
        document
            .set_selection(Selection::single(TextRange::new(0, source.len())))
            .unwrap();

        toggle_emphasis(&mut document).unwrap();

        assert_eq!(document.text(), expected);
        assert_eq!(
            document.selection().primary(),
            TextRange::new("*".len(), expected.len() - "*".len())
        );
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
    fn insert_link_wraps_selection_and_selects_destination_placeholder() {
        let mut document = Document::new("Read Hanji");
        document
            .set_selection(Selection::single(TextRange::new(5, 10)))
            .unwrap();

        insert_link(&mut document).unwrap();

        assert_eq!(document.text(), "Read [Hanji](https://)");
        assert_eq!(
            document.selection().primary(),
            TextRange::new("Read [Hanji](".len(), "Read [Hanji](https://".len())
        );
    }

    #[test]
    fn insert_link_handles_korean_selection() {
        let text = "Read 한지";
        let mut document = Document::new(text);
        document
            .set_selection(Selection::single(TextRange::new("Read ".len(), text.len())))
            .unwrap();

        insert_link(&mut document).unwrap();

        assert_eq!(document.text(), "Read [한지](https://)");
    }

    #[test]
    fn insert_link_selects_existing_link_destination_without_changing_text() {
        let source = "Read [Hanji](https://hanji.local) today";
        let mut document = Document::new(source);
        document
            .set_selection(Selection::single(TextRange::new(
                "Read [".len(),
                "Read [Hanji".len(),
            )))
            .unwrap();

        insert_link(&mut document).unwrap();

        assert_eq!(document.text(), source);
        assert_eq!(
            document.selection().primary(),
            TextRange::new(
                "Read [Hanji](".len(),
                "Read [Hanji](https://hanji.local".len()
            )
        );
    }

    #[test]
    fn insert_link_noops_without_selection_or_for_multiline_selection() {
        let mut document = Document::new("Read Hanji");
        document.set_selection(Selection::caret(5)).unwrap();

        assert!(!insert_link(&mut document).unwrap());
        assert_eq!(document.text(), "Read Hanji");
        assert_eq!(document.selection().primary(), TextRange::caret(5));

        let mut multiline = Document::new("Read\nHanji");
        multiline
            .set_selection(Selection::single(TextRange::new(0, "Read\nHanji".len())))
            .unwrap();

        assert!(!insert_link(&mut multiline).unwrap());
        assert_eq!(multiline.text(), "Read\nHanji");
    }

    #[test]
    fn execute_markdown_command_dispatches_insert_link() {
        let mut document = Document::new("Read Hanji");
        document
            .set_selection(Selection::single(TextRange::new(5, 10)))
            .unwrap();

        execute_markdown_command(&mut document, MarkdownCommand::InsertLink).unwrap();

        assert_eq!(document.text(), "Read [Hanji](https://)");
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
