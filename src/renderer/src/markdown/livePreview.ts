import { RangeSetBuilder, type Text } from '@codemirror/state';
import { Decoration, type DecorationSet, EditorView, ViewPlugin, type ViewUpdate } from '@codemirror/view';
import { addInlinePreviewDecorations, lineIsOnlyInlineCode } from './inlinePreview';
import {
  collectFencedCodeBlocks,
  getFencedCodeBlockForLine,
  getFencedCodeLineDecoration,
  getPreviewCodeLineDecoration,
  isActiveFencedCodeBlock
} from './fencedCode';
import { bulletMarker, headingClasses, hiddenSyntax, liveCheckedTask } from './decorations';
import { CheckboxWidget, CodeLanguageWidget, HorizontalRuleWidget, NumberedListWidget, TableWidget } from './widgets';
import { lineContainsSelection, lineIntersectsSelection, rangeContainsSelection } from './selection';
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

function buildLivePreviewDecorations(view: EditorView, hoverLine: number | null): DecorationSet {
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
      const isInteractive = line.number === hoverLine || lineIntersectsSelection(view, line.from, line.to);
      const codeBlock = getFencedCodeBlockForLine(codeBlocks, line.number);
      const table = getMarkdownTableForLine(tables, line.number);
      const headingMatch = /^(#{1,6})\s+/.exec(lineText);
      const taskMatch = /^(\s*)([-*+])\s+\[([ xX])\]\s+/.exec(lineText);
      const listMatch = /^(\s*)([-*+])\s+/.exec(lineText);
      const numberedListMatch = /^(\s*)(\d+)([.)])\s+/.exec(lineText);
      const blockquoteMatch = /^(\s*)>\s?/.exec(lineText);
      const horizontalRule = lineIsHorizontalRule(lineText);

      if (codeBlock) {
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
        pending.push({
          from: line.from,
          to: line.from,
          decoration: Decoration.line({ class: 'cm-live-frontmatter' })
        });

        pos = line.to + 1;
        continue;
      }

      if (table) {
        if (line.number === table.startLine) {
          pending.push({
            from: line.from,
            to: line.to,
            decoration: Decoration.replace({ widget: new TableWidget(table) })
          });
        } else {
          pending.push({ from: line.from, to: line.to, decoration: hiddenSyntax });
        }

        pos = line.to + 1;
        continue;
      }

      if (headingMatch) {
        const headingLevel = Math.min(headingMatch[1].length, headingClasses.length);
        pending.push({
          from: line.from,
          to: line.from,
          decoration: Decoration.line({ class: headingClasses[headingLevel - 1] })
        });

        if (!isInteractive) {
          pending.push({ from: line.from, to: line.from + headingMatch[0].length, decoration: hiddenSyntax });
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
        const markerStart = line.from + taskMatch[1].length;
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
        const markerStart = line.from + listMatch[1].length;
        const markerEnd = line.from + listMatch[0].length;

        if (!rangeContainsSelection(view, markerStart, markerEnd)) {
          pending.push({ from: markerStart, to: markerEnd, decoration: bulletMarker });
        }
      } else if (numberedListMatch) {
        const markerStart = line.from + numberedListMatch[1].length;
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
