# Han-ji

Capture the thought.

Hanji is a light, local-first Markdown text editor.

## Principles

- Light by default: fast startup, minimal chrome, and a quiet editing surface.
- Local first: offline editing is the core workflow, not a fallback.
- Markdown native: plain text files should remain readable outside the app.
- Extensible by design: plugins should be easy to author, inspect, install, and remove.
- Collaboration when online: real-time editing should feel optional, direct, and unobtrusive.

## Getting Started

Rust core checks:

```sh
cargo test --workspace --exclude hanji-rust
```

GPUI desktop app:

```sh
make app
make app FILE=/path/to/note.md
```

Current TypeScript desktop app:

```sh
npm install
npm run dev
```

For product philosophy and editor design notes, see [Hanji Docs](docs/README.md).

## Project Layout

```text
docs/                 Product and engineering design notes
docs/design/          Core editor concepts and design vocabulary
crates/               Rust editor crates
apps/                 Rust desktop applications
src/renderer/         TypeScript editor UI
src-tauri/            Tauri app shell and native capabilities
AGENTS.md            Guidance for coding agents working in this repo
```
