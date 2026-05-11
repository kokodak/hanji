import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { markdown } from '@codemirror/lang-markdown';
import { languages } from '@codemirror/language-data';
import { syntaxHighlighting } from '@codemirror/language';
import { EditorState, Prec } from '@codemirror/state';
import { drawSelection, EditorView, keymap } from '@codemirror/view';
import { litheHighlightStyle } from './highlighting';
import { handleBacktickInput, handlePairedSymbolInput, handlePlainTextPaste } from './inputHandlers';
import { continueListItem, stableVerticalMovement, tabIndentation } from './keymaps';
import { liveMarkdownPreview } from '../markdown/livePreview';

interface CreateEditorOptions {
  parent: HTMLElement;
  initialText: string;
  initialSelection?: number;
  onChange: (text: string) => void;
  onCursorChange: (view: EditorView) => void;
}

export function createEditor(options: CreateEditorOptions): EditorView {
  return new EditorView({
    parent: options.parent,
    state: EditorState.create({
      doc: options.initialText,
      selection: { anchor: options.initialSelection ?? 0 },
      extensions: [
        history(),
        markdown({ codeLanguages: languages }),
        syntaxHighlighting(litheHighlightStyle, { fallback: true }),
        drawSelection(),
        handlePlainTextPaste,
        handleBacktickInput,
        handlePairedSymbolInput,
        liveMarkdownPreview,
        EditorView.lineWrapping,
        stableVerticalMovement,
        Prec.highest(tabIndentation),
        Prec.highest(keymap.of([{ key: 'Enter', run: continueListItem }])),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            options.onChange(update.view.state.doc.toString());
          }

          if (update.docChanged || update.selectionSet) {
            options.onCursorChange(update.view);
          }
        })
      ]
    })
  });
}

export function warmCodeLanguages(): void {
  const loadLanguages = (): void => {
    void Promise.allSettled(languages.map((language) => language.load()));
  };

  if (typeof window.requestIdleCallback === 'function') {
    window.requestIdleCallback(loadLanguages, { timeout: 2000 });
    return;
  }

  globalThis.setTimeout(loadLanguages, 400);
}
