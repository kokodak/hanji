# Projection and Rendering

Status: Current

Hanji separates semantic projection from pixel rendering. `hanji-markdown` decides how Markdown source maps to visible content; the GPUI app decides how that content is measured, placed, hit-tested, and painted.

## Pipeline

```text
UTF-8 Markdown source
    -> hanji-markdown::MarkdownProjection
    -> GPUI document measurement
    -> request_layout
    -> prepaint snapshots and hitboxes
    -> paint
```

`Editor::projection()` returns a borrowed projection tied to the current source. The projection contains line classification, source ranges, visible segments, inline styles, Markdown marker ranges, links, tasks, and table cells.

## Projection Responsibilities

`hanji-markdown` owns:

- classifying lines and inline syntax;
- identifying content and marker ranges;
- producing visible text segments backed by source ranges;
- mapping visible offsets back to source offsets with explicit affinity;
- exposing table, task, link, and code-fence structure without creating a second document model.

Projection uses source coordinates only. It knows nothing about fonts, wrapping widths, scroll positions, mouse coordinates, or GPUI elements.

## GPUI Responsibilities

`apps/hanji` owns:

- font shaping and soft wrapping;
- line and document measurements;
- pixel bounds, scrolling, and viewport state;
- marker reveal presentation;
- caret and selection painting;
- link and task hitboxes;
- mouse hit testing and vertical caret targeting;
- transient snapshots required by GPUI's layout and paint phases.

The app may read `hanji-markdown` projection types directly for rendering. This is a read-only adapter dependency, not a mutation path.

## Derived State Rule

Layout snapshots, hitboxes, and projections are caches. They must be invalidated or rebuilt when their source, selection, viewport, wrap width, or style inputs change. They are never serialized as document state.

The durable behavior of hidden markers and visible/source mapping is documented in [Source-Backed WYSIWYG](../design/source-backed-wysiwyg.md). Known scaling risks and the proposed viewport architecture are tracked in [Large Document Performance](../plans/large-document-performance.md).
