"""Tests for the check selection of tools/ci/run_headless.py."""
from __future__ import annotations

import contextlib
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools" / "ci"))
import run_headless as rh  # noqa: E402
import transcripts  # noqa: E402


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

    def test_no_diff_and_ci_changes_run_every_check(self):
        for changed in (None, [], ["tools/ci/run_headless.py"], ["docs/wiki/log.md", ".github/workflows/x.yml"]):
            with self.subTest(changed=changed):
                mode, _, checks = self.mode(changed)
                self.assertEqual(mode, "full")
                self.assertIs(checks, rh.CHECKS)

    def test_anything_else_runs_every_check_but_heavy_ones_outside_its_area(self):
        light = [c for c in rh.CHECKS if c[0] not in rh.HEAVY_AREAS]
        for changed in (["docs/wiki/data.json"], ["docs/README.md"], ["README.md"], ["AGENTS.md"],
                        ["docs/progress/status.json"], ["docs/wiki"], ["docs/wikix/a.md"], ["renderer/shell/x.cs"]):
            with self.subTest(changed=changed):
                mode, reason, checks = self.mode(changed)
                self.assertEqual(mode, "pr")
                self.assertEqual(checks, light)
                self.assertIn(f"{len(rh.HEAVY_AREAS)} heavy check(s)", reason)

    def test_heavy_checks_follow_their_area(self):
        def heavy(changed):
            return {c[0] for c in self.mode(changed)[2] if c[0] in rh.HEAVY_AREAS}
        rust = {"tests/test_language_rust.py"} | {p for p in rh.HEAVY_AREAS if "rust_" in p or "command_route" in p}
        j1 = set(rh.HEAVY_AREAS) - {"tests/test_language_rust.py", "tests/test_language_rust_checkers.py"}
        self.assertEqual(heavy(["rust/c600-kernel/src/lib.rs"]), rust)
        self.assertEqual(heavy(["research/jumbling/differential/x.py"]), j1)
        self.assertEqual(heavy(["docs/wiki/log.md", "core.py"]), j1)
        self.assertEqual(heavy(["assets/census.json"]), j1)
        self.assertEqual(heavy(["tests/test_jumbling_x1_witness.py"]), {"tests/test_jumbling_x1_witness.py"})
        self.assertEqual(heavy(["docs/progress/1.0/jumbling-midstage.md"]), {"tests/test_jumbling_word_check.py"})
        self.assertEqual(heavy(["rusty/a.rs", "core.pyx"]), set())

    def test_heavy_checks_are_listed_and_not_light(self):
        listed = {check[0] for check in rh.CHECKS}
        light = {check[0] for check in rh.LIGHT_CHECKS}
        for path in rh.HEAVY_AREAS:
            with self.subTest(path=path):
                self.assertIn(path, listed)
                self.assertNotIn(path, light)
        # The slowest checks of the run on main are heavy.
        for path in ("tests/test_jumbling_store_j1.py", "tests/test_jumbling_j1_replay.py",
                     "tests/test_jumbling_x1_witness.py", "tests/test_language_rust.py",
                     "tests/test_language_rust_checkers.py"):
            self.assertIn(path, rh.HEAVY_AREAS)

    def test_markdown_that_a_full_check_reads_runs_the_pull_request_checks(self):
        for path in ("docs/progress/1.0/jumbling-midstage.md", "docs/progress/1.0/packets/renderer/README.md",
                     "docs/progress/1.0/inventory/notes.md", "docs/progress/1.0/dispositions/d.md"):
            with self.subTest(path=path):
                self.assertEqual(self.mode(["docs/wiki/log.md", path])[0], "pr")

    def test_a_path_named_in_a_full_check_runs_the_pull_request_checks(self):
        self.assertEqual(self.mode(["docs/progress/1.0/new.md"], {"docs/progress/1.0/new.md"})[0], "pr")
        self.assertEqual(self.mode(["docs/progress/1.0/x/y.md"], {"docs/progress/1.0/x"})[0], "pr")
        # A literal that only shares a prefix with the path does not count.
        self.assertEqual(self.mode(["docs/progress/1.0/xy.md"], {"docs/progress/1.0/x"})[0], "docs-only")

    def test_light_checks_are_listed_checks(self):
        self.assertTrue(all(check in rh.CHECKS for check in rh.LIGHT_CHECKS))
        self.assertIn(["tests/test_ci_headless.py"], rh.LIGHT_CHECKS)

    def test_the_frozen_format_packet_is_checked_on_documentation_changes(self):
        # Every pin mismatch is fatal (FP.1), and the packet pins Markdown in the documentation trees.
        packet = json.loads((ROOT / "docs/architecture/1.0/language/format-packet/candidate.json").read_bytes())
        self.assertTrue([item["path"] for item in packet["inputs"] if item["path"].startswith(rh.DOCS_ONLY_TREES)])
        self.assertIn(["docs/architecture/1.0/language/format-packet/check_vectors.py"], rh.LIGHT_CHECKS)
        for check in rh.LANGUAGE_DOCS_CHECKS:
            self.assertTrue((ROOT / check[0]).is_file(), check)

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

    def test_a_long_check_has_its_own_limit(self):
        self.assertEqual(rh.time_limit(["tests/test_core.py"]), rh.TIME_LIMIT_SECONDS)
        listed = {check[0] for check in rh.CHECKS}
        for path, limit in rh.LONG_CHECK_SECONDS.items():
            with self.subTest(path=path):
                self.assertIn(path, listed)
                self.assertGreater(limit, rh.TIME_LIMIT_SECONDS)
                self.assertEqual(rh.time_limit([path]), limit)
                # The check stops itself first, so a slow run is reported by the check, and its measured
                # time is below both limits.
                text = (ROOT / path).read_text(encoding="utf-8")
                own = int(re.search(r"^TIME_LIMIT_SECONDS = (\d+)$", text, re.M).group(1))
                self.assertLess(own, limit)
                self.assertLess(rh.SECONDS[path], own)

    def test_more_shards_than_checks_leave_empty_shards(self):
        parts = rh.shards([["tests/a.py"], ["tests/b.py"]], 3)
        self.assertEqual(sum(map(len, parts)), 2)
        self.assertIn([], parts)

    def test_shard_counts_and_parsing(self):
        self.assertEqual(rh.shard_count("full"), rh.FULL_SHARDS)
        self.assertEqual(rh.shard_count("pr"), rh.FULL_SHARDS)
        self.assertEqual(rh.shard_count("docs-only"), 1)
        self.assertEqual(rh.parse_shard("2/5"), (2, 5))
        for text in ("0/5", "6/5", "5", "a/b", "1/0", "-1/2"):
            with self.subTest(text=text), self.assertRaises(Exception):
                rh.parse_shard(text)

    def run_script(self, *args, base=""):
        env = dict(os.environ, RUN_HEADLESS_BASE=base, RUN_HEADLESS_TIER="")
        return subprocess.run([sys.executable, str(ROOT / "tools/ci/run_headless.py"), *args], cwd=ROOT, env=env,
                              capture_output=True, text=True)

    def test_plan_github_without_a_base_is_a_full_plan(self):
        result = self.run_script("--plan-github")
        self.assertEqual(result.returncode, 0, result.stderr)
        comparisons = [c[0][len("tests/test_jumbling_rust_"):-3] for c in rh.CHECKS
                       if c[0].startswith("tests/test_jumbling_rust_")]
        self.assertEqual(len(comparisons), 6)
        self.assertEqual(result.stdout.splitlines(),
                         ["mode=full", f"count={rh.FULL_SHARDS}",
                          "shards=" + json.dumps(list(range(1, rh.FULL_SHARDS + 1))),
                          "rust=true", "packages=--workspace", "j1=" + json.dumps(comparisons)])
        self.assertIn("Mode: full", result.stderr)
        self.assertNotIn("Deferred", result.stderr)

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


class FastTierTests(unittest.TestCase):
    """The fast tier (owner, 26 November 2026): pr mode defers the checks recorded over the budget."""

    def plan(self, changed):
        return rh.plan(changed, set())

    def test_pr_mode_defers_the_selected_checks_over_the_budget(self):
        changed = ["rust/c600-kernel/src/lib.rs", "core.py"]
        _, _, selected = rh.select_checks(changed, set())
        mode, reason, checks, deferred = self.plan(changed)
        self.assertEqual(mode, "pr")
        self.assertTrue(deferred)
        self.assertIn(f"{len(deferred)} check(s) over {rh.FAST_BUDGET_SECONDS} s deferred to the full tier", reason)
        # The two parts split the path selection and keep its order.
        self.assertEqual(sorted(checks + deferred), sorted(selected))
        self.assertEqual(checks, [c for c in selected if c not in deferred])
        for check in deferred:
            self.assertGreater(rh.SECONDS[check[0]], rh.FAST_BUDGET_SECONDS)
        for check in checks:
            self.assertLessEqual(rh.SECONDS.get(check[0], 0), rh.FAST_BUDGET_SECONDS)
        # The whole-workspace cargo checks and the slowest J1 checks wait for the full tier.
        for path in ("tests/test_language_rust.py", "tests/test_language_rust_checkers.py",
                     "tests/test_jumbling_store_j1.py", "tests/test_jumbling_rust_candidate.py"):
            self.assertIn([path], deferred)

    def test_deferral_follows_the_recorded_time(self):
        budget = rh.FAST_BUDGET_SECONDS
        with mock.patch.dict(rh.SECONDS, {"tests/test_oracle.py": budget + 1, "tests/test_v02.py": budget}):
            _, _, checks, deferred = self.plan(["renderer/shell/x.cs"])
        self.assertEqual(deferred, [["tests/test_oracle.py"]])
        self.assertIn(["tests/test_v02.py"], checks)
        # A check with no recorded time stays in the fast tier.
        unrecorded = [c for c in rh.select_checks(["renderer/shell/x.cs"], set())[2] if c[0] not in rh.SECONDS]
        self.assertTrue(unrecorded)
        self.assertTrue(all(check in checks for check in unrecorded))

    def test_per_change_checks_stay_however_long_they_take(self):
        listed = {check[0] for check in rh.CHECKS}
        self.assertLessEqual(rh.PER_CHANGE_CHECKS, listed)
        slow = {path: 10 * rh.FAST_BUDGET_SECONDS for path in rh.PER_CHANGE_CHECKS}
        with mock.patch.dict(rh.SECONDS, slow):
            _, _, checks, deferred = self.plan(["session.py"])
        self.assertEqual(deferred, [])
        for path in rh.PER_CHANGE_CHECKS:
            self.assertIn([path], checks)
        for path in ("tests/test_core.py", "tests/test_reference_maps.py", "tests/test_crash.py",
                     "tests/test_engine_lifecycle.py", "tests/test_agent_rules_sync.py", "tests/test_codex_review.py",
                     "tests/test_stop_gate.py", "tests/test_bootstrap.py", "tests/test_wiki_lint.py",
                     "tests/test_workbench.py"):
            self.assertIn(path, rh.PER_CHANGE_CHECKS)

    def test_the_full_and_light_runs_defer_nothing(self):
        for changed in (None, ["tools/ci/run_headless.py", "rust/Cargo.lock"]):
            mode, _, checks, deferred = self.plan(changed)
            self.assertEqual((mode, deferred), ("full", []))
            self.assertIs(checks, rh.CHECKS)
        mode, _, checks, deferred = self.plan(["docs/wiki/log.md"])
        self.assertEqual((mode, deferred), ("docs-only", []))
        self.assertIs(checks, rh.LIGHT_CHECKS)

    def test_a_python_change_defers_nothing_outside_the_heavy_areas(self):
        mode, _, checks, deferred = self.plan(["session.py"])
        self.assertEqual((mode, deferred), ("pr", []))
        self.assertEqual(checks, [c for c in rh.CHECKS if c[0] not in rh.HEAVY_AREAS])


class RustPackagesTests(unittest.TestCase):
    """The fast tier's cargo test runs the workspace members that hold a changed file."""

    def test_one_crate_changed(self):
        self.assertEqual(rh.rust_packages(["rust/c600-kernel/src/lib.rs"]), ["c600-kernel"])
        # A member whose name starts like another's is told apart, and each package is named once.
        self.assertEqual(rh.rust_packages(["rust/c600-jumble-tool/src/a.rs", "rust/c600-jumble/src/b.rs",
                                           "rust/c600-jumble/tests/c.rs", "core.py"]),
                         ["c600-jumble", "c600-jumble-tool"])
        self.assertEqual(rh.rust_packages(["rust/c600-ir/README.md"]), ["c600-ir"])

    def test_a_workspace_file_runs_the_whole_workspace(self):
        for path in ("rust/Cargo.toml", "rust/Cargo.lock", "rust/.cargo/config.toml",
                     "rust/vendor/num-bigint/src/lib.rs", "rust/rustfmt.toml", "rust/new-file.rs"):
            with self.subTest(path=path):
                self.assertIsNone(rh.rust_packages(["rust/c600-kernel/src/lib.rs", path]))

    def test_a_change_outside_the_rust_code_runs_no_cargo_test(self):
        for changed in (["core.py"], ["rusty/a.rs", "docs/wiki/log.md"], ["tests/test_language_rust.py"],
                        ["rust/README.md"]):
            with self.subTest(changed=changed):
                self.assertEqual(rh.rust_packages(changed), [])

    def test_members_are_read_from_the_manifests(self):
        members = rh.rust_members()
        self.assertIsNotNone(members)
        self.assertIn("rust/c600-kernel/", members)
        for directory, name in members.items():
            self.assertTrue((ROOT / directory / "Cargo.toml").is_file(), directory)
            self.assertIn(f'name = "{name}"', (ROOT / directory / "Cargo.toml").read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "rust" / "crates" / "alpha").mkdir(parents=True)
            (root / "rust" / "beta").mkdir()
            (root / "rust" / "Cargo.toml").write_text('[workspace]\nmembers = ["crates/alpha", "beta"]\n',
                                                      encoding="utf-8")
            (root / "rust" / "crates" / "alpha" / "Cargo.toml").write_text('[package]\nname = "alpha-pkg"\n',
                                                                            encoding="utf-8")
            (root / "rust" / "beta" / "Cargo.toml").write_text('[package]\nname = "beta"\n', encoding="utf-8")
            self.assertEqual(rh.rust_packages(["rust/crates/alpha/src/lib.rs"], root), ["alpha-pkg"])
            # An unusual package name or an unreadable manifest runs the whole workspace.
            (root / "rust" / "beta" / "Cargo.toml").write_text('[package]\nname = "beta; true"\n', encoding="utf-8")
            self.assertIsNone(rh.rust_packages(["rust/crates/alpha/src/lib.rs"], root))
            (root / "rust" / "beta" / "Cargo.toml").unlink()
            self.assertIsNone(rh.rust_packages(["rust/crates/alpha/src/lib.rs"], root))

    def test_linux_jobs(self):
        budget = rh.FAST_BUDGET_SECONDS
        everything = [c[0][len("tests/test_jumbling_rust_"):-3] for c in rh.CHECKS
                      if c[0].startswith("tests/test_jumbling_rust_")]
        fast = [name for name in everything if rh.SECONDS.get(f"tests/test_jumbling_rust_{name}.py", 0) <= budget]
        self.assertTrue(fast)
        self.assertLess(len(fast), len(everything))
        self.assertEqual(rh.linux_jobs("full", None, rh.CHECKS),
                         {"rust": "true", "packages": "--workspace", "j1": json.dumps(everything)})
        for changed, rust, packages, j1 in (
                (["rust/c600-kernel/src/lib.rs"], "true", "-p c600-kernel", fast),
                (["rust/c600-ir/src/a.rs", "rust/c600-kernel/src/b.rs"], "true", "-p c600-ir -p c600-kernel", fast),
                (["rust/Cargo.lock"], "true", "--workspace", fast),
                (["core.py"], "false", "", fast),
                (["session.py"], "false", "", []),
                (["docs/wiki/log.md"], "false", "", [])):
            with self.subTest(changed=changed):
                mode, _, checks, _ = rh.plan(changed, set())
                self.assertEqual(rh.linux_jobs(mode, changed, checks),
                                 {"rust": rust, "packages": packages, "j1": json.dumps(j1)})


class DispatchTierTests(unittest.TestCase):
    """A manual run's tier input: full (the default) runs every check, fast diffs the branch against main."""

    def test_the_base_for_each_tier(self):
        self.assertEqual(rh.plan_base("full", "abc"), "")
        self.assertEqual(rh.plan_base("full", ""), "")
        self.assertEqual(rh.plan_base("fast", ""), rh.DISPATCH_BASE)
        self.assertEqual(rh.plan_base("fast", "abc"), "abc")
        self.assertEqual(rh.plan_base("", "abc"), "abc")  # a pull request
        self.assertEqual(rh.plan_base("", ""), "")  # a push to main or the nightly run

    def main(self, *argv, changed=("session.py",), tier=""):
        bases = []

        def changed_files(base):
            bases.append(base)
            return list(changed) if base else None

        out, err = io.StringIO(), io.StringIO()
        with mock.patch.object(rh, "changed_files", changed_files), \
                mock.patch.dict(os.environ, {"RUN_HEADLESS_BASE": "", "RUN_HEADLESS_TIER": tier}), \
                mock.patch.object(sys, "argv", ["run_headless.py", *argv]), \
                contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = rh.main()
        outputs = dict(line.split("=", 1) for line in out.getvalue().splitlines() if "=" in line)
        return code, bases, outputs, err.getvalue()

    def test_the_tier_chooses_the_mode(self):
        self.assertEqual(self.main("--plan-github", "--base", "abc")[2]["mode"], "pr")
        code, bases, outputs, _ = self.main("--plan-github", "--base", "abc", "--tier", "full")
        self.assertEqual((code, bases, outputs["mode"], outputs["packages"]), (0, [""], "full", "--workspace"))
        code, bases, outputs, _ = self.main("--plan-github", "--tier", "fast")
        self.assertEqual((code, bases, outputs["mode"]), (0, [rh.DISPATCH_BASE], "pr"))
        self.assertEqual(self.main("--plan-github")[2]["mode"], "full")

    def test_the_tier_comes_from_the_environment_and_is_checked(self):
        # The workflows pass the manual run's input as RUN_HEADLESS_TIER.
        self.assertEqual(self.main("--plan-github", "--base", "abc", tier="full")[2]["mode"], "full")
        self.assertEqual(self.main("--plan-github", tier="fast")[1], [rh.DISPATCH_BASE])
        for argv, tier in ((["--tier", "slow"], ""), ([], "Full")):
            with self.subTest(argv=argv, tier=tier), self.assertRaises(SystemExit) as raised:
                self.main("--plan-github", *argv, tier=tier)
            self.assertEqual(raised.exception.code, 2)

    def test_the_plan_prints_the_deferred_checks(self):
        code, _, _, err = self.main("--plan-github", "--base", "abc", changed=["rust/c600-kernel/src/lib.rs"])
        self.assertEqual(code, 0)
        self.assertIn("Deferred to the full tier: ", err)
        self.assertIn(f"tests/test_language_rust.py ({rh.SECONDS['tests/test_language_rust.py']} s)", err)


class TranscriptTests(unittest.TestCase):
    """tools/ci/transcripts.py: what the cross-platform job keeps and how it compares two systems' folders."""

    def main(self, argv):
        with contextlib.redirect_stdout(io.StringIO()):
            return transcripts.main(argv)

    def write(self, folder, name, data):
        path = folder / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)

    def test_keep_copies_the_work_folder_only_when_asked(self):
        with tempfile.TemporaryDirectory() as temporary:
            work, kept = Path(temporary) / "work", Path(temporary) / "kept"
            self.write(work, "a.trace.json", b"{}\n")
            self.write(work, "sub/receipts.json", b"[]\n")
            saved = os.environ.pop(transcripts.ENVIRONMENT, None)
            try:
                transcripts.keep("check", work)
                self.assertFalse(kept.exists())
                os.environ[transcripts.ENVIRONMENT] = str(kept)
                transcripts.keep("check", work)
            finally:
                os.environ.pop(transcripts.ENVIRONMENT, None)
                if saved is not None:
                    os.environ[transcripts.ENVIRONMENT] = saved
            self.assertEqual(sorted(transcripts.files(kept)), ["check/a.trace.json", "check/sub/receipts.json"])
            self.assertEqual((kept / "check" / "sub" / "receipts.json").read_bytes(), b"[]\n")

    def test_compare_passes_only_identical_folders(self):
        with tempfile.TemporaryDirectory() as temporary:
            a, b = Path(temporary) / "a", Path(temporary) / "b"
            for folder in (a, b):
                self.write(folder, "x.jsonl", b"> {}\n< {}\n")
            self.assertEqual(transcripts.compare(a, b), ([], 1, 10))
            self.assertEqual(self.main(["compare", str(a), str(b)]), 0)
            self.write(b, "x.jsonl", b"> {}\n< {1}\n")
            self.write(b, "y.jsonl", b"")
            self.write(a, "y.jsonl", b"z")
            self.write(a, "z.jsonl", b"z")
            problems, count, _ = transcripts.compare(a, b)
            self.assertEqual(count, 2)
            self.assertEqual(problems[0], "only in a: z.jsonl")
            self.assertIn("x.jsonl: 10 bytes against 11; line 2: '< {}' against '< {1}'", problems)
            self.assertIn("y.jsonl: empty on b", problems)
            self.assertEqual(self.main(["compare", str(a), str(b)]), 1)
            self.assertEqual(transcripts.compare(Path(temporary) / "c", Path(temporary) / "d")[0],
                             ["no transcript on either system"])
            self.assertEqual(self.main(["compare", str(a)]), 2)
            self.assertEqual(self.main(["compare", str(a), str(Path(temporary) / "missing")]), 2)


if __name__ == "__main__":
    unittest.main()
