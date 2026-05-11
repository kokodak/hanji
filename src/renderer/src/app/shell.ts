export interface AppShell {
  captureComposer: HTMLTextAreaElement;
  captureOverlay: HTMLDivElement;
  captureSaveButton: HTMLButtonElement;
  captureStatus: HTMLElement;
  editor: HTMLDivElement;
  noteList: HTMLDivElement;
  newNoteButton: HTMLButtonElement;
  newNoteName: HTMLInputElement;
  noteMenu: HTMLDivElement;
  deleteNoteButton: HTMLButtonElement;
  spacePath: HTMLElement;
  openSpaceButton: HTMLButtonElement;
  spaceName: HTMLElement;
  currentTime: HTMLTimeElement;
  cursorPosition: HTMLElement;
}

export function createAppShell(root: HTMLDivElement): AppShell {
  root.innerHTML = `
    <main class="shell">
      <aside class="space-panel" aria-label="Space" draggable="false">
        <div class="space-panel-header" data-tauri-drag-region>
          <div class="space-panel-title" data-tauri-drag-region>
            <div id="space-name" class="space-name">Lithe</div>
            <div id="space-path" class="space-path" aria-label="Space path"></div>
          </div>
          <button id="open-space" class="icon-button" type="button" title="Open Space">Open</button>
        </div>

        <div class="new-note-row">
          <input id="new-note-name" aria-label="New note name" placeholder="New note" spellcheck="false" />
          <button id="new-note" class="icon-button" type="button" title="Create note">+</button>
        </div>

        <div id="note-list" class="note-list" role="listbox" aria-label="Notes"></div>
        <div id="note-menu" class="context-menu" hidden>
          <button id="delete-note" type="button">Delete</button>
        </div>
      </aside>

      <section class="workspace" aria-label="Editor">
        <header class="toolbar" draggable="false" data-tauri-drag-region>
          <time id="current-time" dateTime=""></time>
          <span id="cursor-position" class="cursor-position"></span>
        </header>

        <div class="editor-layout">
          <div id="editor" aria-label="Markdown editor"></div>
        </div>
      </section>

      <div id="capture-overlay" class="capture-overlay" hidden>
        <section class="capture-composer" aria-label="Capture composer">
          <textarea id="capture-composer" aria-label="Capture thought" placeholder="Write a thought..." rows="1" spellcheck="true"></textarea>
          <div class="capture-actions">
            <span id="capture-status" class="capture-status" role="status"></span>
            <button id="capture-save" class="capture-save" type="button">Save Capture</button>
          </div>
        </section>
      </div>
    </main>
  `;

  const captureComposer = document.querySelector<HTMLTextAreaElement>('#capture-composer');
  const captureOverlay = document.querySelector<HTMLDivElement>('#capture-overlay');
  const captureSaveButton = document.querySelector<HTMLButtonElement>('#capture-save');
  const captureStatus = document.querySelector<HTMLElement>('#capture-status');
  const editor = document.querySelector<HTMLDivElement>('#editor');
  const noteList = document.querySelector<HTMLDivElement>('#note-list');
  const newNoteButton = document.querySelector<HTMLButtonElement>('#new-note');
  const newNoteName = document.querySelector<HTMLInputElement>('#new-note-name');
  const noteMenu = document.querySelector<HTMLDivElement>('#note-menu');
  const deleteNoteButton = document.querySelector<HTMLButtonElement>('#delete-note');
  const spacePath = document.querySelector<HTMLElement>('#space-path');
  const openSpaceButton = document.querySelector<HTMLButtonElement>('#open-space');
  const spaceName = document.querySelector<HTMLElement>('#space-name');
  const currentTime = document.querySelector<HTMLTimeElement>('#current-time');
  const cursorPosition = document.querySelector<HTMLElement>('#cursor-position');

  if (
    !captureComposer ||
    !captureOverlay ||
    !captureSaveButton ||
    !captureStatus ||
    !editor ||
    !noteList ||
    !newNoteButton ||
    !newNoteName ||
    !noteMenu ||
    !deleteNoteButton ||
    !spacePath ||
    !openSpaceButton ||
    !spaceName ||
    !currentTime ||
    !cursorPosition
  ) {
    throw new Error('Editor controls were not found.');
  }

  return {
    captureComposer,
    captureOverlay,
    captureSaveButton,
    captureStatus,
    editor,
    noteList,
    newNoteButton,
    newNoteName,
    noteMenu,
    deleteNoteButton,
    spacePath,
    openSpaceButton,
    spaceName,
    currentTime,
    cursorPosition
  };
}
