use hanji_core::{Document, TextRange};

use crate::{MarkdownLine, classify_line};

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
    StrongMarker,
    StrongContent,
    CodeMarker,
    CodeContent,
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
    Strong { markers: MarkdownMarkerRanges },
    Code { markers: MarkdownMarkerRanges },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkdownMarkerRanges {
    pub opening: TextRange,
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
        let mut segments = Vec::new();
        let mut visible_start = 0;

        for inline in &self.inlines {
            if reveal_range.is_some_and(|range| should_reveal_inline(inline.source_range, range)) {
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
        let mut segments = Vec::new();

        for inline in &self.inlines {
            match inline.kind {
                MarkdownInline::Text => {
                    push_segment(
                        &mut segments,
                        self.source,
                        self.range.start,
                        inline.source_range,
                        ProjectedSegmentKind::Text,
                    );
                }
                MarkdownInline::Strong { markers } => {
                    push_segment(
                        &mut segments,
                        self.source,
                        self.range.start,
                        markers.opening,
                        ProjectedSegmentKind::StrongMarker,
                    );
                    push_segment(
                        &mut segments,
                        self.source,
                        self.range.start,
                        inline.content_range,
                        ProjectedSegmentKind::StrongContent,
                    );
                    push_segment(
                        &mut segments,
                        self.source,
                        self.range.start,
                        markers.closing,
                        ProjectedSegmentKind::StrongMarker,
                    );
                }
                MarkdownInline::Code { markers } => {
                    push_segment(
                        &mut segments,
                        self.source,
                        self.range.start,
                        markers.opening,
                        ProjectedSegmentKind::CodeMarker,
                    );
                    push_segment(
                        &mut segments,
                        self.source,
                        self.range.start,
                        inline.content_range,
                        ProjectedSegmentKind::CodeContent,
                    );
                    push_segment(
                        &mut segments,
                        self.source,
                        self.range.start,
                        markers.closing,
                        ProjectedSegmentKind::CodeMarker,
                    );
                }
            }
        }

        segments
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

pub fn project_document(document: &Document) -> MarkdownProjection<'_> {
    let mut lines = Vec::new();

    for line_index in 0..document.line_count() {
        let range = document
            .line_range(line_index)
            .unwrap_or_else(|| TextRange::caret(document.len()));
        let source = &document.text()[range.start..range.end];

        lines.push(ProjectedLine {
            range,
            source,
            kind: classify_line(source),
            inlines: project_inlines(source, range.start),
        });
    }

    MarkdownProjection { lines }
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
) {
    let kind = match inline.kind {
        MarkdownInline::Text => ProjectedSegmentKind::Text,
        MarkdownInline::Strong { .. } => ProjectedSegmentKind::StrongContent,
        MarkdownInline::Code { .. } => ProjectedSegmentKind::CodeContent,
    };

    push_visible_segment(
        segments,
        line_source,
        line_start,
        visible_start,
        inline.source_range,
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

        push_text_with_strong_inlines(
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

    push_text_with_strong_inlines(&mut inlines, source, source_start, text_start, source.len());

    inlines
}

fn push_text_with_strong_inlines<'a>(
    inlines: &mut Vec<ProjectedInline<'a>>,
    source: &'a str,
    source_start: usize,
    start: usize,
    end: usize,
) {
    let mut text_start = start;

    for (opening_start, closing_start) in strong_pairs(source, start, end) {
        if opening_start < text_start {
            continue;
        }

        push_text_inline(inlines, source, source_start, text_start, opening_start);
        push_strong_inline(
            inlines,
            source,
            source_start,
            opening_start,
            opening_start + 2,
            closing_start,
        );

        text_start = closing_start + 2;
    }

    push_text_inline(inlines, source, source_start, text_start, end);
}

fn strong_pairs(source: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let mut pending_opening = None;
    let mut search_start = start;

    while let Some(marker_start) = find_exact_strong_marker(source, search_start, end) {
        let can_close = can_close_strong(source, marker_start);
        let can_open = can_open_strong(source, marker_start, end);

        if let Some(opening_start) = pending_opening
            && can_close
            && opening_start + 2 < marker_start
        {
            pairs.push((opening_start, marker_start));
            pending_opening = None;
        } else if can_open {
            pending_opening = Some(marker_start);
        }

        search_start = marker_start + 2;
    }

    pairs
}

fn find_exact_strong_marker(source: &str, search_start: usize, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = search_start;

    while cursor + 2 <= end {
        if bytes[cursor] == b'*' && bytes[cursor + 1] == b'*' {
            let starts_run = cursor == 0 || bytes[cursor - 1] != b'*';
            let ends_run = cursor + 2 >= end || bytes[cursor + 2] != b'*';

            if starts_run && ends_run {
                return Some(cursor);
            }

            while cursor < end && bytes[cursor] == b'*' {
                cursor += 1;
            }
        } else {
            cursor += 1;
        }
    }

    None
}

fn can_open_strong(source: &str, marker_start: usize, end: usize) -> bool {
    next_char(source, marker_start + 2, end).is_some_and(|character| !character.is_whitespace())
}

fn can_close_strong(source: &str, marker_start: usize) -> bool {
    previous_char(source, marker_start).is_some_and(|character| !character.is_whitespace())
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
    source[search_start..]
        .find('`')
        .map(|offset| search_start + offset)
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
        assert_eq!(lines[0].inlines.len(), 1);
        assert_eq!(lines[1].range, TextRange::new(8, 8));
        assert_eq!(lines[1].source, "");
        assert_eq!(lines[1].kind, MarkdownLine::Blank);
        assert_eq!(lines[1].inlines.len(), 0);
        assert_eq!(lines[2].range, TextRange::new(9, 16));
        assert_eq!(lines[2].source, "> Quote");
        assert_eq!(lines[2].kind, MarkdownLine::Blockquote);
        assert_eq!(lines[2].inlines.len(), 1);
        assert_eq!(lines[3].range, TextRange::new(17, 22));
        assert_eq!(lines[3].source, "Notes");
        assert_eq!(lines[3].kind, MarkdownLine::Paragraph);
        assert_eq!(lines[3].inlines.len(), 1);
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
    fn projects_visible_text_without_markers() {
        let document = Document::new("This is **bold** and `code`");
        let projection = project_document(&document);
        let line = &projection.lines()[0];

        assert_eq!(line.visible_text(), "This is bold and code");
        assert_eq!(line.visible_len(), "This is bold and code".len());
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
    fn does_not_project_single_asterisk_emphasis_as_strong() {
        let document = Document::new("This is *not strong*");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 1);
        assert_eq!(inlines[0].source, "This is *not strong*");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
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
    fn does_not_project_strong_from_partially_deleted_adjacent_markers() {
        let document = Document::new("This is *abc* before **bold**");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 2);
        assert_eq!(inlines[0].source, "This is *abc* before ");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
        assert_eq!(inlines[1].source, "**bold**");
        assert!(matches!(inlines[1].kind, MarkdownInline::Strong { .. }));
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
