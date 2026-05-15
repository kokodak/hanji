# Philosophy

Hanji is a light, local-first Markdown editor.

It should feel simple enough for notes, durable enough for plain text writing, and open enough for user plugins.

## North Star

Capture the thought.

Hanji should make it easy to start writing without ceremony, keep documents understandable outside the app, and add power only when the user asks for it.

## Principles

- Light by default: fast startup, minimal chrome, and a quiet editing surface.
- Local first: offline editing is the core workflow, not a fallback.
- Markdown native: plain text files remain readable and useful outside Hanji.
- Source-backed WYSIWYG: visual editing should never hide the Markdown source of truth.
- Extensible by design: plugins should be easy to inspect, install, disable, and remove.
- Calm copy: UI text should be brief, useful, and unobtrusive.

## Product Boundaries

Hanji is not trying to become a database-first workspace, a heavy block editor, or a sync account system.

The app may grow collaboration and plugins later, but those layers should sit on top of a strong local Markdown editor rather than define the core.

## Design Taste

The interface should feel closer to a focused writing surface than a dashboard. Prefer direct manipulation, predictable keyboard behavior, and visible local files over complex navigation or hidden state.
