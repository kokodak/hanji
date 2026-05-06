import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const styles = readFileSync(new URL('../styles.css', import.meta.url), 'utf8');
const createEditorSource = readFileSync(new URL('./createEditor.ts', import.meta.url), 'utf8');

function getRuleBody(selector: string): string {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = new RegExp(`${escapedSelector}\\s*\\{([^}]*)\\}`).exec(styles);

  assert.ok(match, `Expected ${selector} rule to exist.`);

  return match[1];
}

export const tests = [
  {
    name: 'uses the CodeMirror cursor instead of the native browser caret',
    run() {
      const contentRule = getRuleBody('#editor .cm-content');
      const cursorRule = getRuleBody('#editor .cm-cursor');

      assert.match(createEditorSource, /import \{ drawSelection, EditorView, keymap \} from '@codemirror\/view';/);
      assert.match(createEditorSource, /drawSelection\(\),/);
      assert.match(contentRule, /caret-color:\s*transparent;/);
      assert.match(cursorRule, /border-left-color:\s*#2f5f62;/);
    }
  }
];
