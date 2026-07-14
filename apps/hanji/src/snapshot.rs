use gpui::{App, Bounds, Pixels, Point, TextAlign, Window, WrappedLine, point, px};
use hanji_core::TextRange;
use hanji_markdown::{ProjectedSegmentKind, ProjectedVisibleSegment};

#[derive(Clone)]
pub(crate) struct LineSnapshot {
    pub(crate) range: TextRange,
    pub(crate) marker_range: Option<TextRange>,
    pub(crate) visible_len: usize,
    segments: Vec<LineSegmentSnapshot>,
    pub(crate) layout: WrappedLine,
    pub(crate) line_height: Pixels,
    pub(crate) bounds: Bounds<Pixels>,
    table_range: Option<TextRange>,
    table_cells: Option<Vec<TableCellSnapshot>>,
}

pub(crate) struct TableCellLayout<'a> {
    pub(crate) source_range: TextRange,
    pub(crate) source_outer_range: TextRange,
    pub(crate) lines: Vec<TableCellLineLayout<'a>>,
    pub(crate) bounds: Bounds<Pixels>,
    pub(crate) hit_bounds: Bounds<Pixels>,
}

pub(crate) struct TableCellLineLayout<'a> {
    pub(crate) source_range: TextRange,
    pub(crate) visible_segments: Vec<ProjectedVisibleSegment<'a>>,
    pub(crate) layout: WrappedLine,
    pub(crate) bounds: Bounds<Pixels>,
}

#[derive(Clone)]
struct TableCellSnapshot {
    source_range: TextRange,
    source_outer_range: TextRange,
    lines: Vec<TableCellLineSnapshot>,
    bounds: Bounds<Pixels>,
    hit_bounds: Bounds<Pixels>,
}

#[derive(Clone)]
struct TableCellLineSnapshot {
    source_range: TextRange,
    visible_len: usize,
    segments: Vec<LineSegmentSnapshot>,
    layout: WrappedLine,
    bounds: Bounds<Pixels>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TableCellHit {
    pub(crate) table_range: TextRange,
    pub(crate) row_range: TextRange,
    pub(crate) column: usize,
    pub(crate) source_range: TextRange,
    pub(crate) source_outer_range: TextRange,
}

impl LineSnapshot {
    pub(crate) fn new(
        range: TextRange,
        marker_range: Option<TextRange>,
        visible_len: usize,
        visible_segments: Vec<ProjectedVisibleSegment<'_>>,
        layout: WrappedLine,
        line_height: Pixels,
        bounds: Bounds<Pixels>,
    ) -> Self {
        Self {
            range,
            marker_range,
            visible_len,
            segments: visible_segments.into_iter().map(Into::into).collect(),
            layout,
            line_height,
            bounds,
            table_range: None,
            table_cells: None,
        }
    }

    pub(crate) fn new_table(
        range: TextRange,
        table_range: TextRange,
        layout: WrappedLine,
        line_height: Pixels,
        bounds: Bounds<Pixels>,
        cells: Vec<TableCellLayout<'_>>,
    ) -> Self {
        Self {
            range,
            marker_range: None,
            visible_len: 0,
            segments: Vec::new(),
            layout,
            line_height,
            bounds,
            table_range: Some(table_range),
            table_cells: Some(
                cells
                    .into_iter()
                    .map(|cell| TableCellSnapshot {
                        source_range: cell.source_range,
                        source_outer_range: cell.source_outer_range,
                        lines: cell
                            .lines
                            .into_iter()
                            .map(|line| {
                                let visible_len = line
                                    .visible_segments
                                    .last()
                                    .map_or(0, |segment| segment.visible_range.end);
                                TableCellLineSnapshot {
                                    source_range: line.source_range,
                                    visible_len,
                                    segments: line
                                        .visible_segments
                                        .into_iter()
                                        .map(Into::into)
                                        .collect(),
                                    layout: line.layout,
                                    bounds: line.bounds,
                                }
                            })
                            .collect(),
                        bounds: cell.bounds,
                        hit_bounds: cell.hit_bounds,
                    })
                    .collect(),
            ),
        }
    }

    pub(crate) fn paint(&self, window: &mut Window, cx: &mut App) {
        if let Some(cells) = &self.table_cells {
            for cell in cells {
                for line in &cell.lines {
                    line.layout
                        .paint(
                            line.bounds.origin,
                            self.line_height,
                            TextAlign::Left,
                            Some(cell.bounds),
                            window,
                            cx,
                        )
                        .ok();
                }
            }
            return;
        }

        self.layout
            .paint(
                self.bounds.origin,
                self.line_height,
                TextAlign::Left,
                Some(self.bounds),
                window,
                cx,
            )
            .ok();
    }

    pub(crate) fn source_caret_position(&self, source_offset: usize) -> Option<Point<Pixels>> {
        let Some(cells) = &self.table_cells else {
            let visible_offset = self.source_to_visible_offset(source_offset);
            return self.wrapped_caret_position(visible_offset);
        };
        let cell = table_cell_for_source_offset(cells, source_offset)?;
        let line = table_cell_line_for_source_offset(cell, source_offset)?;
        let visible_offset = source_to_visible_offset_in_segments(
            &line.segments,
            line.source_range,
            line.visible_len,
            source_offset,
        );
        let position = wrapped_caret_position_for_layout(
            &line.layout,
            line.visible_len,
            self.line_height,
            visible_offset,
        )?;

        Some(point(
            line.bounds.left() - self.bounds.left() + position.x,
            line.bounds.top() - self.bounds.top() + position.y,
        ))
    }

    pub(crate) fn source_offset_for_local_position(&self, position: Point<Pixels>) -> usize {
        let Some(cells) = &self.table_cells else {
            let visible_offset = self.visible_offset_for_local_position(position);
            return self.visible_to_source_caret_offset(visible_offset);
        };
        let Some(cell) = table_cell_for_local_x(cells, self.bounds.left() + position.x) else {
            return self.range.start;
        };
        let global_y = self.bounds.top() + position.y;
        let Some(line) = table_cell_line_for_y(cell, global_y) else {
            return cell.source_range.start;
        };
        let local_position = point(
            position.x + self.bounds.left() - line.bounds.left(),
            position.y + self.bounds.top() - line.bounds.top(),
        );
        let visible_offset = line
            .layout
            .closest_index_for_position(local_position, self.line_height)
            .unwrap_or_else(|offset| offset)
            .min(line.visible_len);

        visible_segments_to_source_caret_offset(
            &line.segments,
            line.source_range,
            line.visible_len,
            visible_offset,
        )
    }

    pub(crate) fn table_cell_hit_for_local_position(
        &self,
        position: Point<Pixels>,
    ) -> Option<TableCellHit> {
        let table_range = self.table_range?;
        let cells = self.table_cells.as_ref()?;
        let (column, cell) = cells
            .iter()
            .enumerate()
            .find(|(_, cell)| {
                let x = self.bounds.left() + position.x;
                x >= cell.hit_bounds.left() && x <= cell.hit_bounds.right()
            })
            .or_else(|| {
                let x = self.bounds.left() + position.x;
                cells
                    .iter()
                    .enumerate()
                    .find(|(_, cell)| x < cell.hit_bounds.left())
            })
            .or_else(|| cells.iter().enumerate().next_back())?;

        Some(TableCellHit {
            table_range,
            row_range: self.range,
            column,
            source_range: cell.source_range,
            source_outer_range: cell.source_outer_range,
        })
    }

    pub(crate) fn table_cell_bounds_for_selection(
        &self,
        table_range: TextRange,
        row_start: usize,
        row_end: usize,
        column_start: usize,
        column_end: usize,
    ) -> Vec<Bounds<Pixels>> {
        if self.table_range != Some(table_range)
            || self.range.start < row_start
            || self.range.start > row_end
        {
            return Vec::new();
        }

        self.table_cells
            .as_ref()
            .into_iter()
            .flatten()
            .enumerate()
            .filter(|(column, _)| *column >= column_start && *column <= column_end)
            .map(|(_, cell)| cell.hit_bounds)
            .collect()
    }

    pub(crate) fn source_range_bounds(
        &self,
        range: TextRange,
        min_width: Pixels,
    ) -> Vec<Bounds<Pixels>> {
        let Some(cells) = &self.table_cells else {
            let visible_start = self.source_to_visible_offset(range.start);
            let visible_end = self.source_to_visible_offset(range.end);
            return self
                .wrapped_range_bounds(TextRange::new(visible_start, visible_end), min_width);
        };

        cells
            .iter()
            .flat_map(|cell| {
                let cell_is_fully_selected = !range.is_empty()
                    && range.start <= cell.source_range.start
                    && range.end >= cell.source_range.end;
                if cell_is_fully_selected {
                    return vec![cell.hit_bounds];
                }

                cell.lines
                    .iter()
                    .flat_map(|line| {
                        let start = range.start.max(line.source_range.start);
                        let end = range.end.min(line.source_range.end);
                        if start >= end {
                            return Vec::new();
                        }

                        let visible_start = source_to_visible_offset_in_segments(
                            &line.segments,
                            line.source_range,
                            line.visible_len,
                            start,
                        );
                        let visible_end = source_to_visible_offset_in_segments(
                            &line.segments,
                            line.source_range,
                            line.visible_len,
                            end,
                        );
                        wrapped_range_bounds_for_layout(
                            &line.layout,
                            line.bounds,
                            line.visible_len,
                            self.line_height,
                            TextRange::new(visible_start, visible_end),
                            min_width,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub(crate) fn source_visual_row(&self, source_offset: usize) -> Option<usize> {
        if let Some(cells) = &self.table_cells {
            let cell = table_cell_for_source_offset(cells, source_offset)?;
            let (line_index, line) = table_cell_line_index_for_source_offset(cell, source_offset)?;
            let visible_offset = source_to_visible_offset_in_segments(
                &line.segments,
                line.source_range,
                line.visible_len,
                source_offset,
            );
            let position = wrapped_caret_position_for_layout(
                &line.layout,
                line.visible_len,
                self.line_height,
                visible_offset,
            )?;
            let preceding_rows = cell.lines[..line_index]
                .iter()
                .map(table_cell_line_row_count)
                .sum::<usize>();

            return Some(preceding_rows + (position.y / self.line_height) as usize);
        }

        let visible_offset = self.source_to_visible_offset(source_offset);
        self.wrapped_row_for_visible_offset(visible_offset)
    }

    pub(crate) fn source_offset_for_visual_row_x(&self, row: usize, x: Pixels) -> usize {
        if let Some(cells) = &self.table_cells {
            let content_top = cells
                .first()
                .map_or(px(0.0), |cell| cell.bounds.top() - self.bounds.top());
            let row = row.min(self.wrapped_row_count().saturating_sub(1));
            return self.source_offset_for_local_position(point(
                x,
                content_top + self.line_height * row as f32 + self.line_height / 2.0,
            ));
        }

        let visible_offset = self.visible_offset_for_wrapped_row_x(row, x);
        self.visible_to_source_caret_offset(visible_offset)
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

    pub(crate) fn wrapped_row_count(&self) -> usize {
        if let Some(cells) = &self.table_cells {
            return cells
                .iter()
                .map(|cell| cell.lines.iter().map(table_cell_line_row_count).sum())
                .max()
                .unwrap_or(0);
        }
        self.layout.wrap_boundaries().len() + 1
    }

    pub(crate) fn wrapped_caret_position(&self, visible_offset: usize) -> Option<Point<Pixels>> {
        wrapped_caret_position_for_layout(
            &self.layout,
            self.visible_len,
            self.line_height,
            visible_offset,
        )
    }

    pub(crate) fn wrapped_row_for_visible_offset(&self, visible_offset: usize) -> Option<usize> {
        let position = self.wrapped_caret_position(visible_offset)?;
        Some((position.y / self.line_height) as usize)
    }

    pub(crate) fn visible_offset_for_local_position(&self, position: Point<Pixels>) -> usize {
        self.layout
            .closest_index_for_position(position, self.line_height)
            .unwrap_or_else(|offset| offset)
            .min(self.visible_len)
    }

    pub(crate) fn visible_offset_for_wrapped_row_x(&self, row: usize, x: Pixels) -> usize {
        let row = row.min(self.wrapped_row_count().saturating_sub(1));
        self.visible_offset_for_local_position(point(
            x.max(px(0.0)),
            self.line_height * row as f32 + self.line_height / 2.0,
        ))
    }

    pub(crate) fn wrapped_range_bounds(
        &self,
        range: TextRange,
        min_width: Pixels,
    ) -> Vec<Bounds<Pixels>> {
        wrapped_range_bounds_for_layout(
            &self.layout,
            self.bounds,
            self.visible_len,
            self.line_height,
            range,
            min_width,
        )
    }
}

#[derive(Clone, Copy)]
struct LineSegmentSnapshot {
    visible_range: TextRange,
    source_range: TextRange,
    source_outer_range: TextRange,
    uses_outer_caret_edges: bool,
}

#[derive(Clone, Copy)]
struct WrappedVisualRange {
    row: usize,
    range: TextRange,
    start_x: Pixels,
}

fn wrapped_caret_position_for_layout(
    layout: &WrappedLine,
    visible_len: usize,
    line_height: Pixels,
    visible_offset: usize,
) -> Option<Point<Pixels>> {
    let visible_offset = visible_offset.min(visible_len);

    for visual in wrapped_visual_ranges_for_layout(layout, visible_len) {
        let is_last_row = visual.range.end == visible_len;

        if visible_offset < visual.range.start {
            continue;
        }

        if visible_offset < visual.range.end || (is_last_row && visible_offset == visual.range.end)
        {
            let x = layout.unwrapped_layout.x_for_index(visible_offset) - visual.start_x;
            return Some(point(x, line_height * visual.row as f32));
        }

        if visible_offset == visual.range.end && !is_last_row {
            continue;
        }
    }

    layout.position_for_index(visible_offset, line_height)
}

fn wrapped_range_bounds_for_layout(
    layout: &WrappedLine,
    layout_bounds: Bounds<Pixels>,
    visible_len: usize,
    line_height: Pixels,
    range: TextRange,
    min_width: Pixels,
) -> Vec<Bounds<Pixels>> {
    if range.is_empty() {
        return Vec::new();
    }

    let range = TextRange::new(range.start.min(visible_len), range.end.min(visible_len));
    let mut bounds = Vec::new();

    for visual in wrapped_visual_ranges_for_layout(layout, visible_len) {
        let start = range.start.max(visual.range.start);
        let end = range.end.min(visual.range.end);

        if start >= end {
            continue;
        }

        let start_x = layout.unwrapped_layout.x_for_index(start) - visual.start_x;
        let end_x = layout.unwrapped_layout.x_for_index(end) - visual.start_x;
        let top = layout_bounds.top() + line_height * visual.row as f32;
        let left = layout_bounds.left() + start_x;
        let right = (layout_bounds.left() + end_x).max(left + min_width);

        bounds.push(Bounds::from_corners(
            point(left, top),
            point(right, top + line_height),
        ));
    }

    bounds
}

fn wrapped_visual_ranges_for_layout(
    layout: &WrappedLine,
    visible_len: usize,
) -> Vec<WrappedVisualRange> {
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut start_x = px(0.0);

    for (row, boundary) in layout.wrap_boundaries().iter().enumerate() {
        let Some(run) = layout.runs().get(boundary.run_ix) else {
            continue;
        };
        let Some(glyph) = run.glyphs.get(boundary.glyph_ix) else {
            continue;
        };
        let end = glyph.index.min(visible_len);

        if start < end {
            ranges.push(WrappedVisualRange {
                row,
                range: TextRange::new(start, end),
                start_x,
            });
        }

        start = end;
        start_x = glyph.position.x;
    }

    ranges.push(WrappedVisualRange {
        row: layout.wrap_boundaries().len(),
        range: TextRange::new(start, visible_len),
        start_x,
    });

    ranges
}

impl From<ProjectedVisibleSegment<'_>> for LineSegmentSnapshot {
    fn from(segment: ProjectedVisibleSegment<'_>) -> Self {
        Self {
            visible_range: segment.visible_range,
            source_range: segment.source_range,
            source_outer_range: segment.source_outer_range,
            uses_outer_caret_edges: uses_outer_caret_edges(segment),
        }
    }
}

fn uses_outer_caret_edges(segment: ProjectedVisibleSegment<'_>) -> bool {
    matches!(
        segment.kind,
        ProjectedSegmentKind::StrongContent
            | ProjectedSegmentKind::EmphasisContent
            | ProjectedSegmentKind::StrikethroughContent
            | ProjectedSegmentKind::CodeContent
            | ProjectedSegmentKind::LinkText
    ) || (!segment.style.is_plain() && segment.source_outer_range != segment.source_range)
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
            if segment.uses_outer_caret_edges {
                return segment.source_outer_range.start;
            }

            return segment.source_range.start;
        }

        if visible_offset == segment.visible_range.end {
            if let Some(next_segment) = segments.get(index + 1)
                && next_segment.visible_range.start == visible_offset
            {
                if next_segment.uses_outer_caret_edges {
                    return next_segment.source_outer_range.start;
                }

                return next_segment.source_range.start;
            }

            if segment.uses_outer_caret_edges {
                return segment.source_outer_range.end;
            }

            return segment.source_range.end;
        }

        return segment.source_range.start + visible_offset - segment.visible_range.start;
    }

    line_range.end
}

fn table_cell_for_source_offset(
    cells: &[TableCellSnapshot],
    source_offset: usize,
) -> Option<&TableCellSnapshot> {
    cells
        .iter()
        .find(|cell| {
            source_offset >= cell.source_range.start && source_offset <= cell.source_range.end
        })
        .or_else(|| {
            cells
                .iter()
                .find(|cell| source_offset < cell.source_range.start)
        })
        .or_else(|| cells.last())
}

fn table_cell_line_for_source_offset(
    cell: &TableCellSnapshot,
    source_offset: usize,
) -> Option<&TableCellLineSnapshot> {
    table_cell_line_index_for_source_offset(cell, source_offset).map(|(_, line)| line)
}

fn table_cell_line_index_for_source_offset(
    cell: &TableCellSnapshot,
    source_offset: usize,
) -> Option<(usize, &TableCellLineSnapshot)> {
    cell.lines
        .iter()
        .enumerate()
        .find(|(_, line)| {
            source_offset >= line.source_range.start && source_offset <= line.source_range.end
        })
        .or_else(|| {
            cell.lines
                .iter()
                .enumerate()
                .find(|(_, line)| source_offset < line.source_range.start)
        })
        .or_else(|| cell.lines.iter().enumerate().next_back())
}

fn table_cell_line_for_y(cell: &TableCellSnapshot, y: Pixels) -> Option<&TableCellLineSnapshot> {
    cell.lines
        .iter()
        .find(|line| y >= line.bounds.top() && y <= line.bounds.bottom())
        .or_else(|| cell.lines.iter().find(|line| y < line.bounds.top()))
        .or_else(|| cell.lines.last())
}

fn table_cell_line_row_count(line: &TableCellLineSnapshot) -> usize {
    line.layout.wrap_boundaries().len() + 1
}

fn table_cell_for_local_x(cells: &[TableCellSnapshot], x: Pixels) -> Option<&TableCellSnapshot> {
    cells
        .iter()
        .find(|cell| x >= cell.hit_bounds.left() && x <= cell.hit_bounds.right())
        .or_else(|| cells.iter().find(|cell| x < cell.hit_bounds.left()))
        .or_else(|| cells.last())
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
    use gpui::size;
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
            "Capture ".len()
        );
        assert_eq!(
            visible_segments_to_source_caret_offset(
                &segments,
                line.range,
                visible_len,
                "Capture thought".len()
            ),
            "Capture **thought**".len()
        );
        assert_eq!(
            visible_segments_to_source_caret_offset(
                &segments,
                line.range,
                visible_len,
                "Capture thought with ".len()
            ),
            "Capture **thought** with ".len()
        );
        assert_eq!(
            visible_segments_to_source_caret_offset(
                &segments,
                line.range,
                visible_len,
                "Capture thought with code".len()
            ),
            "Capture **thought** with `code`".len()
        );
    }

    #[test]
    fn hidden_inline_end_at_line_end_hit_tests_to_outer_marker_edge() {
        let document = Document::new("Capture **thought**");
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
                "Capture thought".len()
            ),
            "Capture **thought**".len()
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
    fn table_cell_selection_uses_complete_cell_bounds() {
        let first_hit = Bounds::new(point(px(0.0), px(0.0)), size(px(100.0), px(32.0)));
        let second_hit = Bounds::new(point(px(100.0), px(0.0)), size(px(100.0), px(32.0)));
        let content_bounds = |left| Bounds::new(point(px(left), px(4.0)), size(px(84.0), px(24.0)));
        let snapshot = LineSnapshot::new_table(
            TextRange::new(0, 16),
            TextRange::new(0, 16),
            WrappedLine::default(),
            px(24.0),
            Bounds::new(point(px(0.0), px(0.0)), size(px(200.0), px(32.0))),
            vec![
                TableCellLayout {
                    source_range: TextRange::new(1, 5),
                    source_outer_range: TextRange::new(0, 6),
                    lines: vec![TableCellLineLayout {
                        source_range: TextRange::new(1, 5),
                        visible_segments: Vec::new(),
                        layout: WrappedLine::default(),
                        bounds: content_bounds(8.0),
                    }],
                    bounds: content_bounds(8.0),
                    hit_bounds: first_hit,
                },
                TableCellLayout {
                    source_range: TextRange::new(8, 14),
                    source_outer_range: TextRange::new(7, 15),
                    lines: vec![TableCellLineLayout {
                        source_range: TextRange::new(8, 14),
                        visible_segments: Vec::new(),
                        layout: WrappedLine::default(),
                        bounds: content_bounds(108.0),
                    }],
                    bounds: content_bounds(108.0),
                    hit_bounds: second_hit,
                },
            ],
        );
        let hit = snapshot
            .table_cell_hit_for_local_position(point(px(150.0), px(16.0)))
            .unwrap();

        assert_eq!(
            snapshot.source_range_bounds(TextRange::new(1, 14), px(0.0)),
            vec![first_hit, second_hit]
        );
        assert_eq!(hit.column, 1);
        assert_eq!(hit.source_range, TextRange::new(8, 14));
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
