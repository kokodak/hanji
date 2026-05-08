# Architecture

Lithe starts as a Tauri application using Vite, TypeScript, and a thin Rust app shell.

## Runtime Shape

- Tauri shell: owns native windows, application lifecycle, and future filesystem access.
- Rust commands: expose narrow native capabilities to the TypeScript UI.
- Renderer process: owns the editor UI, plugin host, and local interaction state.
- Editor core: CodeMirror 6 provides the text editing engine, document state, transactions, keymaps, search, and Markdown syntax support.
- Live preview: lightweight CodeMirror decorations hide Markdown syntax on inactive lines and reveal source syntax on the cursor or hovered line.
- Code highlighting: fenced Markdown code blocks use CodeMirror's broad language registry and warm language parsers during idle time so many languages are available without blocking initial editor paint.
- Table editing: Markdown tables should graduate from live preview decorations to a dedicated editable block surface. See [Table Block Editor](./table-block-editor.md).

## Local-First Model

The initial skeleton keeps draft state in the renderer. The first storage milestone should move document access behind Tauri commands so the renderer never receives unrestricted filesystem power.

Planned document flow:

1. The renderer requests a document action through a typed Tauri command.
2. The Rust shell performs native file access.
3. The renderer receives plain document data and metadata.
4. Autosave stores local draft state without requiring network access.

## Plugin Direction

Plugins should run through explicit capabilities rather than direct access to app internals. Early plugin APIs should be small:

- Read editor state.
- Update editor state.
- Register commands.
- Contribute UI actions.
- Store plugin-scoped settings.

The core app should remain useful with every plugin disabled.

Editor-facing plugin APIs should wrap CodeMirror behavior instead of exposing every CodeMirror primitive at once. This keeps the public Lithe API stable while still allowing deeper editor extensions later.

## Language Boundary

Most product code should remain TypeScript. Rust should stay focused on app-shell responsibilities: filesystem access, native menus and dialogs, secure storage, window lifecycle, and future capability enforcement.
