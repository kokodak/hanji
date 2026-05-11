import assert from 'node:assert/strict';
import { captureNotePath, captureToNote, createNote, deleteNote, loadSpace, readNote, rememberActiveNote, writeNote } from './spaceStorage';

class LocalStorageStub {
  private values = new Map<string, string>();

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }

  clear(): void {
    this.values.clear();
  }
}

const localStorageStub = new LocalStorageStub();

Object.defineProperty(globalThis, 'window', {
  value: { localStorage: localStorageStub },
  configurable: true
});

function resetWebSpace(): void {
  localStorageStub.clear();
}

export const tests = [
  {
    name: 'uses one markdown note for captures',
    run() {
      assert.equal(captureNotePath(), 'Captures.md');
    }
  },
  {
    name: 'captures single-line thoughts in the shared capture note',
    async run() {
      resetWebSpace();

      const snapshot = await captureToNote('Capture first. Organize later.', {
        now: new Date(2026, 4, 11, 19, 42)
      });

      assert.equal(snapshot.active_note.path, 'Captures.md');
      assert.equal(snapshot.content, '# Captures\n\n- Capture first. Organize later.\n');
      assert.equal((await readNote('Captures.md')).content, snapshot.content);
    }
  },
  {
    name: 'appends later captures to the shared capture note',
    async run() {
      resetWebSpace();

      await captureToNote('First thought', { now: new Date(2026, 4, 11, 9, 0) });
      const snapshot = await captureToNote('Second thought', { now: new Date(2026, 4, 11, 9, 5) });

      assert.equal(snapshot.content, '# Captures\n\n- First thought\n- Second thought\n');
    }
  },
  {
    name: 'captures multiline thoughts as one markdown list item',
    async run() {
      resetWebSpace();

      const snapshot = await captureToNote('A longer thought\nwith a second line', {
        now: new Date(2026, 4, 11, 20, 7)
      });

      assert.equal(snapshot.content, '# Captures\n\n- A longer thought\n  with a second line\n');
    }
  },
  {
    name: 'stores capture time metadata outside markdown content',
    async run() {
      resetWebSpace();

      await captureToNote('A timestamped thought', { now: new Date(2026, 4, 11, 20, 7) });

      const stored = JSON.parse(localStorageStub.getItem('lithe:web-space') ?? '{}') as {
        captureMetadata?: {
          records: Array<{
            path: string;
            start_line: number;
            end_line: number;
            year: number;
            month: number;
            day: number;
            date: string;
            weekday: string;
            time: string;
          }>;
        };
      };
      const [record] = stored.captureMetadata?.records ?? [];

      assert.ok(record);
      assert.equal(record.path, 'Captures.md');
      assert.equal(record.start_line, 3);
      assert.equal(record.end_line, 3);
      assert.equal(record.year, 2026);
      assert.equal(record.month, 5);
      assert.equal(record.day, 11);
      assert.equal(record.date, '2026-05-11');
      assert.equal(record.weekday, 'Monday');
      assert.equal(record.time, '20:07');
    }
  },
  {
    name: 'tracks consecutive capture line ranges',
    async run() {
      resetWebSpace();

      await captureToNote('First thought', { now: new Date(2026, 4, 11, 9, 0) });
      await captureToNote('Second thought\nwith detail', { now: new Date(2026, 4, 11, 9, 5) });

      const stored = JSON.parse(localStorageStub.getItem('lithe:web-space') ?? '{}') as {
        captureMetadata?: {
          records: Array<{
            start_line: number;
            end_line: number;
          }>;
        };
      };
      const records = stored.captureMetadata?.records ?? [];

      assert.equal(records[0]?.start_line, 3);
      assert.equal(records[0]?.end_line, 3);
      assert.equal(records[1]?.start_line, 4);
      assert.equal(records[1]?.end_line, 5);
    }
  },
  {
    name: 'rejects empty captures',
    async run() {
      resetWebSpace();

      await assert.rejects(() => captureToNote('   '), /Capture text cannot be empty/);
    }
  },
  {
    name: 'loads a browser QA space outside the Tauri runtime',
    async run() {
      resetWebSpace();

      const snapshot = await loadSpace();

      assert.equal(snapshot.space.name, 'Lithe Web QA');
      assert.equal(snapshot.space.path, 'browser://lithe-web-qa');
      assert.equal(snapshot.active_note.path, 'default.md');
      assert.equal(snapshot.notes.length, 1);
      assert.match(snapshot.content, /^# Welcome to Lithe/);
    }
  },
  {
    name: 'persists browser QA note edits in local storage',
    async run() {
      resetWebSpace();

      const created = await createNote('QA Cursor Check');
      await writeNote(created.active_note.path, '# Cursor\n\nBackspace check');
      const loaded = await readNote(created.active_note.path);

      assert.equal(created.active_note.path, 'QA-Cursor-Check.md');
      assert.equal(loaded.content, '# Cursor\n\nBackspace check');
    }
  },
  {
    name: 'does not change the active browser QA note when saving another note',
    async run() {
      resetWebSpace();

      const first = await createNote('First');
      const second = await createNote('Second');

      await writeNote(first.active_note.path, 'late save');
      const loaded = await loadSpace();

      assert.equal(second.active_note.path, 'Second.md');
      assert.equal(loaded.active_note.path, 'Second.md');
      assert.equal((await readNote(first.active_note.path)).content, 'late save');
    }
  },
  {
    name: 'does not change the active browser QA note when reading another note',
    async run() {
      resetWebSpace();

      const first = await createNote('First');
      const second = await createNote('Second');

      await readNote(first.active_note.path);
      const loaded = await loadSpace();

      assert.equal(second.active_note.path, 'Second.md');
      assert.equal(loaded.active_note.path, 'Second.md');
    }
  },
  {
    name: 'remembers the active browser QA note explicitly',
    async run() {
      resetWebSpace();

      const first = await createNote('First');
      await createNote('Second');

      await rememberActiveNote(first.active_note.path);
      const loaded = await loadSpace();

      assert.equal(loaded.active_note.path, 'First.md');
    }
  },
  {
    name: 'keeps a default browser QA note after deleting the last note',
    async run() {
      resetWebSpace();

      const snapshot = await loadSpace();
      const afterDelete = await deleteNote(snapshot.active_note.path);

      assert.equal(afterDelete.active_note.path, 'default.md');
      assert.equal(afterDelete.notes.length, 1);
      assert.match(afterDelete.content, /^# Welcome to Lithe/);
    }
  }
];
