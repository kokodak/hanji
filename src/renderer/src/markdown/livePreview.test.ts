import assert from 'node:assert/strict';
import { lineIsHorizontalRule, lineIsIndentedCodeBlock, nextHoverLineAfterEditorUpdate } from './livePreview';

export const tests = [
  {
    name: 'clears stale hover preview state after document edits',
    run() {
      assert.equal(
        nextHoverLineAfterEditorUpdate(1, {
          docChanged: true,
          selectionSet: false,
          viewportChanged: false
        }),
        null
      );
    }
  },
  {
    name: 'clears stale hover preview state after selection changes',
    run() {
      assert.equal(
        nextHoverLineAfterEditorUpdate(1, {
          docChanged: false,
          selectionSet: true,
          viewportChanged: false
        }),
        null
      );
    }
  },
  {
    name: 'keeps hover preview state for viewport-only updates',
    run() {
      assert.equal(
        nextHoverLineAfterEditorUpdate(1, {
          docChanged: false,
          selectionSet: false,
          viewportChanged: true
        }),
        1
      );
    }
  },
  {
    name: 'recognizes standalone Markdown horizontal rules',
    run() {
      assert.equal(lineIsHorizontalRule('---'), true);
      assert.equal(lineIsHorizontalRule('***'), true);
      assert.equal(lineIsHorizontalRule('___'), true);
      assert.equal(lineIsHorizontalRule(' - - - '), true);
    }
  },
  {
    name: 'rejects non-rule lines that contain rule characters',
    run() {
      assert.equal(lineIsHorizontalRule('--'), false);
      assert.equal(lineIsHorizontalRule('---- text'), false);
      assert.equal(lineIsHorizontalRule('    ---'), false);
      assert.equal(lineIsHorizontalRule('*** emphasis ***'), false);
    }
  },
  {
    name: 'recognizes Markdown indented code block lines',
    run() {
      assert.equal(lineIsIndentedCodeBlock('    const value = 1;'), true);
      assert.equal(lineIsIndentedCodeBlock('        deeply indented code'), true);
    }
  },
  {
    name: 'does not treat nested Markdown structures as indented code',
    run() {
      assert.equal(lineIsIndentedCodeBlock('    - nested item'), false);
      assert.equal(lineIsIndentedCodeBlock('    1. nested item'), false);
      assert.equal(lineIsIndentedCodeBlock('    - [ ] nested task'), false);
      assert.equal(lineIsIndentedCodeBlock('    > nested quote'), false);
    }
  }
];
