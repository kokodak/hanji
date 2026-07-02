# Releasing Hanji

Hanji releases are distributed through GitHub Releases. The release source is an annotated Git tag, the release notes come from `CHANGELOG.md`, and the macOS download artifact is a DMG.

## Versioning

Use `MAJOR.MINOR.PATCH` tags with a `v` prefix, such as `v0.1.0`.

While Hanji is in `0.x`, minor versions may include meaningful product, file-format, or plugin-contract changes. Patch versions should stay limited to bug fixes, compatibility fixes, and packaging corrections.

Use prerelease tags such as `v0.2.0-alpha.1` only when a build should be shared for testing without becoming the default stable release.

## Changelog

Keep `CHANGELOG.md` as the human-edited release source, using the Keep a Changelog structure. Every released version needs a section like this:

```md
## [0.1.0] - 2026-07-02
```

Use the standard change groups: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, and `Security`.

The GitHub Actions workflow extracts that section and uses it as the GitHub Release body.

## Local Package

Run the same checks the release workflow runs:

```sh
make test
make check-app
make package-macos VERSION=0.1.0
```

The package command writes these files:

```text
dist/Hanji-0.1.0-macos-arm64.dmg
dist/Hanji-0.1.0-macos-arm64.dmg.sha256
```

The architecture suffix follows the build machine. Apple Silicon builds use `macos-arm64`; Intel builds use `macos-x86_64`.

## GitHub Release

Create and push an annotated tag:

```sh
git tag -a v0.1.0 -m "Release Hanji 0.1.0"
git push origin v0.1.0
```

Pushing a `v*.*.*` tag starts `.github/workflows/release.yml`. The workflow:

- checks out the tag,
- validates that the tag version matches `Cargo.toml`,
- runs Rust tests and the GPUI app check,
- builds a macOS DMG,
- writes a SHA-256 checksum file,
- creates or updates a draft GitHub Release with the DMG assets.

Review the draft release before publishing it.

## Signing

The current packaging script can codesign the app when `CODESIGN_IDENTITY` is provided. It does not yet import certificates or notarize the DMG in CI.

Before linking a public website download to the DMG, add Developer ID signing and notarization to the release workflow.
