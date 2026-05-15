# AGENTS.md

This repository is built in English only. Keep code, comments, docs, commit messages, issue templates, and user-facing strings in English unless a future localization system explicitly scopes another language.

## Product North Star

Hanji is a light, local-first Markdown editor. It should feel simple enough for notes, durable enough for plain text writing, and open enough for user plugins.

## Engineering Guidelines

- Prefer small, explicit modules over framework-heavy abstractions.
- Keep the default app fast, offline-capable, and understandable.
- Treat plugin APIs as public contracts. Document them before broadening them.
- Avoid introducing persistent services, telemetry, or network behavior without a design document.
- Keep UI copy calm, brief, and useful.

## Repository Conventions

- Put design documents under `docs/`.
- Put core editor concepts under `docs/design/`.
- Keep the Rust and GPUI architecture direction in `docs/architecture.md` until a dedicated ADR structure exists.
- Keep renderer UI code in `src/renderer/`.
- Keep Tauri app shell and native commands in `src-tauri/`.
- Add tests alongside the feature once test infrastructure is introduced.
- Start commit messages with an English imperative verb and make the scope broad enough to describe the full change.

## Initial Commands

```sh
npm install
npm run dev
npm run typecheck
npm run build
cargo test --workspace --exclude hanji-rust
```
