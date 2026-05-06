# QA Harness

This document tracks hands-on editing issues that should become repeatable checks for Lithe. Each item should be fixed with a small reproduction note, an expected behavior, and a manual verification step until automated editor tests exist.

## Current Focus

The first harness pass focuses on live Markdown editing quality:

- Cursor state should remain singular, stable, and visually aligned after edits.
- Markdown source syntax should reveal only where the user is actively editing.
- Continuation rules should make common Markdown writing feel natural.
- Layout chrome should stay fixed while only the editor surface scrolls.

## Editing Checks

| ID | Area | Reproduction | Expected Behavior | Status |
| --- | --- | --- | --- | --- |
| QA-001 | Cursor rendering | Type text, press Backspace repeatedly, and watch the caret immediately after deletion. | The caret moves with the document change and only one caret is visible. Test: `src/renderer/src/editor/cursorStyle.test.ts`. | Fixed pending manual verification |
| QA-002 | Heading live preview | Type `# Heading`, then press Enter. | The completed heading switches back to preview styling immediately after the cursor leaves the line. Test: `src/renderer/src/markdown/livePreview.test.ts`. | Fixed pending manual verification |
| QA-003 | List continuation | Type `- item`, then press Enter. Repeat from several cursor positions and after live preview toggles. | A new line starts with the same unordered-list marker when the previous list item has content. Tests: `src/renderer/src/editor/keymaps.test.ts`, `src/renderer/src/markdown/livePreview.test.ts`. | Fixed pending manual verification |
| QA-004 | Tab handling | Press Tab inside normal text, list items, and indented Markdown blocks. | Tab inserts four spaces or indents the current selection by four spaces. Shift+Tab should outdent where applicable. Test: `src/renderer/src/editor/keymaps.test.ts`. | Fixed pending manual verification |
| QA-005 | Markdown indentation | Create nested lists, blockquotes with nested content, and mixed indentation. | Markdown indentation is preserved in source and rendered accurately in preview decorations. Indented plain text should remain plain text rather than becoming a code block preview. Tests: `src/renderer/src/editor/keymaps.test.ts`, `src/renderer/src/markdown/livePreview.test.ts`. | Fixed pending manual verification |
| QA-006 | Inline code isolation | Type `` `ego_path_conflict_candidate` `` and move the cursor away. | The inline code span renders as code, underscores remain literal, and emphasis is not applied inside the code span. Test: `src/renderer/src/markdown/inlinePreview.test.ts`. | Fixed pending manual verification |
| QA-007 | Tables | Type a GitHub-Flavored Markdown table. | The table is recognized and rendered in live preview without breaking editing on the active line. Test: `src/renderer/src/markdown/livePreview.test.ts`. | Fixed pending manual verification |
| QA-008 | Frontmatter | Add a YAML frontmatter block at the top of the document. | Frontmatter remains readable and does not interfere with Markdown table or horizontal-rule parsing. Test: `src/renderer/src/markdown/livePreview.test.ts`. | Fixed pending manual verification |
| QA-009 | Horizontal rules | Type `---`, `***`, and `___` as standalone lines. | Horizontal rules render as separators when the cursor leaves the line. Test: `src/renderer/src/markdown/livePreview.test.ts`. | Fixed pending manual verification |

## Layout And Scroll Checks

| ID | Area | Reproduction | Expected Behavior | Status |
| --- | --- | --- | --- | --- |
| QA-010 | Sidebar position | Open a long document and scroll the editor. | The left sidebar stays fixed and does not move with editor content. Test: `src/renderer/src/app/layoutStyle.test.ts`. | Fixed pending manual verification |
| QA-011 | Scroll containment | Use fast trackpad scrolling with momentum. | Scroll momentum and bounce are contained to the editor surface. The clock and sidebar remain visually stable. Test: `src/renderer/src/app/layoutStyle.test.ts`. | Fixed pending manual verification |

## Verification Rhythm

For each fix:

1. Add or update the reproduction note if the bug is narrower than described here.
2. Add or update a compact scenario test for the behavior whenever the surface can be exercised without brittle visual timing.
3. Fix the smallest editor, Markdown, or layout surface that owns the behavior.
4. Run `npm run typecheck`.
5. Run `npm run test`.
6. Manually verify the affected QA item in the running app.
7. Mark the item as fixed only after the expected behavior is visible in the app.

Scenario tests should stay focused on one user-visible rule at a time. Prefer small editor-state inputs and clear expected document, selection, parser, or decoration outputs over broad end-to-end flows. If a behavior currently needs manual visual verification, keep the manual step and add the closest stable unit or integration test around the underlying rule.

Use `npm run web:dev` for fast browser checks while iterating on renderer behavior. The web runtime uses a local browser QA Space instead of native Tauri filesystem commands, so editor behavior can be inspected at `http://127.0.0.1:1420/`. Final user-facing fixes should still be spot-checked in the Tauri app when they depend on native shell behavior.

Each closed QA item should keep or gain a test reference before it is marked fixed.
