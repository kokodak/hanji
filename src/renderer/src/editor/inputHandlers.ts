import { EditorView } from '@codemirror/view';

interface PairRule {
  open: string;
  close: string;
}

const pairRules: PairRule[] = [
  { open: '*', close: '*' },
  { open: '"', close: '"' },
  { open: "'", close: "'" },
  { open: '(', close: ')' },
  { open: '[', close: ']' },
  { open: '{', close: '}' },
  { open: '<', close: '>' }
];

const pairRulesByOpen = new Map(pairRules.map((rule) => [rule.open, rule]));
const pairRulesByClose = new Map(pairRules.map((rule) => [rule.close, rule]));

export function normalizePastedText(text: string): string {
  return text.replace(/\r\n?/g, '\n');
}

const emptyListMarkerLinePattern =
  /^(\s*)(?:[-*+]\s+(?:\[[ xX]\]\s*)?|\d+[.)]\s*)[\u200b\u200c\u200d\ufeff]*$/;
const pastedListItemPattern = /^\s*(?:[-*+]\s+(?:\[[ xX]\]\s+)?|\d+[.)]\s+)/;

export interface PasteReplacement {
  from: number;
  to: number;
  insert: string;
}

export function pastedTextStartsWithListItem(text: string): boolean {
  return pastedListItemPattern.test(text);
}

export function lineIsEmptyListMarker(text: string): boolean {
  return emptyListMarkerLinePattern.test(text);
}

export function getPlainTextPasteReplacement(view: EditorView, from: number, to: number, text: string): PasteReplacement {
  const insert = normalizePastedText(text);
  const selection = view.state.selection.main;

  if (selection.empty && from === to && pastedTextStartsWithListItem(insert)) {
    const line = view.state.doc.lineAt(from);
    const lineText = view.state.sliceDoc(line.from, line.to);

    if (lineIsEmptyListMarker(lineText)) {
      return {
        from: line.from,
        to: line.to,
        insert
      };
    }
  }

  return { from, to, insert };
}

function positionIsInsideFencedCodeBlock(view: EditorView, position: number): boolean {
  const currentLine = view.state.doc.lineAt(position);
  let insideFence = false;

  for (let lineNumber = 1; lineNumber < currentLine.number; lineNumber += 1) {
    const line = view.state.doc.line(lineNumber).text;

    if (/^\s*```/.test(line)) {
      insideFence = !insideFence;
    }
  }

  return insideFence;
}

export const handleBacktickInput = EditorView.inputHandler.of((view, from, to, text) => {
  if (text !== '`') return false;
  if (positionIsInsideFencedCodeBlock(view, from)) return false;

  if (from !== to) {
    const selectedText = view.state.sliceDoc(from, to);

    view.dispatch({
      changes: { from, to, insert: `\`${selectedText}\`` },
      selection: { anchor: to + 2 }
    });

    return true;
  }

  const line = view.state.doc.lineAt(from);
  const textBeforeCursor = view.state.sliceDoc(line.from, from);

  if (from >= 2 && textBeforeCursor.endsWith('``') && textBeforeCursor.slice(0, -2).trim() === '') {
    const insert = '```\n\n```';
    const replaceFrom = from - 2;

    view.dispatch({
      changes: { from: replaceFrom, to, insert },
      selection: { anchor: replaceFrom + 4 }
    });

    return true;
  }

  if (view.state.sliceDoc(from, from + 1) === '`') {
    view.dispatch({ selection: { anchor: from + 1 } });
    return true;
  }

  const previousCharacter = from > line.from ? view.state.sliceDoc(from - 1, from) : '';
  const nextCharacter = from < line.to ? view.state.sliceDoc(from, from + 1) : '';
  const adjacentToExistingText = /\S/.test(previousCharacter) || /\S/.test(nextCharacter);

  if (adjacentToExistingText) {
    view.dispatch({
      changes: { from, to, insert: '`' },
      selection: { anchor: from + 1 }
    });
    return true;
  }

  view.dispatch({
    changes: { from, to, insert: '``' },
    selection: { anchor: from + 1 }
  });

  return true;
});

export const handlePairedSymbolInput = EditorView.inputHandler.of((view, from, to, text) => {
  if (positionIsInsideFencedCodeBlock(view, from)) return false;

  const openRule = pairRulesByOpen.get(text);
  const closeRule = pairRulesByClose.get(text);

  if (!openRule && !closeRule) return false;

  if (from !== to) {
    const rule = openRule ?? closeRule;
    if (!rule) return false;

    const selectedText = view.state.sliceDoc(from, to);

    view.dispatch({
      changes: { from, to, insert: `${rule.open}${selectedText}${rule.close}` },
      selection: { anchor: to + 2 }
    });

    return true;
  }

  if (closeRule && view.state.sliceDoc(from, from + text.length) === text) {
    view.dispatch({ selection: { anchor: from + 1 } });
    return true;
  }

  if (!openRule) return false;

  const line = view.state.doc.lineAt(from);
  const previousCharacter = from > line.from ? view.state.sliceDoc(from - 1, from) : '';
  const nextCharacter = from < line.to ? view.state.sliceDoc(from, from + 1) : '';
  const adjacentToExistingText = /\S/.test(previousCharacter) || /\S/.test(nextCharacter);

  if (adjacentToExistingText) {
    view.dispatch({
      changes: { from, to, insert: openRule.open },
      selection: { anchor: from + 1 }
    });
    return true;
  }

  view.dispatch({
    changes: { from, to, insert: `${openRule.open}${openRule.close}` },
    selection: { anchor: from + 1 }
  });

  return true;
});

export const handlePlainTextPaste = EditorView.domEventHandlers({
  paste(event, view) {
    const text = event.clipboardData?.getData('text/plain');
    if (!text) return false;

    event.preventDefault();
    const selection = view.state.selection.main;
    const replacement = getPlainTextPasteReplacement(view, selection.from, selection.to, text);

    view.dispatch({
      changes: replacement,
      selection: { anchor: replacement.from + replacement.insert.length },
      userEvent: 'input.paste'
    });
    return true;
  }
});
