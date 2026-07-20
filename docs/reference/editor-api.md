# Editor API

Status: Current

Stability: Unpublished workspace contract

`hanji-editor` is the standard platform-independent facade for Hanji editing. Native and future WebAssembly adapters use it for every document mutation.

The contract favors a coherent boundary over migration compatibility while no external package has been published. Semantic-versioning commitments begin when the crate or a derived package is released for external consumers.

## Public Vocabulary

The crate exports:

```rust
pub use hanji_core::{TextPosition, TextRange};

pub struct Editor;
pub struct TextSelection;
pub struct TextInput;
pub enum TextInputMode;
pub enum Command;
pub struct Update;
pub enum Error;
```

Facade-owned struct fields are private. Consumers use constructors and accessors so internal representation can evolve without exposing core documents or transactions. The re-exported core value types currently expose their range and position fields.

## Creating and Reading an Editor

```rust
let editor = Editor::new("# Hanji\n");

assert_eq!(editor.source(), "# Hanji\n");
assert_eq!(editor.selection(), TextSelection::caret(0));
```

Primary read methods:

```rust
pub fn source(&self) -> &str;
pub fn len(&self) -> usize;
pub fn is_empty(&self) -> bool;
pub fn selected_source(&self) -> Option<&str>;
pub const fn selection(&self) -> TextSelection;
pub fn projection(&self) -> MarkdownProjection<'_>;
pub fn can_undo(&self) -> bool;
pub fn can_redo(&self) -> bool;
```

The borrowed projection is a Rust renderer contract. It is not suitable as a direct JavaScript binding because its data borrows the editor source.

## Mutation Methods

There are exactly three public mutation paths:

```rust
pub fn set_selection(
    &mut self,
    selection: TextSelection,
) -> Result<Update, Error>;

pub fn replace_text(
    &mut self,
    input: TextInput,
) -> Result<Update, Error>;

pub fn execute(
    &mut self,
    command: Command,
) -> Result<Update, Error>;
```

The facade does not expose its core `Document`, accept a `Transaction`, provide a mutable-document callback, or offer an insertion command that competes with `TextInput`.

## Selection

`TextSelection` represents one directional source selection:

```rust
let forward = TextSelection::new(2, 8);
assert_eq!(forward.anchor(), 2);
assert_eq!(forward.head(), 8);
assert_eq!(forward.range(), TextRange::new(2, 8));

let caret = TextSelection::caret(5);
```

`anchor` and `head` are UTF-8 source byte offsets on grapheme boundaries. `range()` returns their ordered half-open range. Selection direction is retained even though edits operate on an ordered range.

## Text Input

Text input is created according to origin:

```rust
let typing = TextInput::typing("*");
let paste = TextInput::literal("**source**");

let replacement = TextInput::typing("Hanji")
    .replacing(TextRange::new(0, 5))
    .selecting_after(TextSelection::caret(5));
```

- `Typing` may run marker completion, wrapping, or skipping policy.
- `Literal` inserts the supplied source exactly and is intended for paste and IME updates.
- Without `replacing`, input replaces the current selection.
- Without `selecting_after`, the caret is placed after inserted source.
- An explicit selection-after is expressed in source coordinates after replacement.

## Commands

`Command` describes logical intent:

| Command | Meaning |
| --- | --- |
| `DeleteBackward` | Delete selection or previous grapheme with Markdown-aware boundaries. |
| `DeleteWordBackward` | Delete selection or previous word. |
| `DeleteLineBackward` | Delete selection or source to the current line start. |
| `DeleteForward` | Delete selection or next grapheme. |
| `InsertNewline` | Apply blockquote, list, table, or plain newline policy. |
| `Indent` / `Outdent` | Change indentation for selected list lines. |
| `ToggleStrong` | Toggle strong formatting. |
| `ToggleEmphasis` | Toggle emphasis formatting. |
| `ToggleCode` | Toggle inline code formatting. |
| `InsertLink` | Insert a link or select an existing link destination. |
| `ToggleTaskAt(offset)` | Toggle the task marker at a source offset. |
| `Undo` / `Redo` | Move through editor history. |

Keys, menu items, and toolbar controls are adapter concerns and map to these same commands.

## Updates

Every successful operation returns an `Update`:

```rust
update.text_changed();
update.selection_changed();
update.history_changed();
update.changed();
```

Success does not imply a change. A deletion at the beginning of an empty selection can return an unchanged update. Consumers should use these flags instead of comparing full source snapshots.

## Errors

The facade exposes editor-owned errors only:

```rust
pub enum Error {
    InvalidRange,
    InvalidBoundary,
    InvalidSelection,
    Internal,
}
```

Core edit errors and Markdown command errors are mapped internally. Storage I/O errors and platform exceptions remain separate.

## Source Queries

The facade exposes read-only navigation required by platform adapters:

```rust
pub fn line_count(&self) -> usize;
pub fn line_range(&self, line_index: usize) -> Option<TextRange>;
pub fn line_index_at_offset(&self, offset: usize) -> Result<usize, Error>;
pub fn position_at_offset(&self, offset: usize) -> Result<TextPosition, Error>;
pub fn offset_at_position(&self, position: TextPosition) -> Result<usize, Error>;
pub fn nearest_grapheme_offset(&self, offset: usize) -> Result<usize, Error>;
pub fn previous_grapheme_offset(&self, offset: usize) -> Result<Option<usize>, Error>;
pub fn next_grapheme_offset(&self, offset: usize) -> Result<Option<usize>, Error>;
pub fn previous_word_offset(&self, offset: usize) -> Result<Option<usize>, Error>;
pub fn next_word_offset(&self, offset: usize) -> Result<Option<usize>, Error>;
pub fn word_range_at_offset(&self, offset: usize) -> Result<Option<TextRange>, Error>;
```

These methods use the source coordinate contract in [Coordinate Systems](../design/coordinate-systems.md). Pixel-based movement is deliberately absent because it depends on frontend layout.

## Boundary Rules

- No GPUI, filesystem, DOM, or `wasm-bindgen` types.
- No public mutable access to the core document.
- No raw transactions or core commands as facade input.
- No second implementation of Markdown policy in an adapter.
- No implicit range units.
- No lower-layer error types in the public error contract.

Architecture ownership is documented in [Crate Boundaries](../architecture/crate-boundaries.md), and future JavaScript packaging is tracked in [Web Editor](../plans/web-editor.md).
