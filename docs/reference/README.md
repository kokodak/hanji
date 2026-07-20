# Reference

Reference documents describe interfaces and behavior available in the current repository. They are more exact and implementation-adjacent than design documents.

## Belongs Here

- Exact public APIs and value semantics available now.
- Current supported Markdown syntax and user-visible behavior.
- Current shortcuts, commands, configuration, or operational facts.
- Tables and examples that should change whenever implementation changes.

## Does Not Belong Here

- Why the system is divided into its current components; put it in `docs/architecture/`.
- Durable behavioral principles; put them in `docs/design/`.
- Proposed APIs or packages; put them in `docs/plans/`.
- Historical tradeoffs; put them in `docs/decisions/`.

## Contents

- [Editor API](editor-api.md): the current portable Rust facade.
- [Markdown Support](markdown-support.md): syntax, preview, and interaction currently implemented.
- [Keyboard Shortcuts](keyboard-shortcuts.md): current GPUI key bindings.

## Maintenance Rule

Reference documents must change with the code they describe. Future or proposed surfaces belong in [`../plans/`](../plans/README.md). Mark unpublished contracts explicitly without presenting them as stable external APIs.
