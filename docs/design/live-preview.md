# Live Preview

Status: Current

Live preview hides supported Markdown syntax when it is inactive and reveals the source the user is actively editing. It is a presentation policy over source-backed ranges, not a different editing mode or document model.

## Activation

A projected syntax span or structural line becomes active when:

- the caret lies within its outer source range, including marker boundaries; or
- the active selection intersects its outer source range.

Hover alone does not reveal source. Activating one inline span does not reveal unrelated spans.

## Reveal Rules

- Inactive recognized markers may be hidden or replaced with a visual preview.
- Active markers are rendered as ordinary editable source.
- Marker text uses syntax styling while content retains its semantic typography.
- Unsupported, malformed, pending, or unclosed syntax remains visible plain source.
- When the caret can delete or overwrite a marker, that marker must be visible.
- Hidden source is never selected or edited implicitly; the user must cross its mapped boundary.

## Boundary Placement

The exact visible start of hidden inline content maps to the outer source start, and the exact visible end maps to the outer source end. Interior visible positions map inside the content.

For hidden `**bold**`:

```text
click before bold -> |**bold**
click inside bold -> **bo|ld**
click after bold  -> **bold**|
```

This makes edge clicks select the Markdown span boundary while interior editing remains focused on content.

## Inline Presentation

- Emphasis, strong, strong-emphasis, and strikethrough hide exact recognized delimiters and compose their styles over supported children.
- Inline code treats its content literally and has precedence over Markdown-looking text inside it.
- Inline links hide brackets and destinations while showing a styled label; activation reveals the full source.
- Autolinks hide angle brackets while keeping the URL visible.
- Raw URLs add link presentation without inserting or hiding source.
- Escapes hide the backslash while inactive and reveal it with muted syntax styling when active.

## Block Presentation

- Recognized headings hide their ATX marker while inactive and reveal the source on activation.
- Horizontal rules render as dividers while inactive and source while active.
- Blockquote and list markers are represented in the gutter while content remains source-backed.
- Task markers use a checkbox preview whose state change edits only the task-state source character.
- Closed fenced code blocks keep literal content, hide inactive fences without collapsing their visual lines, and reveal both fences when the block is active.
- Tables render source-backed cells while retaining marker and cell source ranges for navigation and copy.

The exact supported syntax and current presentation are listed in [Markdown Support](../reference/markdown-support.md).

## Raw Source Escape Hatch

Raw Markdown is not a debug representation. Hanji must always preserve a path to inspect and edit the literal source when projection is incomplete, malformed, or surprising.
