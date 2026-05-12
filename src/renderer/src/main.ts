import type { EditorView } from '@codemirror/view';
import { open } from '@tauri-apps/plugin-dialog';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { startClock } from './app/clock';
import { createAppShell } from './app/shell';
import {
  captureToNote,
  createNote,
  deleteNote,
  loadSpace,
  openSpace,
  readNote,
  rememberActiveNote,
  writeNote,
  type NoteEntry,
  type SpaceSnapshot
} from './app/spaceStorage';
import { createEditor, warmCodeLanguages } from './editor/createEditor';
import './styles.css';

const app = document.querySelector<HTMLDivElement>('#app');

if (!app) {
  throw new Error('App root was not found.');
}

function hasTauriRuntime(): boolean {
  return '__TAURI_INTERNALS__' in window;
}

function applyRuntimeStyleMode(): void {
  document.documentElement.classList.toggle('is-desktop-app', hasTauriRuntime());
  document.documentElement.classList.toggle('is-web-preview', !hasTauriRuntime());
}

applyRuntimeStyleMode();

const shell = createAppShell(app);
let saveTimer: number | undefined;
let activeNote: NoteEntry | undefined;
let editorView: EditorView | undefined;
let loadingDocument = false;
let contextMenuNote: NoteEntry | undefined;
let snapshotRequestId = 0;
let saveChain = Promise.resolve();
let savingCapture = false;
let isSidebarOpen = false;

function saveNoteContent(notePath: string, text: string): Promise<void> {
  const write = saveChain.then(() => writeNote(notePath, text));
  saveChain = write.catch(() => undefined);

  return write;
}

function scheduleSave(text: string): void {
  if (loadingDocument || !activeNote) return;

  window.clearTimeout(saveTimer);
  const notePath = activeNote.path;
  saveTimer = window.setTimeout(() => {
    saveTimer = undefined;
    void saveNoteContent(notePath, text);
  }, 160);
}

async function flushPendingSave(): Promise<void> {
  if (loadingDocument || !activeNote || !editorView) return;

  window.clearTimeout(saveTimer);
  saveTimer = undefined;
  await saveNoteContent(activeNote.path, editorView.state.doc.toString());
}

function updateCursorPosition(view: EditorView): void {
  const currentLine = view.state.doc.lineAt(view.state.selection.main.head).number;
  const totalLines = view.state.doc.lines;
  shell.cursorPosition.textContent = `Line ${currentLine} / ${totalLines}`;
}

function isInteractiveChromeTarget(target: HTMLElement): boolean {
  return Boolean(target.closest('button, input, textarea, select, [contenteditable="true"], .context-menu'));
}

function isTextEntryTarget(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && Boolean(target.closest('input, textarea, [contenteditable="true"]'));
}

function registerWindowDragging(): void {
  if (!hasTauriRuntime()) return;

  const appWindow = getCurrentWindow();

  document.addEventListener(
    'mousedown',
    (event) => {
      if (event.button !== 0) return;
      if (!(event.target instanceof HTMLElement)) return;
      if (!event.target.closest('[data-tauri-drag-region]')) return;
      if (isInteractiveChromeTarget(event.target)) return;

      event.preventDefault();
      void appWindow.startDragging().catch(() => undefined);
    },
    { capture: true }
  );
}

function blockChromeDragInteractions(): void {
  document.addEventListener('selectstart', (event) => {
    if (event.target instanceof Node && shell.editor.contains(event.target)) return;
    if (isTextEntryTarget(event.target)) return;
    event.preventDefault();
  });

  document.addEventListener('dragstart', (event) => {
    if (event.target instanceof Node && shell.editor.contains(event.target)) return;
    if (isTextEntryTarget(event.target)) return;
    event.preventDefault();
  });

  document.addEventListener(
    'pointerdown',
    (event) => {
      if (event.target instanceof Node && shell.editor.contains(event.target)) return;
      if (event.target instanceof HTMLElement && event.target.closest('[data-tauri-drag-region]')) return;
      if (event.target instanceof HTMLButtonElement) return;
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) return;

      event.preventDefault();
    },
    { capture: true }
  );
}

function resizeCaptureComposer(): void {
  shell.captureComposer.style.height = 'auto';
  shell.captureComposer.style.height = `${shell.captureComposer.scrollHeight}px`;
}

function openCaptureComposer(): void {
  shell.captureOverlay.hidden = false;
  shell.captureStatus.textContent = '';
  resizeCaptureComposer();

  window.requestAnimationFrame(() => {
    shell.captureComposer.focus();
  });
}

function closeCaptureComposer(): void {
  shell.captureOverlay.hidden = true;
  shell.captureStatus.textContent = '';
  shell.captureComposer.value = '';
  shell.captureComposer.style.height = '';
  editorView?.focus();
}

function isCaptureShortcut(event: KeyboardEvent): boolean {
  return (event.metaKey || event.ctrlKey) && event.shiftKey && event.key === ' ';
}

function isSidebarShortcut(event: KeyboardEvent): boolean {
  return (event.metaKey || event.ctrlKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === 'b';
}

function setSidebarOpen(open: boolean): void {
  isSidebarOpen = open;
  shell.shellRoot.classList.toggle('is-sidebar-open', open);
  shell.spacePanel.setAttribute('aria-hidden', String(!open));
  shell.spacePanel.inert = !open;

  if (!open) {
    hideNoteMenu();
  }
}

async function saveCapture(): Promise<void> {
  const text = shell.captureComposer.value.trim();

  if (!text || savingCapture) {
    if (!text) {
      shell.captureStatus.textContent = 'Write something first';
    }
    return;
  }

  savingCapture = true;
  shell.captureSaveButton.disabled = true;
  shell.captureStatus.textContent = 'Saving...';

  const requestId = ++snapshotRequestId;

  try {
    await flushPendingSave();
    const snapshot = await captureToNote(text);
    closeCaptureComposer();
    applyLatestSnapshot(requestId, snapshot, snapshot.content.length);
  } catch (error) {
    shell.captureStatus.textContent = 'Could not save';
    console.error(error);
  } finally {
    savingCapture = false;
    shell.captureSaveButton.disabled = false;
  }
}

function initialCursorPosition(text: string): number {
  const firstLine = text.split('\n', 1)[0] ?? '';
  const markdownPrefix = firstLine.match(
    /^(?:#{1,6}\s+|>\s?|[-+*]\s+|[-+*]\s+\[[ xX]\]\s+|\d+[.)]\s+)/
  );

  return markdownPrefix?.[0].length ?? 0;
}

function focusEditorAt(view: EditorView, position: number): void {
  window.requestAnimationFrame(() => {
    view.dispatch({
      selection: { anchor: position },
      scrollIntoView: true
    });
    view.focus();
  });
}

function setEditorText(view: EditorView, text: string, cursorPosition = initialCursorPosition(text)): void {
  loadingDocument = true;
  view.dispatch({
    changes: { from: 0, to: view.state.doc.length, insert: text },
    selection: { anchor: cursorPosition },
    scrollIntoView: true
  });
  loadingDocument = false;
  focusEditorAt(view, cursorPosition);
}

function renderNotes(notes: NoteEntry[]): void {
  shell.noteList.replaceChildren();

  for (const note of notes) {
    const button = document.createElement('button');
    button.className = [
      'note-item',
      note.path === activeNote?.path ? 'is-active' : '',
      note.path === contextMenuNote?.path ? 'is-menu-target' : ''
    ]
      .filter(Boolean)
      .join(' ');
    button.type = 'button';
    button.textContent = note.name;
    button.title = note.path;
    button.setAttribute('role', 'option');
    button.setAttribute('aria-selected', String(note.path === activeNote?.path));
    button.addEventListener('click', () => {
      hideNoteMenu();
      void selectNote(note.path);
    });
    button.addEventListener('contextmenu', (event) => {
      event.preventDefault();
      showNoteMenu(note, event.clientX, event.clientY);
    });

    shell.noteList.append(button);
  }
}

function hideNoteMenu(): void {
  contextMenuNote = undefined;
  shell.noteMenu.hidden = true;
  renderNotes(activeRenderedNotes);
}

let activeRenderedNotes: NoteEntry[] = [];

function showNoteMenu(note: NoteEntry, x: number, y: number): void {
  contextMenuNote = note;
  renderNotes(activeRenderedNotes);
  shell.noteMenu.hidden = false;
  shell.noteMenu.style.left = `${x}px`;
  shell.noteMenu.style.top = `${y}px`;
}

function applySnapshot(snapshot: SpaceSnapshot, cursorPosition?: number): void {
  hideNoteMenu();
  activeNote = snapshot.active_note;
  activeRenderedNotes = snapshot.notes;
  shell.spaceName.textContent = snapshot.space.name;
  shell.spacePath.textContent = snapshot.space.path;
  renderNotes(activeRenderedNotes);

  if (editorView) {
    setEditorText(editorView, snapshot.content, cursorPosition);
    updateCursorPosition(editorView);
  }
}

function applyLatestSnapshot(requestId: number, snapshot: SpaceSnapshot, cursorPosition?: number): void {
  if (requestId !== snapshotRequestId) return;

  void rememberActiveNote(snapshot.active_note.path);
  applySnapshot(snapshot, cursorPosition);
}

async function selectNote(notePath: string): Promise<void> {
  if (notePath === activeNote?.path) return;

  const requestId = ++snapshotRequestId;
  await flushPendingSave();
  applyLatestSnapshot(requestId, await readNote(notePath));
}

async function startApp(): Promise<void> {
  const snapshot = await loadSpace();

  setSidebarOpen(false);
  activeNote = snapshot.active_note;
  activeRenderedNotes = snapshot.notes;
  shell.spaceName.textContent = snapshot.space.name;
  shell.spacePath.textContent = snapshot.space.path;
  renderNotes(activeRenderedNotes);

  editorView = createEditor({
    parent: shell.editor,
    initialText: snapshot.content,
    initialSelection: initialCursorPosition(snapshot.content),
    onChange: scheduleSave,
    onCursorChange: updateCursorPosition
  });

  shell.newNoteButton.addEventListener('click', () => {
    const noteName = shell.newNoteName.value.trim() || 'Untitled';
    shell.newNoteName.value = '';
    const requestId = ++snapshotRequestId;
    void flushPendingSave().then(() => createNote(noteName).then((snapshot) => applyLatestSnapshot(requestId, snapshot)));
  });

  shell.newNoteName.addEventListener('keydown', (event) => {
    if (event.key === 'Enter') {
      event.preventDefault();
      shell.newNoteButton.click();
    }
  });

  shell.captureComposer.addEventListener('input', resizeCaptureComposer);

  shell.captureComposer.addEventListener('keydown', (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
      event.preventDefault();
      void saveCapture();
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      closeCaptureComposer();
    }
  });

  shell.captureSaveButton.addEventListener('click', () => {
    void saveCapture();
  });

  shell.captureOverlay.addEventListener('mousedown', (event) => {
    if (event.target === shell.captureOverlay) {
      closeCaptureComposer();
    }
  });

  shell.deleteNoteButton.addEventListener('click', () => {
    if (!contextMenuNote) return;

    const notePath = contextMenuNote.path;
    hideNoteMenu();
    const requestId = ++snapshotRequestId;
    void flushPendingSave().then(() => deleteNote(notePath).then((snapshot) => applyLatestSnapshot(requestId, snapshot)));
  });

  window.addEventListener('click', (event) => {
    if (event.target instanceof Node && shell.noteMenu.contains(event.target)) return;
    hideNoteMenu();
  });

  window.addEventListener('keydown', (event) => {
    if (isCaptureShortcut(event)) {
      event.preventDefault();
      openCaptureComposer();
      return;
    }

    if (isSidebarShortcut(event)) {
      event.preventDefault();
      setSidebarOpen(!isSidebarOpen);
      return;
    }

    if (event.key === 'Escape') {
      if (!shell.captureOverlay.hidden) {
        closeCaptureComposer();
        return;
      }

      hideNoteMenu();
    }
  });

  shell.openSpaceButton.addEventListener('click', () => {
    void (async () => {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: 'Open Space'
      });

      if (typeof selectedPath !== 'string') return;

      const requestId = ++snapshotRequestId;
      await flushPendingSave();
      await openSpace(selectedPath).then((snapshot) => applyLatestSnapshot(requestId, snapshot));
    })();
  });

  startClock(shell.currentTime);
  registerWindowDragging();
  blockChromeDragInteractions();
  warmCodeLanguages();
  updateCursorPosition(editorView);
  focusEditorAt(editorView, initialCursorPosition(snapshot.content));
}

void startApp();
