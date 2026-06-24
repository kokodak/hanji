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

fn project_inlines(source: &str, source_start: usize) -> Vec<ProjectedInline<'_>> {
    let mut inlines = Vec::new();
    let mut text_start = 0;
    let mut search_start = 0;

    while let Some(marker) = find_next_marker(source, search_start) {
        match marker {
            InlineMarker::Strong { start } => {
                let content_start = start + 2;
                let Some(closing_start) = find_strong_marker(source, content_start) else {
                    break;
                };

                if closing_start == content_start {
                    search_start = content_start;
                    continue;
                }

                push_text_inline(&mut inlines, source, source_start, text_start, start);
                push_strong_inline(
                    &mut inlines,
                    source,
                    source_start,
                    start,
                    content_start,
                    closing_start,
                );

                search_start = closing_start + 2;
                text_start = search_start;
            }
            InlineMarker::Code { start } => {
                let content_start = start + 1;
                let Some(closing_start) = find_code_marker(source, content_start) else {
                    break;
                };

                if closing_start == content_start {
                    search_start = content_start;
                    continue;
                }

                push_text_inline(&mut inlines, source, source_start, text_start, start);
                push_code_inline(
                    &mut inlines,
                    source,
                    source_start,
                    start,
                    content_start,
                    closing_start,
                );

                search_start = closing_start + 1;
                text_start = search_start;
            }
        }
    }

    push_text_inline(&mut inlines, source, source_start, text_start, source.len());

    inlines
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineMarker {
    Strong { start: usize },
    Code { start: usize },
}

fn find_next_marker(source: &str, search_start: usize) -> Option<InlineMarker> {
    let strong = find_strong_marker(source, search_start);
    let code = find_code_marker(source, search_start);

    match (strong, code) {
        (Some(strong), Some(code)) if strong <= code => {
            Some(InlineMarker::Strong { start: strong })
        }
        (Some(_), Some(code)) => Some(InlineMarker::Code { start: code }),
        (Some(strong), None) => Some(InlineMarker::Strong { start: strong }),
        (None, Some(code)) => Some(InlineMarker::Code { start: code }),
        (None, None) => None,
    }
}

fn find_strong_marker(source: &str, search_start: usize) -> Option<usize> {
    source[search_start..]
        .find("**")
        .map(|offset| search_start + offset)
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

    #[test]
    fn projects_lines_with_source_ranges_and_kinds() {
        let document = Document::new("# Hanji\n\nNotes");
        let projection = project_document(&document);
        let lines = projection.lines();

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].range, TextRange::new(0, 7));
        assert_eq!(lines[0].source, "# Hanji");
        assert_eq!(lines[0].kind, MarkdownLine::Heading { level: 1 });
        assert_eq!(lines[0].inlines.len(), 1);
        assert_eq!(lines[1].range, TextRange::new(8, 8));
        assert_eq!(lines[1].source, "");
        assert_eq!(lines[1].kind, MarkdownLine::Blank);
        assert_eq!(lines[1].inlines.len(), 0);
        assert_eq!(lines[2].range, TextRange::new(9, 14));
        assert_eq!(lines[2].source, "Notes");
        assert_eq!(lines[2].kind, MarkdownLine::Paragraph);
        assert_eq!(lines[2].inlines.len(), 1);
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
    fn leaves_unmatched_strong_markers_as_text() {
        let document = Document::new("This is **not closed");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 1);
        assert_eq!(inlines[0].source, "This is **not closed");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
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
    fn leaves_unmatched_code_markers_as_text() {
        let document = Document::new("Use `not closed");
        let projection = project_document(&document);
        let inlines = &projection.lines()[0].inlines;

        assert_eq!(inlines.len(), 1);
        assert_eq!(inlines[0].source, "Use `not closed");
        assert_eq!(inlines[0].kind, MarkdownInline::Text);
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
