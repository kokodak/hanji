# Web Editor

Status: Proposed

Progress: Portable Rust foundation complete

Hanji should expose its existing source-backed editor engine to JavaScript without moving business rules into browser bindings. The first package should be a headless editor engine rather than a reusable DOM component.

The current native and adapter boundaries are documented in [Platform Adapters](../architecture/platform-adapters.md). This plan records only work that has not landed yet.

## Goals

- Run the same core and Markdown editing policy in desktop and browser frontends.
- Provide a small handwritten TypeScript-facing API.
- Make UTF-8 engine offsets and UTF-16 browser offsets explicit.
- Return owned projection snapshots suitable for JavaScript rendering.
- Keep generated WebAssembly glue and Rust lifetimes out of the public package contract.

## Non-Goals

- Reuse the GPUI renderer in the browser.
- Put browser storage in the Rust editor engine.
- Stabilize the plugin API as part of the first package.
- Add multi-selection to the first web API.
- Publish a framework-specific editor component before the headless contract is proven.

## Proposed Dependency Direction

```text
@hanji/editor -> hanji-wasm -> hanji-editor -> hanji-markdown -> hanji-core
                                    └────────────────────────> hanji-core
```

`hanji-editor` already exists and is checked for `wasm32-unknown-unknown`. `hanji-wasm` and `@hanji/editor` do not exist yet.

## WebAssembly Adapter

The proposed `hanji-wasm` crate is a thin `cdylib` responsible for:

- `wasm-bindgen` exports;
- JavaScript UTF-16 and engine UTF-8 conversion;
- stable error-code mapping;
- conversion from borrowed Rust projections to owned snapshots;
- module initialization and explicit disposal.

Business rules must remain in `hanji-core`, `hanji-markdown`, or `hanji-editor`. A behavior that can be tested without a browser does not belong in the binding crate.

## Projection Snapshot

JavaScript must not receive `MarkdownProjection<'_>` directly. The adapter should return a deliberately smaller owned schema, for example:

```ts
export interface ProjectionSnapshot {
  lines: ProjectedLine[];
}

export interface ProjectedLine {
  sourceRange: TextRange;
  markerRange?: TextRange;
  kind: LineKind;
  visibleText: string;
  segments: ProjectedSegment[];
}

export interface ProjectedSegment {
  visibleRange: TextRange;
  sourceRange: TextRange;
  sourceOuterRange: TextRange;
  kind: SegmentKind;
  style: InlineStyle;
}
```

The snapshot is a JavaScript rendering contract, not a serialization of every private Rust projection field.

## JavaScript Facade

The public surface should be handwritten rather than exposing raw generated bindings:

```ts
const editor = await HanjiEditor.create("# Hanji");

editor.setSelection({ anchor: 7, head: 7 });
editor.replaceText({ text: " notes", mode: "typing" });
editor.execute("toggleStrong");

const source = editor.source;
const projection = editor.getProjection();

editor.execute("undo");
editor.dispose();
```

Initialization may be asynchronous because WebAssembly must load. Editing operations should be synchronous and deterministic after initialization.

The package should provide TypeScript declarations, stable error codes, explicit UTF-16 range documentation, and package-level tests. Raw generated glue remains internal.

## Coordinate Boundary

The adapter must convert every incoming UTF-16 code-unit position into a validated UTF-8 source offset, execute in source coordinates, and convert outgoing source and visible ranges back to UTF-16. Source and visible ranges remain separate even when both use UTF-16 numbers in JavaScript.

Visible-to-source mapping must preserve explicit boundary affinity. See [Coordinate Systems](../design/coordinate-systems.md).

## Work Packages

1. Complete: extract portable Markdown editing policy into `hanji-markdown`.
2. Complete: introduce `hanji-editor` and make GPUI/storage consume it.
3. Add `hanji-wasm`, error codes, owned snapshots, and browser-independent binding tests.
4. Add `@hanji/editor` with a handwritten facade and TypeScript declarations.
5. Add WebAssembly checks and package tests to CI.
6. Build a small local-storage-backed demo under `site/`.
7. Evaluate a reusable DOM or canvas surface only after the headless API is exercised by the demo.

These should remain separate reviewable changes. No migration shim is required while neither Rust nor JavaScript APIs are externally published; coherence takes priority until the first public release establishes compatibility commitments.

## Open Decisions

- JavaScript error codes and exception shape.
- IME composition history grouping.
- Final owned projection snapshot fields.
- npm package scope and versioning relationship with Rust crates.
- Browser persistence used by the demo.
- Whether a later reusable surface is a custom element or a lower-level rendering package.
