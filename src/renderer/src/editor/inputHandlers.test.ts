import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { normalizePastedText } from './inputHandlers';

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
    name: 'installs the plain text paste handler before editor input helpers',
    run() {
      assert.match(createEditorSource, /handlePlainTextPaste,\n\s+handleBacktickInput,/);
    }
  }
];
