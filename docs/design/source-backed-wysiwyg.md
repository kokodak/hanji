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

Current inline projection starts with plain text, escaped punctuation, emphasis spans, strong spans, inline code spans, and inline links. Current line projection recognizes headings once the ATX marker is followed by whitespace, blockquotes once a `>` marker is followed by a space, unordered or ordered list items once a list marker is followed by whitespace, and task list markers written as `[ ]`, `[x]`, or `[X]` at the start of list content only after the closing bracket is followed by whitespace. A pending heading marker such as `#` and a pending task marker such as `- [ ]` remain visible source until the user types the following space. The GPUI app hides inactive inline markers, hides inactive heading markers, hides blockquote and list line markers, draws separate list marker or checkbox previews, styles emphasis content with italic text, styles strong content with a heavier font weight, underlines link text without changing its color, draws inline code backgrounds from source-backed visible ranges, and renders blockquote lines with a quote bar and indentation. Revealed Markdown syntax markers are highlighted in green; revealed escape backslashes use a muted syntax color so they are visible without competing with primary Markdown markers. Active heading hash markers also use the green syntax color while the rest of the revealed heading source keeps heading typography and normal black text. GPUI 0.2.2 can merge line layout runs when only font changes, so emphasis and strong text currently force an invisible decoration boundary in the app renderer. Emphasis projection recognizes exact single-asterisk delimiter runs, and strong projection recognizes exact two-asterisk delimiter runs; longer or malformed asterisk runs remain text. Link projection recognizes simple inline links in the form `[label](destination)` when both label and destination are non-empty and the destination has no whitespace. Escaped ASCII punctuation such as `\*`, `\[`, `\\`, and a backslash before a backtick is treated as literal text in preview. Malformed markers should not stop projection of later valid spans. Nesting, images, other line marker hiding, and parser-grade CommonMark behavior should be added incrementally with source mapping tests.

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

For headings, ATX markers become a heading only after the hash run is followed by whitespace. A pending marker such as `#` remains a normal paragraph source. Once recognized, inactive headings render as preview text: `## Title` renders as `Title`. When the caret or active selection enters the heading line, the source is revealed as `## Title`. Only the hash run is marked as Markdown syntax and highlighted in green. The following whitespace and content remain visible with heading font size and weight.

For escapes, the backslash is a Markdown marker. In inactive preview, `\*` renders as `*`, `\[` renders as `[`, and `\\` renders as `\`. When the caret or active selection enters the escaped source range, or any non-whitespace token containing that escaped source, the backslash is revealed with a muted syntax color. This makes escaped pairs such as `\*literal\*` reveal both backslashes while the caret is inside `literal`. Escaped punctuation must not start Markdown spans.

For inline links, inactive preview hides `[`, `](`, the destination, and `)`, leaving only the label as editable visible text. The label is underlined with the normal text color. When the caret or active selection enters any part of the link source, Hanji reveals the full `[label](destination)` source. Link syntax markers are highlighted in green when revealed; the destination remains normal source text. Clicking a link label opens `http` and `https` destinations with the system browser. Other URL schemes are ignored by the opener.

For blockquotes, the visible start of the line maps after the hidden `> ` marker. Pressing Enter in a non-empty blockquote line continues the blockquote by inserting a new `> ` marker. Pressing Enter again on an empty blockquote marker line removes that marker and leaves a clean normal line. Consecutive blockquote lines should render as one visual quote block with a continuous quote bar; an unquoted line breaks the run.

For list items, the visible start of the line maps after the hidden list marker and aligns with normal paragraph text. The renderer draws the visual bullet, ordered marker, or checkbox separately in the gutter to the left of editable text. A normal click in the marker gutter places the caret at the content start, while dragging into the marker gutter resolves to the marker source range so selection can reveal and select the raw marker. When the caret or active selection enters the hidden list marker source range, the raw marker should be revealed and the separate visual marker should be hidden. Task checkbox previews are source-backed controls: clicking an unchecked preview updates `[ ]` to `[x]`, and clicking a checked preview updates `[x]` or `[X]` to `[ ]`. Pressing Enter in a non-empty list item continues the list with the same unordered marker or the next ordered number. Task list items continue as unchecked tasks. Pressing Enter again on an empty list marker line removes that marker and leaves a clean normal line.

For selection placement, source range boundaries remain meaningful. A selection that starts outside an inline span and extends into that span should reveal and select the marker text it crosses. A selection that starts inside the inline content uses the same caret placement rule as editing, so it selects the content without implicitly adding hidden markers.

Keyboard selection expansion uses the same source coordinate rules. `Shift+Arrow` extends by visible caret movement, `Shift+Option+Left/Right` extends to the previous or next source word boundary within the current line, and `Shift+Cmd` extends to the current line or document boundary depending on the arrow direction. Left and right movement shortcuts should not cross line boundaries; moving between lines belongs to up and down movement.

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
- Heading recognition starts only after the ATX hash run is followed by whitespace.
- Inactive recognized headings hide the ATX marker and render content as preview text.
- Active recognized headings reveal source, with only the hash run marked as syntax.
- Pending heading markers without trailing whitespace stay visible as paragraph source.
- Hidden blockquote markers are omitted from visible text while visible line starts map after the marker.
- Enter continues non-empty blockquote lines and exits from empty blockquote marker lines.
- Hidden list markers are omitted from visible text while visible line starts map after the marker.
- A caret or selection inside a hidden list marker reveals the raw marker source.
- A pending task marker without trailing whitespace stays visible as source.
- A normal click in the list marker gutter places the caret at content start, while dragging into the gutter reveals and selects marker source.
- Enter continues non-empty list items and exits from empty list marker lines.
- Task list markers are hidden with the list marker while checkbox state remains available to the renderer.
- Clicking a checkbox preview toggles only the source state character inside `[ ]`, `[x]`, or `[X]`.
- Backspace and Delete at revealed marker boundaries remove one source character, not the whole formatting span.
- Adjacent or malformed markers do not leak styles into unrelated spans.
- Inline code and strong spans remain independent when one of them becomes malformed.
- Simple inline links hide markers and destinations while preserving source mapping for the label.
- A caret or selection inside link label, marker, or destination reveals the full link source.
- Malformed links and escaped link openings remain plain text.
- Escaped punctuation hides only the escape backslash in inactive preview and reveals it at the caret or inside the same non-whitespace token.
- Escaped punctuation must not start emphasis, strong, code, or link spans.
- Revealed escape backslashes use a muted syntax color.
- Clicking a simple inline link label opens only supported `http` and `https` destinations.

## Ownership

`hanji-markdown` owns Markdown-specific projection data such as line kinds, inline spans, content ranges, and marker ranges.

`hanji-core` owns source editing primitives such as text ranges, selections, transactions, undo, and grapheme-safe caret boundaries.

`apps/hanji-rust` owns GPUI rendering, hit testing, and shortcut routing. It should consume projection data, translate platform events back into core source positions, and route formatting shortcuts such as strong and inline code through `hanji-markdown` commands instead of editing Markdown markers in renderer code.
