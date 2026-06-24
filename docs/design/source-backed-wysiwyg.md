# Source-Backed WYSIWYG

Hanji's visual editor is a projection over Markdown source, not a separate rich text document.

The saved file is always Markdown. A WYSIWYG view can hide markers, style text, or render block widgets, but every visible object must be traceable back to source text.

## Coordinate Spaces

Hanji uses two coordinate spaces:

- Source coordinates are byte offsets into the Markdown document.
- Visible coordinates are positions in the rendered editor view.

Source coordinates are the durable coordinate space. Editing commands, selections, undo history, and saving should eventually resolve back to source ranges.

Visible coordinates are temporary. They exist to render, hit test, and present the document in a friendlier form.

Inline projection exposes visible line coordinates before marker hiding is rendered in the app. A visible line offset is a byte offset into the line text after hidden markers are omitted. A projected visible segment stores both the visible range and the source range that produced it; styled segments also keep an outer source range that includes hidden markers.

Visible-to-source mapping needs an explicit boundary affinity because a single visible caret position can represent two valid source positions around hidden markers. For example, the visible position before `bold` in `**bold**` can map either before the opening marker or inside the strong content. Hit testing, keyboard movement, and editing commands should choose that affinity intentionally instead of guessing inside renderer code.

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

The first rendering step kept markers visible and only applied styling to known spans. Marker hiding now depends on explicit visible-to-source mapping.

Current inline projection starts with plain text, strong spans, and inline code spans. The GPUI app hides inactive inline markers, styles strong content with a heavier font weight, and draws inline code backgrounds from source-backed visible ranges. GPUI 0.2.2 can merge line layout runs when only font changes, so strong text currently forces an invisible decoration boundary in the app renderer. Strong projection only recognizes exact two-asterisk delimiter runs for now; single, longer, or malformed asterisk runs remain text. Malformed markers should not stop projection of later valid spans. Escapes, nesting, emphasis, links, and parser-grade CommonMark behavior should be added incrementally with source mapping tests.

## Marker Policy

Markdown markers are not decoration. They are source text.

When markers are visible, source and visible coordinates are close to one-to-one. When markers are hidden, projection code must map visible positions back to source positions explicitly.

Hanji uses an Obsidian-like live preview policy for supported inline Markdown:

- Hide recognized inline markers by default.
- Reveal source markers for the inline span whose outer source range contains the text caret.
- Reveal source markers for any inline span whose outer source range intersects the active selection.
- Do not reveal markers on mouse hover alone.
- Keep unrelated inline spans hidden when one span is active.
- Treat revealed markers as ordinary source text. Typing, Backspace, and Delete should operate on what is visible, even if that leaves temporarily malformed Markdown.
- Treat malformed or unsupported Markdown as plain source text instead of guessing a WYSIWYG shape.

Caret reveal includes the opening and closing marker boundaries. This keeps deletion and insertion near marker edges honest: when the caret can edit a marker, the marker should be visible.

Hidden markers must never be edited implicitly. Any edit that starts from visible coordinates must first resolve to a source range with explicit boundary affinity.

For caret placement, the visible start of hidden inline content maps after the opening marker, and the visible end maps before the closing marker. That means clicking before `bold` in hidden `**bold**` places the caret at `**|bold**`, while clicking after `bold` places it at `**bold|**`. The next Backspace or Delete then edits visible source text one character at a time.

For selection placement, source range boundaries remain meaningful. A selection that starts outside an inline span and extends into that span should reveal and select the marker text it crosses. A selection that starts inside the inline content uses the same caret placement rule as editing, so it selects the content without implicitly adding hidden markers.

Keyboard selection expansion uses the same source coordinate rules. `Shift+Arrow` extends by visible caret movement, `Shift+Option+Left/Right` extends to the previous or next source word boundary, and `Shift+Cmd` extends to the current line or document boundary depending on the arrow direction.

## Test Scenarios

Projection tests should focus on behavior that can change editing meaning:

- Hidden markers are omitted from the default visible text while content keeps source ranges.
- A caret inside an inline span reveals that span's markers only.
- A caret on an opening or closing marker boundary reveals the span.
- A selection that intersects hidden markers reveals the span.
- A selection spanning multiple inline spans reveals each intersected span.
- A selection starting outside an inline span includes crossed marker text.
- A selection starting inside inline content excludes hidden markers unless the user explicitly extends into them.
- Hidden inline content boundaries map to editable marker edges.
- Backspace and Delete at revealed marker boundaries remove one source character, not the whole formatting span.
- Adjacent or malformed markers do not leak styles into unrelated spans.
- Inline code and strong spans remain independent when one of them becomes malformed.

## Ownership

`hanji-markdown` owns Markdown-specific projection data such as line kinds, inline spans, content ranges, and marker ranges.

`hanji-core` owns source editing primitives such as text ranges, selections, transactions, undo, and grapheme-safe caret boundaries.

`apps/hanji-rust` owns GPUI rendering, hit testing, and shortcut routing. It should consume projection data, translate platform events back into core source positions, and route formatting shortcuts such as strong and inline code through `hanji-markdown` commands instead of editing Markdown markers in renderer code.
