import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const styles = readFileSync(new URL('../styles.css', import.meta.url), 'utf8');
const shellSource = readFileSync(new URL('./shell.ts', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../main.ts', import.meta.url), 'utf8');
const cargoManifest = readFileSync(new URL('../../../../src-tauri/Cargo.toml', import.meta.url), 'utf8');
const tauriCapability = JSON.parse(readFileSync(new URL('../../../../src-tauri/capabilities/default.json', import.meta.url), 'utf8'));
const tauriConfig = JSON.parse(readFileSync(new URL('../../../../src-tauri/tauri.conf.json', import.meta.url), 'utf8'));

function getRuleBody(selector: string): string {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = new RegExp(`${escapedSelector}\\s*\\{([^}]*)\\}`).exec(styles);

  assert.ok(match, `Expected ${selector} rule to exist.`);

  return match[1];
}

export const tests = [
  {
    name: 'keeps web and desktop app backgrounds separate',
    run() {
      const rootRule = getRuleBody(':root');
      const bodyRule = getRuleBody('body');
      const desktopRootRule = getRuleBody('html.is-desktop-app');
      const desktopHostRule = getRuleBody('html.is-desktop-app,\nhtml.is-desktop-app body,\nhtml.is-desktop-app #app');
      const desktopShellRule = getRuleBody('html.is-desktop-app .shell');
      const editorRule = getRuleBody('#editor');
      const desktopEditorRule = getRuleBody('html.is-desktop-app #editor');
      const desktopTextRule = getRuleBody(
        'html.is-desktop-app .space-name,\nhtml.is-desktop-app .note-item,\nhtml.is-desktop-app #current-time'
      );
      const mainWindow = tauriConfig.app.windows[0];

      assert.equal(tauriConfig.app.macOSPrivateApi, true);
      assert.match(cargoManifest, /tauri = \{ version = "2", features = \["macos-private-api"\] \}/);
      assert.equal(mainWindow.backgroundColor, '#00000000');
      assert.equal(mainWindow.transparent, true);
      assert.equal(mainWindow.titleBarStyle, 'Overlay');
      assert.equal(mainWindow.hiddenTitle, true);
      assert.deepEqual(mainWindow.windowEffects.effects, ['popover']);
      assert.match(rootRule, /background:\s*#f4f6f3;/);
      assert.match(bodyRule, /linear-gradient\(135deg, rgb\(244 246 243 \/ 96%\), rgb\(238 243 240 \/ 96%\)\)/);
      assert.match(desktopRootRule, /--app-ink:\s*#ffffff;/);
      assert.match(desktopRootRule, /--app-hairline:\s*rgb\(255 255 255 \/ 24%\);/);
      assert.match(desktopRootRule, /--app-glass-strong:\s*rgb\(255 255 252 \/ 86%\);/);
      assert.match(desktopHostRule, /background:\s*transparent;/);
      assert.match(desktopShellRule, /background:\s*var\(--app-glass\);/);
      assert.match(desktopShellRule, /backdrop-filter:\s*blur\(42px\) saturate\(1\.95\) brightness\(1\.08\);/);
      assert.match(editorRule, /background-color:\s*var\(--app-paper\);/);
      assert.match(editorRule, /radial-gradient\(circle at 20% 30%, rgb\(31 38 36 \/ 0\.9%\)/);
      assert.match(editorRule, /background-size:\s*17px 17px, 23px 23px;/);
      assert.match(desktopEditorRule, /0 0 0 1px rgb\(16 19 20 \/ 10%\);/);
      assert.match(desktopEditorRule, /backdrop-filter:\s*blur\(6px\) saturate\(1\.04\) brightness\(1\.03\);/);
      assert.match(desktopTextRule, /color:\s*var\(--app-ink\);/);
    }
  },
  {
    name: 'keeps overlay title bar drag regions available in the app',
    run() {
      assert.deepEqual(tauriCapability.permissions, ['core:default', 'core:window:allow-start-dragging', 'dialog:default']);
      assert.match(mainSource, /import \{ getCurrentWindow \} from '@tauri-apps\/api\/window';/);
      assert.match(mainSource, /document\.documentElement\.classList\.toggle\('is-desktop-app', hasTauriRuntime\(\)\);/);
      assert.match(mainSource, /function registerWindowDragging\(\): void/);
      assert.match(mainSource, /appWindow\.startDragging\(\)/);
      assert.match(shellSource, /class="space-panel-header" data-tauri-drag-region/);
      assert.match(shellSource, /class="space-panel-title" data-tauri-drag-region/);
      assert.match(shellSource, /class="toolbar" draggable="false" data-tauri-drag-region/);
      assert.match(mainSource, /closest\('\[data-tauri-drag-region\]'\)/);
    }
  },
  {
    name: 'starts the file explorer sidebar collapsed and toggles it with Cmd B',
    run() {
      const shellRule = getRuleBody('.shell');
      const openShellRule = getRuleBody('.shell.is-sidebar-open');
      const spacePanelRule = getRuleBody('.space-panel');
      const openSpacePanelRule = getRuleBody('.shell.is-sidebar-open .space-panel');

      assert.match(shellRule, /grid-template-columns:\s*0 minmax\(0, 1fr\);/);
      assert.match(openShellRule, /grid-template-columns:\s*260px minmax\(0, 1fr\);/);
      assert.match(spacePanelRule, /opacity:\s*0;/);
      assert.match(spacePanelRule, /pointer-events:\s*none;/);
      assert.match(openSpacePanelRule, /opacity:\s*1;/);
      assert.match(mainSource, /function isSidebarShortcut\(event: KeyboardEvent\): boolean/);
      assert.match(mainSource, /event\.key\.toLowerCase\(\) === 'b'/);
      assert.match(mainSource, /setSidebarOpen\(false\);/);
      assert.match(mainSource, /shell\.spacePanel\.inert = !open;/);
      assert.match(mainSource, /setSidebarOpen\(!isSidebarOpen\);/);
    }
  },
  {
    name: 'contains the capture composer flow',
    run() {
      const overlayRule = getRuleBody('.capture-overlay');
      const composerRule = getRuleBody('.capture-composer');
      const textareaRule = getRuleBody('.capture-composer textarea');
      const desktopComposerRule = getRuleBody('html.is-desktop-app .capture-composer');
      const desktopTextareaRule = getRuleBody('html.is-desktop-app .capture-composer textarea');

      assert.match(shellSource, /id="capture-overlay"/);
      assert.match(shellSource, /id="capture-composer"/);
      assert.match(shellSource, /id="capture-save"/);
      assert.match(mainSource, /captureToNote\(text\)/);
      assert.match(mainSource, /applyLatestSnapshot\(requestId, snapshot, snapshot\.content\.length\)/);
      assert.match(mainSource, /function isCaptureShortcut\(event: KeyboardEvent\): boolean/);
      assert.match(mainSource, /event\.shiftKey && event\.key === ' '/);
      assert.match(mainSource, /shell\.captureComposer\.addEventListener\('keydown'/);
      assert.match(mainSource, /if \(isTextEntryTarget\(event\.target\)\) return;/);
      assert.match(overlayRule, /position:\s*fixed;/);
      assert.match(overlayRule, /justify-content:\s*flex-end;/);
      assert.match(composerRule, /max-width:\s*44vw;/);
      assert.match(composerRule, /border-radius:\s*8px;/);
      assert.match(textareaRule, /max-height:\s*52vh;/);
      assert.match(textareaRule, /resize:\s*none;/);
      assert.match(desktopComposerRule, /background:\s*rgb\(16 19 20 \/ 52%\);/);
      assert.match(desktopTextareaRule, /color:\s*#ffffff;/);
    }
  },
  {
    name: 'keeps the app chrome fixed inside the viewport',
    run() {
      assert.match(getRuleBody('body'), /overflow:\s*hidden;/);
      assert.match(getRuleBody('.shell'), /height:\s*100vh;/);
      assert.match(getRuleBody('.shell'), /overflow:\s*hidden;/);
      assert.match(getRuleBody('.space-panel'), /height:\s*100vh;/);
      assert.match(getRuleBody('.space-panel'), /overflow:\s*hidden;/);
    }
  },
  {
    name: 'contains scroll momentum to scrollable content surfaces',
    run() {
      assert.match(getRuleBody('.workspace'), /overflow:\s*hidden;/);
      assert.match(getRuleBody('.editor-layout'), /overflow:\s*hidden;/);
      assert.match(getRuleBody('.editor-layout'), /height:\s*100%;/);
      assert.match(getRuleBody('#editor'), /overflow:\s*hidden;/);
      assert.match(getRuleBody('#editor .cm-editor'), /min-height:\s*0;/);
      assert.match(getRuleBody('.note-list'), /overscroll-behavior:\s*contain;/);
      assert.match(getRuleBody('#editor .cm-scroller'), /height:\s*100%;/);
      assert.match(getRuleBody('#editor .cm-scroller'), /max-height:\s*100%;/);
      assert.match(getRuleBody('#editor .cm-scroller'), /overflow:\s*auto;/);
      assert.match(getRuleBody('#editor .cm-scroller'), /overscroll-behavior:\s*contain;/);
    }
  }
];
