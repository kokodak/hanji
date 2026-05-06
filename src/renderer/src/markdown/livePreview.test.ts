import assert from 'node:assert/strict';
import { Text } from '@codemirror/state';
import {
  collectMarkdownTables,
  collectYamlFrontmatterBlock,
  lineIsHorizontalRule,
  nextHoverLineAfterEditorUpdate
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
    name: 'collects GitHub-Flavored Markdown tables',
    run() {
      const tables = collectMarkdownTables(Text.of(['| Name | Status |', '| --- | --- |', '| QA | Open |', '', 'after']));

      assert.deepEqual(tables, [
        {
          startLine: 1,
          endLine: 3,
          headers: ['Name', 'Status'],
          rows: [['QA', 'Open']]
        }
      ]);
    }
  },
  {
    name: 'rejects pipe text without a table delimiter row',
    run() {
      const tables = collectMarkdownTables(Text.of(['Name | Status', 'QA | Open']));

      assert.deepEqual(tables, []);
    }
  }
];
