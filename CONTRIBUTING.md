# Contributing to Hanji

Thank you for contributing to Hanji.

Hanji is a light, local-first Markdown editor. Contributions are most helpful when they keep the app simple, fast, source-backed, and understandable.

## Issue Guidelines

Open an issue before starting work when the change affects product behavior, Markdown rendering, editing policy, file storage, plugin contracts, releases, or performance.

Use a pull request directly for small fixes, documentation edits, clear bugs, or follow-up changes that already have enough context.

Good issues usually include:

- A short summary.
- Why the change matters.
- Steps to reproduce, for bugs.
- Markdown samples, screenshots, or screen recordings when the behavior is visual.
- Any known edge cases.

## Pull Request Guidelines

Keep pull requests limited to one user-visible change or one cleanup. Small PRs are easier to review and easier to revert if something goes wrong.

Each PR should include:

- What changed.
- Why it changed.
- The related issue, when one exists.
- Tests that were added or updated.
- Screenshots or screen recordings for visible UI changes.
- Follow-up work that should not block the PR.

AI-assisted contributions are welcome, but the contributor is responsible for understanding, reviewing, and testing the result before submitting it.

## Development setup

Run the app:

```sh
make app
```

Run the app with a file:

```sh
make app FILE=/path/to/note.md
```

Run the main checks:

```sh
make check-app
cargo test --workspace --exclude hanji
cargo test -p hanji
```

If you skip a relevant check, explain why in the pull request.

## Project shape

The main Rust workspace is split by responsibility:

- `crates/hanji-core`: text buffer, selections, transactions, undo, and core commands.
- `crates/hanji-markdown`: Markdown parsing, source mapping, projection, and formatting commands.
- `crates/hanji-storage`: local files and document sessions.
- `crates/hanji-plugin-api`: future public plugin contracts.
- `apps/hanji`: the GPUI desktop app.

Keep core editor logic independent from GPUI. Keep Markdown files as the source of truth. Avoid adding persistent services, telemetry, or network behavior without a design discussion first.

## Style

- Use English for code, comments, docs, commit messages, templates, and user-facing strings.
- Prefer small, explicit modules over broad abstractions.
- Keep UI copy calm, brief, and useful.
- Add tests alongside behavior changes when practical.
- Put design notes under `docs/`, and core editor concepts under `docs/design/`.

## Review

Review is a conversation. Maintainers may ask for smaller scope, clearer tests, or a short design note before merging. That is normal, especially while Hanji is still early.
