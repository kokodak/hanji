# Plugin API Draft

This is a draft contract for Hanji plugins. It is intentionally small.

## Design Constraints

- Plugins must not make the core editor slower when disabled.
- Plugin permissions should be visible and understandable.
- APIs should be versioned before third-party plugins are encouraged.
- The app should be able to disable or unload a plugin cleanly.

## Example Shape

```ts
export interface HanjiPlugin {
  id: string;
  name: string;
  version: string;
  activate(context: HanjiPluginContext): void | Promise<void>;
  deactivate?(): void | Promise<void>;
}

export interface HanjiPluginContext {
  editor: {
    getText(): string;
    setText(nextText: string): void;
    insertText(text: string): void;
  };
  commands: {
    register(command: HanjiCommand): Disposable;
  };
  storage: {
    get<T>(key: string): Promise<T | undefined>;
    set<T>(key: string, value: T): Promise<void>;
  };
}

export interface HanjiCommand {
  id: string;
  title: string;
  run(): void | Promise<void>;
}

export interface Disposable {
  dispose(): void;
}
```

## Open Questions

- Should plugins run in a sandboxed renderer, a worker, or a separate process?
- How should plugin permissions be declared and reviewed?
- What is the smallest command contribution model that still feels useful?
- How much of collaboration state should plugins be able to observe?
