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

### Transaction

A transaction is one intentional edit. It can include text changes, selection changes, and metadata needed for undo.

### Selection

A selection identifies one or more ranges in the text buffer. The first Rust editor can start with a single selection, but the core should not assume the UI can never grow multi-cursor editing.

### Command

A command is a named editing operation such as insert text, toggle emphasis, create heading, or split list item.

Commands should operate on core state and return an outcome the UI can render.

### Projection

A projection is a visual interpretation of the Markdown source. WYSIWYG editing changes the projection, not the source of truth.

## Boundary Rule

No GPUI types should enter the editor core. If the boundary feels awkward, define a small Hanji-owned type instead.
