import { EditorView, WidgetType } from '@codemirror/view';
import { serializeMarkdownTable, type MarkdownTable } from './table';

const CELL_DRAG_THRESHOLD_PX = 6;
const selectedCellClasses = ['is-selected'];

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
    bullet.textContent = '•';
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

  private markdownFromDOM(table: HTMLTableElement): string {
    const headers = Array.from(table.querySelectorAll('thead th')).map((cell) => cell.textContent ?? '');
    const rows = Array.from(table.querySelectorAll('tbody tr')).map((row) =>
      Array.from(row.querySelectorAll('td')).map((cell) => cell.textContent ?? '')
    );
    return serializeMarkdownTable(headers, rows);
  }

  private updateDocument(view: EditorView, table: HTMLTableElement): void {
    const markdown = this.markdownFromDOM(table);
    if (markdown === this.markdownSource()) return;
    if (this.table.endLine > view.state.doc.lines) return;

    const startLine = view.state.doc.line(this.table.startLine);
    const endLine = view.state.doc.line(this.table.endLine);
    view.dispatch({
      changes: {
        from: startLine.from,
        to: endLine.to,
        insert: markdown
      }
    });
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
    const table = document.createElement('table');
    table.className = 'cm-live-table';
    table.dataset.markdown = this.markdownSource();
    table.tabIndex = 0;
    const abortController = new AbortController();

    let dragStart: { row: number; column: number } | null = null;
    let dragOrigin: { x: number; y: number } | null = null;
    let externalDragStart: { x: number; y: number } | null = null;
    let draggingCells = false;

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
    const clearDocumentSelection = (): void => {
      window.getSelection()?.removeAllRanges();
      const selection = view.state.selection.main;
      if (!selection.empty) {
        view.dispatch({ selection: { anchor: selection.head } });
      }
    };

    const getSelectedCells = (): HTMLTableCellElement[] => Array.from(table.querySelectorAll<HTMLTableCellElement>('.is-selected'));
    const getAllCells = (): HTMLTableCellElement[] => Array.from(table.querySelectorAll<HTMLTableCellElement>('th, td'));
    const updateSelectionOutline = (): void => {
      const selectedCells = getSelectedCells();
      if (selectedCells.length === 0) {
        table.style.removeProperty('--selection-outline-left');
        table.style.removeProperty('--selection-outline-top');
        table.style.removeProperty('--selection-outline-width');
        table.style.removeProperty('--selection-outline-height');
        return;
      }

      const tableRect = table.getBoundingClientRect();
      const selectedRects = selectedCells.map((cell) => cell.getBoundingClientRect());
      const left = Math.min(...selectedRects.map((rect) => rect.left)) - tableRect.left;
      const top = Math.min(...selectedRects.map((rect) => rect.top)) - tableRect.top;
      const right = Math.max(...selectedRects.map((rect) => rect.right)) - tableRect.left;
      const bottom = Math.max(...selectedRects.map((rect) => rect.bottom)) - tableRect.top;

      table.style.setProperty('--selection-outline-left', `${left}px`);
      table.style.setProperty('--selection-outline-top', `${top}px`);
      table.style.setProperty('--selection-outline-width', `${right - left}px`);
      table.style.setProperty('--selection-outline-height', `${bottom - top}px`);
    };
    const clearCellSelection = (): void => {
      table.classList.remove('has-cell-selection');
      table.classList.remove('is-cell-dragging');
      for (const cell of getSelectedCells()) {
        cell.classList.remove(...selectedCellClasses);
      }
      updateSelectionOutline();
    };
    const selectCellRange = (from: { row: number; column: number }, to: { row: number; column: number }): void => {
      clearCellSelection();
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
      table.classList.toggle('has-cell-selection', getSelectedCells().length > 0);
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
      if (table.contains(event.target as Node | null)) {
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
      const activeDragStart = externalDragStart ?? dragOrigin;
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

      if (externalDragStart === null && cellPositionsMatch(dragStart, cellPosition)) {
        clearCellSelection();
        draggingCells = false;
        return;
      }

      dragStart ??= getCellPosition(
        externalDragStart === null ? cell : (getNearestCellAtPoint(externalDragStart.x, externalDragStart.y) ?? cell)
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
      draggingCells = false;
    };

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
      table.classList.remove('is-cell-dragging');
      draggingCells = false;
    });
    table.addEventListener('focusout', (event) => {
      if (table.contains(event.relatedTarget as Node | null)) return;

      clearCellSelection();
    });
    table.addEventListener('keydown', handleKeydown);
    document.addEventListener('pointerdown', clearSelectionOnOutsidePointer, { capture: true, signal: abortController.signal });
    document.addEventListener('pointermove', extendCellDrag, { capture: true, signal: abortController.signal });
    document.addEventListener('pointerup', clearExternalDragStart, { capture: true, signal: abortController.signal });
    document.addEventListener('mousedown', clearSelectionOnOutsidePointer, { capture: true, signal: abortController.signal });
    document.addEventListener('mousemove', extendCellDrag, { capture: true, signal: abortController.signal });
    document.addEventListener('mouseup', clearExternalDragStart, { capture: true, signal: abortController.signal });
    document.addEventListener('selectionchange', convertNativeSelectionToTableSelection, { signal: abortController.signal });
    table.addEventListener('lithe-table-destroy', () => abortController.abort(), { once: true });

    const thead = document.createElement('thead');
    const headerRow = document.createElement('tr');
    for (const [index, header] of this.table.headers.entries()) {
      const cell = this.createCell('th', header, 0, index);
      headerRow.append(cell);
    }
    thead.append(headerRow);
    table.append(thead);

    const tbody = document.createElement('tbody');
    for (const [rowIndex, row] of this.table.rows.entries()) {
      const tableRow = document.createElement('tr');
      for (let index = 0; index < this.table.headers.length; index += 1) {
        const cell = this.createCell('td', row[index] ?? '', rowIndex + 1, index);
        tableRow.append(cell);
      }
      tbody.append(tableRow);
    }
    table.append(tbody);

    for (const cell of getAllCells()) {
      cell.addEventListener('mousedown', (event) => {
        dragStart = getCellPosition(cell);
        dragOrigin = { x: event.clientX, y: event.clientY };
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
        table.classList.remove('is-cell-dragging');
        draggingCells = false;
      });
      cell.addEventListener('focus', () => {
        if (!draggingCells) {
          clearCellSelection();
        }
      });
      cell.addEventListener('blur', () => {
        this.updateDocument(view, table);
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

    if (this.selectedByEditorSelection) {
      selectAllCells({ focus: false });
    }

    return table;
  }

  destroy(dom: HTMLElement): void {
    dom.dispatchEvent(new CustomEvent('lithe-table-destroy'));
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

    checkbox.addEventListener('mousedown', (event) => {
      event.preventDefault();
    });
    checkbox.addEventListener('click', toggle);
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
