# Platform Adapters

Status: Current

The portable editor is headless. A platform adapter translates native or browser facilities into source-coordinate editor operations and renders the resulting projection.

## Responsibility Matrix

| Concern | Portable engine | GPUI desktop | Future web adapter |
| --- | --- | --- | --- |
| Source mutation and history | `hanji-editor` | delegates | delegates |
| Markdown editing policy | `hanji-markdown` | delegates | delegates |
| Source projection | `hanji-markdown` | borrows projection | converts to owned snapshot |
| Offset model | UTF-8 source bytes | converts GPUI UTF-16 ranges | converts DOM UTF-16 ranges |
| IME lifecycle | accepts literal replacements | owns marked range and events | owns composition events |
| Rendering | none | GPUI layout and paint | DOM or canvas |
| Clipboard | none | native clipboard | browser clipboard APIs |
| Persistence | none | `hanji-storage` and local files | local storage, IndexedDB, or host app |
| Windows and dialogs | none | GPUI | browser or host application |

## Adapter Contract

An adapter must:

1. Convert untrusted platform offsets into validated source offsets.
2. Preserve anchor/head direction when setting a selection.
3. Use `TextInput::typing` only for interactive typing.
4. Use `TextInput::literal` for paste and IME composition updates.
5. Express shortcuts and controls as logical `Command` values.
6. Use `Update` to synchronize platform state.
7. Keep pixel geometry and platform exceptions outside portable crates.

## Native Adapter

`apps/hanji` is the production adapter. It integrates `DocumentSession` with GPUI window lifecycle, file dialogs, menus, actions, input delivery, clipboard, rendering, scrolling, and hit testing.

It imports selected `hanji-core` offset helpers and `hanji-markdown` projection types because those are adapter concerns. It must not apply core transactions or implement a second copy of Markdown policy.

## Web Boundary

The WebAssembly and JavaScript packages do not exist yet. Their intended package boundary, owned snapshot shape, and implementation order live in [Web Editor](../plans/web-editor.md).

The WebAssembly crate should remain a translation layer. Business rules that can be tested without a browser belong in the existing Rust engine crates.
