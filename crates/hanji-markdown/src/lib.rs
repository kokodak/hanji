mod command;
mod line;
mod projection;
mod table;

pub use command::{
    MarkdownCommand, MarkdownCommandError, execute_markdown_command, insert_link, toggle_code,
    toggle_emphasis, toggle_strong,
};
pub use line::{
    MarkdownCodeBlockLine, MarkdownLine, MarkdownListItem, MarkdownListMarker, MarkdownTableLine,
    MarkdownTaskState, OrderedListDelimiter, blockquote_content_start, classify_line,
    first_heading, heading_content_start, horizontal_rule_marker_range, list_item,
    list_item_content_start,
};
pub use projection::{
    MarkdownInline, MarkdownLinkRanges, MarkdownMarkerRanges, MarkdownProjection, ProjectedInline,
    ProjectedInlineStyle, ProjectedLine, ProjectedSegment, ProjectedSegmentKind,
    ProjectedTableCell, ProjectedVisibleSegment, VisibleOffsetAffinity, project_document,
};
