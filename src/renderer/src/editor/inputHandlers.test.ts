import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { EditorState } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import {
  getPlainTextPasteReplacement,
  lineIsEmptyListMarker,
  normalizePastedText,
  pastedTextStartsWithListItem
} from './inputHandlers';

const createEditorSource = readFileSync(new URL('./createEditor.ts', import.meta.url), 'utf8');

export const tests = [
  {
    name: 'normalizes pasted text line endings',
    run() {
      assert.equal(normalizePastedText('first\r\nsecond\rthird'), 'first\nsecond\nthird');
    }
  },
  {
    name: 'preserves tab-delimited pasted text as plain text',
    run() {
      assert.equal(normalizePastedText('Name\tStatus\nLithe\tOK'), 'Name\tStatus\nLithe\tOK');
    }
  },
  {
    name: 'detects empty live preview list marker lines',
    run() {
      assert.equal(lineIsEmptyListMarker('- '), true);
      assert.equal(lineIsEmptyListMarker('- [ ] '), true);
      assert.equal(lineIsEmptyListMarker('  1. '), true);
      assert.equal(lineIsEmptyListMarker('- item'), false);
    }
  },
  {
    name: 'detects pasted text that starts with list items',
    run() {
      assert.equal(pastedTextStartsWithListItem('- item\n- next'), true);
      assert.equal(pastedTextStartsWithListItem('- [x] done\n- [ ] next'), true);
      assert.equal(pastedTextStartsWithListItem('plain\n- item'), false);
    }
  },
  {
    name: 'replaces an empty task marker when pasting task list items',
    run() {
      const state = EditorState.create({
        doc: '- [ ] ',
        selection: { anchor: '- [ ] '.length }
      });
      const view = { state } as unknown as EditorView;
      const replacement = getPlainTextPasteReplacement(view, '- [ ] '.length, '- [ ] '.length, '- [ ] qwe\r\n- [ ] qwe');

      assert.deepEqual(replacement, {
        from: 0,
        to: '- [ ] '.length,
        insert: '- [ ] qwe\n- [ ] qwe'
      });
    }
  },
  {
    name: 'keeps an empty list marker when pasted text is normal prose',
    run() {
      const state = EditorState.create({
        doc: '- ',
        selection: { anchor: '- '.length }
      });
      const view = { state } as unknown as EditorView;
      const replacement = getPlainTextPasteReplacement(view, '- '.length, '- '.length, 'plain text');

      assert.deepEqual(replacement, {
        from: '- '.length,
        to: '- '.length,
        insert: 'plain text'
      });
    }
  },
  {
    name: 'installs the plain text paste handler before editor input helpers',
    run() {
      assert.match(createEditorSource, /handlePlainTextPaste,\n\s+handleBacktickInput,/);
    }
  }
];
