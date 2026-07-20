use std::ops::Range;

use hanji_core::{TextEdit, TextRange};

use crate::{
    MarkdownListMarker, MarkdownTaskState, OrderedListDelimiter, ProjectedTableCell,
    blockquote_content_start, list_item,
};

const LIST_INDENT_WIDTH: usize = 2;
pub const TABLE_LINE_BREAK_SOURCE: &str = "<br>";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListIndentDirection {
    Increase,
    Decrease,
}

pub fn blockquote_newline_edit_for_line(
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

pub fn list_newline_edit_for_line(
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

pub fn list_indent_edit(
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

pub fn marker_autocomplete_edit(
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

pub fn marker_skip_offset(text: &str, range: &Range<usize>, new_text: &str) -> Option<usize> {
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

pub fn empty_marker_pair_delete_backward_edit(
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
        || !marker_count_before(text, opening_start, marker).is_multiple_of(2)
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
        "*" | "**" | "***" | "`" | "``" | "~" | "~~" => {
            wrap_selection_with_marker(text, range, new_text)
        }
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

pub fn ordered_list_delimiter(delimiter: OrderedListDelimiter) -> &'static str {
    match delimiter {
        OrderedListDelimiter::Dot => ".",
        OrderedListDelimiter::Paren => ")",
    }
}

pub fn task_marker_state_char_range(text: &str, marker_range: TextRange) -> Option<Range<usize>> {
    let marker_source = text.get(marker_range.start..marker_range.end)?;
    let marker_offset = marker_source
        .find("[ ]")
        .or_else(|| marker_source.find("[x]"))
        .or_else(|| marker_source.find("[X]"))?;
    let state_offset = marker_range.start + marker_offset + 1;

    Some(state_offset..state_offset + 1)
}

pub fn table_cell_at_offset(
    cells: &[ProjectedTableCell],
    offset: usize,
) -> Option<ProjectedTableCell> {
    cells
        .iter()
        .copied()
        .find(|cell| offset >= cell.content_range.start && offset <= cell.content_range.end)
        .or_else(|| {
            cells.iter().copied().find(|cell| {
                offset >= cell.source_outer_range.start && offset <= cell.source_outer_range.end
            })
        })
}

pub fn table_horizontal_caret_offset(
    cells: &[ProjectedTableCell],
    offset: usize,
    direction: isize,
) -> Option<usize> {
    let index = cells
        .iter()
        .position(|cell| offset >= cell.content_range.start && offset <= cell.content_range.end)?;
    let cell = cells[index];

    if direction < 0 && offset == cell.content_range.start {
        return Some(
            index
                .checked_sub(1)
                .and_then(|index| cells.get(index))
                .map_or(offset, |cell| cell.content_range.end),
        );
    }
    if direction > 0 && offset == cell.content_range.end {
        return Some(
            cells
                .get(index + 1)
                .map_or(offset, |cell| cell.content_range.start),
        );
    }

    None
}

pub fn table_newline_edit(
    cells: &[ProjectedTableCell],
    range: &Range<usize>,
) -> Option<(Range<usize>, String, Range<usize>)> {
    let start_cell = table_cell_at_offset(cells, range.start)?;
    let end_cell = table_cell_at_offset(cells, range.end)?;
    if start_cell.content_range != end_cell.content_range
        || range.start < start_cell.content_range.start
        || range.end > start_cell.content_range.end
    {
        return None;
    }

    let caret = range.start + TABLE_LINE_BREAK_SOURCE.len();
    Some((
        range.clone(),
        TABLE_LINE_BREAK_SOURCE.to_string(),
        caret..caret,
    ))
}

pub fn table_line_break_delete_edit(
    text: &str,
    range: &Range<usize>,
    direction: isize,
) -> Option<(Range<usize>, String, Range<usize>)> {
    if range.start != range.end {
        return None;
    }

    let marker_range = if direction < 0 {
        let start = range.start.checked_sub(TABLE_LINE_BREAK_SOURCE.len())?;
        start..range.start
    } else {
        range.end..range.end.checked_add(TABLE_LINE_BREAK_SOURCE.len())?
    };
    if text.get(marker_range.clone()) != Some(TABLE_LINE_BREAK_SOURCE) {
        return None;
    }

    let caret = marker_range.start;
    Some((marker_range, String::new(), caret..caret))
}

pub fn table_line_break_caret_offset(text: &str, offset: usize, direction: isize) -> Option<usize> {
    let marker_range = if direction < 0 {
        let start = offset.checked_sub(TABLE_LINE_BREAK_SOURCE.len())?;
        start..offset
    } else {
        offset..offset.checked_add(TABLE_LINE_BREAK_SOURCE.len())?
    };

    (text.get(marker_range.clone()) == Some(TABLE_LINE_BREAK_SOURCE)).then_some(if direction < 0 {
        marker_range.start
    } else {
        marker_range.end
    })
}

pub fn table_cell_line_start_for_offset(
    text: &str,
    cell: ProjectedTableCell,
    offset: usize,
) -> usize {
    let cell_start = cell.content_range.start;
    let Some(prefix) = text.get(cell_start..offset) else {
        return cell_start;
    };

    prefix
        .rfind(TABLE_LINE_BREAK_SOURCE)
        .map_or(cell_start, |relative_offset| {
            cell_start + relative_offset + TABLE_LINE_BREAK_SOURCE.len()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanji_core::{Document, TextEdit};

    #[test]
    fn blockquote_newline_continues_or_exits_the_quote() {
        assert_eq!(
            blockquote_newline_edit_for_line("> Quote", TextRange::new(0, 7), &(7..7)),
            Some((7..7, "\n> ".to_string(), 10..10))
        );
        assert_eq!(
            blockquote_newline_edit_for_line("> ", TextRange::new(8, 10), &(10..10)),
            Some((8..10, String::new(), 8..8))
        );
    }

    #[test]
    fn list_newline_continues_and_increments_markers() {
        assert_eq!(
            list_newline_edit_for_line("- Item", TextRange::new(0, 6), &(6..6)),
            Some((6..6, "\n- ".to_string(), 9..9))
        );
        assert_eq!(
            list_newline_edit_for_line("9) Item", TextRange::new(0, 7), &(7..7)),
            Some((7..7, "\n10) ".to_string(), 12..12))
        );
    }

    #[test]
    fn list_indent_plans_source_edits_and_selection() {
        assert_eq!(
            list_indent_edit("- Item", &(3..3), ListIndentDirection::Increase),
            Some((vec![TextEdit::insert(0, "  ")], 5..5))
        );
        assert_eq!(
            list_indent_edit("  - Item", &(4..4), ListIndentDirection::Decrease),
            Some((vec![TextEdit::replace(TextRange::new(0, 2), "")], 2..2))
        );
    }

    #[test]
    fn typing_completes_fences_and_strong_markers() {
        assert_eq!(
            marker_autocomplete_edit("``", &(2..2), "`"),
            Some((2..2, "`\n\n```".to_string(), 4..4))
        );
        assert_eq!(
            marker_autocomplete_edit("*", &(1..1), "*"),
            Some((1..1, "***".to_string(), 2..2))
        );
    }

    #[test]
    fn marker_input_wraps_selected_source() {
        assert_eq!(
            marker_autocomplete_edit("Hanji notes", &(6..11), "**"),
            Some((6..11, "**notes**".to_string(), 8..13))
        );
    }

    #[test]
    fn strong_marker_skip_and_pair_deletion_are_source_aware() {
        assert_eq!(marker_skip_offset("**bold**", &(6..6), "*"), Some(8));
        assert_eq!(
            empty_marker_pair_delete_backward_edit("****", &(2..2)),
            Some((0..4, String::new(), 0..0))
        );
    }

    #[test]
    fn task_marker_range_targets_only_the_state_character() {
        assert_eq!(
            task_marker_state_char_range("- [x] Task", TextRange::new(0, 6)),
            Some(3..4)
        );
    }

    #[test]
    fn table_newline_and_deletion_use_a_source_backed_marker() {
        let document = Document::new("| Hanji | Ready |\n| --- | --- |");
        let projection = crate::project_document(&document);
        let cell = projection.lines()[0].table_cells()[0];
        let caret = cell.content_range.start + 2;

        assert_eq!(
            table_newline_edit(projection.lines()[0].table_cells(), &(caret..caret)),
            Some((
                caret..caret,
                TABLE_LINE_BREAK_SOURCE.to_string(),
                caret + TABLE_LINE_BREAK_SOURCE.len()..caret + TABLE_LINE_BREAK_SOURCE.len(),
            ))
        );
        assert_eq!(
            table_line_break_delete_edit("Hanji<br>Editor", &(9..9), -1),
            Some((5..9, String::new(), 5..5))
        );
    }
}
