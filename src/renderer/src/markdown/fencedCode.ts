import type { Text } from '@codemirror/state';
import { Decoration, EditorView } from '@codemirror/view';
import {
  fencedCodeFirstLine,
  fencedCodeLastLine,
  fencedCodeLine,
  fencedCodeSingleLine
} from './decorations';
import type { FencedCodeBlock } from './types';

function getFenceInfo(lineText: string): { marker: string; language: string } | null {
  const match = /^\s*(```+|~~~+)\s*([A-Za-z0-9_+.-]*)?/.exec(lineText);
  if (!match) return null;

  return {
    marker: match[1],
    language: match[2] || 'plain text'
  };
}

export function collectFencedCodeBlocks(doc: Text): FencedCodeBlock[] {
  const blocks: FencedCodeBlock[] = [];
  let openBlock: FencedCodeBlock | null = null;

  for (let lineNumber = 1; lineNumber <= doc.lines; lineNumber += 1) {
    const line = doc.line(lineNumber);
    const fence = getFenceInfo(line.text);

    if (fence) {
      if (!openBlock) {
        openBlock = {
          startLine: line.number,
          endLine: doc.lines,
          marker: fence.marker,
          language: fence.language
        };
      } else if (fence.marker[0] === openBlock.marker[0] && fence.marker.length >= openBlock.marker.length) {
        blocks.push({ ...openBlock, endLine: line.number });
        openBlock = null;
      }
    }
  }

  return blocks;
}

export function getFencedCodeBlockForLine(blocks: FencedCodeBlock[], lineNumber: number): FencedCodeBlock | null {
  return blocks.find((block) => lineNumber >= block.startLine && lineNumber <= block.endLine) ?? null;
}

export function isActiveFencedCodeBlock(view: EditorView, block: FencedCodeBlock): boolean {
  return view.state.selection.ranges.some((range) => {
    const fromLine = view.state.doc.lineAt(range.from).number;
    const toLine = view.state.doc.lineAt(range.to).number;

    return fromLine <= block.endLine && toLine >= block.startLine;
  });
}

export function getFencedCodeLineDecoration(block: FencedCodeBlock, lineNumber: number): Decoration {
  if (block.startLine === block.endLine) return fencedCodeSingleLine;
  if (lineNumber === block.startLine) return fencedCodeFirstLine;
  if (lineNumber === block.endLine) return fencedCodeLastLine;
  return fencedCodeLine;
}

export function getPreviewCodeLineDecoration(block: FencedCodeBlock, lineNumber: number): Decoration {
  const firstCodeLine = block.startLine + 1;
  const lastCodeLine = block.endLine - 1;

  if (lineNumber === firstCodeLine && lineNumber === lastCodeLine) return fencedCodeSingleLine;
  if (lineNumber === firstCodeLine) return fencedCodeFirstLine;
  if (lineNumber === lastCodeLine) return fencedCodeLastLine;
  return fencedCodeLine;
}
