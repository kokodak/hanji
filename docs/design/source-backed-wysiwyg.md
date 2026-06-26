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

Current inline projection starts with plain text, escaped punctuation, emphasis spans, strong spans, strong-emphasis spans, strikethrough spans, inline code spans, inline links, angle-bracket autolinks, and raw URL linkification. Current line projection recognizes headings once the ATX marker is followed by whitespace, horizontal rules written with at least three hyphens plus optional leading indentation and trailing whitespace, blockquotes once a `>` marker is followed by a space, unordered or ordered list items once a list marker is followed by whitespace, task list markers written as `[ ]`, `[x]`, or `[X]` at the start of list content only after the closing bracket is followed by whitespace, and closed backtick or tilde fenced code blocks. A pending heading marker such as `#`, a pending task marker such as `- [ ]`, and an unclosed code fence remain visible source. The GPUI app hides inactive inline markers, hides inactive heading markers, renders inactive horizontal rules as a divider line, hides blockquote and list line markers, hides inactive code fence lines, draws separate list marker or checkbox previews, styles emphasis content with italic text, styles strong content with a heavier font weight, styles strikethrough content with a line-through decoration, renders link text with a yellow color and underline, draws inline code backgrounds from source-backed visible ranges, draws fenced code block line backgrounds, and renders blockquote lines with a quote bar and indentation. Revealed Markdown syntax markers are highlighted in green; revealed escape backslashes use a muted syntax color so they are visible without competing with primary Markdown markers. Active heading hash markers also use the green syntax color while the rest of the revealed heading source keeps heading typography and normal black text. GPUI 0.2.2 can merge line layout runs when only font changes, so emphasis, strong, and strikethrough text currently force an invisible decoration boundary in the app renderer. Emphasis projection recognizes exact single-asterisk delimiter runs, strong projection recognizes exact two-asterisk delimiter runs, strong-emphasis projection recognizes exact three-asterisk delimiter runs, and strikethrough projection recognizes exact two-tilde delimiter runs; longer or malformed delimiter runs remain text. Strong, emphasis, and strikethrough wrappers compose over the projected inline children inside their content range, so mixed text, inline links, autolinks, raw URLs, and inline code can share the same outer styles without creating combination-specific segment kinds. For example, `**hello [label](url) now**`, `~~***thought***~~`, `*[label](url)*`, `***[label](url)***`, and matching inline-code forms keep all applicable styles. Inline code has higher precedence than Markdown markers inside its own backticks, so code content remains literal even when it contains `**`, `*`, links, or URLs. Link projection recognizes simple inline links in the form `[label](destination)` when both label and destination are non-empty and the destination has no whitespace. Autolink projection recognizes `<http://...>` and `<https://...>` when the URL contains no whitespace. Raw URL linkification recognizes `http://...` and `https://...` in plain text without rewriting source text. Fenced code projection supports backtick and tilde fences; the opening fence must use at least three identical marker characters and the closing fence must use the same marker character with at least as many markers followed only by whitespace. Escaped ASCII punctuation such as `\*`, `\[`, `\\`, and a backslash before a backtick is treated as literal text in preview. Malformed markers should not stop projection of later valid spans. Images, other line marker hiding, and parser-grade CommonMark behavior should be added incrementally with source mapping tests.

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

Marker autocomplete is an input helper, not a separate document model. When the user completes a supported source marker at a caret, Hanji may insert the matching closing marker and place the caret inside the generated source. Typing the third backtick or tilde on an otherwise empty fence line inserts a matching closing fence below and places the caret on the empty code content line. Typing the second asterisk of a strong marker inserts the closing strong marker and places the caret between the marker pair. Typing a supported Markdown delimiter over a selection, such as an asterisk, tilde, or backtick, wraps the selected source and keeps the original content selected inside the generated markers; repeating the delimiter grows the marker run without replacing the selection. Direct marker typing preserves the selected source exactly and does not normalize existing Markdown. Typing a strong marker at an existing closing strong marker should move the caret over that marker without changing the document. Backspace in an empty generated strong pair removes both marker runs. Autocomplete should only fire for unambiguous edits; malformed partial markers should remain ordinary source edits. A malformed strong span earlier in the document should not disable later strong autocomplete after whitespace or a line break.

Formatting commands express stronger intent than raw marker typing. Commands such as strong and emphasis wrap the selected source semantically, so they may flatten recognized inner spans that already use the same delimiter while preserving different syntax such as inline code. The link command wraps selected single-line source as `[label](https://)` and selects the placeholder destination so it can be replaced immediately. Running the link command inside an existing inline link selects that link's destination without changing source text.

For caret placement, the exact visible start of hidden inline content maps to the outer source start, and the exact visible end maps to the outer source end. That means clicking before `bold` in hidden `**bold**` places the caret at `|**bold**`, while clicking after `bold` places it at `**bold**|`. Interior content offsets still map inside the content. This keeps edge clicks and selections honest: grabbing the visual boundary grabs the Markdown span boundary, while editing from inside the content stays focused on content.

For headings, ATX markers become a heading only after the hash run is followed by whitespace. A pending marker such as `#` remains a normal paragraph source. Once recognized, inactive headings render as preview text: `## Title` renders as `Title`. When the caret or active selection enters the heading line, the source is revealed as `## Title`. Only the hash run is marked as Markdown syntax and highlighted in green. The following whitespace and content remain visible with heading font size and weight.

For horizontal rules, a line containing at least three hyphens with only optional leading indentation and trailing whitespace renders as a divider line in inactive preview. When the caret or active selection enters the horizontal rule line, the raw source is revealed and the hyphen run is highlighted as Markdown syntax.

For escapes, the backslash is a Markdown marker. In inactive preview, `\*` renders as `*`, `\[` renders as `[`, and `\\` renders as `\`. When the caret or active selection enters the escaped source range, or any non-whitespace token containing that escaped source, the backslash is revealed with a muted syntax color. This makes escaped pairs such as `\*literal\*` reveal both backslashes while the caret is inside `literal`. Escaped punctuation must not start Markdown spans.

For inline links, inactive preview hides `[`, `](`, the destination, and `)`, leaving only the label as editable visible text. The label is underlined with a yellow link color. Strong and emphasis wrappers around the whole link combine with the link presentation, so styled links remain clickable while also using bold and italic text where applicable. When the caret or active selection enters any part of the link source, Hanji reveals the full `[label](destination)` source. Link syntax markers are highlighted in green when revealed; the destination remains normal source text. Clicking a link label opens `http` and `https` destinations with the system browser. Other URL schemes are ignored by the opener.

For autolinks, inactive preview hides the surrounding `<` and `>` while leaving the URL as yellow underlined editable visible text. When the caret or active selection enters any part of the autolink source, Hanji reveals the full `<url>` source. Autolink syntax markers are highlighted in green when revealed. Clicking an `http` or `https` autolink opens it with the system browser.

For raw URLs, Hanji does not insert or hide Markdown syntax. Plain `http://...` and `https://...` text remains visible as typed, uses the same yellow underlined link styling, and can be clicked when the detected URL has a supported scheme. Strong and emphasis wrappers around the whole URL combine with the link presentation. Common trailing sentence punctuation such as `.`, `,`, `!`, `?`, `)`, and `]` stays outside the clickable URL. Raw URL detection must not create nested link spans inside inline code, inline links, or angle-bracket autolinks.

For fenced code blocks, Hanji treats the block as a literal source-backed surface. A closed backtick or tilde fence pair hides the opening and closing fence text in inactive preview while keeping those fence lines in the visual block, so revealing source does not move surrounding lines up or down. Each content line renders as literal code text with a continuous rounded block background and an inner text inset. Inline Markdown inside the block is not projected: `**bold**`, `[link](url)`, and `` `inline` `` stay visible as source text. When the caret or active selection enters any source position inside the code block, including a content line, the opening and closing fence source is revealed. Only the fence marker run is highlighted as Markdown syntax; the optional info string remains normal source text. An unclosed fence remains plain source text until a matching closing fence exists.

For blockquotes, the visible start of the line maps after the hidden `> ` marker. Pressing Enter in a non-empty blockquote line continues the blockquote by inserting a new `> ` marker. Pressing Enter again on an empty blockquote marker line removes that marker and leaves a clean normal line. Consecutive blockquote lines should render as one visual quote block with a continuous quote bar; an unquoted line breaks the run.

For list items, the visible start of the line maps after the hidden list marker and aligns with normal paragraph text. The renderer draws the visual bullet, ordered marker, or checkbox separately in the gutter to the left of editable text. Leading spaces before the list marker are treated as the nesting indent for supported list previews, so nested list items remain source-backed list lines instead of falling back to plain text. A normal click in the marker gutter places the caret at the content start, while dragging into the marker gutter resolves to the marker source range so selection can reveal and select the raw marker. When the caret or active selection enters the hidden list marker source range, the raw marker should be revealed and the separate visual marker should be hidden. Task checkbox previews are source-backed controls: clicking an unchecked preview updates `[ ]` to `[x]`, and clicking a checked preview updates `[x]` or `[X]` to `[ ]`. Pressing Enter in a non-empty list item continues the list with the same unordered marker or the next ordered number. Task list items continue as unchecked tasks. Pressing Enter on an empty nested list marker line removes one indentation level first. Pressing Enter again at depth 0 removes that marker and leaves a clean normal line. Pressing Tab in a list item indents the current selected list line or selected list lines by two spaces. Pressing Shift+Tab removes up to two leading spaces from selected list lines. Non-list lines inside the selected range are left unchanged.

For selection placement, source range boundaries remain meaningful. A selection that starts outside an inline span and extends into that span should reveal and select the marker text it crosses. A selection that starts inside the inline content uses the same caret placement rule as editing, so it selects the content without implicitly adding hidden markers.

Double-click selection uses the same word definition as keyboard word movement. A double click inside or at the edge of a word selects that word's source range. Clicking punctuation or whitespace that is not adjacent to a word should keep normal caret placement.

Keyboard selection expansion uses the same source coordinate rules. `Shift+Arrow` extends by visible caret movement, `Shift+Option+Left/Right` extends to the previous or next source word boundary within the current line, and `Shift+Cmd` extends to the current line or document boundary depending on the arrow direction. Left and right movement shortcuts should not cross line boundaries; moving between lines belongs to up and down movement.

Select all selects the full Markdown source range, including hidden markers and structural syntax. It should behave like a normal plain-text editor selection for replacement, copy, and subsequent keyboard selection changes.

Clipboard operations are source-backed. Copy and cut use the selected Markdown source range, including hidden markers, escaped punctuation, destinations, fence lines, and newlines. Paste inserts clipboard text as Markdown source text and does not invoke marker autocomplete, so pasting `**` or a three-backtick fence marker keeps exactly that text instead of generating matching markers.

## Test Scenarios

Projection tests should focus on behavior that can change editing meaning:

- Hidden markers are omitted from the default visible text while content keeps source ranges.
- A caret inside an inline span reveals that span's markers only.
- A caret on an opening or closing marker boundary reveals the span.
- A selection that intersects hidden markers reveals the span.
- A selection spanning multiple inline spans reveals each intersected span.
- A selection starting outside an inline span includes crossed marker text.
- A selection starting inside inline content excludes hidden markers unless the user explicitly extends into them.
- Double-clicking a word selects the word source range with the same boundary rules as keyboard word movement.
- Select all covers the full Markdown source, including currently hidden syntax markers.
- Copy, cut, and paste preserve Markdown source text, including hidden markers and newlines.
- Hidden inline content boundaries map to the outer source edges of the Markdown span.
- Heading recognition starts only after the ATX hash run is followed by whitespace.
- Inactive recognized headings hide the ATX marker and render content as preview text.
- Active recognized headings reveal source, with only the hash run marked as syntax.
- Pending heading markers without trailing whitespace stay visible as paragraph source.
- Inactive horizontal rules hide source and render a divider line.
- Active horizontal rules reveal source, with the hyphen run marked as syntax.
- Hidden blockquote markers are omitted from visible text while visible line starts map after the marker.
- Enter continues non-empty blockquote lines and exits from empty blockquote marker lines.
- Hidden list markers are omitted from visible text while visible line starts map after the marker.
- A caret or selection inside a hidden list marker reveals the raw marker source.
- A pending task marker without trailing whitespace stays visible as source.
- A normal click in the list marker gutter places the caret at content start, while dragging into the gutter reveals and selects marker source.
- Enter continues non-empty list items, outdents empty nested list marker lines, and exits from empty depth-0 list marker lines.
- Tab and Shift+Tab indent or outdent selected list lines while leaving selected non-list lines unchanged.
- Task list markers are hidden with the list marker while checkbox state remains available to the renderer.
- Clicking a checkbox preview toggles only the source state character inside `[ ]`, `[x]`, or `[X]`.
- Backspace and Delete at revealed marker boundaries remove one source character, not the whole formatting span.
- Typing a third backtick or tilde on a valid fence-start line inserts a matching closing fence below and places the caret inside the code block.
- Typing the second asterisk of a strong marker inserts the closing marker and places the caret inside the strong span.
- Strong autocomplete still works after whitespace or a line break even when an earlier malformed strong span remains unclosed.
- Typing supported Markdown delimiter characters over a selection wraps that source and keeps the content selected.
- Direct marker typing over a selection preserves the selected source without normalizing existing Markdown.
- Formatting commands such as strong and emphasis flatten recognized inner spans that already use the same delimiter.
- The link command wraps selected single-line source and selects the destination placeholder.
- Running the link command inside an existing inline link selects the destination source range.
- Typing a strong marker at an existing closing strong marker moves the caret past the marker without dirtying the document.
- Backspace inside an empty generated strong pair removes both marker runs.
- Adjacent or malformed markers do not leak styles into unrelated spans.
- Inline code and strong spans remain independent when one of them becomes malformed.
- Strikethrough spans hide exact `~~` markers in preview and reveal those markers when active.
- Longer tilde delimiter runs such as `~~~text~~~` remain plain text and must not conflict with fenced code blocks.
- Simple inline links hide markers and destinations while preserving source mapping for the label.
- A caret or selection inside link label, marker, or destination reveals the full link source.
- Malformed links and escaped link openings remain plain text.
- Angle-bracket autolinks hide `<` and `>` in preview while preserving source mapping for the URL.
- A caret or selection inside an autolink URL or marker reveals the full autolink source.
- Unsupported or malformed autolinks remain plain text.
- Raw URLs become underlined clickable link text without changing the visible or source text.
- Raw URL detection excludes trailing sentence punctuation and does not override inline code, inline links, or angle-bracket autolinks.
- Strong, emphasis, and strikethrough wrappers compose over mixed inline children, including text, whole links, autolinks, raw URLs, and inline code.
- Nested style wrappers such as `~~***thought***~~` and `***~~thought~~***` keep every applicable style.
- Inline code keeps Markdown-looking text literal even when it is inside an outer strong or emphasis range.
- Inline code content remains literal and blocks Markdown projection inside the code span.
- Escaped punctuation hides only the escape backslash in inactive preview and reveals it at the caret or inside the same non-whitespace token.
- Escaped punctuation must not start emphasis, strong, code, or link spans.
- Revealed escape backslashes use a muted syntax color.
- Clicking a simple inline link label opens only supported `http` and `https` destinations.
- Closed backtick and tilde fenced code blocks hide inactive fence lines while keeping content lines visible.
- Backtick fences and tilde fences do not close each other.
- A caret or selection anywhere inside a fenced code block reveals the opening and closing fence source.
- Inline Markdown inside fenced code content remains literal source text.
- Unclosed fences and closing fences with non-whitespace trailing text remain plain source text.

## Ownership

`hanji-markdown` owns Markdown-specific projection data such as line kinds, inline spans, content ranges, and marker ranges.

`hanji-core` owns source editing primitives such as text ranges, selections, transactions, undo, and grapheme-safe caret boundaries.

`apps/hanji` owns GPUI rendering, hit testing, and shortcut routing. It should consume projection data, translate platform events back into core source positions, and route formatting shortcuts such as strong and inline code through `hanji-markdown` commands instead of editing Markdown markers in renderer code.
