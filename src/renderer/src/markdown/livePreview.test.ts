import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { EditorState, Text } from '@codemirror/state';
import type { Decoration, EditorView } from '@codemirror/view';
import {
  buildLivePreviewDecorations,
  collectYamlFrontmatterBlock,
  getTableCursorTarget,
  lineIsHorizontalRule,
  nextHoverLineAfterEditorUpdate,
  safePosAtCoords
} from './livePreview';

const livePreviewSource = readFileSync(new URL('./livePreview.ts', import.meta.url), 'utf8');

interface DecorationSummary {
  from: number;
  to: number;
  className: string | undefined;
  widgetName: string | undefined;
}

function collectDecorationSummaries(docText: string, selection: { anchor: number; head?: number }): DecorationSummary[] {
  const state = EditorState.create({ doc: docText, selection });
  const view = {
    state,
    visibleRanges: [{ from: 0, to: state.doc.length }]
  } as unknown as EditorView;
  const decorations = buildLivePreviewDecorations(view, null);
  const summaries: DecorationSummary[] = [];

  decorations.between(0, state.doc.length, (from: number, to: number, decoration: Decoration) => {
    summaries.push({
      from,
      to,
      className: decoration.spec.class as string | undefined,
      widgetName: decoration.spec.widget?.constructor.name
    });
  });

  return summaries;
}

export const tests = [
  {
    name: 'clears stale hover preview state after document edits',
    run() {
      assert.equal(
        nextHoverLineAfterEditorUpdate(1, {
          docChanged: true,
          selectionSet: false,
          viewportChanged: false
        }),
        null
      );
    }
  },
  {
    name: 'clears stale hover preview state after selection changes',
    run() {
      assert.equal(
        nextHoverLineAfterEditorUpdate(1, {
          docChanged: false,
          selectionSet: true,
          viewportChanged: false
        }),
        null
      );
    }
  },
  {
    name: 'keeps hover preview state for viewport-only updates',
    run() {
      assert.equal(
        nextHoverLineAfterEditorUpdate(1, {
          docChanged: false,
          selectionSet: false,
          viewportChanged: true
        }),
        1
      );
    }
  },
  {
    name: 'does not rebuild hover preview while dragging',
    run() {
      assert.match(livePreviewSource, /if \(event\.buttons !== 0\) return;/);
    }
  },
  {
    name: 'keeps a compact selection mark for empty selected lines',
    run() {
      assert.match(livePreviewSource, /class EmptyLineSelectionWidget extends WidgetType/);
      assert.match(livePreviewSource, /cm-compact-empty-selection/);
      assert.match(livePreviewSource, /from === to/);
      assert.match(livePreviewSource, /Decoration\.widget\(\{ widget: new EmptyLineSelectionWidget\(\), side: 1 \}\)/);
    }
  },
  {
    name: 'adds hanging indent metadata to list preview lines',
    run() {
      assert.match(livePreviewSource, /function listWrapLine\(indentLength: number\): Decoration/);
      assert.match(livePreviewSource, /--list-wrap-indent: \$\{indentLength\}ch;/);
      assert.match(livePreviewSource, /class: 'cm-live-list-line'/);
      assert.match(livePreviewSource, /const indentLength = taskMatch\[1\]\.length;/);
      assert.match(livePreviewSource, /const indentLength = listMatch\[1\]\.length;/);
      assert.match(livePreviewSource, /const indentLength = numberedListMatch\[1\]\.length;/);
      assert.match(livePreviewSource, /listWrapLine\(indentLength\)/);
    }
  },
  {
    name: 'replaces nested list indentation with depth-aware marker widgets',
    run() {
      const doc = '- root\n    - nested\n    1. numbered\n    - [ ] task';
      const summaries = collectDecorationSummaries(doc, { anchor: doc.length });
      const nestedLine = Text.of(doc.split('\n')).line(2);
      const numberedLine = Text.of(doc.split('\n')).line(3);
      const taskLine = Text.of(doc.split('\n')).line(4);

      assert.equal(
        summaries.some((summary) => summary.widgetName === 'BulletWidget' && summary.from === nestedLine.from && summary.to === nestedLine.from + 6),
        true
      );
      assert.equal(
        summaries.some((summary) => summary.widgetName === 'NumberedListWidget' && summary.from === numberedLine.from && summary.to === numberedLine.from + 7),
        true
      );
      assert.equal(
        summaries.some((summary) => summary.widgetName === 'CheckboxWidget' && summary.from === taskLine.from && summary.to === taskLine.from + 10),
        true
      );
    }
  },
  {
    name: 'reveals source markers on selected preview lines while tables stay rendered',
    run() {
      assert.match(livePreviewSource, /lineIntersectsSelection\(view, line\.from, line\.to\)/);
      assert.match(livePreviewSource, /rangeContainsSelection\(view, markerStart, markerEnd\)/);
      assert.match(livePreviewSource, /lineContainsSelection\(view, line\.from, line\.to\)/);
      assert.match(livePreviewSource, /new TableWidget\(table, selectedTable\)/);
    }
  },
  {
    name: 'keeps selected non-table Markdown source visible under preview styling',
    run() {
      const doc = '# Heading\n- item\n---\n*emphasis*';
      const summaries = collectDecorationSummaries(doc, { anchor: 0, head: doc.length });

      assert.equal(summaries.some((summary) => summary.className === 'cm-live-heading-1'), true);
      assert.equal(summaries.some((summary) => summary.className === 'cm-markdown-syntax-hidden'), false);
      assert.equal(summaries.some((summary) => summary.widgetName === 'BulletWidget'), false);
      assert.equal(summaries.some((summary) => summary.widgetName === 'HorizontalRuleWidget'), false);
    }
  },
  {
    name: 'keeps selected Markdown tables rendered as preview widgets',
    run() {
      const doc = '| Name | Status |\n| --- | --- |\n| Lithe | Ready |';
      const summaries = collectDecorationSummaries(doc, { anchor: 0, head: doc.length });

      assert.equal(summaries.some((summary) => summary.className === 'cm-live-table-line cm-live-table-selection-hidden'), true);
      assert.equal(summaries.some((summary) => summary.widgetName === 'TableWidget'), true);
    }
  },
  {
    name: 'ignores transient coordinate lookup failures',
    run() {
      const view = {
        posAtCoords() {
          throw new Error('No tile at position 73');
        }
      };

      assert.equal(safePosAtCoords(view, { x: 10, y: 10 }), null);
    }
  },
  {
    name: 'returns coordinate positions when lookup succeeds',
    run() {
      const view = {
        posAtCoords() {
          return 4;
        }
      };

      assert.equal(safePosAtCoords(view, { x: 10, y: 10 }), 4);
    }
  },
  {
    name: 'recognizes standalone Markdown horizontal rules',
    run() {
      assert.equal(lineIsHorizontalRule('---'), true);
      assert.equal(lineIsHorizontalRule('***'), true);
      assert.equal(lineIsHorizontalRule('___'), true);
      assert.equal(lineIsHorizontalRule(' - - - '), true);
    }
  },
  {
    name: 'rejects non-rule lines that contain rule characters',
    run() {
      assert.equal(lineIsHorizontalRule('--'), false);
      assert.equal(lineIsHorizontalRule('---- text'), false);
      assert.equal(lineIsHorizontalRule('    ---'), false);
      assert.equal(lineIsHorizontalRule('*** emphasis ***'), false);
    }
  },
  {
    name: 'collects a YAML frontmatter block at the top of the document',
    run() {
      const block = collectYamlFrontmatterBlock(Text.of(['---', 'title: Draft', 'tags:', '  - qa', '---', '# Body']));

      assert.deepEqual(block, { startLine: 1, endLine: 5 });
    }
  },
  {
    name: 'does not collect horizontal rules away from the document start as frontmatter',
    run() {
      const block = collectYamlFrontmatterBlock(Text.of(['# Body', '', '---']));

      assert.equal(block, null);
    }
  },
  {
    name: 'moves a rendered table cursor to the next existing line',
    run() {
      const doc = Text.of(['| Name | Status |', '| --- | --- |', '| Lithe | Ready |', 'after']);

      assert.deepEqual(
        getTableCursorTarget(
          doc,
          {
            startLine: 1,
            endLine: 3,
            headers: ['Name', 'Status'],
            rows: [['Lithe', 'Ready']]
          },
          doc.line(2).from
        ),
        { anchor: doc.line(4).from, insertBreakAt: null }
      );
    }
  },
  {
    name: 'creates a following line for a rendered table at document end',
    run() {
      const doc = Text.of(['| Name | Status |', '| --- | --- |', '| Lithe | Ready |']);

      assert.deepEqual(
        getTableCursorTarget(
          doc,
          {
            startLine: 1,
            endLine: 3,
            headers: ['Name', 'Status'],
            rows: [['Lithe', 'Ready']]
          },
          doc.line(3).to
        ),
        { anchor: doc.line(3).to + 1, insertBreakAt: doc.line(3).to }
      );
    }
  },
];
