# Editing Policy

Status: Current

Editing policy translates user intent into deterministic source edits. The same policy must run for the GPUI app and future web adapters.

## Input Origins

Text replacement distinguishes two origins:

- `Typing` is interactive input and may apply Markdown completion, wrapping, or marker skipping.
- `Literal` preserves the supplied source exactly and is used for paste and IME composition updates.

This distinction prevents paste or intermediate composition text from triggering autocomplete. A new input mode should be added only when it changes engine-owned semantics such as history grouping.

## Logical Commands

Commands express intent independently of keys or widgets:

- deletion by grapheme, word, or line;
- newline;
- list indentation and outdentation;
- strong, emphasis, inline code, and link formatting;
- task toggling at a source offset;
- undo and redo.

Text insertion is not a command. All inserted platform text uses `TextInput`, keeping typing policy on the only insertion path.

## Marker Input

Typing policy currently supports:

- inserting a closing strong marker after a completed opening marker;
- inserting a matching closing backtick or tilde fence;
- moving over an existing closing strong marker;
- removing an empty generated strong pair with Backspace;
- wrapping a selection with supported delimiter input while preserving the selection;
- leaving ambiguous, malformed, or unsupported marker input as literal source.

Autocomplete is a convenience over source. It must not normalize unrelated Markdown or create hidden state.

## Structural Newline Policy

- A non-empty blockquote continues with the same marker.
- An empty blockquote marker exits the blockquote.
- A non-empty list item continues with the corresponding marker.
- Ordered lists advance the displayed number.
- Task list items continue as unchecked tasks.
- An empty nested list item first removes one indentation level.
- An empty top-level list marker exits the list.
- A table cell newline inserts the source representation used for an in-cell line break rather than splitting the Markdown row.

## Formatting Commands

Formatting commands carry stronger semantic intent than raw delimiter typing. They may replace an equivalent recognized wrapper while preserving other syntax. The link command wraps selected single-line source with a placeholder destination; inside an existing link it selects the destination without changing source.

## Tables and Source-Aware Interaction

Table policy uses projected cell ranges to:

- keep deletion and caret movement within valid cell boundaries;
- represent an in-cell line break as `<br>` source;
- navigate across cells without treating hidden separators as ordinary visible text;
- expand copy ranges when a visual cell selection must include its structural source.

The renderer supplies hit-tested intent; `hanji-markdown` plans source edits; `hanji-editor` coordinates selection and mutation.

## Selection, Navigation, and Clipboard

- Horizontal and word movement resolve to grapheme-safe source offsets.
- Pixel-based vertical movement remains a platform responsibility because it depends on wrapping and layout.
- Select all covers the entire Markdown source, including hidden syntax.
- Copy and cut return selected Markdown source, including markers and destinations.
- Paste uses literal input and preserves clipboard source exactly.

## Policy Ownership

Syntax-agnostic buffer rules belong in `hanji-core`. Syntax-dependent planning belongs in `hanji-markdown`. Operation coordination belongs in `hanji-editor`. IME lifecycle, key bindings, pixels, and clipboard delivery belong to the platform adapter.

A policy function should be deterministic and independently testable from source, selection, and intent. It should return an explicit edit plan or transaction rather than mutate UI state.
