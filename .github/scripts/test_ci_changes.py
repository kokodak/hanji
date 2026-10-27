"""Exercise classification and actual Git diffs without external dependencies."""

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from ci_changes import changed_paths, needs_rust


class ClassificationTests(unittest.TestCase):
    def test_documentation_only(self):
        self.assertFalse(needs_rust(["README.md", "docs/design/document-model.md"]))

    def test_build_sensitive_and_unknown_paths(self):
        for path in (
            "crates/hanji-core/src/lib.rs", "Cargo.toml", "Cargo.lock", "build.rs",
            "apps/hanji/config.json", ".github/workflows/ci.yml", ".github/notes.md",
            "scripts/check-docs.py", "Makefile", "docs/build.rs", "site/index.html",
            "docs/../build.md", "/README.md", "docs\\notes.md", "unknown.txt",
        ):
            with self.subTest(path=path):
                self.assertTrue(needs_rust(["README.md", path]))

    def test_empty_diff_runs_rust(self):
        self.assertTrue(needs_rust([]))

    def test_push_runs_rust_without_a_pr_diff(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / "output"
            subprocess.run(
                [os.sys.executable, str(Path(__file__).with_name("ci_changes.py"))],
                env={**os.environ, "GITHUB_EVENT_NAME": "push", "GITHUB_OUTPUT": str(output)},
                check=True,
            )
            self.assertEqual(output.read_text(encoding="utf-8"), "run_rust=true\n")

    def test_invalid_pr_diff_does_not_emit_a_skip(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / "output"
            result = subprocess.run(
                [os.sys.executable, str(Path(__file__).with_name("ci_changes.py"))],
                env={**os.environ, "GITHUB_EVENT_NAME": "pull_request", "GITHUB_OUTPUT": str(output),
                     "BASE_SHA": "missing-base", "HEAD_SHA": "missing-head"},
                cwd=folder,
                capture_output=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(output.exists())

    def test_git_rename_and_merge_base(self):
        with tempfile.TemporaryDirectory() as folder:
            repo = Path(folder)

            def git(*args):
                return subprocess.check_output(["git", "-C", folder, *args], text=True).strip()

            git("init", "-q", "-b", "main")
            git("config", "user.name", "CI test")
            git("config", "user.email", "ci@example.invalid")
            (repo / "source.rs").write_text("source\n", encoding="utf-8")
            git("add", ".")
            git("commit", "-qm", "base")
            base = git("rev-parse", "HEAD")
            git("checkout", "-qb", "docs-pr")
            (repo / "docs").mkdir()
            git("mv", "source.rs", "docs/source.md")
            (repo / "docs" / "line\nbreak.md").write_text("doc\n", encoding="utf-8")
            git("add", ".")
            git("commit", "-qm", "rename")
            head = git("rev-parse", "HEAD")
            git("checkout", "-q", "main")
            (repo / "unrelated.rs").write_text("main advanced\n", encoding="utf-8")
            git("add", ".")
            git("commit", "-qm", "advance main")
            advanced_base = git("rev-parse", "HEAD")
            previous = Path.cwd()
            try:
                os.chdir(repo)
                paths = changed_paths(advanced_base, head)
                self.assertCountEqual(paths, ["source.rs", "docs/source.md", "docs/line\nbreak.md"])
                self.assertTrue(needs_rust(paths))
                self.assertEqual(changed_paths(base, base), [])
            finally:
                os.chdir(previous)


if __name__ == "__main__":
    unittest.main()
