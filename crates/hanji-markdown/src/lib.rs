mod command;
mod line;
mod projection;

pub use command::{
    MarkdownCommand, MarkdownCommandError, execute_markdown_command, toggle_code, toggle_emphasis,
    toggle_strong,
};
pub use line::{
    MarkdownCodeBlockLine, MarkdownLine, MarkdownListItem, MarkdownListMarker, MarkdownTaskState,
    OrderedListDelimiter, blockquote_content_start, classify_line, first_heading,
    heading_content_start, list_item, list_item_content_start,
};
pub use projection::{
    MarkdownInline, MarkdownLinkRanges, MarkdownMarkerRanges, MarkdownProjection, ProjectedInline,
    ProjectedLine, ProjectedSegment, ProjectedSegmentKind, ProjectedVisibleSegment,
    VisibleOffsetAffinity, project_document,
};
