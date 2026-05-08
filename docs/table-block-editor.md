# Table Block Editor

Lithe should treat Markdown tables as editable block widgets instead of switching between raw source lines and a rendered preview table.

## Problem

Live preview currently tries to fit two different layouts into the same editor space:

- Source Markdown uses one monospaced line per table row.
- Rendered tables use proportional fonts, borders, padding, and browser table layout.

That mismatch makes the editor height change when the cursor enters or leaves a table. It also pushes CodeMirror decorations toward unsafe multiline replacement behavior.

## Direction

Tables should use a dedicated table block editor:

1. Render table blocks as tables by default.
2. Let users edit cells directly in the table surface.
3. Update the underlying Markdown source after each cell edit.
4. Copy table selections as Markdown source by default.
5. Keep an escape hatch to edit the raw Markdown table.

The stored document remains plain Markdown. The table editor is an interaction layer over that source, not a new file format.

## Current First Pass

The current implementation keeps recognized tables in a rendered table surface, lets existing cells be edited directly, and serializes those edits back to Markdown. Copy events from the table surface write Markdown text to the clipboard. Users can drag across cells to select a rectangular range, copy the selected cells as Markdown, clear selected cells, or delete the table when the full cell range is selected. Row and column insertion controls remain future work.

## Interaction Rules

- A table block keeps a stable block height while editing cells.
- The editor cursor moves to the line after a rendered table instead of staying in hidden Markdown table source.
- Arrow keys move within cell text first, then between cells at cell boundaries.
- Enter inserts a line break inside a cell only when explicitly supported; otherwise it commits the cell edit.
- Tab and Shift+Tab move to the next or previous cell.
- Copying the full table or a selected cell range writes Markdown to the clipboard.
- Pasting Markdown table text into the editor creates a table block.
- Pasting tab-delimited rows inside a table fills cells.

## Implementation Notes

- Avoid plugin decorations that replace ranges spanning line breaks.
- Prefer a StateField-backed block representation or a separate overlay surface that maps table coordinates back to document ranges.
- Keep table parsing and Markdown serialization in small pure functions with scenario tests.
- Keep raw source available so users are never trapped in a rich editing surface.

## Test Scenarios

- Parse a compact GitHub-Flavored Markdown table into headers and rows.
- Serialize edited headers and rows back to Markdown.
- Copying a table block returns Markdown source.
- Editing a cell updates only the owning table range.
- Moving the cursor into and out of a table does not change the block height.
