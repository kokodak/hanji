# Persistence

Status: Current

Hanji treats local Markdown files as durable state. Persistence is intentionally a native adapter around the portable editor, not a feature of the editor engine itself.

## Document Session

`hanji-storage::DocumentSession` owns:

- the current file path;
- a `hanji-editor::Editor`;
- the source from the last successful save;
- a monotonic text revision;
- the revision observed at the last successful save.

The session forwards `set_selection`, `replace_text`, and `execute` to the editor. It exposes the editor only as `&Editor`, preventing callers from changing source without dirty-state tracking.

## Dirty and Revision Semantics

Dirty state answers one question: does the current source differ from the last successfully saved source?

It is computed from source equality, not revision equality. This means an edit marks the session dirty, but undoing exactly back to the saved source makes it clean again.

The revision is a monotonic notification counter. It increments for each successful operation whose `Update::text_changed()` is true. Selection-only operations and no-op commands do not increment it.

## Open and Save

Opening reads the complete UTF-8 Markdown file into a new editor and establishes that source as the saved baseline.

Saving performs these steps:

1. Copy the editor's current source.
2. Create a unique temporary file beside the destination.
3. Write all bytes and synchronize the temporary file.
4. Rename the temporary file over the destination.
5. Best-effort synchronize the parent directory.
6. Update the saved baseline only after the write succeeds.

On failure, the temporary file is removed when possible and the session keeps its previous path and saved baseline. `save_as` updates the active path only after a successful write.

## Boundaries

- File dialogs and unsaved-change prompts belong to `apps/hanji`.
- File bytes, paths, atomic replacement, and saved-state tracking belong to `hanji-storage`.
- Source editing and history belong to `hanji-editor` and lower layers.
- Browser persistence is a future platform adapter and must not be added to `hanji-storage`.

Hanji currently reads and writes the whole file. External-change detection, conflict handling, autosave, file watching, and recovery journals are not implemented and should be designed explicitly before being added.
