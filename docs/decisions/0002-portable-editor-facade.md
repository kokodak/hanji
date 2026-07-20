# 0002: Use a Portable Editor Facade

Status: Accepted

## Context

The original GPUI app contained both platform event handling and Markdown editing policy. A future browser editor would either duplicate that behavior or require a shared engine boundary.

Exposing the low-level core document or raw transactions would share primitives but still allow frontends to bypass syntax-aware policy and history coordination.

## Decision

`hanji-editor` is the single platform-independent mutation facade. It owns a core document and coordinates `hanji-core` with `hanji-markdown`.

Frontends mutate through `set_selection`, `replace_text`, or `execute`. The facade does not expose a mutable core document, raw transactions, core commands, or lower-layer error types.

## Consequences

- GPUI and future WebAssembly adapters share one editing policy.
- Text insertion is distinct from logical commands so typing origin remains explicit.
- Storage forwards facade operations and can reliably observe `Update`.
- Platform rendering and persistence remain outside the engine.
- The facade must expose sufficient read-only navigation and projection queries.
- Public compatibility commitments begin when an external package is published; until then the boundary can be corrected without migration shims.

## Living Documentation

- [Crate Boundaries](../architecture/crate-boundaries.md)
- [Editing Runtime](../architecture/editing-runtime.md)
- [Editor API](../reference/editor-api.md)
- [Web Editor](../plans/web-editor.md)
