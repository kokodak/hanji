use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkdownTableRow {
    pub(crate) cells: Vec<Range<usize>>,
    pub(crate) separators: Vec<Range<usize>>,
    pub(crate) leading_separator: Option<Range<usize>>,
    pub(crate) trailing_separator: Option<Range<usize>>,
}

pub(crate) fn parse_table_row(source: &str) -> Option<MarkdownTableRow> {
    let separators = unescaped_pipe_ranges(source);
    if separators.is_empty() {
        return None;
    }

    let leading_separator = separators
        .first()
        .filter(|separator| source[..separator.start].trim().is_empty())
        .cloned();
    let trailing_separator = separators
        .last()
        .filter(|separator| source[separator.end..].trim().is_empty())
        .cloned();

    let content_start = leading_separator.as_ref().map_or(0, |range| range.end);
    let content_end = trailing_separator
        .as_ref()
        .map_or(source.len(), |range| range.start);
    if content_start > content_end {
        return None;
    }

    let internal_separators = separators
        .iter()
        .filter(|separator| {
            separator.start >= content_start
                && separator.end <= content_end
                && leading_separator.as_ref() != Some(*separator)
                && trailing_separator.as_ref() != Some(*separator)
        })
        .cloned()
        .collect::<Vec<_>>();

    let mut cells = Vec::with_capacity(internal_separators.len() + 1);
    let mut cell_start = content_start;
    for separator in &internal_separators {
        cells.push(cell_start..separator.start);
        cell_start = separator.end;
    }
    cells.push(cell_start..content_end);

    Some(MarkdownTableRow {
        cells,
        separators,
        leading_separator,
        trailing_separator,
    })
}

pub(crate) fn is_delimiter_row(source: &str, row: &MarkdownTableRow) -> bool {
    row.cells
        .iter()
        .all(|cell| is_delimiter_cell(&source[cell.clone()]))
}

fn is_delimiter_cell(source: &str) -> bool {
    let source = source.trim();
    let source = source.strip_prefix(':').unwrap_or(source);
    let source = source.strip_suffix(':').unwrap_or(source);

    source.len() >= 3 && source.bytes().all(|byte| byte == b'-')
}

fn unescaped_pipe_ranges(source: &str) -> Vec<Range<usize>> {
    let mut separators = Vec::new();
    let mut preceding_backslashes = 0;

    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\\' {
            preceding_backslashes += 1;
            continue;
        }

        if byte == b'|' && preceding_backslashes % 2 == 0 {
            separators.push(index..index + 1);
        }
        preceding_backslashes = 0;
    }

    separators
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rows_with_optional_boundary_pipes() {
        let row = parse_table_row("| Name | Status |").unwrap();
        assert_eq!(row.cells, vec![1..7, 8..16]);
        assert_eq!(row.leading_separator, Some(0..1));
        assert_eq!(row.trailing_separator, Some(16..17));

        let row = parse_table_row("Name | Status").unwrap();
        assert_eq!(row.cells, vec![0..5, 6..13]);
        assert_eq!(row.leading_separator, None);
        assert_eq!(row.trailing_separator, None);
    }

    #[test]
    fn ignores_escaped_pipes_when_splitting_cells() {
        let row = parse_table_row(r"| Name \| Alias | Status |").unwrap();

        assert_eq!(row.cells.len(), 2);
        assert_eq!(
            &r"| Name \| Alias | Status |"[row.cells[0].clone()],
            " Name \\| Alias "
        );
    }

    #[test]
    fn recognizes_complete_delimiter_rows() {
        let source = "| :--- | ---: | :---: |";
        let row = parse_table_row(source).unwrap();

        assert!(is_delimiter_row(source, &row));
    }

    #[test]
    fn rejects_incomplete_delimiter_rows() {
        for source in ["| -- | --- |", "| --- | status |", "| :--: | --- |"] {
            let row = parse_table_row(source).unwrap();
            assert!(!is_delimiter_row(source, &row), "{source}");
        }
    }
}
