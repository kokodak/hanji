import assert from 'node:assert/strict';
import { Text } from '@codemirror/state';
import {
  collectMarkdownTables,
  insertMarkdownTableColumn,
  insertMarkdownTableRow,
  moveMarkdownTableColumn,
  moveMarkdownTableRow,
  serializeMarkdownTable,
  splitTableCells
} from './table';

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
  },
  {
    name: 'inserts Markdown table columns at the requested index',
    run() {
      assert.deepEqual(insertMarkdownTableColumn({ headers: ['Name', 'Status'], rows: [['Lithe', 'Ready']] }, 1), {
        headers: ['Name', '', 'Status'],
        rows: [['Lithe', '', 'Ready']]
      });
    }
  },
  {
    name: 'inserts Markdown table rows with the current column count',
    run() {
      assert.deepEqual(insertMarkdownTableRow({ headers: ['Name', 'Status'], rows: [['Lithe', 'Ready']] }, 1), {
        headers: ['Name', 'Status'],
        rows: [
          ['Lithe', 'Ready'],
          ['', '']
        ]
      });
    }
  },
  {
    name: 'moves Markdown table columns across headers and rows',
    run() {
      assert.deepEqual(moveMarkdownTableColumn({ headers: ['Name', 'Status', 'Owner'], rows: [['Lithe', 'Ready', 'QA']] }, 2, 0), {
        headers: ['Owner', 'Name', 'Status'],
        rows: [['QA', 'Lithe', 'Ready']]
      });
    }
  },
  {
    name: 'moves Markdown table body rows',
    run() {
      assert.deepEqual(
        moveMarkdownTableRow(
          {
            headers: ['Name', 'Status'],
            rows: [
              ['Lithe', 'Ready'],
              ['Docs', 'Draft']
            ]
          },
          1,
          0
        ),
        {
          headers: ['Name', 'Status'],
          rows: [
            ['Docs', 'Draft'],
            ['Lithe', 'Ready']
          ]
        }
      );
    }
  }
];
