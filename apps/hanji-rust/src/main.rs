mod editing;
mod encoding;
mod external;
mod renderer;
mod session;
mod snapshot;

use std::{ops::Range, process};

use gpui::{
    App, Application, Bounds, ClipboardItem, Context, CursorStyle, EntityInputHandler, FocusHandle,
    Focusable, IntoElement, KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Render, SharedString, UTF16Selection, Window, WindowBounds, WindowOptions, actions,
    div, point, prelude::*, px, rgb, size,
};
use hanji_core::{EditorCommand, Selection, TextPosition, TextRange, Transaction};
use hanji_markdown::{
    MarkdownCommand, MarkdownCommandError, MarkdownTaskState, execute_markdown_command,
};
use hanji_storage::DocumentSession;

use editing::{
    ListIndentDirection, MarkerHitMode, blockquote_newline_edit_for_line, bounds_contains_point,
    clipboard_paste_edit, document_selection_range, drag_distance_exceeds_threshold,
    empty_marker_pair_delete_backward_edit, extension_points_for_selection,
    horizontal_offset_within_line, line_marker_hit_offset, list_indent_edit,
    list_newline_edit_for_line, marker_autocomplete_edit, marker_skip_offset, selected_source_text,
    selection_is_reversed, selection_range_from_anchor_and_head, task_marker_state_char_range,
};
use encoding::byte_offset_to_utf16;
use external::external_url_command;
use renderer::{EditorElement, FONT_SIZE, LINE_HEIGHT, LinkHitbox, TaskMarkerHitbox};
use session::open_initial_session;
use snapshot::LineSnapshot;

fn editor_cursor_style(is_hovering_link: bool) -> CursorStyle {
    if is_hovering_link {
        CursorStyle::PointingHand
    } else {
        CursorStyle::IBeam
    }
}

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
        InsertLink,
        SelectAll,
        IndentList,
        OutdentList,
        Copy,
        Cut,
        Paste,
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
    last_link_hitboxes: Vec<LinkHitbox>,
    hovered_link_url: Option<String>,
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
        let range = self.selected_range();
        if let Some((range, replacement, selection_after)) =
            empty_marker_pair_delete_backward_edit(self.session.document().text(), &range)
        {
            self.replace_range(range, &replacement, selection_after, window, cx);
            return;
        }

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

    fn insert_link(&mut self, _: &InsertLink, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_markdown_command(
            MarkdownCommand::InsertLink,
            "Could not insert link.",
            window,
            cx,
        );
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.is_selecting = false;
        self.selection_drag_origin = None;
        let range = document_selection_range(self.session.document().len());

        self.select_from_anchor_to(range.start, range.end, None, cx);
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        let range = self.selected_range();
        if let Some(text) = selected_source_text(self.session.document().text(), &range) {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        let range = self.selected_range();
        let Some(text) = selected_source_text(self.session.document().text(), &range) else {
            return;
        };

        self.marked_range = None;
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.replace_range(range.clone(), "", range.start..range.start, window, cx);
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        if text.is_empty() {
            return;
        }

        let (range, replacement, selection_after) =
            clipboard_paste_edit(&self.selected_range(), &text);

        self.marked_range = None;
        self.replace_range(range, &replacement, selection_after, window, cx);
    }

    fn indent_list(&mut self, _: &IndentList, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_list_indent(ListIndentDirection::Increase, window, cx);
    }

    fn outdent_list(&mut self, _: &OutdentList, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_list_indent(ListIndentDirection::Decrease, window, cx);
    }

    fn apply_list_indent(
        &mut self,
        direction: ListIndentDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked_range = None;
        let range = self.selected_range();
        let Some((edits, selection_after)) =
            list_indent_edit(self.session.document().text(), &range, direction)
        else {
            return;
        };
        let selection_after =
            Selection::single(TextRange::new(selection_after.start, selection_after.end));
        let transaction = Transaction::new(edits, Some(selection_after));

        if self.session.apply(transaction).is_ok() {
            self.preferred_column = None;
            self.clear_selection_tracking();
            self.document_changed(window, cx);
        }
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
        self.update_hovered_link_at_position(event.position, cx);
        if !event.modifiers.shift
            && let Some(task_marker) = self.task_marker_at_position(event.position)
            && self.toggle_task_marker(task_marker, window, cx)
        {
            return;
        }
        if !event.modifiers.shift
            && let Some(link) = self.link_at_position(event.position)
            && self.open_link(link, cx)
        {
            return;
        }

        let offset = self.index_for_mouse_position(event.position);
        if !event.modifiers.shift
            && event.click_count >= 2
            && let Some(range) = self.word_selection_range_for_offset(offset)
        {
            self.is_selecting = false;
            self.selection_drag_origin = None;
            self.select_from_anchor_to(range.start, range.end, None, cx);
            return;
        }

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
            self.update_hovered_link_at_position(event.position, cx);
            return;
        }

        self.clear_hovered_link(cx);

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

    fn on_mouse_up(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.is_selecting = false;
        self.selection_drag_origin = None;
        self.update_hovered_link_at_position(event.position, cx);
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

    fn word_selection_range_for_offset(&self, offset: usize) -> Option<TextRange> {
        self.session
            .document()
            .word_range_at_offset(offset)
            .ok()
            .flatten()
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

    fn link_at_position(&self, position: gpui::Point<Pixels>) -> Option<LinkHitbox> {
        self.last_link_hitboxes
            .iter()
            .find(|link| bounds_contains_point(link.bounds, position))
            .cloned()
    }

    fn update_hovered_link_at_position(
        &mut self,
        position: gpui::Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let hovered_link_url = self.link_at_position(position).map(|link| link.url);
        if self.hovered_link_url != hovered_link_url {
            self.hovered_link_url = hovered_link_url;
            cx.notify();
        }
    }

    fn clear_hovered_link(&mut self, cx: &mut Context<Self>) {
        if self.hovered_link_url.take().is_some() {
            cx.notify();
        }
    }

    fn open_link(&mut self, link: LinkHitbox, cx: &mut Context<Self>) -> bool {
        let Some(command) = external_url_command(&link.url) else {
            self.status_message = Some("Only http and https links can be opened.".to_string());
            cx.notify();
            return false;
        };

        match command.status() {
            Ok(status) if status.success() => true,
            Ok(status) => {
                self.status_message = Some(format!("Open link failed: {status}"));
                cx.notify();
                false
            }
            Err(error) => {
                self.status_message = Some(format!("Open link failed: {error}"));
                cx.notify();
                false
            }
        }
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
        let range = encoding::utf16_range_to_byte(text, range);

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
        if let Some(offset) = marker_skip_offset(self.session.document().text(), &range, new_text) {
            self.move_caret(offset, cx);
            self.marked_range = None;
            return;
        }

        let (range, replacement, selection_after) =
            marker_autocomplete_edit(self.session.document().text(), &range, new_text)
                .unwrap_or_else(|| {
                    let caret = range.start + new_text.len();
                    (range, new_text.to_string(), caret..caret)
                });

        if self.replace_range(range, &replacement, selection_after, window, cx) {
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
            let selected_range = encoding::utf16_range_to_byte(new_text, selected_range);
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
        let editor_cursor = editor_cursor_style(self.hovered_link_url.is_some());

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
            .on_action(cx.listener(Self::insert_link))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::indent_list))
            .on_action(cx.listener(Self::outdent_list))
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
                    .cursor(editor_cursor)
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
                    .on_mouse_move(cx.listener(Self::on_mouse_move))
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
                    .child(EditorElement {
                        editor: cx.entity(),
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_cursor_uses_pointing_hand_when_hovering_link() {
        assert_eq!(editor_cursor_style(true), CursorStyle::PointingHand);
        assert_eq!(editor_cursor_style(false), CursorStyle::IBeam);
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
            KeyBinding::new("tab", IndentList, None),
            KeyBinding::new("shift-tab", OutdentList, None),
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
            KeyBinding::new("cmd-k", InsertLink, None),
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-x", Cut, None),
            KeyBinding::new("cmd-v", Paste, None),
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
                            last_link_hitboxes: Vec::new(),
                            hovered_link_url: None,
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
