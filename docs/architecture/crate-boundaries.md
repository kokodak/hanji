# Crate Boundaries

Status: Current

Hanji uses crates as ownership boundaries rather than as packaging for its own sake. Each crate has one reason to change and an explicit list of concepts it must not absorb.

## Dependency Direction

```text
hanji-storage ──> hanji-editor ──> hanji-markdown ──> hanji-core
                         └──────────────────────────> hanji-core

apps/hanji ──> hanji-storage, hanji-editor, hanji-markdown, hanji-core, gpui
```

Dependencies point toward more portable and less platform-specific code. A lower layer must not import a higher layer.

## Ownership Matrix

| Component | Owns | May depend on | Must not own |
| --- | --- | --- | --- |
| `hanji-core` | UTF-8 text buffer, ranges, positions, selections, transactions, history, plain-text commands, offset conversion | standard library, Unicode helpers | Markdown syntax, persistence, UI events, rendering |
| `hanji-markdown` | line classification, inline parsing, projection, source mapping, formatting commands, syntax-aware edit planning | `hanji-core` | editor lifetime, history ownership, files, platform events |
| `hanji-editor` | portable mutation facade, directional selection, policy coordination, stable outcomes and errors | `hanji-core`, `hanji-markdown` | GPUI, storage, browser APIs, generated bindings |
| `hanji-storage` | local file I/O, atomic writes, path and saved-state tracking | `hanji-editor` | editing policy, rendering, dialogs |
| `hanji-plugin-api` | future public plugin contracts | nothing yet | internal convenience types before a contract is designed |
| `apps/hanji` | windows, menus, native input, IME state, clipboard, file dialogs, layout, hit testing, painting | all runtime crates and GPUI | portable editing rules |
| `site` | static public website | browser-native HTML and CSS | editor engine behavior until a web adapter exists |

## `hanji-core`

Modules:

- `text`: buffer, edits, byte ranges, line index, grapheme and word navigation.
- `selection`: core selection representation and validation.
- `transaction`: one or more text edits plus the resulting selection.
- `document`: buffer, selection, undo, and redo coordination.
- `command`: syntax-agnostic deletion and insertion commands.
- `encoding`: validated UTF-8 and UTF-16 conversion helpers for adapters.

The crate may expose low-level types because it is also an engine crate, but frontend code must not use those types to bypass `hanji-editor` mutation policy.

## `hanji-markdown`

Modules:

- `line`: Markdown line classification and marker discovery.
- `projection`: source-backed lines, segments, inline styles, and visible/source mapping.
- `command`: formatting operations such as strong, emphasis, code, and links.
- `editing`: lists, blockquotes, marker pairs, tasks, table editing, and navigation policy.
- `table`: table parsing used by projection and editing.

Functions in this crate inspect source and return deterministic plans or transactions. They do not retain document state.

## `hanji-editor`

Modules are organized by public concept:

- `editor`: owns the core document and coordinates all operations.
- `input`: typing and literal replacement requests.
- `command`: logical editor intentions.
- `selection`: one directional public selection.
- `update`: observable operation outcomes.
- `error`: editor-owned public failures.

The only public mutation methods are `set_selection`, `replace_text`, and `execute`. `Document`, `Transaction`, core commands, and Markdown errors are not re-exported.

## `hanji-storage`

`DocumentSession` owns an `Editor`, a file path, a saved source snapshot, and revision counters. It forwards the three editor mutation methods and tracks source changes from `Update`.

Storage exposes its editor read-only. There is no mutable editor accessor because dirty tracking must not be bypassed.

## Boundary Tests

Focused unit tests stay beside the layer that owns the behavior:

- buffer and transaction invariants in `hanji-core`;
- Markdown syntax and edit cases in `hanji-markdown`;
- end-to-end portable operations in `hanji-editor`;
- persistence and dirty-state behavior in `hanji-storage`;
- layout, hit testing, input delivery, and window behavior in `apps/hanji`.

When a test needs GPUI only to exercise a pure source edit, that behavior is in the wrong layer.
