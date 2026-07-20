# 0001: Keep Markdown as the Source of Truth

Status: Accepted

## Context

A visual Markdown editor can either store a separate rich document model and export Markdown, or treat Markdown text as the document and derive its visual representation.

A separate model can simplify some widgets, but it creates synchronization, round-trip, and portability risks. Hanji's product direction prioritizes durable local files that remain useful outside the app.

## Decision

Hanji stores and edits UTF-8 Markdown as the only durable document model. WYSIWYG presentation, parsed structure, layout, and widgets are derived projections with source mappings.

## Consequences

- Saving is direct Markdown persistence rather than export.
- Formatting commands and widgets must produce explicit source edits.
- Projection must retain enough source mapping for selection and mutation.
- Unsupported syntax remains editable source.
- Raw Markdown is a first-class trust surface.
- Rich features are constrained by reliable Markdown round-tripping.

## Living Documentation

- [Document Model](../design/document-model.md)
- [Source-Backed WYSIWYG](../design/source-backed-wysiwyg.md)
- [Persistence](../architecture/persistence.md)
