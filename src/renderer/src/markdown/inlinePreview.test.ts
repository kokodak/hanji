import assert from 'node:assert/strict';
import { EditorState } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { addInlinePreviewDecorations, getInlinePreviewCursorTarget, lineIsOnlyInlineCode } from './inlinePreview';
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
    name: 'collapses inline syntax markers in preview mode',
    run() {
      const code = decorationSummariesFor('`code`');
      const emphasis = decorationSummariesFor('*emphasis*');
      const strong = decorationSummariesFor('**strong**');

      assert.deepEqual(
        code.filter((item) => item.from === 0 || item.to === '`code`'.length).map((item) => item.className),
        [undefined, undefined]
      );
      assert.deepEqual(
        emphasis.filter((item) => item.from === 0 || item.to === '*emphasis*'.length).map((item) => item.className),
        [undefined, undefined]
      );
      assert.deepEqual(
        strong.filter((item) => item.from === 0 || item.to === '**strong**'.length).map((item) => item.className),
        [undefined, undefined]
      );
      assert.equal(code.some((item) => item.className === 'cm-markdown-syntax-hidden'), false);
      assert.equal(emphasis.some((item) => item.className === 'cm-markdown-syntax-hidden'), false);
      assert.equal(strong.some((item) => item.className === 'cm-markdown-syntax-hidden'), false);
    }
  },
  {
    name: 'previews emphasis after one strong marker is deleted',
    run() {
      const leftDeleted = decorationSummariesFor('*qwe**');
      const rightDeleted = decorationSummariesFor('**qwe*');

      assert.deepEqual(leftDeleted, [
        { from: 0, to: 1, className: undefined },
        { from: 1, to: 4, className: 'cm-live-emphasis' },
        { from: 4, to: 5, className: undefined }
      ]);
      assert.deepEqual(rightDeleted, [
        { from: 1, to: 2, className: undefined },
        { from: 2, to: 5, className: 'cm-live-emphasis' },
        { from: 5, to: 6, className: undefined }
      ]);
    }
  },
  {
    name: 'keeps inline syntax visible during range selection',
    run() {
      const classes = decorationClassesFor('*emphasis*', { anchor: 0, head: '*emphasis*'.length });

      assert.equal(classes.includes('cm-live-emphasis'), true);
      assert.equal(classes.includes('cm-markdown-syntax-hidden'), false);
    }
  },
  {
    name: 'moves clicks at trailing inline preview syntax outside the markers',
    run() {
      assert.equal(getInlinePreviewCursorTarget(0, '**strong**', '**strong'.length), '**strong**'.length);
      assert.equal(getInlinePreviewCursorTarget(0, '*emphasis*', '*emphasis'.length), '*emphasis*'.length);
      assert.equal(getInlinePreviewCursorTarget(0, '`code`', '`code'.length), '`code`'.length);
      assert.equal(getInlinePreviewCursorTarget(0, '**strong** after', '**strong'.length), null);
    }
  }
];
