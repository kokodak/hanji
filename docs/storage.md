# Storage

Hanji is local-first. Documents should live as Markdown files on the local file system and remain easy to inspect, sync, back up, and edit with other tools.

## Terminology

A **Space** is a local folder where Hanji keeps user-visible Markdown documents.

## Current Implementation

The first desktop storage path is a user-visible folder in Documents:

```text
~/Documents/Hanji/
  default.md
  .hanji/
```

The note file is visible as normal Markdown:

```text
~/Documents/Hanji/default.md
```

The hidden `.hanji` folder is reserved for Hanji-specific metadata, such as future settings, plugin data, indexes, and workspace state.

## Native File Access

The renderer does not use browser storage for documents. It calls Tauri commands, and Rust reads or writes the Markdown file:

- `load_space`
- `open_space`
- `create_note`
- `read_note`
- `write_note`
- `delete_note`

This keeps the desktop app's document model tied to the file system instead of a WebView storage origin.

## Direction

The current `~/Documents/Hanji/default.md` file is the first step toward a folder-based Space.

The likely long-term structure is:

```text
Selected Hanji Space/
  Notes/
    Example.md
  Attachments/
    image.png
  .hanji/
    settings.json
    plugins/
    workspace.json
```

Markdown documents should stay visible. Hanji-specific state should stay inside `.hanji`.

## Git Policy

User documents and generated app data should not be committed to this repository.

Source files that define storage behavior should be committed:

- Tauri commands
- renderer storage adapters
- storage design documents
