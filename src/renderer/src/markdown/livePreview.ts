import { RangeSetBuilder, type Text } from '@codemirror/state';
import { Decoration, type DecorationSet, EditorView, ViewPlugin, type ViewUpdate, WidgetType } from '@codemirror/view';
import { addInlinePreviewDecorations, lineIsOnlyInlineCode } from './inlinePreview';
import {
  collectFencedCodeBlocks,
  getFencedCodeBlockForLine,
  getFencedCodeLineDecoration,
  getPreviewCodeLineDecoration,
  isActiveFencedCodeBlock
} from './fencedCode';
import {
  compactSelection,
  headingClasses,
  hiddenHeadingSyntax,
  hiddenSyntax,
  hiddenTableSourceLine,
  liveCheckedTask,
  selectedHiddenTableSourceLine,
  selectedTablePreviewLine,
  tablePreviewLine
} from './decorations';
import { BulletWidget, CheckboxWidget, CodeLanguageWidget, HorizontalRuleWidget, NumberedListWidget, TableWidget } from './widgets';
import { lineContainsCursor, lineContainsSelection, lineIntersectsSelection, rangeContainsSelection } from './selection';
import { collectMarkdownTables, getMarkdownTableForLine, type MarkdownTable } from './table';
import type { PendingDecoration } from './types';

export { collectMarkdownTables } from './table';
export type { MarkdownTable } from './table';

export function safePosAtCoords(view: Pick<EditorView, 'posAtCoords'>, coords: { x: number; y: number }): number | null {
  try {
    return view.posAtCoords(coords);
  } catch {
    return null;
  }
}

class EmptyLineSelectionWidget extends WidgetType {
  toDOM(): HTMLElement {
    const selection = document.createElement('span');
    selection.className = 'cm-compact-empty-selection';
    return selection;
  }
}

function moveSingleInlineCodeClickToLineEnd(view: EditorView, event: MouseEvent): boolean {
  const position = safePosAtCoords(view, { x: event.clientX, y: event.clientY });
  if (position === null) return false;

  const line = view.state.doc.lineAt(position);
  if (!lineIsOnlyInlineCode(line.text)) return false;

  const lineEndCoords = view.coordsAtPos(line.to);
  if (!lineEndCoords || event.clientX < lineEndCoords.left) return false;

  event.preventDefault();
  view.dispatch({
    selection: { anchor: line.to },
    scrollIntoView: true
  });
  view.focus();

  return true;
}

function tableIntersectsSelection(view: EditorView, table: MarkdownTable): boolean {
  const startLine = view.state.doc.line(table.startLine);
  const endLine = view.state.doc.line(table.endLine);

  return view.state.selection.ranges.some((range) => {
    const selectionFrom = Math.min(range.from, range.to);
    const selectionTo = Math.max(range.from, range.to);

    return selectionFrom < endLine.to && selectionTo > startLine.from;
  });
}

function addCompactSelectionDecorations(view: EditorView, pending: PendingDecoration[], from: number, to: number): void {
  for (const range of view.state.selection.ranges) {
    if (range.empty) continue;

    const selectionFrom = Math.min(range.from, range.to);
    const selectionTo = Math.max(range.from, range.to);

    if (from === to) {
      if (selectionFrom <= from && selectionTo >= from) {
        pending.push({
          from,
          to: from,
          decoration: Decoration.widget({ widget: new EmptyLineSelectionWidget(), side: 1 })
        });
      }
      continue;
    }

    const selectedFrom = Math.max(selectionFrom, from);
    const selectedTo = Math.min(selectionTo, to);

    if (selectedFrom < selectedTo) {
      pending.push({ from: selectedFrom, to: selectedTo, decoration: compactSelection });
    }
  }
}

function listWrapLine(indentLength: number): Decoration {
  return Decoration.line({
    attributes: {
      style: `--list-wrap-indent: ${indentLength}ch;`
    },
    class: 'cm-live-list-line'
  });
}

export function buildLivePreviewDecorations(view: EditorView, hoverLine: number | null): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const pending: PendingDecoration[] = [];
  const codeBlocks = collectFencedCodeBlocks(view.state.doc);
  const frontmatterBlock = collectYamlFrontmatterBlock(view.state.doc);
  const tables = collectMarkdownTables(view.state.doc);

  for (const range of view.visibleRanges) {
    for (let pos = range.from; pos <= range.to;) {
      const line = view.state.doc.lineAt(pos);
      const lineText = line.text;
      const inFrontmatter =
        frontmatterBlock !== null && line.number >= frontmatterBlock.startLine && line.number <= frontmatterBlock.endLine;
      const isInteractive = line.number === hoverLine || lineContainsCursor(view, line.from, line.to) || lineIntersectsSelection(view, line.from, line.to);
      const codeBlock = getFencedCodeBlockForLine(codeBlocks, line.number);
      const table = getMarkdownTableForLine(tables, line.number);
      const headingMatch = /^(#{1,6})\s+/.exec(lineText);
      const taskMatch = /^(\s*)([-*+])\s+\[([ xX])\]\s+/.exec(lineText);
      const listMatch = /^(\s*)([-*+])\s+/.exec(lineText);
      const numberedListMatch = /^(\s*)(\d+)([.)])\s+/.exec(lineText);
      const blockquoteMatch = /^(\s*)>\s?/.exec(lineText);
      const horizontalRule = lineIsHorizontalRule(lineText);

      if (codeBlock) {
        addCompactSelectionDecorations(view, pending, line.from, line.to);
        const activeCodeBlock = isActiveFencedCodeBlock(view, codeBlock);
        const fenceLine = line.number === codeBlock.startLine || line.number === codeBlock.endLine;

        if (activeCodeBlock || !fenceLine) {
          const lineDecoration = activeCodeBlock
            ? getFencedCodeLineDecoration(codeBlock, line.number)
            : getPreviewCodeLineDecoration(codeBlock, line.number);

          pending.push({ from: line.from, to: line.from, decoration: lineDecoration });
        }

        if (!activeCodeBlock && fenceLine) {
          pending.push({ from: line.from, to: line.to, decoration: hiddenSyntax });
        }

        if (line.number === (activeCodeBlock ? codeBlock.startLine : codeBlock.startLine + 1)) {
          pending.push({
            from: line.from,
            to: line.from,
            decoration: Decoration.widget({
              widget: new CodeLanguageWidget(codeBlock.language),
              side: 1
            })
          });
        }

        pos = line.to + 1;
        continue;
      }

      if (inFrontmatter) {
        addCompactSelectionDecorations(view, pending, line.from, line.to);
        pending.push({
          from: line.from,
          to: line.from,
          decoration: Decoration.line({ class: 'cm-live-frontmatter' })
        });

        pos = line.to + 1;
        continue;
      }

      if (table) {
        const selectedTable = tableIntersectsSelection(view, table);

        if (line.number === table.startLine) {
          pending.push({ from: line.from, to: line.from, decoration: selectedTable ? selectedTablePreviewLine : tablePreviewLine });
          pending.push({
            from: line.from,
            to: line.to,
            decoration: Decoration.replace({ widget: new TableWidget(table, selectedTable) })
          });
        } else {
          pending.push({ from: line.from, to: line.from, decoration: selectedTable ? selectedHiddenTableSourceLine : hiddenTableSourceLine });
          pending.push({ from: line.from, to: line.to, decoration: hiddenSyntax });
        }

        pos = line.to + 1;
        continue;
      }

      addCompactSelectionDecorations(view, pending, line.from, line.to);

      if (headingMatch) {
        const headingLevel = Math.min(headingMatch[1].length, headingClasses.length);
        pending.push({
          from: line.from,
          to: line.from,
          decoration: Decoration.line({ class: headingClasses[headingLevel - 1] })
        });

        if (!isInteractive) {
          pending.push({ from: line.from, to: line.from + headingMatch[0].length, decoration: hiddenHeadingSyntax });
        }
      }

      if (horizontalRule) {
        if (!isInteractive) {
          pending.push({
            from: line.from,
            to: line.to,
            decoration: Decoration.replace({ widget: new HorizontalRuleWidget() })
          });
        }

        pos = line.to + 1;
        continue;
      }

      if (taskMatch) {
        const indentLength = taskMatch[1].length;
        pending.push({ from: line.from, to: line.from, decoration: listWrapLine(indentLength) });
        const markerStart = line.from + indentLength;
        const taskEnd = line.from + taskMatch[0].length;
        const checkPosition = markerStart + taskMatch[2].length + 2;
        const isChecked = taskMatch[3].toLowerCase() === 'x';
        const editingTaskMarker = rangeContainsSelection(view, markerStart, taskEnd);

        if (!editingTaskMarker) {
          pending.push({
            from: markerStart,
            to: taskEnd,
            decoration: Decoration.replace({ widget: new CheckboxWidget(isChecked, checkPosition) })
          });
        }

        if (isChecked) {
          pending.push({ from: taskEnd, to: line.to, decoration: liveCheckedTask });
        }
      } else if (listMatch) {
        const indentLength = listMatch[1].length;
        pending.push({ from: line.from, to: line.from, decoration: listWrapLine(indentLength) });
        const markerStart = line.from + indentLength;
        const markerEnd = line.from + listMatch[0].length;

        if (!rangeContainsSelection(view, markerStart, markerEnd)) {
          pending.push({ from: markerStart, to: markerEnd, decoration: Decoration.replace({ widget: new BulletWidget() }) });
        }
      } else if (numberedListMatch) {
        const indentLength = numberedListMatch[1].length;
        pending.push({ from: line.from, to: line.from, decoration: listWrapLine(indentLength) });
        const markerStart = line.from + indentLength;
        const markerEnd = line.from + numberedListMatch[0].length;
        const markerText = `${numberedListMatch[2]}${numberedListMatch[3]}`;

        if (!rangeContainsSelection(view, markerStart, markerEnd)) {
          pending.push({
            from: markerStart,
            to: markerEnd,
            decoration: Decoration.replace({ widget: new NumberedListWidget(markerText) })
          });
        }
      }

      if (blockquoteMatch) {
        const markerStart = line.from + blockquoteMatch[1].length;
        const markerEnd = line.from + blockquoteMatch[0].length;

        pending.push({
          from: line.from,
          to: line.from,
          decoration: Decoration.line({ class: 'cm-live-blockquote' })
        });

        if (!lineContainsSelection(view, line.from, line.to)) {
          pending.push({ from: markerStart, to: markerEnd, decoration: hiddenSyntax });
        }
      }

      addInlinePreviewDecorations(view, pending, line.from, lineText);

      pos = line.to + 1;
    }
  }

  pending
    .sort((a, b) => a.from - b.from || a.to - b.to)
    .forEach((item) => {
      builder.add(item.from, item.to, item.decoration);
    });

  return builder.finish();
}

export interface TableCursorTarget {
  anchor: number;
  insertBreakAt: number | null;
}

export function getTableCursorTarget(doc: Text, table: MarkdownTable, position: number): TableCursorTarget | null {
  const startLine = doc.line(table.startLine);
  const endLine = doc.line(table.endLine);
  if (position < startLine.from || position > endLine.to) return null;

  if (table.endLine < doc.lines) {
    return { anchor: doc.line(table.endLine + 1).from, insertBreakAt: null };
  }

  return { anchor: endLine.to + 1, insertBreakAt: endLine.to };
}

function moveCursorOutsideRenderedTables(view: EditorView): void {
  const selection = view.state.selection.main;
  if (!selection.empty) return;

  const tables = collectMarkdownTables(view.state.doc);
  const table = tables.find((item) => getTableCursorTarget(view.state.doc, item, selection.head) !== null);
  if (!table) return;

  const target = getTableCursorTarget(view.state.doc, table, selection.head);
  if (!target) return;

  view.dispatch({
    changes: target.insertBreakAt === null ? undefined : { from: target.insertBreakAt, insert: '\n' },
    selection: { anchor: target.anchor },
    scrollIntoView: true
  });
}

function scheduleCursorMoveOutsideRenderedTables(view: EditorView): void {
  window.setTimeout(() => {
    moveCursorOutsideRenderedTables(view);
  }, 0);
}

export function nextHoverLineAfterEditorUpdate(
  hoverLine: number | null,
  update: Pick<ViewUpdate, 'docChanged' | 'selectionSet' | 'viewportChanged'>
): number | null {
  if (update.docChanged || update.selectionSet) {
    return null;
  }

  return hoverLine;
}

export function lineIsHorizontalRule(lineText: string): boolean {
  return /^\s{0,3}([-*_])(?:\s*\1){2,}\s*$/.test(lineText);
}

export interface FrontmatterBlock {
  startLine: number;
  endLine: number;
}

export function collectYamlFrontmatterBlock(doc: Text): FrontmatterBlock | null {
  if (doc.lines < 2 || !/^---\s*$/.test(doc.line(1).text)) {
    return null;
  }

  for (let lineNumber = 2; lineNumber <= doc.lines; lineNumber += 1) {
    if (/^---\s*$/.test(doc.line(lineNumber).text)) {
      return { startLine: 1, endLine: lineNumber };
    }
  }

  return { startLine: 1, endLine: doc.lines };
}

export const liveMarkdownPreview = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    hoverLine: number | null = null;

    constructor(view: EditorView) {
      this.decorations = buildLivePreviewDecorations(view, this.hoverLine);
    }

    update(update: ViewUpdate): void {
      if (update.docChanged || update.selectionSet || update.viewportChanged) {
        this.hoverLine = nextHoverLineAfterEditorUpdate(this.hoverLine, update);
        this.decorations = buildLivePreviewDecorations(update.view, this.hoverLine);
        update.view.requestMeasure();
        if (update.docChanged || update.selectionSet) {
          scheduleCursorMoveOutsideRenderedTables(update.view);
        }
      }
    }

    setHoverLine(view: EditorView, lineNumber: number | null): void {
      if (this.hoverLine === lineNumber) return;
      this.hoverLine = lineNumber;
      this.decorations = buildLivePreviewDecorations(view, this.hoverLine);
      view.requestMeasure();
    }
  },
  {
    decorations: (plugin) => plugin.decorations,
    eventHandlers: {
      mousedown(event, view) {
        return moveSingleInlineCodeClickToLineEnd(view, event);
      },
      mousemove(event, view) {
        if (event.buttons !== 0) return;

        const plugin = view.plugin(liveMarkdownPreview);
        if (!plugin) return;

        const position = safePosAtCoords(view, { x: event.clientX, y: event.clientY });
        plugin.setHoverLine(view, position === null ? null : view.state.doc.lineAt(position).number);
      },
      mouseleave(_, view) {
        view.plugin(liveMarkdownPreview)?.setHoverLine(view, null);
      }
    }
  }
);
