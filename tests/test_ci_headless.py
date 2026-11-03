"""Tests for the check selection of tools/ci/run_headless.py."""
from __future__ import annotations

import json
import os
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


class ShardTests(unittest.TestCase):
    def test_shards_partition_the_checks_in_list_order(self):
        for count in (1, 2, 5, 7):
            with self.subTest(count=count):
                parts = rh.shards(rh.CHECKS, count)
                self.assertEqual(len(parts), count)
                flat = [check for part in parts for check in part]
                self.assertEqual(sorted(map(" ".join, flat)), sorted(map(" ".join, rh.CHECKS)))
                position = {" ".join(check): n for n, check in enumerate(rh.CHECKS)}
                for part in parts:
                    self.assertEqual([position[" ".join(c)] for c in part],
                                     sorted(position[" ".join(c)] for c in part))
                self.assertEqual(parts, rh.shards(rh.CHECKS, count))

    def test_full_shards_are_balanced(self):
        weight = lambda part: sum(rh.SECONDS.get(c[0], rh.DEFAULT_SECONDS) for c in part)
        parts = rh.shards(rh.CHECKS, rh.FULL_SHARDS)
        total = sum(weight(part) for part in parts)
        longest = max(rh.SECONDS.values())
        self.assertLessEqual(max(weight(part) for part in parts), max(longest, total / rh.FULL_SHARDS) + longest)
        # The three longest checks land in different shards.
        heavy = sorted(rh.SECONDS, key=rh.SECONDS.get)[-3:]
        homes = {n for n, part in enumerate(parts) for c in part if c[0] in heavy}
        self.assertEqual(len(homes), 3)

    def test_more_shards_than_checks_leave_empty_shards(self):
        parts = rh.shards([["tests/a.py"], ["tests/b.py"]], 3)
        self.assertEqual(sum(map(len, parts)), 2)
        self.assertIn([], parts)

    def test_shard_counts_and_parsing(self):
        self.assertEqual(rh.shard_count("full"), rh.FULL_SHARDS)
        self.assertEqual(rh.shard_count("docs-only"), 1)
        self.assertEqual(rh.parse_shard("2/5"), (2, 5))
        for text in ("0/5", "6/5", "5", "a/b", "1/0", "-1/2"):
            with self.subTest(text=text), self.assertRaises(Exception):
                rh.parse_shard(text)

    def run_script(self, *args, base=""):
        env = dict(os.environ, RUN_HEADLESS_BASE=base)
        return subprocess.run([sys.executable, str(ROOT / "tools/ci/run_headless.py"), *args], cwd=ROOT, env=env,
                              capture_output=True, text=True)

    def test_plan_github_without_a_base_is_a_full_plan(self):
        result = self.run_script("--plan-github")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(),
                         ["mode=full", f"count={rh.FULL_SHARDS}",
                          "shards=" + json.dumps(list(range(1, rh.FULL_SHARDS + 1)))])
        self.assertIn("Mode: full", result.stderr)

    def test_a_shard_plan_and_a_mode_mismatch(self):
        result = self.run_script("--plan", "--shard", f"1/{rh.FULL_SHARDS}", "--expect-mode", "full")
        self.assertEqual(result.returncode, 0, result.stderr)
        listed = result.stdout.splitlines()[2:]
        self.assertEqual(listed, [" ".join(c) for c in rh.shards(rh.CHECKS, rh.FULL_SHARDS)[0]])
        mismatch = self.run_script("--plan", "--expect-mode", "docs-only")
        self.assertEqual(mismatch.returncode, 2)
        self.assertIn("the plan chose docs-only mode", mismatch.stderr)


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
