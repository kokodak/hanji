use std::ops::Range;

use gpui::{Bounds, Pixels};
use hanji_core::TextRange;
use hanji_markdown::{
    MarkdownListMarker, MarkdownTaskState, OrderedListDelimiter, blockquote_content_start,
    list_item,
};

const DRAG_SELECTION_THRESHOLD: f64 = 2.0;

pub(crate) fn selection_range_from_anchor_and_head(anchor: usize, head: usize) -> TextRange {
    TextRange::new(anchor.min(head), anchor.max(head))
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
        let caret = line_range.start;
        return Some((
            line_range.start..line_range.end,
            String::new(),
            caret..caret,
        ));
    }

    let indent_len = line_source
        .bytes()
        .take_while(|byte| *byte == b' ')
        .take(4)
        .count();
    let marker = next_list_item_marker_text(list_item.marker, list_item.task);
    let replacement = format!("\n{}{marker} ", &line_source[..indent_len]);
    let caret = range.start + replacement.len();

    Some((range.clone(), replacement, caret..caret))
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
        "```" => wrap_selection_with_fence(text, range),
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
) -> Option<(Range<usize>, String, Range<usize>)> {
    let selected = text.get(range.clone())?;
    let prefix = "```\n";
    let suffix = "\n```";
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
    if !matches!(new_text, "`" | "```") {
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

    let indent = match new_text {
        "`" => fence_indent_for_pending_prefix(line_prefix, "``")?,
        "```" => fence_indent_for_pending_prefix(line_prefix, "")?,
        _ => return None,
    };
    let replacement = format!("{new_text}\n\n{indent}```");
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
                || has_unclosed_marker_before(text, marker_start, "**")
                || following_text_starts_with(text, range.start, "*")
            {
                return None;
            }

            ("***".to_string(), range.start + 1)
        }
        "**" => {
            if previous_text_is_exact_marker(text, range.start, "*")
                || has_unclosed_marker_before(text, range.start, "**")
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
    let mut open_marker_len = None;

    for line in prefix.split('\n') {
        if let Some(marker_len) = open_marker_len {
            if is_closing_backtick_fence(line, marker_len) {
                open_marker_len = None;
            }
        } else if let Some(marker_len) = opening_backtick_fence_len(line) {
            open_marker_len = Some(marker_len);
        }
    }

    open_marker_len.is_some()
}

fn opening_backtick_fence_len(line: &str) -> Option<usize> {
    let indent = line
        .bytes()
        .take_while(|byte| *byte == b' ')
        .take(4)
        .count();
    if indent >= 4 {
        return None;
    }

    let marker_len = line[indent..]
        .bytes()
        .take_while(|byte| *byte == b'`')
        .count();
    (marker_len >= 3).then_some(marker_len)
}

fn is_closing_backtick_fence(line: &str, min_marker_len: usize) -> bool {
    let Some(marker_len) = opening_backtick_fence_len(line) else {
        return false;
    };
    marker_len >= min_marker_len
        && line[line
            .bytes()
            .take_while(|byte| *byte == b' ')
            .take(4)
            .count()
            + marker_len..]
            .bytes()
            .all(|byte| matches!(byte, b' ' | b'\t'))
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
    fn code_fence_autocomplete_preserves_closing_fence_indent() {
        assert_eq!(
            marker_autocomplete_edit("  ``", &(4..4), "`"),
            Some((4..4, "`\n\n  ```".to_string(), 6..6))
        );
    }

    #[test]
    fn code_fence_autocomplete_requires_line_start_marker() {
        assert_eq!(marker_autocomplete_edit("Note ``", &(7..7), "`"), None);
        assert_eq!(marker_autocomplete_edit("``rust", &(2..2), "`"), None);
        assert_eq!(marker_autocomplete_edit("    ``", &(6..6), "`"), None);
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
    fn marker_input_wraps_selected_text() {
        assert_eq!(
            marker_autocomplete_edit("Hanji notes", &(6..11), "**"),
            Some((6..11, "**notes**".to_string(), 8..13))
        );
        assert_eq!(
            marker_autocomplete_edit("let value = 1;", &(0..14), "```"),
            Some((0..14, "```\nlet value = 1;\n```".to_string(), 4..18))
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
