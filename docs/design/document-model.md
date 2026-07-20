# Document Model

Status: Current

Markdown text is Hanji's source of truth.

The editor may build parsed trees, rendered runs, block widgets, outlines, and search indexes, but those are derived views. The saved document remains Markdown.

## Source of Truth

- A document is UTF-8 Markdown text.
- The text buffer is the source of truth during editing.
- Formatting commands transform the text buffer.
- Saving writes Markdown back to disk.

## Derived Views

The editor can derive several views from the source:

- A Markdown syntax tree for structure.
- A layout model for visible lines and blocks.
- Line classifications for headings and blockquotes.
- Inline style runs for emphasis, links, and code, with source and marker ranges preserved.
- Block widgets for tables, images, and future interactive surfaces.

Derived views must be disposable. Rebuilding them should not lose document data.

## Raw Markdown

Raw Markdown is not a debug mode. It is a first-class editing mode and a trust mechanism.

## Saving

Editing happens in memory. Saving writes the current Markdown text to disk.

User-visible Markdown files should be saved atomically: write the new contents to a temporary file in the same directory, sync it, rename it over the destination, then best-effort sync the parent directory.

## Document Session

A document session connects an in-memory document to a file path.

It owns the current `Editor`, tracks whether the persisted text is dirty, and saves the current Markdown text through the storage layer. Selection-only changes should not mark the session dirty because they do not change the saved Markdown source. The implemented lifecycle is documented in [Persistence](../architecture/persistence.md).
