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

  private createCell(tagName: 'td' | 'th', text: string, view: EditorView, table: HTMLTableElement): HTMLTableCellElement {
    const cell = document.createElement(tagName);
    cell.textContent = text;
    cell.contentEditable = 'plaintext-only';
    cell.spellcheck = false;
    cell.addEventListener('blur', () => {
      this.updateDocument(view, table);
    });
    cell.addEventListener('keydown', (event) => {
      if (event.key === 'Enter') {
        event.preventDefault();
        cell.blur();
        view.focus();
      }
    });
    return cell;
  }

  toDOM(view: EditorView): HTMLElement {
    const table = document.createElement('table');
    table.className = 'cm-live-table';
    table.dataset.markdown = this.markdownSource();
    table.addEventListener('copy', (event) => {
      event.preventDefault();
      event.clipboardData?.setData('text/plain', this.markdownFromDOM(table));
    });
    table.addEventListener('mousedown', (event) => {
      event.stopPropagation();
    });
    table.addEventListener('click', (event) => {
      event.stopPropagation();
    });

    const thead = document.createElement('thead');
    const headerRow = document.createElement('tr');
    for (const header of this.table.headers) {
      const cell = this.createCell('th', header, view, table);
      headerRow.append(cell);
    }
    thead.append(headerRow);
    table.append(thead);

    const tbody = document.createElement('tbody');
    for (const row of this.table.rows) {
      const tableRow = document.createElement('tr');
      for (let index = 0; index < this.table.headers.length; index += 1) {
        const cell = this.createCell('td', row[index] ?? '', view, table);
        tableRow.append(cell);
      }
      tbody.append(tableRow);
    }
    table.append(tbody);

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
