use std::ops::Range;

use gpui::{Bounds, Pixels};
use hanji_core::{TextEdit, TextRange};
use hanji_markdown::{
    MarkdownListMarker, MarkdownTaskState, OrderedListDelimiter, blockquote_content_start,
    list_item,
};

const DRAG_SELECTION_THRESHOLD: f64 = 2.0;
const LIST_INDENT_WIDTH: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListIndentDirection {
    Increase,
    Decrease,
}

pub(crate) fn selection_range_from_anchor_and_head(anchor: usize, head: usize) -> TextRange {
    TextRange::new(anchor.min(head), anchor.max(head))
}

pub(crate) fn document_selection_range(document_len: usize) -> TextRange {
    TextRange::new(0, document_len)
}

pub(crate) fn selected_source_text(text: &str, range: &Range<usize>) -> Option<String> {
    if range.start == range.end {
        return None;
    }

    text.get(range.clone()).map(ToString::to_string)
}

pub(crate) fn clipboard_paste_edit(
    range: &Range<usize>,
    text: &str,
) -> (Range<usize>, String, Range<usize>) {
    let caret = range.start + text.len();

    (range.clone(), text.to_string(), caret..caret)
}

pub(crate) fn selection_is_reversed(anchor: usize, head: usize) -> bool {
    head < anchor
}

pub(crate) fn extension_points_for_selection(
    selection: TextRange,
    direction: isize,
) -> (usize, usize) {
    if selection.is_empty() {
        (selection.start, selection.start)
    } else if direction < 0 {
        (selection.end, selection.start)
    } else {
        (selection.start, selection.end)
    }
}

pub(crate) fn horizontal_offset_within_line(
    line_range: TextRange,
    origin: usize,
    candidate: usize,
    direction: isize,
) -> Option<usize> {
    if direction < 0 {
        if candidate < line_range.start {
            return (origin > line_range.start).then_some(line_range.start);
        }
    } else if candidate > line_range.end {
        return (origin < line_range.end).then_some(line_range.end);
    }

    Some(candidate)
}

pub(crate) fn blockquote_newline_edit_for_line(
    line_source: &str,
    line_range: TextRange,
    range: &Range<usize>,
) -> Option<(Range<usize>, String, Range<usize>)> {
    if range.start != range.end {
        return None;
    }

    let content_start = blockquote_content_start(line_source)?;
    let marker = &line_source[..content_start];
    let content = &line_source[content_start..];

    if content.trim().is_empty() {
        let caret = line_range.start;
        return Some((
            line_range.start..line_range.end,
            String::new(),
            caret..caret,
        ));
    }

    let replacement = format!("\n{marker}");
    let caret = range.start + replacement.len();

    Some((range.clone(), replacement, caret..caret))
}

pub(crate) fn list_newline_edit_for_line(
    line_source: &str,
    line_range: TextRange,
    range: &Range<usize>,
) -> Option<(Range<usize>, String, Range<usize>)> {
    if range.start != range.end {
        return None;
    }

    let list_item = list_item(line_source)?;
    let content = &line_source[list_item.content_start..];

    if content.trim().is_empty() {
        let indent_len = list_indent_len(line_source);
        if indent_len > 0 {
            let remove_len = indent_len.min(LIST_INDENT_WIDTH);
            let caret = range.start.saturating_sub(remove_len);
            return Some((
                line_range.start..line_range.start + remove_len,
                String::new(),
                caret..caret,
            ));
        }

        let caret = line_range.start;
        return Some((
            line_range.start..line_range.end,
            String::new(),
            caret..caret,
        ));
    }

    let indent_len = line_source.bytes().take_while(|byte| *byte == b' ').count();
    let marker = next_list_item_marker_text(list_item.marker, list_item.task);
    let replacement = format!("\n{}{marker} ", &line_source[..indent_len]);
    let caret = range.start + replacement.len();

    Some((range.clone(), replacement, caret..caret))
}

pub(crate) fn list_indent_edit(
    text: &str,
    range: &Range<usize>,
    direction: ListIndentDirection,
) -> Option<(Vec<TextEdit>, Range<usize>)> {
    let edits = selected_list_line_ranges(text, range)
        .into_iter()
        .filter_map(|line_range| list_indent_edit_for_line(text, line_range, direction))
        .collect::<Vec<_>>();

    if edits.is_empty() {
        return None;
    }

    let selection_after = transform_range_after_edits(range, &edits);
    Some((edits, selection_after))
}

fn selected_list_line_ranges(text: &str, range: &Range<usize>) -> Vec<TextRange> {
    let effective_end = effective_line_selection_end(text, range);
    let mut line_ranges = Vec::new();
    let mut line_start = 0;

    loop {
        let line_end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |offset| line_start + offset);

        if line_end >= range.start && line_start <= effective_end {
            line_ranges.push(TextRange::new(line_start, line_end));
        }

        if line_end == text.len() || line_start > effective_end {
            break;
        }

        line_start = line_end + 1;
    }

    line_ranges
}

fn effective_line_selection_end(text: &str, range: &Range<usize>) -> usize {
    if range.start == range.end {
        return range.end;
    }

    if range.end > 0 && text.as_bytes().get(range.end - 1) == Some(&b'\n') {
        range.end - 1
    } else {
        range.end
    }
}

fn list_indent_edit_for_line(
    text: &str,
    line_range: TextRange,
    direction: ListIndentDirection,
) -> Option<TextEdit> {
    let line_source = text.get(line_range.start..line_range.end)?;
    list_item(line_source)?;

    match direction {
        ListIndentDirection::Increase => Some(TextEdit::insert(
            line_range.start,
            " ".repeat(LIST_INDENT_WIDTH),
        )),
        ListIndentDirection::Decrease => {
            let remove_len = list_indent_len(line_source).min(LIST_INDENT_WIDTH);
            if remove_len == 0 {
                return None;
            }

            Some(TextEdit::replace(
                TextRange::new(line_range.start, line_range.start + remove_len),
                "",
            ))
        }
    }
}

fn list_indent_len(line_source: &str) -> usize {
    line_source.bytes().take_while(|byte| *byte == b' ').count()
}

fn transform_range_after_edits(range: &Range<usize>, edits: &[TextEdit]) -> Range<usize> {
    transform_offset_after_edits(range.start, edits)..transform_offset_after_edits(range.end, edits)
}

fn transform_offset_after_edits(offset: usize, edits: &[TextEdit]) -> usize {
    let mut transformed = offset;

    for edit in edits {
        let removed_len = edit.range.end - edit.range.start;
        let inserted_len = edit.text.len();
        if removed_len == 0 {
            if edit.range.start <= offset {
                transformed += inserted_len;
            }
        } else if edit.range.end <= offset {
            transformed -= removed_len.saturating_sub(inserted_len);
        } else if edit.range.start < offset {
            transformed = transformed.saturating_sub(offset - edit.range.start);
        }
    }

    transformed
}

pub(crate) fn marker_autocomplete_edit(
    text: &str,
    range: &Range<usize>,
    new_text: &str,
) -> Option<(Range<usize>, String, Range<usize>)> {
    if range.start != range.end {
        return marker_wrap_selection_edit(text, range, new_text);
    }

    code_fence_autocomplete_edit(text, range, new_text)
        .or_else(|| strong_marker_autocomplete_edit(text, range, new_text))
}

pub(crate) fn marker_skip_offset(
    text: &str,
    range: &Range<usize>,
    new_text: &str,
) -> Option<usize> {
    if range.start != range.end || !matches!(new_text, "*" | "**") {
        return None;
    }

    let marker = "**";
    if !text
        .get(range.start..)
        .is_some_and(|suffix| suffix.starts_with(marker))
        || !has_unclosed_marker_before(text, range.start, marker)
    {
        return None;
    }

    Some(range.start + marker.len())
}

pub(crate) fn empty_marker_pair_delete_backward_edit(
    text: &str,
    range: &Range<usize>,
) -> Option<(Range<usize>, String, Range<usize>)> {
    if range.start != range.end {
        return None;
    }

    let marker = "**";
    let opening_start = range.start.checked_sub(marker.len())?;
    let closing_end = range.start + marker.len();
    if text.get(opening_start..range.start) != Some(marker)
        || text.get(range.start..closing_end) != Some(marker)
        || marker_count_before(text, opening_start, marker) % 2 != 0
    {
        return None;
    }

    Some((
        opening_start..closing_end,
        String::new(),
        opening_start..opening_start,
    ))
}

fn marker_wrap_selection_edit(
    text: &str,
    range: &Range<usize>,
    new_text: &str,
) -> Option<(Range<usize>, String, Range<usize>)> {
    match new_text {
        "**" => wrap_selection_with_marker(text, range, "**"),
        "```" => wrap_selection_with_fence(text, range, "```"),
        "~~~" => wrap_selection_with_fence(text, range, "~~~"),
        _ => None,
    }
}

fn wrap_selection_with_marker(
    text: &str,
    range: &Range<usize>,
    marker: &str,
) -> Option<(Range<usize>, String, Range<usize>)> {
    let selected = text.get(range.clone())?;
    let marker_len = marker.len();
    let replacement = format!("{marker}{selected}{marker}");
    let selection_start = range.start + marker_len;
    let selection_end = selection_start + selected.len();

    Some((range.clone(), replacement, selection_start..selection_end))
}

fn wrap_selection_with_fence(
    text: &str,
    range: &Range<usize>,
    marker: &str,
) -> Option<(Range<usize>, String, Range<usize>)> {
    let selected = text.get(range.clone())?;
    let prefix = format!("{marker}\n");
    let suffix = format!("\n{marker}");
    let replacement = format!("{prefix}{selected}{suffix}");
    let selection_start = range.start + prefix.len();
    let selection_end = selection_start + selected.len();

    Some((range.clone(), replacement, selection_start..selection_end))
}

fn code_fence_autocomplete_edit(
    text: &str,
    range: &Range<usize>,
    new_text: &str,
) -> Option<(Range<usize>, String, Range<usize>)> {
    if !matches!(new_text, "`" | "```" | "~" | "~~~") {
        return None;
    }

    let (line_start, line_end) = line_bounds_for_offset(text, range.start);
    let line_prefix = text.get(line_start..range.start)?;
    let line_suffix = text.get(range.start..line_end)?;
    if !line_suffix.is_empty() {
        return None;
    }
    if has_unclosed_code_fence_before(text, line_start) {
        return None;
    }

    let (pending_prefix, closing_marker) = match new_text {
        "`" => ("``", "```"),
        "```" => ("", "```"),
        "~" => ("~~", "~~~"),
        "~~~" => ("", "~~~"),
        _ => return None,
    };
    let indent = fence_indent_for_pending_prefix(line_prefix, pending_prefix)?;
    let replacement = format!("{new_text}\n\n{indent}{closing_marker}");
    let caret = range.start + new_text.len() + "\n".len();

    Some((range.clone(), replacement, caret..caret))
}

fn fence_indent_for_pending_prefix<'a>(line_prefix: &'a str, marker: &str) -> Option<&'a str> {
    let indent_len = line_prefix
        .bytes()
        .take_while(|byte| *byte == b' ')
        .take(4)
        .count();
    if indent_len >= 4 {
        return None;
    }

    let indent = &line_prefix[..indent_len];
    (&line_prefix[indent_len..] == marker).then_some(indent)
}

fn strong_marker_autocomplete_edit(
    text: &str,
    range: &Range<usize>,
    new_text: &str,
) -> Option<(Range<usize>, String, Range<usize>)> {
    let (replacement, caret) = match new_text {
        "*" => {
            let marker_start = range.start.checked_sub("*".len())?;
            if !previous_text_is_exact_marker(text, range.start, "*")
                || would_close_unclosed_strong_at(text, marker_start)
                || following_text_starts_with(text, range.start, "*")
            {
                return None;
            }

            ("***".to_string(), range.start + 1)
        }
        "**" => {
            if previous_text_is_exact_marker(text, range.start, "*")
                || would_close_unclosed_strong_at(text, range.start)
                || following_text_starts_with(text, range.start, "*")
            {
                return None;
            }

            ("****".to_string(), range.start + 2)
        }
        _ => return None,
    };

    Some((range.clone(), replacement, caret..caret))
}

fn would_close_unclosed_strong_at(text: &str, marker_start: usize) -> bool {
    has_unclosed_marker_before(text, marker_start, "**")
        && previous_char_is_inline_content(text, marker_start)
}

fn previous_char_is_inline_content(text: &str, offset: usize) -> bool {
    text.get(..offset)
        .and_then(|prefix| prefix.chars().next_back())
        .is_some_and(|character| !character.is_whitespace())
}

fn previous_text_is_exact_marker(text: &str, offset: usize, marker: &str) -> bool {
    let Some(marker_start) = offset.checked_sub(marker.len()) else {
        return false;
    };
    if text.get(marker_start..offset) != Some(marker) {
        return false;
    }

    marker_start
        .checked_sub(marker.len())
        .and_then(|previous_start| text.get(previous_start..marker_start))
        != Some(marker)
}

fn following_text_starts_with(text: &str, offset: usize, marker: &str) -> bool {
    text.get(offset..)
        .is_some_and(|suffix| suffix.starts_with(marker))
}

fn has_unclosed_marker_before(text: &str, offset: usize, marker: &str) -> bool {
    marker_count_before(text, offset, marker) % 2 == 1
}

fn marker_count_before(text: &str, offset: usize, marker: &str) -> usize {
    text.get(..offset)
        .map_or(0, |prefix| prefix.match_indices(marker).count())
}

fn has_unclosed_code_fence_before(text: &str, offset: usize) -> bool {
    let Some(prefix) = text.get(..offset) else {
        return false;
    };
    let mut open_fence: Option<CodeFence> = None;

    for line in prefix.split('\n') {
        if let Some(fence) = open_fence {
            if is_closing_code_fence(line, fence.marker, fence.marker_len) {
                open_fence = None;
            }
        } else if let Some(fence) = opening_code_fence(line) {
            open_fence = Some(fence);
        }
    }

    open_fence.is_some()
}

#[derive(Clone, Copy)]
struct CodeFence {
    marker: CodeFenceMarker,
    marker_len: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CodeFenceMarker {
    Backtick,
    Tilde,
}

impl CodeFenceMarker {
    fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            b'`' => Some(Self::Backtick),
            b'~' => Some(Self::Tilde),
            _ => None,
        }
    }

    fn byte(self) -> u8 {
        match self {
            Self::Backtick => b'`',
            Self::Tilde => b'~',
        }
    }
}

fn opening_code_fence(line: &str) -> Option<CodeFence> {
    code_fence(line, 3, false, None)
}

fn is_closing_code_fence(line: &str, marker: CodeFenceMarker, min_marker_len: usize) -> bool {
    code_fence(line, min_marker_len, true, Some(marker)).is_some()
}

fn code_fence(
    line: &str,
    min_marker_len: usize,
    require_trailing_whitespace: bool,
    required_marker: Option<CodeFenceMarker>,
) -> Option<CodeFence> {
    let indent = line
        .bytes()
        .take_while(|byte| *byte == b' ')
        .take(4)
        .count();
    if indent >= 4 {
        return None;
    }

    let content = &line[indent..];
    let marker = required_marker.or_else(|| {
        content
            .as_bytes()
            .first()
            .copied()
            .and_then(CodeFenceMarker::from_byte)
    })?;
    let marker_len = content
        .bytes()
        .take_while(|byte| *byte == marker.byte())
        .count();
    if marker_len < min_marker_len {
        return None;
    }

    if require_trailing_whitespace
        && !content[marker_len..]
            .bytes()
            .all(|byte| matches!(byte, b' ' | b'\t'))
    {
        return None;
    }

    Some(CodeFence { marker, marker_len })
}

fn line_bounds_for_offset(text: &str, offset: usize) -> (usize, usize) {
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line_end = text[offset..]
        .find('\n')
        .map_or(text.len(), |index| offset + index);

    (line_start, line_end)
}

fn next_list_item_marker_text(
    marker: MarkdownListMarker,
    task: Option<MarkdownTaskState>,
) -> String {
    let marker = next_list_marker_text(marker);
    if task.is_some() {
        format!("{marker} [ ]")
    } else {
        marker
    }
}

fn next_list_marker_text(marker: MarkdownListMarker) -> String {
    match marker {
        MarkdownListMarker::Unordered { marker } => marker.to_string(),
        MarkdownListMarker::Ordered { number, delimiter } => {
            format!(
                "{}{}",
                number.saturating_add(1),
                ordered_list_delimiter(delimiter)
            )
        }
    }
}

pub(crate) fn ordered_list_delimiter(delimiter: OrderedListDelimiter) -> &'static str {
    match delimiter {
        OrderedListDelimiter::Dot => ".",
        OrderedListDelimiter::Paren => ")",
    }
}

pub(crate) fn drag_distance_exceeds_threshold(
    origin: gpui::Point<Pixels>,
    position: gpui::Point<Pixels>,
) -> bool {
    (position - origin).magnitude() > DRAG_SELECTION_THRESHOLD
}

#[derive(Clone, Copy)]
pub(crate) enum MarkerHitMode {
    ContentStart,
    MarkerStart,
}

pub(crate) fn line_marker_hit_offset(
    line_left: Pixels,
    marker_range: Option<TextRange>,
    position_x: Pixels,
    mode: MarkerHitMode,
) -> Option<usize> {
    if position_x < line_left {
        marker_range.map(|range| match mode {
            MarkerHitMode::ContentStart => range.end,
            MarkerHitMode::MarkerStart => range.start,
        })
    } else {
        None
    }
}

pub(crate) fn bounds_contains_point(bounds: Bounds<Pixels>, position: gpui::Point<Pixels>) -> bool {
    position.x >= bounds.left()
        && position.x <= bounds.right()
        && position.y >= bounds.top()
        && position.y <= bounds.bottom()
}

pub(crate) fn task_marker_state_char_range(
    text: &str,
    marker_range: TextRange,
) -> Option<Range<usize>> {
    let marker_source = text.get(marker_range.start..marker_range.end)?;
    let marker_offset = marker_source
        .find("[ ]")
        .or_else(|| marker_source.find("[x]"))
        .or_else(|| marker_source.find("[X]"))?;
    let state_offset = marker_range.start + marker_offset + 1;

    Some(state_offset..state_offset + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px};

    #[test]
    fn selection_helpers_keep_anchor_direction_separate_from_range_order() {
        assert_eq!(
            selection_range_from_anchor_and_head(16, 8),
            TextRange::new(8, 16)
        );
        assert!(selection_is_reversed(16, 8));
        assert_eq!(
            extension_points_for_selection(TextRange::new(8, 16), -1),
            (16, 8)
        );
        assert_eq!(
            extension_points_for_selection(TextRange::new(8, 16), 1),
            (8, 16)
        );
    }

    #[test]
    fn document_selection_range_covers_entire_source() {
        assert_eq!(document_selection_range(0), TextRange::new(0, 0));
        assert_eq!(document_selection_range(12), TextRange::new(0, 12));
    }

    #[test]
    fn selected_source_text_copies_raw_markdown_source() {
        assert_eq!(
            selected_source_text("A **bold** word", &(2..10)),
            Some("**bold**".to_string())
        );
    }

    #[test]
    fn selected_source_text_ignores_empty_and_invalid_ranges() {
        assert_eq!(selected_source_text("A **bold** word", &(2..2)), None);
        assert_eq!(selected_source_text("A **bold** word", &(1..99)), None);
    }

    #[test]
    fn clipboard_paste_edit_preserves_text_without_autocomplete() {
        assert_eq!(
            clipboard_paste_edit(&(4..4), "**"),
            (4..4, "**".to_string(), 6..6)
        );
        assert_eq!(
            clipboard_paste_edit(&(4..4), "```"),
            (4..4, "```".to_string(), 7..7)
        );
    }

    #[test]
    fn clipboard_paste_edit_replaces_selection_and_keeps_newlines() {
        assert_eq!(
            clipboard_paste_edit(&(2..6), "a\nb"),
            (2..6, "a\nb".to_string(), 5..5)
        );
    }

    #[test]
    fn horizontal_offsets_do_not_cross_line_boundaries() {
        let first_line = TextRange::new(0, 5);
        let second_line = TextRange::new(6, 10);

        assert_eq!(horizontal_offset_within_line(first_line, 4, 5, 1), Some(5));
        assert_eq!(horizontal_offset_within_line(first_line, 5, 6, 1), None);
        assert_eq!(horizontal_offset_within_line(first_line, 4, 8, 1), Some(5));

        assert_eq!(
            horizontal_offset_within_line(second_line, 7, 6, -1),
            Some(6)
        );
        assert_eq!(horizontal_offset_within_line(second_line, 6, 5, -1), None);
        assert_eq!(
            horizontal_offset_within_line(second_line, 7, 3, -1),
            Some(6)
        );
    }

    #[test]
    fn blockquote_newline_continues_marker() {
        assert_eq!(
            blockquote_newline_edit_for_line("> Quote", TextRange::new(0, 7), &(7..7)),
            Some((7..7, "\n> ".to_string(), 10..10))
        );
    }

    #[test]
    fn blockquote_newline_preserves_indented_marker() {
        assert_eq!(
            blockquote_newline_edit_for_line("   > Quote", TextRange::new(0, 10), &(10..10)),
            Some((10..10, "\n   > ".to_string(), 16..16))
        );
    }

    #[test]
    fn blockquote_newline_exits_empty_quote_line() {
        assert_eq!(
            blockquote_newline_edit_for_line("> ", TextRange::new(8, 10), &(10..10)),
            Some((8..10, String::new(), 8..8))
        );
        assert_eq!(
            blockquote_newline_edit_for_line(">    ", TextRange::new(0, 5), &(5..5)),
            Some((0..5, String::new(), 0..0))
        );
    }

    #[test]
    fn blockquote_newline_ignores_plain_lines_and_selections() {
        assert_eq!(
            blockquote_newline_edit_for_line("Quote", TextRange::new(0, 5), &(5..5)),
            None
        );
        assert_eq!(
            blockquote_newline_edit_for_line("> Quote", TextRange::new(0, 7), &(2..7)),
            None
        );
    }

    #[test]
    fn unordered_list_newline_continues_marker() {
        assert_eq!(
            list_newline_edit_for_line("- Item", TextRange::new(0, 6), &(6..6)),
            Some((6..6, "\n- ".to_string(), 9..9))
        );
        assert_eq!(
            list_newline_edit_for_line("  + Item", TextRange::new(0, 8), &(8..8)),
            Some((8..8, "\n  + ".to_string(), 13..13))
        );
    }

    #[test]
    fn ordered_list_newline_increments_marker() {
        assert_eq!(
            list_newline_edit_for_line("1. Item", TextRange::new(0, 7), &(7..7)),
            Some((7..7, "\n2. ".to_string(), 11..11))
        );
        assert_eq!(
            list_newline_edit_for_line("9) Item", TextRange::new(0, 7), &(7..7)),
            Some((7..7, "\n10) ".to_string(), 12..12))
        );
    }

    #[test]
    fn task_list_newline_continues_unchecked_marker() {
        assert_eq!(
            list_newline_edit_for_line("- [ ] Task", TextRange::new(0, 10), &(10..10)),
            Some((10..10, "\n- [ ] ".to_string(), 17..17))
        );
        assert_eq!(
            list_newline_edit_for_line("- [x] Done", TextRange::new(0, 10), &(10..10)),
            Some((10..10, "\n- [ ] ".to_string(), 17..17))
        );
        assert_eq!(
            list_newline_edit_for_line("1. [X] Done", TextRange::new(0, 11), &(11..11)),
            Some((11..11, "\n2. [ ] ".to_string(), 19..19))
        );
    }

    #[test]
    fn list_newline_exits_empty_item() {
        assert_eq!(
            list_newline_edit_for_line("- ", TextRange::new(8, 10), &(10..10)),
            Some((8..10, String::new(), 8..8))
        );
        assert_eq!(
            list_newline_edit_for_line("2.    ", TextRange::new(0, 6), &(6..6)),
            Some((0..6, String::new(), 0..0))
        );
        assert_eq!(
            list_newline_edit_for_line("- [ ] ", TextRange::new(0, 6), &(6..6)),
            Some((0..6, String::new(), 0..0))
        );
    }

    #[test]
    fn list_newline_outdents_empty_nested_item() {
        assert_eq!(
            list_newline_edit_for_line("  - ", TextRange::new(0, 4), &(4..4)),
            Some((0..2, String::new(), 2..2))
        );
        assert_eq!(
            list_newline_edit_for_line("    - [ ] ", TextRange::new(0, 10), &(10..10)),
            Some((0..2, String::new(), 8..8))
        );
    }

    #[test]
    fn list_newline_preserves_full_nested_indent() {
        assert_eq!(
            list_newline_edit_for_line("      - Item", TextRange::new(0, 12), &(12..12)),
            Some((12..12, "\n      - ".to_string(), 21..21))
        );
    }

    #[test]
    fn list_newline_ignores_plain_lines_and_selections() {
        assert_eq!(
            list_newline_edit_for_line("Item", TextRange::new(0, 4), &(4..4)),
            None
        );
        assert_eq!(
            list_newline_edit_for_line("- Item", TextRange::new(0, 6), &(2..6)),
            None
        );
    }

    #[test]
    fn list_indent_increases_current_list_item_indent() {
        assert_eq!(
            list_indent_edit("- Item", &(3..3), ListIndentDirection::Increase),
            Some((vec![TextEdit::insert(0, "  ")], 5..5))
        );
    }

    #[test]
    fn list_indent_decreases_current_list_item_indent() {
        assert_eq!(
            list_indent_edit("  - Item", &(4..4), ListIndentDirection::Decrease),
            Some((vec![TextEdit::replace(TextRange::new(0, 2), "")], 2..2))
        );
        assert_eq!(
            list_indent_edit("- Item", &(2..2), ListIndentDirection::Decrease),
            None
        );
    }

    #[test]
    fn list_indent_targets_only_selected_list_lines() {
        let text = "- One\nplain\n- Two";
        assert_eq!(
            list_indent_edit(text, &(0..text.len()), ListIndentDirection::Increase),
            Some((
                vec![TextEdit::insert(0, "  "), TextEdit::insert(12, "  ")],
                2..21
            ))
        );
    }

    #[test]
    fn list_indent_selection_ending_at_next_line_start_excludes_that_line() {
        let text = "- One\n- Two";
        assert_eq!(
            list_indent_edit(text, &(0..6), ListIndentDirection::Increase),
            Some((vec![TextEdit::insert(0, "  ")], 2..8))
        );
    }

    #[test]
    fn third_backtick_autocompletes_fenced_code_block() {
        assert_eq!(
            marker_autocomplete_edit("``", &(2..2), "`"),
            Some((2..2, "`\n\n```".to_string(), 4..4))
        );
    }

    #[test]
    fn direct_backtick_fence_input_autocompletes_fenced_code_block() {
        assert_eq!(
            marker_autocomplete_edit("", &(0..0), "```"),
            Some((0..0, "```\n\n```".to_string(), 4..4))
        );
    }

    #[test]
    fn third_tilde_autocompletes_fenced_code_block() {
        assert_eq!(
            marker_autocomplete_edit("~~", &(2..2), "~"),
            Some((2..2, "~\n\n~~~".to_string(), 4..4))
        );
    }

    #[test]
    fn direct_tilde_fence_input_autocompletes_fenced_code_block() {
        assert_eq!(
            marker_autocomplete_edit("", &(0..0), "~~~"),
            Some((0..0, "~~~\n\n~~~".to_string(), 4..4))
        );
    }

    #[test]
    fn code_fence_autocomplete_preserves_closing_fence_indent() {
        assert_eq!(
            marker_autocomplete_edit("  ``", &(4..4), "`"),
            Some((4..4, "`\n\n  ```".to_string(), 6..6))
        );
        assert_eq!(
            marker_autocomplete_edit("  ~~", &(4..4), "~"),
            Some((4..4, "~\n\n  ~~~".to_string(), 6..6))
        );
    }

    #[test]
    fn code_fence_autocomplete_requires_line_start_marker() {
        assert_eq!(marker_autocomplete_edit("Note ``", &(7..7), "`"), None);
        assert_eq!(marker_autocomplete_edit("``rust", &(2..2), "`"), None);
        assert_eq!(marker_autocomplete_edit("    ``", &(6..6), "`"), None);
        assert_eq!(marker_autocomplete_edit("Note ~~", &(7..7), "~"), None);
        assert_eq!(marker_autocomplete_edit("~~rust", &(2..2), "~"), None);
        assert_eq!(marker_autocomplete_edit("    ~~", &(6..6), "~"), None);
    }

    #[test]
    fn code_fence_autocomplete_does_not_interfere_with_closing_unclosed_fence() {
        assert_eq!(
            marker_autocomplete_edit(
                "```\ncode\n``",
                &("```\ncode\n``".len().."```\ncode\n``".len()),
                "`"
            ),
            None
        );
        assert_eq!(
            marker_autocomplete_edit(
                "```\ncode\n",
                &("```\ncode\n".len().."```\ncode\n".len()),
                "```"
            ),
            None
        );
        assert_eq!(
            marker_autocomplete_edit(
                "~~~\ncode\n~~",
                &("~~~\ncode\n~~".len().."~~~\ncode\n~~".len()),
                "~"
            ),
            None
        );
        assert_eq!(
            marker_autocomplete_edit(
                "```\ncode\n",
                &("```\ncode\n".len().."```\ncode\n".len()),
                "~~~"
            ),
            None
        );
        assert_eq!(
            marker_autocomplete_edit(
                "~~~\ncode\n",
                &("~~~\ncode\n".len().."~~~\ncode\n".len()),
                "```"
            ),
            None
        );
    }

    #[test]
    fn second_asterisk_autocompletes_strong_markers() {
        assert_eq!(
            marker_autocomplete_edit("*", &(1..1), "*"),
            Some((1..1, "***".to_string(), 2..2))
        );
        assert_eq!(
            marker_autocomplete_edit("", &(0..0), "**"),
            Some((0..0, "****".to_string(), 2..2))
        );
    }

    #[test]
    fn strong_autocomplete_ignores_existing_marker_runs() {
        assert_eq!(marker_autocomplete_edit("**", &(2..2), "*"), None);
        assert_eq!(marker_autocomplete_edit("*", &(1..1), "**"), None);
        assert_eq!(marker_autocomplete_edit("**", &(1..1), "*"), None);
    }

    #[test]
    fn strong_autocomplete_does_not_interfere_with_closing_unclosed_strong() {
        assert_eq!(
            marker_autocomplete_edit("**qwe*", &("**qwe*".len().."**qwe*".len()), "*"),
            None
        );
        assert_eq!(
            marker_autocomplete_edit("**qwe", &("**qwe".len().."**qwe".len()), "**"),
            None
        );
    }

    #[test]
    fn strong_autocomplete_starts_after_broken_strong_at_text_boundaries() {
        let after_space = "**qwe* ";
        assert_eq!(
            marker_autocomplete_edit(after_space, &(after_space.len()..after_space.len()), "**"),
            Some((
                after_space.len()..after_space.len(),
                "****".to_string(),
                after_space.len() + 2..after_space.len() + 2
            ))
        );

        let after_newline = "**qwe*\n";
        assert_eq!(
            marker_autocomplete_edit(
                after_newline,
                &(after_newline.len()..after_newline.len()),
                "**"
            ),
            Some((
                after_newline.len()..after_newline.len(),
                "****".to_string(),
                after_newline.len() + 2..after_newline.len() + 2
            ))
        );

        let typed_first_marker_after_space = "**qwe* *";
        assert_eq!(
            marker_autocomplete_edit(
                typed_first_marker_after_space,
                &(typed_first_marker_after_space.len()..typed_first_marker_after_space.len()),
                "*"
            ),
            Some((
                typed_first_marker_after_space.len()..typed_first_marker_after_space.len(),
                "***".to_string(),
                typed_first_marker_after_space.len() + 1..typed_first_marker_after_space.len() + 1
            ))
        );
    }

    #[test]
    fn marker_input_wraps_selected_text() {
        assert_eq!(
            marker_autocomplete_edit("Hanji notes", &(6..11), "**"),
            Some((6..11, "**notes**".to_string(), 8..13))
        );
        assert_eq!(
            marker_autocomplete_edit("let value = 1;", &(0..14), "```"),
            Some((0..14, "```\nlet value = 1;\n```".to_string(), 4..18))
        );
        assert_eq!(
            marker_autocomplete_edit("let value = 1;", &(0..14), "~~~"),
            Some((0..14, "~~~\nlet value = 1;\n~~~".to_string(), 4..18))
        );
    }

    #[test]
    fn marker_input_does_not_wrap_selection_with_partial_markers() {
        assert_eq!(marker_autocomplete_edit("Hanji notes", &(6..11), "*"), None);
        assert_eq!(marker_autocomplete_edit("Hanji notes", &(6..11), "`"), None);
    }

    #[test]
    fn strong_closing_marker_input_skips_existing_closing_marker() {
        assert_eq!(
            marker_skip_offset("**bold**", &("**bold".len().."**bold".len()), "*"),
            Some("**bold**".len())
        );
        assert_eq!(
            marker_skip_offset("**bold**", &("**bold".len().."**bold".len()), "**"),
            Some("**bold**".len())
        );
    }

    #[test]
    fn marker_skip_requires_open_strong_span() {
        assert_eq!(marker_skip_offset("plain **", &(6..6), "*"), None);
        assert_eq!(marker_skip_offset("**bold**", &(0..0), "*"), None);
        assert_eq!(marker_skip_offset("**bold**", &(2..6), "*"), None);
    }

    #[test]
    fn backspace_inside_empty_strong_pair_deletes_both_markers() {
        assert_eq!(
            empty_marker_pair_delete_backward_edit("****", &(2..2)),
            Some((0..4, String::new(), 0..0))
        );
        assert_eq!(
            empty_marker_pair_delete_backward_edit("Say **** now", &(6..6)),
            Some((4..8, String::new(), 4..4))
        );
    }

    #[test]
    fn empty_pair_delete_does_not_cross_non_empty_strong_pairs() {
        assert_eq!(
            empty_marker_pair_delete_backward_edit("**a****b**", &(5..5)),
            None
        );
        assert_eq!(
            empty_marker_pair_delete_backward_edit("****", &(1..1)),
            None
        );
    }

    #[test]
    fn drag_selection_starts_only_after_pointer_moves_past_threshold() {
        let origin = point(px(10.0), px(10.0));

        assert!(!drag_distance_exceeds_threshold(
            origin,
            point(px(12.0), px(10.0))
        ));
        assert!(drag_distance_exceeds_threshold(
            origin,
            point(px(12.1), px(10.0))
        ));
    }

    #[test]
    fn left_gutter_click_targets_content_start() {
        assert_eq!(
            line_marker_hit_offset(
                px(34.0),
                Some(TextRange::new(8, 10)),
                px(33.0),
                MarkerHitMode::ContentStart
            ),
            Some(10)
        );
        assert_eq!(
            line_marker_hit_offset(
                px(34.0),
                Some(TextRange::new(8, 10)),
                px(34.0),
                MarkerHitMode::ContentStart
            ),
            None
        );
        assert_eq!(
            line_marker_hit_offset(px(34.0), None, px(33.0), MarkerHitMode::ContentStart),
            None
        );
    }

    #[test]
    fn left_gutter_drag_targets_line_marker_start() {
        assert_eq!(
            line_marker_hit_offset(
                px(34.0),
                Some(TextRange::new(8, 10)),
                px(33.0),
                MarkerHitMode::MarkerStart
            ),
            Some(8)
        );
        assert_eq!(
            line_marker_hit_offset(
                px(34.0),
                Some(TextRange::new(8, 10)),
                px(34.0),
                MarkerHitMode::MarkerStart
            ),
            None
        );
        assert_eq!(
            line_marker_hit_offset(px(34.0), None, px(33.0), MarkerHitMode::MarkerStart),
            None
        );
    }

    #[test]
    fn task_marker_state_char_range_targets_checkbox_state() {
        assert_eq!(
            task_marker_state_char_range("- [ ] Task", TextRange::new(0, 6)),
            Some(3..4)
        );
        assert_eq!(
            task_marker_state_char_range("- [x] Task", TextRange::new(0, 6)),
            Some(3..4)
        );
        assert_eq!(
            task_marker_state_char_range("1. [X] Task", TextRange::new(0, 7)),
            Some(4..5)
        );
        assert_eq!(
            task_marker_state_char_range("- Item", TextRange::new(0, 2)),
            None
        );
    }
}
