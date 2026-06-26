use gpui::{
    App, BorderStyle, Bounds, Element, ElementId, ElementInputHandler, Entity, FontWeight,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, PaintQuad, Pixels, ShapedLine,
    SharedString, StrikethroughStyle, Style, TextRun, TextStyle, UnderlineStyle, Window, fill,
    point, px, quad, relative, rgb, rgba, size,
};
use hanji_core::TextRange;
use hanji_markdown::{
    MarkdownLine, MarkdownListMarker, MarkdownTaskState, ProjectedSegmentKind,
    ProjectedVisibleSegment, project_document,
};

use crate::Hanji;
use crate::editing::ordered_list_delimiter;
use crate::snapshot::{LineSnapshot, line_for_offset};

pub(crate) const LINE_HEIGHT: f32 = 24.0;
pub(crate) const FONT_SIZE: f32 = 16.0;

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
const CODE_BLOCK_BACKGROUND_COLOR: u32 = 0x25231f14;
const CODE_BLOCK_CORNER_RADIUS: f32 = 5.0;
const CODE_BLOCK_TEXT_INSET: f32 = 12.0;
const HORIZONTAL_RULE_COLOR: u32 = 0xd8d3c7;
const HORIZONTAL_RULE_INSET: f32 = 4.0;
const HORIZONTAL_RULE_HEIGHT: f32 = 1.0;

pub(crate) struct EditorElement {
    pub(crate) editor: Entity<Hanji>,
}

pub(crate) struct EditorPrepaintState {
    lines: Vec<LineSnapshot>,
    blockquote_bars: Vec<PaintQuad>,
    list_markers: Vec<ListMarkerSnapshot>,
    task_marker_hitboxes: Vec<TaskMarkerHitbox>,
    link_hitboxes: Vec<LinkHitbox>,
    code_backgrounds: Vec<PaintQuad>,
    horizontal_rules: Vec<PaintQuad>,
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
pub(crate) struct TaskMarkerHitbox {
    pub(crate) bounds: Bounds<Pixels>,
    pub(crate) marker_range: TextRange,
    pub(crate) state: MarkdownTaskState,
}

#[derive(Clone)]
pub(crate) struct LinkHitbox {
    pub(crate) bounds: Bounds<Pixels>,
    pub(crate) url: String,
}

#[derive(Clone, Copy)]
pub(crate) struct LinePresentation {
    pub(crate) font_size: f32,
    pub(crate) line_height: f32,
    pub(crate) is_heading: bool,
    pub(crate) is_blockquote: bool,
    pub(crate) is_code_block: bool,
    pub(crate) is_checked_task: bool,
    pub(crate) text_indent: f32,
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
        let mut link_hitboxes = Vec::new();
        let mut code_backgrounds = Vec::new();
        let mut horizontal_rules = Vec::new();
        let mut code_block_background_runs = Vec::new();
        let mut code_block_background_run = None;
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
            record_code_block_background_run(
                &mut code_block_background_runs,
                &mut code_block_background_run,
                container_bounds,
                presentation.is_code_block,
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
            if matches!(line.kind, MarkdownLine::HorizontalRule)
                && !horizontal_rule_source_is_revealed(&visible_segments)
            {
                horizontal_rules.push(horizontal_rule_quad(container_bounds));
            }
            code_backgrounds.extend(code_background_quads(
                &visible_segments,
                &layout,
                line_bounds,
            ));
            link_hitboxes.extend(link_hitboxes_for_segments(
                &visible_segments,
                &layout,
                line_bounds,
                line.source,
                line.range.start,
            ));
            top += presentation.line_height;
            let visible_len = visible_segments
                .last()
                .map_or(0, |segment| segment.visible_range.end);

            lines.push(LineSnapshot::new(
                line.range,
                line.marker_range,
                visible_len,
                visible_segments,
                layout,
                line_bounds,
            ));
        }
        flush_blockquote_bar_run(&mut blockquote_bar_runs, &mut blockquote_bar_run);
        flush_code_block_background_run(
            &mut code_block_background_runs,
            &mut code_block_background_run,
        );
        let blockquote_bars = blockquote_bar_runs
            .into_iter()
            .map(blockquote_bar_quad)
            .collect();
        code_backgrounds.extend(
            code_block_background_runs
                .into_iter()
                .map(code_block_background_quad),
        );

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
            link_hitboxes,
            code_backgrounds,
            horizontal_rules,
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

        for rule in prepaint.horizontal_rules.drain(..) {
            window.paint_quad(rule);
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
        let link_hitboxes = prepaint.link_hitboxes.clone();
        self.editor.update(cx, |editor, _cx| {
            editor.last_lines = lines;
            editor.last_task_markers = task_markers;
            editor.last_link_hitboxes = link_hitboxes;
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

fn horizontal_rule_source_is_revealed(segments: &[ProjectedVisibleSegment<'_>]) -> bool {
    segments
        .iter()
        .any(|segment| matches!(segment.kind, ProjectedSegmentKind::HorizontalRuleMarker))
}

fn horizontal_rule_quad(bounds: Bounds<Pixels>) -> PaintQuad {
    let line_height = bounds.bottom() - bounds.top();
    let rule_height = px(HORIZONTAL_RULE_HEIGHT);
    let rule_top = bounds.top() + (line_height - rule_height) / 2.0;

    fill(
        Bounds::new(
            point(bounds.left() + px(HORIZONTAL_RULE_INSET), rule_top),
            size(
                (bounds.right() - bounds.left() - px(HORIZONTAL_RULE_INSET * 2.0)).max(px(0.0)),
                rule_height,
            ),
        ),
        rgb(HORIZONTAL_RULE_COLOR),
    )
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
            ProjectedSegmentKind::StrikethroughContent => InlineRunStyle::Strikethrough,
            ProjectedSegmentKind::CodeContent => InlineRunStyle::Code,
            ProjectedSegmentKind::CodeBlockContent => InlineRunStyle::CodeBlock,
            ProjectedSegmentKind::LinkText => InlineRunStyle::Link,
            ProjectedSegmentKind::LinkDestination => InlineRunStyle::Plain,
            ProjectedSegmentKind::EscapeMarker => InlineRunStyle::EscapeMarker,
            ProjectedSegmentKind::HeadingMarker
            | ProjectedSegmentKind::HorizontalRuleMarker
            | ProjectedSegmentKind::BlockquoteMarker
            | ProjectedSegmentKind::ListMarker
            | ProjectedSegmentKind::StrongMarker
            | ProjectedSegmentKind::EmphasisMarker
            | ProjectedSegmentKind::StrikethroughMarker
            | ProjectedSegmentKind::CodeMarker
            | ProjectedSegmentKind::CodeBlockFence
            | ProjectedSegmentKind::LinkMarker => InlineRunStyle::Marker,
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
    Strikethrough,
    Code,
    CodeBlock,
    Link,
    EscapeMarker,
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
    let color = if matches!(style, InlineRunStyle::EscapeMarker) {
        rgb(0x8f8a82).into()
    } else if matches!(style, InlineRunStyle::Marker) {
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

    let underline = match style {
        InlineRunStyle::Strong | InlineRunStyle::Emphasis => Some(font_run_boundary_marker()),
        InlineRunStyle::Link => Some(link_underline()),
        InlineRunStyle::Plain
        | InlineRunStyle::Marker
        | InlineRunStyle::Strikethrough
        | InlineRunStyle::Code
        | InlineRunStyle::CodeBlock
        | InlineRunStyle::EscapeMarker => None,
    };
    let strikethrough = if matches!(style, InlineRunStyle::Strikethrough) {
        Some(strikethrough_style())
    } else {
        None
    };

    runs.push(TextRun {
        len,
        font,
        color,
        background_color: None,
        underline,
        strikethrough,
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

fn link_underline() -> UnderlineStyle {
    UnderlineStyle {
        thickness: px(1.0),
        color: Some(rgb(0x25231f).into()),
        wavy: false,
    }
}

fn strikethrough_style() -> StrikethroughStyle {
    StrikethroughStyle {
        thickness: px(1.0),
        color: Some(rgb(0x25231f).into()),
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
            | ProjectedSegmentKind::HorizontalRuleMarker
            | ProjectedSegmentKind::BlockquoteMarker
            | ProjectedSegmentKind::ListMarker
            | ProjectedSegmentKind::EscapeMarker
            | ProjectedSegmentKind::StrongMarker
            | ProjectedSegmentKind::StrongContent
            | ProjectedSegmentKind::EmphasisMarker
            | ProjectedSegmentKind::EmphasisContent
            | ProjectedSegmentKind::StrikethroughMarker
            | ProjectedSegmentKind::StrikethroughContent
            | ProjectedSegmentKind::CodeBlockFence
            | ProjectedSegmentKind::CodeBlockContent
            | ProjectedSegmentKind::LinkMarker
            | ProjectedSegmentKind::LinkText
            | ProjectedSegmentKind::LinkDestination => None,
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

fn record_code_block_background_run(
    runs: &mut Vec<Bounds<Pixels>>,
    current_run: &mut Option<Bounds<Pixels>>,
    bounds: Bounds<Pixels>,
    is_code_block: bool,
) {
    if is_code_block {
        let bounds = match current_run.take() {
            Some(run) => merge_code_block_background_bounds(run, bounds),
            None => bounds,
        };
        *current_run = Some(bounds);
    } else {
        flush_code_block_background_run(runs, current_run);
    }
}

fn flush_code_block_background_run(
    runs: &mut Vec<Bounds<Pixels>>,
    current_run: &mut Option<Bounds<Pixels>>,
) {
    if let Some(run) = current_run.take() {
        runs.push(run);
    }
}

fn merge_code_block_background_bounds(
    start: Bounds<Pixels>,
    end: Bounds<Pixels>,
) -> Bounds<Pixels> {
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

#[cfg(test)]
fn collect_code_block_background_bounds(lines: &[(Bounds<Pixels>, bool)]) -> Vec<Bounds<Pixels>> {
    let mut runs = Vec::new();
    let mut current_run = None;

    for (bounds, is_code_block) in lines {
        record_code_block_background_run(&mut runs, &mut current_run, *bounds, *is_code_block);
    }

    flush_code_block_background_run(&mut runs, &mut current_run);
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

fn code_block_background_quad(bounds: Bounds<Pixels>) -> PaintQuad {
    quad(
        bounds,
        px(CODE_BLOCK_CORNER_RADIUS),
        rgba(CODE_BLOCK_BACKGROUND_COLOR),
        px(0.0),
        rgba(0x00000000),
        BorderStyle::Solid,
    )
}

fn link_hitboxes_for_segments(
    segments: &[ProjectedVisibleSegment<'_>],
    layout: &ShapedLine,
    bounds: Bounds<Pixels>,
    line_source: &str,
    line_start: usize,
) -> Vec<LinkHitbox> {
    segments
        .iter()
        .filter_map(|segment| {
            if !matches!(segment.kind, ProjectedSegmentKind::LinkText) {
                return None;
            }

            let url =
                link_destination_from_source(line_source, line_start, segment.source_outer_range)?;
            let start_x = layout.x_for_index(segment.visible_range.start);
            let end_x = layout
                .x_for_index(segment.visible_range.end)
                .max(start_x + px(2.0));

            Some(LinkHitbox {
                bounds: Bounds::from_corners(
                    point(bounds.left() + start_x, bounds.top()),
                    point(bounds.left() + end_x, bounds.bottom()),
                ),
                url: url.to_string(),
            })
        })
        .collect()
}

fn link_destination_from_source<'a>(
    line_source: &'a str,
    line_start: usize,
    source_outer_range: TextRange,
) -> Option<&'a str> {
    let start = source_outer_range.start.checked_sub(line_start)?;
    let end = source_outer_range.end.checked_sub(line_start)?;
    let source = line_source.get(start..end)?;
    let separator = source.find("](")?;

    source
        .strip_prefix('[')?
        .strip_suffix(')')
        .and_then(|_| source.get(separator + 2..source.len() - 1))
        .filter(|destination| !destination.is_empty())
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
                is_code_block: false,
                is_checked_task: false,
                text_indent: 0.0,
            }
        }
        MarkdownLine::Blockquote => LinePresentation {
            font_size: FONT_SIZE,
            line_height: LINE_HEIGHT,
            is_heading: false,
            is_blockquote: true,
            is_code_block: false,
            is_checked_task: false,
            text_indent: 18.0,
        },
        MarkdownLine::HorizontalRule => LinePresentation {
            font_size: FONT_SIZE,
            line_height: LINE_HEIGHT,
            is_heading: false,
            is_blockquote: false,
            is_code_block: false,
            is_checked_task: false,
            text_indent: 0.0,
        },
        MarkdownLine::ListItem { task, .. } => LinePresentation {
            font_size: FONT_SIZE,
            line_height: LINE_HEIGHT,
            is_heading: false,
            is_blockquote: false,
            is_code_block: false,
            is_checked_task: matches!(task, Some(MarkdownTaskState::Checked)),
            text_indent: 0.0,
        },
        MarkdownLine::CodeBlock { .. } => LinePresentation {
            font_size: FONT_SIZE,
            line_height: LINE_HEIGHT,
            is_heading: false,
            is_blockquote: false,
            is_code_block: true,
            is_checked_task: false,
            text_indent: CODE_BLOCK_TEXT_INSET,
        },
        MarkdownLine::Blank | MarkdownLine::Paragraph => LinePresentation {
            font_size: FONT_SIZE,
            line_height: LINE_HEIGHT,
            is_heading: false,
            is_blockquote: false,
            is_code_block: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::FontStyle;
    use hanji_core::Document;
    use hanji_markdown::{MarkdownCodeBlockLine, OrderedListDelimiter};

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
    fn fenced_code_block_lines_use_block_presentation() {
        let presentation = line_presentation(MarkdownLine::CodeBlock {
            role: MarkdownCodeBlockLine::Content,
        });

        assert!(!presentation.is_heading);
        assert!(!presentation.is_blockquote);
        assert!(presentation.is_code_block);
        assert_eq!(presentation.font_size, FONT_SIZE);
        assert_eq!(presentation.line_height, LINE_HEIGHT);
        assert_eq!(presentation.text_indent, CODE_BLOCK_TEXT_INSET);
    }

    #[test]
    fn fenced_code_block_content_uses_line_background_not_inline_ranges() {
        let document = Document::new("```\nlet value = `literal`;\n```");
        let projection = project_document(&document);
        let line = &projection.lines()[1];
        let segments = line.visible_segments();

        assert!(line_presentation(line.kind).is_code_block);
        assert!(
            segments
                .iter()
                .all(|segment| matches!(segment.kind, ProjectedSegmentKind::CodeBlockContent))
        );
        assert!(code_background_ranges(&segments).is_empty());
    }

    #[test]
    fn horizontal_rule_lines_use_plain_line_presentation() {
        let presentation = line_presentation(MarkdownLine::HorizontalRule);

        assert!(!presentation.is_heading);
        assert!(!presentation.is_blockquote);
        assert!(!presentation.is_code_block);
        assert!(!presentation.is_checked_task);
        assert_eq!(presentation.font_size, FONT_SIZE);
        assert_eq!(presentation.line_height, LINE_HEIGHT);
        assert_eq!(presentation.text_indent, 0.0);
    }

    #[test]
    fn revealed_horizontal_rule_marker_uses_green_syntax_color() {
        let document = Document::new("---");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments = line.visible_segments_revealing_source_in(Some(TextRange::caret(1)));
        let runs = line_text_runs(
            &segments,
            line_presentation(line.kind),
            &TextStyle::default(),
        );

        assert_eq!(segments[0].kind, ProjectedSegmentKind::HorizontalRuleMarker);
        assert_eq!(runs[0].color, rgb(MARKDOWN_MARKER_COLOR).into());
    }

    #[test]
    fn consecutive_fenced_code_lines_share_one_background_run() {
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
                true,
            ),
            (
                Bounds::new(point(px(0.0), px(72.0)), size(px(100.0), px(24.0))),
                false,
            ),
        ];

        let runs = collect_code_block_background_bounds(&lines);

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].top(), px(0.0));
        assert_eq!(runs[0].bottom(), px(72.0));
    }

    #[test]
    fn revealed_fenced_code_marker_uses_green_syntax_color() {
        let source = "```rust\nlet value = 1;\n```";
        let document = Document::new(source);
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments = line.visible_segments_revealing_source_in(Some(TextRange::caret(
            source.find("value").unwrap(),
        )));
        let runs = line_text_runs(
            &segments,
            line_presentation(line.kind),
            &TextStyle::default(),
        );

        assert_eq!(segments[0].kind, ProjectedSegmentKind::CodeBlockFence);
        assert_eq!(runs[0].color, rgb(MARKDOWN_MARKER_COLOR).into());
    }

    #[test]
    fn blockquote_lines_use_indented_presentation() {
        let presentation = line_presentation(MarkdownLine::Blockquote);

        assert!(!presentation.is_heading);
        assert!(presentation.is_blockquote);
        assert!(!presentation.is_code_block);
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
        assert!(!presentation.is_code_block);
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
    fn link_text_uses_plain_color_with_underline() {
        let document = Document::new("Read [Hanji](https://hanji.local)");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let text_style = TextStyle::default();
        let segments = line.visible_segments();
        let runs = line_text_runs(&segments, line_presentation(line.kind), &text_style);
        let link_run = segments
            .iter()
            .zip(runs.iter())
            .find(|(segment, _)| matches!(segment.kind, ProjectedSegmentKind::LinkText))
            .map(|(_, run)| run)
            .expect("link text run");

        assert_eq!(link_run.color, text_style.color);
        assert!(link_run.underline.is_some());
    }

    #[test]
    fn strikethrough_content_uses_line_through() {
        let document = Document::new("Remove ~~this~~ text");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let text_style = TextStyle::default();
        let segments = line.visible_segments();
        let runs = line_text_runs(&segments, line_presentation(line.kind), &text_style);
        let strike_run = segments
            .iter()
            .zip(runs.iter())
            .find(|(segment, _)| matches!(segment.kind, ProjectedSegmentKind::StrikethroughContent))
            .map(|(_, run)| run)
            .expect("strikethrough text run");

        assert_eq!(strike_run.color, text_style.color);
        assert!(strike_run.strikethrough.is_some());
    }

    #[test]
    fn revealed_link_markers_use_green_and_escape_markers_use_muted_color() {
        let source = "Read [Hanji](https://hanji.local) and \\*";
        let document = Document::new(source);
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments = line.visible_segments_revealing_source_in(Some(TextRange::new(
            "Read ".len(),
            source.len(),
        )));
        let runs = line_text_runs(
            &segments,
            line_presentation(line.kind),
            &TextStyle::default(),
        );

        for (segment, run) in segments.iter().zip(runs.iter()) {
            match segment.kind {
                ProjectedSegmentKind::LinkMarker => {
                    assert_eq!(run.color, rgb(MARKDOWN_MARKER_COLOR).into());
                }
                ProjectedSegmentKind::EscapeMarker => {
                    assert_eq!(run.color, rgb(0x8f8a82).into());
                }
                _ => {}
            }
        }
    }

    #[test]
    fn extracts_link_destination_from_source_range() {
        let source = "Read [Hanji](https://hanji.local) now";

        assert_eq!(
            link_destination_from_source(source, 0, TextRange::new(5, 33)),
            Some("https://hanji.local")
        );
        assert_eq!(
            link_destination_from_source(source, 0, TextRange::new(0, source.len())),
            None
        );
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
}
