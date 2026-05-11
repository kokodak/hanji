export interface AppShell {
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
    </main>
  `;

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
