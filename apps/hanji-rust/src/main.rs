use std::{env, io, ops::Range, path::PathBuf, process};

use gpui::{
    App, Application, BorderStyle, Bounds, Context, CursorStyle, Element, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, FocusHandle, Focusable, FontWeight,
    GlobalElementId, InspectorElementId, IntoElement, KeyBinding, LayoutId, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Render, ShapedLine,
    SharedString, Style, TextRun, TextStyle, UTF16Selection, UnderlineStyle, Window, WindowBounds,
    WindowOptions, actions, div, fill, point, prelude::*, px, quad, relative, rgb, rgba, size,
};
use hanji_core::{EditorCommand, Selection, TextPosition, TextRange, Transaction};
use hanji_markdown::{
    MarkdownCommand, MarkdownCommandError, MarkdownLine, MarkdownListMarker, MarkdownTaskState,
    OrderedListDelimiter, ProjectedSegmentKind, ProjectedVisibleSegment, blockquote_content_start,
    execute_markdown_command, list_item, project_document,
};
use hanji_storage::DocumentSession;

const LINE_HEIGHT: f32 = 24.0;
const FONT_SIZE: f32 = 16.0;
const BLOCKQUOTE_BAR_LEFT_INSET: f32 = 4.0;
const BLOCKQUOTE_BAR_WIDTH: f32 = 3.0;
const BLOCKQUOTE_BAR_TOP_INSET: f32 = 2.0;
const BLOCKQUOTE_BAR_BOTTOM_INSET: f32 = 0.0;
const LIST_BULLET_FONT_SIZE: f32 = 11.0;
const CHECKBOX_MARKER_FONT_SIZE: f32 = 22.0;
const CHECKBOX_BOX_SIZE: f32 = 16.0;
const CHECKBOX_CHECK_FONT_SIZE: f32 = 17.0;
const CHECKBOX_CONTENT_GAP: f32 = 5.0;
const MARKDOWN_MARKER_COLOR: u32 = 0x238636;
const DRAG_SELECTION_THRESHOLD: f64 = 2.0;
const SAMPLE_DOCUMENT: &str = "# Hanji\n\nCapture the **thought** with `code`.";

actions!(
    hanji,
    [
        Backspace,
        Delete,
        Left,
        Right,
        ShiftLeft,
        ShiftRight,
        OptionLeft,
        OptionRight,
        ShiftOptionLeft,
        ShiftOptionRight,
        Up,
        Down,
        ShiftUp,
        ShiftDown,
        CmdUp,
        CmdDown,
        ShiftCmdLeft,
        ShiftCmdRight,
        ShiftCmdUp,
        ShiftCmdDown,
        Home,
        End,
        Newline,
        ToggleStrong,
        ToggleItalic,
        ToggleCode,
        Undo,
        Redo,
        Save,
        Quit
    ]
);

struct Hanji {
    focus_handle: FocusHandle,
    session: DocumentSession,
    marked_range: Option<Range<usize>>,
    last_lines: Vec<LineSnapshot>,
    last_task_markers: Vec<TaskMarkerHitbox>,
    preferred_column: Option<usize>,
    selection_anchor: Option<usize>,
    selection_head: Option<usize>,
    is_selecting: bool,
    selection_drag_origin: Option<gpui::Point<Pixels>>,
    status_message: Option<String>,
}

impl Hanji {
    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.clear_selection_tracking();
        let changed = self
            .session
            .execute(EditorCommand::DeleteBackward)
            .unwrap_or(false);

        if changed {
            self.document_changed(window, cx);
        }
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.clear_selection_tracking();
        let changed = self
            .session
            .execute(EditorCommand::DeleteForward)
            .unwrap_or(false);

        if changed {
            self.document_changed(window, cx);
        }
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        let document = self.session.document();
        let range = document.selection().primary();
        let offset = if range.is_empty() {
            document
                .previous_grapheme_offset(range.start)
                .ok()
                .flatten()
                .and_then(|offset| {
                    self.horizontal_offset_within_current_line(range.start, offset, -1)
                })
        } else {
            Some(range.start)
        };

        if let Some(offset) = offset {
            self.move_caret(offset, cx);
        }
    }

    fn shift_left(&mut self, _: &ShiftLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.extend_selection_horizontally(-1, cx);
    }

    fn option_left(&mut self, _: &OptionLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        let document = self.session.document();
        let range = document.selection().primary();
        let offset = if range.is_empty() {
            document
                .previous_word_offset(range.start)
                .ok()
                .flatten()
                .and_then(|offset| {
                    self.horizontal_offset_within_current_line(range.start, offset, -1)
                })
        } else {
            Some(range.start)
        };

        if let Some(offset) = offset {
            self.move_caret(offset, cx);
        }
    }

    fn shift_option_left(&mut self, _: &ShiftOptionLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.extend_selection_by_word(-1, cx);
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        let document = self.session.document();
        let range = document.selection().primary();
        let offset = if range.is_empty() {
            document
                .next_grapheme_offset(range.end)
                .ok()
                .flatten()
                .and_then(|offset| self.horizontal_offset_within_current_line(range.end, offset, 1))
        } else {
            Some(range.end)
        };

        if let Some(offset) = offset {
            self.move_caret(offset, cx);
        }
    }

    fn shift_right(&mut self, _: &ShiftRight, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.extend_selection_horizontally(1, cx);
    }

    fn option_right(&mut self, _: &OptionRight, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        let document = self.session.document();
        let range = document.selection().primary();
        let offset = if range.is_empty() {
            document
                .next_word_offset(range.end)
                .ok()
                .flatten()
                .and_then(|offset| self.horizontal_offset_within_current_line(range.end, offset, 1))
        } else {
            Some(range.end)
        };

        if let Some(offset) = offset {
            self.move_caret(offset, cx);
        }
    }

    fn shift_option_right(&mut self, _: &ShiftOptionRight, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.extend_selection_by_word(1, cx);
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.move_caret_vertically(-1, cx);
    }

    fn shift_up(&mut self, _: &ShiftUp, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.extend_selection_vertically(-1, cx);
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.move_caret_vertically(1, cx);
    }

    fn shift_down(&mut self, _: &ShiftDown, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.extend_selection_vertically(1, cx);
    }

    fn cmd_up(&mut self, _: &CmdUp, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.move_caret(0, cx);
    }

    fn cmd_down(&mut self, _: &CmdDown, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.move_caret(self.session.document().len(), cx);
    }

    fn shift_cmd_left(&mut self, _: &ShiftCmdLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.extend_selection_to_line_boundary(-1, cx);
    }

    fn shift_cmd_right(&mut self, _: &ShiftCmdRight, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.extend_selection_to_line_boundary(1, cx);
    }

    fn shift_cmd_up(&mut self, _: &ShiftCmdUp, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.extend_selection_to_document_boundary(-1, cx);
    }

    fn shift_cmd_down(&mut self, _: &ShiftCmdDown, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.extend_selection_to_document_boundary(1, cx);
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        let range = self.session.document().selection().primary();
        let Some(line_range) = self.line_range_for_offset(range.start) else {
            return;
        };

        self.move_caret(line_range.start, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        let range = self.session.document().selection().primary();
        let Some(line_range) = self.line_range_for_offset(range.end) else {
            return;
        };

        self.move_caret(line_range.end, cx);
    }

    fn newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.clear_selection_tracking();
        let range = self.selected_range();

        if let Some((range, replacement, selection_after)) = self.blockquote_newline_edit(&range) {
            self.replace_range(range, &replacement, selection_after, window, cx);
            return;
        }

        if let Some((range, replacement, selection_after)) = self.list_newline_edit(&range) {
            self.replace_range(range, &replacement, selection_after, window, cx);
            return;
        }

        let caret = range.start + "\n".len();
        self.replace_range(range, "\n", caret..caret, window, cx);
    }

    fn toggle_strong(&mut self, _: &ToggleStrong, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_markdown_command(
            MarkdownCommand::ToggleStrong,
            "Could not toggle strong text.",
            window,
            cx,
        );
    }

    fn toggle_italic(&mut self, _: &ToggleItalic, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_markdown_command(
            MarkdownCommand::ToggleEmphasis,
            "Could not toggle italic text.",
            window,
            cx,
        );
    }

    fn toggle_code(&mut self, _: &ToggleCode, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_markdown_command(
            MarkdownCommand::ToggleCode,
            "Could not toggle code text.",
            window,
            cx,
        );
    }

    fn apply_markdown_command(
        &mut self,
        command: MarkdownCommand,
        failure_message: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked_range = None;
        self.clear_selection_tracking();

        match self
            .session
            .edit_document(|document| execute_markdown_command(document, command))
        {
            Ok(true) => {
                self.preferred_column = None;
                self.document_changed(window, cx);
            }
            Ok(false) => {}
            Err(MarkdownCommandError::MultipleSelectionsUnsupported) => {
                self.status_message =
                    Some("Multiple selections are not supported yet.".to_string());
                cx.notify();
            }
            Err(MarkdownCommandError::Edit(_)) => {
                self.status_message = Some(failure_message.to_string());
                cx.notify();
            }
        }
    }

    fn undo(&mut self, _: &Undo, window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.clear_selection_tracking();

        if self.session.undo().is_some() {
            self.document_changed(window, cx);
        }
    }

    fn redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.clear_selection_tracking();

        if self.session.redo().is_some() {
            self.document_changed(window, cx);
        }
    }

    fn save(&mut self, _: &Save, window: &mut Window, cx: &mut Context<Self>) {
        match self.session.save() {
            Ok(()) => {
                self.status_message = Some("Saved.".to_string());
                self.update_window_title(window);
            }
            Err(error) => {
                self.status_message = Some(format!("Save failed: {error}"));
            }
        }

        cx.notify();
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        self.marked_range = None;
        if !event.modifiers.shift
            && let Some(task_marker) = self.task_marker_at_position(event.position)
            && self.toggle_task_marker(task_marker, window, cx)
        {
            return;
        }

        let offset = self.index_for_mouse_position(event.position);

        self.is_selecting = false;
        self.selection_drag_origin = Some(event.position);
        if event.modifiers.shift {
            let (anchor, _) = self.selection_extension_points(1);
            self.select_from_anchor_to(anchor, offset, None, cx);
        } else {
            self.select_from_anchor_to(offset, offset, None, cx);
        }
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.pressed_button != Some(MouseButton::Left) {
            return;
        }

        if !self.is_selecting {
            let Some(origin) = self.selection_drag_origin else {
                return;
            };

            if !drag_distance_exceeds_threshold(origin, event.position) {
                return;
            }

            self.is_selecting = true;
        }

        let anchor = self
            .selection_anchor
            .unwrap_or_else(|| self.session.document().selection().primary().start);
        self.select_from_anchor_to(
            anchor,
            self.index_for_mouse_position_with_marker_mode(
                event.position,
                MarkerHitMode::MarkerStart,
            ),
            None,
            cx,
        );
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
        self.selection_drag_origin = None;
    }

    fn move_caret(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.preferred_column = None;
        self.clear_selection_tracking();
        let offset = self
            .session
            .document()
            .nearest_grapheme_offset(offset)
            .unwrap_or(offset);

        if self.session.set_selection(Selection::caret(offset)).is_ok() {
            cx.notify();
        }
    }

    fn move_caret_preserving_column(
        &mut self,
        offset: usize,
        column: usize,
        cx: &mut Context<Self>,
    ) {
        self.clear_selection_tracking();

        if self.session.set_selection(Selection::caret(offset)).is_ok() {
            self.preferred_column = Some(column);
            cx.notify();
        }
    }

    fn move_caret_vertically(&mut self, direction: isize, cx: &mut Context<Self>) {
        let document = self.session.document();
        let range = document.selection().primary();
        let offset = if range.is_empty() {
            range.start
        } else if direction < 0 {
            range.start
        } else {
            range.end
        };

        let Ok(position) = document.position_at_offset(offset) else {
            return;
        };
        let Some(target_line) = position.line.checked_add_signed(direction) else {
            return;
        };

        if target_line >= document.line_count() {
            return;
        }

        let column = self.preferred_column.unwrap_or(position.column);
        let target_offset = document
            .offset_at_position(TextPosition::new(target_line, column))
            .or_else(|_| {
                document
                    .line_range(target_line)
                    .map(|range| range.end)
                    .ok_or(hanji_core::EditError::InvalidRange)
            });

        if let Ok(target_offset) = target_offset {
            self.move_caret_preserving_column(target_offset, column, cx);
        }
    }

    fn extend_selection_horizontally(&mut self, direction: isize, cx: &mut Context<Self>) {
        let document = self.session.document();
        let (anchor, head) = self.selection_extension_points(direction);
        let offset = if direction < 0 {
            document.previous_grapheme_offset(head).ok().flatten()
        } else {
            document.next_grapheme_offset(head).ok().flatten()
        }
        .and_then(|offset| self.horizontal_offset_within_current_line(head, offset, direction));

        if let Some(offset) = offset {
            self.select_from_anchor_to(anchor, offset, None, cx);
        }
    }

    fn extend_selection_vertically(&mut self, direction: isize, cx: &mut Context<Self>) {
        let document = self.session.document();
        let (anchor, head) = self.selection_extension_points(direction);
        let Ok(position) = document.position_at_offset(head) else {
            return;
        };
        let Some(target_line) = position.line.checked_add_signed(direction) else {
            return;
        };

        if target_line >= document.line_count() {
            return;
        }

        let column = self.preferred_column.unwrap_or(position.column);
        let target_offset = document
            .offset_at_position(TextPosition::new(target_line, column))
            .or_else(|_| {
                document
                    .line_range(target_line)
                    .map(|range| range.end)
                    .ok_or(hanji_core::EditError::InvalidRange)
            });

        if let Ok(target_offset) = target_offset {
            self.select_from_anchor_to(anchor, target_offset, Some(column), cx);
        }
    }

    fn extend_selection_by_word(&mut self, direction: isize, cx: &mut Context<Self>) {
        let document = self.session.document();
        let (anchor, head) = self.selection_extension_points(direction);
        let offset = if direction < 0 {
            document.previous_word_offset(head).ok().flatten()
        } else {
            document.next_word_offset(head).ok().flatten()
        }
        .and_then(|offset| self.horizontal_offset_within_current_line(head, offset, direction));

        if let Some(offset) = offset {
            self.select_from_anchor_to(anchor, offset, None, cx);
        }
    }

    fn extend_selection_to_line_boundary(&mut self, direction: isize, cx: &mut Context<Self>) {
        let (anchor, head) = self.selection_extension_points(direction);
        let Some(line_range) = self.line_range_for_offset(head) else {
            return;
        };
        let offset = if direction < 0 {
            line_range.start
        } else {
            line_range.end
        };

        self.select_from_anchor_to(anchor, offset, None, cx);
    }

    fn extend_selection_to_document_boundary(&mut self, direction: isize, cx: &mut Context<Self>) {
        let (anchor, _) = self.selection_extension_points(direction);
        let offset = if direction < 0 {
            0
        } else {
            self.session.document().len()
        };

        self.select_from_anchor_to(anchor, offset, None, cx);
    }

    fn selection_extension_points(&self, direction: isize) -> (usize, usize) {
        if let (Some(anchor), Some(head)) = (self.selection_anchor, self.selection_head) {
            return (anchor, head);
        }

        extension_points_for_selection(self.session.document().selection().primary(), direction)
    }

    fn select_from_anchor_to(
        &mut self,
        anchor: usize,
        head: usize,
        preferred_column: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        let document = self.session.document();
        let anchor = document.nearest_grapheme_offset(anchor).unwrap_or(anchor);
        let head = document.nearest_grapheme_offset(head).unwrap_or(head);
        let range = selection_range_from_anchor_and_head(anchor, head);

        if self.session.set_selection(Selection::single(range)).is_ok() {
            self.selection_anchor = Some(anchor);
            self.selection_head = Some(head);
            self.preferred_column = preferred_column;
            cx.notify();
        }
    }

    fn clear_selection_tracking(&mut self) {
        self.selection_anchor = None;
        self.selection_head = None;
        self.is_selecting = false;
        self.selection_drag_origin = None;
    }

    fn selected_range(&self) -> Range<usize> {
        let range = self.session.document().selection().primary();
        range.start..range.end
    }

    fn replace_range(
        &mut self,
        range: Range<usize>,
        new_text: &str,
        selection_after: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let transaction = Transaction::replace(TextRange::new(range.start, range.end), new_text)
            .with_selection_after(Selection::single(TextRange::new(
                selection_after.start,
                selection_after.end,
            )));

        if self.session.apply(transaction).is_err() {
            return false;
        }

        self.preferred_column = None;
        self.clear_selection_tracking();
        self.document_changed(window, cx);
        true
    }

    fn blockquote_newline_edit(
        &self,
        range: &Range<usize>,
    ) -> Option<(Range<usize>, String, Range<usize>)> {
        if range.start != range.end {
            return None;
        }

        let document = self.session.document();
        let line_range = self.line_range_for_offset(range.start)?;
        let line_source = &document.text()[line_range.start..line_range.end];

        blockquote_newline_edit_for_line(line_source, line_range, range)
    }

    fn list_newline_edit(
        &self,
        range: &Range<usize>,
    ) -> Option<(Range<usize>, String, Range<usize>)> {
        if range.start != range.end {
            return None;
        }

        let document = self.session.document();
        let line_range = self.line_range_for_offset(range.start)?;
        let line_source = &document.text()[line_range.start..line_range.end];

        list_newline_edit_for_line(line_source, line_range, range)
    }

    fn line_range_for_offset(&self, offset: usize) -> Option<TextRange> {
        let document = self.session.document();
        let line_index = document.line_index_at_offset(offset).ok()?;

        document.line_range(line_index)
    }

    fn horizontal_offset_within_current_line(
        &self,
        origin: usize,
        candidate: usize,
        direction: isize,
    ) -> Option<usize> {
        let line_range = self.line_range_for_offset(origin)?;

        horizontal_offset_within_line(line_range, origin, candidate, direction)
    }

    fn index_for_mouse_position(&self, position: gpui::Point<Pixels>) -> usize {
        self.index_for_mouse_position_with_marker_mode(position, MarkerHitMode::ContentStart)
    }

    fn index_for_mouse_position_with_marker_mode(
        &self,
        position: gpui::Point<Pixels>,
        marker_mode: MarkerHitMode,
    ) -> usize {
        let Some(first_line) = self.last_lines.first() else {
            return 0;
        };

        if position.y < first_line.bounds.top() {
            return 0;
        }

        let Some(last_line) = self.last_lines.last() else {
            return 0;
        };

        if position.y > last_line.bounds.bottom() {
            return self.session.document().len();
        }

        let line = self
            .last_lines
            .iter()
            .find(|line| position.y >= line.bounds.top() && position.y <= line.bounds.bottom())
            .unwrap_or(last_line);
        if let Some(offset) = line_marker_hit_offset(
            line.bounds.left(),
            line.marker_range,
            position.x,
            marker_mode,
        ) {
            return offset;
        }

        let local_x = position.x - line.bounds.left();
        let visible_offset = line
            .layout
            .closest_index_for_x(local_x)
            .min(line.visible_len);
        let offset = line.visible_to_source_caret_offset(visible_offset);

        self.session
            .document()
            .nearest_grapheme_offset(offset)
            .unwrap_or(offset)
    }

    fn task_marker_at_position(&self, position: gpui::Point<Pixels>) -> Option<TaskMarkerHitbox> {
        self.last_task_markers
            .iter()
            .copied()
            .find(|marker| bounds_contains_point(marker.bounds, position))
    }

    fn toggle_task_marker(
        &mut self,
        task_marker: TaskMarkerHitbox,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let document = self.session.document();
        let Some(range) = task_marker_state_char_range(document.text(), task_marker.marker_range)
        else {
            return false;
        };
        let replacement = match task_marker.state {
            MarkdownTaskState::Unchecked => "x",
            MarkdownTaskState::Checked => " ",
        };
        let selection = document.selection().primary();

        self.replace_range(
            range,
            replacement,
            selection.start..selection.end,
            window,
            cx,
        )
    }

    fn byte_range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        byte_offset_to_utf16(self.session.document().text(), range.start)
            ..byte_offset_to_utf16(self.session.document().text(), range.end)
    }

    fn utf16_range_to_byte(&self, range: &Range<usize>) -> Range<usize> {
        let text = self.session.document().text();
        let range = utf16_offset_to_byte(text, range.start)..utf16_offset_to_byte(text, range.end);

        self.snap_byte_range(range)
    }

    fn bounds_for_byte_range(&self, range: Range<usize>) -> Option<Bounds<Pixels>> {
        let range = self.snap_byte_range(range);
        let line = self.line_for_offset(range.start)?;
        let start = line.source_to_visible_offset(range.start);
        let end = line.source_to_visible_offset(range.end);
        let start_x = line.layout.x_for_index(start);
        let end_x = line.layout.x_for_index(end).max(start_x + px(2.0));

        Some(Bounds::from_corners(
            point(line.bounds.left() + start_x, line.bounds.top()),
            point(line.bounds.left() + end_x, line.bounds.bottom()),
        ))
    }

    fn line_for_offset(&self, offset: usize) -> Option<&LineSnapshot> {
        self.last_lines
            .iter()
            .find(|line| offset >= line.range.start && offset <= line.range.end)
            .or_else(|| self.last_lines.last())
    }

    fn snap_byte_range(&self, range: Range<usize>) -> Range<usize> {
        let document = self.session.document();
        let start = document
            .nearest_grapheme_offset(range.start)
            .unwrap_or(range.start);
        let end = document
            .nearest_grapheme_offset(range.end)
            .unwrap_or(range.end);

        start.min(end)..start.max(end)
    }

    fn document_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.status_message = None;
        self.update_window_title(window);
        cx.notify();
    }

    fn update_window_title(&self, window: &mut Window) {
        window.set_window_title(&self.window_title());
    }

    fn window_title(&self) -> String {
        format!("{} - Hanji", self.document_label())
    }

    fn document_label(&self) -> String {
        let name = self
            .session
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");
        let dirty = if self.session.is_dirty() { " *" } else { "" };

        format!("{name}{dirty}")
    }
}

impl EntityInputHandler for Hanji {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.utf16_range_to_byte(&range_utf16);
        adjusted_range.replace(self.byte_range_to_utf16(&range));
        Some(self.session.document().text()[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.byte_range_to_utf16(&self.selected_range()),
            reversed: self
                .selection_anchor
                .zip(self.selection_head)
                .is_some_and(|(anchor, head)| selection_is_reversed(anchor, head)),
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.byte_range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.utf16_range_to_byte(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range());
        let caret = range.start + new_text.len();

        if self.replace_range(range, new_text, caret..caret, window, cx) {
            self.marked_range = None;
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.utf16_range_to_byte(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range());
        let insert_start = range.start;
        let marked_range =
            (!new_text.is_empty()).then_some(insert_start..insert_start + new_text.len());
        let selection_after = if let Some(selected_range) = new_selected_range_utf16.as_ref() {
            let selected_range = utf16_range_to_byte(new_text, selected_range);
            insert_start + selected_range.start..insert_start + selected_range.end
        } else {
            let caret = range.start + new_text.len();
            caret..caret
        };

        if self.replace_range(range, new_text, selection_after, window, cx) {
            self.marked_range = marked_range;
        }
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        self.bounds_for_byte_range(self.utf16_range_to_byte(&range_utf16))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(byte_offset_to_utf16(
            self.session.document().text(),
            self.index_for_mouse_position(point),
        ))
    }
}

impl Focusable for Hanji {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Hanji {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let document_label: SharedString = self.document_label().into();
        let status_message: SharedString = self.status_message.clone().unwrap_or_default().into();

        div()
            .track_focus(&self.focus_handle(cx))
            .key_context("HanjiEditor")
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::shift_left))
            .on_action(cx.listener(Self::shift_right))
            .on_action(cx.listener(Self::option_left))
            .on_action(cx.listener(Self::option_right))
            .on_action(cx.listener(Self::shift_option_left))
            .on_action(cx.listener(Self::shift_option_right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::shift_up))
            .on_action(cx.listener(Self::shift_down))
            .on_action(cx.listener(Self::cmd_up))
            .on_action(cx.listener(Self::cmd_down))
            .on_action(cx.listener(Self::shift_cmd_left))
            .on_action(cx.listener(Self::shift_cmd_right))
            .on_action(cx.listener(Self::shift_cmd_up))
            .on_action(cx.listener(Self::shift_cmd_down))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::toggle_strong))
            .on_action(cx.listener(Self::toggle_italic))
            .on_action(cx.listener(Self::toggle_code))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::save))
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .bg(rgb(0xf8f7f2))
            .p_8()
            .text_color(rgb(0x25231f))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xl().child("Hanji"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x69645a))
                            .child(document_label),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x69645a))
                    .child(status_message),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_h(px(360.0))
                    .p_4()
                    .border_1()
                    .border_color(rgb(0xd8d3c7))
                    .bg(rgb(0xfffefb))
                    .line_height(px(LINE_HEIGHT))
                    .text_size(px(FONT_SIZE))
                    .font_family("Menlo")
                    .cursor(CursorStyle::IBeam)
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
                    .on_mouse_move(cx.listener(Self::on_mouse_move))
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
                    .child(EditorElement {
                        editor: cx.entity(),
                    }),
            )
    }
}

#[derive(Clone)]
struct LineSnapshot {
    range: TextRange,
    marker_range: Option<TextRange>,
    visible_len: usize,
    segments: Vec<LineSegmentSnapshot>,
    layout: ShapedLine,
    bounds: Bounds<Pixels>,
}

impl LineSnapshot {
    fn source_to_visible_offset(&self, source_offset: usize) -> usize {
        source_to_visible_offset_in_segments(
            &self.segments,
            self.range,
            self.visible_len,
            source_offset,
        )
    }

    fn visible_to_source_caret_offset(&self, visible_offset: usize) -> usize {
        visible_segments_to_source_caret_offset(
            &self.segments,
            self.range,
            self.visible_len,
            visible_offset,
        )
    }
}

#[derive(Clone, Copy)]
struct LineSegmentSnapshot {
    visible_range: TextRange,
    source_range: TextRange,
    source_outer_range: TextRange,
}

impl From<ProjectedVisibleSegment<'_>> for LineSegmentSnapshot {
    fn from(segment: ProjectedVisibleSegment<'_>) -> Self {
        Self {
            visible_range: segment.visible_range,
            source_range: segment.source_range,
            source_outer_range: segment.source_outer_range,
        }
    }
}

fn selection_range_from_anchor_and_head(anchor: usize, head: usize) -> TextRange {
    TextRange::new(anchor.min(head), anchor.max(head))
}

fn selection_is_reversed(anchor: usize, head: usize) -> bool {
    head < anchor
}

fn extension_points_for_selection(selection: TextRange, direction: isize) -> (usize, usize) {
    if selection.is_empty() {
        (selection.start, selection.start)
    } else if direction < 0 {
        (selection.end, selection.start)
    } else {
        (selection.start, selection.end)
    }
}

fn horizontal_offset_within_line(
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

fn blockquote_newline_edit_for_line(
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

fn list_newline_edit_for_line(
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

fn ordered_list_delimiter(delimiter: OrderedListDelimiter) -> &'static str {
    match delimiter {
        OrderedListDelimiter::Dot => ".",
        OrderedListDelimiter::Paren => ")",
    }
}

fn drag_distance_exceeds_threshold(
    origin: gpui::Point<Pixels>,
    position: gpui::Point<Pixels>,
) -> bool {
    (position - origin).magnitude() > DRAG_SELECTION_THRESHOLD
}

#[derive(Clone, Copy)]
enum MarkerHitMode {
    ContentStart,
    MarkerStart,
}

fn line_marker_hit_offset(
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

fn bounds_contains_point(bounds: Bounds<Pixels>, position: gpui::Point<Pixels>) -> bool {
    position.x >= bounds.left()
        && position.x <= bounds.right()
        && position.y >= bounds.top()
        && position.y <= bounds.bottom()
}

fn task_marker_state_char_range(text: &str, marker_range: TextRange) -> Option<Range<usize>> {
    let marker_source = text.get(marker_range.start..marker_range.end)?;
    let marker_offset = marker_source
        .find("[ ]")
        .or_else(|| marker_source.find("[x]"))
        .or_else(|| marker_source.find("[X]"))?;
    let state_offset = marker_range.start + marker_offset + 1;

    Some(state_offset..state_offset + 1)
}

fn source_to_visible_offset_in_segments(
    segments: &[LineSegmentSnapshot],
    line_range: TextRange,
    visible_len: usize,
    source_offset: usize,
) -> usize {
    let source_offset = source_offset.clamp(line_range.start, line_range.end);

    for segment in segments {
        if source_offset < segment.source_outer_range.start {
            return segment.visible_range.start;
        }

        if source_offset <= segment.source_outer_range.end {
            if source_offset < segment.source_range.start {
                return segment.visible_range.start;
            }

            if source_offset <= segment.source_range.end {
                return segment.visible_range.start + source_offset - segment.source_range.start;
            }

            return segment.visible_range.end;
        }
    }

    visible_len
}

fn visible_segments_to_source_caret_offset(
    segments: &[LineSegmentSnapshot],
    line_range: TextRange,
    visible_len: usize,
    visible_offset: usize,
) -> usize {
    let visible_offset = visible_offset.min(visible_len);

    for (index, segment) in segments.iter().enumerate() {
        if visible_offset < segment.visible_range.start {
            return segment.source_outer_range.start;
        }

        if visible_offset > segment.visible_range.end {
            continue;
        }

        if visible_offset == segment.visible_range.start {
            if segment.source_range.start > segment.source_outer_range.start {
                return segment.source_range.start;
            }

            if let Some(previous_segment) =
                index.checked_sub(1).and_then(|index| segments.get(index))
                && previous_segment.visible_range.end == visible_offset
                && previous_segment.source_range.end < previous_segment.source_outer_range.end
            {
                return previous_segment.source_range.end;
            }

            return segment.source_range.start;
        }

        if visible_offset == segment.visible_range.end {
            if segment.source_range.end < segment.source_outer_range.end {
                return segment.source_range.end;
            }

            if let Some(next_segment) = segments.get(index + 1)
                && next_segment.visible_range.start == visible_offset
            {
                if next_segment.source_range.start > next_segment.source_outer_range.start {
                    return next_segment.source_range.start;
                }

                return next_segment.source_range.start;
            }

            return segment.source_range.end;
        }

        return segment.source_range.start + visible_offset - segment.visible_range.start;
    }

    line_range.end
}

struct EditorElement {
    editor: Entity<Hanji>,
}

struct EditorPrepaintState {
    lines: Vec<LineSnapshot>,
    blockquote_bars: Vec<PaintQuad>,
    list_markers: Vec<ListMarkerSnapshot>,
    task_marker_hitboxes: Vec<TaskMarkerHitbox>,
    code_backgrounds: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
    selections: Vec<PaintQuad>,
}

enum ListMarkerSnapshot {
    Text(TextListMarkerSnapshot),
    Checkbox(CheckboxMarkerSnapshot),
}

struct TextListMarkerSnapshot {
    layout: ShapedLine,
    origin: gpui::Point<Pixels>,
    height: Pixels,
    bounds: Bounds<Pixels>,
}

struct CheckboxMarkerSnapshot {
    box_quad: PaintQuad,
    check_layout: Option<ShapedLine>,
    check_origin: gpui::Point<Pixels>,
    check_height: Pixels,
    bounds: Bounds<Pixels>,
}

impl ListMarkerSnapshot {
    fn bounds(&self) -> Bounds<Pixels> {
        match self {
            Self::Text(marker) => marker.bounds,
            Self::Checkbox(marker) => marker.bounds,
        }
    }

    fn paint(&self, window: &mut Window, cx: &mut App) {
        match self {
            Self::Text(marker) => {
                marker
                    .layout
                    .paint(marker.origin, marker.height, window, cx)
                    .ok();
            }
            Self::Checkbox(marker) => {
                window.paint_quad(marker.box_quad.clone());
                if let Some(check_layout) = &marker.check_layout {
                    check_layout
                        .paint(marker.check_origin, marker.check_height, window, cx)
                        .ok();
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct HiddenListMarkerGeometry {
    content_offset_width: Pixels,
    marker_bounds: Bounds<Pixels>,
}

#[derive(Clone, Copy)]
struct TaskMarkerHitbox {
    bounds: Bounds<Pixels>,
    marker_range: TextRange,
    state: MarkdownTaskState,
}

#[derive(Clone, Copy)]
struct LinePresentation {
    font_size: f32,
    line_height: f32,
    is_heading: bool,
    is_blockquote: bool,
    is_checked_task: bool,
    text_indent: f32,
}

impl IntoElement for EditorElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for EditorElement {
    type RequestLayoutState = ();
    type PrepaintState = EditorPrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let editor = self.editor.read(cx);
        let document = editor.session.document();
        let projection = project_document(document);
        let mut height = 0.0;

        for line in projection.lines() {
            height += line_presentation(line.kind).line_height;
        }

        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = px(height.max(LINE_HEIGHT)).into();

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let editor = self.editor.read(cx);
        let text_style = window.text_style();
        let mut lines = Vec::new();
        let mut blockquote_bar_runs = Vec::new();
        let mut blockquote_bar_run = None;
        let mut list_markers = Vec::new();
        let mut task_marker_hitboxes = Vec::new();
        let mut code_backgrounds = Vec::new();
        let mut top = 0.0;

        let document = editor.session.document();
        let selection = document.selection().primary();

        let projection = project_document(document);

        for line in projection.lines() {
            let presentation = line_presentation(line.kind);
            let visible_segments = line.visible_segments_revealing_source_in(Some(selection));
            let list_marker_geometry = hidden_list_marker_geometry(
                line.kind,
                line.source,
                line.range,
                line.marker_range,
                &visible_segments,
                Bounds::new(
                    point(bounds.left(), bounds.top() + px(top)),
                    size(bounds.size.width, px(presentation.line_height)),
                ),
                presentation,
                &text_style,
                window,
            );
            let line_text: SharedString = visible_text_from_segments(&visible_segments).into();
            let runs = line_text_runs(&visible_segments, presentation, &text_style);
            let layout =
                window
                    .text_system()
                    .shape_line(line_text, px(presentation.font_size), &runs, None);
            let container_bounds = Bounds::new(
                point(bounds.left(), bounds.top() + px(top)),
                size(bounds.size.width, px(presentation.line_height)),
            );
            let line_bounds = Bounds::new(
                point(
                    bounds.left()
                        + px(presentation.text_indent)
                        + list_marker_geometry
                            .map_or(px(0.0), |geometry| geometry.content_offset_width),
                    bounds.top() + px(top),
                ),
                size(bounds.size.width, px(presentation.line_height)),
            );
            record_blockquote_bar_run(
                &mut blockquote_bar_runs,
                &mut blockquote_bar_run,
                container_bounds,
                presentation.is_blockquote,
            );
            if let MarkdownLine::ListItem { marker, task } = line.kind
                && !line_marker_is_revealed(&visible_segments)
            {
                let marker_snapshot = list_marker_snapshot(
                    marker,
                    task,
                    list_marker_geometry
                        .map_or(container_bounds, |geometry| geometry.marker_bounds),
                    presentation,
                    &text_style,
                    window,
                );
                if let Some(state) = task
                    && let Some(marker_range) = line.marker_range
                {
                    task_marker_hitboxes.push(TaskMarkerHitbox {
                        bounds: marker_snapshot.bounds(),
                        marker_range,
                        state,
                    });
                }
                list_markers.push(marker_snapshot);
            }
            code_backgrounds.extend(code_background_quads(
                &visible_segments,
                &layout,
                line_bounds,
            ));
            top += presentation.line_height;
            let visible_len = visible_segments
                .last()
                .map_or(0, |segment| segment.visible_range.end);

            lines.push(LineSnapshot {
                range: line.range,
                marker_range: line.marker_range,
                visible_len,
                segments: visible_segments.into_iter().map(Into::into).collect(),
                layout,
                bounds: line_bounds,
            });
        }
        flush_blockquote_bar_run(&mut blockquote_bar_runs, &mut blockquote_bar_run);
        let blockquote_bars = blockquote_bar_runs
            .into_iter()
            .map(blockquote_bar_quad)
            .collect();

        let cursor = if selection.is_empty() {
            caret_quad(&lines, selection.start)
        } else {
            None
        };
        let selections = selection_quads(&lines, selection);

        EditorPrepaintState {
            lines,
            blockquote_bars,
            list_markers,
            task_marker_hitboxes,
            code_backgrounds,
            cursor,
            selections,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.editor.clone()),
            cx,
        );

        for bar in prepaint.blockquote_bars.drain(..) {
            window.paint_quad(bar);
        }

        for marker in prepaint.list_markers.drain(..) {
            marker.paint(window, cx);
        }

        for background in prepaint.code_backgrounds.drain(..) {
            window.paint_quad(background);
        }

        for selection in prepaint.selections.drain(..) {
            window.paint_quad(selection);
        }

        for line in &prepaint.lines {
            line.layout
                .paint(
                    line.bounds.origin,
                    line.bounds.bottom() - line.bounds.top(),
                    window,
                    cx,
                )
                .ok();
        }

        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        let lines = prepaint.lines.clone();
        let task_markers = prepaint.task_marker_hitboxes.clone();
        self.editor.update(cx, |editor, _cx| {
            editor.last_lines = lines;
            editor.last_task_markers = task_markers;
        });
    }
}

fn visible_text_from_segments(segments: &[ProjectedVisibleSegment<'_>]) -> String {
    let mut text = String::new();

    for segment in segments {
        text.push_str(segment.source);
    }

    text
}

fn line_marker_is_revealed(segments: &[ProjectedVisibleSegment<'_>]) -> bool {
    segments
        .iter()
        .any(|segment| matches!(segment.kind, ProjectedSegmentKind::ListMarker))
}

fn list_marker_snapshot(
    marker: MarkdownListMarker,
    task: Option<MarkdownTaskState>,
    bounds: Bounds<Pixels>,
    presentation: LinePresentation,
    text_style: &TextStyle,
    window: &mut Window,
) -> ListMarkerSnapshot {
    if let Some(task) = task {
        return checkbox_marker_snapshot(task, bounds, text_style, window);
    }

    let marker_text: SharedString = list_marker_preview_text(marker, task).into();
    let mut font = text_style.font();
    font.weight = FontWeight::BOLD;
    let runs = vec![TextRun {
        len: marker_text.len(),
        font,
        color: rgb(0x5f6267).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    }];
    let layout = window.text_system().shape_line(
        marker_text,
        px(list_marker_font_size(marker, task, presentation)),
        &runs,
        None,
    );
    let marker_left = if matches!(marker, MarkdownListMarker::Unordered { .. }) {
        bounds.left() + ((bounds.right() - bounds.left() - layout.width) / 2.0).max(px(0.0))
    } else {
        bounds.left()
    };
    let origin = point(marker_left, bounds.top());

    ListMarkerSnapshot::Text(TextListMarkerSnapshot {
        layout,
        origin,
        height: px(presentation.line_height),
        bounds,
    })
}

fn checkbox_marker_snapshot(
    task: MarkdownTaskState,
    bounds: Bounds<Pixels>,
    text_style: &TextStyle,
    window: &mut Window,
) -> ListMarkerSnapshot {
    let box_size = px(CHECKBOX_BOX_SIZE);
    let line_height = bounds.bottom() - bounds.top();
    let bounds_width = bounds.right() - bounds.left();
    let box_left = bounds.left() + ((bounds_width - box_size) / 2.0).max(px(0.0));
    let box_top = bounds.top() + (line_height - box_size) / 2.0;
    let box_bounds = Bounds::new(point(box_left, box_top), size(box_size, box_size));
    let checked = matches!(task, MarkdownTaskState::Checked);
    let box_quad = quad(
        box_bounds,
        px(3.0),
        if checked {
            rgb(0x25231f)
        } else {
            rgb(0xfffefb)
        },
        px(1.4),
        rgb(0x25231f),
        BorderStyle::Solid,
    );
    let (check_layout, check_origin) = if checked {
        let check_text: SharedString = "\u{2713}".into();
        let mut font = text_style.font();
        font.weight = FontWeight::BOLD;
        let runs = vec![TextRun {
            len: check_text.len(),
            font,
            color: rgb(0xfffefb).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }];
        let layout =
            window
                .text_system()
                .shape_line(check_text, px(CHECKBOX_CHECK_FONT_SIZE), &runs, None);
        let origin = point(
            box_bounds.left() + (box_size - layout.width) / 2.0,
            box_bounds.top(),
        );

        (Some(layout), origin)
    } else {
        (None, box_bounds.origin)
    };

    ListMarkerSnapshot::Checkbox(CheckboxMarkerSnapshot {
        box_quad,
        check_layout,
        check_origin,
        check_height: box_size,
        bounds,
    })
}

fn hidden_list_marker_geometry(
    line_kind: MarkdownLine,
    line_source: &str,
    line_range: TextRange,
    marker_range: Option<TextRange>,
    visible_segments: &[ProjectedVisibleSegment<'_>],
    container_bounds: Bounds<Pixels>,
    presentation: LinePresentation,
    text_style: &TextStyle,
    window: &mut Window,
) -> Option<HiddenListMarkerGeometry> {
    if !matches!(line_kind, MarkdownLine::ListItem { .. })
        || line_marker_is_revealed(visible_segments)
    {
        return None;
    }

    let line_kind = match line_kind {
        MarkdownLine::ListItem { marker, task } => MarkdownLine::ListItem { marker, task },
        _ => return None,
    };
    let marker_range = marker_range?;
    let marker_source = line_source
        .get(marker_range.start - line_range.start..marker_range.end - line_range.start)?;
    let indent_len = list_marker_indent_len(marker_source);
    let content_marker_source = hidden_list_content_marker_source(marker_source, line_kind);
    let marker_cell_width = text_width(
        content_marker_source,
        presentation.font_size,
        text_style,
        window,
    );
    let content_offset_width = marker_cell_width + px(hidden_list_content_extra_gap(line_kind));
    let indent_width = text_width(
        &marker_source[..indent_len],
        presentation.font_size,
        text_style,
        window,
    );
    let marker_bounds = Bounds::new(
        point(
            container_bounds.left() + px(presentation.text_indent) + indent_width,
            container_bounds.top(),
        ),
        size(
            marker_cell_width - indent_width,
            px(presentation.line_height),
        ),
    );

    Some(HiddenListMarkerGeometry {
        content_offset_width,
        marker_bounds,
    })
}

fn list_marker_indent_len(source: &str) -> usize {
    source.bytes().take_while(|byte| *byte == b' ').count()
}

fn hidden_list_content_marker_source(source: &str, line_kind: MarkdownLine) -> &str {
    if !matches!(line_kind, MarkdownLine::ListItem { task: Some(_), .. }) {
        return source;
    }

    &source[..task_list_base_marker_end(source)]
}

fn hidden_list_content_extra_gap(line_kind: MarkdownLine) -> f32 {
    if matches!(line_kind, MarkdownLine::ListItem { task: Some(_), .. }) {
        CHECKBOX_CONTENT_GAP
    } else {
        0.0
    }
}

fn task_list_base_marker_end(source: &str) -> usize {
    let bytes = source.as_bytes();
    let indent_len = list_marker_indent_len(source);
    let marker_end = match bytes.get(indent_len) {
        Some(b'-' | b'*' | b'+') => indent_len + 1,
        Some(byte) if byte.is_ascii_digit() => {
            let digit_len = bytes[indent_len..]
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            indent_len + digit_len + 1
        }
        _ => return indent_len,
    };
    let padding = bytes[marker_end..]
        .iter()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();

    marker_end + padding
}

fn text_width(source: &str, font_size: f32, text_style: &TextStyle, window: &mut Window) -> Pixels {
    if source.is_empty() {
        return px(0.0);
    }

    let text: SharedString = source.to_string().into();
    let runs = vec![TextRun {
        len: text.len(),
        font: text_style.font(),
        color: rgb(0x25231f).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    }];

    window
        .text_system()
        .shape_line(text, px(font_size), &runs, None)
        .width
}

fn list_marker_font_size(
    marker: MarkdownListMarker,
    task: Option<MarkdownTaskState>,
    presentation: LinePresentation,
) -> f32 {
    if task.is_some() {
        CHECKBOX_MARKER_FONT_SIZE
    } else if matches!(marker, MarkdownListMarker::Unordered { .. }) {
        LIST_BULLET_FONT_SIZE
    } else {
        presentation.font_size
    }
}

fn list_marker_preview_text(marker: MarkdownListMarker, task: Option<MarkdownTaskState>) -> String {
    if let Some(task) = task {
        return task_marker_preview_text(task).to_string();
    }

    match marker {
        MarkdownListMarker::Unordered { .. } => "\u{2022}".to_string(),
        MarkdownListMarker::Ordered { number, delimiter } => {
            format!("{number}{}", ordered_list_delimiter(delimiter))
        }
    }
}

fn task_marker_preview_text(task: MarkdownTaskState) -> &'static str {
    match task {
        MarkdownTaskState::Unchecked => "\u{2610}",
        MarkdownTaskState::Checked => "\u{2611}",
    }
}

fn line_text_runs(
    segments: &[ProjectedVisibleSegment<'_>],
    presentation: LinePresentation,
    text_style: &TextStyle,
) -> Vec<TextRun> {
    let mut runs = Vec::new();

    for segment in segments {
        let style = match segment.kind {
            ProjectedSegmentKind::StrongContent => InlineRunStyle::Strong,
            ProjectedSegmentKind::EmphasisContent => InlineRunStyle::Emphasis,
            ProjectedSegmentKind::CodeContent => InlineRunStyle::Code,
            ProjectedSegmentKind::HeadingMarker
            | ProjectedSegmentKind::BlockquoteMarker
            | ProjectedSegmentKind::ListMarker
            | ProjectedSegmentKind::StrongMarker
            | ProjectedSegmentKind::EmphasisMarker
            | ProjectedSegmentKind::CodeMarker => InlineRunStyle::Marker,
            ProjectedSegmentKind::Text => InlineRunStyle::Plain,
        };

        push_text_run(
            &mut runs,
            segment.source.len(),
            style,
            presentation,
            text_style,
        );
    }

    runs
}

#[derive(Clone, Copy)]
enum InlineRunStyle {
    Plain,
    Marker,
    Strong,
    Emphasis,
    Code,
}

fn push_text_run(
    runs: &mut Vec<TextRun>,
    len: usize,
    style: InlineRunStyle,
    presentation: LinePresentation,
    text_style: &TextStyle,
) {
    if len == 0 {
        return;
    }

    let mut font = text_style.font();
    if presentation.is_heading || matches!(style, InlineRunStyle::Strong) {
        font.weight = if matches!(style, InlineRunStyle::Strong) {
            FontWeight::BLACK
        } else {
            FontWeight::BOLD
        };
    }
    if matches!(style, InlineRunStyle::Emphasis) {
        font = font.italic();
    }
    let color = if matches!(style, InlineRunStyle::Marker) {
        rgb(MARKDOWN_MARKER_COLOR).into()
    } else if presentation.is_heading {
        rgb(0x25231f).into()
    } else if presentation.is_blockquote {
        rgb(0x5f6267).into()
    } else if presentation.is_checked_task && !matches!(style, InlineRunStyle::Marker) {
        rgb(0x8f8a82).into()
    } else {
        text_style.color
    };

    let underline = if matches!(style, InlineRunStyle::Strong | InlineRunStyle::Emphasis) {
        Some(font_run_boundary_marker())
    } else {
        None
    };

    runs.push(TextRun {
        len,
        font,
        color,
        background_color: None,
        underline,
        strikethrough: None,
    });
}

fn font_run_boundary_marker() -> UnderlineStyle {
    // GPUI 0.2.2 can merge runs when only the font changes.
    // A zero-width transparent underline separates the font run without drawing.
    UnderlineStyle {
        thickness: px(0.0),
        color: Some(rgba(0x00000000).into()),
        wavy: false,
    }
}

fn code_background_ranges(segments: &[ProjectedVisibleSegment<'_>]) -> Vec<TextRange> {
    segments
        .iter()
        .filter_map(|segment| match segment.kind {
            ProjectedSegmentKind::CodeMarker | ProjectedSegmentKind::CodeContent => {
                Some(segment.visible_range)
            }
            ProjectedSegmentKind::Text
            | ProjectedSegmentKind::HeadingMarker
            | ProjectedSegmentKind::BlockquoteMarker
            | ProjectedSegmentKind::ListMarker
            | ProjectedSegmentKind::StrongMarker
            | ProjectedSegmentKind::StrongContent
            | ProjectedSegmentKind::EmphasisMarker
            | ProjectedSegmentKind::EmphasisContent => None,
        })
        .collect()
}

fn blockquote_bar_quad(bounds: Bounds<Pixels>) -> PaintQuad {
    fill(blockquote_bar_bounds(bounds), rgb(0xaeb6bf))
}

fn blockquote_bar_bounds(bounds: Bounds<Pixels>) -> Bounds<Pixels> {
    Bounds::new(
        point(
            bounds.left() + px(BLOCKQUOTE_BAR_LEFT_INSET),
            bounds.top() + px(BLOCKQUOTE_BAR_TOP_INSET),
        ),
        size(
            px(BLOCKQUOTE_BAR_WIDTH),
            bounds.bottom()
                - bounds.top()
                - px(BLOCKQUOTE_BAR_TOP_INSET)
                - px(BLOCKQUOTE_BAR_BOTTOM_INSET),
        ),
    )
}

fn record_blockquote_bar_run(
    runs: &mut Vec<Bounds<Pixels>>,
    current_run: &mut Option<Bounds<Pixels>>,
    bounds: Bounds<Pixels>,
    is_blockquote: bool,
) {
    if is_blockquote {
        let bounds = match current_run.take() {
            Some(run) => merge_blockquote_bar_bounds(run, bounds),
            None => bounds,
        };
        *current_run = Some(bounds);
    } else {
        flush_blockquote_bar_run(runs, current_run);
    }
}

fn flush_blockquote_bar_run(
    runs: &mut Vec<Bounds<Pixels>>,
    current_run: &mut Option<Bounds<Pixels>>,
) {
    if let Some(run) = current_run.take() {
        runs.push(run);
    }
}

fn merge_blockquote_bar_bounds(start: Bounds<Pixels>, end: Bounds<Pixels>) -> Bounds<Pixels> {
    Bounds::from_corners(
        point(start.left(), start.top()),
        point(start.right(), end.bottom()),
    )
}

#[cfg(test)]
fn collect_blockquote_bar_bounds(lines: &[(Bounds<Pixels>, bool)]) -> Vec<Bounds<Pixels>> {
    let mut runs = Vec::new();
    let mut current_run = None;

    for (bounds, is_blockquote) in lines {
        record_blockquote_bar_run(&mut runs, &mut current_run, *bounds, *is_blockquote);
    }

    flush_blockquote_bar_run(&mut runs, &mut current_run);
    runs
}

fn code_background_quads(
    segments: &[ProjectedVisibleSegment<'_>],
    layout: &ShapedLine,
    bounds: Bounds<Pixels>,
) -> Vec<PaintQuad> {
    code_background_ranges(segments)
        .into_iter()
        .filter_map(|range| code_background_quad(layout, bounds, range))
        .collect()
}

fn code_background_quad(
    layout: &ShapedLine,
    bounds: Bounds<Pixels>,
    range: TextRange,
) -> Option<PaintQuad> {
    if range.is_empty() {
        return None;
    }

    let start_x = layout.x_for_index(range.start);
    let end_x = layout.x_for_index(range.end);

    Some(fill(
        Bounds::from_corners(
            point(bounds.left() + start_x, bounds.top()),
            point(bounds.left() + end_x, bounds.bottom()),
        ),
        rgba(0x25231f2a),
    ))
}

fn line_presentation(line: MarkdownLine) -> LinePresentation {
    match line {
        MarkdownLine::Heading { level } => {
            let font_size = match level {
                1 => 24.0,
                2 => 21.0,
                3 => 19.0,
                _ => 17.0,
            };

            LinePresentation {
                font_size,
                line_height: font_size + 12.0,
                is_heading: true,
                is_blockquote: false,
                is_checked_task: false,
                text_indent: 0.0,
            }
        }
        MarkdownLine::Blockquote => LinePresentation {
            font_size: FONT_SIZE,
            line_height: LINE_HEIGHT,
            is_heading: false,
            is_blockquote: true,
            is_checked_task: false,
            text_indent: 18.0,
        },
        MarkdownLine::ListItem { task, .. } => LinePresentation {
            font_size: FONT_SIZE,
            line_height: LINE_HEIGHT,
            is_heading: false,
            is_blockquote: false,
            is_checked_task: matches!(task, Some(MarkdownTaskState::Checked)),
            text_indent: 0.0,
        },
        MarkdownLine::Blank | MarkdownLine::Paragraph => LinePresentation {
            font_size: FONT_SIZE,
            line_height: LINE_HEIGHT,
            is_heading: false,
            is_blockquote: false,
            is_checked_task: false,
            text_indent: 0.0,
        },
    }
}

fn caret_quad(lines: &[LineSnapshot], offset: usize) -> Option<PaintQuad> {
    let line = line_for_offset(lines, offset)?;
    let visible_offset = line.source_to_visible_offset(offset);
    let x = line.layout.x_for_index(visible_offset);

    Some(fill(
        Bounds::new(
            point(line.bounds.left() + x, line.bounds.top()),
            size(px(2.0), line.bounds.bottom() - line.bounds.top()),
        ),
        rgb(0x25231f),
    ))
}

fn selection_quads(lines: &[LineSnapshot], selection: TextRange) -> Vec<PaintQuad> {
    if selection.is_empty() {
        return Vec::new();
    }

    lines
        .iter()
        .filter_map(|line| {
            let start = selection.start.max(line.range.start);
            let end = selection.end.min(line.range.end);

            if start >= end {
                return None;
            }

            let visible_start = line.source_to_visible_offset(start);
            let visible_end = line.source_to_visible_offset(end);

            if visible_start >= visible_end {
                return None;
            }

            let start_x = line.layout.x_for_index(visible_start);
            let end_x = line.layout.x_for_index(visible_end);

            Some(fill(
                Bounds::from_corners(
                    point(line.bounds.left() + start_x, line.bounds.top()),
                    point(line.bounds.left() + end_x, line.bounds.bottom()),
                ),
                rgba(0x276ef140),
            ))
        })
        .collect()
}

fn line_for_offset(lines: &[LineSnapshot], offset: usize) -> Option<&LineSnapshot> {
    lines
        .iter()
        .find(|line| offset >= line.range.start && offset <= line.range.end)
        .or_else(|| lines.last())
}

fn byte_offset_to_utf16(text: &str, offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut byte_offset = 0;

    for character in text.chars() {
        if byte_offset >= offset {
            break;
        }

        byte_offset += character.len_utf8();
        utf16_offset += character.len_utf16();
    }

    utf16_offset
}

fn utf16_offset_to_byte(text: &str, offset: usize) -> usize {
    let mut byte_offset = 0;
    let mut utf16_offset = 0;

    for character in text.chars() {
        if utf16_offset >= offset {
            break;
        }

        utf16_offset += character.len_utf16();
        byte_offset += character.len_utf8();
    }

    byte_offset
}

fn utf16_range_to_byte(text: &str, range: &Range<usize>) -> Range<usize> {
    utf16_offset_to_byte(text, range.start)..utf16_offset_to_byte(text, range.end)
}

fn open_initial_session() -> io::Result<DocumentSession> {
    let Some(path) = env::args_os().nth(1).map(PathBuf::from) else {
        return Ok(DocumentSession::new(
            scratch_document_path(),
            SAMPLE_DOCUMENT,
        ));
    };

    if path.exists() {
        DocumentSession::open(path)
    } else {
        Ok(DocumentSession::new(path, ""))
    }
}

fn scratch_document_path() -> PathBuf {
    env::temp_dir().join("hanji-scratch.md")
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::FontStyle;
    use hanji_core::Document;

    #[test]
    fn inline_code_background_ranges_survive_without_strong_runs() {
        let document = Document::new("Capture thought with `code`.");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments = line.visible_segments();

        assert_eq!(
            code_background_ranges(&segments),
            vec![TextRange::new(
                "Capture thought with ".len(),
                "Capture thought with code".len()
            )]
        );
    }

    #[test]
    fn inline_code_uses_source_backed_background_instead_of_text_run_background() {
        let document = Document::new("Capture **thought** with `code`.");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments = line.visible_segments();
        let runs = line_text_runs(
            &segments,
            line_presentation(line.kind),
            &TextStyle::default(),
        );

        assert!(runs.iter().all(|run| run.background_color.is_none()));
        assert!(
            runs.iter()
                .find(|run| run.font.weight == FontWeight::BLACK)
                .is_some_and(|run| run.underline.is_some())
        );
        assert_eq!(
            code_background_ranges(&segments),
            vec![TextRange::new(
                "Capture thought with ".len(),
                "Capture thought with code".len()
            )]
        );
    }

    #[test]
    fn inline_code_background_survives_broken_strong_marker() {
        let document = Document::new("Capture **thought* with `code`.");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments = line.visible_segments();

        assert_eq!(
            code_background_ranges(&segments),
            vec![TextRange::new(
                "Capture **thought* with ".len(),
                "Capture **thought* with code".len()
            )]
        );
    }

    #[test]
    fn blockquote_lines_use_indented_presentation() {
        let presentation = line_presentation(MarkdownLine::Blockquote);

        assert!(!presentation.is_heading);
        assert!(presentation.is_blockquote);
        assert_eq!(presentation.font_size, FONT_SIZE);
        assert_eq!(presentation.line_height, LINE_HEIGHT);
        assert!(!presentation.is_checked_task);
        assert!(presentation.text_indent > 0.0);
    }

    #[test]
    fn list_lines_align_content_with_plain_text() {
        let presentation = line_presentation(MarkdownLine::ListItem {
            marker: MarkdownListMarker::Unordered { marker: '-' },
            task: None,
        });

        assert!(!presentation.is_heading);
        assert!(!presentation.is_blockquote);
        assert_eq!(presentation.font_size, FONT_SIZE);
        assert_eq!(presentation.line_height, LINE_HEIGHT);
        assert!(!presentation.is_checked_task);
        assert_eq!(presentation.text_indent, 0.0);
    }

    #[test]
    fn checked_task_content_uses_dim_text_color() {
        let document = Document::new("- [x] Done");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let presentation = line_presentation(line.kind);
        let runs = line_text_runs(
            &line.visible_segments(),
            presentation,
            &TextStyle::default(),
        );

        assert!(presentation.is_checked_task);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].color, rgb(0x8f8a82).into());
    }

    #[test]
    fn heading_preview_hides_hash_marker_outside_caret() {
        let document = Document::new("# Hanji");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let presentation = line_presentation(line.kind);
        let runs = line_text_runs(
            &line.visible_segments(),
            presentation,
            &TextStyle::default(),
        );

        assert!(presentation.is_heading);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].font.weight, FontWeight::BOLD);
        assert_eq!(runs[0].color, rgb(0x25231f).into());
    }

    #[test]
    fn heading_source_uses_heading_weight_with_green_hash_marker() {
        let document = Document::new("# Hanji");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let presentation = line_presentation(line.kind);
        let runs = line_text_runs(
            &line.visible_segments_revealing_source_in(Some(TextRange::caret(3))),
            presentation,
            &TextStyle::default(),
        );

        assert!(presentation.is_heading);
        assert_eq!(runs.len(), 3);
        assert!(runs.iter().all(|run| run.font.weight == FontWeight::BOLD));
        assert_eq!(runs[0].color, rgb(MARKDOWN_MARKER_COLOR).into());
        assert_eq!(runs[1].color, rgb(0x25231f).into());
        assert_eq!(runs[2].color, rgb(0x25231f).into());
    }

    #[test]
    fn empty_heading_source_colors_only_hash_as_marker() {
        let document = Document::new("# ");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let presentation = line_presentation(line.kind);
        let runs = line_text_runs(
            &line.visible_segments_revealing_source_in(Some(TextRange::caret(2))),
            presentation,
            &TextStyle::default(),
        );

        assert!(presentation.is_heading);
        assert_eq!(runs.len(), 2);
        assert!(runs.iter().all(|run| run.font.weight == FontWeight::BOLD));
        assert_eq!(runs[0].color, rgb(MARKDOWN_MARKER_COLOR).into());
        assert_eq!(runs[1].color, rgb(0x25231f).into());
    }

    #[test]
    fn revealed_inline_markers_use_green() {
        let source = "This is **bold** and `code`";
        let document = Document::new(source);
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments = line.visible_segments_revealing_source_in(Some(TextRange::new(
            "This is ".len(),
            source.len(),
        )));
        let runs = line_text_runs(
            &segments,
            line_presentation(line.kind),
            &TextStyle::default(),
        );

        for (segment, run) in segments.iter().zip(runs.iter()) {
            if matches!(
                segment.kind,
                ProjectedSegmentKind::StrongMarker | ProjectedSegmentKind::CodeMarker
            ) {
                assert_eq!(run.color, rgb(MARKDOWN_MARKER_COLOR).into());
            }
        }
    }

    #[test]
    fn list_marker_preview_text_uses_visual_markers() {
        assert_eq!(
            list_marker_preview_text(MarkdownListMarker::Unordered { marker: '-' }, None),
            "\u{2022}"
        );
        assert_eq!(
            list_marker_preview_text(
                MarkdownListMarker::Ordered {
                    number: 3,
                    delimiter: OrderedListDelimiter::Dot,
                },
                None
            ),
            "3."
        );
        assert_eq!(
            list_marker_preview_text(
                MarkdownListMarker::Ordered {
                    number: 3,
                    delimiter: OrderedListDelimiter::Paren,
                },
                None
            ),
            "3)"
        );
    }

    #[test]
    fn unordered_bullet_marker_uses_smaller_preview_font() {
        let presentation = line_presentation(MarkdownLine::ListItem {
            marker: MarkdownListMarker::Unordered { marker: '-' },
            task: None,
        });

        assert_eq!(
            list_marker_font_size(
                MarkdownListMarker::Unordered { marker: '-' },
                None,
                presentation
            ),
            LIST_BULLET_FONT_SIZE
        );
        assert_eq!(
            list_marker_font_size(
                MarkdownListMarker::Ordered {
                    number: 1,
                    delimiter: OrderedListDelimiter::Dot,
                },
                None,
                presentation
            ),
            FONT_SIZE
        );
        assert_eq!(
            list_marker_font_size(
                MarkdownListMarker::Unordered { marker: '-' },
                Some(MarkdownTaskState::Unchecked),
                presentation
            ),
            CHECKBOX_MARKER_FONT_SIZE
        );
    }

    #[test]
    fn task_list_marker_preview_text_uses_checkbox_markers() {
        assert_eq!(
            list_marker_preview_text(
                MarkdownListMarker::Unordered { marker: '-' },
                Some(MarkdownTaskState::Unchecked)
            ),
            "\u{2610}"
        );
        assert_eq!(
            list_marker_preview_text(
                MarkdownListMarker::Unordered { marker: '-' },
                Some(MarkdownTaskState::Checked)
            ),
            "\u{2611}"
        );
    }

    #[test]
    fn hidden_task_list_content_uses_base_marker_width() {
        assert_eq!(
            hidden_list_content_marker_source(
                "- [ ] ",
                MarkdownLine::ListItem {
                    marker: MarkdownListMarker::Unordered { marker: '-' },
                    task: Some(MarkdownTaskState::Unchecked),
                },
            ),
            "- "
        );
        assert_eq!(
            hidden_list_content_marker_source(
                "  -   [x] ",
                MarkdownLine::ListItem {
                    marker: MarkdownListMarker::Unordered { marker: '-' },
                    task: Some(MarkdownTaskState::Checked),
                },
            ),
            "  -   "
        );
        assert_eq!(
            hidden_list_content_marker_source(
                "12. [ ] ",
                MarkdownLine::ListItem {
                    marker: MarkdownListMarker::Ordered {
                        number: 12,
                        delimiter: OrderedListDelimiter::Dot,
                    },
                    task: Some(MarkdownTaskState::Unchecked),
                },
            ),
            "12. "
        );
    }

    #[test]
    fn hidden_task_list_content_adds_checkbox_gap() {
        assert_eq!(
            hidden_list_content_extra_gap(MarkdownLine::ListItem {
                marker: MarkdownListMarker::Unordered { marker: '-' },
                task: Some(MarkdownTaskState::Unchecked),
            }),
            CHECKBOX_CONTENT_GAP
        );
        assert_eq!(
            hidden_list_content_extra_gap(MarkdownLine::ListItem {
                marker: MarkdownListMarker::Unordered { marker: '-' },
                task: None,
            }),
            0.0
        );
    }

    #[test]
    fn revealed_list_source_counts_as_line_marker() {
        let document = Document::new("- Item");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert!(!line_marker_is_revealed(&line.visible_segments()));
        assert!(line_marker_is_revealed(
            &line.visible_segments_revealing_source_in(Some(TextRange::caret(1)))
        ));
    }

    #[test]
    fn consecutive_blockquote_lines_share_one_bar_run() {
        let lines = [
            (
                Bounds::new(point(px(0.0), px(0.0)), size(px(100.0), px(24.0))),
                true,
            ),
            (
                Bounds::new(point(px(0.0), px(24.0)), size(px(100.0), px(24.0))),
                true,
            ),
            (
                Bounds::new(point(px(0.0), px(48.0)), size(px(100.0), px(24.0))),
                false,
            ),
            (
                Bounds::new(point(px(0.0), px(72.0)), size(px(100.0), px(24.0))),
                true,
            ),
        ];

        let runs = collect_blockquote_bar_bounds(&lines);

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].top(), px(0.0));
        assert_eq!(runs[0].bottom(), px(48.0));
        assert_eq!(runs[1].top(), px(72.0));
        assert_eq!(runs[1].bottom(), px(96.0));
    }

    #[test]
    fn blockquote_bar_extends_to_run_bottom() {
        let bounds = Bounds::new(point(px(10.0), px(20.0)), size(px(100.0), px(48.0)));
        let bar = blockquote_bar_bounds(bounds);

        assert_eq!(bar.left(), px(14.0));
        assert_eq!(bar.top(), px(22.0));
        assert_eq!(bar.right(), px(17.0));
        assert_eq!(bar.bottom(), px(68.0));
    }

    #[test]
    fn single_asterisk_emphasis_uses_italic_runs() {
        let document = Document::new("Capture *thought* with `code`.");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments = line.visible_segments();
        let runs = line_text_runs(
            &segments,
            line_presentation(line.kind),
            &TextStyle::default(),
        );

        assert!(
            runs.iter()
                .find(|run| run.font.style == FontStyle::Italic)
                .is_some_and(|run| run.underline.is_some())
        );
        assert!(runs.iter().all(|run| run.font.weight != FontWeight::BLACK));
        assert_eq!(
            code_background_ranges(&segments),
            vec![TextRange::new(
                "Capture thought with ".len(),
                "Capture thought with code".len()
            )]
        );
    }

    #[test]
    fn caret_inside_strong_reveals_only_that_inline_source() {
        let document = Document::new("Capture **thought** with `code`.");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments = line
            .visible_segments_revealing_source_in(Some(TextRange::caret("Capture **tho".len())));

        assert_eq!(
            visible_text_from_segments(&segments),
            "Capture **thought** with code."
        );
        assert_eq!(
            code_background_ranges(&segments),
            vec![TextRange::new(
                "Capture **thought** with ".len(),
                "Capture **thought** with code".len()
            )]
        );
    }

    #[test]
    fn caret_inside_code_reveals_code_markers() {
        let document = Document::new("Capture **thought** with `code`.");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments = line.visible_segments_revealing_source_in(Some(TextRange::caret(
            "Capture **thought** with `co".len(),
        )));

        assert_eq!(
            visible_text_from_segments(&segments),
            "Capture thought with `code`."
        );
        assert_eq!(
            code_background_ranges(&segments),
            vec![
                TextRange::new(
                    "Capture thought with ".len(),
                    "Capture thought with `".len()
                ),
                TextRange::new(
                    "Capture thought with `".len(),
                    "Capture thought with `code".len()
                ),
                TextRange::new(
                    "Capture thought with `code".len(),
                    "Capture thought with `code`".len()
                ),
            ]
        );
    }

    #[test]
    fn hidden_inline_boundaries_hit_test_to_marker_edit_edges() {
        let document = Document::new("Capture **thought** with `code`.");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments: Vec<LineSegmentSnapshot> = line
            .visible_segments()
            .into_iter()
            .map(Into::into)
            .collect();
        let visible_len = segments
            .last()
            .map_or(0, |segment| segment.visible_range.end);

        assert_eq!(
            visible_segments_to_source_caret_offset(
                &segments,
                line.range,
                visible_len,
                "Capture ".len()
            ),
            "Capture **".len()
        );
        assert_eq!(
            visible_segments_to_source_caret_offset(
                &segments,
                line.range,
                visible_len,
                "Capture thought".len()
            ),
            "Capture **thought".len()
        );
        assert_eq!(
            visible_segments_to_source_caret_offset(
                &segments,
                line.range,
                visible_len,
                "Capture thought with ".len()
            ),
            "Capture **thought** with `".len()
        );
        assert_eq!(
            visible_segments_to_source_caret_offset(
                &segments,
                line.range,
                visible_len,
                "Capture thought with code".len()
            ),
            "Capture **thought** with `code".len()
        );
    }

    #[test]
    fn source_selection_from_outside_inline_includes_markers() {
        let selection = TextRange::new("Capture ".len(), "Capture **thought".len());

        assert_eq!(
            visible_selection_text("Capture **thought** with `code`.", selection),
            "**thought"
        );
    }

    #[test]
    fn source_selection_from_inside_inline_excludes_markers() {
        let selection = TextRange::new("Capture **".len(), "Capture **thought".len());

        assert_eq!(
            visible_selection_text("Capture **thought** with `code`.", selection),
            "thought"
        );
    }

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

    fn visible_selection_text(source: &str, selection: TextRange) -> String {
        let document = Document::new(source);
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments = line.visible_segments_revealing_source_in(Some(selection));
        let visible_text = visible_text_from_segments(&segments);
        let segments: Vec<LineSegmentSnapshot> = segments.into_iter().map(Into::into).collect();
        let visible_len = segments
            .last()
            .map_or(0, |segment| segment.visible_range.end);
        let visible_start = source_to_visible_offset_in_segments(
            &segments,
            line.range,
            visible_len,
            selection.start,
        );
        let visible_end =
            source_to_visible_offset_in_segments(&segments, line.range, visible_len, selection.end);

        visible_text[visible_start..visible_end].to_string()
    }
}

fn main() {
    let session = open_initial_session().unwrap_or_else(|error| {
        eprintln!("Could not open document: {error}");
        process::exit(1);
    });

    Application::new().run(move |cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("delete", Delete, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("shift-left", ShiftLeft, None),
            KeyBinding::new("shift-right", ShiftRight, None),
            KeyBinding::new("alt-left", OptionLeft, None),
            KeyBinding::new("alt-right", OptionRight, None),
            KeyBinding::new("alt-shift-left", ShiftOptionLeft, None),
            KeyBinding::new("alt-shift-right", ShiftOptionRight, None),
            KeyBinding::new("up", Up, None),
            KeyBinding::new("down", Down, None),
            KeyBinding::new("shift-up", ShiftUp, None),
            KeyBinding::new("shift-down", ShiftDown, None),
            KeyBinding::new("home", Home, None),
            KeyBinding::new("end", End, None),
            KeyBinding::new("cmd-left", Home, None),
            KeyBinding::new("cmd-right", End, None),
            KeyBinding::new("cmd-up", CmdUp, None),
            KeyBinding::new("cmd-down", CmdDown, None),
            KeyBinding::new("cmd-shift-left", ShiftCmdLeft, None),
            KeyBinding::new("cmd-shift-right", ShiftCmdRight, None),
            KeyBinding::new("cmd-shift-up", ShiftCmdUp, None),
            KeyBinding::new("cmd-shift-down", ShiftCmdDown, None),
            KeyBinding::new("enter", Newline, None),
            KeyBinding::new("cmd-b", ToggleStrong, None),
            KeyBinding::new("cmd-i", ToggleItalic, None),
            KeyBinding::new("cmd-e", ToggleCode, None),
            KeyBinding::new("cmd-z", Undo, None),
            KeyBinding::new("cmd-shift-z", Redo, None),
            KeyBinding::new("cmd-s", Save, None),
            KeyBinding::new("cmd-q", Quit, None),
        ]);
        cx.on_action(|_: &Quit, cx| cx.quit());

        let mut session = Some(session);
        let bounds = Bounds::centered(None, size(px(720.0), px(520.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                move |window, cx| {
                    cx.new(|cx| {
                        let editor = Hanji {
                            focus_handle: cx.focus_handle(),
                            session: session.take().expect("initial session was already opened"),
                            marked_range: None,
                            last_lines: Vec::new(),
                            last_task_markers: Vec::new(),
                            preferred_column: None,
                            selection_anchor: None,
                            selection_head: None,
                            is_selecting: false,
                            selection_drag_origin: None,
                            status_message: None,
                        };
                        editor.update_window_title(window);
                        editor
                    })
                },
            )
            .unwrap();

        window
            .update(cx, |view, window, cx| {
                window.focus(&view.focus_handle(cx));
                cx.activate(true);
            })
            .unwrap();
    });
}
