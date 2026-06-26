use hanji_core::{Document, TextRange};

use crate::{
    MarkdownCodeBlockLine, MarkdownLine, blockquote_content_start, classify_line,
    heading_content_start, horizontal_rule_marker_range, list_item_content_start,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownProjection<'a> {
    lines: Vec<ProjectedLine<'a>>,
}

impl<'a> MarkdownProjection<'a> {
    pub fn lines(&self) -> &[ProjectedLine<'a>] {
        &self.lines
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectedSegmentKind {
    Text,
    HeadingMarker,
    HorizontalRuleMarker,
    BlockquoteMarker,
    ListMarker,
    EscapeMarker,
    StrongMarker,
    StrongContent,
    EmphasisMarker,
    EmphasisContent,
    CodeMarker,
    CodeContent,
    CodeBlockFence,
    CodeBlockContent,
    LinkMarker,
    LinkText,
    LinkDestination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectedSegment<'a> {
    pub range: TextRange,
    pub source: &'a str,
    pub kind: ProjectedSegmentKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibleOffsetAffinity {
    Before,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectedVisibleSegment<'a> {
    pub visible_range: TextRange,
    pub source_range: TextRange,
    pub source_outer_range: TextRange,
    pub source: &'a str,
    pub kind: ProjectedSegmentKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownInline {
    Text,
    Escape { marker: TextRange },
    Strong { markers: MarkdownMarkerRanges },
    Emphasis { markers: MarkdownMarkerRanges },
    Code { markers: MarkdownMarkerRanges },
    Link { markers: MarkdownLinkRanges },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkdownMarkerRanges {
    pub opening: TextRange,
    pub closing: TextRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkdownLinkRanges {
    pub opening: TextRange,
    pub separator: TextRange,
    pub destination: TextRange,
    pub closing: TextRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectedInline<'a> {
    pub source_range: TextRange,
    pub content_range: TextRange,
    pub source: &'a str,
    pub content: &'a str,
    pub kind: MarkdownInline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedLine<'a> {
    pub range: TextRange,
    pub marker_range: Option<TextRange>,
    pub code_block_range: Option<TextRange>,
    pub source: &'a str,
    pub kind: MarkdownLine,
    pub inlines: Vec<ProjectedInline<'a>>,
}

impl<'a> ProjectedLine<'a> {
    pub fn visible_text(&self) -> String {
        self.visible_text_revealing_source_in(None)
    }

    pub fn visible_text_revealing_source_in(&self, reveal_range: Option<TextRange>) -> String {
        let mut text = String::new();

        for segment in self.visible_segments_revealing_source_in(reveal_range) {
            text.push_str(segment.source);
        }

        text
    }

    pub fn visible_len(&self) -> usize {
        self.visible_segments()
            .last()
            .map_or(0, |segment| segment.visible_range.end)
    }

    pub fn visible_segments(&self) -> Vec<ProjectedVisibleSegment<'a>> {
        self.visible_segments_revealing_source_in(None)
    }

    pub fn visible_segments_revealing_source_in(
        &self,
        reveal_range: Option<TextRange>,
    ) -> Vec<ProjectedVisibleSegment<'a>> {
        if matches!(self.kind, MarkdownLine::Heading { .. }) {
            return self.heading_visible_segments_revealing_source_in(reveal_range);
        }

        if matches!(self.kind, MarkdownLine::CodeBlock { .. }) {
            return self.code_block_visible_segments_revealing_source_in(reveal_range);
        }

        if matches!(self.kind, MarkdownLine::HorizontalRule) {
            return self.horizontal_rule_visible_segments_revealing_source_in(reveal_range);
        }

        let mut segments = Vec::new();
        let mut visible_start = 0;
        let reveal_marker = self.should_reveal_marker(reveal_range);

        if reveal_marker && let Some(marker_range) = self.marker_range {
            push_visible_segment(
                &mut segments,
                self.source,
                self.range.start,
                &mut visible_start,
                marker_range,
                marker_range,
                block_marker_kind(self.kind),
            );
        }

        for (index, inline) in self.inlines.iter().enumerate() {
            if reveal_range.is_some_and(|range| self.should_reveal_inline_source(inline, range)) {
                push_source_visible_segments(
                    &mut segments,
                    self.source,
                    self.range.start,
                    &mut visible_start,
                    inline,
                );
            } else {
                push_hidden_visible_segment(
                    &mut segments,
                    self.source,
                    self.range.start,
                    &mut visible_start,
                    inline,
                    self.hidden_source_outer_range(index, inline, reveal_marker),
                );
            }
        }

        segments
    }

    pub fn source_to_visible_offset(&self, source_offset: usize) -> usize {
        let source_offset = source_offset.clamp(self.range.start, self.range.end);

        for segment in self.visible_segments() {
            if source_offset < segment.source_outer_range.start {
                return segment.visible_range.start;
            }

            if source_offset <= segment.source_outer_range.end {
                if source_offset < segment.source_range.start {
                    return segment.visible_range.start;
                }

                if source_offset <= segment.source_range.end {
                    return segment.visible_range.start + source_offset
                        - segment.source_range.start;
                }

                return segment.visible_range.end;
            }
        }

        self.visible_len()
    }

    pub fn visible_to_source_offset(
        &self,
        visible_offset: usize,
        affinity: VisibleOffsetAffinity,
    ) -> usize {
        let visible_offset = visible_offset.min(self.visible_len());
        let segments = self.visible_segments();

        for (index, segment) in segments.iter().enumerate() {
            if visible_offset < segment.visible_range.start {
                return segment.source_outer_range.start;
            }

            if visible_offset > segment.visible_range.end {
                continue;
            }

            if visible_offset == segment.visible_range.start
                && matches!(affinity, VisibleOffsetAffinity::Before)
            {
                return segment.source_outer_range.start;
            }

            if visible_offset == segment.visible_range.end
                && matches!(affinity, VisibleOffsetAffinity::After)
            {
                if let Some(next_segment) = segments.get(index + 1)
                    && next_segment.visible_range.start == visible_offset
                {
                    return next_segment.source_range.start;
                }

                return segment.source_outer_range.end;
            }

            return segment.source_range.start + visible_offset - segment.visible_range.start;
        }

        self.range.end
    }

    pub fn visible_to_source_caret_offset(&self, visible_offset: usize) -> usize {
        let segments = self.visible_segments();
        let visible_len = segments
            .last()
            .map_or(0, |segment| segment.visible_range.end);

        visible_segments_to_source_caret_offset(&segments, self.range, visible_len, visible_offset)
    }

    pub fn source_visible_segments(&self) -> Vec<ProjectedSegment<'a>> {
        if matches!(self.kind, MarkdownLine::Heading { .. }) {
            return self.heading_source_visible_segments();
        }

        if matches!(self.kind, MarkdownLine::CodeBlock { .. }) {
            return self.code_block_source_visible_segments();
        }

        if matches!(self.kind, MarkdownLine::HorizontalRule) {
            return self.horizontal_rule_source_visible_segments();
        }

        let mut segments = Vec::new();

        if let Some(marker_range) = self.marker_range {
            push_segment(
                &mut segments,
                self.source,
                self.range.start,
                marker_range,
                block_marker_kind(self.kind),
            );
        }

        push_inline_source_segments(&mut segments, self.source, self.range.start, &self.inlines);

        segments
    }

    fn heading_visible_segments_revealing_source_in(
        &self,
        reveal_range: Option<TextRange>,
    ) -> Vec<ProjectedVisibleSegment<'a>> {
        let mut segments = Vec::new();
        let mut visible_start = 0;
        let content_start = self
            .inlines
            .first()
            .map_or(self.range.end, |inline| inline.source_range.start);
        let reveal_heading_source =
            reveal_range.is_some_and(|range| should_reveal_line_source(self.range, range));

        if !reveal_heading_source {
            for (index, inline) in self.inlines.iter().enumerate() {
                let source_outer_range = if index == 0 {
                    TextRange::new(self.range.start, inline.source_range.end)
                } else {
                    inline.source_range
                };

                push_hidden_visible_segment(
                    &mut segments,
                    self.source,
                    self.range.start,
                    &mut visible_start,
                    inline,
                    source_outer_range,
                );
            }

            return segments;
        }

        if let Some(marker_range) = self.marker_range {
            push_visible_segment(
                &mut segments,
                self.source,
                self.range.start,
                &mut visible_start,
                TextRange::new(self.range.start, marker_range.start),
                TextRange::new(self.range.start, marker_range.start),
                ProjectedSegmentKind::Text,
            );
            push_visible_segment(
                &mut segments,
                self.source,
                self.range.start,
                &mut visible_start,
                marker_range,
                marker_range,
                ProjectedSegmentKind::HeadingMarker,
            );
            push_visible_segment(
                &mut segments,
                self.source,
                self.range.start,
                &mut visible_start,
                TextRange::new(marker_range.end, content_start),
                TextRange::new(marker_range.end, content_start),
                ProjectedSegmentKind::Text,
            );
        }

        for inline in &self.inlines {
            if reveal_range.is_some_and(|range| self.should_reveal_inline_source(inline, range)) {
                push_source_visible_segments(
                    &mut segments,
                    self.source,
                    self.range.start,
                    &mut visible_start,
                    inline,
                );
            } else {
                push_hidden_visible_segment(
                    &mut segments,
                    self.source,
                    self.range.start,
                    &mut visible_start,
                    inline,
                    inline.source_range,
                );
            }
        }

        segments
    }

    fn heading_source_visible_segments(&self) -> Vec<ProjectedSegment<'a>> {
        let mut segments = Vec::new();
        let content_start = self
            .inlines
            .first()
            .map_or(self.range.end, |inline| inline.source_range.start);

        if let Some(marker_range) = self.marker_range {
            push_segment(
                &mut segments,
                self.source,
                self.range.start,
                TextRange::new(self.range.start, marker_range.start),
                ProjectedSegmentKind::Text,
            );
            push_segment(
                &mut segments,
                self.source,
                self.range.start,
                marker_range,
                ProjectedSegmentKind::HeadingMarker,
            );
            push_segment(
                &mut segments,
                self.source,
                self.range.start,
                TextRange::new(marker_range.end, content_start),
                ProjectedSegmentKind::Text,
            );
        }

        push_inline_source_segments(&mut segments, self.source, self.range.start, &self.inlines);

        segments
    }

    fn horizontal_rule_visible_segments_revealing_source_in(
        &self,
        reveal_range: Option<TextRange>,
    ) -> Vec<ProjectedVisibleSegment<'a>> {
        if !reveal_range.is_some_and(|range| should_reveal_line_source(self.range, range)) {
            return Vec::new();
        }

        self.horizontal_rule_source_visible_segments()
            .into_iter()
            .scan(0, |visible_start, segment| {
                let start = *visible_start;
                *visible_start += segment.source.len();
                Some(ProjectedVisibleSegment {
                    visible_range: TextRange::new(start, *visible_start),
                    source_range: segment.range,
                    source_outer_range: segment.range,
                    source: segment.source,
                    kind: segment.kind,
                })
            })
            .collect()
    }

    fn horizontal_rule_source_visible_segments(&self) -> Vec<ProjectedSegment<'a>> {
        let mut segments = Vec::new();

        if let Some(marker_range) = self.marker_range {
            push_segment(
                &mut segments,
                self.source,
                self.range.start,
                TextRange::new(self.range.start, marker_range.start),
                ProjectedSegmentKind::Text,
            );
            push_segment(
                &mut segments,
                self.source,
                self.range.start,
                marker_range,
                ProjectedSegmentKind::HorizontalRuleMarker,
            );
            push_segment(
                &mut segments,
                self.source,
                self.range.start,
                TextRange::new(marker_range.end, self.range.end),
                ProjectedSegmentKind::Text,
            );
        }

        segments
    }

    fn code_block_visible_segments_revealing_source_in(
        &self,
        reveal_range: Option<TextRange>,
    ) -> Vec<ProjectedVisibleSegment<'a>> {
        let MarkdownLine::CodeBlock { role } = self.kind else {
            return Vec::new();
        };

        match role {
            MarkdownCodeBlockLine::Content => self.code_block_content_visible_segments(),
            MarkdownCodeBlockLine::OpeningFence | MarkdownCodeBlockLine::ClosingFence => {
                if !self.should_reveal_code_block_source(reveal_range) {
                    return Vec::new();
                }

                self.code_block_fence_visible_segments()
            }
        }
    }

    fn code_block_content_visible_segments(&self) -> Vec<ProjectedVisibleSegment<'a>> {
        let mut segments = Vec::new();
        let mut visible_start = 0;

        push_visible_segment(
            &mut segments,
            self.source,
            self.range.start,
            &mut visible_start,
            self.range,
            self.range,
            ProjectedSegmentKind::CodeBlockContent,
        );

        segments
    }

    fn code_block_fence_visible_segments(&self) -> Vec<ProjectedVisibleSegment<'a>> {
        let mut segments = Vec::new();
        let mut visible_start = 0;

        if let Some(marker_range) = self.marker_range {
            push_visible_segment(
                &mut segments,
                self.source,
                self.range.start,
                &mut visible_start,
                TextRange::new(self.range.start, marker_range.start),
                TextRange::new(self.range.start, marker_range.start),
                ProjectedSegmentKind::Text,
            );
            push_visible_segment(
                &mut segments,
                self.source,
                self.range.start,
                &mut visible_start,
                marker_range,
                marker_range,
                ProjectedSegmentKind::CodeBlockFence,
            );
            push_visible_segment(
                &mut segments,
                self.source,
                self.range.start,
                &mut visible_start,
                TextRange::new(marker_range.end, self.range.end),
                TextRange::new(marker_range.end, self.range.end),
                ProjectedSegmentKind::Text,
            );
        }

        segments
    }

    fn code_block_source_visible_segments(&self) -> Vec<ProjectedSegment<'a>> {
        let MarkdownLine::CodeBlock { role } = self.kind else {
            return Vec::new();
        };

        let mut segments = Vec::new();

        match role {
            MarkdownCodeBlockLine::Content => push_segment(
                &mut segments,
                self.source,
                self.range.start,
                self.range,
                ProjectedSegmentKind::CodeBlockContent,
            ),
            MarkdownCodeBlockLine::OpeningFence | MarkdownCodeBlockLine::ClosingFence => {
                if let Some(marker_range) = self.marker_range {
                    push_segment(
                        &mut segments,
                        self.source,
                        self.range.start,
                        TextRange::new(self.range.start, marker_range.start),
                        ProjectedSegmentKind::Text,
                    );
                    push_segment(
                        &mut segments,
                        self.source,
                        self.range.start,
                        marker_range,
                        ProjectedSegmentKind::CodeBlockFence,
                    );
                    push_segment(
                        &mut segments,
                        self.source,
                        self.range.start,
                        TextRange::new(marker_range.end, self.range.end),
                        ProjectedSegmentKind::Text,
                    );
                }
            }
        }

        segments
    }

    fn should_reveal_code_block_source(&self, reveal_range: Option<TextRange>) -> bool {
        let Some(reveal_range) = reveal_range else {
            return false;
        };
        let block_range = self.code_block_range.unwrap_or(self.range);

        should_reveal_line_source(block_range, reveal_range)
    }

    fn hidden_source_outer_range(
        &self,
        inline_index: usize,
        inline: &ProjectedInline<'_>,
        marker_revealed: bool,
    ) -> TextRange {
        if !marker_revealed
            && inline_index == 0
            && let Some(marker_range) = self.marker_range
        {
            return TextRange::new(marker_range.start, inline.source_range.end);
        }

        inline.source_range
    }

    fn should_reveal_marker(&self, reveal_range: Option<TextRange>) -> bool {
        let Some(marker_range) = self.marker_range else {
            return false;
        };

        reveal_range.is_some_and(|range| should_reveal_line_marker(marker_range, range))
    }

    fn should_reveal_inline_source(
        &self,
        inline: &ProjectedInline<'_>,
        reveal_range: TextRange,
    ) -> bool {
        if should_reveal_inline(inline.source_range, reveal_range) {
            return true;
        }

        if !matches!(inline.kind, MarkdownInline::Escape { .. }) {
            return false;
        }

        should_reveal_escape_marker(
            self.source,
            self.range.start,
            inline.source_range,
            reveal_range,
        )
    }
}

fn visible_segments_to_source_caret_offset(
    segments: &[ProjectedVisibleSegment<'_>],
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

fn should_reveal_inline(inline_range: TextRange, reveal_range: TextRange) -> bool {
    if reveal_range.is_empty() {
        reveal_range.start >= inline_range.start && reveal_range.start <= inline_range.end
    } else {
        reveal_range.start < inline_range.end && reveal_range.end > inline_range.start
    }
}

fn should_reveal_line_marker(marker_range: TextRange, reveal_range: TextRange) -> bool {
    if reveal_range.is_empty() {
        reveal_range.start >= marker_range.start && reveal_range.start < marker_range.end
    } else {
        reveal_range.start < marker_range.end && reveal_range.end > marker_range.start
    }
}

fn should_reveal_line_source(line_range: TextRange, reveal_range: TextRange) -> bool {
    if reveal_range.is_empty() {
        reveal_range.start >= line_range.start && reveal_range.start <= line_range.end
    } else {
        reveal_range.start < line_range.end && reveal_range.end > line_range.start
    }
}

fn should_reveal_escape_marker(
    line_source: &str,
    line_start: usize,
    escape_range: TextRange,
    reveal_range: TextRange,
) -> bool {
    let token_range = escaped_token_range(line_source, line_start, escape_range);

    should_reveal_inline(token_range, reveal_range)
}

fn escaped_token_range(line_source: &str, line_start: usize, escape_range: TextRange) -> TextRange {
    let mut start = escape_range.start.saturating_sub(line_start);
    let mut end = escape_range.end.saturating_sub(line_start);

    while let Some((previous_start, character)) = previous_char_with_start(line_source, start) {
        if character.is_whitespace() {
            break;
        }

        start = previous_start;
    }

    while end < line_source.len() {
        let Some(character) = line_source[end..].chars().next() else {
            break;
        };
        if character.is_whitespace() {
            break;
        }

        end += character.len_utf8();
    }

    TextRange::new(line_start + start, line_start + end)
}

fn previous_char_with_start(source: &str, end: usize) -> Option<(usize, char)> {
    source[..end].char_indices().next_back()
}

pub fn project_document(document: &Document) -> MarkdownProjection<'_> {
    let mut lines = Vec::new();
    let mut line_index = 0;

    while line_index < document.line_count() {
        let (range, source) = document_line(document, line_index);

        if let Some(opening_fence) = opening_code_fence(source, range.start)
            && let Some((closing_line_index, closing_fence)) = find_closing_code_fence(
                document,
                line_index + 1,
                opening_fence.marker,
                opening_fence.marker_len,
            )
        {
            let code_block_range = TextRange::new(range.start, closing_fence.line_range.end);

            lines.push(project_code_block_line(
                range,
                source,
                MarkdownCodeBlockLine::OpeningFence,
                Some(opening_fence.marker_range),
                code_block_range,
            ));

            for content_line_index in line_index + 1..closing_line_index {
                let (content_range, content_source) = document_line(document, content_line_index);
                lines.push(project_code_block_line(
                    content_range,
                    content_source,
                    MarkdownCodeBlockLine::Content,
                    None,
                    code_block_range,
                ));
            }

            let (closing_range, closing_source) = document_line(document, closing_line_index);
            lines.push(project_code_block_line(
                closing_range,
                closing_source,
                MarkdownCodeBlockLine::ClosingFence,
                Some(closing_fence.marker_range),
                code_block_range,
            ));

            line_index = closing_line_index + 1;
            continue;
        }

        lines.push(project_normal_line(range, source));
        line_index += 1;
    }

    MarkdownProjection { lines }
}

fn document_line(document: &Document, line_index: usize) -> (TextRange, &str) {
    let range = document
        .line_range(line_index)
        .unwrap_or_else(|| TextRange::caret(document.len()));

    (range, &document.text()[range.start..range.end])
}

fn project_normal_line<'a>(range: TextRange, source: &'a str) -> ProjectedLine<'a> {
    let kind = classify_line(source);
    let content_start = match kind {
        MarkdownLine::Heading { .. } => heading_content_start(source).unwrap_or(0),
        MarkdownLine::HorizontalRule => 0,
        MarkdownLine::Blockquote => blockquote_content_start(source).unwrap_or(0),
        MarkdownLine::ListItem { .. } => list_item_content_start(source).unwrap_or(0),
        MarkdownLine::Blank | MarkdownLine::Paragraph | MarkdownLine::CodeBlock { .. } => 0,
    };
    let marker_range = match kind {
        MarkdownLine::Heading { .. } => heading_marker_range(source, range.start),
        MarkdownLine::HorizontalRule => horizontal_rule_marker_range(source)
            .map(|marker| TextRange::new(range.start + marker.start, range.start + marker.end)),
        MarkdownLine::Blockquote | MarkdownLine::ListItem { .. } => {
            (content_start > 0).then_some(TextRange::new(range.start, range.start + content_start))
        }
        MarkdownLine::Blank | MarkdownLine::Paragraph | MarkdownLine::CodeBlock { .. } => None,
    };

    ProjectedLine {
        range,
        marker_range,
        code_block_range: None,
        source,
        kind,
        inlines: if matches!(kind, MarkdownLine::HorizontalRule) {
            Vec::new()
        } else {
            project_inlines(&source[content_start..], range.start + content_start)
        },
    }
}

fn project_code_block_line<'a>(
    range: TextRange,
    source: &'a str,
    role: MarkdownCodeBlockLine,
    marker_range: Option<TextRange>,
    code_block_range: TextRange,
) -> ProjectedLine<'a> {
    ProjectedLine {
        range,
        marker_range,
        code_block_range: Some(code_block_range),
        source,
        kind: MarkdownLine::CodeBlock { role },
        inlines: Vec::new(),
    }
}

fn block_marker_kind(kind: MarkdownLine) -> ProjectedSegmentKind {
    match kind {
        MarkdownLine::Heading { .. } => ProjectedSegmentKind::HeadingMarker,
        MarkdownLine::HorizontalRule => ProjectedSegmentKind::HorizontalRuleMarker,
        MarkdownLine::Blockquote => ProjectedSegmentKind::BlockquoteMarker,
        MarkdownLine::ListItem { .. } => ProjectedSegmentKind::ListMarker,
        MarkdownLine::Blank | MarkdownLine::Paragraph | MarkdownLine::CodeBlock { .. } => {
            ProjectedSegmentKind::Text
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CodeFence {
    marker: CodeFenceMarker,
    marker_len: usize,
    marker_range: TextRange,
    line_range: TextRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

fn opening_code_fence(source: &str, line_start: usize) -> Option<CodeFence> {
    code_fence(source, line_start, 3, false, None)
}

fn closing_code_fence(
    source: &str,
    line_start: usize,
    marker: CodeFenceMarker,
    min_marker_len: usize,
) -> Option<CodeFence> {
    code_fence(source, line_start, min_marker_len, true, Some(marker))
}

fn code_fence(
    source: &str,
    line_start: usize,
    min_marker_len: usize,
    require_trailing_whitespace: bool,
    required_marker: Option<CodeFenceMarker>,
) -> Option<CodeFence> {
    let indent = source
        .bytes()
        .take_while(|byte| *byte == b' ')
        .take(4)
        .count();
    if indent >= 4 {
        return None;
    }

    let content = &source[indent..];
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

    Some(CodeFence {
        marker,
        marker_len,
        marker_range: TextRange::new(line_start + indent, line_start + indent + marker_len),
        line_range: TextRange::new(line_start, line_start + source.len()),
    })
}

fn find_closing_code_fence(
    document: &Document,
    start_line_index: usize,
    marker: CodeFenceMarker,
    marker_len: usize,
) -> Option<(usize, CodeFence)> {
    for line_index in start_line_index..document.line_count() {
        let (range, source) = document_line(document, line_index);
        if let Some(fence) = closing_code_fence(source, range.start, marker, marker_len) {
            return Some((line_index, fence));
        }
    }

    None
}

fn heading_marker_range(source: &str, line_start: usize) -> Option<TextRange> {
    let indent = source
        .bytes()
        .take_while(|byte| *byte == b' ')
        .take(4)
        .count();
    let content = &source[indent..];
    let level = content.bytes().take_while(|byte| *byte == b'#').count();

    if !(1..=6).contains(&level) || !matches!(content.as_bytes().get(level), Some(b' ' | b'\t')) {
        return None;
    }

    Some(TextRange::new(
        line_start + indent,
        line_start + indent + level,
    ))
}

fn push_segment<'a>(
    segments: &mut Vec<ProjectedSegment<'a>>,
    line_source: &'a str,
    line_start: usize,
    range: TextRange,
    kind: ProjectedSegmentKind,
) {
    if range.is_empty() {
        return;
    }

    let start = range.start - line_start;
    let end = range.end - line_start;

    segments.push(ProjectedSegment {
        range,
        source: &line_source[start..end],
        kind,
    });
}

fn push_inline_source_segments<'a>(
    segments: &mut Vec<ProjectedSegment<'a>>,
    line_source: &'a str,
    line_start: usize,
    inlines: &[ProjectedInline<'a>],
) {
    for inline in inlines {
        match inline.kind {
            MarkdownInline::Text => {
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    inline.source_range,
                    ProjectedSegmentKind::Text,
                );
            }
            MarkdownInline::Escape { marker } => {
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    marker,
                    ProjectedSegmentKind::EscapeMarker,
                );
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    inline.content_range,
                    ProjectedSegmentKind::Text,
                );
            }
            MarkdownInline::Strong { markers } => {
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    markers.opening,
                    ProjectedSegmentKind::StrongMarker,
                );
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    inline.content_range,
                    ProjectedSegmentKind::StrongContent,
                );
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    markers.closing,
                    ProjectedSegmentKind::StrongMarker,
                );
            }
            MarkdownInline::Emphasis { markers } => {
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    markers.opening,
                    ProjectedSegmentKind::EmphasisMarker,
                );
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    inline.content_range,
                    ProjectedSegmentKind::EmphasisContent,
                );
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    markers.closing,
                    ProjectedSegmentKind::EmphasisMarker,
                );
            }
            MarkdownInline::Code { markers } => {
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    markers.opening,
                    ProjectedSegmentKind::CodeMarker,
                );
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    inline.content_range,
                    ProjectedSegmentKind::CodeContent,
                );
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    markers.closing,
                    ProjectedSegmentKind::CodeMarker,
                );
            }
            MarkdownInline::Link { markers } => {
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    markers.opening,
                    ProjectedSegmentKind::LinkMarker,
                );
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    inline.content_range,
                    ProjectedSegmentKind::LinkText,
                );
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    markers.separator,
                    ProjectedSegmentKind::LinkMarker,
                );
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    markers.destination,
                    ProjectedSegmentKind::LinkDestination,
                );
                push_segment(
                    segments,
                    line_source,
                    line_start,
                    markers.closing,
                    ProjectedSegmentKind::LinkMarker,
                );
            }
        }
    }
}

fn push_visible_segment<'a>(
    segments: &mut Vec<ProjectedVisibleSegment<'a>>,
    line_source: &'a str,
    line_start: usize,
    visible_start: &mut usize,
    source_outer_range: TextRange,
    source_range: TextRange,
    kind: ProjectedSegmentKind,
) {
    if source_range.is_empty() {
        return;
    }

    let start = source_range.start - line_start;
    let end = source_range.end - line_start;
    let len = source_range.len();

    segments.push(ProjectedVisibleSegment {
        visible_range: TextRange::new(*visible_start, *visible_start + len),
        source_range,
        source_outer_range,
        source: &line_source[start..end],
        kind,
    });

    *visible_start += len;
}

fn push_hidden_visible_segment<'a>(
    segments: &mut Vec<ProjectedVisibleSegment<'a>>,
    line_source: &'a str,
    line_start: usize,
    visible_start: &mut usize,
    inline: &ProjectedInline<'a>,
    source_outer_range: TextRange,
) {
    let kind = match inline.kind {
        MarkdownInline::Text => ProjectedSegmentKind::Text,
        MarkdownInline::Escape { .. } => ProjectedSegmentKind::Text,
        MarkdownInline::Strong { .. } => ProjectedSegmentKind::StrongContent,
        MarkdownInline::Emphasis { .. } => ProjectedSegmentKind::EmphasisContent,
        MarkdownInline::Code { .. } => ProjectedSegmentKind::CodeContent,
        MarkdownInline::Link { .. } => ProjectedSegmentKind::LinkText,
    };

    push_visible_segment(
        segments,
        line_source,
        line_start,
        visible_start,
        source_outer_range,
        inline.content_range,
        kind,
    );
}

fn push_source_visible_segments<'a>(
    segments: &mut Vec<ProjectedVisibleSegment<'a>>,
    line_source: &'a str,
    line_start: usize,
    visible_start: &mut usize,
    inline: &ProjectedInline<'a>,
) {
    match inline.kind {
        MarkdownInline::Text => {
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                inline.source_range,
                inline.source_range,
                ProjectedSegmentKind::Text,
            );
        }
        MarkdownInline::Escape { marker } => {
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                marker,
                marker,
                ProjectedSegmentKind::EscapeMarker,
            );
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                inline.content_range,
                inline.content_range,
                ProjectedSegmentKind::Text,
            );
        }
        MarkdownInline::Strong { markers } => {
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                markers.opening,
                markers.opening,
                ProjectedSegmentKind::StrongMarker,
            );
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                inline.content_range,
                inline.content_range,
                ProjectedSegmentKind::StrongContent,
            );
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                markers.closing,
                markers.closing,
                ProjectedSegmentKind::StrongMarker,
            );
        }
        MarkdownInline::Emphasis { markers } => {
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                markers.opening,
                markers.opening,
                ProjectedSegmentKind::EmphasisMarker,
            );
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                inline.content_range,
                inline.content_range,
                ProjectedSegmentKind::EmphasisContent,
            );
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                markers.closing,
                markers.closing,
                ProjectedSegmentKind::EmphasisMarker,
            );
        }
        MarkdownInline::Code { markers } => {
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                markers.opening,
                markers.opening,
                ProjectedSegmentKind::CodeMarker,
            );
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                inline.content_range,
                inline.content_range,
                ProjectedSegmentKind::CodeContent,
            );
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                markers.closing,
                markers.closing,
                ProjectedSegmentKind::CodeMarker,
            );
        }
        MarkdownInline::Link { markers } => {
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                markers.opening,
                markers.opening,
                ProjectedSegmentKind::LinkMarker,
            );
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                inline.content_range,
                inline.content_range,
                ProjectedSegmentKind::LinkText,
            );
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                markers.separator,
                markers.separator,
                ProjectedSegmentKind::LinkMarker,
            );
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                markers.destination,
                markers.destination,
                ProjectedSegmentKind::LinkDestination,
            );
            push_visible_segment(
                segments,
                line_source,
                line_start,
                visible_start,
                markers.closing,
                markers.closing,
                ProjectedSegmentKind::LinkMarker,
            );
        }
    }
}

fn project_inlines(source: &str, source_start: usize) -> Vec<ProjectedInline<'_>> {
    let mut inlines = Vec::new();
    let mut text_start = 0;
    let mut search_start = 0;

    while let Some(opening_start) = find_code_marker(source, search_start) {
        let content_start = opening_start + 1;
        let Some(closing_start) = find_code_marker(source, content_start) else {
            search_start = content_start;
            continue;
        };

        if closing_start == content_start {
            search_start = content_start;
            continue;
        }

        push_text_with_projected_inlines(
            &mut inlines,
            source,
            source_start,
            text_start,
            opening_start,
        );
        push_code_inline(
            &mut inlines,
            source,
            source_start,
            opening_start,
            content_start,
            closing_start,
        );

        search_start = closing_start + 1;
        text_start = search_start;
    }

    push_text_with_projected_inlines(&mut inlines, source, source_start, text_start, source.len());

    inlines
}

fn push_text_with_projected_inlines<'a>(
    inlines: &mut Vec<ProjectedInline<'a>>,
    source: &'a str,
    source_start: usize,
    start: usize,
    end: usize,
) {
    let mut text_start = start;

    for span in inline_spans(source, start, end) {
        let opening_start = span.start();
        if opening_start < text_start {
            continue;
        }

        push_text_inline(inlines, source, source_start, text_start, opening_start);
        push_projected_inline(inlines, source, source_start, span);

        text_start = span.end();
    }

    push_text_inline(inlines, source, source_start, text_start, end);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineSpan {
    Escape(EscapeSpan),
    Emphasis(EmphasisPair),
    Link(LinkSpan),
}

impl InlineSpan {
    fn start(self) -> usize {
        match self {
            Self::Escape(span) => span.start,
            Self::Emphasis(pair) => pair.opening_start,
            Self::Link(span) => span.opening_start,
        }
    }

    fn end(self) -> usize {
        match self {
            Self::Escape(span) => span.end,
            Self::Emphasis(pair) => pair.closing_start + pair.kind.marker_len(),
            Self::Link(span) => span.closing_start + 1,
        }
    }

    fn priority(self) -> u8 {
        match self {
            Self::Escape(_) => 0,
            Self::Link(_) => 1,
            Self::Emphasis(_) => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EscapeSpan {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EmphasisPair {
    kind: EmphasisDelimiterKind,
    opening_start: usize,
    closing_start: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LinkSpan {
    opening_start: usize,
    label_end: usize,
    closing_start: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmphasisDelimiterKind {
    Emphasis,
    Strong,
}

impl EmphasisDelimiterKind {
    fn marker_len(self) -> usize {
        match self {
            Self::Emphasis => 1,
            Self::Strong => 2,
        }
    }
}

fn inline_spans(source: &str, start: usize, end: usize) -> Vec<InlineSpan> {
    let mut candidates = Vec::new();

    candidates.extend(
        escape_spans(source, start, end)
            .into_iter()
            .map(InlineSpan::Escape),
    );
    candidates.extend(
        link_spans(source, start, end)
            .into_iter()
            .map(InlineSpan::Link),
    );
    candidates.extend(
        emphasis_pairs(source, start, end)
            .into_iter()
            .map(InlineSpan::Emphasis),
    );

    candidates.sort_by_key(|span| (span.start(), span.priority(), span.end()));

    let mut spans = Vec::new();
    let mut occupied_end = start;
    for span in candidates {
        if span.start() < occupied_end {
            continue;
        }

        occupied_end = span.end();
        spans.push(span);
    }

    spans
}

fn escape_spans(source: &str, start: usize, end: usize) -> Vec<EscapeSpan> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut cursor = start;

    while cursor < end {
        if bytes[cursor] == b'\\'
            && !is_escaped(source, cursor)
            && let Some(next) = next_char(source, cursor + 1, end)
            && is_escapable_character(next)
        {
            spans.push(EscapeSpan {
                start: cursor,
                end: cursor + 1 + next.len_utf8(),
            });
            cursor += 1 + next.len_utf8();
        } else {
            cursor += 1;
        }
    }

    spans
}

fn link_spans(source: &str, start: usize, end: usize) -> Vec<LinkSpan> {
    let mut spans = Vec::new();
    let mut search_start = start;

    while let Some(opening_start) = find_unescaped_byte(source, b'[', search_start, end) {
        let label_start = opening_start + 1;
        let Some(label_end) = find_link_label_end(source, label_start, end) else {
            search_start = label_start;
            continue;
        };
        if label_end == label_start {
            search_start = label_start;
            continue;
        }

        let destination_start = label_end + 2;
        let Some(closing_start) = find_link_destination_end(source, destination_start, end) else {
            search_start = destination_start;
            continue;
        };
        if closing_start == destination_start
            || source[destination_start..closing_start]
                .chars()
                .any(char::is_whitespace)
        {
            search_start = destination_start;
            continue;
        }

        spans.push(LinkSpan {
            opening_start,
            label_end,
            closing_start,
        });
        search_start = closing_start + 1;
    }

    spans
}

fn find_link_label_end(source: &str, search_start: usize, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = search_start;

    while cursor + 1 < end {
        if bytes[cursor] == b']'
            && !is_escaped(source, cursor)
            && matches!(bytes.get(cursor + 1), Some(b'('))
        {
            return Some(cursor);
        }

        cursor += 1;
    }

    None
}

fn find_link_destination_end(source: &str, search_start: usize, end: usize) -> Option<usize> {
    find_unescaped_byte(source, b')', search_start, end)
}

fn emphasis_pairs(source: &str, start: usize, end: usize) -> Vec<EmphasisPair> {
    let mut pairs = Vec::new();
    let mut pending_emphasis = None;
    let mut pending_strong = None;
    let mut search_start = start;

    while let Some((marker_start, kind)) = find_exact_emphasis_marker(source, search_start, end) {
        let marker_len = kind.marker_len();
        let can_close = can_close_emphasis(source, marker_start);
        let can_open = can_open_emphasis(source, marker_start, marker_len, end);
        let pending_opening = match kind {
            EmphasisDelimiterKind::Emphasis => &mut pending_emphasis,
            EmphasisDelimiterKind::Strong => &mut pending_strong,
        };

        if let Some(opening_start) = *pending_opening
            && can_close
            && opening_start + marker_len < marker_start
        {
            pairs.push(EmphasisPair {
                kind,
                opening_start,
                closing_start: marker_start,
            });
            *pending_opening = None;
        } else if can_open {
            *pending_opening = Some(marker_start);
        }

        search_start = marker_start + marker_len;
    }

    pairs.sort_by_key(|pair| pair.opening_start);
    pairs
}

fn find_exact_emphasis_marker(
    source: &str,
    search_start: usize,
    end: usize,
) -> Option<(usize, EmphasisDelimiterKind)> {
    let bytes = source.as_bytes();
    let mut cursor = search_start;

    while cursor < end {
        if bytes[cursor] == b'*' {
            if is_escaped(source, cursor) {
                cursor += 1;
                continue;
            }

            let marker_start = cursor;

            while cursor < end && bytes[cursor] == b'*' {
                cursor += 1;
            }

            return match cursor - marker_start {
                1 => Some((marker_start, EmphasisDelimiterKind::Emphasis)),
                2 => Some((marker_start, EmphasisDelimiterKind::Strong)),
                _ => continue,
            };
        }

        cursor += 1;
    }

    None
}

fn can_open_emphasis(source: &str, marker_start: usize, marker_len: usize, end: usize) -> bool {
    next_char(source, marker_start + marker_len, end)
        .is_some_and(|character| !character.is_whitespace() && !matches!(character, '*' | '`'))
}

fn can_close_emphasis(source: &str, marker_start: usize) -> bool {
    previous_char(source, marker_start)
        .is_some_and(|character| !character.is_whitespace() && !matches!(character, '*' | '`'))
}

fn push_projected_inline<'a>(
    inlines: &mut Vec<ProjectedInline<'a>>,
    source: &'a str,
    source_start: usize,
    span: InlineSpan,
) {
    match span {
        InlineSpan::Escape(span) => {
            push_escape_inline(inlines, source, source_start, span);
        }
        InlineSpan::Emphasis(pair) => match pair.kind {
            EmphasisDelimiterKind::Emphasis => {
                push_emphasis_inline(
                    inlines,
                    source,
                    source_start,
                    pair.opening_start,
                    pair.opening_start + pair.kind.marker_len(),
                    pair.closing_start,
                );
            }
            EmphasisDelimiterKind::Strong => {
                push_strong_inline(
                    inlines,
                    source,
                    source_start,
                    pair.opening_start,
                    pair.opening_start + pair.kind.marker_len(),
                    pair.closing_start,
                );
            }
        },
        InlineSpan::Link(span) => {
            push_link_inline(inlines, source, source_start, span);
        }
    }
}

fn push_escape_inline<'a>(
    inlines: &mut Vec<ProjectedInline<'a>>,
    source: &'a str,
    source_start: usize,
    span: EscapeSpan,
) {
    let marker = TextRange::new(source_start + span.start, source_start + span.start + 1);
    let content_range = TextRange::new(source_start + span.start + 1, source_start + span.end);

    inlines.push(ProjectedInline {
        source_range: TextRange::new(source_start + span.start, source_start + span.end),
        content_range,
        source: &source[span.start..span.end],
        content: &source[span.start + 1..span.end],
        kind: MarkdownInline::Escape { marker },
    });
}

fn push_emphasis_inline<'a>(
    inlines: &mut Vec<ProjectedInline<'a>>,
    source: &'a str,
    source_start: usize,
    opening_start: usize,
    content_start: usize,
    closing_start: usize,
) {
    let closing_end = closing_start + 1;

    inlines.push(ProjectedInline {
        source_range: TextRange::new(source_start + opening_start, source_start + closing_end),
        content_range: TextRange::new(source_start + content_start, source_start + closing_start),
        source: &source[opening_start..closing_end],
        content: &source[content_start..closing_start],
        kind: MarkdownInline::Emphasis {
            markers: MarkdownMarkerRanges {
                opening: TextRange::new(source_start + opening_start, source_start + content_start),
                closing: TextRange::new(source_start + closing_start, source_start + closing_end),
            },
        },
    });
}

fn next_char(source: &str, start: usize, end: usize) -> Option<char> {
    if start >= end {
        return None;
    }

    source[start..end].chars().next()
}

fn previous_char(source: &str, end: usize) -> Option<char> {
    if end == 0 {
        return None;
    }

    source[..end].chars().next_back()
}

fn find_code_marker(source: &str, search_start: usize) -> Option<usize> {
    find_unescaped_byte(source, b'`', search_start, source.len())
}

fn find_unescaped_byte(source: &str, target: u8, search_start: usize, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = search_start;

    while cursor < end {
        if bytes[cursor] == target && !is_escaped(source, cursor) {
            return Some(cursor);
        }

        cursor += 1;
    }

    None
}

fn is_escaped(source: &str, index: usize) -> bool {
    source[..index]
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\\')
        .count()
        % 2
        == 1
}

fn is_escapable_character(character: char) -> bool {
    character.is_ascii_punctuation()
}

fn push_text_inline<'a>(
    inlines: &mut Vec<ProjectedInline<'a>>,
    source: &'a str,
    source_start: usize,
    start: usize,
    end: usize,
) {
    if start >= end {
        return;
    }

    let range = TextRange::new(source_start + start, source_start + end);

    inlines.push(ProjectedInline {
        source_range: range,
        content_range: range,
        source: &source[start..end],
        content: &source[start..end],
        kind: MarkdownInline::Text,
    });
}

fn push_strong_inline<'a>(
    inlines: &mut Vec<ProjectedInline<'a>>,
    source: &'a str,
    source_start: usize,
    opening_start: usize,
    content_start: usize,
    closing_start: usize,
) {
    let closing_end = closing_start + 2;

    inlines.push(ProjectedInline {
        source_range: TextRange::new(source_start + opening_start, source_start + closing_end),
        content_range: TextRange::new(source_start + content_start, source_start + closing_start),
        source: &source[opening_start..closing_end],
        content: &source[content_start..closing_start],
        kind: MarkdownInline::Strong {
            markers: MarkdownMarkerRanges {
                opening: TextRange::new(source_start + opening_start, source_start + content_start),
                closing: TextRange::new(source_start + closing_start, source_start + closing_end),
            },
        },
    });
}

fn push_code_inline<'a>(
    inlines: &mut Vec<ProjectedInline<'a>>,
    source: &'a str,
    source_start: usize,
    opening_start: usize,
    content_start: usize,
    closing_start: usize,
) {
    let closing_end = closing_start + 1;

    inlines.push(ProjectedInline {
        source_range: TextRange::new(source_start + opening_start, source_start + closing_end),
        content_range: TextRange::new(source_start + content_start, source_start + closing_start),
        source: &source[opening_start..closing_end],
        content: &source[content_start..closing_start],
        kind: MarkdownInline::Code {
            markers: MarkdownMarkerRanges {
                opening: TextRange::new(source_start + opening_start, source_start + content_start),
                closing: TextRange::new(source_start + closing_start, source_start + closing_end),
            },
        },
    });
}

fn push_link_inline<'a>(
    inlines: &mut Vec<ProjectedInline<'a>>,
    source: &'a str,
    source_start: usize,
    span: LinkSpan,
) {
    let destination_start = span.label_end + 2;
    let closing_end = span.closing_start + 1;

    inlines.push(ProjectedInline {
        source_range: TextRange::new(
            source_start + span.opening_start,
            source_start + closing_end,
        ),
        content_range: TextRange::new(
            source_start + span.opening_start + 1,
            source_start + span.label_end,
        ),
        source: &source[span.opening_start..closing_end],
        content: &source[span.opening_start + 1..span.label_end],
        kind: MarkdownInline::Link {
            markers: MarkdownLinkRanges {
                opening: TextRange::new(
                    source_start + span.opening_start,
                    source_start + span.opening_start + 1,
                ),
                separator: TextRange::new(
                    source_start + span.label_end,
                    source_start + destination_start,
                ),
                destination: TextRange::new(
                    source_start + destination_start,
                    source_start + span.closing_start,
                ),
                closing: TextRange::new(
                    source_start + span.closing_start,
                    source_start + closing_end,
                ),
            },
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanji_core::{EditorCommand, Selection};

    #[test]
    fn projects_lines_with_source_ranges_and_kinds() {
        let document = Document::new("# Hanji\n\n> Quote\nNotes");
        let projection = project_document(&document);
        let lines = projection.lines();

        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].range, TextRange::new(0, 7));
        assert_eq!(lines[0].source, "# Hanji");
        assert_eq!(lines[0].kind, MarkdownLine::Heading { level: 1 });
        assert_eq!(lines[0].marker_range, Some(TextRange::new(0, 1)));
        assert_eq!(lines[0].inlines.len(), 1);
        assert_eq!(lines[0].inlines[0].source, "Hanji");
        assert_eq!(lines[1].range, TextRange::new(8, 8));
        assert_eq!(lines[1].source, "");
        assert_eq!(lines[1].kind, MarkdownLine::Blank);
        assert_eq!(lines[1].inlines.len(), 0);
        assert_eq!(lines[2].range, TextRange::new(9, 16));
        assert_eq!(lines[2].source, "> Quote");
        assert_eq!(lines[2].kind, MarkdownLine::Blockquote);
        assert_eq!(lines[2].marker_range, Some(TextRange::new(9, 11)));
        assert_eq!(lines[2].inlines.len(), 1);
        assert_eq!(lines[2].inlines[0].source, "Quote");
        assert_eq!(lines[3].range, TextRange::new(17, 22));
        assert_eq!(lines[3].source, "Notes");
        assert_eq!(lines[3].kind, MarkdownLine::Paragraph);
        assert_eq!(lines[3].inlines.len(), 1);
    }

    #[test]
    fn projects_horizontal_rule_lines() {
        let document = Document::new("Before\n---\nAfter");
        let projection = project_document(&document);
        let lines = projection.lines();

        assert_eq!(lines[1].range, TextRange::new(7, 10));
        assert_eq!(lines[1].source, "---");
        assert_eq!(lines[1].kind, MarkdownLine::HorizontalRule);
        assert_eq!(lines[1].marker_range, Some(TextRange::new(7, 10)));
        assert!(lines[1].inlines.is_empty());
    }

    #[test]
    fn hides_horizontal_rule_source_in_preview() {
        let document = Document::new("  ---  ");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "");
        assert_eq!(line.visible_len(), 0);
        assert_eq!(
            line.source_visible_segments(),
            vec![
                ProjectedSegment {
                    range: TextRange::new(0, 2),
                    source: "  ",
                    kind: ProjectedSegmentKind::Text,
                },
                ProjectedSegment {
                    range: TextRange::new(2, 5),
                    source: "---",
                    kind: ProjectedSegmentKind::HorizontalRuleMarker,
                },
                ProjectedSegment {
                    range: TextRange::new(5, 7),
                    source: "  ",
                    kind: ProjectedSegmentKind::Text,
                },
            ]
        );
        assert_eq!(line.visible_to_source_caret_offset(0), 7);
        assert_eq!(line.source_to_visible_offset(2), 0);
    }

    #[test]
    fn caret_inside_horizontal_rule_reveals_source() {
        let document = Document::new("---");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret(1))),
            "---"
        );
        assert_eq!(
            line.visible_segments_revealing_source_in(Some(TextRange::caret(1))),
            vec![ProjectedVisibleSegment {
                visible_range: TextRange::new(0, 3),
                source_range: TextRange::new(0, 3),
                source_outer_range: TextRange::new(0, 3),
                source: "---",
                kind: ProjectedSegmentKind::HorizontalRuleMarker,
            }]
        );
    }

    #[test]
    fn hides_heading_marker_in_preview() {
        let document = Document::new("# Hanji");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "Hanji");
        assert_eq!(line.visible_len(), "Hanji".len());
        assert_eq!(
            line.source_visible_segments()[0],
            ProjectedSegment {
                range: TextRange::new(0, 1),
                source: "#",
                kind: ProjectedSegmentKind::HeadingMarker,
            }
        );
        assert_eq!(
            line.visible_segments(),
            vec![ProjectedVisibleSegment {
                visible_range: TextRange::new(0, 5),
                source_range: TextRange::new(2, 7),
                source_outer_range: TextRange::new(0, 7),
                source: "Hanji",
                kind: ProjectedSegmentKind::Text,
            }]
        );
        assert_eq!(line.visible_to_source_caret_offset(0), 2);
        assert_eq!(line.source_to_visible_offset(0), 0);
        assert_eq!(line.source_to_visible_offset(2), 0);
    }

    #[test]
    fn caret_inside_heading_marker_keeps_source_visible() {
        let document = Document::new("# Hanji");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret(1))),
            "# Hanji"
        );
        assert_eq!(
            line.visible_segments_revealing_source_in(Some(TextRange::caret(1)))[0],
            ProjectedVisibleSegment {
                visible_range: TextRange::new(0, 1),
                source_range: TextRange::new(0, 1),
                source_outer_range: TextRange::new(0, 1),
                source: "#",
                kind: ProjectedSegmentKind::HeadingMarker,
            }
        );
    }

    #[test]
    fn caret_inside_heading_content_keeps_source_visible() {
        let document = Document::new("# Hanji");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret(4))),
            "# Hanji"
        );
    }

    #[test]
    fn selection_over_heading_marker_keeps_source_visible() {
        let document = Document::new("# Hanji");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::new(1, 4))),
            "# Hanji"
        );
    }

    #[test]
    fn hides_indented_heading_marker_in_preview() {
        let document = Document::new("   ## Hanji");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "Hanji");
        assert_eq!(line.marker_range, Some(TextRange::new(3, 5)));
        assert_eq!(line.visible_to_source_caret_offset(0), 6);
        assert_eq!(line.source_to_visible_offset(0), 0);
        assert_eq!(line.source_to_visible_offset(6), 0);
        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret(8))),
            "   ## Hanji"
        );
    }

    #[test]
    fn keeps_pending_heading_marker_source_visible_until_padding() {
        let document = Document::new("#");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.kind, MarkdownLine::Paragraph);
        assert_eq!(line.marker_range, None);
        assert_eq!(line.visible_text(), "#");
    }

    #[test]
    fn hides_empty_heading_source_until_caret_enters_line() {
        let document = Document::new("# ");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.kind, MarkdownLine::Heading { level: 1 });
        assert_eq!(line.marker_range, Some(TextRange::new(0, 1)));
        assert_eq!(line.visible_text(), "");
        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret(2))),
            "# "
        );
        assert_eq!(
            line.visible_segments_revealing_source_in(Some(TextRange::caret(2))),
            vec![
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(0, 1),
                    source_range: TextRange::new(0, 1),
                    source_outer_range: TextRange::new(0, 1),
                    source: "#",
                    kind: ProjectedSegmentKind::HeadingMarker,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(1, 2),
                    source_range: TextRange::new(1, 2),
                    source_outer_range: TextRange::new(1, 2),
                    source: " ",
                    kind: ProjectedSegmentKind::Text,
                },
            ]
        );
    }

    #[test]
    fn hides_blockquote_markers_from_visible_text() {
        let document = Document::new("> Quote");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "Quote");
        assert_eq!(line.visible_len(), "Quote".len());
        assert_eq!(
            line.source_visible_segments()[0],
            ProjectedSegment {
                range: TextRange::new(0, 2),
                source: "> ",
                kind: ProjectedSegmentKind::BlockquoteMarker,
            }
        );
        assert_eq!(
            line.visible_segments(),
            vec![ProjectedVisibleSegment {
                visible_range: TextRange::new(0, 5),
                source_range: TextRange::new(2, 7),
                source_outer_range: TextRange::new(0, 7),
                source: "Quote",
                kind: ProjectedSegmentKind::Text,
            }]
        );
        assert_eq!(line.visible_to_source_caret_offset(0), 2);
        assert_eq!(line.source_to_visible_offset(0), 0);
        assert_eq!(line.source_to_visible_offset(2), 0);
    }

    #[test]
    fn hides_indented_blockquote_markers_from_visible_text() {
        let document = Document::new("   > Quote");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "Quote");
        assert_eq!(line.marker_range, Some(TextRange::new(0, 5)));
        assert_eq!(line.visible_to_source_caret_offset(0), 5);
    }

    #[test]
    fn hides_unordered_list_markers_from_visible_text() {
        let document = Document::new("- Item");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(
            line.kind,
            MarkdownLine::ListItem {
                marker: crate::MarkdownListMarker::Unordered { marker: '-' },
                task: None,
            }
        );
        assert_eq!(line.visible_text(), "Item");
        assert_eq!(line.marker_range, Some(TextRange::new(0, 2)));
        assert_eq!(
            line.source_visible_segments()[0],
            ProjectedSegment {
                range: TextRange::new(0, 2),
                source: "- ",
                kind: ProjectedSegmentKind::ListMarker,
            }
        );
        assert_eq!(
            line.visible_segments(),
            vec![ProjectedVisibleSegment {
                visible_range: TextRange::new(0, 4),
                source_range: TextRange::new(2, 6),
                source_outer_range: TextRange::new(0, 6),
                source: "Item",
                kind: ProjectedSegmentKind::Text,
            }]
        );
        assert_eq!(line.visible_to_source_caret_offset(0), 2);
    }

    #[test]
    fn caret_inside_list_marker_reveals_marker_source() {
        let document = Document::new("- Item");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret(1))),
            "- Item"
        );
        assert_eq!(
            line.visible_segments_revealing_source_in(Some(TextRange::caret(1))),
            vec![
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(0, 2),
                    source_range: TextRange::new(0, 2),
                    source_outer_range: TextRange::new(0, 2),
                    source: "- ",
                    kind: ProjectedSegmentKind::ListMarker,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(2, 6),
                    source_range: TextRange::new(2, 6),
                    source_outer_range: TextRange::new(2, 6),
                    source: "Item",
                    kind: ProjectedSegmentKind::Text,
                },
            ]
        );
        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret(2))),
            "Item"
        );
    }

    #[test]
    fn selection_over_list_marker_reveals_marker_source() {
        let document = Document::new("- Item");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::new(1, 4))),
            "- Item"
        );
    }

    #[test]
    fn hides_ordered_list_markers_from_visible_text() {
        let document = Document::new("12. Item");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(
            line.kind,
            MarkdownLine::ListItem {
                marker: crate::MarkdownListMarker::Ordered {
                    number: 12,
                    delimiter: crate::OrderedListDelimiter::Dot,
                },
                task: None,
            }
        );
        assert_eq!(line.visible_text(), "Item");
        assert_eq!(line.marker_range, Some(TextRange::new(0, 4)));
        assert_eq!(line.visible_to_source_caret_offset(0), 4);
    }

    #[test]
    fn hides_task_list_markers_from_visible_text() {
        let document = Document::new("- [x] Done");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(
            line.kind,
            MarkdownLine::ListItem {
                marker: crate::MarkdownListMarker::Unordered { marker: '-' },
                task: Some(crate::MarkdownTaskState::Checked),
            }
        );
        assert_eq!(line.visible_text(), "Done");
        assert_eq!(line.marker_range, Some(TextRange::new(0, 6)));
        assert_eq!(
            line.source_visible_segments()[0],
            ProjectedSegment {
                range: TextRange::new(0, 6),
                source: "- [x] ",
                kind: ProjectedSegmentKind::ListMarker,
            }
        );
        assert_eq!(
            line.visible_segments(),
            vec![ProjectedVisibleSegment {
                visible_range: TextRange::new(0, 4),
                source_range: TextRange::new(6, 10),
                source_outer_range: TextRange::new(0, 10),
                source: "Done",
                kind: ProjectedSegmentKind::Text,
            }]
        );
        assert_eq!(line.visible_to_source_caret_offset(0), 6);
    }

    #[test]
    fn keeps_pending_task_marker_source_visible_until_padding() {
        let document = Document::new("- [ ]");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.kind, MarkdownLine::Paragraph);
        assert_eq!(line.marker_range, None);
        assert_eq!(line.visible_text(), "- [ ]");
    }

    #[test]
    fn projects_closed_fenced_code_blocks_without_inline_markdown() {
        let document = Document::new("Before\n```rust\n**literal**\n```\nAfter");
        let projection = project_document(&document);
        let lines = projection.lines();

        assert_eq!(lines[0].kind, MarkdownLine::Paragraph);
        assert_eq!(
            lines[1].kind,
            MarkdownLine::CodeBlock {
                role: MarkdownCodeBlockLine::OpeningFence,
            }
        );
        assert_eq!(
            lines[2].kind,
            MarkdownLine::CodeBlock {
                role: MarkdownCodeBlockLine::Content,
            }
        );
        assert_eq!(
            lines[3].kind,
            MarkdownLine::CodeBlock {
                role: MarkdownCodeBlockLine::ClosingFence,
            }
        );
        assert_eq!(lines[4].kind, MarkdownLine::Paragraph);
        assert_eq!(lines[1].visible_text(), "");
        assert_eq!(lines[2].visible_text(), "**literal**");
        assert_eq!(lines[3].visible_text(), "");
        assert_eq!(
            lines[2].visible_segments(),
            vec![ProjectedVisibleSegment {
                visible_range: TextRange::new(0, "**literal**".len()),
                source_range: lines[2].range,
                source_outer_range: lines[2].range,
                source: "**literal**",
                kind: ProjectedSegmentKind::CodeBlockContent,
            }]
        );
        assert!(lines[2].inlines.is_empty());
    }

    #[test]
    fn projects_closed_tilde_fenced_code_blocks_without_inline_markdown() {
        let document = Document::new("Before\n~~~rust\n**literal**\n~~~\nAfter");
        let projection = project_document(&document);
        let lines = projection.lines();

        assert_eq!(lines[0].kind, MarkdownLine::Paragraph);
        assert_eq!(
            lines[1].kind,
            MarkdownLine::CodeBlock {
                role: MarkdownCodeBlockLine::OpeningFence,
            }
        );
        assert_eq!(
            lines[2].kind,
            MarkdownLine::CodeBlock {
                role: MarkdownCodeBlockLine::Content,
            }
        );
        assert_eq!(
            lines[3].kind,
            MarkdownLine::CodeBlock {
                role: MarkdownCodeBlockLine::ClosingFence,
            }
        );
        assert_eq!(lines[4].kind, MarkdownLine::Paragraph);
        assert_eq!(lines[1].visible_text(), "");
        assert_eq!(lines[2].visible_text(), "**literal**");
        assert_eq!(lines[3].visible_text(), "");
        assert!(lines[2].inlines.is_empty());
    }

    #[test]
    fn reveals_fenced_code_fences_when_caret_is_inside_block() {
        let source = "```rust\nlet value = `literal`;\n```";
        let document = Document::new(source);
        let projection = project_document(&document);
        let lines = projection.lines();
        let caret = TextRange::caret(source.find("value").unwrap());

        assert_eq!(
            lines[0].visible_text_revealing_source_in(Some(caret)),
            "```rust"
        );
        assert_eq!(
            lines[2].visible_text_revealing_source_in(Some(caret)),
            "```"
        );
        assert_eq!(
            lines[0].visible_segments_revealing_source_in(Some(caret))[0],
            ProjectedVisibleSegment {
                visible_range: TextRange::new(0, 3),
                source_range: TextRange::new(0, 3),
                source_outer_range: TextRange::new(0, 3),
                source: "```",
                kind: ProjectedSegmentKind::CodeBlockFence,
            }
        );
    }

    #[test]
    fn reveals_tilde_fenced_code_fences_when_caret_is_inside_block() {
        let source = "~~~rust\nlet value = `literal`;\n~~~";
        let document = Document::new(source);
        let projection = project_document(&document);
        let lines = projection.lines();
        let caret = TextRange::caret(source.find("value").unwrap());

        assert_eq!(
            lines[0].visible_text_revealing_source_in(Some(caret)),
            "~~~rust"
        );
        assert_eq!(
            lines[2].visible_text_revealing_source_in(Some(caret)),
            "~~~"
        );
        assert_eq!(
            lines[0].visible_segments_revealing_source_in(Some(caret))[0],
            ProjectedVisibleSegment {
                visible_range: TextRange::new(0, 3),
                source_range: TextRange::new(0, 3),
                source_outer_range: TextRange::new(0, 3),
                source: "~~~",
                kind: ProjectedSegmentKind::CodeBlockFence,
            }
        );
    }

    #[test]
    fn keeps_unclosed_fence_as_plain_source() {
        let document = Document::new("```rust\nlet value = 1;");
        let projection = project_document(&document);
        let lines = projection.lines();

        assert_eq!(lines[0].kind, MarkdownLine::Paragraph);
        assert_eq!(lines[0].visible_text(), "```rust");
        assert_eq!(lines[1].kind, MarkdownLine::Paragraph);
    }

    #[test]
    fn code_fence_closing_marker_must_match_opening_marker() {
        let tilde_opening = Document::new("~~~\ncode\n```");
        let tilde_projection = project_document(&tilde_opening);
        let tilde_lines = tilde_projection.lines();

        assert_eq!(tilde_lines[0].kind, MarkdownLine::Paragraph);
        assert_eq!(tilde_lines[1].kind, MarkdownLine::Paragraph);
        assert_eq!(tilde_lines[2].kind, MarkdownLine::Paragraph);

        let backtick_opening = Document::new("```\ncode\n~~~");
        let backtick_projection = project_document(&backtick_opening);
        let backtick_lines = backtick_projection.lines();

        assert_eq!(backtick_lines[0].kind, MarkdownLine::Paragraph);
        assert_eq!(backtick_lines[1].kind, MarkdownLine::Paragraph);
        assert_eq!(backtick_lines[2].kind, MarkdownLine::Paragraph);
    }

    #[test]
    fn closing_fence_requires_only_whitespace_after_marker() {
        let document = Document::new("```\ncode\n```tail");
        let projection = project_document(&document);
        let lines = projection.lines();

        assert_eq!(lines[0].kind, MarkdownLine::Paragraph);
        assert_eq!(lines[1].kind, MarkdownLine::Paragraph);
        assert_eq!(lines[2].kind, MarkdownLine::Paragraph);
    }

    #[test]
    fn caret_inside_task_list_marker_reveals_marker_source() {
        let document = Document::new("- [x] Done");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret(5))),
            "- [x] Done"
        );
    }

    #[test]
    fn projects_strong_inline_spans_with_source_ranges() {
        let document = Document::new("This is **bold** text");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 3);
        assert_eq!(
            inlines[0],
            ProjectedInline {
                source_range: TextRange::new(0, 8),
                content_range: TextRange::new(0, 8),
                source: "This is ",
                content: "This is ",
                kind: MarkdownInline::Text,
            }
        );
        assert_eq!(
            inlines[1],
            ProjectedInline {
                source_range: TextRange::new(8, 16),
                content_range: TextRange::new(10, 14),
                source: "**bold**",
                content: "bold",
                kind: MarkdownInline::Strong {
                    markers: MarkdownMarkerRanges {
                        opening: TextRange::new(8, 10),
                        closing: TextRange::new(14, 16),
                    },
                },
            }
        );
        assert_eq!(
            inlines[2],
            ProjectedInline {
                source_range: TextRange::new(16, 21),
                content_range: TextRange::new(16, 21),
                source: " text",
                content: " text",
                kind: MarkdownInline::Text,
            }
        );
    }

    #[test]
    fn exposes_source_visible_segments_for_strong_spans() {
        let document = Document::new("This is **bold** text");
        let projection = project_document(&document);
        let segments = projection.lines()[0].source_visible_segments();

        assert_eq!(
            segments,
            vec![
                ProjectedSegment {
                    range: TextRange::new(0, 8),
                    source: "This is ",
                    kind: ProjectedSegmentKind::Text,
                },
                ProjectedSegment {
                    range: TextRange::new(8, 10),
                    source: "**",
                    kind: ProjectedSegmentKind::StrongMarker,
                },
                ProjectedSegment {
                    range: TextRange::new(10, 14),
                    source: "bold",
                    kind: ProjectedSegmentKind::StrongContent,
                },
                ProjectedSegment {
                    range: TextRange::new(14, 16),
                    source: "**",
                    kind: ProjectedSegmentKind::StrongMarker,
                },
                ProjectedSegment {
                    range: TextRange::new(16, 21),
                    source: " text",
                    kind: ProjectedSegmentKind::Text,
                },
            ]
        );
    }

    #[test]
    fn projects_emphasis_inline_spans_with_source_ranges() {
        let document = Document::new("This is *soft* text");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 3);
        assert_eq!(
            inlines[1],
            ProjectedInline {
                source_range: TextRange::new(8, 14),
                content_range: TextRange::new(9, 13),
                source: "*soft*",
                content: "soft",
                kind: MarkdownInline::Emphasis {
                    markers: MarkdownMarkerRanges {
                        opening: TextRange::new(8, 9),
                        closing: TextRange::new(13, 14),
                    },
                },
            }
        );
    }

    #[test]
    fn exposes_source_visible_segments_for_emphasis_spans() {
        let document = Document::new("This is *soft* text");
        let projection = project_document(&document);
        let segments = projection.lines()[0].source_visible_segments();

        assert_eq!(
            segments,
            vec![
                ProjectedSegment {
                    range: TextRange::new(0, 8),
                    source: "This is ",
                    kind: ProjectedSegmentKind::Text,
                },
                ProjectedSegment {
                    range: TextRange::new(8, 9),
                    source: "*",
                    kind: ProjectedSegmentKind::EmphasisMarker,
                },
                ProjectedSegment {
                    range: TextRange::new(9, 13),
                    source: "soft",
                    kind: ProjectedSegmentKind::EmphasisContent,
                },
                ProjectedSegment {
                    range: TextRange::new(13, 14),
                    source: "*",
                    kind: ProjectedSegmentKind::EmphasisMarker,
                },
                ProjectedSegment {
                    range: TextRange::new(14, 19),
                    source: " text",
                    kind: ProjectedSegmentKind::Text,
                },
            ]
        );
    }

    #[test]
    fn projects_visible_text_without_markers() {
        let document = Document::new("This is *soft*, **bold**, and `code`");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "This is soft, bold, and code");
        assert_eq!(line.visible_len(), "This is soft, bold, and code".len());
    }

    #[test]
    fn exposes_visible_segments_with_source_mapping() {
        let document = Document::new("This is **bold** text");
        let projection = project_document(&document);
        let segments = projection.lines()[0].visible_segments();

        assert_eq!(
            segments,
            vec![
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(0, 8),
                    source_range: TextRange::new(0, 8),
                    source_outer_range: TextRange::new(0, 8),
                    source: "This is ",
                    kind: ProjectedSegmentKind::Text,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(8, 12),
                    source_range: TextRange::new(10, 14),
                    source_outer_range: TextRange::new(8, 16),
                    source: "bold",
                    kind: ProjectedSegmentKind::StrongContent,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(12, 17),
                    source_range: TextRange::new(16, 21),
                    source_outer_range: TextRange::new(16, 21),
                    source: " text",
                    kind: ProjectedSegmentKind::Text,
                },
            ]
        );
    }

    #[test]
    fn reveals_source_markers_for_inline_at_caret() {
        let document = Document::new("This is **bold** and `code`");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let reveal_range = TextRange::caret("This is **bo".len());

        assert_eq!(
            line.visible_text_revealing_source_in(Some(reveal_range)),
            "This is **bold** and code"
        );
        assert_eq!(
            line.visible_segments_revealing_source_in(Some(reveal_range)),
            vec![
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(0, 8),
                    source_range: TextRange::new(0, 8),
                    source_outer_range: TextRange::new(0, 8),
                    source: "This is ",
                    kind: ProjectedSegmentKind::Text,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(8, 10),
                    source_range: TextRange::new(8, 10),
                    source_outer_range: TextRange::new(8, 10),
                    source: "**",
                    kind: ProjectedSegmentKind::StrongMarker,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(10, 14),
                    source_range: TextRange::new(10, 14),
                    source_outer_range: TextRange::new(10, 14),
                    source: "bold",
                    kind: ProjectedSegmentKind::StrongContent,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(14, 16),
                    source_range: TextRange::new(14, 16),
                    source_outer_range: TextRange::new(14, 16),
                    source: "**",
                    kind: ProjectedSegmentKind::StrongMarker,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(16, 21),
                    source_range: TextRange::new(16, 21),
                    source_outer_range: TextRange::new(16, 21),
                    source: " and ",
                    kind: ProjectedSegmentKind::Text,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(21, 25),
                    source_range: TextRange::new(22, 26),
                    source_outer_range: TextRange::new(21, 27),
                    source: "code",
                    kind: ProjectedSegmentKind::CodeContent,
                },
            ]
        );
    }

    #[test]
    fn reveals_source_markers_for_selected_inline() {
        let document = Document::new("This is **bold** and `code`");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let reveal_range = TextRange::new("This is **b".len(), "This is **bol".len());

        assert_eq!(
            line.visible_text_revealing_source_in(Some(reveal_range)),
            "This is **bold** and code"
        );
    }

    #[test]
    fn keeps_other_inline_markers_hidden_when_revealing_current_inline() {
        let document = Document::new("This is **bold** and `code`");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let reveal_range = TextRange::caret("This is **bold** and `co".len());

        assert_eq!(
            line.visible_text_revealing_source_in(Some(reveal_range)),
            "This is bold and `code`"
        );
    }

    #[test]
    fn caret_on_inline_source_boundary_reveals_markers() {
        let document = Document::new("This is **bold** and `code`");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        for caret in ["This is ".len(), "This is **bold**".len()] {
            assert_eq!(
                line.visible_text_revealing_source_in(Some(TextRange::caret(caret))),
                "This is **bold** and code"
            );
        }

        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret("This is".len()))),
            "This is bold and code"
        );
        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret(
                "This is **bold** ".len()
            ))),
            "This is bold and code"
        );
    }

    #[test]
    fn selection_over_hidden_marker_reveals_inline_source() {
        let document = Document::new("This is **bold** and `code`");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        for reveal_range in [
            TextRange::new("This is ".len(), "This is **".len()),
            TextRange::new("This is **bold".len(), "This is **bold**".len()),
        ] {
            assert_eq!(
                line.visible_text_revealing_source_in(Some(reveal_range)),
                "This is **bold** and code"
            );
        }
    }

    #[test]
    fn selection_spanning_multiple_inlines_reveals_each_source() {
        let document = Document::new("This is **bold** and `code`");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let reveal_range = TextRange::new("This is **bo".len(), "This is **bold** and `co".len());

        assert_eq!(
            line.visible_text_revealing_source_in(Some(reveal_range)),
            "This is **bold** and `code`"
        );
    }

    #[test]
    fn maps_source_offsets_inside_hidden_markers_to_visible_boundaries() {
        let document = Document::new("This is **bold** text");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.source_to_visible_offset(0), 0);
        assert_eq!(line.source_to_visible_offset(8), 8);
        assert_eq!(line.source_to_visible_offset(9), 8);
        assert_eq!(line.source_to_visible_offset(10), 8);
        assert_eq!(line.source_to_visible_offset(12), 10);
        assert_eq!(line.source_to_visible_offset(14), 12);
        assert_eq!(line.source_to_visible_offset(15), 12);
        assert_eq!(line.source_to_visible_offset(16), 12);
        assert_eq!(line.source_to_visible_offset(21), 17);
    }

    #[test]
    fn maps_visible_offsets_back_to_source_with_boundary_affinity() {
        let document = Document::new("This is **bold** text");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(
            line.visible_to_source_offset(8, VisibleOffsetAffinity::Before),
            8
        );
        assert_eq!(
            line.visible_to_source_offset(8, VisibleOffsetAffinity::After),
            10
        );
        assert_eq!(
            line.visible_to_source_offset(10, VisibleOffsetAffinity::Before),
            12
        );
        assert_eq!(
            line.visible_to_source_offset(12, VisibleOffsetAffinity::Before),
            14
        );
        assert_eq!(
            line.visible_to_source_offset(12, VisibleOffsetAffinity::After),
            16
        );
        assert_eq!(
            line.visible_to_source_offset(17, VisibleOffsetAffinity::After),
            21
        );
    }

    #[test]
    fn maps_hidden_inline_boundaries_to_editable_marker_edges() {
        let document = Document::new("This is **bold** text");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "This is bold text");
        assert_eq!(
            line.visible_to_source_caret_offset("This is".len()),
            "This is".len()
        );
        assert_eq!(
            line.visible_to_source_caret_offset("This is ".len()),
            "This is **".len()
        );
        assert_eq!(
            line.visible_to_source_caret_offset("This is bo".len()),
            "This is **bo".len()
        );
        assert_eq!(
            line.visible_to_source_caret_offset("This is bold".len()),
            "This is **bold".len()
        );
        assert_eq!(
            line.visible_to_source_caret_offset("This is bold ".len()),
            "This is **bold** ".len()
        );
    }

    #[test]
    fn projects_link_inline_spans_with_source_ranges() {
        let document = Document::new("Read [Hanji](https://hanji.local) now");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 3);
        assert_eq!(inlines[0].source, "Read ");
        assert_eq!(
            inlines[1],
            ProjectedInline {
                source_range: TextRange::new(5, 33),
                content_range: TextRange::new(6, 11),
                source: "[Hanji](https://hanji.local)",
                content: "Hanji",
                kind: MarkdownInline::Link {
                    markers: MarkdownLinkRanges {
                        opening: TextRange::new(5, 6),
                        separator: TextRange::new(11, 13),
                        destination: TextRange::new(13, 32),
                        closing: TextRange::new(32, 33),
                    },
                },
            }
        );
        assert_eq!(inlines[2].source, " now");
    }

    #[test]
    fn exposes_source_visible_segments_for_link_spans() {
        let document = Document::new("Read [Hanji](https://hanji.local) now");
        let projection = project_document(&document);
        let segments = projection.lines()[0].source_visible_segments();

        assert_eq!(
            segments,
            vec![
                ProjectedSegment {
                    range: TextRange::new(0, 5),
                    source: "Read ",
                    kind: ProjectedSegmentKind::Text,
                },
                ProjectedSegment {
                    range: TextRange::new(5, 6),
                    source: "[",
                    kind: ProjectedSegmentKind::LinkMarker,
                },
                ProjectedSegment {
                    range: TextRange::new(6, 11),
                    source: "Hanji",
                    kind: ProjectedSegmentKind::LinkText,
                },
                ProjectedSegment {
                    range: TextRange::new(11, 13),
                    source: "](",
                    kind: ProjectedSegmentKind::LinkMarker,
                },
                ProjectedSegment {
                    range: TextRange::new(13, 32),
                    source: "https://hanji.local",
                    kind: ProjectedSegmentKind::LinkDestination,
                },
                ProjectedSegment {
                    range: TextRange::new(32, 33),
                    source: ")",
                    kind: ProjectedSegmentKind::LinkMarker,
                },
                ProjectedSegment {
                    range: TextRange::new(33, 37),
                    source: " now",
                    kind: ProjectedSegmentKind::Text,
                },
            ]
        );
    }

    #[test]
    fn hides_link_markers_and_destination_in_preview() {
        let document = Document::new("Read [Hanji](https://hanji.local) now");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "Read Hanji now");
        assert_eq!(
            line.visible_segments(),
            vec![
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(0, 5),
                    source_range: TextRange::new(0, 5),
                    source_outer_range: TextRange::new(0, 5),
                    source: "Read ",
                    kind: ProjectedSegmentKind::Text,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(5, 10),
                    source_range: TextRange::new(6, 11),
                    source_outer_range: TextRange::new(5, 33),
                    source: "Hanji",
                    kind: ProjectedSegmentKind::LinkText,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(10, 14),
                    source_range: TextRange::new(33, 37),
                    source_outer_range: TextRange::new(33, 37),
                    source: " now",
                    kind: ProjectedSegmentKind::Text,
                },
            ]
        );
    }

    #[test]
    fn caret_inside_link_reveals_link_source() {
        let document = Document::new("Read [Hanji](https://hanji.local) now");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        for caret in [
            "Read [Ha".len(),
            "Read [Hanji](https".len(),
            "Read [Hanji](https://hanji.local)".len(),
        ] {
            assert_eq!(
                line.visible_text_revealing_source_in(Some(TextRange::caret(caret))),
                "Read [Hanji](https://hanji.local) now"
            );
        }

        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret("Read ".len()))),
            "Read [Hanji](https://hanji.local) now"
        );
        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret("Read".len()))),
            "Read Hanji now"
        );
    }

    #[test]
    fn maps_hidden_link_boundaries_to_editable_marker_edges() {
        let document = Document::new("Read [Hanji](https://hanji.local) now");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "Read Hanji now");
        assert_eq!(line.visible_to_source_caret_offset("Read ".len()), 6);
        assert_eq!(line.visible_to_source_caret_offset("Read Ha".len()), 8);
        assert_eq!(line.visible_to_source_caret_offset("Read Hanji".len()), 11);
        assert_eq!(
            line.visible_to_source_caret_offset("Read Hanji ".len()),
            "Read [Hanji](https://hanji.local) ".len()
        );
    }

    #[test]
    fn escaped_markdown_markers_remain_text() {
        let document = Document::new("Use \\*literal\\* and \\[Hanji](url) plus \\\\ slash");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(
            line.visible_text(),
            "Use *literal* and [Hanji](url) plus \\ slash"
        );
        assert!(line.inlines.iter().all(|inline| {
            !matches!(
                inline.kind,
                MarkdownInline::Strong { .. }
                    | MarkdownInline::Emphasis { .. }
                    | MarkdownInline::Code { .. }
                    | MarkdownInline::Link { .. }
            )
        }));
    }

    #[test]
    fn escaped_markers_reveal_backslash_at_caret() {
        let document = Document::new("Use \\*literal");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "Use *literal");
        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret("Use \\".len()))),
            "Use \\*literal"
        );
        assert_eq!(
            line.visible_to_source_caret_offset("Use ".len()),
            "Use \\".len()
        );
    }

    #[test]
    fn escaped_markers_reveal_when_caret_enters_same_token() {
        let document = Document::new("Use \\*literal\\* here");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "Use *literal* here");
        assert_eq!(
            line.visible_text_revealing_source_in(Some(TextRange::caret("Use \\*lit".len()))),
            "Use \\*literal\\* here"
        );
    }

    #[test]
    fn escaped_code_marker_does_not_start_code_span() {
        let document = Document::new("Use \\`code\\` here");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "Use `code` here");
        assert!(
            line.inlines
                .iter()
                .all(|inline| !matches!(inline.kind, MarkdownInline::Code { .. }))
        );
    }

    #[test]
    fn malformed_links_remain_text() {
        let document = Document::new("Read [Hanji](https://hanji.local now");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "Read [Hanji](https://hanji.local now");
        assert!(
            line.inlines
                .iter()
                .all(|inline| !matches!(inline.kind, MarkdownInline::Link { .. }))
        );
    }

    #[test]
    fn delete_backward_at_hidden_opening_boundary_edits_one_marker() {
        let mut document = Document::new("This is **bold** and `code`");
        let caret = {
            let projection = project_document(&document);
            projection.lines()[0].visible_to_source_caret_offset("This is ".len())
        };

        assert_eq!(caret, "This is **".len());
        document.set_selection(Selection::caret(caret)).unwrap();
        document.execute(EditorCommand::DeleteBackward).unwrap();

        assert_eq!(document.text(), "This is *bold** and `code`");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "This is *bold** and code");
        assert!(line.inlines.iter().any(|inline| {
            inline.source == "`code`" && matches!(inline.kind, MarkdownInline::Code { .. })
        }));
    }

    #[test]
    fn delete_forward_at_hidden_closing_boundary_edits_one_marker() {
        let mut document = Document::new("This is **bold** and `code`");
        let caret = {
            let projection = project_document(&document);
            projection.lines()[0].visible_to_source_caret_offset("This is bold".len())
        };

        assert_eq!(caret, "This is **bold".len());
        document.set_selection(Selection::caret(caret)).unwrap();
        document.execute(EditorCommand::DeleteForward).unwrap();

        assert_eq!(document.text(), "This is **bold* and `code`");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "This is **bold* and code");
        assert!(line.inlines.iter().any(|inline| {
            inline.source == "`code`" && matches!(inline.kind, MarkdownInline::Code { .. })
        }));
    }

    #[test]
    fn maps_multibyte_visible_offsets_to_source_offsets() {
        let document = Document::new("한지 **노트**");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "한지 노트");
        assert_eq!(
            line.source_to_visible_offset("한지 **노".len()),
            "한지 노".len()
        );
        assert_eq!(
            line.visible_to_source_offset("한지 노".len(), VisibleOffsetAffinity::Before),
            "한지 **노".len()
        );
    }

    #[test]
    fn leaves_unmatched_strong_markers_as_text() {
        let document = Document::new("This is **not closed");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 1);
        assert_eq!(inlines[0].source, "This is **not closed");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
    }

    #[test]
    fn projects_single_asterisk_emphasis_without_strong_runs() {
        let document = Document::new("This is *soft*");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 2);
        assert_eq!(inlines[0].source, "This is ");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
        assert_eq!(inlines[1].source, "*soft*");
        assert!(matches!(inlines[1].kind, MarkdownInline::Emphasis { .. }));
    }

    #[test]
    fn does_not_project_strong_from_longer_asterisk_runs() {
        let document = Document::new("This is ***not stable***");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 1);
        assert_eq!(inlines[0].source, "This is ***not stable***");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
    }

    #[test]
    fn projects_emphasis_before_strong_without_style_leakage() {
        let document = Document::new("This is *abc* before **bold**");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 4);
        assert_eq!(inlines[0].source, "This is ");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
        assert_eq!(inlines[1].source, "*abc*");
        assert!(matches!(inlines[1].kind, MarkdownInline::Emphasis { .. }));
        assert_eq!(inlines[2].source, " before ");
        assert_eq!(inlines[2].kind, MarkdownInline::Text);
        assert_eq!(inlines[3].source, "**bold**");
        assert!(matches!(inlines[3].kind, MarkdownInline::Strong { .. }));
    }

    #[test]
    fn uses_later_opening_when_an_earlier_strong_marker_stays_unclosed() {
        let document = Document::new("This is **broken **strong**");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 2);
        assert_eq!(inlines[0].source, "This is **broken ");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
        assert_eq!(inlines[1].source, "**strong**");
        assert!(matches!(inlines[1].kind, MarkdownInline::Strong { .. }));
    }

    #[test]
    fn keeps_scanning_after_unmatched_strong_marker() {
        let document = Document::new("This is **not closed with `code`");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 2);
        assert_eq!(inlines[0].source, "This is **not closed with ");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
        assert_eq!(inlines[1].source, "`code`");
        assert!(matches!(inlines[1].kind, MarkdownInline::Code { .. }));
    }

    #[test]
    fn keeps_scanning_after_broken_closing_strong_marker() {
        let document = Document::new("This is **bold* with `code`");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 2);
        assert_eq!(inlines[0].source, "This is **bold* with ");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
        assert_eq!(inlines[1].source, "`code`");
        assert!(matches!(inlines[1].kind, MarkdownInline::Code { .. }));
    }

    #[test]
    fn projects_multiple_strong_spans() {
        let document = Document::new("**one** and **two**");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 3);
        assert_eq!(inlines[0].content, "one");
        assert!(matches!(inlines[0].kind, MarkdownInline::Strong { .. }));
        assert_eq!(inlines[1].content, " and ");
        assert_eq!(inlines[1].kind, MarkdownInline::Text);
        assert_eq!(inlines[2].content, "two");
        assert!(matches!(inlines[2].kind, MarkdownInline::Strong { .. }));
    }

    #[test]
    fn projects_inline_code_spans_with_source_ranges() {
        let document = Document::new("Use `code` here");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 3);
        assert_eq!(inlines[0].source, "Use ");
        assert_eq!(
            inlines[1],
            ProjectedInline {
                source_range: TextRange::new(4, 10),
                content_range: TextRange::new(5, 9),
                source: "`code`",
                content: "code",
                kind: MarkdownInline::Code {
                    markers: MarkdownMarkerRanges {
                        opening: TextRange::new(4, 5),
                        closing: TextRange::new(9, 10),
                    },
                },
            }
        );
        assert_eq!(inlines[2].source, " here");
    }

    #[test]
    fn exposes_source_visible_segments_for_code_spans() {
        let document = Document::new("Use `code` here");
        let projection = project_document(&document);
        let segments = projection.lines()[0].source_visible_segments();

        assert_eq!(
            segments,
            vec![
                ProjectedSegment {
                    range: TextRange::new(0, 4),
                    source: "Use ",
                    kind: ProjectedSegmentKind::Text,
                },
                ProjectedSegment {
                    range: TextRange::new(4, 5),
                    source: "`",
                    kind: ProjectedSegmentKind::CodeMarker,
                },
                ProjectedSegment {
                    range: TextRange::new(5, 9),
                    source: "code",
                    kind: ProjectedSegmentKind::CodeContent,
                },
                ProjectedSegment {
                    range: TextRange::new(9, 10),
                    source: "`",
                    kind: ProjectedSegmentKind::CodeMarker,
                },
                ProjectedSegment {
                    range: TextRange::new(10, 15),
                    source: " here",
                    kind: ProjectedSegmentKind::Text,
                },
            ]
        );
    }

    #[test]
    fn maps_code_visible_segments_without_projecting_nested_strong() {
        let document = Document::new("Use `**code**` here");
        let projection = project_document(&document);
        let line = &projection.lines()[0];
        let segments = line.visible_segments();

        assert_eq!(line.visible_text(), "Use **code** here");
        assert_eq!(
            segments,
            vec![
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(0, 4),
                    source_range: TextRange::new(0, 4),
                    source_outer_range: TextRange::new(0, 4),
                    source: "Use ",
                    kind: ProjectedSegmentKind::Text,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(4, 12),
                    source_range: TextRange::new(5, 13),
                    source_outer_range: TextRange::new(4, 14),
                    source: "**code**",
                    kind: ProjectedSegmentKind::CodeContent,
                },
                ProjectedVisibleSegment {
                    visible_range: TextRange::new(12, 17),
                    source_range: TextRange::new(14, 19),
                    source_outer_range: TextRange::new(14, 19),
                    source: " here",
                    kind: ProjectedSegmentKind::Text,
                },
            ]
        );
        assert_eq!(line.source_to_visible_offset(4), 4);
        assert_eq!(line.source_to_visible_offset(5), 4);
        assert_eq!(
            line.visible_to_source_offset(4, VisibleOffsetAffinity::Before),
            4
        );
        assert_eq!(
            line.visible_to_source_offset(4, VisibleOffsetAffinity::After),
            5
        );
        assert_eq!(
            line.visible_to_source_offset(12, VisibleOffsetAffinity::Before),
            13
        );
        assert_eq!(
            line.visible_to_source_offset(12, VisibleOffsetAffinity::After),
            14
        );
    }

    #[test]
    fn leaves_unmatched_code_markers_as_text() {
        let document = Document::new("Use `not closed");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 1);
        assert_eq!(inlines[0].source, "Use `not closed");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
    }

    #[test]
    fn keeps_scanning_after_unmatched_code_marker() {
        let document = Document::new("Use `not closed with **strong**");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 2);
        assert_eq!(inlines[0].source, "Use `not closed with ");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
        assert_eq!(inlines[1].source, "**strong**");
        assert!(matches!(inlines[1].kind, MarkdownInline::Strong { .. }));
    }

    #[test]
    fn does_not_project_strong_inside_inline_code() {
        let document = Document::new("Use `**code**` here");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 3);
        assert_eq!(inlines[1].content, "**code**");
        assert!(matches!(inlines[1].kind, MarkdownInline::Code { .. }));
    }
}
