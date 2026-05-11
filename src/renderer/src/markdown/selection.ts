import type { EditorView } from '@codemirror/view';

export function lineIntersectsSelection(view: EditorView, lineFrom: number, lineTo: number): boolean {
  return view.state.selection.ranges.some((range) => {
    const selectionFrom = Math.min(range.from, range.to);
    const selectionTo = Math.max(range.from, range.to);

    if (range.empty) {
      return range.head >= lineFrom && range.head <= lineTo;
    }

    return selectionFrom <= lineTo && selectionTo >= lineFrom;
  });
}

export function rangeContainsSelection(view: EditorView, from: number, to: number): boolean {
  return view.state.selection.ranges.some((range) => {
    const selectionFrom = Math.min(range.from, range.to);
    const selectionTo = Math.max(range.from, range.to);

    if (range.empty) {
      return range.head >= from && range.head < to;
    }

    return selectionFrom < to && selectionTo > from;
  });
}

export function lineContainsCursor(view: EditorView, lineFrom: number, lineTo: number): boolean {
  return view.state.selection.ranges.some((range) => range.empty && range.head >= lineFrom && range.head <= lineTo);
}

export function rangeContainsCursor(view: EditorView, from: number, to: number): boolean {
  return view.state.selection.ranges.some((range) => range.empty && range.head >= from && range.head < to);
}

export function syntaxRangeIsActive(view: EditorView, from: number, to: number): boolean {
  return rangeContainsSelection(view, from, to);
}

export function inlineCodeRangeIsActive(view: EditorView, from: number, to: number): boolean {
  return view.state.selection.ranges.some((range) => {
    const selectionFrom = Math.min(range.from, range.to);
    const selectionTo = Math.max(range.from, range.to);

    if (range.empty) {
      return range.head >= from && range.head <= to;
    }

    return selectionFrom < to && selectionTo > from;
  });
}

export function lineContainsSelection(view: EditorView, lineFrom: number, lineTo: number): boolean {
  return view.state.selection.ranges.some((range) => {
    if (range.empty) {
      return range.head >= lineFrom && range.head <= lineTo;
    }

    const selectionFrom = Math.min(range.from, range.to);
    const selectionTo = Math.max(range.from, range.to);

    return selectionFrom <= lineTo && selectionTo >= lineFrom;
  });
}
