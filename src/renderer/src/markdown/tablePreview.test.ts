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
      assert.match(livePreviewSource, /new TableWidget\(table, selectedTable\)/);
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
      assert.match(widgetsSource, /serializeMarkdownTableRows\(selectedRowsContent\)/);
      assert.match(widgetsSource, /selectedRows\[0\] === 0 && selectedRowsContent\.length > 1/);
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
      assert.match(widgetsSource, /CELL_DRAG_THRESHOLD_PX = 6/);
      assert.match(widgetsSource, /pointerMovedBeyondThreshold/);
      assert.match(widgetsSource, /cellPositionsMatch/);
      assert.match(widgetsSource, /dragEndsInStartCell/);
      assert.match(widgetsSource, /cancelCellDragSelection/);
      assert.match(widgetsSource, /selectedCellClasses/);
      assert.match(widgetsSource, /classList\.remove\('is-cell-dragging'\)/);
      assert.match(widgetsSource, /classList\.toggle\('is-cell-dragging', draggingCells\)/);
      assert.match(widgetsSource, /table\.classList\.remove\('is-cell-dragging'\);\n\s+draggingCells = false;/);
      assert.match(widgetsSource, /activeDragStart = externalDragStart \?\? dragOrigin/);
      assert.match(widgetsSource, /crossesTableX/);
      assert.match(widgetsSource, /crossesTableY/);
      assert.match(widgetsSource, /nativeSelectionTouchesTable/);
      assert.match(widgetsSource, /nativeSelectionTouchesRenderedTable/);
      assert.match(widgetsSource, /editorSelectionTouchesTable/);
      assert.match(widgetsSource, /selection\.getRangeAt\(index\)\.getClientRects\(\)/);
      assert.match(widgetsSource, /event\.buttons !== 1 && externalDragStart === null/);
      assert.match(widgetsSource, /selectAllCells\(\);/);
      assert.match(widgetsSource, /selectAllCells\(\{ focus: false \}\);/);
      assert.match(widgetsSource, /getNearestCellAtPoint\(externalDragStart\.x, externalDragStart\.y\)/);
      assert.match(widgetsSource, /dragOrigin = \{ x: event\.clientX, y: event\.clientY \}/);
      assert.match(widgetsSource, /externalDragStart === null && cellPositionsMatch\(dragStartPosition, cellPosition\)/);
      assert.match(widgetsSource, /clearCellSelection\(\);\n\s+draggingCells = false;\n\s+return;/);
      assert.match(widgetsSource, /dragStart \?\?= getCellPosition\(/);
      assert.match(widgetsSource, /window\.getSelection\(\)\?\.removeAllRanges\(\)/);
      assert.match(widgetsSource, /clearStaleTableInteractionSelection/);
      assert.match(widgetsSource, /activeTableDrag = \{ key: tableSelectionKey, origin: dragOrigin, start: dragStart \};\n\s+clearStaleTableInteractionSelection\(\);/);
      assert.match(widgetsSource, /activeTableDrag/);
      assert.match(widgetsSource, /continuedDrag/);
      assert.match(widgetsSource, /cellAtStoredPosition/);
      assert.match(widgetsSource, /activeTableDrag = \{ key: tableSelectionKey, origin: dragOrigin, start: dragStart \};/);
      assert.match(widgetsSource, /activeTableDrag\?\.key !== tableSelectionKey/);
      assert.match(widgetsSource, /storedTableSelection/);
      assert.match(widgetsSource, /selectionKey/);
      assert.match(widgetsSource, /preserveStoredSelection/);
      assert.match(widgetsSource, /storedTableSelection = \{ key: tableSelectionKey, from, to \};/);
      assert.match(widgetsSource, /storedTableSelection\?\.key === tableSelectionKey/);
      assert.match(widgetsSource, /classList\.add\('is-selected'\)/);
      assert.match(widgetsSource, /updateSelectionOutline/);
      assert.match(widgetsSource, /selectedByEditorSelection/);
      assert.match(widgetsSource, /setEditorTableCursorHidden/);
      assert.match(widgetsSource, /has-live-table-cursor-hidden/);
      assert.match(widgetsSource, /table\.addEventListener\('mouseenter'/);
      assert.match(widgetsSource, /table\.addEventListener\('mouseleave'/);
      assert.match(widgetsSource, /table\.addEventListener\('focusin'/);
      assert.match(widgetsSource, /closest\('\.cm-editor'\)\?\.classList\.remove\('has-live-table-cursor-hidden'\)/);
      assert.match(widgetsSource, /--selection-outline-left/);
      assert.match(widgetsSource, /--selection-outline-height/);
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
    name: 'maps editor table selection to cell selection only',
    run() {
      const hiddenSelectionRule = getRuleBody('#editor .cm-line.cm-live-table-selection-hidden .cm-selectionBackground');
      const nativeSelectionRule = getRuleBody('#editor .cm-line.cm-live-table-selection-hidden ::selection');
      const tableSelectionRule = getRuleBody('#editor .cm-live-table.has-cell-selection');

      assert.match(decorationsSource, /selectedTablePreviewLine/);
      assert.match(decorationsSource, /selectedHiddenTableSourceLine/);
      assert.match(livePreviewSource, /tableIntersectsSelection/);
      assert.match(livePreviewSource, /new TableWidget\(table, selectedTable\)/);
      assert.match(livePreviewSource, /selectedTable \? selectedTablePreviewLine : tablePreviewLine/);
      assert.match(livePreviewSource, /selectedTable \? selectedHiddenTableSourceLine : hiddenTableSourceLine/);
      assert.match(hiddenSelectionRule, /background:\s*transparent;/);
      assert.match(nativeSelectionRule, /background:\s*transparent;/);
      assert.match(tableSelectionRule, /user-select:\s*none;/);
    }
  },
  {
    name: 'shows selected table cells with divider color only',
    run() {
      const tableSelectionRule = getRuleBody('#editor .cm-live-table.has-cell-selection');
      const frameSelectionRule = getRuleBody('#editor .cm-live-table-frame.has-cell-selection::after');
      const tableRule = getRuleBody('#editor .cm-live-table');
      const tableCellRule = getRuleBody('#editor .cm-live-table th,\n#editor .cm-live-table td');
      const focusedEditableCellRule = getRuleBody(
        '#editor .cm-live-table th[contenteditable]:focus,\n#editor .cm-live-table td[contenteditable]:focus'
      );
      const focusedHeaderRule = getRuleBody('#editor .cm-live-table.has-cell-selection th[contenteditable]:focus');
      const focusedCellRule = getRuleBody('#editor .cm-live-table.has-cell-selection td[contenteditable]:focus');

      assert.match(tableRule, /position:\s*relative;/);
      assert.match(tableSelectionRule, /outline:\s*0;/);
      assert.match(focusedEditableCellRule, /position:\s*relative;/);
      assert.match(focusedEditableCellRule, /z-index:\s*2;/);
      assert.match(focusedEditableCellRule, /outline:\s*0;/);
      assert.match(focusedEditableCellRule, /box-shadow:\s*none;/);
      assert.match(focusedHeaderRule, /outline:\s*0 !important;/);
      assert.match(focusedHeaderRule, /box-shadow:\s*none;/);
      assert.match(focusedCellRule, /outline:\s*0 !important;/);
      assert.match(focusedCellRule, /box-shadow:\s*none;/);
      assert.doesNotMatch(styles, /#editor \.cm-live-table \.is-selected\s*\{/);
      assert.match(frameSelectionRule, /border:\s*2px solid #6fa09f;/);
      assert.match(frameSelectionRule, /pointer-events:\s*none;/);
      assert.match(frameSelectionRule, /top:\s*var\(--selection-outline-top\);/);
      assert.match(frameSelectionRule, /left:\s*var\(--selection-outline-left\);/);
      assert.doesNotMatch(styles, /#editor \.cm-live-table-frame\.is-structure-dragging\.has-cell-selection::after/);
      assert.match(tableCellRule, /min-width:\s*32px;/);
      assert.match(tableCellRule, /height:\s*30px;/);
      assert.match(widgetsSource, /const outlineOutset = 1\.5;/);
      assert.match(widgetsSource, /frame\.classList\.toggle\('has-cell-selection'/);
      assert.doesNotMatch(widgetsSource, /frame\.classList\.add\('has-focused-cell'\)/);
      assert.doesNotMatch(widgetsSource, /setSelectionOutlineForCells\(\[cell\]\)/);
      assert.match(widgetsSource, /--selection-outline-width/);
      assert.match(widgetsSource, /right - left/);
    }
  },
  {
    name: 'hides the editor cursor around rendered tables',
    run() {
      const cursorRule = getRuleBody('#editor .cm-editor.has-live-table-cursor-hidden .cm-cursor');
      const hoverCursorRule = getRuleBody(
        '#editor .cm-editor:has(.cm-live-table-frame:hover) .cm-cursor,\n#editor .cm-editor:has(.cm-live-table-frame:focus-within) .cm-cursor,\n#editor .cm-editor:has(.cm-live-table.has-cell-selection) .cm-cursor'
      );

      assert.match(cursorRule, /border-left-color:\s*transparent;/);
      assert.match(hoverCursorRule, /border-left-color:\s*transparent;/);
    }
  },
  {
    name: 'hides native table text selection while cell dragging',
    run() {
      const rule = getRuleBody('#editor .cm-live-table.is-cell-dragging ::selection');

      assert.match(rule, /background:\s*transparent;/);
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
      const frameRule = getRuleBody('#editor .cm-live-table-frame');
      const tableRule = getRuleBody('#editor .cm-live-table');
      const tableLineRule = getRuleBody('#editor .cm-line.cm-live-table-line');

      assert.match(decorationsSource, /tablePreviewLine/);
      assert.match(livePreviewSource, /tablePreviewLine/);
      assert.match(frameRule, /display:\s*inline-block;/);
      assert.match(frameRule, /padding:\s*18px 24px 24px 0;/);
      assert.match(tableRule, /display:\s*table;/);
      assert.match(tableRule, /margin:\s*0;/);
      assert.match(tableRule, /line-height:\s*1\.4;/);
      assert.match(tableRule, /vertical-align:\s*top;/);
      assert.match(tableLineRule, /line-height:\s*0;/);
    }
  },
  {
    name: 'supports rendered table structure controls',
    run() {
      const frameRule = getRuleBody('#editor .cm-live-table-frame');
      const controlsRule = getRuleBody('#editor .cm-live-table-controls');
      const addColumnRule = getRuleBody('#editor .cm-live-table-add-column');
      const addRowRule = getRuleBody('#editor .cm-live-table-add-row');
      const sourceCellRule = getRuleBody('#editor .cm-live-table .is-structure-drag-source-cell');
      const previewCellRule = getRuleBody('#editor .cm-live-table .is-structure-preview-cell');
      const previewHeaderRule = getRuleBody('#editor .cm-live-table .is-structure-preview-header-cell');
      const previewBodyRule = getRuleBody('#editor .cm-live-table .is-structure-preview-body-cell');
      const frameSelectionRule = getRuleBody('#editor .cm-live-table-frame.has-cell-selection::after');
      const visibleControlRule = getRuleBody(
        '#editor .cm-live-table-handle.is-control-visible,\n#editor .cm-live-table-add.is-control-visible,\n#editor .cm-live-table-handle.is-drop-target,\n#editor .cm-live-table-handle.is-drag-source'
      );

      assert.match(widgetsSource, /cm-live-table-column-handle/);
      assert.match(widgetsSource, /cm-live-table-row-handle/);
      assert.match(widgetsSource, /cm-live-table-add-column/);
      assert.match(widgetsSource, /cm-live-table-add-row/);
      assert.match(widgetsSource, /startStructureDrag/);
      assert.match(widgetsSource, /finishStructureDrag/);
      assert.match(widgetsSource, /insertMarkdownTableColumn/);
      assert.match(widgetsSource, /insertMarkdownTableRow/);
      assert.match(widgetsSource, /moveMarkdownTableColumn/);
      assert.match(widgetsSource, /moveMarkdownTableVisualRow/);
      assert.match(widgetsSource, /updateVisibleTableControls/);
      assert.match(widgetsSource, /applyStructureDragPreview/);
      assert.match(widgetsSource, /nearestStructureTarget/);
      assert.match(widgetsSource, /getStructureDragTargets/);
      assert.match(widgetsSource, /structureTargetAtPoint/);
      assert.match(widgetsSource, /rowIndexAfterMove/);
      assert.match(widgetsSource, /setStructurePreviewCellState/);
      assert.match(widgetsSource, /structureDragOffsets/);
      assert.match(widgetsSource, /target\.start/);
      assert.match(widgetsSource, /cursor \+= targets\[index\]\?\.size/);
      assert.match(widgetsSource, /sourceRects: getSelectionRects\(getSelectedCells\(\)\)/);
      assert.match(widgetsSource, /shiftedSelectionRects/);
      assert.match(widgetsSource, /setSelectionOutlineForRects/);
      assert.match(widgetsSource, /to:\s*from/);
      assert.match(widgetsSource, /if \(nextTarget === null\) return;/);
      assert.match(widgetsSource, /activeStructureDrag\.to/);
      assert.match(widgetsSource, /document\.addEventListener\('pointermove', updateVisibleTableControls/);
      assert.match(widgetsSource, /clearStructureDragPreview/);
      assert.match(widgetsSource, /selectColumn\(from\)/);
      assert.match(widgetsSource, /selectVisualRow\(from\)/);
      assert.match(widgetsSource, /is-structure-drag-source-cell/);
      assert.match(widgetsSource, /firstVisualRowCells/);
      assert.match(widgetsSource, /createRowHandle\(0\)/);
      assert.match(widgetsSource, /ResizeObserver/);
      assert.match(frameRule, /position:\s*relative;/);
      assert.match(controlsRule, /pointer-events:\s*none;/);
      assert.match(addColumnRule, /border-radius:\s*5px;/);
      assert.match(addRowRule, /height:\s*18px;/);
      assert.match(visibleControlRule, /opacity:\s*1;/);
      assert.match(sourceCellRule, /box-shadow:\s*none;/);
      assert.match(previewCellRule, /transition:\s*transform 150ms ease;/);
      assert.match(frameSelectionRule, /top 150ms ease/);
      assert.match(previewHeaderRule, /font-weight:\s*700;/);
      assert.match(previewBodyRule, /font-weight:\s*400;/);
      assert.match(styles, /height:\s*18px;/);
      assert.match(styles, /width:\s*18px;/);
      assert.match(styles, /min-height:\s*18px;/);
      assert.match(styles, /min-width:\s*22px;/);
    }
  }
];
