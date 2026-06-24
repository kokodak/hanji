mod command;
mod line;
mod projection;

pub use command::{
    MarkdownCommand, MarkdownCommandError, execute_markdown_command, toggle_code, toggle_emphasis,
    toggle_strong,
};
pub use line::{MarkdownLine, blockquote_content_start, classify_line, first_heading};
pub use projection::{
    MarkdownInline, MarkdownMarkerRanges, MarkdownProjection, ProjectedInline, ProjectedLine,
    ProjectedSegment, ProjectedSegmentKind, ProjectedVisibleSegment, VisibleOffsetAffinity,
    project_document,
};
