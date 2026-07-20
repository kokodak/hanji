//! Markdown-specific parsing, projection, and portable editing policy.
//!
//! Functions in this crate inspect Markdown source and plan edits using `hanji-core` types. They do
//! not own editor history, persistence, or platform event handling.

mod command;
mod editing;
mod line;
mod projection;
mod table;

pub use command::{
    MarkdownCommand, MarkdownCommandError, execute_markdown_command, insert_link, toggle_code,
    toggle_emphasis, toggle_strong,
};
pub use editing::{
    ListIndentDirection, TABLE_LINE_BREAK_SOURCE, blockquote_newline_edit_for_line,
    empty_marker_pair_delete_backward_edit, list_indent_edit, list_newline_edit_for_line,
    marker_autocomplete_edit, marker_skip_offset, ordered_list_delimiter, table_cell_at_offset,
    table_cell_line_start_for_offset, table_horizontal_caret_offset, table_line_break_caret_offset,
    table_line_break_delete_edit, table_newline_edit, task_marker_state_char_range,
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
