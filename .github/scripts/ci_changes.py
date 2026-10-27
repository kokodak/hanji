#!/usr/bin/env python3
"""Classify PR paths conservatively; unknown changes keep the Rust suite."""

from __future__ import annotations

import os
import subprocess
from pathlib import PurePosixPath


def needs_rust(paths: list[str]) -> bool:
    if not paths:
        return True
    for name in paths:
        path = PurePosixPath(name)
        if (
            path.is_absolute()
            or ".." in path.parts
            or "\\" in name
            or path.suffix != ".md"
            or not (len(path.parts) == 1 or path.parts[0] == "docs")
        ):
            return True
    return False


def changed_paths(base: str, head: str) -> list[str]:
    ancestor = subprocess.check_output(
        ["git", "merge-base", base, head], text=True
    ).strip()
    # Do not collapse renames: moving source into docs still changes the build.
    raw = subprocess.check_output(
        ["git", "diff", "--name-only", "--no-renames", "-z", ancestor, head, "--"]
    )
    return [os.fsdecode(name) for name in raw.split(b"\0") if name]


def main() -> None:
    # main pushes and unrecognized events always run the complete suite.
    run = True
    if os.environ["GITHUB_EVENT_NAME"] == "pull_request":
        run = needs_rust(changed_paths(os.environ["BASE_SHA"], os.environ["HEAD_SHA"]))
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"run_rust={str(run).lower()}\n")
    print("Run Rust suite" if run else "Documentation-only PR: skip Rust suite")


if __name__ == "__main__":
    main()
