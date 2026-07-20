# Editor Core

Status: Current

The editor core owns text editing behavior independent of the UI framework.

GPUI should handle windows, input delivery, rendering, and platform integration. The core should handle document state, edits, selections, commands, and undo history.

## Responsibilities

- Store the text buffer.
- Track selections and carets.
- Apply transactions.
- Maintain undo and redo history.
- Run editor commands.
- Keep editing ranges and caret movement on user-visible text boundaries.
- Expose source and navigation queries to higher layers.

## Non-Responsibilities

- Window management.
- GPU rendering.
- Native menus.
- File dialogs.
- Plugin sandboxing.

## Core Concepts

### Text Buffer

The text buffer stores Markdown source. It should support efficient insertions, deletions, line lookup, and source range mapping.

Hanji currently uses byte ranges that must fall on Unicode grapheme cluster boundaries. This keeps source mapping direct while still rejecting invalid edits inside multi-byte characters, combined emoji, and other user-visible characters made from multiple Unicode scalar values.

The buffer also keeps a line index of byte offsets where each line starts. This gives the UI a cheap way to ask which line contains an offset or which source range belongs to a line, without threading GPUI layout types into the core.

`TextPosition` represents a source position as a line plus a grapheme column. The core can convert between `TextPosition` and byte offsets so UI code can talk in line-oriented terms while transactions still edit precise UTF-8 ranges.

### Unicode Boundaries

The source text remains UTF-8 Markdown, and `TextRange` continues to store byte offsets. Those offsets are valid edit positions only when they are also Unicode grapheme cluster boundaries.

The core owns this rule because every UI surface should agree on what a user-visible character is. A flag emoji such as `🇰🇷`, a combined emoji, or a character plus combining marks should move, select, and delete as one visible unit.

Core APIs should expose previous, next, and nearest grapheme boundary helpers for UI adapters. Platform input APIs may report UTF-16 offsets or hit-tested byte indexes that land inside a grapheme cluster; adapters should convert and snap those positions through the core before calling the editor facade. See [Coordinate Systems](coordinate-systems.md).

Core should also expose word-boundary movement for accelerated keyboard navigation. Word movement must still return grapheme boundaries, skip punctuation and Markdown marker characters around words, and avoid splitting emoji or combined Unicode clusters.

### Transaction

A transaction is one intentional edit. It can include text changes, selection changes, and metadata needed for undo.

Transactions are applied atomically: text edits and the resulting selection must all validate before the document state changes.

### Selection

A selection identifies one or more ranges in the text buffer. The first Rust editor can start with a single selection, but the core should not assume the UI can never grow multi-cursor editing.

The current core supports a collection of ranges with one primary range. The UI can use a single caret today without forcing that assumption into the core type.

### History

Undo and redo belong to the document state. The first implementation stores whole text snapshots so behavior stays simple and trustworthy before compact history storage is needed.

### Command

A core command is a named syntax-agnostic editing operation such as inserting or deleting plain text.

Commands should operate on core state and return an outcome the UI can render.

The current core command layer covers plain text insertion and deletion primitives. Deletion commands operate on grapheme clusters, not Unicode scalar values. Markdown-specific commands belong in `hanji-markdown`, where they can build core transactions without making the core depend on Markdown syntax. Platform-facing commands belong to `hanji-editor`, which prevents adapters from bypassing policy.

## Boundary Rule

No Markdown, storage, GPUI, DOM, or WebAssembly binding types should enter the editor core. If the boundary feels awkward, define a small Hanji-owned type in the narrowest owning layer.

Byte offsets from UI adapters should be treated as untrusted until the core validates or snaps them to a grapheme boundary.
