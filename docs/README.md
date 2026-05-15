# Hanji Docs

These docs are the working shelf for Hanji's product philosophy, Rust editor architecture, and design concepts.

Hanji is a light, local-first WYSIWYG Markdown editor built around simple plain text durability.

## Current Direction

- Use GPUI for the Rust desktop editor.
- Keep Markdown text as the source of truth.
- Build a small editor core before broad product features.
- Keep storage local, visible, and boring.
- Treat plugins as future public contracts, not early internal shortcuts.

## Map

- [Philosophy](philosophy.md): product values and boundaries.
- [Architecture](architecture.md): Rust track shape and GPUI boundary.
- [Design Notes](design/README.md): core editor concepts.

## Rust Commands

Core crates can be checked without the GPUI desktop dependency:

```sh
cargo test --workspace --exclude hanji-rust
```

The GPUI app requires a macOS toolchain that can compile Metal shaders:

```sh
cargo run -p hanji-rust
```

If Xcode reports a missing Metal Toolchain, install it and run Cargo with the installed toolchain identifier:

```sh
xcodebuild -downloadComponent metalToolchain -exportPath /private/tmp/HanjiMetalToolchain
xcodebuild -showComponent MetalToolchain
TOOLCHAINS=<Toolchain Identifier> cargo run -p hanji-rust
```

## Writing Rule

Docs should stay short, concrete, and easy to revise. Prefer a small durable note over a broad speculative document.
