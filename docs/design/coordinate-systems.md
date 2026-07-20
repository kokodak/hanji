# Coordinate Systems

Status: Current

Hanji crosses several coordinate spaces. Every API and document should name the space it uses; a bare `offset` is meaningful only inside a context that has already established its unit.

## Source Offsets

The canonical engine coordinate is a zero-based UTF-8 byte offset into the Markdown source.

- `TextRange` is a half-open source range `[start, end)`.
- Edit boundaries must be valid Unicode grapheme-cluster boundaries.
- `TextPosition` represents a zero-based line and grapheme column and can be converted to or from a source offset.
- Line ranges and Markdown projection ranges are source byte ranges unless explicitly named visible ranges.

Byte offsets make slicing and source mapping direct. Grapheme validation prevents caret movement, selection, or deletion from splitting combined characters, emoji sequences, or combining marks.

## Directional Selections

A public `TextSelection` stores `anchor` and `head`, both in source offsets. Its ordered edit range is `min(anchor, head)..max(anchor, head)`, but the direction is preserved so platform selections do not flip after an operation.

The current facade supports one directional selection. The lower-level core selection type can represent multiple ranges, but multi-selection is not part of the public editor contract.

## Visible Offsets

A projected visible offset is a UTF-8 byte offset into one projected line's visible text. It is temporary and cannot be saved or sent directly to an editor mutation.

Visible ranges and source ranges may have different lengths because Markdown markers can be hidden. Mapping a visible boundary back to source requires `Before` or `After` affinity when hidden source occupies that boundary.

Source and visible ranges must remain distinct even when they happen to contain the same numbers.

## Platform Offsets

GPUI text input and browser DOM APIs use UTF-16 code-unit offsets. Platform offsets are untrusted until an adapter:

1. validates that the UTF-16 position is within the platform string;
2. converts it to a UTF-8 source offset;
3. snaps or rejects positions that do not land on a grapheme boundary;
4. calls the portable editor with source coordinates.

Outgoing source ranges are converted back to UTF-16 only at the platform boundary. `hanji-core` owns conversion helpers; adapters own when to use them.

## Pixel Coordinates

Points, bounds, line heights, wrapping rows, and scroll offsets belong exclusively to the renderer. They are resolved through current layout snapshots into visible positions and then source positions.

Pixel coordinates must never appear in `hanji-core`, `hanji-markdown`, or `hanji-editor` public editing contracts.

## Naming Rules

- Use `source_range` and `source_offset` for UTF-8 document coordinates.
- Use `visible_range` and `visible_offset` for projected-line coordinates.
- Use `utf16_range` and `utf16_offset` at native or browser text boundaries.
- Use `point`, `bounds`, `x`, and `y` only in platform layout code.
- Document affinity whenever hidden markers make a boundary ambiguous.
