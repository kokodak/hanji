# Document Model

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
- Inline style runs for emphasis, links, code, and headings, with source and marker ranges preserved.
- Block widgets for tables, images, and future interactive surfaces.

Derived views must be disposable. Rebuilding them should not lose document data.

## Spaces

A Space is a local folder that contains user-visible Markdown documents.

Hanji-specific metadata should live beside the documents in a clearly reserved folder, but documents should remain useful if that metadata is deleted.

## Raw Markdown

Raw Markdown is not a debug mode. It is a first-class editing mode and a trust mechanism.

## Saving

Editing happens in memory. Saving writes the current Markdown text to disk.

User-visible Markdown files should be saved atomically: write the new contents to a temporary file in the same directory, sync it, rename it over the destination, then best-effort sync the parent directory.

## Document Session

A document session connects an in-memory document to a file path.

It owns the current `Document`, tracks whether the persisted text is dirty, and saves the current Markdown text through the storage layer. Selection-only changes should not mark the session dirty because they do not change the saved Markdown source.
