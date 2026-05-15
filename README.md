# Hanji

Capture the thought.

Hanji is a light, local-first Markdown text editor.

## Principles

- Light by default: fast startup, minimal chrome, and a quiet editing surface.
- Local first: offline editing is the core workflow, not a fallback.
- Markdown native: plain text files should remain readable outside the app.
- Extensible by design: plugins should be easy to author, inspect, install, and remove.
- Collaboration when online: real-time editing should feel optional, direct, and unobtrusive.

## Getting Started

```sh
npm install
npm run dev
```

For local desktop builds, see [Tauri Build](docs/tauri-build.md).

For local file storage notes, see [Storage](docs/storage.md).

## Project Layout

```text
docs/                 Product and engineering design notes
src/renderer/         TypeScript editor UI
src-tauri/            Tauri app shell and native capabilities
AGENTS.md            Guidance for coding agents working in this repo
```
