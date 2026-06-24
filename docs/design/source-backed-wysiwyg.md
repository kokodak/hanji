# Source-Backed WYSIWYG

Hanji's visual editor is a projection over Markdown source, not a separate rich text document.

The saved file is always Markdown. A WYSIWYG view can hide markers, style text, or render block widgets, but every visible object must be traceable back to source text.

## Coordinate Spaces

Hanji uses two coordinate spaces:

- Source coordinates are byte offsets into the Markdown document.
- Visible coordinates are positions in the rendered editor view.

Source coordinates are the durable coordinate space. Editing commands, selections, undo history, and saving should eventually resolve back to source ranges.

Visible coordinates are temporary. They exist to render, hit test, and present the document in a friendlier form.

## Projection

A projection derives visible structure from source text.

The projection must preserve enough source mapping to answer these questions:

- Which source range produced this visible object?
- Which source range is visible content?
- Which source ranges are syntax markers?
- If the user clicks or edits here, which source position should change?

Derived projection data must be disposable. Rebuilding it from Markdown source should not lose document data.

## Inline Spans

Inline projection starts with spans inside a source line.

For this source:

```md
This is **bold** text
```

Hanji can derive spans like this:

```text
Text:
  source range:  This is
  content range: This is

Strong:
  source range:  **bold**
  content range: bold
  marker ranges: ** and **

Text:
  source range:  text
  content range: text
```

The first rendering step can keep markers visible and only apply styling to known spans. Hiding markers should come later, after caret mapping and editing behavior are trustworthy.

Current inline projection starts with plain text, strong spans, and inline code spans. The GPUI app keeps source markers visible: strong content uses a heavier font weight, and inline code markers plus content share the code background. GPUI 0.2.2 does not split line layout font runs for font-only changes, so strong rendering currently forces an invisible decoration boundary in the app renderer. Escapes, nesting, emphasis, links, and parser-grade CommonMark behavior should be added incrementally with source mapping tests.

## Marker Policy

Markdown markers are not decoration. They are source text.

When markers are visible, source and visible coordinates are close to one-to-one. When markers are hidden, projection code must map visible positions back to source positions explicitly.

Hanji should introduce marker hiding gradually:

- Build source-backed spans first.
- Render styles while keeping source text visible.
- Add tests for caret movement, selection, and editing around markers.
- Hide markers only when the visible-to-source mapping is explicit.
- Consider showing markers near the caret if editing would otherwise feel ambiguous.

## Ownership

`hanji-markdown` owns Markdown-specific projection data such as line kinds, inline spans, content ranges, and marker ranges.

`hanji-core` owns source editing primitives such as text ranges, selections, transactions, undo, and grapheme-safe caret boundaries.

`apps/hanji-rust` owns GPUI rendering, hit testing, and shortcut routing. It should consume projection data, translate platform events back into core source positions, and route formatting shortcuts such as strong and inline code through `hanji-markdown` commands instead of editing Markdown markers in renderer code.
