import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { EditorState, Text } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import {
  collectFencedCodeBlocks,
  getFencedCodeBlockForLine,
  getFencedCodeLineDecoration,
  getPreviewCodeLineDecoration,
  isActiveFencedCodeBlock
} from './fencedCode';

const styles = readFileSync(new URL('../styles.css', import.meta.url), 'utf8');

function getRuleBody(selector: string): string {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = new RegExp(`${escapedSelector}\\s*\\{([^}]*)\\}`).exec(styles);

  assert.ok(match, `Expected ${selector} rule to exist.`);

  return match[1];
}

function text(lines: string[]): Text {
  return Text.of(lines);
}

export const tests = [
  {
    name: 'collects closed backtick fenced code blocks',
    run() {
      const blocks = collectFencedCodeBlocks(text(['before', '```ts', 'const ok = true;', '```', 'after']));

      assert.deepEqual(blocks, [
        {
          startLine: 2,
          endLine: 4,
          marker: '```',
          language: 'ts'
        }
      ]);
    }
  },
  {
    name: 'collects tilde fenced code blocks with plain text fallback',
    run() {
      const blocks = collectFencedCodeBlocks(text(['~~~', 'plain', '~~~']));

      assert.deepEqual(blocks, [
        {
          startLine: 1,
          endLine: 3,
          marker: '~~~',
          language: 'plain text'
        }
      ]);
    }
  },
  {
    name: 'requires the closing fence to match marker family and length',
    run() {
      const blocks = collectFencedCodeBlocks(text(['````js', 'code', '```', 'still code', '````']));

      assert.deepEqual(blocks, [
        {
          startLine: 1,
          endLine: 5,
          marker: '````',
          language: 'js'
        }
      ]);
    }
  },
  {
    name: 'finds the fenced code block for a document line',
    run() {
      const blocks = collectFencedCodeBlocks(text(['```', 'inside', '```']));

      assert.deepEqual(getFencedCodeBlockForLine(blocks, 2), blocks[0]);
      assert.equal(getFencedCodeBlockForLine(blocks, 4), null);
    }
  },
  {
    name: 'does not enter fenced code edit mode for range selections',
    run() {
      const doc = text(['before', '```ts', 'const ok = true;', '```', 'after']);
      const block = collectFencedCodeBlocks(doc)[0];
      const view = {
        state: EditorState.create({
          doc,
          selection: { anchor: 0, head: doc.length }
        })
      } as unknown as EditorView;

      assert.equal(isActiveFencedCodeBlock(view, block), false);
    }
  },
  {
    name: 'enters fenced code edit mode when the cursor is inside the block',
    run() {
      const doc = text(['before', '```ts', 'const ok = true;', '```', 'after']);
      const block = collectFencedCodeBlocks(doc)[0];
      const cursor = doc.line(3).from;
      const view = {
        state: EditorState.create({
          doc,
          selection: { anchor: cursor }
        })
      } as unknown as EditorView;

      assert.equal(isActiveFencedCodeBlock(view, block), true);
    }
  },
  {
    name: 'assigns edit-mode line decorations by fence position',
    run() {
      const block = collectFencedCodeBlocks(text(['```', 'inside', '```']))[0];

      assert.equal(getFencedCodeLineDecoration(block, 1).spec.class, 'cm-live-codeblock cm-live-codeblock-first');
      assert.equal(getFencedCodeLineDecoration(block, 2).spec.class, 'cm-live-codeblock');
      assert.equal(getFencedCodeLineDecoration(block, 3).spec.class, 'cm-live-codeblock cm-live-codeblock-last');
    }
  },
  {
    name: 'assigns preview-mode line decorations to content lines',
    run() {
      const block = collectFencedCodeBlocks(text(['```', 'one', 'two', '```']))[0];

      assert.equal(getPreviewCodeLineDecoration(block, 2).spec.class, 'cm-live-codeblock cm-live-codeblock-first');
      assert.equal(getPreviewCodeLineDecoration(block, 3).spec.class, 'cm-live-codeblock cm-live-codeblock-last');
    }
  },
  {
    name: 'keeps fenced code preview lines full width',
    run() {
      const lineRule = getRuleBody('#editor .cm-line.cm-live-codeblock');
      const blockRule = getRuleBody('#editor .cm-live-codeblock');

      assert.match(lineRule, /width:\s*100%;/);
      assert.match(blockRule, /width:\s*100%;/);
    }
  }
];
