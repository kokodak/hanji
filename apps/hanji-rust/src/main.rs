use std::{env, io, ops::Range, path::PathBuf, process};

use gpui::{
    App, Application, Bounds, Context, CursorStyle, Element, ElementId, ElementInputHandler,
    Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId, InspectorElementId,
    IntoElement, KeyBinding, LayoutId, MouseButton, MouseDownEvent, PaintQuad, Pixels, Render,
    ShapedLine, SharedString, Style, TextRun, UTF16Selection, Window, WindowBounds, WindowOptions,
    actions, div, fill, point, prelude::*, px, relative, rgb, rgba, size,
};
use hanji_core::{EditorCommand, Selection, TextPosition, TextRange, Transaction};
use hanji_storage::DocumentSession;

const LINE_HEIGHT: f32 = 24.0;
const FONT_SIZE: f32 = 16.0;
const SAMPLE_DOCUMENT: &str = "# Hanji\n\nCapture the thought.";

actions!(
    hanji,
    [
        Backspace, Delete, Left, Right, Up, Down, Home, End, Newline, Undo, Redo, Save, Quit
    ]
);

struct Hanji {
    focus_handle: FocusHandle,
    session: DocumentSession,
    marked_range: Option<Range<usize>>,
    last_lines: Vec<LineSnapshot>,
    preferred_column: Option<usize>,
    status_message: Option<String>,
}

impl Hanji {
    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
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
        let range = self.session.document().selection().primary();
        let offset = if range.is_empty() {
            previous_char_offset(self.session.document().text(), range.start)
        } else {
            Some(range.start)
        };

        if let Some(offset) = offset {
            self.move_caret(offset, cx);
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        let range = self.session.document().selection().primary();
        let offset = if range.is_empty() {
            next_char_offset(self.session.document().text(), range.end)
        } else {
            Some(range.end)
        };

        if let Some(offset) = offset {
            self.move_caret(offset, cx);
        }
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.move_caret_vertically(-1, cx);
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.move_caret_vertically(1, cx);
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
        let range = self.selected_range();
        let caret = range.start + "\n".len();

        self.replace_range(range, "\n", caret..caret, window, cx);
    }

    fn undo(&mut self, _: &Undo, window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;

        if self.session.undo().is_some() {
            self.document_changed(window, cx);
        }
    }

    fn redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;

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
        self.move_caret(self.index_for_mouse_position(event.position), cx);
    }

    fn move_caret(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.preferred_column = None;

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
        self.document_changed(window, cx);
        true
    }

    fn line_range_for_offset(&self, offset: usize) -> Option<TextRange> {
        let document = self.session.document();
        let line_index = document.line_index_at_offset(offset).ok()?;

        document.line_range(line_index)
    }

    fn index_for_mouse_position(&self, position: gpui::Point<Pixels>) -> usize {
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
        let local_x = position.x - line.bounds.left();
        let local_index = line.layout.closest_index_for_x(local_x);

        line.range.start + local_index.min(line.range.len())
    }

    fn byte_range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        byte_offset_to_utf16(self.session.document().text(), range.start)
            ..byte_offset_to_utf16(self.session.document().text(), range.end)
    }

    fn utf16_range_to_byte(&self, range: &Range<usize>) -> Range<usize> {
        utf16_offset_to_byte(self.session.document().text(), range.start)
            ..utf16_offset_to_byte(self.session.document().text(), range.end)
    }

    fn bounds_for_byte_range(&self, range: Range<usize>) -> Option<Bounds<Pixels>> {
        let line = self.line_for_offset(range.start)?;
        let start = range
            .start
            .saturating_sub(line.range.start)
            .min(line.range.len());
        let end = range
            .end
            .saturating_sub(line.range.start)
            .min(line.range.len());
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
            reversed: false,
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
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::newline))
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
                    .child(EditorElement {
                        editor: cx.entity(),
                    }),
            )
    }
}

#[derive(Clone)]
struct LineSnapshot {
    range: TextRange,
    layout: ShapedLine,
    bounds: Bounds<Pixels>,
}

struct EditorElement {
    editor: Entity<Hanji>,
}

struct EditorPrepaintState {
    lines: Vec<LineSnapshot>,
    cursor: Option<PaintQuad>,
    selections: Vec<PaintQuad>,
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
        let line_count = self.editor.read(cx).session.document().line_count().max(1);
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = (window.line_height() * line_count).into();

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
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let line_height = window.line_height();
        let mut lines = Vec::new();

        let document = editor.session.document();

        for line_index in 0..document.line_count() {
            let range = editor
                .session
                .document()
                .line_range(line_index)
                .unwrap_or_else(|| TextRange::caret(document.len()));
            let line_text: SharedString = document.text()[range.start..range.end].to_owned().into();
            let runs = if line_text.is_empty() {
                Vec::new()
            } else {
                vec![TextRun {
                    len: line_text.len(),
                    font: text_style.font(),
                    color: text_style.color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }]
            };
            let layout = window
                .text_system()
                .shape_line(line_text, font_size, &runs, None);
            let line_bounds = Bounds::new(
                point(bounds.left(), bounds.top() + line_height * line_index),
                size(bounds.size.width, line_height),
            );

            lines.push(LineSnapshot {
                range,
                layout,
                bounds: line_bounds,
            });
        }

        let selection = editor.session.document().selection().primary();
        let cursor = if selection.is_empty() {
            caret_quad(&lines, selection.start)
        } else {
            None
        };
        let selections = selection_quads(&lines, selection);

        EditorPrepaintState {
            lines,
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

        for selection in prepaint.selections.drain(..) {
            window.paint_quad(selection);
        }

        for line in &prepaint.lines {
            line.layout
                .paint(line.bounds.origin, window.line_height(), window, cx)
                .ok();
        }

        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        let lines = prepaint.lines.clone();
        self.editor.update(cx, |editor, _cx| {
            editor.last_lines = lines;
        });
    }
}

fn caret_quad(lines: &[LineSnapshot], offset: usize) -> Option<PaintQuad> {
    let line = line_for_offset(lines, offset)?;
    let local_index = offset
        .saturating_sub(line.range.start)
        .min(line.range.len());
    let x = line.layout.x_for_index(local_index);

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

            let start_x = line.layout.x_for_index(start - line.range.start);
            let end_x = line.layout.x_for_index(end - line.range.start);

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

fn previous_char_offset(text: &str, offset: usize) -> Option<usize> {
    if offset == 0 {
        return None;
    }

    text[..offset]
        .char_indices()
        .last()
        .map(|(offset, _)| offset)
}

fn next_char_offset(text: &str, offset: usize) -> Option<usize> {
    if offset >= text.len() {
        return None;
    }

    text[offset..]
        .char_indices()
        .nth(1)
        .map(|(next_offset, _)| offset + next_offset)
        .or(Some(text.len()))
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
            KeyBinding::new("up", Up, None),
            KeyBinding::new("down", Down, None),
            KeyBinding::new("home", Home, None),
            KeyBinding::new("end", End, None),
            KeyBinding::new("cmd-left", Home, None),
            KeyBinding::new("cmd-right", End, None),
            KeyBinding::new("enter", Newline, None),
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
                            preferred_column: None,
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
