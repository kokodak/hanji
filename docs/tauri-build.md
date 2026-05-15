# Tauri Build

Hanji uses Tauri as its desktop app framework. The renderer is built with Vite, then Tauri packages the static renderer output into a native macOS app bundle.

## Prerequisites

- Node.js and npm
- Rust and Cargo
- Tauri CLI through the local npm dependency

Install JavaScript dependencies first:

```sh
npm install
```

## Development

Run the Tauri development app:

```sh
npm run dev
```

This starts the Vite dev server from `beforeDevCommand` and opens the native Tauri shell against `http://127.0.0.1:1420`.

For renderer-only browser testing:

```sh
npm run web:dev
```

## Validation

Run TypeScript checks:

```sh
npm run typecheck
```

Build only the renderer:

```sh
npm run web:build
```

This writes Vite output to `dist/`.

## Build The macOS App

For local app testing, build only the `.app` bundle:

```sh
npx tauri build --bundles app
```

The app is written to:

```text
src-tauri/target/release/bundle/macos/Hanji.app
```

Open it directly:

```sh
open src-tauri/target/release/bundle/macos/Hanji.app
```

## Full Bundle Build

The default production command is:

```sh
npm run build
```

This runs TypeScript checks and then `tauri build`. With the current Tauri configuration, this may try to create every enabled bundle target, including a DMG on macOS.

If DMG packaging fails but `Hanji.app` is created successfully, use the `.app` bundle for local testing and investigate DMG packaging separately.

## Git Policy

Build outputs should not be committed.

Ignored outputs include:

- `dist/`
- `src-tauri/target/`
- `src-tauri/gen/`

Files required to build the app should be committed. For example, `src-tauri/icons/icon.png` is a build input and should stay in Git.

## Current Notes

- The renderer build may warn about a large JavaScript chunk because CodeMirror language support is broad. This is a known optimization target, not a build failure.
- The local app bundle build command is the safest command while the packaging story is still being refined.
- The desktop shell uses a transparent macOS window with a native background effect for local app builds. This enables the lighter glass-like editor chrome, but it relies on Tauri's macOS private API setting and is not App Store compatible as configured.
