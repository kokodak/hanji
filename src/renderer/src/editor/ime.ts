import { EditorView, ViewPlugin } from '@codemirror/view';

const RECENT_COMPOSITION_WINDOW_MS = 120;
const hangulTextPattern = /[\u1100-\u11ff\u3130-\u318f\uac00-\ud7a3]/;

class ImeCompositionState {
  composing = false;
  endedAt = 0;
}

export const imeCompositionTracker = ViewPlugin.fromClass(ImeCompositionState, {
  eventObservers: {
    compositionstart() {
      this.composing = true;
      this.endedAt = 0;
    },
    compositionend() {
      this.composing = false;
      this.endedAt = Date.now();
    }
  }
});

function trackedImeState(view: EditorView): ImeCompositionState | null {
  const plugin = (view as { plugin?: EditorView['plugin'] }).plugin;
  return typeof plugin === 'function' ? (plugin.call(view, imeCompositionTracker) as ImeCompositionState | null) : null;
}

export function textContainsHangul(text: string): boolean {
  return hangulTextPattern.test(text);
}

export function editorHasActiveImeComposition(view: EditorView): boolean {
  const tracker = trackedImeState(view);
  return (view as { compositionStarted?: boolean }).compositionStarted === true || tracker?.composing === true;
}

export function editorHasActiveOrRecentImeComposition(view: EditorView): boolean {
  const tracker = trackedImeState(view);
  if ((view as { compositionStarted?: boolean }).compositionStarted === true || tracker?.composing === true) return true;

  return tracker !== null && tracker.endedAt > 0 && Date.now() - tracker.endedAt < RECENT_COMPOSITION_WINDOW_MS;
}

export function imeCompositionSelectionCursor(view: EditorView): number | null {
  if (!editorHasActiveOrRecentImeComposition(view)) return null;

  const selection = view.state.selection.main;
  if (selection.empty) return null;

  const from = Math.min(selection.from, selection.to);
  const to = Math.max(selection.from, selection.to);
  const selectedText = view.state.sliceDoc(from, to);

  if (selectedText.includes('\n') || !textContainsHangul(selectedText)) return null;

  return to;
}
