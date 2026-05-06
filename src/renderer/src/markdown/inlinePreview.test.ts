import assert from 'node:assert/strict';
import { lineIsOnlyInlineCode } from './inlinePreview';

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
  }
];
