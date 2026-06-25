use gpui::{Bounds, Pixels, ShapedLine};
use hanji_core::TextRange;
use hanji_markdown::ProjectedVisibleSegment;

#[derive(Clone)]
pub(crate) struct LineSnapshot {
    pub(crate) range: TextRange,
    pub(crate) marker_range: Option<TextRange>,
    pub(crate) visible_len: usize,
    segments: Vec<LineSegmentSnapshot>,
    pub(crate) layout: ShapedLine,
    pub(crate) bounds: Bounds<Pixels>,
}

impl LineSnapshot {
    pub(crate) fn new(
        range: TextRange,
        marker_range: Option<TextRange>,
        visible_len: usize,
        visible_segments: Vec<ProjectedVisibleSegment<'_>>,
        layout: ShapedLine,
        bounds: Bounds<Pixels>,
    ) -> Self {
        Self {
            range,
            marker_range,
            visible_len,
            segments: visible_segments.into_iter().map(Into::into).collect(),
            layout,
            bounds,
        }
    }

    pub(crate) fn source_to_visible_offset(&self, source_offset: usize) -> usize {
        source_to_visible_offset_in_segments(
            &self.segments,
            self.range,
            self.visible_len,
            source_offset,
        )
    }

    pub(crate) fn visible_to_source_caret_offset(&self, visible_offset: usize) -> usize {
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

pub(crate) fn line_for_offset(lines: &[LineSnapshot], offset: usize) -> Option<&LineSnapshot> {
    lines
        .iter()
        .find(|line| offset >= line.range.start && offset <= line.range.end)
        .or_else(|| lines.last())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanji_core::Document;
    use hanji_markdown::{ProjectedVisibleSegment, project_document};

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

    fn visible_text_from_segments(segments: &[ProjectedVisibleSegment<'_>]) -> String {
        let mut text = String::new();

        for segment in segments {
            text.push_str(segment.source);
        }

        text
    }
}
