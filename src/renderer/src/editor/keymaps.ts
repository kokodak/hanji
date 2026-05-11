import { EditorView, keymap } from '@codemirror/view';
import { collectFencedCodeBlocks, getFencedCodeBlockForLine } from '../markdown/fencedCode';

const TAB_SPACES = '    ';
const listLinePattern = /^(\s*)(?:[-*+]\s+(?:\[[ xX]\]\s+)?|\d+[.)]\s+)/;
const invisibleCaretTextPattern = /[\u200b\u200c\u200d\ufeff]/g;
const emptyTaskLinePattern = /^(\s*)([-*+])\s+\[([ xX])\][\s\u200b\u200c\u200d\ufeff]*$/;
const emptyBulletLinePattern = /^(\s*)([-*+])[\s\u200b\u200c\u200d\ufeff]*$/;
const emptyNumberedLinePattern = /^(\s*)(\d+)([.)])[\s\u200b\u200c\u200d\ufeff]*$/;
const emptyBlockquoteLinePattern = /^(\s*)>\s+[\u200b\u200c\u200d\ufeff]*$/;
const blockquoteLinePattern = /^(\s*)>\s+/;

function reduceIndent(indent: string): string {
  return indent.slice(0, Math.max(0, indent.length - TAB_SPACES.length));
}

function selectionTouchesFencedCode(view: EditorView): boolean {
  const blocks = collectFencedCodeBlocks(view.state.doc);

  return view.state.selection.ranges.some((range) => {
    const fromLine = view.state.doc.lineAt(Math.min(range.from, range.to)).number;
    const toLine = view.state.doc.lineAt(Math.max(range.from, range.to)).number;

    for (let lineNumber = fromLine; lineNumber <= toLine; lineNumber += 1) {
      if (getFencedCodeBlockForLine(blocks, lineNumber) !== null) return true;
    }

    return false;
  });
}

function blockquoteContinuationPrefix(lineText: string): string | null {
  const match = blockquoteLinePattern.exec(lineText);
  return match ? `${match[1]}> ` : null;
}

export function insertSoftBreak(view: EditorView): boolean {
  const selection = view.state.selection.main;
  const line = view.state.doc.lineAt(selection.head);
  const quotePrefix = blockquoteContinuationPrefix(line.text);
  const insert = selectionTouchesFencedCode(view) ? '\n' : quotePrefix ? `  \n${quotePrefix}` : '  \n';

  view.dispatch({
    changes: { from: selection.from, to: selection.to, insert },
    selection: { anchor: selection.from + insert.length }
  });

  return true;
}

function isBlankListContent(text: string): boolean {
  return text.replace(invisibleCaretTextPattern, '').trim() === '';
}

function replacementForEmptyListLine(text: string): string | null {
  const blockquoteMatch = emptyBlockquoteLinePattern.exec(text);
  if (blockquoteMatch) {
    return blockquoteMatch[1];
  }

  const taskMatch = emptyTaskLinePattern.exec(text);
  if (taskMatch) {
    const [, indent, marker, checkboxState] = taskMatch;
    const nextIndent = reduceIndent(indent);

    return indent.length > 0 ? `${nextIndent}${marker} [${checkboxState}] ` : nextIndent;
  }

  const bulletMatch = emptyBulletLinePattern.exec(text);
  if (bulletMatch) {
    const [, indent, marker] = bulletMatch;
    const nextIndent = reduceIndent(indent);

    return indent.length > 0 ? `${nextIndent}${marker} ` : nextIndent;
  }

  const numberedMatch = emptyNumberedLinePattern.exec(text);
  if (numberedMatch) {
    const [, indent, numberText, marker] = numberedMatch;
    const nextIndent = reduceIndent(indent);

    return indent.length > 0 ? `${nextIndent}${numberText}${marker} ` : nextIndent;
  }

  return null;
}

function lineStartsListItem(text: string): boolean {
  return listLinePattern.test(text);
}

function previousNonEmptyLineStartsListItem(view: EditorView, lineNumber: number): boolean {
  for (let previousLineNumber = lineNumber - 1; previousLineNumber >= 1; previousLineNumber -= 1) {
    const previousLine = view.state.doc.line(previousLineNumber);
    if (previousLine.text.trim() === '') continue;

    return lineStartsListItem(previousLine.text);
  }

  return false;
}

export function continueListItem(view: EditorView): boolean {
  const selection = view.state.selection.main;
  const line = view.state.doc.lineAt(selection.head);
  const emptyListLineReplacement = replacementForEmptyListLine(line.text);

  if (!selection.empty) {
    const selectionStartLine = view.state.doc.lineAt(selection.from);
    const selectionEndLine = view.state.doc.lineAt(selection.to);
    const selectionStaysOnLine = selectionStartLine.number === selectionEndLine.number && selectionStartLine.number === line.number;

    if (!selectionStaysOnLine || emptyListLineReplacement === null) {
      return false;
    }

    view.dispatch({
      changes: { from: line.from, to: line.to, insert: emptyListLineReplacement },
      selection: { anchor: line.from + emptyListLineReplacement.length }
    });
    return true;
  }

  const textBeforeCursor = view.state.sliceDoc(line.from, selection.head);
  const textAfterCursor = view.state.sliceDoc(selection.head, line.to);
  const taskMatch = /^(\s*)([-*+])\s+\[([ xX])\]\s*(.*)$/.exec(textBeforeCursor);
  const bulletMatch = /^(\s*)([-*+])\s+(.*)$/.exec(textBeforeCursor);
  const numberedMatch = /^(\s*)(\d+)([.)])\s+(.*)$/.exec(textBeforeCursor);
  const blockquoteMatch = /^(\s*)>\s+(.*)$/.exec(textBeforeCursor);

  if (emptyListLineReplacement !== null) {
    view.dispatch({
      changes: { from: line.from, to: line.to, insert: emptyListLineReplacement },
      selection: { anchor: line.from + emptyListLineReplacement.length }
    });
    return true;
  }

  if (/^\s+$/.test(line.text)) {
    const indent = line.text;
    const nextIndent = reduceIndent(indent);

    view.dispatch({
      changes: { from: line.from, to: line.to, insert: nextIndent },
      selection: { anchor: line.from + nextIndent.length }
    });
    return true;
  }

  if (taskMatch) {
    const [, indent, marker, , content] = taskMatch;

    if (isBlankListContent(content)) {
      if (!isBlankListContent(textAfterCursor)) return false;

      const nextIndent = reduceIndent(indent);
      view.dispatch({
        changes: { from: line.from, to: selection.head, insert: nextIndent },
        selection: { anchor: line.from + nextIndent.length }
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

    if (isBlankListContent(content)) {
      if (!isBlankListContent(textAfterCursor)) return false;

      const nextIndent = reduceIndent(indent);
      view.dispatch({
        changes: { from: line.from, to: selection.head, insert: nextIndent },
        selection: { anchor: line.from + nextIndent.length }
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

  if (numberedMatch) {
    const [, indent, numberText, marker, content] = numberedMatch;

    if (isBlankListContent(content)) {
      if (!isBlankListContent(textAfterCursor)) return false;

      const nextIndent = reduceIndent(indent);
      view.dispatch({
        changes: { from: line.from, to: selection.head, insert: nextIndent },
        selection: { anchor: line.from + nextIndent.length }
      });
      return true;
    }

    const nextNumber = Number(numberText) + 1;
    const insert = `\n${indent}${nextNumber}${marker} `;
    view.dispatch({
      changes: { from: selection.head, insert },
      selection: { anchor: selection.head + insert.length }
    });
    return true;
  }

  if (blockquoteMatch) {
    const [, indent, content] = blockquoteMatch;

    if (isBlankListContent(content)) {
      if (!isBlankListContent(textAfterCursor)) return false;

      view.dispatch({
        changes: { from: line.from, to: selection.head, insert: indent },
        selection: { anchor: line.from + indent.length }
      });
      return true;
    }

    const insert = `\n${indent}> `;
    view.dispatch({
      changes: { from: selection.head, insert },
      selection: { anchor: selection.head + insert.length }
    });
    return true;
  }

  if (line.text.trim() !== '' && previousNonEmptyLineStartsListItem(view, line.number)) {
    view.dispatch({
      changes: { from: selection.head, insert: '\n' },
      selection: { anchor: selection.head + 1 }
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
    const line = view.state.doc.lineAt(selection.head);

    if (listLinePattern.test(line.text)) {
      view.dispatch({
        changes: { from: line.from, insert: TAB_SPACES },
        selection: { anchor: selection.head + TAB_SPACES.length }
      });
      return true;
    }

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

export const softBreakKeymap = keymap.of([{ key: 'Shift-Enter', run: insertSoftBreak }]);

export const tabIndentation = keymap.of([
  { key: 'Tab', run: indentWithSpaces },
  { key: 'Shift-Tab', run: outdentSpaces }
]);
