"""Tests for the check selection of tools/ci/run_headless.py."""
from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools" / "ci"))
import run_headless as rh  # noqa: E402


def git(root, *args):
    return subprocess.run(["git", "-c", "core.autocrlf=false", "-c", "user.name=t", "-c", "user.email=t@example.invalid",
                           *args], cwd=root, check=True, capture_output=True, text=True).stdout.strip()


class SelectChecksTests(unittest.TestCase):
    def mode(self, changed, literals=frozenset()):
        return rh.select_checks(changed, set(literals))

    def test_documentation_markdown_runs_the_light_checks(self):
        mode, _, checks = self.mode(["docs/wiki/log.md", "docs/wiki/decisions/x.md",
                                     "docs/development-guide/AGENT_BRIEFING.md", "docs/progress/1.0/plan.md"])
        self.assertEqual(mode, "docs-only")
        self.assertIs(checks, rh.LIGHT_CHECKS)

    def test_anything_else_runs_every_check(self):
        for changed in (None, [], ["docs/wiki/log.md", "core.py"], ["docs/wiki/data.json"],
                        ["docs/README.md"], ["research/jumbling/restoration/README.md"], ["README.md"],
                        ["AGENTS.md"], ["docs/progress/status.json"], ["docs/wiki"], ["docs/wikix/a.md"]):
            with self.subTest(changed=changed):
                mode, _, checks = self.mode(changed)
                self.assertEqual(mode, "full")
                self.assertIs(checks, rh.CHECKS)

    def test_markdown_that_a_full_check_reads_runs_every_check(self):
        for path in ("docs/progress/1.0/jumbling-midstage.md", "docs/progress/1.0/packets/renderer/README.md",
                     "docs/progress/1.0/inventory/notes.md", "docs/progress/1.0/dispositions/d.md"):
            with self.subTest(path=path):
                self.assertEqual(self.mode(["docs/wiki/log.md", path])[0], "full")

    def test_a_path_named_in_a_full_check_runs_every_check(self):
        self.assertEqual(self.mode(["docs/progress/1.0/new.md"], {"docs/progress/1.0/new.md"})[0], "full")
        self.assertEqual(self.mode(["docs/progress/1.0/x/y.md"], {"docs/progress/1.0/x"})[0], "full")
        # A literal that only shares a prefix with the path does not count.
        self.assertEqual(self.mode(["docs/progress/1.0/xy.md"], {"docs/progress/1.0/x"})[0], "docs-only")

    def test_light_checks_are_listed_checks(self):
        self.assertTrue(all(check in rh.CHECKS for check in rh.LIGHT_CHECKS))
        self.assertIn(["tests/test_ci_headless.py"], rh.LIGHT_CHECKS)

    def test_repository_literals_name_the_audited_readers(self):
        literals = rh.full_check_literals()
        self.assertIn("docs/progress/1.0/jumbling-midstage.md", literals)
        self.assertIn("docs/progress/1.0/packets/renderer", literals)
        self.assertFalse(any(literal.startswith("docs/wiki") for literal in literals), literals)


class ChangedFilesTests(unittest.TestCase):
    def test_paths_from_the_merge_base(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            git(root, "init", "-q", "-b", "main")
            (root / "a.md").write_text("a\n", encoding="utf-8")
            git(root, "add", "a.md")
            git(root, "commit", "-q", "-m", "base")
            base = git(root, "rev-parse", "HEAD")
            git(root, "checkout", "-q", "-b", "topic")
            (root / "docs").mkdir()
            (root / "docs" / "b c.md").write_text("b\n", encoding="utf-8")
            git(root, "mv", "a.md", "d.md")
            git(root, "add", "-A")
            git(root, "commit", "-q", "-m", "topic")
            self.assertEqual(sorted(rh.changed_files(base, root)), ["a.md", "d.md", "docs/b c.md"])
            self.assertEqual(rh.changed_files("HEAD", root), None)
            self.assertIsNone(rh.changed_files("no-such-commit", root))
            self.assertIsNone(rh.changed_files("", root))


if __name__ == "__main__":
    unittest.main()
