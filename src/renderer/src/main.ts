import type { EditorView } from '@codemirror/view';
import { open } from '@tauri-apps/plugin-dialog';
import { startClock } from './app/clock';
import { createAppShell } from './app/shell';
import {
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

const shell = createAppShell(app);
let saveTimer: number | undefined;
let activeNote: NoteEntry | undefined;
let editorView: EditorView | undefined;
let loadingDocument = false;
let contextMenuNote: NoteEntry | undefined;
let snapshotRequestId = 0;
let saveChain = Promise.resolve();

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

function blockChromeDragInteractions(): void {
  document.addEventListener('selectstart', (event) => {
    if (event.target instanceof Node && shell.editor.contains(event.target)) return;
    event.preventDefault();
  });

  document.addEventListener('dragstart', (event) => {
    if (event.target instanceof Node && shell.editor.contains(event.target)) return;
    event.preventDefault();
  });

  document.addEventListener(
    'pointerdown',
    (event) => {
      if (event.target instanceof Node && shell.editor.contains(event.target)) return;
      if (event.target instanceof HTMLButtonElement) return;
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) return;

      event.preventDefault();
    },
    { capture: true }
  );
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

function setEditorText(view: EditorView, text: string): void {
  const cursorPosition = initialCursorPosition(text);

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

function applySnapshot(snapshot: SpaceSnapshot): void {
  hideNoteMenu();
  activeNote = snapshot.active_note;
  activeRenderedNotes = snapshot.notes;
  shell.spaceName.textContent = snapshot.space.name;
  shell.spacePath.textContent = snapshot.space.path;
  renderNotes(activeRenderedNotes);

  if (editorView) {
    setEditorText(editorView, snapshot.content);
    updateCursorPosition(editorView);
  }
}

function applyLatestSnapshot(requestId: number, snapshot: SpaceSnapshot): void {
  if (requestId !== snapshotRequestId) return;

  void rememberActiveNote(snapshot.active_note.path);
  applySnapshot(snapshot);
}

async function selectNote(notePath: string): Promise<void> {
  if (notePath === activeNote?.path) return;

  const requestId = ++snapshotRequestId;
  await flushPendingSave();
  applyLatestSnapshot(requestId, await readNote(notePath));
}

async function startApp(): Promise<void> {
  const snapshot = await loadSpace();

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
    if (event.key === 'Escape') {
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
  blockChromeDragInteractions();
  warmCodeLanguages();
  updateCursorPosition(editorView);
  focusEditorAt(editorView, initialCursorPosition(snapshot.content));
}

void startApp();
