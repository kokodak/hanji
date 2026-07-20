# Performance Benchmarking

Status: Proposed

Performance work needs repeatable measurement before and after each change. The benchmark suite should follow the same boundaries as the product architecture: core text edits, Markdown projection, renderer preparation, and app-level interaction.

This plan supports [Large Document Performance](large-document-performance.md). Microbenchmarks identify hot paths, while user-facing budgets come from complete scenarios such as paste, scroll, type, select, and caret movement.

## Benchmark Types

- Core benchmarks: text edits, line lookup and indexing, selection mapping, undo, and redo.
- Markdown benchmarks: line classification, inline projection, fenced code blocks, links, marker reveal, and visible/source mapping.
- Renderer benchmarks: projection consumption, visible segments, shaping, wrapping, height caches, hitboxes, and paint preparation.
- Interaction benchmarks: paste, continuous scroll, single-character edits, vertical caret movement, mouse hit testing, selection drag, and select all.
- Memory benchmarks: peak memory, retained cache size, line snapshots, shaped lines, history entry size, and buffer allocations.

## Fixture Corpus

Use deterministic local fixtures so results can be compared across commits:

- `small-note`: a short ordinary note that protects the common case.
- `long-prose-1mb`: paragraphs with soft wrapping and common inline Markdown.
- `long-prose-5mb`: a stress version of the prose fixture.
- `many-short-lines`: many simple lines for indexing, scrolling, and hit testing.
- `many-long-lines`: very long soft-wrapped lines for shaping and vertical movement.
- `mixed-markdown`: headings, quotes, lists, tasks, links, code, styles, rules, and escapes.
- `many-fences`: many closed backtick and tilde fenced code blocks.
- `unclosed-fence-tail`: malformed input that stresses fallback scanning.

Synthetic fixtures are preferred because they are stable. Anonymized real notes may be added later only when they are safe to store in the repository.

## Metrics

Each run should record:

- document bytes, line count, maximum line length, and estimated wrapped rows;
- total and per-stage elapsed time;
- p50, p95, and maximum latency for repeated interactions;
- frame count and slow-frame count for app-level runs;
- projected, shaped, painted, visible, and cached line counts;
- projection and measurement cache hit rates;
- peak and retained memory.

Interaction latency and predictable frame time matter more than average throughput.

## Measurement Discipline

- Warm each scenario once before recording.
- Record multiple iterations and retain p50, p95, and maximum values.
- Separate cold-start cost from steady-state interaction cost.
- Use release builds for recorded numbers.
- Keep debug instrumentation opt-in.
- Store machine and build metadata with every result.
- Do not compare unrelated machines as pass or fail.
- Do not tune regression thresholds until the suite is stable enough to trust.

## Baselines and Regression Gates

The first milestone should check in a baseline from a known machine. Later changes can compare with that baseline.

- Stable core and Markdown microbenchmarks may run in CI.
- GPUI interaction benchmarks can begin as manual or nightly checks.
- A regression should be flagged when p95 latency or peak memory exceeds an agreed threshold for the same fixture and machine profile.
- A claimed improvement should include both the benchmark delta and the user-visible scenario it changes.

## Viewport Counters

Large-document runs should record whether work scales with visible content or total source size:

- total and visible source lines;
- overscan lines;
- lines projected, shaped, and painted for the frame;
- lines retained in caches;
- height metadata entries.

After viewport rendering lands, per-frame shaped and painted line counts should remain stable as total document size grows.

## App Harness Direction

The app harness should open a fixture, wait for the editor to become idle, perform deterministic actions, and write structured results.

Proposed commands:

```sh
make bench-core
make bench-markdown
make bench-app FILE=fixtures/bench/long-prose-1mb.md
make bench-app SCENARIO=paste FILE=fixtures/bench/mixed-markdown.md
```

Exact command names may change. The workflow should remain simple enough to run before and after a rendering change.
