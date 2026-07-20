# Development

This directory contains contributor workflows. Product semantics and architecture belong elsewhere; these documents explain how to build, verify, publish, and maintain the repository.

## Belongs Here

- Local setup, build, test, lint, packaging, and release procedures.
- CI and deployment workflows.
- Documentation and website maintenance instructions.
- Commands contributors run against the repository.

## Does Not Belong Here

- Product or editor semantics; put them in `docs/design/`.
- Runtime component ownership; put it in `docs/architecture/`.
- User-facing API reference; put it in `docs/reference/`.
- Unimplemented engineering proposals; put them in `docs/plans/`.

## Prerequisites

- A current Rust toolchain compatible with edition 2024.
- Python 3 for documentation validation.
- macOS and Xcode for the GPUI desktop app.
- The Metal compiler or Xcode Metal Toolchain for GPUI shader compilation.

## Common Commands

```sh
make app
make app FILE=/path/to/note.md
make check-app
make check-docs
make test
cargo test -p hanji
cargo fmt --all -- --check
```

`make test` checks all workspace crates except the GPUI app. `make check-app` compiles the native application with the available Metal toolchain. `cargo test -p hanji` runs app-level tests.

## Check Scope

| Change | Minimum focused check |
| --- | --- |
| `hanji-core` | `cargo test -p hanji-core` |
| `hanji-markdown` | `cargo test -p hanji-markdown` |
| `hanji-editor` | `cargo test -p hanji-editor` |
| `hanji-storage` | `cargo test -p hanji-storage` |
| GPUI app | `make check-app` and `cargo test -p hanji` |
| Documentation | `make check-docs` |
| Formatting | `cargo fmt --all -- --check` |
| Portable engine | `cargo check --target wasm32-unknown-unknown -p hanji-core -p hanji-markdown -p hanji-editor` |

Run broader checks when a change crosses crate boundaries.

## Documentation Workflow

- Update `docs/architecture/` when ownership or dependency direction changes.
- Update `docs/design/` when editing semantics or invariants change.
- Update `docs/reference/` when public APIs, shortcuts, or supported behavior change.
- Put unimplemented proposals in `docs/plans/`.
- Add a decision record when a durable architectural tradeoff needs history.
- Keep links relative so GitHub renders the docs from forks and branches.
- Follow the folder contracts, status values, and templates in the [Documentation Guide](../documentation-guide.md).

## Contents

- [Website](website.md): local preview and GitHub Pages deployment.
- [Releasing](releasing.md): versioning, packaging, tags, and GitHub Releases.
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md): issue, pull request, review, and AI-assistance policy.

## Maintenance Rule

Commands in development documents must exist and should be exercised before being described as working. Keep environment requirements explicit and update CI documentation with workflow changes.
