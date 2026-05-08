import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const livePreviewSource = readFileSync(new URL('./livePreview.ts', import.meta.url), 'utf8');
const widgetsSource = readFileSync(new URL('./widgets.ts', import.meta.url), 'utf8');

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
      assert.match(widgetsSource, /event\.clipboardData\?\.setData\('text\/plain', this\.markdownFromDOM\(table\)\);/);
      assert.match(widgetsSource, /view\.dispatch\(\{/);
      assert.match(widgetsSource, /insert: markdown/);
    }
  }
];
