import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const livePreviewSource = readFileSync(new URL('./livePreview.ts', import.meta.url), 'utf8');

export const tests = [
  {
    name: 'does not create table previews as plugin block decorations',
    run() {
      assert.doesNotMatch(livePreviewSource, /new TableWidget\([^)]*\),\s*block:\s*true/);
    }
  }
];
