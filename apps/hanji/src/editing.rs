use std::ops::Range;

use gpui::{Bounds, Pixels};
use hanji_editor::TextRange;

const DRAG_SELECTION_THRESHOLD: f64 = 2.0;

pub(crate) fn document_selection_range(document_len: usize) -> TextRange {
    TextRange::new(0, document_len)
}

pub(crate) fn selected_source_text(text: &str, range: &Range<usize>) -> Option<String> {
    if range.start == range.end {
        return None;
    }

    text.get(range.clone()).map(ToString::to_string)
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px};
    use hanji_core::TextEdit;
    use hanji_markdown::{
        ListIndentDirection, blockquote_newline_edit_for_line,
        empty_marker_pair_delete_backward_edit, list_indent_edit, list_newline_edit_for_line,
        marker_autocomplete_edit, marker_skip_offset, task_marker_state_char_range,
    };

    #[test]
    fn selection_helpers_keep_anchor_direction_separate_from_range_order() {
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
            marker_autocomplete_edit("Hanji notes", &(6..11), "*"),
            Some((6..11, "*notes*".to_string(), 7..12))
        );
        assert_eq!(
            marker_autocomplete_edit("Hanji notes", &(6..11), "**"),
            Some((6..11, "**notes**".to_string(), 8..13))
        );
        assert_eq!(
            marker_autocomplete_edit("Hanji notes", &(6..11), "***"),
            Some((6..11, "***notes***".to_string(), 9..14))
        );
        assert_eq!(
            marker_autocomplete_edit("Hanji notes", &(6..11), "`"),
            Some((6..11, "`notes`".to_string(), 7..12))
        );
        assert_eq!(
            marker_autocomplete_edit("Hanji notes", &(6..11), "~"),
            Some((6..11, "~notes~".to_string(), 7..12))
        );
        assert_eq!(
            marker_autocomplete_edit("Hanji notes", &(6..11), "~~"),
            Some((6..11, "~~notes~~".to_string(), 8..13))
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
    fn marker_input_preserves_same_style_markers_inside_selection() {
        let strong_source = "Capture **the** **thought** with `Hanji`.";
        let strong_expected = format!("**{strong_source}**");
        assert_eq!(
            marker_autocomplete_edit(strong_source, &(0..strong_source.len()), "**"),
            Some((
                0..strong_source.len(),
                strong_expected.clone(),
                "**".len()..strong_source.len() + "**".len()
            ))
        );

        let emphasis_source = "Capture *the* *thought* with Hanji.";
        let emphasis_expected = format!("*{emphasis_source}*");
        assert_eq!(
            marker_autocomplete_edit(emphasis_source, &(0..emphasis_source.len()), "*"),
            Some((
                0..emphasis_source.len(),
                emphasis_expected.clone(),
                "*".len()..emphasis_source.len() + "*".len()
            ))
        );

        let strike_source = "Capture ~~the~~ ~~thought~~ with Hanji.";
        let strike_expected = format!("~~{strike_source}~~");
        assert_eq!(
            marker_autocomplete_edit(strike_source, &(0..strike_source.len()), "~~"),
            Some((
                0..strike_source.len(),
                strike_expected.clone(),
                "~~".len()..strike_source.len() + "~~".len()
            ))
        );
    }

    #[test]
    fn repeated_single_marker_input_keeps_selection_inside_wrapped_text() {
        assert_eq!(
            marker_autocomplete_edit("Hanji *notes*", &(7..12), "*"),
            Some((7..12, "*notes*".to_string(), 8..13))
        );
        assert_eq!(
            marker_autocomplete_edit("Hanji ~notes~", &(7..12), "~"),
            Some((7..12, "~notes~".to_string(), 8..13))
        );
    }

    #[test]
    fn plain_input_does_not_wrap_selected_text() {
        assert_eq!(marker_autocomplete_edit("Hanji notes", &(6..11), "a"), None);
        assert_eq!(marker_autocomplete_edit("Hanji notes", &(6..11), "."), None);
        assert_eq!(marker_autocomplete_edit("Hanji notes", &(6..11), " "), None);
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
