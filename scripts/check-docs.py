#!/usr/bin/env python3
"""Validate Hanji's documentation structure without third-party packages."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"

CATEGORY_STATUSES = {
    "architecture": {"Current"},
    "design": {"Current"},
    "reference": {"Current"},
    "development": {"Current"},
    "plans": {"Proposed"},
    "decisions": {"Accepted", "Superseded"},
}

ROOT_DOCUMENT_STATUSES = {
    "documentation-guide.md": {"Current"},
    "philosophy.md": {"Current"},
}

FOLDER_CONTRACT_HEADINGS = {
    "## Belongs Here",
    "## Does Not Belong Here",
    "## Contents",
    "## Maintenance Rule",
}

LINK_PATTERN = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
INLINE_CODE_PATTERN = re.compile(r"(`+).*?\1")
FENCE_PATTERN = re.compile(r"^\s*(`{3,}|~{3,})")
SCHEME_PATTERN = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")
PAGE_NAME_PATTERN = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*\.md$")
DECISION_NAME_PATTERN = re.compile(r"^[0-9]{4}-[a-z0-9]+(?:-[a-z0-9]+)*\.md$")


def repository_markdown_files() -> list[Path]:
    files = list(ROOT.glob("*.md"))
    files.extend(DOCS.rglob("*.md"))
    github = ROOT / ".github"
    if github.exists():
        files.extend(github.rglob("*.md"))
    return sorted(set(path.resolve() for path in files))


def markdown_links(path: Path) -> list[tuple[int, str]]:
    links: list[tuple[int, str]] = []
    fence_character: str | None = None

    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        fence = FENCE_PATTERN.match(line)
        if fence:
            character = fence.group(1)[0]
            if fence_character is None:
                fence_character = character
            elif fence_character == character:
                fence_character = None
            continue

        if fence_character is not None:
            continue

        visible_line = INLINE_CODE_PATTERN.sub("", line)
        for match in LINK_PATTERN.finditer(visible_line):
            links.append((line_number, match.group(1).strip()))

    return links


def link_path(raw_target: str) -> str | None:
    if not raw_target or raw_target.startswith("#"):
        return None

    if raw_target.startswith("<"):
        closing = raw_target.find(">")
        if closing == -1:
            return raw_target
        target = raw_target[1:closing]
    else:
        target = raw_target.split(maxsplit=1)[0]

    if SCHEME_PATTERN.match(target):
        return None

    return unquote(target.split("#", 1)[0].split("?", 1)[0])


def relative(path: Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def validate() -> list[str]:
    errors: list[str] = []
    docs_files = sorted(path.resolve() for path in DOCS.rglob("*.md"))
    docs_set = set(docs_files)

    allowed_root_files = {"README.md", *ROOT_DOCUMENT_STATUSES}
    for path in DOCS.glob("*.md"):
        if path.name not in allowed_root_files:
            errors.append(
                f"{relative(path)}: top-level technical pages must live in a documentation category"
            )

    category_indexes: dict[str, Path] = {}
    for category in CATEGORY_STATUSES:
        index = (DOCS / category / "README.md").resolve()
        category_indexes[category] = index
        if not index.is_file():
            errors.append(f"{relative(index)}: required category index is missing")
            continue

        text = index.read_text(encoding="utf-8")
        for heading in sorted(FOLDER_CONTRACT_HEADINGS):
            if heading not in text.splitlines():
                errors.append(f"{relative(index)}: missing folder contract heading {heading!r}")

    for path in docs_files:
        if path.name == "README.md":
            continue

        rel = path.relative_to(DOCS)
        if len(rel.parts) > 2:
            errors.append(
                f"{relative(path)}: category folders must remain flat until a nested contract is defined"
            )

        if len(rel.parts) == 1:
            expected = ROOT_DOCUMENT_STATUSES.get(path.name)
        else:
            expected = CATEGORY_STATUSES.get(rel.parts[0])
            name_pattern = (
                DECISION_NAME_PATTERN if rel.parts[0] == "decisions" else PAGE_NAME_PATTERN
            )
            if not name_pattern.fullmatch(path.name):
                errors.append(f"{relative(path)}: filename does not match its category convention")

        if expected is None:
            errors.append(f"{relative(path)}: no status policy exists for this location")
            continue

        status = None
        for line in path.read_text(encoding="utf-8").splitlines()[:8]:
            if line.startswith("Status: "):
                status = line.removeprefix("Status: ").strip()
                break

        if status is None:
            errors.append(f"{relative(path)}: missing Status line in the first eight lines")
        elif status not in expected:
            allowed = ", ".join(sorted(expected))
            errors.append(
                f"{relative(path)}: status {status!r} is not allowed here; expected {allowed}"
            )

    markdown_files = repository_markdown_files()
    incoming: dict[Path, int] = {path: 0 for path in docs_files}
    targets_by_source: dict[Path, set[Path]] = {}
    local_link_count = 0

    for source in markdown_files:
        targets: set[Path] = set()
        for line_number, raw_target in markdown_links(source):
            path_part = link_path(raw_target)
            if path_part is None or not path_part:
                continue

            if path_part.startswith("/"):
                errors.append(
                    f"{relative(source)}:{line_number}: local documentation links must be relative: {raw_target}"
                )
                continue

            target = (source.parent / path_part).resolve()
            try:
                target.relative_to(ROOT)
            except ValueError:
                errors.append(
                    f"{relative(source)}:{line_number}: link points outside the repository: {raw_target}"
                )
                continue

            local_link_count += 1
            if not target.exists():
                errors.append(
                    f"{relative(source)}:{line_number}: local link target does not exist: {raw_target}"
                )
                continue

            targets.add(target)
            if target in docs_set:
                incoming[target] += 1

        targets_by_source[source] = targets

    docs_index = (DOCS / "README.md").resolve()
    for path, count in incoming.items():
        if path != docs_index and count == 0:
            errors.append(f"{relative(path)}: page is not linked from another Markdown page")

    for category, index in category_indexes.items():
        if not index.exists():
            continue
        indexed_targets = targets_by_source.get(index, set())
        for page in sorted((DOCS / category).glob("*.md")):
            page = page.resolve()
            if page == index:
                continue
            if page not in indexed_targets:
                errors.append(
                    f"{relative(index)}: Contents must link sibling page {page.name}"
                )

    if not errors:
        print(
            "Documentation checks passed: "
            f"{len(docs_files)} pages, {len(category_indexes)} category indexes, "
            f"{local_link_count} local links."
        )

    return errors


def main() -> int:
    errors = validate()
    if not errors:
        return 0

    print("Documentation checks failed:", file=sys.stderr)
    for error in sorted(errors):
        print(f"- {error}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
