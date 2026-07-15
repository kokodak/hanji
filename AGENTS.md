# AGENTS.md

## Project Overview

Hanji is a lightweight, local-first Markdown editor built in Rust. It keeps local Markdown files as the source of truth and presents them through a focused visual editing surface.

The editor is designed for simple writing: open a Markdown file, edit it directly, and keep the saved document readable outside Hanji.

## Product North Star

Hanji is a light, local-first Markdown editor. It should feel simple enough for notes, durable enough for plain text writing, and open enough for user plugins.

## Project Structure

- [docs/](docs/) contains product and engineering design notes.
- [docs/design/](docs/design/) contains core editor concepts and design vocabulary.
- [site/](site/) contains the static project website deployed with GitHub Pages.
- [crates/hanji-core](crates/hanji-core/) owns text editing primitives: text buffers, selections, transactions, undo, and core commands.
- [crates/hanji-markdown](crates/hanji-markdown/) owns Markdown parsing, source mapping, projection, and formatting commands.
- [crates/hanji-storage](crates/hanji-storage/) owns local file and document session behavior.
- [crates/hanji-plugin-api](crates/hanji-plugin-api/) is reserved for future public plugin contracts.
- [apps/hanji](apps/hanji/) contains the GPUI desktop application.

## Engineering Guidelines

- Prefer small, explicit modules over framework-heavy abstractions.
- Keep the default app fast, offline-capable, and understandable.
- Treat plugin APIs as public contracts. Document them before broadening them.
- Avoid introducing persistent services, telemetry, or network behavior without a design document.
- Keep UI copy calm, brief, and useful.

## Repository Conventions

- Put design documents under `docs/`.
- Put core editor concepts under `docs/design/`.
- Keep the public project website under `site/`.
- Keep the Rust and GPUI architecture direction in `docs/architecture.md` until a dedicated ADR structure exists.
- Keep GPUI app code in `apps/hanji/`.
- Add or update focused tests alongside behavior changes when practical.
- Start commit messages with an imperative verb and make the scope broad enough to describe the full change.
- Follow the AI-assisted contribution policy in [CONTRIBUTING.md](CONTRIBUTING.md), including `Assisted-by` commit trailers for material AI assistance.

## Contribution Workflow

- Agents MUST read [CONTRIBUTING.md](CONTRIBUTING.md) before starting contribution work, including issues, pull requests, documentation edits, and implementation changes.
- Before opening an issue, read the available templates under [.github/ISSUE_TEMPLATE/](.github/ISSUE_TEMPLATE/) and use the closest match: [Common issue](.github/ISSUE_TEMPLATE/common_issue.md) for ideas, tasks, and small issues, or [Bug report](.github/ISSUE_TEMPLATE/bug_report.md) for broken or confusing behavior.
- Before opening a pull request, read [.github/PULL_REQUEST_TEMPLATE.md](.github/PULL_REQUEST_TEMPLATE.md) and fill it with the actual changes, related issue, tests, and skipped checks.
- Keep issue and pull request titles concise and specific.
- Prefer small, focused issues and pull requests that describe one user-visible behavior, engineering task, or cleanup.
- When a request is exploratory, draft the issue or pull request body for review before creating it.
- Do not claim that checks, tests, screenshots, or manual verification passed unless they were actually run or captured.
- Link related issues, pull requests, design documents, or code paths when they clarify the scope.

## Initial Commands

```sh
make app
make check-app
make test
cargo test -p hanji
```
