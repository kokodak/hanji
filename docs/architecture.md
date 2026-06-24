# Architecture

Hanji is moving toward a pure Rust WYSIWYG Markdown editor while keeping the existing TypeScript and Tauri implementation available as a working track.

The Rust track should be built around small crates with clear ownership. GPUI is the chosen UI direction for the first native editor spike, but the editor core should not depend on GPUI.

## Tracks

- The TypeScript and Tauri track remains useful for current experiments and reference behavior.
- The Rust and GPUI track becomes the place where the long-term editor engine is designed.
- Shared product decisions live in `docs/`, not inside either implementation track.

## Rust Workspace

```text
crates/
  hanji-core/       Text buffer, selections, transactions, undo, commands
  hanji-markdown/   Markdown parsing, source mapping, projection, formatting commands
  hanji-storage/    Local spaces, files, autosave, app metadata
  hanji-plugin-api/ Future public plugin contracts

apps/
  hanji-rust/       GPUI desktop application
```

This structure is intentionally small. Crates can stay thin until their boundaries become real.

## Core Boundaries

- `hanji-core` owns editing behavior and must not import UI framework types.
- `hanji-markdown` treats Markdown text as the source of truth.
- `hanji-storage` keeps documents visible as normal files and keeps app metadata separate.
- `apps/hanji-rust` translates GPUI input, rendering, and window events into core commands.

## Markdown Crate Boundaries

`hanji-markdown` owns Markdown-specific behavior while keeping the Markdown source as the source of truth. It should not import GPUI or storage types.

- `line` classifies Markdown source lines into semantic block-like roles.
- `command` turns Markdown formatting actions into core transactions.
- `projection` builds source-backed views for WYSIWYG rendering, preserving source ranges so the raw Markdown text remains recoverable.

## WYSIWYG Strategy

Hanji should use source-backed WYSIWYG editing. The Markdown file is the source of truth, and the visual editor is a projection over that text.

This means:

- Markdown remains readable outside Hanji.
- Formatting commands edit Markdown text, not a hidden rich document model.
- The raw Markdown view remains available as a first-class escape hatch.
- Block widgets are derived from source ranges and can always be serialized back to text.

## First Spike

The first GPUI spike should prove the smallest useful loop:

- Open one Markdown file.
- Render plain paragraphs and headings.
- Move a caret through the text.
- Insert and delete text.
- Save the file back as Markdown.

Anything beyond that should wait until the core edit loop feels trustworthy.
