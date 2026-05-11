import assert from 'node:assert/strict';
import { EditorState, type TransactionSpec } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { continueListItem, indentWithSpaces, insertSoftBreak, outdentSpaces } from './keymaps';

class TestEditorView {
  state: EditorState;
  dispatchCount = 0;

  constructor(doc: string, cursor: number = doc.length) {
    this.state = EditorState.create({
      doc,
      selection: { anchor: cursor }
    });
  }

  dispatch(spec: TransactionSpec): void {
    this.dispatchCount += 1;
    this.state = this.state.update(spec).state;
  }
}

function runContinueListItem(doc: string, cursor: number = doc.length): TestEditorView {
  const view = new TestEditorView(doc, cursor);
  const handled = continueListItem(view as unknown as EditorView);

  assert.equal(handled, true);
  assert.equal(view.dispatchCount, 1);

  return view;
}

function runInsertSoftBreak(doc: string, cursor: number = doc.length): TestEditorView {
  const view = new TestEditorView(doc, cursor);
  const handled = insertSoftBreak(view as unknown as EditorView);

  assert.equal(handled, true);
  assert.equal(view.dispatchCount, 1);

  return view;
}

export const tests = [
  {
    name: 'continues unordered bullet lists',
    run() {
      const view = runContinueListItem('- item');

      assert.equal(view.state.doc.toString(), '- item\n- ');
      assert.equal(view.state.selection.main.head, '- item\n- '.length);
    }
  },
  {
    name: 'continues nested unordered bullet lists',
    run() {
      const view = runContinueListItem('  * nested');

      assert.equal(view.state.doc.toString(), '  * nested\n  * ');
      assert.equal(view.state.selection.main.head, '  * nested\n  * '.length);
    }
  },
  {
    name: 'outdents shallow empty bullet markers to the root list',
    run() {
      const view = runContinueListItem('  - ');

      assert.equal(view.state.doc.toString(), '- ');
      assert.equal(view.state.selection.main.head, 2);
    }
  },
  {
    name: 'outdents empty nested bullet markers',
    run() {
      const view = runContinueListItem('        - ');

      assert.equal(view.state.doc.toString(), '    - ');
      assert.equal(view.state.selection.main.head, 6);
    }
  },
  {
    name: 'removes empty root bullet markers',
    run() {
      const view = runContinueListItem('- ');

      assert.equal(view.state.doc.toString(), '');
      assert.equal(view.state.selection.main.head, 0);
    }
  },
  {
    name: 'removes empty root bullet markers with invisible caret text',
    run() {
      const view = runContinueListItem('- \u200b');

      assert.equal(view.state.doc.toString(), '');
      assert.equal(view.state.selection.main.head, 0);
    }
  },
  {
    name: 'outdents empty list markers even when the cursor is inside the marker',
    run() {
      const view = runContinueListItem('    - ', 5);

      assert.equal(view.state.doc.toString(), '- ');
      assert.equal(view.state.selection.main.head, 2);
    }
  },
  {
    name: 'handles Enter when preview selection touches an empty list marker',
    run() {
      const view = new TestEditorView('- ');
      view.dispatch({ selection: { anchor: 0, head: 1 } });

      const handled = continueListItem(view as unknown as EditorView);

      assert.equal(handled, true);
      assert.equal(view.state.doc.toString(), '');
      assert.equal(view.state.selection.main.head, 0);
    }
  },
  {
    name: 'continues task lists with an unchecked marker',
    run() {
      const view = runContinueListItem('- [x] done');

      assert.equal(view.state.doc.toString(), '- [x] done\n- [ ] ');
      assert.equal(view.state.selection.main.head, '- [x] done\n- [ ] '.length);
    }
  },
  {
    name: 'does not remove task text when Enter starts after the marker',
    run() {
      const view = new TestEditorView('- [ ] abc', '- [ ] '.length);
      const handled = continueListItem(view as unknown as EditorView);

      assert.equal(handled, false);
      assert.equal(view.state.doc.toString(), '- [ ] abc');
      assert.equal(view.state.selection.main.head, '- [ ] '.length);
    }
  },
  {
    name: 'keeps text typed after exiting an empty task marker',
    run() {
      const view = runContinueListItem('- [ ] ');
      view.dispatch({
        changes: { from: view.state.selection.main.head, insert: 'abc' },
        selection: { anchor: 'abc'.length }
      });

      const handled = continueListItem(view as unknown as EditorView);

      assert.equal(handled, false);
      assert.equal(view.state.doc.toString(), 'abc');
    }
  },
  {
    name: 'inserts a plain newline after text typed below a task list',
    run() {
      const view = runContinueListItem('- [ ] qwe');
      continueListItem(view as unknown as EditorView);
      view.dispatch({
        changes: { from: view.state.selection.main.head, insert: 'qwe' },
        selection: { anchor: '- [ ] qwe\nqwe'.length }
      });

      const handled = continueListItem(view as unknown as EditorView);

      assert.equal(handled, true);
      assert.equal(view.state.doc.toString(), '- [ ] qwe\nqwe\n');
      assert.equal(view.state.selection.main.head, '- [ ] qwe\nqwe\n'.length);
    }
  },
  {
    name: 'ignores non-list lines',
    run() {
      const view = new TestEditorView('plain text');
      const handled = continueListItem(view as unknown as EditorView);

      assert.equal(handled, false);
      assert.equal(view.dispatchCount, 0);
      assert.equal(view.state.doc.toString(), 'plain text');
    }
  },
  {
    name: 'inserts a Markdown soft break for Shift Enter',
    run() {
      const view = runInsertSoftBreak('hello');

      assert.equal(view.state.doc.toString(), 'hello  \n');
      assert.equal(view.state.selection.main.head, 'hello  \n'.length);
    }
  },
  {
    name: 'continues blockquotes on Shift Enter',
    run() {
      const view = runInsertSoftBreak('> hello');

      assert.equal(view.state.doc.toString(), '> hello  \n> ');
      assert.equal(view.state.selection.main.head, '> hello  \n> '.length);
    }
  },
  {
    name: 'keeps Shift Enter plain inside fenced code',
    run() {
      const doc = '```ts\nconst ok = true;\n```';
      const cursor = '```ts\nconst ok = true;'.length;
      const view = runInsertSoftBreak(doc, cursor);

      assert.equal(view.state.doc.toString(), '```ts\nconst ok = true;\n\n```');
      assert.equal(view.state.selection.main.head, cursor + 1);
    }
  },
  {
    name: 'continues blockquotes on Enter',
    run() {
      const view = runContinueListItem('> hello');

      assert.equal(view.state.doc.toString(), '> hello\n> ');
      assert.equal(view.state.selection.main.head, '> hello\n> '.length);
    }
  },
  {
    name: 'exits blockquotes from an empty quote line',
    run() {
      const view = runContinueListItem('> hello\n> ');

      assert.equal(view.state.doc.toString(), '> hello\n');
      assert.equal(view.state.selection.main.head, '> hello\n'.length);
    }
  },
  {
    name: 'continues numbered lists',
    run() {
      const view = runContinueListItem('1. item');

      assert.equal(view.state.doc.toString(), '1. item\n2. ');
      assert.equal(view.state.selection.main.head, '1. item\n2. '.length);
    }
  },
  {
    name: 'outdents whitespace-only indented lines on Enter',
    run() {
      const view = runContinueListItem('        ');

      assert.equal(view.state.doc.toString(), '    ');
      assert.equal(view.state.selection.main.head, 4);
    }
  },
  {
    name: 'outdents a nested list marker on a second Enter',
    run() {
      const view = runContinueListItem('    - item');
      const handled = continueListItem(view as unknown as EditorView);

      assert.equal(handled, true);
      assert.equal(view.state.doc.toString(), '    - item\n- ');
      assert.equal(view.state.selection.main.head, '    - item\n- '.length);
    }
  },
  {
    name: 'removes the root list marker on a third Enter',
    run() {
      const view = runContinueListItem('    - item');
      continueListItem(view as unknown as EditorView);
      const handled = continueListItem(view as unknown as EditorView);

      assert.equal(handled, true);
      assert.equal(view.state.doc.toString(), '    - item\n');
      assert.equal(view.state.selection.main.head, '    - item\n'.length);
    }
  },
  {
    name: 'inserts four spaces for Tab at an empty cursor',
    run() {
      const view = new TestEditorView('ab', 1);
      const handled = indentWithSpaces(view as unknown as EditorView);

      assert.equal(handled, true);
      assert.equal(view.state.doc.toString(), 'a    b');
      assert.equal(view.state.selection.main.head, 5);
    }
  },
  {
    name: 'indents the current bullet line from any cursor position',
    run() {
      const view = new TestEditorView('- item', 4);
      const handled = indentWithSpaces(view as unknown as EditorView);

      assert.equal(handled, true);
      assert.equal(view.state.doc.toString(), '    - item');
      assert.equal(view.state.selection.main.head, 8);
    }
  },
  {
    name: 'indents the current task line from any cursor position',
    run() {
      const view = new TestEditorView('- [ ] task', 7);
      const handled = indentWithSpaces(view as unknown as EditorView);

      assert.equal(handled, true);
      assert.equal(view.state.doc.toString(), '    - [ ] task');
      assert.equal(view.state.selection.main.head, 11);
    }
  },
  {
    name: 'indents the current numbered list line from any cursor position',
    run() {
      const view = new TestEditorView('1. item', 5);
      const handled = indentWithSpaces(view as unknown as EditorView);

      assert.equal(handled, true);
      assert.equal(view.state.doc.toString(), '    1. item');
      assert.equal(view.state.selection.main.head, 9);
    }
  },
  {
    name: 'indents every selected line by four spaces',
    run() {
      const view = new TestEditorView('one\ntwo\nthree');
      view.dispatch({ selection: { anchor: 0, head: 'one\ntwo'.length } });

      const handled = indentWithSpaces(view as unknown as EditorView);

      assert.equal(handled, true);
      assert.equal(view.state.doc.toString(), '    one\n    two\nthree');
    }
  },
  {
    name: 'outdents selected lines by up to four spaces',
    run() {
      const view = new TestEditorView('    one\n  two\nthree');
      view.dispatch({ selection: { anchor: 0, head: '    one\n  two'.length } });

      const handled = outdentSpaces(view as unknown as EditorView);

      assert.equal(handled, true);
      assert.equal(view.state.doc.toString(), 'one\ntwo\nthree');
    }
  },
  {
    name: 'outdents the current line at an empty cursor',
    run() {
      const view = new TestEditorView('    item', 6);
      const handled = outdentSpaces(view as unknown as EditorView);

      assert.equal(handled, true);
      assert.equal(view.state.doc.toString(), 'item');
      assert.equal(view.state.selection.main.head, 2);
    }
  }
];
