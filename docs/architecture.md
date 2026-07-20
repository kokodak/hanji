# Architecture

Hanji is a Rust WYSIWYG Markdown editor built around small crates with clear ownership. GPUI is the native UI direction, but the editor core should not depend on GPUI.

Shared product decisions live in `docs/`, outside implementation-specific modules.

## Rust Workspace

```text
crates/
  hanji-core/       Text buffer, selections, transactions, undo, commands
  hanji-markdown/   Markdown parsing, projection, source mapping, editing policy
  hanji-editor/     Portable editor facade coordinating core and Markdown behavior
  hanji-storage/    Native file persistence and document sessions
  hanji-plugin-api/ Future public plugin contracts

apps/
  hanji/            GPUI desktop application
```

This structure is intentionally small. Crates can stay thin until their boundaries become real.

## Core Boundaries

- `hanji-core` owns syntax-agnostic text primitives and must not import Markdown or UI framework types.
- `hanji-markdown` owns syntax-dependent projection and edit planning while treating Markdown text as the source of truth.
- `hanji-editor` owns the standard editing workflow and is the only normal entry point for platform text input and logical commands.
- `hanji-storage` owns native file persistence and delegates document editing to `hanji-editor`.
- `apps/hanji` translates GPUI input, rendering, and window events into facade calls.

The mutation boundary is strict: consumers cannot access a mutable core `Document` or submit raw transactions through `hanji-editor` or `hanji-storage`. Text changes use `TextInput`, logical operations use `Command`, and every operation returns `Update`.

## Portable Editor Direction

Hanji exposes one platform-independent editor facade for native and future web frontends. The `hanji-editor` crate coordinates core text editing, Markdown input policy, projection, selection, and history without depending on GPUI, storage, browser APIs, or WebAssembly binding types.

The GPUI app consumes that facade, and a future `hanji-wasm` adapter should do the same. Platform adapters continue to own input delivery, rendering, hit testing, clipboard integration, and persistence. See [Portable Editor API](design/editor-api.md) for the boundary and implementation sequence.

## Markdown Crate Boundaries

`hanji-markdown` owns Markdown-specific behavior while keeping the Markdown source as the source of truth. It should not import GPUI or storage types.

- `line` classifies Markdown source lines into semantic block-like roles.
- `command` turns Markdown formatting actions into core transactions.
- `editing` plans portable Markdown-aware typing, newline, list, task, and table behavior.
- `projection` builds source-backed views for WYSIWYG rendering, preserving source ranges so the raw Markdown text remains recoverable.

## WYSIWYG Strategy

Hanji should use source-backed WYSIWYG editing. The Markdown file is the source of truth, and the visual editor is a projection over that text.

This means:

- Markdown remains readable outside Hanji.
- Formatting commands edit Markdown text, not a hidden rich document model.
- The raw Markdown view remains available as a first-class escape hatch.
- Block widgets are derived from source ranges and can always be serialized back to text.

## First Spike

The first GPUI spike proved the smallest useful loop:

- Open one Markdown file.
- Render plain paragraphs and headings.
- Move a caret through the text.
- Insert and delete text.
- Save the file back as Markdown.

Anything beyond that should wait until the core edit loop feels trustworthy.
