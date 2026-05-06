import assert from 'node:assert/strict';
import { EditorState } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { addInlinePreviewDecorations, lineIsOnlyInlineCode } from './inlinePreview';
import type { PendingDecoration } from './types';

function decorationClassesFor(lineText: string): Array<string | undefined> {
  const pending: PendingDecoration[] = [];
  const view = {
    state: EditorState.create({
      doc: `${lineText}\n`,
      selection: { anchor: lineText.length + 1 }
    })
  } as unknown as EditorView;

  addInlinePreviewDecorations(view, pending, 0, lineText);

  return pending.map((item) => item.decoration.spec.class as string | undefined);
}

export const tests = [
  {
    name: 'detects a line made only of inline code',
    run() {
      assert.equal(lineIsOnlyInlineCode('`code`'), true);
      assert.equal(lineIsOnlyInlineCode('`ego_path_conflict_candidate`'), true);
    }
  },
  {
    name: 'rejects inline code mixed with surrounding text',
    run() {
      assert.equal(lineIsOnlyInlineCode('before `code`'), false);
      assert.equal(lineIsOnlyInlineCode('`code` after'), false);
    }
  },
  {
    name: 'rejects incomplete inline code',
    run() {
      assert.equal(lineIsOnlyInlineCode('`code'), false);
      assert.equal(lineIsOnlyInlineCode('code`'), false);
    }
  },
  {
    name: 'does not apply emphasis preview inside inline code',
    run() {
      const classes = decorationClassesFor('`ego_path_conflict_candidate`');

      assert.equal(classes.includes('cm-live-code'), true);
      assert.equal(classes.includes('cm-live-emphasis'), false);
    }
  },
  {
    name: 'still applies emphasis preview outside inline code',
    run() {
      const classes = decorationClassesFor('`code_value` and _emphasis_');

      assert.equal(classes.includes('cm-live-code'), true);
      assert.equal(classes.includes('cm-live-emphasis'), true);
    }
  }
];
