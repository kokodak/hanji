import assert from 'node:assert/strict';
import { Text } from '@codemirror/state';
import { collectMarkdownTables, serializeMarkdownTable, splitTableCells } from './table';

export const tests = [
  {
    name: 'splits compact Markdown table cells',
    run() {
      assert.deepEqual(splitTableCells('| Name | Status |'), ['Name', 'Status']);
    }
  },
  {
    name: 'collects GitHub-Flavored Markdown tables',
    run() {
      const tables = collectMarkdownTables(Text.of(['| Name | Status |', '| --- | --- |', '| QA | Open |', '', 'after']));

      assert.deepEqual(tables, [
        {
          startLine: 1,
          endLine: 3,
          headers: ['Name', 'Status'],
          rows: [['QA', 'Open']]
        }
      ]);
    }
  },
  {
    name: 'rejects pipe text without a table delimiter row',
    run() {
      const tables = collectMarkdownTables(Text.of(['Name | Status', 'QA | Open']));

      assert.deepEqual(tables, []);
    }
  },
  {
    name: 'serializes edited table cells as Markdown',
    run() {
      assert.equal(
        serializeMarkdownTable(['Name', 'Status'], [['Lithe', 'Ready']]),
        '| Name | Status |\n| --- | --- |\n| Lithe | Ready |'
      );
    }
  },
  {
    name: 'escapes Markdown table pipes while serializing cells',
    run() {
      assert.equal(serializeMarkdownTable(['Name'], [['A | B']]), '| Name |\n| --- |\n| A \\| B |');
    }
  }
];
