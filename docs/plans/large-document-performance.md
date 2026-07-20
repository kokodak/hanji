# Large Document Performance

Status: Proposed

Hanji should stay responsive when editing long local Markdown files. Large document work should protect the source-backed WYSIWYG model while avoiding full-document work during ordinary interaction.

This note records the likely bottlenecks and the next optimization tasks. It is a planning document, not a benchmark report.

The currently implemented pipeline and cache boundary are documented in [Projection and Rendering](../architecture/rendering.md).

## Current Risks

The current editor path appears to scale too closely with total document size:

- Layout projects the whole document and shapes every rendered line.
- Prepaint projects the whole document again and shapes every rendered line again.
- Paint iterates every prepared line, even when most lines are outside the viewport.
- The editor keeps full line snapshots for hit testing, selection, link hitboxes, task markers, and caret placement.
- Mouse hit testing, caret lookup, and some selection helpers scan line snapshots linearly.
- Applying an edit clones the text buffer before and after the edit for undo history.
- Applying an edit rebuilds the full line index after replacing text.
- A large paste runs synchronously on the UI path, then immediately triggers full projection, layout, and paint work.

These costs are acceptable for small notes, but they compound for large pasted documents. The visible symptom is delayed paste, janky scrolling, and slow caret movement or editing after the paste completes.

## Optimization Principles

- Do work proportional to the visible viewport whenever possible.
- Keep source text as the durable coordinate space.
- Keep projection data disposable, but cache it when the source line and rendering context are unchanged.
- Preserve stable visual positions when Markdown markers reveal or hide.
- Prefer measurable improvements over large rewrites.
- Keep the first pass compatible with the existing `String`-backed document model unless measurement shows it is the dominant bottleneck.
- Keep implementation and dependency choices deliberate.

## Workstreams

### 1. Add Performance Instrumentation

Start with lightweight timing around the major stages:

- Clipboard read and paste transaction creation.
- Text edit application.
- Line index rebuild.
- Markdown projection.
- Layout measurement.
- Prepaint preparation.
- Paint.
- Mouse hit testing and vertical caret movement.

The timings should be hidden behind a development flag or environment variable. The goal is to compare changes against a repeatable large-document scenario before committing to deeper architecture changes.

Useful scenarios:

- Paste a 1 MB Markdown document.
- Paste a 5 MB Markdown document.
- Scroll continuously through a document with many paragraphs.
- Scroll through a document with many fenced code blocks.
- Move the caret vertically through soft-wrapped long lines.
- Edit a single character near the top, middle, and end of a large document.

### 2. Introduce Viewport-Based Rendering

The renderer should prepare only visible lines plus a small overscan region. This is the highest-priority optimization because scrolling and painting should not require shaping every line in the document.

Expected shape:

- Track document range, viewport range, visible ranges, and active reveal range separately.
- Maintain document layout metadata separately from visible line snapshots.
- Use the scroll offset and viewport height to determine the visible source line range.
- Shape and paint only visible lines plus overscan.
- Keep task marker and link hitboxes only for visible lines.
- Keep enough mapping data to resolve caret and selection positions outside the viewport without forcing a full render.

Open design question:

- Hanji needs total scroll height even when most lines are not shaped. The first version can use cached measured heights and conservative estimates for unmeasured lines.

### 3. Add A Height Map

Viewport rendering needs a source-backed vertical layout index.

Expected shape:

- Store per-line or per-block height metadata.
- Mark entries as estimated, measured, or invalid.
- Support source offset to vertical bounds lookup.
- Support y coordinate to source line lookup.
- Preserve scroll anchors when estimates are corrected.
- Keep total content height available without shaping every line.

### 4. Cache Line Measurement and Projection

Projection and text shaping should be cached per source line or block when the inputs are unchanged.

Cache keys should account for:

- Source line range or line revision.
- The line source text.
- Active reveal range or selection intersection.
- Wrap width.
- Text style and line presentation.

Invalidation should be narrow:

- Editing one paragraph should not invalidate unrelated paragraphs.
- Editing a line inside a fenced code block may invalidate the whole fence range.
- Editing a fence marker may invalidate following lines until the matching fence state is known.
- Changing wrap width invalidates shaped layout, but should not require reparsing unchanged Markdown source.

### 5. Add Change-Set Driven Invalidation

Derived editor state should be mapped through edits before falling back to recomputation.

Expected shape:

- Transactions expose changed source ranges and length deltas.
- Selection and reveal ranges map through the transaction.
- Projection, height, link, task marker, and syntax range caches map or invalidate narrowly.
- Undo and redo store inverse edit data rather than full text snapshots.

### 6. Replace Linear Interaction Lookups

Interactive lookup structures should avoid scanning every line:

- Find line by vertical position with binary search over line top offsets.
- Find line by source offset with binary search over source ranges.
- Track wrapped visual rows with prefix counts or cached row metadata.
- Keep visible hitboxes small and viewport-scoped.

This matters after viewport rendering because `last_lines` will no longer represent the whole document.

### 7. Reduce Edit-Time Full Copies

The current undo strategy stores full text buffers before and after each transaction. That is simple and correct, but expensive for large documents.

A better history entry should store:

- The source edit ranges.
- Inserted text.
- Removed text.
- Selection before and after.

Undo and redo can replay inverse transactions instead of restoring full text snapshots. This should be done carefully because correctness matters more than raw speed in history code.

### 8. Make Line Index Updates Incremental

After each edit, the line index should not always rebuild from the entire text.

Possible first step:

- Update line starts after the edited range by applying the byte-length delta.
- Replace only the line-start entries affected by the edited text.
- Rebuild fully only as a fallback for complex multi-edit transactions.

Longer-term option:

- Evaluate a rope-backed text buffer if large edits and random line access remain expensive after incremental indexing.

### 9. Keep Paste Responsive

Large paste should not monopolize the UI longer than necessary.

Possible improvements:

- Apply the source edit once, then defer expensive projection and layout work to the next frame.
- Avoid marker autocomplete during paste, as paste should remain source-preserving.
- Avoid rebuilding non-visible render caches immediately after paste.
- Consider showing a minimal responsive state while visible layout catches up.

## Measurement

Implementation work should be gated by repeatable measurements at the core, Markdown, renderer, and app interaction boundaries. Fixtures, metrics, baselines, and the proposed app harness are specified separately in [Performance Benchmarking](performance-benchmarking.md).

## Success Criteria

Hanji should eventually satisfy these targets on a typical local development machine:

- Pasting a 1 MB Markdown document should not feel frozen.
- Scrolling a long document should stay visually continuous.
- Editing a single character in a large document should not require visible full-document recomputation.
- Moving the caret through soft-wrapped lines should stay predictable and responsive.
- Source-backed WYSIWYG behavior should remain unchanged for hidden markers, reveal policy, links, lists, headings, and fenced code blocks.

## Suggested Implementation Order

1. Add opt-in performance instrumentation.
2. Add a repeatable large-document fixture or benchmark scenario.
3. Add benchmark commands and a first baseline result format.
4. Add internal update flags for document, selection, viewport, geometry, wrap width, and style changes.
5. Introduce a height map with estimated and measured line heights.
6. Introduce viewport-scoped line preparation in the renderer.
7. Add projection and line measurement caches.
8. Add change-set driven cache invalidation.
9. Replace linear line lookup paths with indexed lookup helpers.
10. Convert undo history from full snapshots to edit-based entries.
11. Make line index updates incremental.
12. Revisit the text buffer representation only if measurement still points to the core model as the dominant bottleneck.
