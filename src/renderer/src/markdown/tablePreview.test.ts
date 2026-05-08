import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const livePreviewSource = readFileSync(new URL('./livePreview.ts', import.meta.url), 'utf8');
const widgetsSource = readFileSync(new URL('./widgets.ts', import.meta.url), 'utf8');
const decorationsSource = readFileSync(new URL('./decorations.ts', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../styles.css', import.meta.url), 'utf8');

function getRuleBody(selector: string): string {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = new RegExp(`${escapedSelector}\\s*\\{([^}]*)\\}`).exec(styles);

  assert.ok(match, `Expected ${selector} rule to exist.`);

  return match[1];
}

export const tests = [
  {
    name: 'does not create table previews as multiline replacement decorations',
    run() {
      assert.match(livePreviewSource, /new TableWidget\(table\)/);
      assert.doesNotMatch(livePreviewSource, /activeTable/);
      assert.doesNotMatch(livePreviewSource, /new TableWidget\([^)]*\),\s*block:\s*true/);
      assert.doesNotMatch(livePreviewSource, /to:\s*endLine\.to,\s*decoration:\s*Decoration\.replace/);
    }
  },
  {
    name: 'keeps table widgets editable while copying Markdown text',
    run() {
      assert.match(widgetsSource, /contentEditable = 'plaintext-only';/);
      assert.match(widgetsSource, /event\.clipboardData\?\.setData\('text\/plain', selectedMarkdown\(\)\);/);
      assert.match(widgetsSource, /view\.dispatch\(\{/);
      assert.match(widgetsSource, /insert: markdown/);
    }
  },
  {
    name: 'supports table cell range selection and deletion',
    run() {
      assert.match(widgetsSource, /selectCellRange/);
      assert.match(widgetsSource, /getBoundingClientRect/);
      assert.match(widgetsSource, /externalDragStart/);
      assert.match(widgetsSource, /externalDragCrossesTable/);
      assert.match(widgetsSource, /crossesTableX/);
      assert.match(widgetsSource, /crossesTableY/);
      assert.match(widgetsSource, /nativeSelectionTouchesTable/);
      assert.match(widgetsSource, /selection\.getRangeAt\(index\)\.getClientRects\(\)/);
      assert.match(widgetsSource, /event\.buttons !== 1 && externalDragStart === null/);
      assert.match(widgetsSource, /selectAllCells\(\);/);
      assert.match(widgetsSource, /getNearestCellAtPoint\(externalDragStart\.x, externalDragStart\.y\)/);
      assert.match(widgetsSource, /dragStart \?\?= getCellPosition\(/);
      assert.match(widgetsSource, /window\.getSelection\(\)\?\.removeAllRanges\(\)/);
      assert.match(widgetsSource, /classList\.add\('is-selected'\)/);
      assert.match(widgetsSource, /deleteDocumentTable/);
      assert.match(widgetsSource, /event\.key === 'Backspace' \|\| event\.key === 'Delete'/);
      assert.match(widgetsSource, /event\.key\.toLowerCase\(\) === 'a'/);
      assert.match(widgetsSource, /clearSelectionOnOutsidePointer/);
      assert.match(widgetsSource, /document\.addEventListener\('pointermove', extendCellDrag, \{ capture: true/);
      assert.match(widgetsSource, /document\.addEventListener\('mousemove', extendCellDrag, \{ capture: true/);
      assert.match(widgetsSource, /document\.addEventListener\('selectionchange', convertNativeSelectionToTableSelection/);
      assert.match(widgetsSource, /abortController\.abort\(\)/);
    }
  },
  {
    name: 'clips selection paint to the editor area',
    run() {
      const editorRule = getRuleBody('#editor .cm-editor');

      assert.match(editorRule, /contain:\s*paint;/);
    }
  },
  {
    name: 'collapses hidden Markdown table source lines',
    run() {
      const rule = getRuleBody('#editor .cm-line.cm-live-table-source-hidden');

      assert.match(decorationsSource, /hiddenTableSourceLine/);
      assert.match(livePreviewSource, /hiddenTableSourceLine/);
      assert.match(rule, /height:\s*0;/);
      assert.match(rule, /font-size:\s*0;/);
      assert.match(rule, /line-height:\s*0;/);
    }
  },
  {
    name: 'keeps rendered tables tight to surrounding editor lines',
    run() {
      const tableRule = getRuleBody('#editor .cm-live-table');
      const tableLineRule = getRuleBody('#editor .cm-line.cm-live-table-line');

      assert.match(decorationsSource, /tablePreviewLine/);
      assert.match(livePreviewSource, /tablePreviewLine/);
      assert.match(tableRule, /display:\s*table;/);
      assert.match(tableRule, /margin:\s*0;/);
      assert.match(tableRule, /line-height:\s*1\.4;/);
      assert.match(tableRule, /vertical-align:\s*top;/);
      assert.match(tableLineRule, /line-height:\s*0;/);
    }
  }
];
