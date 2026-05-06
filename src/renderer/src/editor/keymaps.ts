import { EditorView, keymap } from '@codemirror/view';

const TAB_SPACES = '    ';

export function continueListItem(view: EditorView): boolean {
  const selection = view.state.selection.main;
  if (!selection.empty) return false;

  const line = view.state.doc.lineAt(selection.head);
  const textBeforeCursor = view.state.sliceDoc(line.from, selection.head);
  const taskMatch = /^(\s*)([-*+])\s+\[([ xX])\]\s*(.*)$/.exec(textBeforeCursor);
  const bulletMatch = /^(\s*)([-*+])\s+(.*)$/.exec(textBeforeCursor);

  if (taskMatch) {
    const [, indent, marker, , content] = taskMatch;

    if (content.trim() === '') {
      view.dispatch({
        changes: { from: line.from, to: selection.head, insert: indent },
        selection: { anchor: line.from + indent.length }
      });
      return true;
    }

    const insert = `\n${indent}${marker} [ ] `;
    view.dispatch({
      changes: { from: selection.head, insert },
      selection: { anchor: selection.head + insert.length }
    });
    return true;
  }

  if (bulletMatch) {
    const [, indent, marker, content] = bulletMatch;

    if (content.trim() === '') {
      view.dispatch({
        changes: { from: line.from, to: selection.head, insert: indent },
        selection: { anchor: line.from + indent.length }
      });
      return true;
    }

    const insert = `\n${indent}${marker} `;
    view.dispatch({
      changes: { from: selection.head, insert },
      selection: { anchor: selection.head + insert.length }
    });
    return true;
  }

  return false;
}

function selectedLineNumbers(view: EditorView): number[] {
  const lines = new Set<number>();

  for (const range of view.state.selection.ranges) {
    const from = Math.min(range.from, range.to);
    const to = Math.max(range.from, range.to);
    const end = to > from && view.state.doc.lineAt(to).from === to ? to - 1 : to;
    const fromLine = view.state.doc.lineAt(from);
    const toLine = view.state.doc.lineAt(Math.max(from, end));

    for (let lineNumber = fromLine.number; lineNumber <= toLine.number; lineNumber += 1) {
      lines.add(lineNumber);
    }
  }

  return [...lines].sort((first, second) => first - second);
}

export function indentWithSpaces(view: EditorView): boolean {
  const selection = view.state.selection.main;

  if (selection.empty) {
    view.dispatch({
      changes: { from: selection.head, insert: TAB_SPACES },
      selection: { anchor: selection.head + TAB_SPACES.length }
    });
    return true;
  }

  const changes = selectedLineNumbers(view).map((lineNumber) => ({
    from: view.state.doc.line(lineNumber).from,
    insert: TAB_SPACES
  }));

  view.dispatch({ changes });
  return true;
}

export function outdentSpaces(view: EditorView): boolean {
  const selection = view.state.selection.main;
  const lineNumbers = selection.empty ? [view.state.doc.lineAt(selection.head).number] : selectedLineNumbers(view);
  const changes = lineNumbers
    .map((lineNumber) => {
      const line = view.state.doc.line(lineNumber);
      const removableSpaces = /^\s{1,4}/.exec(line.text)?.[0].length ?? 0;

      return removableSpaces > 0
        ? {
            from: line.from,
            to: line.from + removableSpaces,
            insert: ''
          }
        : null;
    })
    .filter((change): change is { from: number; to: number; insert: string } => change !== null);

  if (changes.length === 0) return true;

  view.dispatch({ changes });
  return true;
}

function moveCursorByDocumentLine(view: EditorView, direction: -1 | 1): boolean {
  const selection = view.state.selection.main;
  if (!selection.empty) return false;

  const line = view.state.doc.lineAt(selection.head);
  const targetLineNumber = line.number + direction;

  if (targetLineNumber < 1 || targetLineNumber > view.state.doc.lines) {
    return false;
  }

  const column = selection.head - line.from;
  const targetLine = view.state.doc.line(targetLineNumber);
  const anchor = targetLine.from + Math.min(column, targetLine.length);

  view.dispatch({
    selection: { anchor },
    scrollIntoView: true
  });

  return true;
}

export const stableVerticalMovement = keymap.of([
  { key: 'ArrowUp', run: (view) => moveCursorByDocumentLine(view, -1) },
  { key: 'ArrowDown', run: (view) => moveCursorByDocumentLine(view, 1) }
]);

export const tabIndentation = keymap.of([
  { key: 'Tab', run: indentWithSpaces },
  { key: 'Shift-Tab', run: outdentSpaces }
]);
