import { invoke } from '@tauri-apps/api/core';

const WEB_STORAGE_KEY = 'lithe:web-space';
const WEB_SPACE_PATH = 'browser://lithe-web-qa';
const DEFAULT_NOTE_PATH = 'default.md';
const DEFAULT_NOTE_CONTENT =
  '# Welcome to Lithe\n\nStart with plain Markdown. Stay local. Add power only when you ask for it.\n\n- Fast, quiet editing\n- Portable text\n- Future plugin hooks\n';

export interface SpaceInfo {
  name: string;
  path: string;
}

export interface NoteEntry {
  name: string;
  path: string;
}

export interface SpaceSnapshot {
  space: SpaceInfo;
  notes: NoteEntry[];
  active_note: NoteEntry;
  content: string;
}

interface WebSpaceData {
  activePath: string;
  notes: Record<string, string>;
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function noteNameFromPath(path: string): string {
  return path
    .split('/')
    .pop()
    ?.replace(/\.md$/, '') || 'Untitled';
}

function safeNoteFileName(name: string): string {
  const stem = name
    .trim()
    .split('')
    .map((character) => (/[A-Za-z0-9 _-]/.test(character) ? character : '-'))
    .join('')
    .split(/\s+/)
    .filter(Boolean)
    .join('-');

  return stem || 'Untitled';
}

function readWebSpaceData(): WebSpaceData {
  const stored = window.localStorage.getItem(WEB_STORAGE_KEY);

  if (stored) {
    const parsed = JSON.parse(stored) as WebSpaceData;
    if (parsed.activePath && parsed.notes && Object.keys(parsed.notes).length > 0) {
      return parsed;
    }
  }

  return {
    activePath: DEFAULT_NOTE_PATH,
    notes: {
      [DEFAULT_NOTE_PATH]: DEFAULT_NOTE_CONTENT
    }
  };
}

function writeWebSpaceData(data: WebSpaceData): void {
  window.localStorage.setItem(WEB_STORAGE_KEY, JSON.stringify(data));
}

function snapshotFromWebData(data: WebSpaceData, activePath: string = data.activePath): SpaceSnapshot {
  const notes = Object.keys(data.notes)
    .sort()
    .map((path) => ({
      name: noteNameFromPath(path),
      path
    }));
  const resolvedActivePath = data.notes[activePath] === undefined ? notes[0]?.path ?? DEFAULT_NOTE_PATH : activePath;
  const activeNote = {
    name: noteNameFromPath(resolvedActivePath),
    path: resolvedActivePath
  };

  return {
    space: {
      name: 'Lithe Web QA',
      path: WEB_SPACE_PATH
    },
    notes,
    active_note: activeNote,
    content: data.notes[resolvedActivePath] ?? ''
  };
}

function uniqueWebNotePath(data: WebSpaceData, name: string): string {
  const stem = safeNoteFileName(name);
  let candidate = `${stem}.md`;
  let counter = 2;

  while (data.notes[candidate] !== undefined) {
    candidate = `${stem}-${counter}.md`;
    counter += 1;
  }

  return candidate;
}

export async function loadSpace(): Promise<SpaceSnapshot> {
  if (!isTauriRuntime()) {
    const data = readWebSpaceData();
    writeWebSpaceData(data);
    return snapshotFromWebData(data);
  }

  return await invoke<SpaceSnapshot>('load_space');
}

export async function openSpace(path: string): Promise<SpaceSnapshot> {
  if (!isTauriRuntime()) {
    return await loadSpace();
  }

  return await invoke<SpaceSnapshot>('open_space', { path });
}

export async function createNote(name: string): Promise<SpaceSnapshot> {
  if (!isTauriRuntime()) {
    const data = readWebSpaceData();
    const path = uniqueWebNotePath(data, name);
    data.notes[path] = '';
    data.activePath = path;
    writeWebSpaceData(data);
    return snapshotFromWebData(data, path);
  }

  return await invoke<SpaceSnapshot>('create_note', { name });
}

export async function readNote(path: string): Promise<SpaceSnapshot> {
  if (!isTauriRuntime()) {
    const data = readWebSpaceData();
    data.activePath = path;
    writeWebSpaceData(data);
    return snapshotFromWebData(data, path);
  }

  return await invoke<SpaceSnapshot>('read_note', { path });
}

export async function writeNote(path: string, content: string): Promise<void> {
  if (!isTauriRuntime()) {
    const data = readWebSpaceData();
    data.notes[path] = content;
    data.activePath = path;
    writeWebSpaceData(data);
    return;
  }

  await invoke('write_note', { path, content });
}

export async function deleteNote(path: string): Promise<SpaceSnapshot> {
  if (!isTauriRuntime()) {
    const data = readWebSpaceData();
    delete data.notes[path];

    if (Object.keys(data.notes).length === 0) {
      data.notes[DEFAULT_NOTE_PATH] = DEFAULT_NOTE_CONTENT;
    }

    data.activePath = Object.keys(data.notes).sort()[0] ?? DEFAULT_NOTE_PATH;
    writeWebSpaceData(data);
    return snapshotFromWebData(data);
  }

  return await invoke<SpaceSnapshot>('delete_note', { path });
}
