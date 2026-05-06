import assert from 'node:assert/strict';
import { createNote, deleteNote, loadSpace, readNote, writeNote } from './spaceStorage';

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
