import { Decoration, type EditorView } from '@codemirror/view';
import { hiddenSyntax, liveCode, liveEmphasis, liveStrikethrough, liveStrong } from './decorations';
import { inlineCodeRangeIsActive, syntaxRangeIsActive } from './selection';
import type { PendingDecoration } from './types';
import { ImageWidget, LinkWidget } from './widgets';

interface InlineSyntaxRange {
  from: number;
  to: number;
  markers: InlineSyntaxMarker[];
}

interface InlineSyntaxMarker {
  from: number;
  to: number;
}

function rangeTouchesSelection(view: EditorView, from: number, to: number): boolean {
  return view.state.selection.ranges.some((range) => {
    const selectionFrom = Math.min(range.from, range.to);
    const selectionTo = Math.max(range.from, range.to);

    if (range.empty) {
      return range.head >= from && range.head <= to;
    }

    return selectionFrom < to && selectionTo > from;
  });
}

function rangesConnect(first: InlineSyntaxRange, second: InlineSyntaxRange): boolean {
  return first.from <= second.to && second.from <= first.to;
}

function getActiveInlineRanges(view: EditorView, ranges: InlineSyntaxRange[]): InlineSyntaxRange[] {
  const activeRanges = ranges.filter((range) => rangeTouchesSelection(view, range.from, range.to));
  let changed = true;

  while (changed) {
    changed = false;

    for (const range of ranges) {
      if (activeRanges.includes(range)) continue;

      if (activeRanges.some((activeRange) => rangesConnect(activeRange, range))) {
        activeRanges.push(range);
        changed = true;
      }
    }
  }

  return activeRanges;
}

function rangeIsActive(range: InlineSyntaxRange, activeRanges: InlineSyntaxRange[]): boolean {
  return activeRanges.includes(range);
}

function collectInlineCodeRanges(lineFrom: number, lineText: string): InlineSyntaxRange[] {
  const ranges: InlineSyntaxRange[] = [];

  for (const match of lineText.matchAll(/`([^`]+?)`/g)) {
    const matchStart = match.index ?? 0;
    ranges.push({
      from: lineFrom + matchStart,
      to: lineFrom + matchStart + match[0].length,
      markers: [
        { from: lineFrom + matchStart, to: lineFrom + matchStart + 1 },
        { from: lineFrom + matchStart + match[0].length - 1, to: lineFrom + matchStart + match[0].length }
      ]
    });
  }

  return ranges;
}

function rangeOverlapsInlineCode(from: number, to: number, inlineCodeRanges: InlineSyntaxRange[]): boolean {
  return inlineCodeRanges.some((range) => from < range.to && to > range.from);
}

function matchOverlapsInlineCode(
  lineFrom: number,
  matchStart: number,
  matchLength: number,
  inlineCodeRanges: InlineSyntaxRange[]
): boolean {
  return rangeOverlapsInlineCode(lineFrom + matchStart, lineFrom + matchStart + matchLength, inlineCodeRanges);
}

function addStyledContentDecoration(
  pending: PendingDecoration[],
  from: number,
  to: number,
  decoration: Decoration,
  syntaxRanges: InlineSyntaxRange[],
  activeRanges: InlineSyntaxRange[]
): void {
  const hiddenMarkers = syntaxRanges
    .filter((range) => !rangeIsActive(range, activeRanges))
    .flatMap((range) => range.markers)
    .filter((marker) => marker.from < to && marker.to > from)
    .sort((first, second) => first.from - second.from || first.to - second.to);

  let segmentFrom = from;

  for (const marker of hiddenMarkers) {
    const markerFrom = Math.max(marker.from, from);
    const markerTo = Math.min(marker.to, to);

    if (segmentFrom < markerFrom) {
      pending.push({ from: segmentFrom, to: markerFrom, decoration });
    }

    segmentFrom = Math.max(segmentFrom, markerTo);
  }

  if (segmentFrom < to) {
    pending.push({ from: segmentFrom, to, decoration });
  }
}

export function addInlinePreviewDecorations(
  view: EditorView,
  pending: PendingDecoration[],
  lineFrom: number,
  lineText: string
): void {
  const syntaxRanges: InlineSyntaxRange[] = [];
  const inlineCodeRanges = collectInlineCodeRanges(lineFrom, lineText);
  syntaxRanges.push(...inlineCodeRanges);

  for (const match of lineText.matchAll(/(\*\*|__)(.+?)\1/g)) {
    const matchStart = match.index ?? 0;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length, inlineCodeRanges)) continue;

    const contentStart = matchStart + match[1].length;
    const contentEnd = matchStart + match[0].length - match[1].length;
    syntaxRanges.push({
      from: lineFrom + matchStart,
      to: lineFrom + matchStart + match[0].length,
      markers: [
        { from: lineFrom + matchStart, to: lineFrom + contentStart },
        { from: lineFrom + contentEnd, to: lineFrom + matchStart + match[0].length }
      ]
    });
  }

  for (const match of lineText.matchAll(/!\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g)) {
    const matchStart = match.index ?? 0;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length, inlineCodeRanges)) continue;

    syntaxRanges.push({
      from: lineFrom + matchStart,
      to: lineFrom + matchStart + match[0].length,
      markers: [{ from: lineFrom + matchStart, to: lineFrom + matchStart + match[0].length }]
    });
  }

  for (const match of lineText.matchAll(/~~(.+?)~~/g)) {
    const matchStart = match.index ?? 0;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length, inlineCodeRanges)) continue;

    syntaxRanges.push({
      from: lineFrom + matchStart,
      to: lineFrom + matchStart + match[0].length,
      markers: [
        { from: lineFrom + matchStart, to: lineFrom + matchStart + 2 },
        { from: lineFrom + matchStart + match[0].length - 2, to: lineFrom + matchStart + match[0].length }
      ]
    });
  }

  for (const match of lineText.matchAll(/(^|[^\*])\*([^\s*][^*]*?[^\s*]|\S)\*(?!\*)/g)) {
    const prefix = match[1];
    const matchStart = (match.index ?? 0) + prefix.length;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length - prefix.length, inlineCodeRanges)) continue;

    syntaxRanges.push({
      from: lineFrom + matchStart,
      to: lineFrom + matchStart + match[0].length - prefix.length,
      markers: [
        { from: lineFrom + matchStart, to: lineFrom + matchStart + 1 },
        {
          from: lineFrom + matchStart + match[0].length - prefix.length - 1,
          to: lineFrom + matchStart + match[0].length - prefix.length
        }
      ]
    });
  }

  for (const match of lineText.matchAll(/(^|[^_])_([^\s_][^_]*?[^\s_]|\S)_(?!_)/g)) {
    const prefix = match[1];
    const matchStart = (match.index ?? 0) + prefix.length;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length - prefix.length, inlineCodeRanges)) continue;

    syntaxRanges.push({
      from: lineFrom + matchStart,
      to: lineFrom + matchStart + match[0].length - prefix.length,
      markers: [
        { from: lineFrom + matchStart, to: lineFrom + matchStart + 1 },
        {
          from: lineFrom + matchStart + match[0].length - prefix.length - 1,
          to: lineFrom + matchStart + match[0].length - prefix.length
        }
      ]
    });
  }

  for (const match of lineText.matchAll(/(^|[^!])\[([^\]]+)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g)) {
    const prefix = match[1];
    const matchStart = (match.index ?? 0) + prefix.length;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length - prefix.length, inlineCodeRanges)) continue;

    syntaxRanges.push({
      from: lineFrom + matchStart,
      to: lineFrom + matchStart + match[0].length - prefix.length,
      markers: [{ from: lineFrom + matchStart, to: lineFrom + matchStart + match[0].length - prefix.length }]
    });
  }

  const activeRanges = getActiveInlineRanges(view, syntaxRanges);

  for (const match of lineText.matchAll(/(\*\*|__)(.+?)\1/g)) {
    const marker = match[1];
    const matchStart = match.index ?? 0;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length, inlineCodeRanges)) continue;

    const contentStart = matchStart + marker.length;
    const contentEnd = matchStart + match[0].length - marker.length;
    const openingFrom = lineFrom + matchStart;
    const openingTo = lineFrom + contentStart;
    const closingFrom = lineFrom + contentEnd;
    const closingTo = lineFrom + matchStart + match[0].length;
    const strongRange = syntaxRanges.find((range) => range.from === openingFrom && range.to === closingTo);
    const activeStrong = strongRange ? rangeIsActive(strongRange, activeRanges) : syntaxRangeIsActive(view, openingFrom, closingTo);

    if (!activeStrong) {
      pending.push({ from: openingFrom, to: openingTo, decoration: hiddenSyntax });
    }

    addStyledContentDecoration(
      pending,
      lineFrom + contentStart,
      lineFrom + contentEnd,
      liveStrong,
      syntaxRanges,
      activeRanges
    );

    if (!activeStrong) {
      pending.push({ from: closingFrom, to: closingTo, decoration: hiddenSyntax });
    }
  }

  for (const match of lineText.matchAll(/`([^`]+?)`/g)) {
    const matchStart = match.index ?? 0;
    const contentStart = matchStart + 1;
    const contentEnd = matchStart + match[0].length - 1;
    const openingFrom = lineFrom + matchStart;
    const openingTo = lineFrom + contentStart;
    const closingFrom = lineFrom + contentEnd;
    const closingTo = lineFrom + matchStart + match[0].length;
    const inlineCodeRange = syntaxRanges.find((range) => range.from === openingFrom && range.to === closingTo);
    const activeInlineCode = inlineCodeRange
      ? rangeIsActive(inlineCodeRange, activeRanges)
      : inlineCodeRangeIsActive(view, openingFrom, closingTo);

    if (!activeInlineCode) {
      pending.push({ from: openingFrom, to: openingTo, decoration: hiddenSyntax });
    }

    if (activeInlineCode) {
      pending.push({ from: openingFrom, to: closingTo, decoration: liveCode });
    } else {
      pending.push({ from: lineFrom + contentStart, to: lineFrom + contentEnd, decoration: liveCode });
      pending.push({ from: closingFrom, to: closingTo, decoration: hiddenSyntax });
    }
  }

  for (const match of lineText.matchAll(/!\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g)) {
    const matchStart = match.index ?? 0;
    const matchEnd = matchStart + match[0].length;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length, inlineCodeRanges)) continue;

    const imageRange = syntaxRanges.find((range) => range.from === lineFrom + matchStart && range.to === lineFrom + matchEnd);
    const activeImage = imageRange
      ? rangeIsActive(imageRange, activeRanges)
      : syntaxRangeIsActive(view, lineFrom + matchStart, lineFrom + matchEnd);

    if (!activeImage) {
      pending.push({
        from: lineFrom + matchStart,
        to: lineFrom + matchEnd,
        decoration: Decoration.replace({
          widget: new ImageWidget(match[2], match[1] || 'Markdown image')
        })
      });
    }
  }

  for (const match of lineText.matchAll(/~~(.+?)~~/g)) {
    const matchStart = match.index ?? 0;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length, inlineCodeRanges)) continue;

    const contentStart = matchStart + 2;
    const contentEnd = matchStart + match[0].length - 2;
    const openingFrom = lineFrom + matchStart;
    const openingTo = lineFrom + contentStart;
    const closingFrom = lineFrom + contentEnd;
    const closingTo = lineFrom + matchStart + match[0].length;
    const strikeRange = syntaxRanges.find((range) => range.from === openingFrom && range.to === closingTo);
    const activeStrike = strikeRange ? rangeIsActive(strikeRange, activeRanges) : syntaxRangeIsActive(view, openingFrom, closingTo);

    if (!activeStrike) {
      pending.push({ from: openingFrom, to: openingTo, decoration: hiddenSyntax });
    }

    addStyledContentDecoration(
      pending,
      lineFrom + contentStart,
      lineFrom + contentEnd,
      liveStrikethrough,
      syntaxRanges,
      activeRanges
    );

    if (!activeStrike) {
      pending.push({ from: closingFrom, to: closingTo, decoration: hiddenSyntax });
    }
  }

  for (const match of lineText.matchAll(/(^|[^\*])\*([^\s*][^*]*?[^\s*]|\S)\*(?!\*)/g)) {
    const prefix = match[1];
    const matchStart = (match.index ?? 0) + prefix.length;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length - prefix.length, inlineCodeRanges)) continue;

    const contentStart = matchStart + 1;
    const contentEnd = matchStart + match[0].length - prefix.length - 1;
    const openingFrom = lineFrom + matchStart;
    const openingTo = lineFrom + contentStart;
    const closingFrom = lineFrom + contentEnd;
    const closingTo = lineFrom + matchStart + match[0].length - prefix.length;
    const emphasisRange = syntaxRanges.find((range) => range.from === openingFrom && range.to === closingTo);
    const activeEmphasis = emphasisRange
      ? rangeIsActive(emphasisRange, activeRanges)
      : syntaxRangeIsActive(view, openingFrom, closingTo);

    if (!activeEmphasis) {
      pending.push({ from: openingFrom, to: openingTo, decoration: hiddenSyntax });
    }

    addStyledContentDecoration(
      pending,
      lineFrom + contentStart,
      lineFrom + contentEnd,
      liveEmphasis,
      syntaxRanges,
      activeRanges
    );

    if (!activeEmphasis) {
      pending.push({ from: closingFrom, to: closingTo, decoration: hiddenSyntax });
    }
  }

  for (const match of lineText.matchAll(/(^|[^_])_([^\s_][^_]*?[^\s_]|\S)_(?!_)/g)) {
    const prefix = match[1];
    const matchStart = (match.index ?? 0) + prefix.length;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length - prefix.length, inlineCodeRanges)) continue;

    const contentStart = matchStart + 1;
    const contentEnd = matchStart + match[0].length - prefix.length - 1;
    const openingFrom = lineFrom + matchStart;
    const openingTo = lineFrom + contentStart;
    const closingFrom = lineFrom + contentEnd;
    const closingTo = lineFrom + matchStart + match[0].length - prefix.length;
    const emphasisRange = syntaxRanges.find((range) => range.from === openingFrom && range.to === closingTo);
    const activeEmphasis = emphasisRange
      ? rangeIsActive(emphasisRange, activeRanges)
      : syntaxRangeIsActive(view, openingFrom, closingTo);

    if (!activeEmphasis) {
      pending.push({ from: openingFrom, to: openingTo, decoration: hiddenSyntax });
    }

    addStyledContentDecoration(
      pending,
      lineFrom + contentStart,
      lineFrom + contentEnd,
      liveEmphasis,
      syntaxRanges,
      activeRanges
    );

    if (!activeEmphasis) {
      pending.push({ from: closingFrom, to: closingTo, decoration: hiddenSyntax });
    }
  }

  for (const match of lineText.matchAll(/(^|[^!])\[([^\]]+)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g)) {
    const prefix = match[1];
    const matchStart = (match.index ?? 0) + prefix.length;
    const suffixEnd = matchStart + match[0].length - prefix.length;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length - prefix.length, inlineCodeRanges)) continue;

    const linkRange = syntaxRanges.find((range) => range.from === lineFrom + matchStart && range.to === lineFrom + suffixEnd);
    const activeLink = linkRange
      ? rangeIsActive(linkRange, activeRanges)
      : syntaxRangeIsActive(view, lineFrom + matchStart, lineFrom + suffixEnd);

    if (!activeLink) {
      pending.push({
        from: lineFrom + matchStart,
        to: lineFrom + suffixEnd,
        decoration: Decoration.replace({
          widget: new LinkWidget(match[2], match[3])
        })
      });
    }
  }
}

export function lineIsOnlyInlineCode(lineText: string): boolean {
  return /^`[^`]+`$/.test(lineText);
}
