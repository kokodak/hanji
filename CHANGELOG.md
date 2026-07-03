# Changelog

All notable changes to Hanji will be documented in this file.

The format is based on [Keep a Changelog], and this project follows [Semantic Versioning].

## [Unreleased]

## [0.1.1] - 2026-07-04

### Added

- Added a welcome screen with create and open actions.
- Added a Markdown file browser sidebar for opened folders.

### Changed

- Refined the file browser sidebar layout and status bar integration.
- Updated the README project overview and contributor-facing templates.

### Fixed

- Restored Hanji windows when reopening the macOS app after closing them.
- Improved save-as behavior for new untitled documents.
- Restricted document opening to Markdown files.
- Removed redundant opened-file status copy from the editor chrome.

## [0.1.0] - 2026-07-02

### Added

- Added the Rust GPUI desktop editor as the main Hanji app.
- Added source-backed WYSIWYG Markdown preview for headings, inline emphasis, strong text, inline code, links, autolinks, raw URLs, strikethrough, blockquotes, lists, checkboxes, horizontal rules, and fenced code blocks.
- Added local file open and save support for Markdown files.
- Added core crates for text editing, Markdown projection, local storage, and future plugin API boundaries.
- Added macOS DMG packaging and GitHub Release automation for preview distribution.

[Unreleased]: https://github.com/kokodak/hanji/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/kokodak/hanji/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/kokodak/hanji/releases/tag/v0.1.0
[Keep a Changelog]: https://keepachangelog.com/en/2.0.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html
