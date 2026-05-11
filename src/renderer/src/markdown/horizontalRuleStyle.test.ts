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
    name: 'keeps horizontal rule preview inside the source line box',
    run() {
      const rule = getRuleBody('#editor .cm-live-horizontal-rule');
      const lineRule = getRuleBody('#editor .cm-line:has(.cm-live-horizontal-rule)');

      assert.match(rule, /display:\s*inline-block;/);
      assert.match(rule, /width:\s*100%;/);
      assert.match(rule, /vertical-align:\s*middle;/);
      assert.doesNotMatch(rule, /margin:/);
      assert.match(lineRule, /width:\s*100%;/);
    }
  }
];
