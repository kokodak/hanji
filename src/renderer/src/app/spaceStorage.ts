import { invoke } from '@tauri-apps/api/core';

const WEB_STORAGE_KEY = 'lithe:web-space';
const WEB_SPACE_PATH = 'browser://lithe-web-qa';
const DEFAULT_NOTE_PATH = 'default.md';
const CAPTURE_NOTE_PATH = 'Captures.md';
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

export interface CaptureOptions {
  now?: Date;
}

interface CaptureMetadataRecord {
  id: string;
  path: string;
  start_line: number;
  end_line: number;
  created_at: string;
  year: number;
  month: number;
  day: number;
  date: string;
  weekday: string;
  time: string;
}

interface CaptureMetadataStore {
  version: 1;
  records: CaptureMetadataRecord[];
}

interface WebSpaceData {
  activePath: string;
  notes: Record<string, string>;
  captureMetadata?: CaptureMetadataStore;
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

function padDatePart(value: number): string {
  return String(value).padStart(2, '0');
}

function localDateLabel(date: Date): string {
  return [date.getFullYear(), padDatePart(date.getMonth() + 1), padDatePart(date.getDate())].join('-');
}

function localTimeLabel(date: Date): string {
  return [padDatePart(date.getHours()), padDatePart(date.getMinutes())].join(':');
}

function localTimestampId(date: Date): string {
  const time = [padDatePart(date.getHours()), padDatePart(date.getMinutes()), padDatePart(date.getSeconds())].join('-');
  const milliseconds = String(date.getMilliseconds()).padStart(3, '0');

  return `${localDateLabel(date)}T${time}-${milliseconds}`;
}

function weekdayLabel(date: Date): string {
  return ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'][date.getDay()];
}

export function captureNotePath(): string {
  return CAPTURE_NOTE_PATH;
}

function normalizeCapturedText(text: string): string {
  return text.replace(/\r\n?/g, '\n').trim();
}

function formatCaptureMarkdown(text: string): string {
  if (!text.includes('\n')) {
    return `- ${text}`;
  }

  const [firstLine = '', ...restLines] = text.split('\n');
  return [`- ${firstLine}`, ...restLines.map((line) => `  ${line}`)].join('\n');
}

function lineCount(text: string): number {
  return text.length === 0 ? 0 : text.split('\n').length;
}

function captureMetadataRecord(date: Date, startLine: number, endLine: number): CaptureMetadataRecord {
  return {
    id: `${localTimestampId(date)}-line-${startLine}`,
    path: CAPTURE_NOTE_PATH,
    start_line: startLine,
    end_line: endLine,
    created_at: date.toISOString(),
    year: date.getFullYear(),
    month: date.getMonth() + 1,
    day: date.getDate(),
    date: localDateLabel(date),
    weekday: weekdayLabel(date),
    time: localTimeLabel(date)
  };
}

function appendCaptureToContent(
  content: string,
  text: string,
  date: Date
): { content: string; record: CaptureMetadataRecord } {
  const capturedText = normalizeCapturedText(text);
  if (!capturedText) {
    throw new Error('Capture text cannot be empty.');
  }

  const captureMarkdown = formatCaptureMarkdown(capturedText);
  const captureContent = content.trim().length === 0 ? '# Captures' : content.trimEnd();
  const separator = captureContent === '# Captures' ? '\n\n' : '\n';
  const prefix = `${captureContent}${separator}`;
  const startLine = lineCount(prefix);
  const endLine = startLine + lineCount(captureMarkdown) - 1;

  return {
    content: `${prefix}${captureMarkdown}\n`,
    record: captureMetadataRecord(date, startLine, endLine)
  };
}

function appendWebCaptureMetadata(data: WebSpaceData, record: CaptureMetadataRecord): void {
  data.captureMetadata = data.captureMetadata ?? { version: 1, records: [] };
  data.captureMetadata.records.push(record);
}

async function recordCaptureMetadata(record: CaptureMetadataRecord): Promise<void> {
  if (!isTauriRuntime()) return;

  await invoke('record_capture_metadata', { record });
}

export async function captureToNote(text: string, options: CaptureOptions = {}): Promise<SpaceSnapshot> {
  const now = options.now ?? new Date();
  const path = CAPTURE_NOTE_PATH;

  if (!isTauriRuntime()) {
    const data = readWebSpaceData();
    const appended = appendCaptureToContent(data.notes[path] ?? '', text, now);
    data.notes[path] = appended.content;
    data.activePath = path;
    appendWebCaptureMetadata(data, appended.record);
    writeWebSpaceData(data);
    return snapshotFromWebData(data, path);
  }

  let currentContent = '';

  try {
    currentContent = (await readNote(path)).content;
  } catch {
    currentContent = '';
  }

  const appended = appendCaptureToContent(currentContent, text, now);
  await writeNote(path, appended.content);
  await recordCaptureMetadata(appended.record);
  return await readNote(path);
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
    return snapshotFromWebData(data, path);
  }

  return await invoke<SpaceSnapshot>('read_note', { path });
}

export async function rememberActiveNote(path: string): Promise<void> {
  if (!isTauriRuntime()) {
    const data = readWebSpaceData();
    if (data.notes[path] === undefined) return;

    data.activePath = path;
    writeWebSpaceData(data);
    return;
  }
}

export async function writeNote(path: string, content: string): Promise<void> {
  if (!isTauriRuntime()) {
    const data = readWebSpaceData();
    data.notes[path] = content;
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
