mod command;
mod line;
mod projection;

pub use command::{
    MarkdownCommand, MarkdownCommandError, execute_markdown_command, toggle_code, toggle_strong,
};
pub use line::{MarkdownLine, classify_line, first_heading};
pub use projection::{
    MarkdownInline, MarkdownMarkerRanges, MarkdownProjection, ProjectedInline, ProjectedLine,
    ProjectedSegment, ProjectedSegmentKind, ProjectedVisibleSegment, VisibleOffsetAffinity,
    project_document,
};
