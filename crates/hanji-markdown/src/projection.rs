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
pub struct ProjectedLine<'a> {
    pub range: TextRange,
    pub source: &'a str,
    pub kind: MarkdownLine,
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
        });
    }

    MarkdownProjection { lines }
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
        assert_eq!(lines[1].range, TextRange::new(8, 8));
        assert_eq!(lines[1].source, "");
        assert_eq!(lines[1].kind, MarkdownLine::Blank);
        assert_eq!(lines[2].range, TextRange::new(9, 14));
        assert_eq!(lines[2].source, "Notes");
        assert_eq!(lines[2].kind, MarkdownLine::Paragraph);
    }
}
