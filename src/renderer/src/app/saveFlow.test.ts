import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const mainSource = readFileSync(new URL('../main.ts', import.meta.url), 'utf8');
const spaceStorageSource = readFileSync(new URL('./spaceStorage.ts', import.meta.url), 'utf8');

export const tests = [
  {
    name: 'flushes pending editor saves before switching notes',
    run() {
      assert.match(mainSource, /async function flushPendingSave\(\): Promise<void>/);
      assert.match(mainSource, /await flushPendingSave\(\);\n\s+applyLatestSnapshot\(requestId, await readNote\(notePath\)\);/);
      assert.match(
        mainSource,
        /flushPendingSave\(\)\.then\(\(\) => createNote\(noteName\)\.then\(\(snapshot\) => applyLatestSnapshot\(requestId, snapshot\)\)\)/
      );
    }
  },
  {
    name: 'serializes editor saves so stale writes cannot finish last',
    run() {
      assert.match(mainSource, /let saveChain = Promise\.resolve\(\);/);
      assert.match(mainSource, /function saveNoteContent\(notePath: string, text: string\): Promise<void>/);
      assert.match(mainSource, /const write = saveChain\.then\(\(\) => writeNote\(notePath, text\)\);/);
      assert.match(mainSource, /saveChain = write\.catch\(\(\) => undefined\);/);
      assert.match(mainSource, /await saveNoteContent\(activeNote\.path, editorView\.state\.doc\.toString\(\)\);/);
    }
  },
  {
    name: 'ignores stale note snapshots after rapid switching',
    run() {
      assert.match(mainSource, /let snapshotRequestId = 0;/);
      assert.match(
        mainSource,
        /function applyLatestSnapshot\(requestId: number, snapshot: SpaceSnapshot, cursorPosition\?: number\): void/
      );
      assert.match(mainSource, /if \(requestId !== snapshotRequestId\) return;/);
      assert.match(mainSource, /void rememberActiveNote\(snapshot\.active_note\.path\);/);
      assert.match(mainSource, /const requestId = \+\+snapshotRequestId;\n\s+await flushPendingSave\(\);/);
    }
  },
  {
    name: 'does not select a browser QA note while saving its content',
    run() {
      const writeNoteBody = /export async function writeNote\(path: string, content: string\): Promise<void> \{([\s\S]*?)\n\}/.exec(
        spaceStorageSource
      )?.[1];

      assert.ok(writeNoteBody);
      assert.doesNotMatch(writeNoteBody, /activePath\s*=\s*path/);
    }
  }
];
