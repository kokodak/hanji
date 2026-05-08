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
      assert.match(createEditorSource, /Prec\.highest\(keymap\.of\(\[\{ key: 'Enter', run: continueListItem \}\]\)\)/);
      assert.match(createEditorSource, /drawSelection\(\),/);
      assert.match(contentRule, /caret-color:\s*transparent;/);
      assert.match(cursorRule, /border-left-color:\s*#2f5f62;/);
    }
  },
  {
    name: 'preserves hidden Markdown syntax layout space',
    run() {
      const syntaxRule = getRuleBody('#editor .cm-markdown-syntax-hidden');

      assert.doesNotMatch(syntaxRule, /display:\s*none;/);
      assert.match(syntaxRule, /visibility:\s*hidden;/);
    }
  },
  {
    name: 'keeps list marker widgets inside the editor line box',
    run() {
      const bulletRule = getRuleBody('#editor .cm-live-bullet');
      const checkboxRule = getRuleBody('#editor .cm-live-checkbox');
      const checkboxBoxRule = getRuleBody('#editor .cm-live-checkbox-box');
      const numberedRule = getRuleBody('#editor .cm-live-numbered-marker');

      assert.match(bulletRule, /line-height:\s*inherit;/);
      assert.match(bulletRule, /vertical-align:\s*baseline;/);
      assert.match(checkboxRule, /height:\s*1em;/);
      assert.match(checkboxRule, /vertical-align:\s*-0\.1em;/);
      assert.match(checkboxBoxRule, /height:\s*1em;/);
      assert.match(checkboxBoxRule, /transform:\s*translateY\(0\.08em\);/);
      assert.match(numberedRule, /line-height:\s*inherit;/);
      assert.match(numberedRule, /vertical-align:\s*baseline;/);
    }
  }
];
