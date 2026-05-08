import assert from 'node:assert/strict';
import { Text } from '@codemirror/state';
import {
  collectYamlFrontmatterBlock,
  getTableCursorTarget,
  lineIsHorizontalRule,
  nextHoverLineAfterEditorUpdate,
  safePosAtCoords
} from './livePreview';

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
    name: 'ignores transient coordinate lookup failures',
    run() {
      const view = {
        posAtCoords() {
          throw new Error('No tile at position 73');
        }
      };

      assert.equal(safePosAtCoords(view, { x: 10, y: 10 }), null);
    }
  },
  {
    name: 'returns coordinate positions when lookup succeeds',
    run() {
      const view = {
        posAtCoords() {
          return 4;
        }
      };

      assert.equal(safePosAtCoords(view, { x: 10, y: 10 }), 4);
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
    name: 'collects a YAML frontmatter block at the top of the document',
    run() {
      const block = collectYamlFrontmatterBlock(Text.of(['---', 'title: Draft', 'tags:', '  - qa', '---', '# Body']));

      assert.deepEqual(block, { startLine: 1, endLine: 5 });
    }
  },
  {
    name: 'does not collect horizontal rules away from the document start as frontmatter',
    run() {
      const block = collectYamlFrontmatterBlock(Text.of(['# Body', '', '---']));

      assert.equal(block, null);
    }
  },
  {
    name: 'moves a rendered table cursor to the next existing line',
    run() {
      const doc = Text.of(['| Name | Status |', '| --- | --- |', '| Lithe | Ready |', 'after']);

      assert.deepEqual(
        getTableCursorTarget(
          doc,
          {
            startLine: 1,
            endLine: 3,
            headers: ['Name', 'Status'],
            rows: [['Lithe', 'Ready']]
          },
          doc.line(2).from
        ),
        { anchor: doc.line(4).from, insertBreakAt: null }
      );
    }
  },
  {
    name: 'creates a following line for a rendered table at document end',
    run() {
      const doc = Text.of(['| Name | Status |', '| --- | --- |', '| Lithe | Ready |']);

      assert.deepEqual(
        getTableCursorTarget(
          doc,
          {
            startLine: 1,
            endLine: 3,
            headers: ['Name', 'Status'],
            rows: [['Lithe', 'Ready']]
          },
          doc.line(3).to
        ),
        { anchor: doc.line(3).to + 1, insertBreakAt: doc.line(3).to }
      );
    }
  },
];
