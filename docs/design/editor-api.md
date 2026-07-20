# Portable Editor API

Status: Accepted and implemented

Hanji needs one source-backed editing engine that can serve both the GPUI desktop app and browser clients compiled through WebAssembly. The portable API should preserve Hanji's editing behavior without exposing GPUI, browser, storage, or generated binding types as part of the editor contract.

## Goals

- Keep Markdown source as the only document source of truth.
- Run the same text and Markdown editing policies in native and web frontends.
- Give frontends one small, platform-independent editor facade.
- Keep source ranges, visible ranges, and offset encodings explicit.
- Keep the Rust engine API separate from the JavaScript package API.
- Allow projection internals to evolve without breaking JavaScript consumers.

## Non-Goals

- Reuse the GPUI renderer in a browser.
- Add browser or native file storage to the editor engine.
- Stabilize the plugin API as part of the web editor work.
- Add multi-cursor behavior in the first portable API.
- Define a reusable DOM editor component in the first WebAssembly package.

The first JavaScript package should be a headless editor engine. A DOM renderer or web component can be built on top of that package later.

## Dependency Direction

```text
apps/hanji (GPUI) -----> hanji-editor -----> hanji-markdown -----> hanji-core
hanji-storage ---------> hanji-editor
hanji-wasm ------------> hanji-editor
@hanji/editor ---------> hanji-wasm
```

Arrows point from a consumer to a dependency. `hanji-editor` must not depend on GPUI, native storage, browser APIs, `wasm-bindgen`, or JavaScript serialization types.

`hanji-storage` remains a native persistence adapter. Its `DocumentSession` owns a `hanji-editor::Editor`, forwards the standard editor operations, and tracks persistence state from `Update`. `hanji-editor` must not depend on `hanji-storage`.

## Crate Responsibilities

### `hanji-core`

`hanji-core` owns UI-independent text primitives:

- UTF-8 source text.
- Grapheme-safe source offsets.
- Selections and transactions.
- Undo and redo history.
- Plain text insertion and deletion commands.
- Conversion helpers needed by platform adapters to translate UTF-16 offsets into validated source offsets.

The canonical internal coordinate remains a UTF-8 byte offset. Platform offsets are untrusted until they have been converted and snapped to a valid grapheme boundary.

### `hanji-markdown`

`hanji-markdown` owns every editing rule that depends on Markdown syntax:

- Line and inline classification.
- Source-backed projection and visible-to-source mapping.
- Formatting commands.
- Blockquote and list continuation on newline.
- List indentation and outdentation.
- Marker wrapping, completion, skipping, and paired deletion.
- Code fence recognition used by both projection and editing.
- Task marker source changes.
- Table source edits and source-aware table navigation.
- Source-aware copy expansion for projected structures such as tables.

Markdown editing helpers should return explicit edit plans or core transactions instead of anonymous tuples of replacement ranges, strings, and selections. Planning an edit should be deterministic and independently testable.

A possible internal planning result is:

```rust
pub enum EditPlan {
    Apply(Transaction),
    SetSelection(Selection),
    NotHandled,
}
```

This planning type does not need to be a stable public contract in the first extraction. It exists to keep syntax policy in `hanji-markdown` and mutation coordination in `hanji-editor`.

### `hanji-editor`

`hanji-editor` is the standard platform-independent facade. It owns a core document and coordinates core commands, Markdown policies, projection, and history.

It should expose:

- Source and selection queries.
- Directional selection updates.
- Platform text replacement without platform event types.
- Logical editor commands.
- Undo and redo.
- A borrowed Rust projection for native renderers.
- Explicit edit outcomes that tell a frontend what changed.

It does not expose its inner `Document`, transactions, core commands, or Markdown command errors. Every mutation must pass through `set_selection`, `replace_text`, or `execute`, so a consumer cannot accidentally bypass Markdown policy.

### `hanji-wasm`

`hanji-wasm` is a thin binding crate with a `cdylib` output. It owns only cross-language concerns:

- `wasm-bindgen` exports.
- JavaScript UTF-16 to Hanji source-offset conversion.
- Stable error-code mapping.
- Conversion from borrowed Rust projections to owned JavaScript snapshots.
- Generated WebAssembly initialization and disposal.

Business rules must not live in `hanji-wasm`. If a behavior needs a Rust unit test without a browser, it belongs in `hanji-core`, `hanji-markdown`, or `hanji-editor`.

### Platform adapters

The GPUI app continues to own:

- Windows, menus, focus, and file dialogs.
- Native input event delivery and marked-text state.
- Clipboard integration.
- GPUI layout, rendering, hit testing, and pixel coordinates.
- Mouse drag thresholds and scroll behavior.
- Native file-session integration.

The JavaScript package or a later web UI package owns:

- WebAssembly initialization.
- DOM input and selection events.
- Clipboard integration through browser APIs.
- DOM or canvas rendering.
- Browser persistence such as local storage or IndexedDB.

## Standard Rust API

The facade stays small and treats the following vocabulary as its Rust contract.

```rust
pub struct Editor {
    // Owns the core document and portable editor state.
}

pub struct TextSelection {
    // Private fields with anchor/head accessors.
}

pub enum TextInputMode {
    Typing,
    Literal,
}

pub struct TextInput {
    // Constructed with typing() or literal(), then refined with
    // replacing() and selecting_after().
}

pub enum Command {
    DeleteBackward,
    DeleteWordBackward,
    DeleteLineBackward,
    DeleteForward,
    InsertNewline,
    Indent,
    Outdent,
    ToggleStrong,
    ToggleEmphasis,
    ToggleCode,
    InsertLink,
    ToggleTaskAt(usize),
    Undo,
    Redo,
}

pub struct Update {
    // Read through text_changed(), selection_changed(),
    // history_changed(), and changed().
}

pub enum Error {
    InvalidRange,
    InvalidBoundary,
    InvalidSelection,
    Internal,
}

impl Editor {
    pub fn new(source: impl Into<String>) -> Self;
    pub fn source(&self) -> &str;
    pub fn selected_source(&self) -> Option<String>;
    pub fn selection(&self) -> TextSelection;
    pub fn set_selection(&mut self, selection: TextSelection) -> Result<Update, Error>;
    pub fn replace_text(&mut self, input: TextInput) -> Result<Update, Error>;
    pub fn execute(&mut self, command: Command) -> Result<Update, Error>;
    pub fn projection(&self) -> MarkdownProjection<'_>;
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
    // Read-only source navigation queries are also exposed here.
}
```

`TextSelection` preserves anchor and head instead of reducing every selection to an ordered range. This lets native and web adapters preserve selection direction while the facade converts the selection to core ranges for edits. The first facade supports one directional selection even though `hanji-core` keeps room for multiple ranges.

`TextInput::range` is an optional absolute source range and defaults to the current selection. `selection_after`, when present, is also expressed in absolute source coordinates after the replacement has been applied. The JavaScript facade mirrors these semantics with UTF-16 offsets.

### Text input modes

Input origin affects Markdown policy:

- `Typing` may run marker completion, marker skipping, or selection wrapping.
- `Literal` inserts the provided source exactly and is used for paste, IME updates, and other explicit source insertion.

The platform adapter owns the IME marked-range lifecycle and sends each composition update as `Literal` input. A separate composition mode should be added only if the engine later owns composition-specific history grouping or policy; the input origin alone is not enough reason to broaden the public API.

Keeping this distinction in the standard API prevents a web frontend from accidentally autocompleting pasted Markdown or intermediate IME text.

### Commands and outcomes

Commands describe user intent rather than keys. A GPUI shortcut and a browser toolbar button should call the same command.

An `Update` reports whether source, selection, or history changed. Frontends can use these flags to decide when to repaint, synchronize a textarea, persist source, or refresh undo controls without comparing whole documents.

Errors use Hanji-owned variants. Core errors, Markdown command errors, GPUI strings, `JsValue`, DOM exceptions, and storage errors do not belong in the facade error type.

### Navigation boundary

The first facade does not need to standardize pixel-based caret navigation. Left, right, up, and down movement in the live preview depends on projection mapping and frontend layout. A platform adapter can resolve the intended visible position and then call `set_selection` with the resulting source position.

Source-only movement helpers, such as grapheme and word boundaries, remain in `hanji-core`. Markdown-specific source navigation, such as stepping across a generated table line-break marker, belongs in `hanji-markdown` and is coordinated by `hanji-editor`.

## Coordinate Contract

The Rust engine uses UTF-8 byte offsets on grapheme boundaries. The JavaScript API uses UTF-16 code-unit offsets because those are the coordinates used by DOM selection and text controls.

The WebAssembly adapter must:

1. Convert incoming UTF-16 offsets to UTF-8 byte offsets.
2. Snap untrusted positions through the core's grapheme-boundary rules.
3. Execute the editor operation in source coordinates.
4. Convert outgoing source and visible ranges back to UTF-16 offsets.

Every JavaScript range must document its coordinate space. Source ranges and visible ranges must remain separate even when both are represented as UTF-16 numbers.

Visible-to-source mapping continues to require explicit boundary affinity. The editor facade should not guess which side of a hidden Markdown marker a visible caret represents.

## Projection Contract

Native Rust renderers can borrow the current `MarkdownProjection<'_>` from the editor. JavaScript consumers must not receive the lifetime-based projection types directly.

`hanji-wasm` should return an owned snapshot with only frontend-relevant data, for example:

```ts
export interface ProjectionSnapshot {
  lines: ProjectedLine[];
}

export interface ProjectedLine {
  sourceRange: TextRange;
  markerRange?: TextRange;
  kind: LineKind;
  visibleText: string;
  segments: ProjectedSegment[];
}

export interface ProjectedSegment {
  visibleRange: TextRange;
  sourceRange: TextRange;
  sourceOuterRange: TextRange;
  kind: SegmentKind;
  style: InlineStyle;
}
```

The snapshot schema is the JavaScript rendering contract. It should not expose private parser state or mirror every Rust projection field. This keeps `hanji-markdown` free to split or optimize its projection implementation without forcing a JavaScript breaking change.

## JavaScript API

The public JavaScript surface should be a handwritten package facade such as `@hanji/editor`, not the raw generated `wasm-bindgen` module.

```ts
const editor = await HanjiEditor.create("# Hanji");

editor.setSelection({ anchor: 7, head: 7 });
editor.replaceText({ text: " notes", mode: "typing" });
editor.execute("toggleStrong");

const source = editor.source;
const projection = editor.getProjection();

editor.undo();
editor.dispose();
```

Initialization may be asynchronous because the WebAssembly module must load. Editing operations should remain synchronous and deterministic after initialization.

The JavaScript package should provide TypeScript declarations, stable error codes, and explicit UTF-16 range documentation. Generated glue and the raw WebAssembly ABI remain package internals.

## Policy Extraction from `apps/hanji`

The original GPUI app contained both platform code and portable editing policy. The initial extraction moves behavior without intentionally changing it.

| Current behavior | Destination |
| --- | --- |
| Blockquote newline continuation and exit | `hanji-markdown` |
| List continuation, exit, indent, and outdent | `hanji-markdown` |
| Marker wrapping, completion, skip, and paired deletion | `hanji-markdown` |
| Code fence detection used by autocomplete | shared `hanji-markdown` syntax helper |
| Task marker source toggle | `hanji-markdown` |
| Table source line break insertion and deletion | `hanji-markdown` |
| Table source-aware caret movement | `hanji-markdown`, coordinated by `hanji-editor` |
| Source-aware copy expansion | `hanji-editor` using projection queries |
| UTF-16 and UTF-8 offset conversion | `hanji-core` helpers used by adapters |
| Directional selection state | `hanji-editor` |
| Drag thresholds, bounds checks, and marker hitboxes | GPUI app |
| Pixel-based vertical and horizontal caret targeting | GPUI app |
| IME marked-range lifecycle | platform adapter |

Pure editing policies should move with their focused tests. GPUI-specific helpers should remain in the app even when they are located beside portable code today.

## Implementation Sequence

### 1. Extract Markdown edit planning (initial extraction complete)

- Add focused `hanji-markdown` modules for input policy, block editing, and table editing.
- Move pure functions and their tests from `apps/hanji` without behavior changes.
- Replace remaining implicit replacement tuples with transactions or explicit edit plans as the policy API matures.
- Share code fence recognition between projection and input policy.

The GPUI app should continue calling the extracted functions directly during this step.

### 2. Add `hanji-editor` (complete)

- Add the facade crate with source, selection, text input, command, history, and projection APIs.
- Preserve directional selection in the facade.
- Move command coordination out of the GPUI app.
- Keep `hanji-storage::DocumentSession` as the mutation-tracking owner of the facade.

### 3. Make GPUI consume the facade (complete for editing commands and text input)

- Route native text input through `Editor::replace_text`.
- Route shortcuts and toolbar actions through logical commands.
- Keep marked text, clipboard, hit testing, layout, and rendering in the app.
- Keep only GPUI-specific selection, hit-testing, clipboard delivery, and layout behavior in `apps/hanji`.

The desktop app should become the first production consumer of the standard facade before a WebAssembly binding is published.

### 4. Add the WebAssembly adapter

- Add `hanji-wasm` with `cdylib` output.
- Map UTF-16 JavaScript ranges to validated engine ranges.
- Add owned projection snapshots and stable error mapping.
- Check `hanji-core`, `hanji-markdown`, `hanji-editor`, and `hanji-wasm` against `wasm32-unknown-unknown` in CI.

### 5. Add the JavaScript package and demo

- Wrap generated bindings in `@hanji/editor`.
- Add TypeScript declarations and package-level tests.
- Add a small local-storage-backed editor to `site/` as the first integration.
- Keep the first package headless; add a reusable DOM surface only after the engine contract is proven.

The initial Rust extraction, facade, and GPUI adoption form one coherent boundary change because the desktop app proves the API. The WebAssembly binding, JavaScript package, and web demo should remain separate follow-up changes.

## API Boundary Rules

- No GPUI, native file-system, DOM, or `wasm-bindgen` types in `hanji-editor`.
- No Markdown-specific behavior in `hanji-core`.
- No business logic in `hanji-wasm`.
- No generated WebAssembly ABI exposed as the documented JavaScript API.
- No implicit offset units in public range types or documentation.
- No JavaScript snapshot tied one-to-one to private Rust projection structs.
- No second implementation of Markdown input policy in a platform adapter.
- No public mutation path that accepts `hanji_core::Document`, `Transaction`, or core commands.
- No duplicate text insertion command alongside `TextInput`.

The Rust and JavaScript APIs are not published yet, so the design should prefer a coherent contract over backward compatibility with the current workspace. Compatibility and semantic-versioning guarantees begin only when an external package is published.

## Deferred Decisions

- JavaScript error-code mapping.
- IME composition history grouping.
- The final owned projection snapshot fields.
- Multi-selection representation in the facade and JavaScript API.
- npm package scope, release automation, and whether Rust and JavaScript package versions stay in lockstep.
- Whether a later reusable web surface is a framework-neutral custom element or a lower-level rendering package.
