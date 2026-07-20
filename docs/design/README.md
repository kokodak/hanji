# Design Notes

This folder defines Hanji's durable editing semantics: what the document means, how source maps to the visual surface, and how editing behavior should remain consistent across platforms.

Design documents are not crate maps or roadmaps. Current ownership belongs in [`../architecture/`](../architecture/README.md), exact APIs belong in [`../reference/`](../reference/README.md), and unimplemented work belongs in [`../plans/`](../plans/README.md).

## Belongs Here

- Durable document, selection, coordinate, projection, and editing semantics.
- Behavioral invariants shared by native and future web frontends.
- Vocabulary that should remain useful if crates or modules are renamed.
- Rules that explain what correct editor behavior means.

## Does Not Belong Here

- Current dependency or module maps; put them in `docs/architecture/`.
- Exhaustive current APIs or supported syntax tables; put them in `docs/reference/`.
- Implementation sequences and speculative designs; put them in `docs/plans/`.
- Contributor commands and release procedures; put them in `docs/development/`.

## Contents

- [Document Model](document-model.md)
- [Editor Core](editor-core.md)
- [Coordinate Systems](coordinate-systems.md)
- [Source-Backed WYSIWYG](source-backed-wysiwyg.md)
- [Live Preview](live-preview.md)
- [Editing Policy](editing-policy.md)

## Maintenance Rule

A design note should state invariants and semantics that remain meaningful if modules are renamed. Link to reference documentation for exhaustive current behavior and to architecture documentation for ownership.
