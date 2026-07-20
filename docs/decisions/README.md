# Architecture Decisions

Decision records preserve the reason behind durable choices. They are short historical records, not the place for exhaustive behavior or current module documentation.

## Belongs Here

- Accepted choices with long-term consequences for dependencies, persistence, public contracts, security, or portability.
- The context and alternatives needed to understand why a choice was made.
- Consequences that future changes must account for.
- Superseding records when an accepted choice changes.

## Does Not Belong Here

- Routine refactor descriptions or commit summaries.
- Living architecture and API details that will be edited frequently.
- Unaccepted proposals; keep them in `docs/plans/`.
- Exhaustive product behavior; put it in `docs/design/` or `docs/reference/`.

## Contents

- [0001: Keep Markdown as the source of truth](0001-markdown-source-of-truth.md)
- [0002: Use a portable editor facade](0002-portable-editor-facade.md)

## Maintenance Rule

Each record contains a status, context, decision, consequences, and links to living documentation. Accepted records are not rewritten when implementation details change. Supersede them with a new record and link both directions.

Use a decision record when alternatives have meaningful long-term consequences for dependency direction, persisted data, public contracts, security, or platform portability. Routine refactors belong in normal architecture documents and commit history.
