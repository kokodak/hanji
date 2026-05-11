import type { Text } from '@codemirror/state';

export interface MarkdownTable {
  startLine: number;
  endLine: number;
  headers: string[];
  rows: string[][];
}

export interface MarkdownTableContent {
  headers: string[];
  rows: string[][];
}

export function splitTableCells(lineText: string): string[] {
  const trimmed = lineText.trim().replace(/^\|/, '').replace(/\|$/, '');
  return trimmed.split('|').map((cell) => cell.trim());
}

function lineIsTableDelimiter(lineText: string): boolean {
  const cells = splitTableCells(lineText);
  return cells.length > 1 && cells.every((cell) => /^:?-{3,}:?$/.test(cell));
}

function lineLooksLikeTableRow(lineText: string): boolean {
  return lineText.includes('|') && splitTableCells(lineText).length > 1;
}

export function collectMarkdownTables(doc: Text): MarkdownTable[] {
  const tables: MarkdownTable[] = [];
  let lineNumber = 1;

  while (lineNumber < doc.lines) {
    const headerLine = doc.line(lineNumber);
    const delimiterLine = doc.line(lineNumber + 1);

    if (!lineLooksLikeTableRow(headerLine.text) || !lineIsTableDelimiter(delimiterLine.text)) {
      lineNumber += 1;
      continue;
    }

    const headers = splitTableCells(headerLine.text);
    const rows: string[][] = [];
    let endLine = delimiterLine.number;

    for (let rowLineNumber = delimiterLine.number + 1; rowLineNumber <= doc.lines; rowLineNumber += 1) {
      const rowLine = doc.line(rowLineNumber);
      if (!lineLooksLikeTableRow(rowLine.text)) break;

      rows.push(splitTableCells(rowLine.text));
      endLine = rowLineNumber;
    }

    tables.push({
      startLine: headerLine.number,
      endLine,
      headers,
      rows
    });
    lineNumber = endLine + 1;
  }

  return tables;
}

export function getMarkdownTableForLine(tables: MarkdownTable[], lineNumber: number): MarkdownTable | null {
  return tables.find((table) => lineNumber >= table.startLine && lineNumber <= table.endLine) ?? null;
}

function escapeTableCell(cell: string): string {
  return cell.replace(/\r?\n/g, ' ').replace(/\|/g, '\\|').trim();
}

function serializeTableRow(cells: string[], columnCount: number): string {
  const paddedCells = Array.from({ length: columnCount }, (_, index) => escapeTableCell(cells[index] ?? ''));
  return `| ${paddedCells.join(' | ')} |`;
}

export function serializeMarkdownTable(headers: string[], rows: string[][]): string {
  const columnCount = Math.max(headers.length, ...rows.map((row) => row.length), 1);
  const normalizedHeaders = headers.length > 0 ? headers : Array.from({ length: columnCount }, () => '');
  const delimiter = Array.from({ length: columnCount }, () => '---');

  return [serializeTableRow(normalizedHeaders, columnCount), serializeTableRow(delimiter, columnCount), ...rows.map((row) => serializeTableRow(row, columnCount))].join(
    '\n'
  );
}

export function getMarkdownTableColumnCount(table: MarkdownTableContent): number {
  return Math.max(table.headers.length, ...table.rows.map((row) => row.length), 1);
}

function normalizedTableRows(rows: string[][], columnCount: number): string[][] {
  return rows.map((row) => Array.from({ length: columnCount }, (_, index) => row[index] ?? ''));
}

function clampedInsertIndex(index: number, length: number): number {
  return Math.min(Math.max(index, 0), length);
}

function clampedMoveIndex(index: number, length: number): number {
  return Math.min(Math.max(index, 0), Math.max(0, length - 1));
}

function moveArrayItem<T>(items: T[], from: number, to: number): T[] {
  if (items.length === 0) return items;

  const next = [...items];
  const [item] = next.splice(clampedMoveIndex(from, next.length), 1);
  next.splice(clampedMoveIndex(to, next.length + 1), 0, item);
  return next;
}

export function insertMarkdownTableColumn(table: MarkdownTableContent, index: number): MarkdownTableContent {
  const columnCount = getMarkdownTableColumnCount(table);
  const insertAt = clampedInsertIndex(index, columnCount);
  const headers = Array.from({ length: columnCount }, (_, column) => table.headers[column] ?? '');
  const rows = normalizedTableRows(table.rows, columnCount);

  headers.splice(insertAt, 0, '');
  for (const row of rows) {
    row.splice(insertAt, 0, '');
  }

  return { headers, rows };
}

export function insertMarkdownTableRow(table: MarkdownTableContent, index: number): MarkdownTableContent {
  const columnCount = getMarkdownTableColumnCount(table);
  const rows = normalizedTableRows(table.rows, columnCount);
  rows.splice(clampedInsertIndex(index, rows.length), 0, Array.from({ length: columnCount }, () => ''));

  return {
    headers: Array.from({ length: columnCount }, (_, column) => table.headers[column] ?? ''),
    rows
  };
}

export function moveMarkdownTableColumn(table: MarkdownTableContent, from: number, to: number): MarkdownTableContent {
  const columnCount = getMarkdownTableColumnCount(table);
  const headers = Array.from({ length: columnCount }, (_, column) => table.headers[column] ?? '');
  const rows = normalizedTableRows(table.rows, columnCount);

  return {
    headers: moveArrayItem(headers, from, to),
    rows: rows.map((row) => moveArrayItem(row, from, to))
  };
}

export function moveMarkdownTableRow(table: MarkdownTableContent, from: number, to: number): MarkdownTableContent {
  const columnCount = getMarkdownTableColumnCount(table);

  return {
    headers: Array.from({ length: columnCount }, (_, column) => table.headers[column] ?? ''),
    rows: moveArrayItem(normalizedTableRows(table.rows, columnCount), from, to)
  };
}
