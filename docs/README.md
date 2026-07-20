# Hanji Documentation

Hanji's documentation is organized by purpose so that current architecture, durable editing semantics, exact interfaces, future plans, and historical decisions do not compete inside the same document.

Hanji is a light, local-first Markdown editor. UTF-8 Markdown remains the source of truth and the visual editor is a source-backed projection over it.

## Start Here

For a first technical reading:

1. Read [Philosophy](philosophy.md) for the product constraints.
2. Read [Architecture](architecture/README.md) for the system and runtime map.
3. Read [Crate Boundaries](architecture/crate-boundaries.md) for ownership.
4. Read [Source-Backed WYSIWYG](design/source-backed-wysiwyg.md) and [Editing Policy](design/editing-policy.md) for core semantics.
5. Use [Reference](reference/README.md) for current APIs and supported behavior.

Contributors adding or moving documentation should read the [Documentation Guide](documentation-guide.md).

## Library

| Area | Purpose | Changes when |
| --- | --- | --- |
| [Philosophy](philosophy.md) | Product values and boundaries | the product direction changes |
| [Architecture](architecture/README.md) | Current components, ownership, and runtime flow | code moves across boundaries or dependencies change |
| [Design](design/README.md) | Durable document and editing semantics | behavior or invariants change |
| [Reference](reference/README.md) | Exact current API and supported behavior | public surfaces or implemented behavior change |
| [Development](development/README.md) | Build, test, website, and release workflows | contributor processes change |
| [Plans](plans/README.md) | Proposed or incomplete work | a proposal evolves or becomes implemented |
| [Decisions](decisions/README.md) | Historical rationale for durable choices | a major decision is accepted or superseded |

## Current and Future State

Architecture and reference documents describe the current repository. Plans describe work that does not exist yet. Design documents may define durable semantics ahead of a specific implementation, but must state their status clearly.

Do not describe a proposed type or package in a current API reference. Do not leave implementation history in a design contract after the architecture has settled. Use decision records for rationale that should survive later refactors.

## Documentation Harness

The [Documentation Guide](documentation-guide.md) defines folder contracts, required status values, page templates, lifecycle rules, and the review checklist.

Run the automated structural checks with:

```sh
make check-docs
```

The check verifies category indexes, page status, relative local links, and navigation coverage.

Contributor commands and documentation workflow are in [Development](development/README.md).
