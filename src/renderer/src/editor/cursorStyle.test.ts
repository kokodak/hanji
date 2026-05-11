import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const styles = readFileSync(new URL('../styles.css', import.meta.url), 'utf8');
const createEditorSource = readFileSync(new URL('./createEditor.ts', import.meta.url), 'utf8');
const highlightingSource = readFileSync(new URL('./highlighting.ts', import.meta.url), 'utf8');

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
      assert.match(createEditorSource, /const tabIndentationEvents = EditorView\.domEventHandlers/);
      assert.match(createEditorSource, /event\.key !== 'Tab'/);
      assert.match(createEditorSource, /event\.target instanceof HTMLElement && event\.target\.closest\('\.cm-live-table'\)/);
      assert.match(createEditorSource, /event\.shiftKey \? outdentSpaces\(view\) : indentWithSpaces\(view\)/);
      assert.match(createEditorSource, /Prec\.highest\(tabIndentationEvents\),/);
      assert.match(createEditorSource, /Prec\.highest\(softBreakKeymap\),/);
      assert.match(createEditorSource, /Prec\.highest\(keymap\.of\(\[\{ key: 'Enter', run: continueListItem \}\]\)\)/);
      assert.match(createEditorSource, /Prec\.highest\(tabIndentation\),/);
      assert.match(createEditorSource, /drawSelection\(\),/);
      assert.match(contentRule, /caret-color:\s*transparent;/);
      assert.match(cursorRule, /border-left-color:\s*#2f5f62;/);
    }
  },
  {
    name: 'keeps visual editor margins outside the editable selection surface',
    run() {
      const scrollerRule = getRuleBody('#editor .cm-scroller');
      const contentRule = getRuleBody('#editor .cm-content');
      const lineRule = getRuleBody('#editor .cm-line');

      assert.match(scrollerRule, /padding:\s*34px 38px;/);
      assert.match(contentRule, /padding:\s*0;/);
      assert.match(contentRule, /min-height:\s*calc\(100% - 68px\);/);
      assert.match(lineRule, /width:\s*fit-content;/);
      assert.match(lineRule, /max-width:\s*100%;/);
      assert.match(lineRule, /min-width:\s*1ch;/);
    }
  },
  {
    name: 'uses compact text selection paint',
    run() {
      const selectionLayerRule = getRuleBody('#editor .cm-selectionBackground,\n#editor .cm-focused .cm-selectionBackground');
      const nativeSelectionRule = getRuleBody('#editor .cm-content ::selection');
      const compactSelectionRule = getRuleBody('#editor .cm-compact-selection');
      const emptySelectionRule = getRuleBody('#editor .cm-compact-empty-selection');

      assert.match(selectionLayerRule, /background:\s*transparent;/);
      assert.match(nativeSelectionRule, /background:\s*transparent;/);
      assert.match(compactSelectionRule, /background:\s*#c9dcda;/);
      assert.match(emptySelectionRule, /width:\s*1ch;/);
      assert.match(emptySelectionRule, /height:\s*1lh;/);
      assert.match(emptySelectionRule, /background:\s*#c9dcda;/);
    }
  },
  {
    name: 'keeps hard line breaks visually distinct from soft wraps',
    run() {
      const lineRule = getRuleBody('#editor .cm-line');
      const softBreakLineRule = getRuleBody('#editor .cm-line.cm-soft-break-line');
      const codeblockLineRule = getRuleBody('#editor .cm-line.cm-live-codeblock');
      const hiddenTableLineRule = getRuleBody('#editor .cm-line.cm-live-table-source-hidden');

      assert.match(lineRule, /padding:\s*0 0 0\.48em;/);
      assert.doesNotMatch(lineRule, /margin-bottom:/);
      assert.match(softBreakLineRule, /padding-bottom:\s*0\.06em;/);
      assert.match(codeblockLineRule, /padding-top:\s*0\.1em;/);
      assert.match(codeblockLineRule, /padding-bottom:\s*0\.1em;/);
      assert.match(hiddenTableLineRule, /padding:\s*0;/);
    }
  },
  {
    name: 'does not use parser heading weight for setext-like lines',
    run() {
      assert.doesNotMatch(highlightingSource, /tag:\s*t\.heading,\s*fontWeight/);
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
    name: 'does not let list marker preview widgets start browser drag operations',
    run() {
      const bulletRule = getRuleBody('#editor .cm-live-bullet');
      const numberedRule = getRuleBody('#editor .cm-live-numbered-marker');

      assert.match(bulletRule, /user-select:\s*none;/);
      assert.match(bulletRule, /-webkit-user-drag:\s*none;/);
      assert.match(numberedRule, /user-select:\s*none;/);
      assert.match(numberedRule, /-webkit-user-drag:\s*none;/);
    }
  },
  {
    name: 'keeps list marker widgets inside the editor line box',
    run() {
      const listLineRule = getRuleBody('#editor .cm-live-list-line');
      const bulletRule = getRuleBody('#editor .cm-live-bullet');
      const bulletDotRule = getRuleBody('#editor .cm-live-bullet-dot');
      const checkboxRule = getRuleBody('#editor .cm-live-checkbox');
      const checkboxBoxRule = getRuleBody('#editor .cm-live-checkbox-box');
      const numberedRule = getRuleBody('#editor .cm-live-numbered-marker');
      const selectedCodeRule = getRuleBody(
        '#editor .cm-compact-selection .cm-live-code,\n#editor .cm-live-code .cm-compact-selection,\n#editor .cm-live-code.cm-compact-selection'
      );

      assert.match(listLineRule, /padding-left:\s*var\(--list-wrap-indent, 0\);/);
      assert.match(listLineRule, /text-indent:\s*calc\(var\(--list-wrap-indent, 0\) \* -1\);/);
      assert.match(bulletRule, /display:\s*inline-flex;/);
      assert.match(bulletRule, /width:\s*1\.45em;/);
      assert.match(bulletRule, /justify-content:\s*flex-start;/);
      assert.match(bulletRule, /height:\s*1lh;/);
      assert.match(bulletRule, /line-height:\s*inherit;/);
      assert.match(bulletRule, /text-indent:\s*0;/);
      assert.match(bulletRule, /vertical-align:\s*top;/);
      assert.match(bulletDotRule, /width:\s*1em;/);
      assert.match(bulletDotRule, /text-align:\s*center;/);
      assert.match(checkboxRule, /width:\s*1\.45em;/);
      assert.match(checkboxRule, /height:\s*1lh;/);
      assert.match(checkboxRule, /line-height:\s*inherit;/);
      assert.match(checkboxRule, /text-indent:\s*0;/);
      assert.match(checkboxRule, /vertical-align:\s*top;/);
      assert.doesNotMatch(checkboxRule, /cursor:\s*pointer;/);
      assert.match(checkboxBoxRule, /height:\s*1em;/);
      assert.match(checkboxBoxRule, /cursor:\s*pointer;/);
      assert.doesNotMatch(checkboxBoxRule, /transform:/);
      assert.match(numberedRule, /min-width:\s*2\.1ch;/);
      assert.match(numberedRule, /height:\s*1lh;/);
      assert.match(numberedRule, /line-height:\s*inherit;/);
      assert.match(numberedRule, /text-indent:\s*0;/);
      assert.match(numberedRule, /vertical-align:\s*top;/);
      assert.match(selectedCodeRule, /background:\s*#c9dcda;/);
    }
  }
];
