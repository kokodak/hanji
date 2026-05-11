import { Decoration, type EditorView } from '@codemirror/view';
import { collapsedInlineSyntax, hiddenSyntax, liveCode, liveEmphasis, liveStrikethrough, liveStrong } from './decorations';
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

function collectStrongRanges(lineFrom: number, lineText: string, inlineCodeRanges: InlineSyntaxRange[]): InlineSyntaxRange[] {
  const ranges: InlineSyntaxRange[] = [];

  for (const match of lineText.matchAll(/(\*\*)(.+?)\1/g)) {
    const matchStart = match.index ?? 0;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length, inlineCodeRanges)) continue;

    const contentStart = matchStart + match[1].length;
    const contentEnd = matchStart + match[0].length - match[1].length;
    ranges.push({
      from: lineFrom + matchStart,
      to: lineFrom + matchStart + match[0].length,
      markers: [
        { from: lineFrom + matchStart, to: lineFrom + contentStart },
        { from: lineFrom + contentEnd, to: lineFrom + matchStart + match[0].length }
      ]
    });
  }

  return ranges;
}

function collectStrikethroughRanges(lineFrom: number, lineText: string, inlineCodeRanges: InlineSyntaxRange[]): InlineSyntaxRange[] {
  const ranges: InlineSyntaxRange[] = [];

  for (const match of lineText.matchAll(/~~(.+?)~~/g)) {
    const matchStart = match.index ?? 0;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length, inlineCodeRanges)) continue;

    ranges.push({
      from: lineFrom + matchStart,
      to: lineFrom + matchStart + match[0].length,
      markers: [
        { from: lineFrom + matchStart, to: lineFrom + matchStart + 2 },
        { from: lineFrom + matchStart + match[0].length - 2, to: lineFrom + matchStart + match[0].length }
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

function rangeOverlapsSyntaxRange(from: number, to: number, ranges: InlineSyntaxRange[]): boolean {
  return ranges.some((range) => from < range.to && to > range.from);
}

function collectAsteriskEmphasisRanges(
  lineFrom: number,
  lineText: string,
  inlineCodeRanges: InlineSyntaxRange[],
  protectedRanges: InlineSyntaxRange[]
): InlineSyntaxRange[] {
  const ranges: InlineSyntaxRange[] = [];
  const usedMarkerPositions = new Set<number>();

  for (let opening = 0; opening < lineText.length; opening += 1) {
    if (lineText[opening] !== '*' || usedMarkerPositions.has(opening)) continue;
    if (lineText[opening + 1] === '*' || /\s/.test(lineText[opening + 1] ?? '')) continue;

    for (let closing = opening + 1; closing < lineText.length; closing += 1) {
      if (lineText[closing] !== '*' || usedMarkerPositions.has(closing)) continue;
      if (lineText[closing - 1] === '*' || /\s/.test(lineText[closing - 1] ?? '')) continue;

      const from = lineFrom + opening;
      const to = lineFrom + closing + 1;
      if (rangeOverlapsInlineCode(from, to, inlineCodeRanges) || rangeOverlapsSyntaxRange(from, to, protectedRanges)) break;

      ranges.push({
        from,
        to,
        markers: [
          { from, to: from + 1 },
          { from: lineFrom + closing, to }
        ]
      });
      usedMarkerPositions.add(opening);
      usedMarkerPositions.add(closing);
      break;
    }
  }

  return ranges;
}

function collectCursorTargetInlineRanges(lineFrom: number, lineText: string): InlineSyntaxRange[] {
  const inlineCodeRanges = collectInlineCodeRanges(lineFrom, lineText);
  const strongRanges = collectStrongRanges(lineFrom, lineText, inlineCodeRanges);
  const strikethroughRanges = collectStrikethroughRanges(lineFrom, lineText, inlineCodeRanges);
  const emphasisRanges = collectAsteriskEmphasisRanges(lineFrom, lineText, inlineCodeRanges, strongRanges);

  return [...inlineCodeRanges, ...strongRanges, ...strikethroughRanges, ...emphasisRanges].sort((first, second) => first.from - second.from);
}

export function getInlinePreviewCursorTarget(lineFrom: number, lineText: string, position: number): number | null {
  const lineTo = lineFrom + lineText.length;
  const range = collectCursorTargetInlineRanges(lineFrom, lineText).find((item) => {
    const closingMarker = item.markers[item.markers.length - 1];

    return item.to === lineTo && position >= closingMarker.from && position <= closingMarker.to;
  });

  return range?.to ?? null;
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
  const strongRanges = collectStrongRanges(lineFrom, lineText, inlineCodeRanges);
  syntaxRanges.push(...strongRanges);

  for (const match of lineText.matchAll(/!\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g)) {
    const matchStart = match.index ?? 0;
    if (matchOverlapsInlineCode(lineFrom, matchStart, match[0].length, inlineCodeRanges)) continue;

    syntaxRanges.push({
      from: lineFrom + matchStart,
      to: lineFrom + matchStart + match[0].length,
      markers: [{ from: lineFrom + matchStart, to: lineFrom + matchStart + match[0].length }]
    });
  }

  syntaxRanges.push(...collectStrikethroughRanges(lineFrom, lineText, inlineCodeRanges));

  const emphasisRanges = collectAsteriskEmphasisRanges(lineFrom, lineText, inlineCodeRanges, strongRanges);
  syntaxRanges.push(...emphasisRanges);

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

  for (const match of lineText.matchAll(/(\*\*)(.+?)\1/g)) {
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
      pending.push({ from: openingFrom, to: openingTo, decoration: collapsedInlineSyntax });
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
      pending.push({ from: closingFrom, to: closingTo, decoration: collapsedInlineSyntax });
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
      pending.push({ from: openingFrom, to: openingTo, decoration: collapsedInlineSyntax });
    }

    if (activeInlineCode) {
      pending.push({ from: openingFrom, to: closingTo, decoration: liveCode });
    } else {
      pending.push({ from: lineFrom + contentStart, to: lineFrom + contentEnd, decoration: liveCode });
      pending.push({ from: closingFrom, to: closingTo, decoration: collapsedInlineSyntax });
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

  for (const emphasisRange of emphasisRanges) {
    const openingFrom = emphasisRange.markers[0].from;
    const openingTo = emphasisRange.markers[0].to;
    const closingFrom = emphasisRange.markers[1].from;
    const closingTo = emphasisRange.markers[1].to;
    const activeEmphasis = rangeIsActive(emphasisRange, activeRanges) || syntaxRangeIsActive(view, openingFrom, closingTo);

    if (!activeEmphasis) {
      pending.push({ from: openingFrom, to: openingTo, decoration: collapsedInlineSyntax });
    }

    addStyledContentDecoration(
      pending,
      openingTo,
      closingFrom,
      liveEmphasis,
      syntaxRanges,
      activeRanges
    );

    if (!activeEmphasis) {
      pending.push({ from: closingFrom, to: closingTo, decoration: collapsedInlineSyntax });
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
