# Plans

Plans describe work that is proposed, incomplete, or intentionally deferred. They may discuss target APIs and implementation order, but they must not be read as current behavior.

## Belongs Here

- Problem statements and evidence for work that has not landed.
- Target architecture or API sketches clearly marked as proposed.
- Alternatives, open questions, staged implementation, and success criteria.
- Performance or migration work that spans multiple future changes.

## Does Not Belong Here

- Implemented architecture; move it to `docs/architecture/`.
- Accepted durable semantics; move them to `docs/design/`.
- Exact current behavior; move it to `docs/reference/`.
- A durable decision whose rationale must remain historical; record it in `docs/decisions/`.

## Contents

- [Web Editor](web-editor.md): WebAssembly adapter, JavaScript facade, and website demo.
- [Large Document Performance](large-document-performance.md): measurement and viewport-oriented scaling work.
- [Performance Benchmarking](performance-benchmarking.md): fixtures, metrics, baselines, and harness direction.

## Deferred Topics

The following areas need a focused design before implementation:

- user plugin capabilities and isolation;
- local folder or workspace semantics;
- external file-change detection and conflict handling;
- autosave and crash recovery;
- multi-selection in the public editor API;
- a reusable browser rendering surface.

## Maintenance Rule

When a plan is implemented, move durable semantics to `docs/design/`, current ownership to `docs/architecture/`, exact interfaces to `docs/reference/`, and record major tradeoffs under `docs/decisions/`. Remove or reduce completed plans so readers do not mistake implementation history for the current contract.
