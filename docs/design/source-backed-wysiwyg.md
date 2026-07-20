# Source-Backed WYSIWYG

Status: Current

Hanji's visual editor is a projection over Markdown source, not a separate rich-text document. The saved file is always Markdown, and every visible object must remain traceable to the source that produced it.

## Invariants

- UTF-8 Markdown is the only source of truth.
- Formatting changes edit source text rather than a hidden rich document.
- Every projected line, segment, marker, link, task, and table cell retains source ranges.
- Projection, layout, and widget state are disposable derived data.
- Hidden source is never changed from visible coordinates without an explicit mapping.
- Malformed or unsupported Markdown stays editable source instead of being guessed into a richer shape.
- Raw Markdown remains a first-class trust and recovery surface.

## Projection Model

A projection answers four questions:

1. Which source range produced this visible object?
2. Which part of that source is visible content?
3. Which parts are syntax markers or structural source?
4. How does a visible boundary map back to a source boundary?

For example:

```md
This is **bold** text
```

The strong span keeps an outer source range for `**bold**`, a content range for `bold`, and two marker ranges for the opening and closing `**`. The renderer may hide the markers, but an edit can still recover their exact source positions.

## Source and Visible Coordinates

Source coordinates are durable UTF-8 byte offsets. Visible coordinates are offsets into a projected line after hidden markers are removed. A projected visible segment carries both spaces, and styled content also carries an outer source range that includes its markers.

A visible caret boundary can correspond to more than one source position. The visible start of `bold` could mean before `**bold**` or after the opening `**`. Mapping therefore requires explicit boundary affinity; renderer code must not choose implicitly.

The complete coordinate contract is in [Coordinate Systems](coordinate-systems.md).

## Derived Views

The current projection derives:

- line kinds such as headings, lists, blockquotes, code fences, horizontal rules, and tables;
- inline kinds such as emphasis, strong, strikethrough, code, links, autolinks, raw URLs, and escapes;
- source and visible segments with composed inline styles;
- marker, task, link, code-block, and table ranges used by the platform renderer.

Derived data may be cached for performance, but cache contents are never durable document state. Rebuilding a projection from the same source must preserve the same editing meaning.

## Editing from a Projection

Hit testing starts in pixel coordinates, resolves to a projected visible position, maps through explicit affinity to a source offset, and only then updates the editor selection. Text mutation proceeds through the editor facade using source ranges.

```text
pixel -> visible position -> source position -> TextSelection/TextInput/Command
```

Marker reveal and hidden-source behavior are defined in [Live Preview](live-preview.md). Syntax-aware mutations are defined in [Editing Policy](editing-policy.md). The exact syntax currently recognized is listed in [Markdown Support](../reference/markdown-support.md).

## Ownership

- `hanji-markdown` owns projection and visible/source mapping.
- `hanji-editor` coordinates projection queries with selection and mutation policy.
- Platform adapters own layout, pixels, hit testing, and presentation.
- Storage writes source only and never serializes a projection.
