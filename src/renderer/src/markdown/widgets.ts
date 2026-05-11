import { EditorView, WidgetType } from '@codemirror/view';
import {
  insertMarkdownTableColumn,
  insertMarkdownTableRow,
  moveMarkdownTableColumn,
  moveMarkdownTableVisualRow,
  serializeMarkdownTable,
  type MarkdownTable,
  type MarkdownTableContent
} from './table';

const CELL_DRAG_THRESHOLD_PX = 6;
const selectedCellClasses = ['is-selected'];

interface StoredTableSelection {
  from: { row: number; column: number };
  key: string;
  to: { row: number; column: number };
}

interface ActiveTableDrag {
  key: string;
  origin: { x: number; y: number };
  start: { row: number; column: number };
}

interface TableStructureDragTarget {
  center: number;
  element: HTMLElement;
  index: number;
  size: number;
  start: number;
}

interface TableSelectionRect {
  bottom: number;
  left: number;
  right: number;
  top: number;
}

interface ActiveTableStructureDrag {
  bounds: Pick<DOMRect, 'bottom' | 'left' | 'right' | 'top'>;
  from: number;
  sourceRects: TableSelectionRect[];
  target: HTMLElement | null;
  targets: TableStructureDragTarget[];
  to: number;
  type: 'column' | 'row';
}

let activeTableDrag: ActiveTableDrag | null = null;
let storedTableSelection: StoredTableSelection | null = null;

function pointerMovedBeyondThreshold(start: { x: number; y: number } | null, x: number, y: number): boolean {
  return start !== null && Math.hypot(x - start.x, y - start.y) >= CELL_DRAG_THRESHOLD_PX;
}

function cellPositionsMatch(
  first: { row: number; column: number } | null,
  second: { row: number; column: number }
): boolean {
  return first !== null && first.row === second.row && first.column === second.column;
}

function safeHref(href: string): string {
  if (href.startsWith('#')) return href;

  try {
    const url = new URL(href, window.location.href);
    return ['http:', 'https:', 'mailto:', 'file:'].includes(url.protocol) ? url.href : '#';
  } catch {
    return '#';
  }
}

export class BulletWidget extends WidgetType {
  toDOM(): HTMLElement {
    const bullet = document.createElement('span');
    bullet.className = 'cm-live-bullet';

    const dot = document.createElement('span');
    dot.className = 'cm-live-bullet-dot';
    dot.textContent = '•';
    bullet.append(dot);

    bullet.draggable = false;
    bullet.contentEditable = 'false';
    bullet.setAttribute('aria-hidden', 'true');
    return bullet;
  }

  ignoreEvent(): boolean {
    return true;
  }
}

export class NumberedListWidget extends WidgetType {
  constructor(private readonly marker: string) {
    super();
  }

  eq(other: NumberedListWidget): boolean {
    return other.marker === this.marker;
  }

  toDOM(): HTMLElement {
    const marker = document.createElement('span');
    marker.className = 'cm-live-numbered-marker';
    marker.textContent = this.marker;
    marker.draggable = false;
    marker.contentEditable = 'false';
    marker.setAttribute('aria-hidden', 'true');
    return marker;
  }

  ignoreEvent(): boolean {
    return true;
  }
}

export class HorizontalRuleWidget extends WidgetType {
  toDOM(): HTMLElement {
    const rule = document.createElement('span');
    rule.className = 'cm-live-horizontal-rule';
    return rule;
  }
}

export class TableWidget extends WidgetType {
  constructor(
    private readonly table: MarkdownTable,
    private readonly selectedByEditorSelection = false
  ) {
    super();
  }

  eq(other: TableWidget): boolean {
    return (
      other.table.startLine === this.table.startLine &&
      other.table.endLine === this.table.endLine &&
      this.markdownSource() === other.markdownSource() &&
      other.selectedByEditorSelection === this.selectedByEditorSelection
    );
  }

  private markdownSource(): string {
    return serializeMarkdownTable(this.table.headers, this.table.rows);
  }

  private selectionKey(): string {
    return `${this.table.startLine}:${this.table.endLine}:${this.markdownSource()}`;
  }

  private tableContentFromDOM(table: HTMLTableElement): MarkdownTableContent {
    const headers = Array.from(table.querySelectorAll('thead th')).map((cell) => cell.textContent ?? '');
    const rows = Array.from(table.querySelectorAll('tbody tr')).map((row) =>
      Array.from(row.querySelectorAll('td')).map((cell) => cell.textContent ?? '')
    );

    return { headers, rows };
  }

  private markdownFromDOM(table: HTMLTableElement): string {
    const { headers, rows } = this.tableContentFromDOM(table);
    return serializeMarkdownTable(headers, rows);
  }

  private replaceDocumentTable(view: EditorView, markdown: string, selectionAnchor?: number): void {
    if (markdown === this.markdownSource()) return;
    if (this.table.endLine > view.state.doc.lines) return;

    const startLine = view.state.doc.line(this.table.startLine);
    const endLine = view.state.doc.line(this.table.endLine);
    const transaction: Parameters<EditorView['dispatch']>[0] = {
      changes: {
        from: startLine.from,
        to: endLine.to,
        insert: markdown
      }
    };

    if (selectionAnchor !== undefined) {
      transaction.selection = { anchor: selectionAnchor };
      transaction.scrollIntoView = true;
    }

    view.dispatch(transaction);
  }

  private updateDocument(view: EditorView, table: HTMLTableElement): void {
    this.replaceDocumentTable(view, this.markdownFromDOM(table));
  }

  private updateDocumentContent(view: EditorView, content: MarkdownTableContent): void {
    const startLine = this.table.endLine <= view.state.doc.lines ? view.state.doc.line(this.table.startLine) : null;
    this.replaceDocumentTable(view, serializeMarkdownTable(content.headers, content.rows), startLine?.from);
    view.focus();
  }

  private deleteDocumentTable(view: EditorView): void {
    if (this.table.endLine > view.state.doc.lines) return;

    const startLine = view.state.doc.line(this.table.startLine);
    const endLine = view.state.doc.line(this.table.endLine);
    const to = this.table.endLine < view.state.doc.lines ? view.state.doc.line(this.table.endLine + 1).from : endLine.to;
    view.dispatch({
      changes: {
        from: startLine.from,
        to,
        insert: ''
      },
      selection: { anchor: startLine.from },
      scrollIntoView: true
    });
    view.focus();
  }

  private createCell(tagName: 'td' | 'th', text: string, row: number, column: number): HTMLTableCellElement {
    const cell = document.createElement(tagName);
    cell.textContent = text;
    cell.contentEditable = 'plaintext-only';
    cell.dataset.row = String(row);
    cell.dataset.column = String(column);
    cell.spellcheck = false;
    return cell;
  }

  toDOM(view: EditorView): HTMLElement {
    const frame = document.createElement('span');
    frame.className = 'cm-live-table-frame';

    const controlLayer = document.createElement('span');
    controlLayer.className = 'cm-live-table-controls';

    const columnAddButton = document.createElement('button');
    columnAddButton.className = 'cm-live-table-add cm-live-table-add-column';
    columnAddButton.type = 'button';
    columnAddButton.textContent = '+';
    columnAddButton.tabIndex = -1;
    columnAddButton.title = 'Add column';
    columnAddButton.setAttribute('aria-label', 'Add column');

    const rowAddButton = document.createElement('button');
    rowAddButton.className = 'cm-live-table-add cm-live-table-add-row';
    rowAddButton.type = 'button';
    rowAddButton.textContent = '+';
    rowAddButton.tabIndex = -1;
    rowAddButton.title = 'Add row';
    rowAddButton.setAttribute('aria-label', 'Add row');

    const columnHandleLayer = document.createElement('span');
    columnHandleLayer.className = 'cm-live-table-column-handles';

    const rowHandleLayer = document.createElement('span');
    rowHandleLayer.className = 'cm-live-table-row-handles';

    controlLayer.append(columnHandleLayer, rowHandleLayer, columnAddButton, rowAddButton);

    const table = document.createElement('table');
    table.className = 'cm-live-table';
    table.dataset.markdown = this.markdownSource();
    table.tabIndex = 0;
    const abortController = new AbortController();

    let dragStart: { row: number; column: number } | null = null;
    let dragOrigin: { x: number; y: number } | null = null;
    let externalDragStart: { x: number; y: number } | null = null;
    let draggingCells = false;
    let activeStructureDrag: ActiveTableStructureDrag | null = null;

    const setEditorTableCursorHidden = (hidden: boolean): void => {
      view.dom.classList.toggle('has-live-table-cursor-hidden', hidden);
    };
    const getCellPosition = (cell: HTMLTableCellElement): { row: number; column: number } => ({
      row: Number(cell.dataset.row ?? 0),
      column: Number(cell.dataset.column ?? 0)
    });
    const getCellAtPoint = (x: number, y: number): HTMLTableCellElement | null => {
      const cells = getAllCells();
      if (cells.length === 0) return null;

      const containingCell = cells.find((cell) => {
        const rect = cell.getBoundingClientRect();
        return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
      });
      if (containingCell) return containingCell;

      const tableRect = table.getBoundingClientRect();
      if (x < tableRect.left || x > tableRect.right || y < tableRect.top || y > tableRect.bottom) return null;

      return getNearestCellAtPoint(x, y);
    };
    const getNearestCellAtPoint = (x: number, y: number): HTMLTableCellElement | null => {
      const cells = getAllCells();
      if (cells.length === 0) return null;

      return cells.reduce((nearest, cell) => {
        const rect = cell.getBoundingClientRect();
        const clampedX = Math.min(Math.max(x, rect.left), rect.right);
        const clampedY = Math.min(Math.max(y, rect.top), rect.bottom);
        const distance = Math.hypot(x - clampedX, y - clampedY);

        return distance < nearest.distance ? { cell, distance } : nearest;
      }, { cell: cells[0], distance: Number.POSITIVE_INFINITY }).cell;
    };
    const externalDragCrossesTable = (x: number, y: number): boolean => {
      if (externalDragStart === null) return false;

      const tableRect = table.getBoundingClientRect();
      const startedAboveOrBelow = externalDragStart.y < tableRect.top || externalDragStart.y > tableRect.bottom;
      const crossesTableX = Math.max(externalDragStart.x, x) >= tableRect.left && Math.min(externalDragStart.x, x) <= tableRect.right;
      const crossesTableY = Math.max(externalDragStart.y, y) >= tableRect.top && Math.min(externalDragStart.y, y) <= tableRect.bottom;

      return startedAboveOrBelow && crossesTableX && crossesTableY;
    };
    const tableSelectionKey = this.selectionKey();
    const clearDocumentSelection = (): void => {
      window.getSelection()?.removeAllRanges();
      const selection = view.state.selection.main;
      if (!selection.empty) {
        view.dispatch({ selection: { anchor: selection.head } });
      }
    };
    const clearStaleTableInteractionSelection = (): void => {
      const selection = view.state.selection.main;
      if (selection.empty && getSelectedCells().length === 0) return;

      clearDocumentSelection();
    };

    const getSelectedCells = (): HTMLTableCellElement[] => Array.from(table.querySelectorAll<HTMLTableCellElement>('.is-selected'));
    const getAllCells = (): HTMLTableCellElement[] => Array.from(table.querySelectorAll<HTMLTableCellElement>('th, td'));
    const cellAtStoredPosition = (position: { row: number; column: number }): HTMLTableCellElement =>
      getAllCells().find((cell) => cellPositionsMatch(position, getCellPosition(cell))) ?? getAllCells()[0];
    const getVisualRows = (): HTMLTableRowElement[] => Array.from(table.querySelectorAll<HTMLTableRowElement>('tr'));
    const headerCells = (): HTMLTableCellElement[] => Array.from(table.querySelectorAll<HTMLTableCellElement>('thead th'));
    const firstVisualRowCells = (): HTMLTableCellElement[] => Array.from(table.querySelectorAll<HTMLTableCellElement>('tr > :first-child'));
    const getColumnCells = (column: number): HTMLTableCellElement[] =>
      getAllCells().filter((cell) => getCellPosition(cell).column === column);
    const distanceToRect = (x: number, y: number, rect: Pick<DOMRect, 'bottom' | 'left' | 'right' | 'top'>): number => {
      const clampedX = Math.min(Math.max(x, rect.left), rect.right);
      const clampedY = Math.min(Math.max(y, rect.top), rect.bottom);
      return Math.hypot(x - clampedX, y - clampedY);
    };
    const isPointInRect = (x: number, y: number, rect: Pick<DOMRect, 'bottom' | 'left' | 'right' | 'top'>): boolean =>
      distanceToRect(x, y, rect) === 0;
    const nearestStructureTarget = (
      targets: TableStructureDragTarget[],
      point: number
    ): TableStructureDragTarget | null => {
      let nearest: TableStructureDragTarget | null = null;
      let nearestDistance = Number.POSITIVE_INFINITY;
      for (const target of targets) {
        const distance = Math.abs(point - target.center);
        if (distance < nearestDistance) {
          nearest = target;
          nearestDistance = distance;
        }
      }

      return nearest;
    };
    const setControlVisible = (control: HTMLElement, visible: boolean): void => {
      control.classList.toggle('is-control-visible', visible);
    };
    const clearVisibleControls = (): void => {
      for (const control of controlLayer.querySelectorAll<HTMLElement>('.is-control-visible')) {
        control.classList.remove('is-control-visible');
      }
    };
    const updateVisibleTableControls = (event: PointerEvent): void => {
      if (activeStructureDrag !== null) return;

      const controls = Array.from(
        controlLayer.querySelectorAll<HTMLElement>('.cm-live-table-handle, .cm-live-table-add')
      );
      const tableRect = table.getBoundingClientRect();
      const columnCells = headerCells();
      const rowCells = firstVisualRowCells();
      for (const control of controls) {
        const index = Number(control.dataset.index ?? -1);
        const cell = control.classList.contains('cm-live-table-column-handle')
          ? columnCells[index]
          : control.classList.contains('cm-live-table-row-handle')
            ? rowCells[index]
            : null;
        const rect = cell?.getBoundingClientRect();
        const visibilityRect = control.classList.contains('cm-live-table-column-handle') && rect
          ? { left: rect.left - 16, right: rect.right + 16, top: tableRect.top - 44, bottom: rect.bottom + 8 }
          : control.classList.contains('cm-live-table-row-handle') && rect
            ? { left: tableRect.left - 46, right: tableRect.left + 52, top: rect.top - 12, bottom: rect.bottom + 12 }
            : control.classList.contains('cm-live-table-add-column')
              ? { left: tableRect.right - 10, right: tableRect.right + 36, top: tableRect.top, bottom: tableRect.bottom }
              : control.classList.contains('cm-live-table-add-row')
                ? { left: tableRect.left, right: tableRect.right, top: tableRect.bottom - 10, bottom: tableRect.bottom + 36 }
                : control.getBoundingClientRect();
        setControlVisible(control, isPointInRect(event.clientX, event.clientY, visibilityRect));
      }
    };
    const syncTableControls = (): void => {
      const frameRect = frame.getBoundingClientRect();
      const tableRect = table.getBoundingClientRect();
      const columnHandles = Array.from(columnHandleLayer.querySelectorAll<HTMLElement>('.cm-live-table-column-handle'));
      const rowHandles = Array.from(rowHandleLayer.querySelectorAll<HTMLElement>('.cm-live-table-row-handle'));

      for (const [index, cell] of headerCells().entries()) {
        const handle = columnHandles[index];
        if (!handle) continue;
        const rect = cell.getBoundingClientRect();
        handle.style.left = `${rect.left - frameRect.left}px`;
        handle.style.top = `${tableRect.top - frameRect.top - 21}px`;
        handle.style.width = `${rect.width}px`;
      }

      for (const [index, cell] of firstVisualRowCells().entries()) {
        const handle = rowHandles[index];
        if (!handle) continue;
        const rect = cell.getBoundingClientRect();
        handle.style.left = `${tableRect.left - frameRect.left - 26}px`;
        handle.style.top = `${rect.top - frameRect.top}px`;
        handle.style.width = '22px';
        handle.style.height = `${rect.height}px`;
      }

      columnAddButton.style.left = `${tableRect.right - frameRect.left + 3}px`;
      columnAddButton.style.top = `${tableRect.top - frameRect.top}px`;
      columnAddButton.style.height = `${tableRect.height}px`;
      rowAddButton.style.left = `${tableRect.left - frameRect.left}px`;
      rowAddButton.style.top = `${tableRect.bottom - frameRect.top + 3}px`;
      rowAddButton.style.width = `${tableRect.width}px`;
    };
    const scheduleTableControlSync = (): void => {
      requestAnimationFrame(syncTableControls);
    };
    const getStructureDragBounds = (type: 'column' | 'row'): Pick<DOMRect, 'bottom' | 'left' | 'right' | 'top'> => {
      const tableRect = table.getBoundingClientRect();
      if (type === 'column') {
        return {
          left: tableRect.left - 56,
          right: tableRect.right + 56,
          top: tableRect.top - 56,
          bottom: tableRect.bottom + 40
        };
      }

      return {
        left: tableRect.left - 64,
        right: tableRect.right + 48,
        top: tableRect.top - 40,
        bottom: tableRect.bottom + 48
      };
    };
    const getStructureDragTargets = (type: 'column' | 'row'): TableStructureDragTarget[] => {
      const handles = Array.from(
        (type === 'column' ? columnHandleLayer : rowHandleLayer).querySelectorAll<HTMLElement>(
          type === 'column' ? '.cm-live-table-column-handle' : '.cm-live-table-row-handle'
        )
      );
      const cells = type === 'column' ? headerCells() : firstVisualRowCells();

      return cells.flatMap((cell, index) => {
        const element = handles[index];
        if (!element) return [];

        const rect = cell.getBoundingClientRect();
        return [
          {
            center: type === 'column' ? rect.left + rect.width / 2 : rect.top + rect.height / 2,
            element,
            index,
            size: type === 'column' ? rect.width : rect.height,
            start: type === 'column' ? rect.left : rect.top
          }
        ];
      });
    };
    const clearSelectionOutlineProperties = (): void => {
      if (frame.classList.contains('has-cell-selection')) return;

      frame.style.removeProperty('--selection-outline-left');
      frame.style.removeProperty('--selection-outline-top');
      frame.style.removeProperty('--selection-outline-width');
      frame.style.removeProperty('--selection-outline-height');
    };
    const getSelectionRects = (cells: HTMLTableCellElement[]): TableSelectionRect[] =>
      cells.map((cell) => {
        const rect = cell.getBoundingClientRect();
        return {
          bottom: rect.bottom,
          left: rect.left,
          right: rect.right,
          top: rect.top
        };
      });
    const shiftedSelectionRects = (rects: TableSelectionRect[], x: number, y: number): TableSelectionRect[] =>
      rects.map((rect) => ({
        bottom: rect.bottom + y,
        left: rect.left + x,
        right: rect.right + x,
        top: rect.top + y
      }));
    const setSelectionOutlineForRects = (rects: TableSelectionRect[]): void => {
      if (rects.length === 0) {
        frame.style.removeProperty('--selection-outline-left');
        frame.style.removeProperty('--selection-outline-top');
        frame.style.removeProperty('--selection-outline-width');
        frame.style.removeProperty('--selection-outline-height');
        return;
      }

      const frameRect = frame.getBoundingClientRect();
      const outlineOutset = 1.5;
      const left = Math.min(...rects.map((rect) => rect.left)) - frameRect.left - outlineOutset;
      const top = Math.min(...rects.map((rect) => rect.top)) - frameRect.top - outlineOutset;
      const right = Math.max(...rects.map((rect) => rect.right)) - frameRect.left + outlineOutset;
      const bottom = Math.max(...rects.map((rect) => rect.bottom)) - frameRect.top + outlineOutset;

      frame.style.setProperty('--selection-outline-left', `${left}px`);
      frame.style.setProperty('--selection-outline-top', `${top}px`);
      frame.style.setProperty('--selection-outline-width', `${right - left}px`);
      frame.style.setProperty('--selection-outline-height', `${bottom - top}px`);
    };
    const setSelectionOutlineForCells = (cells: HTMLTableCellElement[]): void => {
      setSelectionOutlineForRects(getSelectionRects(cells));
    };
    const clearFocusedCellOutline = (): void => {
      frame.classList.remove('has-focused-cell');
      clearSelectionOutlineProperties();
    };
    const updateFocusedCellOutline = (): void => {
      clearFocusedCellOutline();
    };
    const updateSelectionOutline = (): void => {
      const selectedCells = getSelectedCells();
      if (selectedCells.length === 0) {
        clearSelectionOutlineProperties();
        return;
      }

      frame.classList.remove('has-focused-cell');
      setSelectionOutlineForCells(selectedCells);
    };
    const clearCellSelection = (options: { preserveStoredSelection?: boolean } = {}): void => {
      table.classList.remove('has-cell-selection');
      frame.classList.remove('has-cell-selection');
      table.classList.remove('is-cell-dragging');
      for (const cell of getSelectedCells()) {
        cell.classList.remove(...selectedCellClasses);
      }
      if (!options.preserveStoredSelection && storedTableSelection?.key === tableSelectionKey) {
        storedTableSelection = null;
      }
      updateSelectionOutline();
    };
    const selectCellRange = (from: { row: number; column: number }, to: { row: number; column: number }): void => {
      clearCellSelection({ preserveStoredSelection: true });
      const minRow = Math.min(from.row, to.row);
      const maxRow = Math.max(from.row, to.row);
      const minColumn = Math.min(from.column, to.column);
      const maxColumn = Math.max(from.column, to.column);

      for (const cell of getAllCells()) {
        const position = getCellPosition(cell);
        if (position.row >= minRow && position.row <= maxRow && position.column >= minColumn && position.column <= maxColumn) {
          cell.classList.add('is-selected');
        }
      }
      storedTableSelection = { key: tableSelectionKey, from, to };
      table.classList.toggle('has-cell-selection', getSelectedCells().length > 0);
      frame.classList.toggle('has-cell-selection', getSelectedCells().length > 0);
      table.classList.toggle('is-cell-dragging', draggingCells);
      updateSelectionOutline();
    };
    const selectAllCells = (options: { focus: boolean } = { focus: true }): void => {
      selectCellRange(
        { row: 0, column: 0 },
        { row: this.table.rows.length, column: Math.max(0, this.table.headers.length - 1) }
      );
      if (options.focus) {
        table.focus();
      }
    };
    const selectColumn = (column: number): void => {
      selectCellRange({ row: 0, column }, { row: this.table.rows.length, column });
      table.focus();
    };
    const selectVisualRow = (row: number): void => {
      selectCellRange({ row, column: 0 }, { row, column: Math.max(0, this.table.headers.length - 1) });
      table.focus();
    };
    const selectedMarkdown = (): string => {
      const selectedCells = getSelectedCells();
      if (selectedCells.length === 0) return this.markdownFromDOM(table);

      const selectedPositions = selectedCells.map(getCellPosition);
      const selectedColumns = Array.from(new Set(selectedPositions.map((position) => position.column))).sort((a, b) => a - b);
      const selectedBodyRows = Array.from(new Set(selectedPositions.map((position) => position.row).filter((row) => row > 0))).sort(
        (a, b) => a - b
      );
      const headers = selectedColumns.map((column) => table.querySelector<HTMLTableCellElement>(`th[data-column="${column}"]`)?.textContent ?? '');
      const rows = selectedBodyRows.map((row) =>
        selectedColumns.map((column) => table.querySelector<HTMLTableCellElement>(`td[data-row="${row}"][data-column="${column}"]`)?.textContent ?? '')
      );

      return serializeMarkdownTable(headers, rows);
    };
    const clearSelectedCellText = (): void => {
      const selectedCells = getSelectedCells();
      if (selectedCells.length === 0) return;
      if (selectedCells.length === getAllCells().length) {
        this.deleteDocumentTable(view);
        return;
      }

      for (const cell of selectedCells) {
        cell.textContent = '';
      }
      this.updateDocument(view, table);
      clearCellSelection();
      table.focus();
    };
    const replaceWithCurrentTableContent = (content: MarkdownTableContent): void => {
      clearCellSelection();
      this.updateDocumentContent(view, content);
    };
    const addColumn = (): void => {
      replaceWithCurrentTableContent(insertMarkdownTableColumn(this.tableContentFromDOM(table), this.table.headers.length));
    };
    const addRow = (): void => {
      replaceWithCurrentTableContent(insertMarkdownTableRow(this.tableContentFromDOM(table), this.table.rows.length));
    };
    const moveColumn = (from: number, to: number): void => {
      if (from === to) return;
      replaceWithCurrentTableContent(moveMarkdownTableColumn(this.tableContentFromDOM(table), from, to));
    };
    const moveRow = (from: number, to: number): void => {
      if (from === to) return;
      replaceWithCurrentTableContent(moveMarkdownTableVisualRow(this.tableContentFromDOM(table), from, to));
    };
    const clearStructureDragPreview = (): void => {
      for (const cell of getAllCells()) {
        cell.classList.remove('is-structure-preview-cell');
        cell.classList.remove('is-structure-drag-source-cell');
        cell.classList.remove('is-structure-preview-header-cell');
        cell.classList.remove('is-structure-preview-body-cell');
        cell.style.transform = '';
      }
    };
    const setStructurePreviewCellState = (
      cell: HTMLTableCellElement,
      options: { header?: boolean; source?: boolean; transform?: string }
    ): void => {
      const transform = options.transform ?? '';
      cell.classList.add('is-structure-preview-cell');
      cell.classList.toggle('is-structure-drag-source-cell', options.source === true);
      cell.classList.toggle('is-structure-preview-header-cell', options.header === true);
      cell.classList.toggle('is-structure-preview-body-cell', options.header === false);
      if (cell.style.transform !== transform) {
        cell.style.transform = transform;
      }
    };
    const rowIndexAfterMove = (index: number, from: number, to: number): number => {
      if (index === from) return to;
      if (from < to && index > from && index <= to) return index - 1;
      if (from > to && index >= to && index < from) return index + 1;
      return index;
    };
    const structureDragOffsets = (targets: TableStructureDragTarget[], from: number, to: number): number[] => {
      const order = targets.map((_, index) => index);
      const [source] = order.splice(from, 1);
      order.splice(to, 0, source);

      const finalStarts = Array.from({ length: targets.length }, () => 0);
      let cursor = targets[0]?.start ?? 0;
      for (const index of order) {
        finalStarts[index] = cursor;
        cursor += targets[index]?.size ?? 0;
      }

      return targets.map((target, index) => finalStarts[index] - target.start);
    };
    const applyRowDragPreview = (drag: ActiveTableStructureDrag, to: number): void => {
      const { from, targets } = drag;
      const rows = getVisualRows();
      const sourceRow = rows[from];
      const targetRow = rows[to];
      const offsets = structureDragOffsets(targets, from, to);
      if (!sourceRow || !targetRow) return;

      for (const [rowIndex, row] of rows.entries()) {
        const offset = offsets[rowIndex] ?? 0;
        const transform = offset === 0 ? '' : `translateY(${offset}px)`;

        const previewIndex = rowIndexAfterMove(rowIndex, from, to);
        for (const cell of Array.from(row.children) as HTMLTableCellElement[]) {
          setStructurePreviewCellState(cell, {
            header: previewIndex === 0,
            source: rowIndex === from,
            transform
          });
        }
      }
      setSelectionOutlineForRects(shiftedSelectionRects(drag.sourceRects, 0, offsets[from] ?? 0));
    };
    const applyColumnDragPreview = (drag: ActiveTableStructureDrag, to: number): void => {
      const { from, targets } = drag;
      const columns = headerCells();
      const sourceColumn = columns[from];
      const targetColumn = columns[to];
      const offsets = structureDragOffsets(targets, from, to);
      if (!sourceColumn || !targetColumn) return;

      for (let column = 0; column < columns.length; column += 1) {
        const offset = offsets[column] ?? 0;
        const transform = offset === 0 ? '' : `translateX(${offset}px)`;

        for (const cell of getColumnCells(column)) {
          setStructurePreviewCellState(cell, {
            source: column === from,
            transform
          });
        }
      }
      setSelectionOutlineForRects(shiftedSelectionRects(drag.sourceRects, offsets[from] ?? 0, 0));
    };
    const applyStructureDragPreview = (drag: ActiveTableStructureDrag, to: number): void => {
      if (drag.type === 'column') {
        applyColumnDragPreview(drag, to);
        return;
      }

      applyRowDragPreview(drag, to);
    };
    const clearStructureDragTarget = (): void => {
      activeStructureDrag?.target?.classList.remove('is-drop-target');
      activeStructureDrag = activeStructureDrag === null ? null : { ...activeStructureDrag, target: null };
    };
    const structureTargetAtPoint = (
      x: number,
      y: number,
      drag: ActiveTableStructureDrag
    ): TableStructureDragTarget | null => {
      if (!isPointInRect(x, y, drag.bounds)) return null;

      return nearestStructureTarget(drag.targets, drag.type === 'column' ? x : y);
    };
    const updateStructureDragTarget = (event: PointerEvent): void => {
      if (activeStructureDrag === null) return;

      event.preventDefault();
      const nextTarget = structureTargetAtPoint(event.clientX, event.clientY, activeStructureDrag);
      if (nextTarget === null) return;

      const { element: target, index: to } = nextTarget;
      if (target === activeStructureDrag.target && to === activeStructureDrag.to) return;

      activeStructureDrag.target?.classList.remove('is-drop-target');
      target.classList.add('is-drop-target');
      const nextDrag = { ...activeStructureDrag, target, to };
      activeStructureDrag = nextDrag;
      applyStructureDragPreview(nextDrag, to);
    };
    const finishStructureDrag = (event: PointerEvent): void => {
      if (activeStructureDrag === null) return;

      event.preventDefault();
      const { from, type } = activeStructureDrag;
      const target = activeStructureDrag.target ?? structureTargetAtPoint(event.clientX, event.clientY, activeStructureDrag)?.element;
      const to = Number(target?.dataset.index ?? activeStructureDrag.to);

      clearStructureDragTarget();
      frame.querySelectorAll('.is-drag-source').forEach((element) => element.classList.remove('is-drag-source'));
      clearStructureDragPreview();
      frame.classList.remove('is-structure-dragging');
      activeStructureDrag = null;

      if (type === 'column') {
        moveColumn(from, to);
        return;
      }

      moveRow(from, to);
    };
    const startStructureDrag = (event: PointerEvent, type: 'column' | 'row', from: number): void => {
      event.preventDefault();
      event.stopPropagation();
      clearDocumentSelection();
      const target = event.currentTarget as HTMLElement;
      if (type === 'column') {
        selectColumn(from);
      } else {
        selectVisualRow(from);
      }
      const drag = {
        bounds: getStructureDragBounds(type),
        from,
        sourceRects: getSelectionRects(getSelectedCells()),
        target,
        targets: getStructureDragTargets(type),
        to: from,
        type
      };
      activeStructureDrag = drag;
      target.classList.add('is-drop-target');
      target.classList.add('is-drag-source');
      frame.classList.add('is-structure-dragging');
      clearVisibleControls();
      applyStructureDragPreview(drag, from);
      setEditorTableCursorHidden(true);
    };
    const handleKeydown = (event: KeyboardEvent): void => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'a') {
        event.preventDefault();
        selectAllCells();
        return;
      }

      if ((event.key === 'Backspace' || event.key === 'Delete') && getSelectedCells().length > 0) {
        event.preventDefault();
        clearSelectedCellText();
        return;
      }

      if (event.key === 'Escape') {
        clearCellSelection();
        view.focus();
      }
    };
    const clearSelectionOnOutsidePointer = (event: MouseEvent): void => {
      if (frame.contains(event.target as Node | null)) {
        externalDragStart = null;
        return;
      }

      dragStart = null;
      dragOrigin = null;
      externalDragStart = { x: event.clientX, y: event.clientY };
      draggingCells = false;
      clearCellSelection();
    };
    const clearExternalDragStart = (): void => {
      if (activeTableDrag?.key === tableSelectionKey) {
        activeTableDrag = null;
      }
      externalDragStart = null;
      dragOrigin = null;
      table.classList.remove('is-cell-dragging');
    };
    const nativeSelectionTouchesTable = (): boolean => {
      if (externalDragStart === null) return false;

      const selection = window.getSelection();
      if (!selection || selection.isCollapsed || selection.rangeCount === 0) return false;

      const tableRect = table.getBoundingClientRect();
      for (let index = 0; index < selection.rangeCount; index += 1) {
        for (const rect of Array.from(selection.getRangeAt(index).getClientRects())) {
          const overlapsTable =
            rect.right >= tableRect.left && rect.left <= tableRect.right && rect.bottom >= tableRect.top && rect.top <= tableRect.bottom;
          if (overlapsTable) return true;
        }
      }

      return false;
    };
    const editorSelectionTouchesTable = (): boolean => {
      const selection = view.state.selection.main;
      if (selection.empty) return false;

      const startLine = view.state.doc.line(this.table.startLine);
      const endLine = view.state.doc.line(this.table.endLine);

      return selection.from < endLine.to && selection.to > startLine.from;
    };
    const convertNativeSelectionToTableSelection = (): void => {
      const nativeSelectionTouchesRenderedTable = nativeSelectionTouchesTable();
      if (!nativeSelectionTouchesRenderedTable && !editorSelectionTouchesTable()) return;

      draggingCells = nativeSelectionTouchesRenderedTable;
      selectAllCells({ focus: false });
    };
    const extendCellDrag = (event: MouseEvent): void => {
      if (event.buttons !== 1 && externalDragStart === null) return;
      const continuedDrag = activeTableDrag?.key === tableSelectionKey ? activeTableDrag : null;
      const activeDragStart = externalDragStart ?? dragOrigin ?? continuedDrag?.origin ?? null;
      if (!pointerMovedBeyondThreshold(activeDragStart, event.clientX, event.clientY)) return;

      if (externalDragCrossesTable(event.clientX, event.clientY)) {
        event.preventDefault();
        draggingCells = true;
        clearDocumentSelection();
        selectAllCells();
        return;
      }

      const cell = getCellAtPoint(event.clientX, event.clientY);
      if (!cell || !table.contains(cell)) return;
      const cellPosition = getCellPosition(cell);
      const dragStartPosition = dragStart ?? continuedDrag?.start ?? null;

      if (externalDragStart === null && cellPositionsMatch(dragStartPosition, cellPosition)) {
        clearCellSelection();
        draggingCells = false;
        return;
      }

      dragStart ??= getCellPosition(
        continuedDrag !== null
          ? cellAtStoredPosition(continuedDrag.start)
          : externalDragStart === null
            ? cell
            : (getNearestCellAtPoint(externalDragStart.x, externalDragStart.y) ?? cell)
      );
      draggingCells = true;
      selectCellRange(dragStart, cellPosition);
    };
    const dragEndsInStartCell = (event: MouseEvent): boolean => {
      if (externalDragStart !== null || dragStart === null) return false;

      const cell = getCellAtPoint(event.clientX, event.clientY);
      return cell !== null && table.contains(cell) && cellPositionsMatch(dragStart, getCellPosition(cell));
    };
    const cancelCellDragSelection = (): void => {
      clearCellSelection();
      const focusedCell = document.activeElement instanceof HTMLTableCellElement ? document.activeElement : null;
      if (focusedCell && table.contains(focusedCell)) {
        updateFocusedCellOutline(focusedCell);
      }
      draggingCells = false;
    };

    const stopFramePointerEvent = (event: Event): void => {
      event.stopPropagation();
    };
    const stopControlPointerEvent = (event: Event): void => {
      event.preventDefault();
      event.stopPropagation();
    };

    frame.addEventListener('mousedown', stopFramePointerEvent);
    frame.addEventListener('pointerdown', stopFramePointerEvent);
    frame.addEventListener('click', stopFramePointerEvent);
    frame.addEventListener('mouseenter', () => {
      setEditorTableCursorHidden(true);
      scheduleTableControlSync();
    });
    frame.addEventListener('mouseleave', () => {
      setEditorTableCursorHidden(frame.contains(document.activeElement));
    });
    frame.addEventListener('focusin', () => {
      setEditorTableCursorHidden(true);
    });
    frame.addEventListener('pointermove', updateVisibleTableControls);

    columnAddButton.addEventListener('mousedown', stopControlPointerEvent);
    columnAddButton.addEventListener('pointerdown', stopControlPointerEvent);
    columnAddButton.addEventListener('click', (event) => {
      stopControlPointerEvent(event);
      addColumn();
    });
    rowAddButton.addEventListener('mousedown', stopControlPointerEvent);
    rowAddButton.addEventListener('pointerdown', stopControlPointerEvent);
    rowAddButton.addEventListener('click', (event) => {
      stopControlPointerEvent(event);
      addRow();
    });

    table.addEventListener('copy', (event) => {
      event.preventDefault();
      event.clipboardData?.setData('text/plain', selectedMarkdown());
    });
    table.addEventListener('mousedown', (event) => {
      event.stopPropagation();
    });
    table.addEventListener('pointerdown', (event) => {
      event.stopPropagation();
    });
    table.addEventListener('click', (event) => {
      event.stopPropagation();
    });
    table.addEventListener('mouseenter', () => {
      setEditorTableCursorHidden(true);
    });
    table.addEventListener('mouseleave', () => {
      setEditorTableCursorHidden(table.contains(document.activeElement));
    });
    table.addEventListener('focusin', () => {
      setEditorTableCursorHidden(true);
    });
    table.addEventListener('pointermove', extendCellDrag);
    table.addEventListener('mousemove', extendCellDrag);
    table.addEventListener('mouseup', (event) => {
      if (dragEndsInStartCell(event)) {
        cancelCellDragSelection();
      }

      dragStart = null;
      dragOrigin = null;
      if (draggingCells) {
        clearDocumentSelection();
        table.focus();
      }
      if (activeTableDrag?.key === tableSelectionKey) {
        activeTableDrag = null;
      }
      table.classList.remove('is-cell-dragging');
      draggingCells = false;
    });
    table.addEventListener('focusout', (event) => {
      if (frame.contains(event.relatedTarget as Node | null)) return;

      setEditorTableCursorHidden(false);
      clearFocusedCellOutline();
      clearCellSelection();
    });
    table.addEventListener('keydown', handleKeydown);
    document.addEventListener('pointermove', updateStructureDragTarget, { capture: true, signal: abortController.signal });
    document.addEventListener('pointerup', finishStructureDrag, { capture: true, signal: abortController.signal });
    document.addEventListener('pointermove', updateVisibleTableControls, { capture: true, signal: abortController.signal });
    document.addEventListener('pointerdown', clearSelectionOnOutsidePointer, { capture: true, signal: abortController.signal });
    document.addEventListener('pointermove', extendCellDrag, { capture: true, signal: abortController.signal });
    document.addEventListener('pointerup', clearExternalDragStart, { capture: true, signal: abortController.signal });
    document.addEventListener('mousedown', clearSelectionOnOutsidePointer, { capture: true, signal: abortController.signal });
    document.addEventListener('mousemove', extendCellDrag, { capture: true, signal: abortController.signal });
    document.addEventListener('mouseup', clearExternalDragStart, { capture: true, signal: abortController.signal });
    document.addEventListener('selectionchange', convertNativeSelectionToTableSelection, { signal: abortController.signal });
    frame.addEventListener('lithe-table-destroy', () => abortController.abort(), { once: true });
    frame.addEventListener('lithe-table-destroy', clearStructureDragPreview, { once: true });

    const createRowHandle = (index: number): HTMLButtonElement => {
      const handle = document.createElement('button');
      handle.className = 'cm-live-table-handle cm-live-table-row-handle';
      handle.type = 'button';
      handle.textContent = '::';
      handle.dataset.index = String(index);
      handle.tabIndex = -1;
      handle.title = 'Move row';
      handle.setAttribute('aria-label', 'Move row');
      handle.addEventListener('mousedown', stopControlPointerEvent);
      handle.addEventListener('pointerdown', (event) => startStructureDrag(event, 'row', index));
      return handle;
    };

    const thead = document.createElement('thead');
    const headerRow = document.createElement('tr');
    for (const [index, header] of this.table.headers.entries()) {
      const cell = this.createCell('th', header, 0, index);
      headerRow.append(cell);

      const handle = document.createElement('button');
      handle.className = 'cm-live-table-handle cm-live-table-column-handle';
      handle.type = 'button';
      handle.textContent = '::';
      handle.dataset.index = String(index);
      handle.tabIndex = -1;
      handle.title = 'Move column';
      handle.setAttribute('aria-label', 'Move column');
      handle.addEventListener('mousedown', stopControlPointerEvent);
      handle.addEventListener('pointerdown', (event) => startStructureDrag(event, 'column', index));
      columnHandleLayer.append(handle);
    }
    thead.append(headerRow);
    table.append(thead);
    rowHandleLayer.append(createRowHandle(0));

    const tbody = document.createElement('tbody');
    for (const [rowIndex, row] of this.table.rows.entries()) {
      const tableRow = document.createElement('tr');
      for (let index = 0; index < this.table.headers.length; index += 1) {
        const cell = this.createCell('td', row[index] ?? '', rowIndex + 1, index);
        tableRow.append(cell);
      }
      tbody.append(tableRow);

      rowHandleLayer.append(createRowHandle(rowIndex + 1));
    }
    table.append(tbody);

    for (const cell of getAllCells()) {
      cell.addEventListener('mousedown', (event) => {
        dragStart = getCellPosition(cell);
        dragOrigin = { x: event.clientX, y: event.clientY };
        activeTableDrag = { key: tableSelectionKey, origin: dragOrigin, start: dragStart };
        clearStaleTableInteractionSelection();
        draggingCells = false;
      });
      cell.addEventListener('mouseenter', (event) => {
        if (event.buttons !== 1 || dragStart === null) return;
        if (!pointerMovedBeyondThreshold(dragOrigin, event.clientX, event.clientY)) return;
        const cellPosition = getCellPosition(cell);

        if (cellPositionsMatch(dragStart, cellPosition)) {
          clearCellSelection();
          draggingCells = false;
          return;
        }

        draggingCells = true;
        selectCellRange(dragStart, cellPosition);
      });
      cell.addEventListener('mouseup', (event) => {
        if (dragEndsInStartCell(event)) {
          cancelCellDragSelection();
        }

        dragStart = null;
        dragOrigin = null;
        if (draggingCells) {
          clearDocumentSelection();
          table.focus();
        }
        if (activeTableDrag?.key === tableSelectionKey) {
          activeTableDrag = null;
        }
        table.classList.remove('is-cell-dragging');
        draggingCells = false;
      });
      cell.addEventListener('focus', () => {
        if (!draggingCells && activeTableDrag?.key !== tableSelectionKey) {
          clearCellSelection();
        }
        updateFocusedCellOutline(cell);
      });
      cell.addEventListener('blur', () => {
        this.updateDocument(view, table);
        if (!frame.contains(document.activeElement)) {
          clearFocusedCellOutline();
        }
      });
      cell.addEventListener('keydown', (event) => {
        handleKeydown(event);
        if (event.defaultPrevented) return;

        if (event.key === 'Enter') {
          event.preventDefault();
          cell.blur();
          view.focus();
        }
      });
    }

    if (storedTableSelection?.key === tableSelectionKey) {
      selectCellRange(storedTableSelection.from, storedTableSelection.to);
    } else if (this.selectedByEditorSelection) {
      selectAllCells({ focus: false });
    }

    frame.append(controlLayer, table);
    scheduleTableControlSync();

    const resizeObserver = new ResizeObserver(scheduleTableControlSync);
    resizeObserver.observe(table);
    frame.addEventListener('lithe-table-destroy', () => resizeObserver.disconnect(), { once: true });

    return frame;
  }

  destroy(dom: HTMLElement): void {
    dom.dispatchEvent(new CustomEvent('lithe-table-destroy'));
    dom.closest('.cm-editor')?.classList.remove('has-live-table-cursor-hidden');
  }

  ignoreEvent(): boolean {
    return true;
  }
}

export class ImageWidget extends WidgetType {
  constructor(
    private readonly source: string,
    private readonly alt: string
  ) {
    super();
  }

  eq(other: ImageWidget): boolean {
    return other.source === this.source && other.alt === this.alt;
  }

  toDOM(): HTMLElement {
    const frame = document.createElement('span');
    frame.className = 'cm-live-image';

    const image = document.createElement('img');
    image.src = this.source;
    image.alt = this.alt;
    image.loading = 'lazy';
    image.draggable = false;

    frame.append(image);
    return frame;
  }
}

export class LinkWidget extends WidgetType {
  constructor(
    private readonly label: string,
    private readonly href: string
  ) {
    super();
  }

  eq(other: LinkWidget): boolean {
    return other.label === this.label && other.href === this.href;
  }

  toDOM(): HTMLElement {
    const link = document.createElement('a');
    link.className = 'cm-live-link';
    link.href = safeHref(this.href);
    link.textContent = this.label;
    link.target = '_blank';
    link.rel = 'noreferrer';
    link.title = this.href;
    link.addEventListener('mousedown', (event) => {
      event.stopPropagation();
    });
    link.addEventListener('click', (event) => {
      event.stopPropagation();
    });
    return link;
  }
}

export class CheckboxWidget extends WidgetType {
  constructor(
    private readonly checked: boolean,
    private readonly checkPosition: number
  ) {
    super();
  }

  eq(other: CheckboxWidget): boolean {
    return other.checked === this.checked && other.checkPosition === this.checkPosition;
  }

  toDOM(view: EditorView): HTMLElement {
    const checkbox = document.createElement('span');
    checkbox.className = `cm-live-checkbox${this.checked ? ' is-checked' : ''}`;
    checkbox.setAttribute('aria-label', this.checked ? 'Mark task incomplete' : 'Mark task complete');
    checkbox.setAttribute('role', 'checkbox');
    checkbox.setAttribute('aria-checked', String(this.checked));
    checkbox.tabIndex = 0;

    const box = document.createElement('span');
    box.className = 'cm-live-checkbox-box';
    checkbox.append(box);

    const toggle = (event: Event): void => {
      event.preventDefault();
      event.stopPropagation();

      view.dispatch({
        changes: {
          from: this.checkPosition,
          to: this.checkPosition + 1,
          insert: this.checked ? ' ' : 'x'
        }
      });
      view.focus();
    };

    box.addEventListener('mousedown', (event) => {
      event.preventDefault();
    });
    box.addEventListener('click', toggle);
    checkbox.addEventListener('keydown', (event) => {
      if (event.key === ' ' || event.key === 'Enter') {
        toggle(event);
      }
    });

    return checkbox;
  }
}

export class CodeLanguageWidget extends WidgetType {
  constructor(private readonly language: string) {
    super();
  }

  eq(other: CodeLanguageWidget): boolean {
    return other.language === this.language;
  }

  toDOM(): HTMLElement {
    const label = document.createElement('span');
    label.className = 'cm-live-code-language';
    label.textContent = this.language;
    return label;
  }
}
