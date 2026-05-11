import assert from 'node:assert/strict';
import { EditorState } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { addInlinePreviewDecorations, lineIsOnlyInlineCode } from './inlinePreview';
import type { PendingDecoration } from './types';

interface DecorationSummary {
  from: number;
  to: number;
  className: string | undefined;
}

function decorationSummariesFor(
  lineText: string,
  selection: { anchor: number; head?: number } = { anchor: lineText.length + 1 }
): DecorationSummary[] {
  const pending: PendingDecoration[] = [];
  const view = {
    state: EditorState.create({
      doc: `${lineText}\n`,
      selection
    })
  } as unknown as EditorView;

  addInlinePreviewDecorations(view, pending, 0, lineText);

  return pending.map((item) => ({
    from: item.from,
    to: item.to,
    className: item.decoration.spec.class as string | undefined
  }));
}

function decorationClassesFor(
  lineText: string,
  selection: { anchor: number; head?: number } = { anchor: lineText.length + 1 }
): Array<string | undefined> {
  return decorationSummariesFor(lineText, selection).map((item) => item.className);
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
      const classes = decorationClassesFor('`code_value` and *emphasis*');

      assert.equal(classes.includes('cm-live-code'), true);
      assert.equal(classes.includes('cm-live-emphasis'), true);
    }
  },
  {
    name: 'does not treat underscores as emphasis syntax',
    run() {
      const classes = decorationClassesFor('_ㅂㅈㄷ_ and __strong__');

      assert.equal(classes.includes('cm-live-emphasis'), false);
      assert.equal(classes.includes('cm-live-strong'), false);
      assert.equal(classes.includes('cm-markdown-syntax-hidden'), false);
    }
  },
  {
    name: 'collapses bold and italic syntax markers in preview mode',
    run() {
      const emphasis = decorationSummariesFor('*emphasis*');
      const strong = decorationSummariesFor('**strong**');

      assert.deepEqual(
        emphasis.filter((item) => item.from === 0 || item.to === '*emphasis*'.length).map((item) => item.className),
        [undefined, undefined]
      );
      assert.deepEqual(
        strong.filter((item) => item.from === 0 || item.to === '**strong**'.length).map((item) => item.className),
        [undefined, undefined]
      );
      assert.equal(emphasis.some((item) => item.className === 'cm-markdown-syntax-hidden'), false);
      assert.equal(strong.some((item) => item.className === 'cm-markdown-syntax-hidden'), false);
    }
  },
  {
    name: 'keeps inline syntax visible during range selection',
    run() {
      const classes = decorationClassesFor('*emphasis*', { anchor: 0, head: '*emphasis*'.length });

      assert.equal(classes.includes('cm-live-emphasis'), true);
      assert.equal(classes.includes('cm-markdown-syntax-hidden'), false);
    }
  }
];
