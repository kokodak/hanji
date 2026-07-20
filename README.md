# Hanji <sub><sub>한지 韓紙</sub></sub>

### <img width="218" height="63" alt="image" src="https://github.com/user-attachments/assets/3834beb8-19f0-49f5-8c85-775bd3be582a" /> 

Capture the thought.

Hanji is a lightweight, local-first Markdown editor built in Rust. It uses local Markdown files as the source of truth and presents them through a source-backed WYSIWYG view.

The editor is designed for simple writing: open a Markdown file, edit it directly, and keep the saved document readable outside Hanji.

## Getting Started

Rust core checks:

```sh
cargo test --workspace --exclude hanji
```

GPUI desktop app:

```sh
make app
make app FILE=/path/to/note.md
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for issue, pull request, and development guidelines.

## Project Layout

- [docs/](docs/) is the documentation index.
- [docs/architecture/](docs/architecture/) describes current component boundaries and runtime flows.
- [docs/design/](docs/design/) defines durable document and editing semantics.
- [docs/reference/](docs/reference/) records current APIs and supported behavior.
- [docs/plans/](docs/plans/) keeps proposed work separate from implemented architecture.
- [site/](site/) contains the static project website deployed with GitHub Pages.
- [crates/hanji-core](crates/hanji-core/) owns text editing primitives.
- [crates/hanji-markdown](crates/hanji-markdown/) owns Markdown projection and editing policy.
- [crates/hanji-editor](crates/hanji-editor/) is the portable editor facade used by platform frontends.
- [crates/hanji-storage](crates/hanji-storage/) owns local file and document session behavior.
- [crates/hanji-plugin-api](crates/hanji-plugin-api/) is reserved for future plugin contracts.
- [apps/hanji](apps/hanji/) contains the GPUI desktop app.
- [AGENTS.md](AGENTS.md) contains guidance for coding agents working in this repo.

## Documentation

- [Hanji Docs](docs/README.md)
- [Architecture](docs/architecture/README.md)
- [Design Notes](docs/design/README.md)
- [Editor API](docs/reference/editor-api.md)

## License

Hanji is released under the [MIT License](LICENSE).
