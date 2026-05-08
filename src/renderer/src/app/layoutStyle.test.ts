import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const styles = readFileSync(new URL('../styles.css', import.meta.url), 'utf8');

function getRuleBody(selector: string): string {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = new RegExp(`${escapedSelector}\\s*\\{([^}]*)\\}`).exec(styles);

  assert.ok(match, `Expected ${selector} rule to exist.`);

  return match[1];
}

export const tests = [
  {
    name: 'keeps the app chrome fixed inside the viewport',
    run() {
      assert.match(getRuleBody('body'), /overflow:\s*hidden;/);
      assert.match(getRuleBody('.shell'), /height:\s*100vh;/);
      assert.match(getRuleBody('.shell'), /overflow:\s*hidden;/);
      assert.match(getRuleBody('.space-panel'), /height:\s*100vh;/);
      assert.match(getRuleBody('.space-panel'), /overflow:\s*hidden;/);
    }
  },
  {
    name: 'contains scroll momentum to scrollable content surfaces',
    run() {
      assert.match(getRuleBody('.workspace'), /overflow:\s*hidden;/);
      assert.match(getRuleBody('.editor-layout'), /overflow:\s*hidden;/);
      assert.match(getRuleBody('.editor-layout'), /height:\s*100%;/);
      assert.match(getRuleBody('#editor'), /overflow:\s*hidden;/);
      assert.match(getRuleBody('#editor .cm-editor'), /min-height:\s*0;/);
      assert.match(getRuleBody('.note-list'), /overscroll-behavior:\s*contain;/);
      assert.match(getRuleBody('#editor .cm-scroller'), /height:\s*100%;/);
      assert.match(getRuleBody('#editor .cm-scroller'), /max-height:\s*100%;/);
      assert.match(getRuleBody('#editor .cm-scroller'), /overflow:\s*auto;/);
      assert.match(getRuleBody('#editor .cm-scroller'), /overscroll-behavior:\s*contain;/);
    }
  }
];
