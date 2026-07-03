mod editing;
mod encoding;
mod external;
mod file_browser;
mod renderer;
mod session;
mod snapshot;
mod time_label;

use std::{
    ops::Range,
    path::{Path, PathBuf},
    process,
    time::{Duration, Instant, SystemTime},
};

use gpui::{
    App, Application, Bounds, ClipboardItem, Context, CursorStyle, EntityInputHandler, FocusHandle,
    Focusable, IntoElement, KeyBinding, Menu, MenuItem, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PathPromptOptions, Pixels, PromptButton, PromptLevel, Render,
    ScrollHandle, SharedString, SystemMenuType, Timer, TitlebarOptions, UTF16Selection, Window,
    WindowBounds, WindowOptions, actions, div, point, prelude::*, px, rgb, size,
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
use file_browser::{MarkdownFile, folder_label, markdown_files_in};
use renderer::{EditorElement, FONT_SIZE, LINE_HEIGHT, LinkHitbox, TaskMarkerHitbox};
use session::{is_scratch_document_path, open_initial_session};
use snapshot::{LineSnapshot, line_for_offset};
use time_label::{document_modified_time, format_last_edited_time};

const CARET_SCROLL_MARGIN: f32 = 24.0;
const CARET_BLINK_IDLE_DELAY_MS: u64 = 900;
const CARET_BLINK_FRAME_MS: u64 = 16;
const CARET_BLINK_VISIBLE_HOLD_MS: u64 = 420;
const CARET_BLINK_FADE_MS: u64 = 80;
const CARET_BLINK_HIDDEN_HOLD_MS: u64 = 420;
const CARET_BLINK_CYCLE_MS: u64 =
    CARET_BLINK_VISIBLE_HOLD_MS + CARET_BLINK_FADE_MS * 2 + CARET_BLINK_HIDDEN_HOLD_MS;
const CARET_OPACITY_EPSILON: f32 = 0.01;

fn caret_blink_idle_delay_elapsed(idle_for: Duration) -> bool {
    idle_for >= Duration::from_millis(CARET_BLINK_IDLE_DELAY_MS)
}

fn caret_blink_opacity(idle_for: Duration) -> f32 {
    if !caret_blink_idle_delay_elapsed(idle_for) {
        return 1.0;
    }

    let active_for = idle_for - Duration::from_millis(CARET_BLINK_IDLE_DELAY_MS);
    let phase_ms = (active_for.as_millis() % u128::from(CARET_BLINK_CYCLE_MS)) as u64;
    let fade_out_start = CARET_BLINK_VISIBLE_HOLD_MS;
    let hidden_start = fade_out_start + CARET_BLINK_FADE_MS;
    let fade_in_start = hidden_start + CARET_BLINK_HIDDEN_HOLD_MS;

    if phase_ms < fade_out_start {
        return 1.0;
    }

    if phase_ms < hidden_start {
        let progress = (phase_ms - fade_out_start) as f32 / CARET_BLINK_FADE_MS as f32;
        return 1.0 - smoothstep(progress);
    }

    if phase_ms < fade_in_start {
        return 0.0;
    }

    let progress = (phase_ms - fade_in_start) as f32 / CARET_BLINK_FADE_MS as f32;

    smoothstep(progress)
}

fn smoothstep(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);

    progress * progress * (3.0 - 2.0 * progress)
}

fn caret_blink_next_delay(idle_for: Duration) -> Duration {
    if caret_blink_idle_delay_elapsed(idle_for) {
        Duration::from_millis(CARET_BLINK_FRAME_MS)
    } else {
        Duration::from_millis(CARET_BLINK_IDLE_DELAY_MS).saturating_sub(idle_for)
    }
}

fn editor_cursor_style(is_hovering_link: bool) -> CursorStyle {
    if is_hovering_link {
        CursorStyle::PointingHand
    } else {
        CursorStyle::IBeam
    }
}

fn document_path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn file_browser_root_for_document(path: &Path) -> Option<PathBuf> {
    if is_scratch_document_path(path) {
        return None;
    }

    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
}

fn window_title_for_session(session: &DocumentSession) -> String {
    format!("{} - Hanji", document_path_label(session.path()))
}

fn window_options_for_session(session: &DocumentSession, cx: &mut App) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(720.0), px(520.0)), cx);

    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some(window_title_for_session(session).into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(14.0), px(14.0))),
        }),
        ..Default::default()
    }
}

fn focus_front_window(cx: &mut App) -> bool {
    let handle = cx
        .window_stack()
        .and_then(|windows| windows.into_iter().next())
        .or_else(|| cx.windows().into_iter().next());

    let Some(handle) = handle else {
        return false;
    };

    handle
        .update(cx, |_, window, cx| {
            window.activate_window();
            cx.activate(true);
        })
        .is_ok()
}

fn open_editor_window(session: DocumentSession, cx: &mut App) -> Result<(), String> {
    let options = window_options_for_session(&session, cx);
    let window = cx
        .open_window(options, move |window, cx| {
            cx.new(|cx| {
                let mut editor = Hanji::new(session, cx);
                editor.update_window_title(window);
                editor.start_caret_blink(window, cx);
                editor
            })
        })
        .map_err(|error| format!("Could not open window: {error}"))?;

    window
        .update(cx, |view, window, cx| {
            window.focus(&view.focus_handle(cx));
            cx.activate(true);
        })
        .map_err(|error| format!("Could not focus window: {error}"))?;

    Ok(())
}

fn reopen_or_focus_editor(cx: &mut App) {
    if focus_front_window(cx) {
        return;
    }

    match open_initial_session() {
        Ok(session) => {
            if let Err(error) = open_editor_window(session, cx) {
                eprintln!("{error}");
            }
        }
        Err(error) => eprintln!("Could not open document: {error}"),
    }
}

fn configure_app_actions(cx: &mut App) {
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
        KeyBinding::new("cmd-shift-e", ToggleFileBrowser, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("cmd-o", OpenDocument, None),
        KeyBinding::new("cmd-s", Save, None),
        KeyBinding::new("cmd-q", Quit, None),
    ]);
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.set_menus(vec![Menu {
        name: "Hanji".into(),
        items: vec![
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Quit Hanji", Quit),
        ],
    }]);
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
        ToggleFileBrowser,
        OpenFolder,
        SelectAll,
        IndentList,
        OutdentList,
        Copy,
        Cut,
        Paste,
        Undo,
        Redo,
        OpenDocument,
        Save,
        Quit
    ]
);

const OPEN_WITHOUT_SAVING_PROMPT_INDEX: usize = 1;

struct Hanji {
    focus_handle: FocusHandle,
    session: DocumentSession,
    marked_range: Option<Range<usize>>,
    last_lines: Vec<LineSnapshot>,
    last_task_markers: Vec<TaskMarkerHitbox>,
    last_link_hitboxes: Vec<LinkHitbox>,
    editor_scroll: ScrollHandle,
    file_browser_scroll: ScrollHandle,
    file_browser_visible: bool,
    file_browser_root: Option<PathBuf>,
    file_browser_files: Vec<MarkdownFile>,
    file_browser_status: Option<String>,
    hovered_link_url: Option<String>,
    preferred_column: Option<usize>,
    preferred_visual_x: Option<Pixels>,
    selection_anchor: Option<usize>,
    selection_head: Option<usize>,
    is_selecting: bool,
    selection_drag_origin: Option<gpui::Point<Pixels>>,
    status_message: Option<String>,
    last_edited_at: SystemTime,
    caret_opacity: f32,
    caret_last_activity_at: Instant,
    caret_blink_started: bool,
}

impl Hanji {
    fn new(session: DocumentSession, cx: &mut Context<Self>) -> Self {
        let file_browser_root = file_browser_root_for_document(session.path());
        let mut editor = Self {
            focus_handle: cx.focus_handle(),
            last_edited_at: document_modified_time(session.path()),
            session,
            marked_range: None,
            last_lines: Vec::new(),
            last_task_markers: Vec::new(),
            last_link_hitboxes: Vec::new(),
            editor_scroll: ScrollHandle::new(),
            file_browser_scroll: ScrollHandle::new(),
            file_browser_visible: false,
            file_browser_root,
            file_browser_files: Vec::new(),
            file_browser_status: None,
            hovered_link_url: None,
            preferred_column: None,
            preferred_visual_x: None,
            selection_anchor: None,
            selection_head: None,
            is_selecting: false,
            selection_drag_origin: None,
            status_message: None,
            caret_opacity: 1.0,
            caret_last_activity_at: Instant::now(),
            caret_blink_started: false,
        };
        editor.refresh_file_browser_files();
        editor
    }

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

    fn toggle_file_browser(
        &mut self,
        _: &ToggleFileBrowser,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.file_browser_visible = !self.file_browser_visible;
        if self.file_browser_visible && self.file_browser_files.is_empty() {
            self.refresh_file_browser_files();
        }
        cx.notify();
    }

    fn open_folder(&mut self, _: &OpenFolder, window: &mut Window, cx: &mut Context<Self>) {
        self.reset_caret_blink();

        cx.spawn_in(window, async move |editor, cx| {
            let selected_paths = match cx.update(|_, app| {
                app.prompt_for_paths(PathPromptOptions {
                    files: false,
                    directories: true,
                    multiple: false,
                    prompt: Some("Open Folder".into()),
                })
            }) {
                Ok(selected_paths) => selected_paths,
                Err(error) => {
                    editor
                        .update_in(cx, |editor, _, cx| {
                            editor.status_message = Some(format!("Open folder failed: {error}"));
                            cx.notify();
                        })
                        .ok();
                    return;
                }
            };

            let selected_paths = match selected_paths.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    editor
                        .update_in(cx, |editor, _, cx| {
                            editor.status_message = Some(format!("Open folder failed: {error}"));
                            cx.notify();
                        })
                        .ok();
                    return;
                }
                Err(error) => {
                    editor
                        .update_in(cx, |editor, _, cx| {
                            editor.status_message = Some(format!("Open folder failed: {error}"));
                            cx.notify();
                        })
                        .ok();
                    return;
                }
            };

            let Some(path) = selected_paths.into_iter().next() else {
                return;
            };

            editor
                .update_in(cx, |editor, _, cx| {
                    editor.set_file_browser_root(path);
                    cx.notify();
                })
                .ok();
        })
        .detach();
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
            self.clear_preferred_vertical_target();
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
                self.clear_preferred_vertical_target();
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
        self.reset_caret_blink();

        match self.session.save() {
            Ok(()) => {
                self.status_message = Some("Saved.".to_string());
                self.last_edited_at = SystemTime::now();
                self.update_window_title(window);
            }
            Err(error) => {
                self.status_message = Some(format!("Save failed: {error}"));
            }
        }

        cx.notify();
    }

    fn open_document(&mut self, _: &OpenDocument, window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        self.clear_selection_tracking();
        self.reset_caret_blink();

        let discard_changes = if self.session.is_dirty() {
            Some(window.prompt(
                PromptLevel::Warning,
                "Open another file?",
                Some("Unsaved changes in the current document will be lost."),
                &[PromptButton::cancel("Cancel"), PromptButton::ok("Open")],
                cx,
            ))
        } else {
            None
        };

        cx.spawn_in(window, async move |editor, cx| {
            if let Some(discard_changes) = discard_changes {
                let Ok(answer) = discard_changes.await else {
                    return;
                };

                if answer != OPEN_WITHOUT_SAVING_PROMPT_INDEX {
                    return;
                }
            }

            let selected_paths = match cx.update(|_, app| {
                app.prompt_for_paths(PathPromptOptions {
                    files: true,
                    directories: false,
                    multiple: false,
                    prompt: Some("Open".into()),
                })
            }) {
                Ok(selected_paths) => selected_paths,
                Err(error) => {
                    editor
                        .update_in(cx, |editor, _, cx| {
                            editor.status_message = Some(format!("Open failed: {error}"));
                            cx.notify();
                        })
                        .ok();
                    return;
                }
            };

            let selected_paths = match selected_paths.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    editor
                        .update_in(cx, |editor, _, cx| {
                            editor.status_message = Some(format!("Open failed: {error}"));
                            cx.notify();
                        })
                        .ok();
                    return;
                }
                Err(error) => {
                    editor
                        .update_in(cx, |editor, _, cx| {
                            editor.status_message = Some(format!("Open failed: {error}"));
                            cx.notify();
                        })
                        .ok();
                    return;
                }
            };

            let Some(path) = selected_paths.into_iter().next() else {
                return;
            };

            let opened_session = DocumentSession::open(path);
            editor
                .update_in(cx, |editor, window, cx| match opened_session {
                    Ok(session) => editor.replace_session(session, window, cx),
                    Err(error) => {
                        editor.status_message = Some(format!("Open failed: {error}"));
                        cx.notify();
                    }
                })
                .ok();
        })
        .detach();
    }

    fn open_file_from_browser(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked_range = None;
        self.clear_selection_tracking();
        self.reset_caret_blink();

        let discard_changes = if self.session.is_dirty() {
            Some(window.prompt(
                PromptLevel::Warning,
                "Open another file?",
                Some("Unsaved changes in the current document will be lost."),
                &[PromptButton::cancel("Cancel"), PromptButton::ok("Open")],
                cx,
            ))
        } else {
            None
        };

        cx.spawn_in(window, async move |editor, cx| {
            if let Some(discard_changes) = discard_changes {
                let Ok(answer) = discard_changes.await else {
                    return;
                };

                if answer != OPEN_WITHOUT_SAVING_PROMPT_INDEX {
                    return;
                }
            }

            let opened_session = DocumentSession::open(path);
            editor
                .update_in(cx, |editor, window, cx| match opened_session {
                    Ok(session) => editor.replace_session(session, window, cx),
                    Err(error) => {
                        editor.status_message = Some(format!("Open failed: {error}"));
                        cx.notify();
                    }
                })
                .ok();
        })
        .detach();
    }

    fn set_file_browser_root(&mut self, root: PathBuf) {
        self.file_browser_visible = true;
        self.file_browser_root = Some(root);
        self.file_browser_scroll.set_offset(point(px(0.0), px(0.0)));
        self.refresh_file_browser_files();
    }

    fn update_file_browser_root_for_session(&mut self) {
        let path = self.session.path();
        let preserves_root = self
            .file_browser_root
            .as_ref()
            .is_some_and(|root| path.starts_with(root));

        if !preserves_root {
            self.file_browser_root = file_browser_root_for_document(path);
            self.file_browser_scroll.set_offset(point(px(0.0), px(0.0)));
        }

        self.refresh_file_browser_files();
    }

    fn refresh_file_browser_files(&mut self) {
        let Some(root) = self.file_browser_root.as_deref() else {
            self.file_browser_files.clear();
            self.file_browser_status = Some("Open a folder to browse Markdown files.".to_string());
            return;
        };

        match markdown_files_in(root) {
            Ok(files) if files.is_empty() => {
                self.file_browser_files = files;
                self.file_browser_status = Some("No Markdown files found.".to_string());
            }
            Ok(files) => {
                self.file_browser_files = files;
                self.file_browser_status = None;
            }
            Err(error) => {
                self.file_browser_files.clear();
                self.file_browser_status = Some(format!("Could not read folder: {error}"));
                self.status_message = Some(format!("Could not read folder: {error}"));
            }
        }
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
        self.clear_preferred_vertical_target();
        self.clear_selection_tracking();
        let offset = self
            .session
            .document()
            .nearest_grapheme_offset(offset)
            .unwrap_or(offset);

        if self.session.set_selection(Selection::caret(offset)).is_ok() {
            self.reset_caret_blink();
            self.scroll_selection_into_view();
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
            self.preferred_visual_x = None;
            self.reset_caret_blink();
            self.scroll_selection_into_view();
            cx.notify();
        }
    }

    fn move_caret_preserving_visual_x(
        &mut self,
        offset: usize,
        visual_x: Pixels,
        cx: &mut Context<Self>,
    ) {
        self.clear_selection_tracking();
        let offset = self
            .session
            .document()
            .nearest_grapheme_offset(offset)
            .unwrap_or(offset);

        if self.session.set_selection(Selection::caret(offset)).is_ok() {
            self.preferred_column = None;
            self.preferred_visual_x = Some(visual_x);
            self.reset_caret_blink();
            self.scroll_selection_into_view();
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

        if let Some((target_offset, visual_x)) =
            self.visual_vertical_target_for_offset(offset, direction)
        {
            self.move_caret_preserving_visual_x(target_offset, visual_x, cx);
            return;
        }

        let Some((target_offset, column)) =
            self.logical_vertical_target_for_offset(offset, direction)
        else {
            return;
        };

        self.move_caret_preserving_column(target_offset, column, cx);
    }

    fn logical_vertical_target_for_offset(
        &self,
        offset: usize,
        direction: isize,
    ) -> Option<(usize, usize)> {
        let document = self.session.document();
        let Ok(position) = document.position_at_offset(offset) else {
            return None;
        };
        let Some(target_line) = position.line.checked_add_signed(direction) else {
            return None;
        };

        if target_line >= document.line_count() {
            return None;
        }

        let column = self.preferred_column.unwrap_or(position.column);
        document
            .offset_at_position(TextPosition::new(target_line, column))
            .or_else(|_| {
                document
                    .line_range(target_line)
                    .map(|range| range.end)
                    .ok_or(hanji_core::EditError::InvalidRange)
            })
            .ok()
            .map(|target_offset| (target_offset, column))
    }

    fn visual_vertical_target_for_offset(
        &self,
        offset: usize,
        direction: isize,
    ) -> Option<(usize, Pixels)> {
        let (line_index, line) = self.line_index_for_offset(offset)?;
        let visible_offset = line.source_to_visible_offset(offset);
        let position = line.wrapped_caret_position(visible_offset)?;
        let row = line.wrapped_row_for_visible_offset(visible_offset)?;
        let current_row = self.flat_visual_row_index(line_index, row);
        let target_row = current_row.checked_add_signed(direction)?;
        let (target_line_index, target_line_row) =
            self.line_index_and_row_for_flat_visual_row(target_row)?;
        let target_line = self.last_lines.get(target_line_index)?;
        let visual_x = self.preferred_visual_x.unwrap_or(position.x);
        let target_visible_offset =
            target_line.visible_offset_for_wrapped_row_x(target_line_row, visual_x);
        let target_offset = target_line.visible_to_source_caret_offset(target_visible_offset);

        Some((target_offset, visual_x))
    }

    fn flat_visual_row_index(&self, line_index: usize, row: usize) -> usize {
        self.last_lines
            .iter()
            .take(line_index)
            .map(LineSnapshot::wrapped_row_count)
            .sum::<usize>()
            + row
    }

    fn line_index_and_row_for_flat_visual_row(&self, mut row: usize) -> Option<(usize, usize)> {
        for (line_index, line) in self.last_lines.iter().enumerate() {
            let row_count = line.wrapped_row_count();
            if row < row_count {
                return Some((line_index, row));
            }
            row -= row_count;
        }

        None
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
        let (anchor, head) = self.selection_extension_points(direction);

        if let Some((target_offset, visual_x)) =
            self.visual_vertical_target_for_offset(head, direction)
        {
            self.select_from_anchor_to_preserving_visual_x(anchor, target_offset, visual_x, cx);
            return;
        }

        let Some((target_offset, column)) =
            self.logical_vertical_target_for_offset(head, direction)
        else {
            return;
        };

        self.select_from_anchor_to(anchor, target_offset, Some(column), cx);
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
            self.preferred_visual_x = None;
            self.reset_caret_blink();
            self.scroll_selection_into_view();
            cx.notify();
        }
    }

    fn select_from_anchor_to_preserving_visual_x(
        &mut self,
        anchor: usize,
        head: usize,
        visual_x: Pixels,
        cx: &mut Context<Self>,
    ) {
        let document = self.session.document();
        let anchor = document.nearest_grapheme_offset(anchor).unwrap_or(anchor);
        let head = document.nearest_grapheme_offset(head).unwrap_or(head);
        let range = selection_range_from_anchor_and_head(anchor, head);

        if self.session.set_selection(Selection::single(range)).is_ok() {
            self.selection_anchor = Some(anchor);
            self.selection_head = Some(head);
            self.preferred_column = None;
            self.preferred_visual_x = Some(visual_x);
            self.reset_caret_blink();
            self.scroll_selection_into_view();
            cx.notify();
        }
    }

    fn clear_preferred_vertical_target(&mut self) {
        self.preferred_column = None;
        self.preferred_visual_x = None;
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

        self.clear_preferred_vertical_target();
        self.clear_selection_tracking();
        self.document_changed(window, cx);
        true
    }

    fn replace_session(
        &mut self,
        session: DocumentSession,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let opened_name = document_path_label(session.path());

        self.session = session;
        self.marked_range = None;
        self.last_lines.clear();
        self.last_task_markers.clear();
        self.last_link_hitboxes.clear();
        self.last_edited_at = document_modified_time(self.session.path());
        self.editor_scroll.set_offset(point(px(0.0), px(0.0)));
        self.hovered_link_url = None;
        self.update_file_browser_root_for_session();
        self.clear_preferred_vertical_target();
        self.clear_selection_tracking();
        self.reset_caret_blink();
        self.status_message = Some(format!("Opened {opened_name}."));

        window.focus(&self.focus_handle);
        self.update_window_title(window);
        cx.notify();
    }

    fn scroll_selection_into_view(&self) {
        let selection = self.session.document().selection().primary();
        let target_offset = self.selection_head.unwrap_or(selection.end);
        let Some(line) = line_for_offset(&self.last_lines, target_offset) else {
            return;
        };

        let viewport = self.editor_scroll.bounds();
        if viewport.size.height <= px(0.0) {
            return;
        }

        let margin = px(CARET_SCROLL_MARGIN);
        let target_top = viewport.top() + margin;
        let target_bottom = viewport.bottom() - margin;
        let target_left = viewport.left() + margin;
        let target_right = viewport.right() - margin;
        let mut offset = self.editor_scroll.offset();
        let caret_position = line
            .wrapped_caret_position(line.source_to_visible_offset(target_offset))
            .unwrap_or_else(|| point(px(0.0), px(0.0)));
        let caret_x = line.bounds.left() + caret_position.x;
        let caret_top = line.bounds.top() + caret_position.y;
        let caret_bottom = caret_top + line.line_height;

        if caret_top < target_top {
            offset.y += target_top - caret_top;
        } else if caret_bottom > target_bottom {
            offset.y -= caret_bottom - target_bottom;
        }

        if caret_x < target_left {
            offset.x += target_left - caret_x;
        } else if caret_x > target_right {
            offset.x -= caret_x - target_right;
        }

        offset.x = offset
            .x
            .clamp(-self.editor_scroll.max_offset().width, px(0.0));
        offset.y = offset
            .y
            .clamp(-self.editor_scroll.max_offset().height, px(0.0));
        self.editor_scroll.set_offset(offset);
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

        let local_position = point(
            position.x - line.bounds.left(),
            position.y - line.bounds.top(),
        );
        let visible_offset = line.visible_offset_for_local_position(local_position);
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
        let start_position = line
            .wrapped_caret_position(start)
            .unwrap_or_else(|| point(px(0.0), px(0.0)));
        let end_position = line.wrapped_caret_position(end).unwrap_or(start_position);
        let left = line.bounds.left() + start_position.x;
        let right = (line.bounds.left() + end_position.x).max(left + px(2.0));
        let top = line.bounds.top() + start_position.y;
        let bottom =
            (line.bounds.top() + end_position.y + line.line_height).max(top + line.line_height);

        Some(Bounds::from_corners(point(left, top), point(right, bottom)))
    }

    fn line_for_offset(&self, offset: usize) -> Option<&LineSnapshot> {
        self.last_lines
            .iter()
            .find(|line| offset >= line.range.start && offset <= line.range.end)
            .or_else(|| self.last_lines.last())
    }

    fn line_index_for_offset(&self, offset: usize) -> Option<(usize, &LineSnapshot)> {
        self.last_lines
            .iter()
            .enumerate()
            .find(|(_, line)| offset >= line.range.start && offset <= line.range.end)
            .or_else(|| {
                self.last_lines
                    .last()
                    .map(|line| (self.last_lines.len() - 1, line))
            })
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
        self.last_edited_at = SystemTime::now();
        self.reset_caret_blink();
        self.update_window_title(window);
        self.scroll_selection_into_view();
        cx.notify();
    }

    fn update_window_title(&self, window: &mut Window) {
        window.set_window_title(&self.window_title());
    }

    fn window_title(&self) -> String {
        format!("{} - Hanji", self.document_label())
    }

    fn document_label(&self) -> String {
        let name = document_path_label(self.session.path());
        let dirty = if self.session.is_dirty() { " *" } else { "" };

        format!("{name}{dirty}")
    }

    fn reset_caret_blink(&mut self) {
        self.caret_opacity = 1.0;
        self.caret_last_activity_at = Instant::now();
    }

    fn start_caret_blink(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.caret_blink_started {
            return;
        }

        self.caret_blink_started = true;
        cx.spawn_in(window, async move |editor, cx| {
            loop {
                let delay = match editor.update_in(cx, |editor, _, cx| {
                    let idle_for = editor.caret_last_activity_at.elapsed();
                    let opacity = caret_blink_opacity(idle_for);

                    if (editor.caret_opacity - opacity).abs() > CARET_OPACITY_EPSILON {
                        editor.caret_opacity = opacity;
                        cx.notify();
                    }

                    caret_blink_next_delay(idle_for)
                }) {
                    Ok(delay) => delay,
                    Err(_) => break,
                };

                Timer::after(delay).await;
            }
        })
        .detach();
    }

    fn render_file_browser(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let root_label = self
            .file_browser_root
            .as_deref()
            .map(folder_label)
            .unwrap_or_else(|| "No folder".to_string());
        let mut files = div()
            .id("file-browser-files")
            .flex()
            .flex_1()
            .flex_col()
            .gap_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .scrollbar_width(px(6.0))
            .track_scroll(&self.file_browser_scroll);

        if let Some(status) = self.file_browser_status.clone() {
            files = files.child(
                div()
                    .px_2()
                    .py_1()
                    .text_xs()
                    .line_height(px(16.0))
                    .text_color(rgb(0x9ca3af))
                    .child(status),
            );
        } else {
            let current_path = self.session.path().to_path_buf();
            for file in self.file_browser_files.clone() {
                let is_active = file.path == current_path;
                files = files.child(self.render_file_browser_file(file, is_active, cx));
            }
        }

        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .w(px(248.0))
            .h_full()
            .min_h(px(0.0))
            .border_r_1()
            .border_color(rgb(0xe5e7eb))
            .bg(rgb(0xffffff))
            .px_3()
            .py_3()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .line_height(px(14.0))
                            .text_color(rgb(0x9ca3af))
                            .child("Folder"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .line_height(px(18.0))
                            .text_color(rgb(0x111111))
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(root_label),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.render_file_browser_button("Open File", OpenDocument, cx))
                    .child(self.render_file_browser_button("Open Folder", OpenFolder, cx)),
            )
            .child(files)
    }

    fn render_file_browser_button(
        &mut self,
        label: &'static str,
        action: impl gpui::Action + 'static,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id(label)
            .px_2()
            .py_1()
            .rounded(px(5.0))
            .border_1()
            .border_color(rgb(0xd1d5db))
            .text_xs()
            .line_height(px(16.0))
            .text_color(rgb(0x111111))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(0xf3f4f6)))
            .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
            .child(label)
    }

    fn render_file_browser_file(
        &mut self,
        file: MarkdownFile,
        is_active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let label = file.label.clone();
        let id = SharedString::from(format!("file-browser-file:{label}"));
        let path = file.path.clone();

        div()
            .id(id)
            .px_2()
            .py_1()
            .rounded(px(5.0))
            .line_height(px(18.0))
            .text_sm()
            .text_color(rgb(0x374151))
            .cursor_pointer()
            .whitespace_nowrap()
            .overflow_hidden()
            .text_ellipsis()
            .hover(|style| style.bg(rgb(0xf3f4f6)))
            .when(is_active, |row| {
                row.bg(rgb(0xecfdf5)).text_color(rgb(0x111111))
            })
            .on_click(cx.listener(move |editor, _, window, cx| {
                editor.open_file_from_browser(path.clone(), window, cx);
            }))
            .child(label)
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
        let mut status_detail = self.document_label();
        if let Some(message) = self
            .status_message
            .as_deref()
            .filter(|message| !message.is_empty())
        {
            status_detail.push_str(" · ");
            status_detail.push_str(message);
        }
        let status_detail: SharedString = status_detail.into();
        let last_edited_label: SharedString = format_last_edited_time(self.last_edited_at).into();
        let editor_cursor = editor_cursor_style(self.hovered_link_url.is_some());
        let mut body = div().flex().flex_1().min_h(px(0.0)).w_full();

        if self.file_browser_visible {
            body = body.child(self.render_file_browser(cx));
        }

        body = body.child(
            div()
                .id("editor-scroll")
                .flex_1()
                .w_full()
                .min_h(px(0.0))
                .p_4()
                .overflow_y_scroll()
                .overflow_x_hidden()
                .scrollbar_width(px(8.0))
                .track_scroll(&self.editor_scroll)
                .line_height(px(LINE_HEIGHT))
                .text_size(px(FONT_SIZE))
                .font_family("Menlo")
                .cursor(editor_cursor)
                .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
                .on_mouse_move(cx.listener(Self::on_mouse_move))
                .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
                .child(
                    div()
                        .id("document-last-edited")
                        .w_full()
                        .h(px(22.0))
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(rgb(0x9ca3af))
                        .child(last_edited_label),
                )
                .child(EditorElement {
                    editor: cx.entity(),
                }),
        );

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
            .on_action(cx.listener(Self::toggle_file_browser))
            .on_action(cx.listener(Self::open_folder))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::indent_list))
            .on_action(cx.listener(Self::outdent_list))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::open_document))
            .on_action(cx.listener(Self::save))
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xffffff))
            .text_color(rgb(0x111111))
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .h(px(38.0))
                    .w_full()
                    .items_center()
                    .justify_between()
                    .bg(rgb(0xffffff))
                    .pl(px(76.0))
                    .pr(px(18.0))
                    .child(div().text_sm().text_color(rgb(0x111111)).child("Hanji"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x6b7280))
                            .child(status_detail),
                    ),
            )
            .child(body)
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

    #[test]
    fn caret_blink_waits_until_idle_delay() {
        assert!(!caret_blink_idle_delay_elapsed(Duration::from_millis(
            CARET_BLINK_IDLE_DELAY_MS - 1
        )));
        assert!(caret_blink_idle_delay_elapsed(Duration::from_millis(
            CARET_BLINK_IDLE_DELAY_MS
        )));
    }

    #[test]
    fn caret_blink_opacity_uses_short_eased_transitions_after_idle_delay() {
        let idle_delay = Duration::from_millis(CARET_BLINK_IDLE_DELAY_MS);
        let visible_hold = Duration::from_millis(CARET_BLINK_VISIBLE_HOLD_MS);
        let half_fade = Duration::from_millis(CARET_BLINK_FADE_MS / 2);
        let fade = Duration::from_millis(CARET_BLINK_FADE_MS);
        let hidden_hold = Duration::from_millis(CARET_BLINK_HIDDEN_HOLD_MS);
        let full_cycle = Duration::from_millis(CARET_BLINK_CYCLE_MS);

        assert_near(
            caret_blink_opacity(idle_delay - Duration::from_millis(1)),
            1.0,
        );
        assert_near(caret_blink_opacity(idle_delay), 1.0);
        assert_near(
            caret_blink_opacity(idle_delay + visible_hold - Duration::from_millis(1)),
            1.0,
        );
        assert_near(
            caret_blink_opacity(idle_delay + visible_hold + half_fade),
            0.5,
        );
        assert!(caret_blink_opacity(idle_delay + visible_hold + fade) < 0.01);
        assert!(caret_blink_opacity(idle_delay + visible_hold + fade + hidden_hold) < 0.01);
        assert_near(
            caret_blink_opacity(idle_delay + visible_hold + fade + hidden_hold + half_fade),
            0.5,
        );
        assert_near(caret_blink_opacity(idle_delay + full_cycle), 1.0);
    }

    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.01,
            "expected {actual} to be near {expected}"
        );
    }

    #[test]
    fn document_path_label_prefers_file_name() {
        assert_eq!(
            document_path_label(Path::new("/tmp/notes/hanji.md")),
            "hanji.md"
        );
        assert_eq!(document_path_label(Path::new("/")), "/");
    }
}

fn main() {
    let session = open_initial_session().unwrap_or_else(|error| {
        eprintln!("Could not open document: {error}");
        process::exit(1);
    });

    let app = Application::new();
    app.on_reopen(reopen_or_focus_editor);
    app.run(move |cx: &mut App| {
        configure_app_actions(cx);

        if let Err(error) = open_editor_window(session, cx) {
            eprintln!("{error}");
            cx.quit();
        }
    });
}
