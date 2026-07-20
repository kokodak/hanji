# Documentation Guide

Status: Current

Documentation is part of Hanji's engineering contract. This guide defines where information belongs, how a page declares its lifecycle, and what automated checks protect the documentation graph.

## Taxonomy

| Location | Owns | Rejects | Page status |
| --- | --- | --- | --- |
| `docs/architecture/` | Current components, ownership, dependencies, runtime flow | product semantics, future targets, historical rationale | `Current` |
| `docs/design/` | Durable document and editing semantics | module maps, exhaustive APIs, implementation plans | `Current` |
| `docs/reference/` | Exact current APIs and supported behavior | proposals and architectural rationale | `Current` |
| `docs/development/` | Contributor, CI, deployment, packaging, and release workflows | product behavior and target architecture | `Current` |
| `docs/plans/` | Proposed or incomplete work, alternatives, milestones, open questions | claims about current implementation | `Proposed` |
| `docs/decisions/` | Accepted or superseded durable choices and their rationale | living API detail and unaccepted proposals | `Accepted` or `Superseded` |
| `docs/philosophy.md` | Product north star, principles, and boundaries | technical architecture | `Current` |

Only the documentation index, this guide, and product philosophy live directly under `docs/`. Technical pages belong to one of the category folders.

## Choosing a Location

Use the first question that matches:

1. Does it explain why a durable tradeoff was accepted? Use `docs/decisions/`.
2. Does it describe work that has not landed? Use `docs/plans/`.
3. Does it describe which component owns something today? Use `docs/architecture/`.
4. Does it define what correct editing behavior means across implementations? Use `docs/design/`.
5. Does it list the exact API or behavior available today? Use `docs/reference/`.
6. Does it tell a contributor how to build, test, deploy, or release? Use `docs/development/`.

If a proposal contains several kinds of information, keep it in `plans` while it is unimplemented. When it lands, distribute its durable result into architecture, design, reference, and decisions instead of moving the whole planning document unchanged.

## Folder README Contract

Every category directory has a `README.md` containing these headings:

- `Belongs Here`: positive ownership examples.
- `Does Not Belong Here`: routing guidance for common mistakes.
- `Contents`: links to every Markdown page directly inside the folder.
- `Maintenance Rule`: when and how the category changes.

The README is both a human contract and the navigation index checked by `make check-docs`.

## Page Contract

Every non-index page starts with:

```md
# Descriptive Title

Status: Current

One paragraph stating the page's scope and why it exists.
```

Use the status required by the taxonomy. Additional metadata such as `Stability`, `Progress`, or `Scope` may follow the status, but must not replace it.

A page should have one primary concern. It may link to another category but should not reproduce that category's complete explanation.

Category folders stay flat. Use lowercase kebab-case filenames such as `editing-runtime.md`; decision records prefix that name with a four-digit sequence such as `0003-editor-history.md`. Introduce nested documentation folders only by extending this guide and the validation harness in the same change.

## Page Templates

### Living Document

Use for architecture, design, reference, and development pages:

```md
# Title

Status: Current

Scope and purpose.

## Context

What a reader needs to understand the subject.

## Contract or Behavior

The current facts, invariants, or procedure.

## Boundaries

What this subject deliberately does not own.

## Related Documentation

Links to adjacent concerns without duplicating them.
```

Use only the sections that improve the page. The required parts are the title, status, and clear scope.

### Plan

```md
# Title

Status: Proposed

Problem and desired outcome.

## Evidence
## Goals
## Non-Goals
## Proposed Direction
## Alternatives
## Work Packages
## Success Criteria
## Open Questions
```

A plan must distinguish completed foundation from proposed work. Use a separate `Progress` line or explicit work-package states.

### Decision Record

Use a zero-padded sequence number in the filename, such as `0003-example.md`:

```md
# 0003: Decision Title

Status: Accepted

## Context
## Decision
## Consequences
## Living Documentation
```

Do not rewrite an accepted record to match a later architecture. Add a new record with `Status: Accepted`, change the old one to `Status: Superseded`, and link the two.

## Source-of-Truth Rules

| Question | Primary document |
| --- | --- |
| What does the code own today? | Architecture |
| What behavior must every frontend preserve? | Design |
| What can a consumer call or observe today? | Reference |
| What should contributors run? | Development |
| What might be built next? | Plans |
| Why was this durable choice made? | Decisions |

Other pages should link to the primary document rather than maintaining a competing copy. Small contextual summaries are fine when they make the local page understandable.

## Lifecycle

1. Start incomplete engineering direction as a plan.
2. Record a decision when an important alternative is accepted.
3. Implement the change and update current architecture and reference in the same change.
4. Move durable behavior into design documentation.
5. Reduce or remove completed planning material after its results have a living home.
6. Supersede decision records instead of rewriting history.

When moving a page, update every repository link in the same change. Do not keep compatibility stubs unless an external documentation URL is intentionally supported.

## Links and Navigation

- Use relative repository links.
- Link every page from its category README.
- Link every category README from `docs/README.md`.
- Prefer descriptive link labels over raw paths.
- Link to code when it clarifies ownership, but keep durable semantics independent of line numbers.
- Use web links only when the external source is intentionally part of the contract.

## Automated Checks

Run:

```sh
make check-docs
```

The harness verifies:

- required category README files and folder-contract headings;
- allowed top-level files under `docs/`;
- category-specific status values near the top of each page;
- flat category layout and filename conventions;
- existence of relative local Markdown link targets;
- that every page is reachable from another Markdown page;
- that each category README links every sibling page.

The check runs in CI. It protects structure, not prose quality or technical truth; reviewers still compare current documents with the implementation.

## Review Checklist

- Is this the correct category for the page's primary concern?
- Does the title and opening paragraph establish scope?
- Is the status correct for its folder?
- Does the page separate current facts from proposed work?
- Is one category the clear source of truth for each claim?
- Are coordinate units, ownership, and non-goals explicit where relevant?
- Is the page linked from its folder README?
- Were obsolete links and completed planning details removed?
- Does `make check-docs` pass?
