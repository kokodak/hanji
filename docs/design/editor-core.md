# Editor Core

The editor core owns text editing behavior independent of the UI framework.

GPUI should handle windows, input delivery, rendering, and platform integration. The core should handle document state, edits, selections, commands, and undo history.

## Responsibilities

- Store the text buffer.
- Track selections and carets.
- Apply transactions.
- Maintain undo and redo history.
- Run editor commands.
- Expose document snapshots to the UI.

## Non-Responsibilities

- Window management.
- GPU rendering.
- Native menus.
- File dialogs.
- Plugin sandboxing.

## Core Concepts

### Text Buffer

The text buffer stores Markdown source. It should support efficient insertions, deletions, line lookup, and source range mapping.

Hanji currently uses byte ranges that must fall on UTF-8 character boundaries. This keeps source mapping direct while still rejecting invalid edits inside multi-byte characters.

### Transaction

A transaction is one intentional edit. It can include text changes, selection changes, and metadata needed for undo.

Transactions are applied atomically: text edits and the resulting selection must all validate before the document state changes.

### Selection

A selection identifies one or more ranges in the text buffer. The first Rust editor can start with a single selection, but the core should not assume the UI can never grow multi-cursor editing.

The current core supports a collection of ranges with one primary range. The UI can use a single caret today without forcing that assumption into the core type.

### History

Undo and redo belong to the document state. The first implementation stores whole text snapshots so behavior stays simple and trustworthy before compact history storage is needed.

### Command

A command is a named editing operation such as insert text, toggle emphasis, create heading, or split list item.

Commands should operate on core state and return an outcome the UI can render.

### Projection

A projection is a visual interpretation of the Markdown source. WYSIWYG editing changes the projection, not the source of truth.

## Boundary Rule

No GPUI types should enter the editor core. If the boundary feels awkward, define a small Hanji-owned type instead.
