# Markdown Support

Status: Current

Hanji implements a focused, source-backed subset of Markdown. This is a behavior reference, not a claim of full CommonMark compliance.

## Inline Syntax

| Syntax | Recognition and preview |
| --- | --- |
| Emphasis | Exact single-asterisk delimiters such as `*text*`; markers hide when inactive. |
| Strong | Exact double-asterisk delimiters such as `**text**`; markers hide when inactive. |
| Strong emphasis | Exact triple-asterisk delimiters; strong and emphasis styles compose. |
| Strikethrough | Exact double-tilde delimiters such as `~~text~~`. |
| Inline code | Backtick-delimited content; Markdown-looking content inside stays literal. |
| Inline links | `[label](destination)` with non-empty label and whitespace-free non-empty destination. |
| Autolinks | Angle-bracket `http://` and `https://` URLs. |
| Raw URLs | Plain HTTP and HTTPS URLs without source rewriting; common trailing punctuation is excluded. |
| Escapes | Backslash before supported ASCII punctuation; the backslash hides when inactive. |

Longer or malformed delimiter runs remain source text. Styled wrappers can compose over text, links, autolinks, raw URLs, and inline code while inline-code content retains literal precedence.

## Line and Block Syntax

| Syntax | Recognition and preview |
| --- | --- |
| ATX headings | Hash run followed by whitespace; inactive marker hides and heading typography remains. |
| Horizontal rules | At least three hyphens with optional leading indentation and trailing whitespace; inactive source becomes a divider. |
| Blockquotes | `>` followed by a space; marker hides and a quote gutter is rendered. |
| Unordered lists | Supported bullet marker followed by whitespace; marker moves to the gutter. |
| Ordered lists | Number and supported delimiter followed by whitespace; marker moves to the gutter. |
| Task lists | `[ ]`, `[x]`, or `[X]` at list-content start followed by whitespace; checkbox preview is interactive. |
| Fenced code blocks | Closed backtick or tilde fences using at least three matching markers; content stays literal. |
| Pipe tables | Rows associated with a valid delimiter row; cells retain source-backed ranges. |

Pending headings, incomplete tasks, unclosed fences, mismatched fence characters, and unsupported structures remain visible source.

## Live Preview Interaction

- A caret or selection inside recognized syntax reveals its source markers.
- Hover does not reveal markers.
- Links reveal their complete source when active.
- Clicking supported HTTP or HTTPS link previews opens the system browser.
- Raw URLs are styled and clickable without changing source.
- Code fences reveal together when the caret or selection is inside the block.
- List and blockquote gutters map back to their hidden source ranges.
- Task checkbox clicks toggle only the state character.

The semantic reveal rules are defined in [Live Preview](../design/live-preview.md).

## Editing Assistance

- Strong and fenced-code marker completion for unambiguous typing.
- Selection wrapping with supported marker input.
- Strong-marker skipping and paired Backspace cleanup.
- Blockquote and list continuation or exit on newline.
- Ordered-list number advancement.
- List indentation and outdentation.
- Task continuation as unchecked and source-backed task toggling.
- Table-aware newline, deletion, horizontal movement, and copy behavior.
- Strong, emphasis, inline-code, and link commands.

Paste and IME updates use literal insertion and do not invoke marker completion.

## Known Scope

Images, parser-grade CommonMark coverage, arbitrary nested block structures, and a general block-widget system are not implemented as stable behavior. Unsupported input must remain editable Markdown source.

Focused cases are tested in `crates/hanji-markdown` and GPUI interaction tests. New syntax support should add source-mapping tests before expanding the preview.
