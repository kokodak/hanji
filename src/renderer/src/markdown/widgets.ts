import { EditorView, WidgetType } from '@codemirror/view';
import { serializeMarkdownTable, type MarkdownTable } from './table';

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
    return bullet;
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
    return marker;
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
  constructor(private readonly table: MarkdownTable) {
    super();
  }

  eq(other: TableWidget): boolean {
    return (
      other.table.startLine === this.table.startLine &&
      other.table.endLine === this.table.endLine &&
      this.markdownSource() === other.markdownSource()
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

    let dragStart: { row: number; column: number } | null = null;
    let draggingCells = false;

    const getCellPosition = (cell: HTMLTableCellElement): { row: number; column: number } => ({
      row: Number(cell.dataset.row ?? 0),
      column: Number(cell.dataset.column ?? 0)
    });
    const getCellFromPoint = (x: number, y: number): HTMLTableCellElement | null => {
      const element = document.elementFromPoint(x, y);
      return element?.closest<HTMLTableCellElement>('.cm-live-table th, .cm-live-table td') ?? null;
    };

    const getSelectedCells = (): HTMLTableCellElement[] => Array.from(table.querySelectorAll<HTMLTableCellElement>('.is-selected'));
    const getAllCells = (): HTMLTableCellElement[] => Array.from(table.querySelectorAll<HTMLTableCellElement>('th, td'));
    const clearCellSelection = (): void => {
      table.classList.remove('has-cell-selection');
      for (const cell of getSelectedCells()) {
        cell.classList.remove('is-selected');
      }
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
    };
    const selectAllCells = (): void => {
      for (const cell of getAllCells()) {
        cell.classList.add('is-selected');
      }
      table.classList.add('has-cell-selection');
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

    table.addEventListener('copy', (event) => {
      event.preventDefault();
      event.clipboardData?.setData('text/plain', selectedMarkdown());
    });
    table.addEventListener('mousedown', (event) => {
      event.stopPropagation();
    });
    table.addEventListener('click', (event) => {
      event.stopPropagation();
    });
    table.addEventListener('mousemove', (event) => {
      if (event.buttons !== 1 || dragStart === null) return;

      const cell = getCellFromPoint(event.clientX, event.clientY);
      if (!cell || !table.contains(cell)) return;

      event.preventDefault();
      draggingCells = true;
      selectCellRange(dragStart, getCellPosition(cell));
    });
    table.addEventListener('mouseup', () => {
      dragStart = null;
      if (draggingCells) {
        table.focus();
      }
      draggingCells = false;
    });
    table.addEventListener('keydown', handleKeydown);

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
      cell.addEventListener('mousedown', () => {
        dragStart = getCellPosition(cell);
        draggingCells = false;
      });
      cell.addEventListener('mouseenter', (event) => {
        if (event.buttons !== 1 || dragStart === null) return;

        event.preventDefault();
        draggingCells = true;
        selectCellRange(dragStart, getCellPosition(cell));
      });
      cell.addEventListener('mouseup', () => {
        dragStart = null;
        if (draggingCells) {
          table.focus();
        }
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

    return table;
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
