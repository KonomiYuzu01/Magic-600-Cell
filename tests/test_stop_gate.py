"""Stop-hook tests on a throwaway Git repository with synthetic hook events."""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
HOOK = ROOT / ".claude" / "hooks" / "stop_gate.py"


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=repo, check=True, capture_output=True)


class StopGateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name)
        (self.repo / "tools" / "agents").mkdir(parents=True)
        shutil.copy(ROOT / "tools" / "repo_digest.py", self.repo / "tools")
        shutil.copy(ROOT / "tools" / "agents" / "critical_paths.json", self.repo / "tools" / "agents")
        (self.repo / "core.py").write_text("x = 1\n")
        (self.repo / "README.md").write_text("readme\n")
        (self.repo / ".gitignore").write_text("__pycache__/\n")
        git(self.repo, "init", "-q")
        git(self.repo, "-c", "user.email=t@example.invalid", "-c", "user.name=t", "add", ".")
        git(self.repo, "-c", "user.email=t@example.invalid", "-c", "user.name=t", "commit", "-qm", "base")

    def tearDown(self):
        self.tmp.cleanup()

    def hook(self, **event):
        event.setdefault("session_id", "s1")
        env = dict(os.environ, CLAUDE_PROJECT_DIR=str(self.repo))
        r = subprocess.run([sys.executable, str(HOOK)], input=json.dumps(event), capture_output=True, text=True, env=env, timeout=20)
        self.assertEqual(r.returncode, 0, r.stderr)
        return json.loads(r.stdout) if r.stdout.strip() else None

    def identity(self) -> dict:
        code = "import json,sys;sys.path.insert(0,'tools');from repo_digest import source_identity;print(json.dumps(source_identity()))"
        return json.loads(subprocess.run([sys.executable, "-c", code], cwd=self.repo, capture_output=True, text=True, check=True).stdout)

    def write_review(self, identity: dict, verdict="pass", findings=(), dispositions=None, kind="review", valid=True, raw=None):
        d = self.repo / "work" / "reviews" / f"call-{len(list((self.repo / 'work' / 'reviews').glob('*'))) if (self.repo / 'work' / 'reviews').exists() else 0}"
        d.mkdir(parents=True)
        (d / "meta.json").write_text(json.dumps({"call_id": d.name, "kind": kind, "valid": valid, "source_identity": identity, "verdict": verdict}))
        full = [dict({"title": "t", "detail": "d", "evidence": [], "counterexample": None, "suggested_experiment": None,
                      "verification_status": "verified"}, **f) for f in findings]
        (d / "review.json").write_text(json.dumps(raw if raw is not None else {"verdict": verdict, "summary": "", "findings": full}))
        if dispositions is not None:
            (d / "dispositions.json").write_text(json.dumps(dispositions))

    def test_no_changes_allows_stop(self):
        self.assertIsNone(self.hook())

    def test_non_critical_change_allows_stop(self):
        (self.repo / "README.md").write_text("changed\n")
        self.assertIsNone(self.hook())

    def test_unreviewed_critical_change_blocks_twice_then_allows(self):
        (self.repo / "core.py").write_text("x = 2\n")
        first, second, third = self.hook(), self.hook(), self.hook()
        self.assertEqual(first["decision"], "block")
        self.assertEqual(second["decision"], "block")
        self.assertNotIn("decision", third)
        self.assertIn("inconclusive", third["systemMessage"])

    def test_budget_survives_new_digests(self):
        for value in ("2", "3", "4"):
            (self.repo / "core.py").write_text(f"x = {value}\n")
            result = self.hook()
        self.assertNotIn("decision", result)  # the third block is refused although the candidate changed

    def test_stop_hook_active_never_blocks(self):
        (self.repo / "core.py").write_text("x = 2\n")
        self.assertIsNone(self.hook(stop_hook_active=True))

    def test_staged_and_untracked_critical_files_count(self):
        (self.repo / "session.py").write_text("new\n")
        self.assertEqual(self.hook(session_id="a")["decision"], "block")
        git(self.repo, "add", "session.py")
        self.assertEqual(self.hook(session_id="b")["decision"], "block")

    def test_valid_review_of_current_candidate_allows(self):
        (self.repo / "core.py").write_text("x = 2\n")
        self.write_review(self.identity())
        self.assertIsNone(self.hook())

    def test_review_of_older_candidate_does_not_count(self):
        (self.repo / "core.py").write_text("x = 2\n")
        self.write_review(self.identity())
        (self.repo / "core.py").write_text("x = 3\n")
        self.assertEqual(self.hook()["decision"], "block")

    def test_review_records_do_not_change_identity(self):
        (self.repo / "core.py").write_text("x = 2\n")
        before = self.identity()["digest"]
        self.write_review({"digest": "unrelated"})
        self.assertEqual(self.identity()["digest"], before)

    def test_blocking_findings_need_evidence_backed_rejection(self):
        (self.repo / "core.py").write_text("x = 2\n")
        finding = {"id": "F1", "severity": "blocker"}
        self.write_review(self.identity(), verdict="findings", findings=[finding], dispositions={"F1": "adopt"})
        self.assertEqual(self.hook(session_id="x")["decision"], "block")
        self.write_review(self.identity(), verdict="findings", findings=[finding], dispositions={"F1": "reject_with_evidence"})
        self.assertIsNone(self.hook(session_id="y"))

    def test_malformed_or_contradictory_pass_does_not_count(self):
        (self.repo / "core.py").write_text("x = 2\n")
        self.write_review(self.identity(), raw={"verdict": "pass"})
        self.assertEqual(self.hook(session_id="m")["decision"], "block")
        self.write_review(self.identity(), verdict="pass", findings=[{"id": "F1", "severity": "blocker"}])
        self.assertEqual(self.hook(session_id="n")["decision"], "block")

    def test_staged_content_is_part_of_the_identity(self):
        (self.repo / "core.py").write_text("x = 2\n")
        git(self.repo, "add", "core.py")
        (self.repo / "core.py").write_text("x = 3\n")
        reviewed = self.identity()["digest"]
        git(self.repo, "add", "core.py")  # now staged content equals the working tree
        (self.repo / "core.py").write_text("x = 3\n")
        self.assertNotEqual(self.identity()["digest"], reviewed)

    def test_hook_identity_matches_repo_digest(self):
        (self.repo / "core.py").write_text("x = 2\n")
        (self.repo / "session.py").write_text("s\n")
        git(self.repo, "add", "session.py")
        code = ("import json,sys,runpy;sys.argv=['x'];"
                f"g=runpy.run_path({str(HOOK)!r});print(json.dumps(g['source_identity'](5)))")
        env = dict(os.environ, CLAUDE_PROJECT_DIR=str(self.repo))
        hook_identity = json.loads(subprocess.run([sys.executable, "-c", code], cwd=self.repo, env=env, capture_output=True, text=True, check=True).stdout)
        self.assertEqual(hook_identity, self.identity())

    def test_invalid_or_plan_records_do_not_count(self):
        (self.repo / "core.py").write_text("x = 2\n")
        self.write_review(self.identity(), valid=False)
        self.write_review(self.identity(), kind="plan")
        self.assertEqual(self.hook()["decision"], "block")

    def test_concurrent_stops_share_one_budget(self):
        (self.repo / "core.py").write_text("x = 2\n")
        env = dict(os.environ, CLAUDE_PROJECT_DIR=str(self.repo))
        procs = [subprocess.Popen([sys.executable, str(HOOK)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, env=env)
                 for _ in range(6)]
        outputs = [p.communicate(json.dumps({"session_id": "c"}), timeout=30)[0] for p in procs]
        self.assertEqual([p.returncode for p in procs], [0] * 6)
        blocks = [o for o in outputs if o.strip() and json.loads(o).get("decision") == "block"]
        self.assertEqual(len(blocks), 2)

    def test_schema_invalid_findings_do_not_count(self):
        (self.repo / "core.py").write_text("x = 2\n")
        bad = {"id": "F1", "severity": "minor", "title": 0, "evidence": "invalid", "verification_status": "invalid"}
        self.write_review(self.identity(), verdict="pass", findings=[bad])
        self.assertEqual(self.hook(session_id="v")["decision"], "block")
        evidence = {"id": "F2", "severity": "minor", "evidence": [{"path": "core.py", "line": True, "sha256": None}]}
        self.write_review(self.identity(), verdict="pass", findings=[evidence])
        self.assertEqual(self.hook(session_id="w")["decision"], "block")
        good = {"id": "F3", "severity": "minor", "evidence": [{"path": "core.py", "line": 1, "sha256": None}]}
        self.write_review(self.identity(), verdict="pass", findings=[good])
        self.assertIsNone(self.hook(session_id="z"))

    def test_inline_schema_matches_the_published_schema(self):
        import runpy
        env_saved = os.environ.get("CLAUDE_PROJECT_DIR")
        g = runpy.run_path(str(HOOK))
        published = json.loads((ROOT / "schemas" / "review-result.schema.json").read_text(encoding="utf-8"))

        for key in ("$schema", "title", "description"):
            published.pop(key)
        self.assertEqual(published, g["REVIEW_SCHEMA"])
        self.assertEqual(env_saved, os.environ.get("CLAUDE_PROJECT_DIR"))

    def test_symlink_replacement_changes_the_identity(self):
        (self.repo / "core.py").write_text("x = 2\n")
        before = self.identity()["digest"]
        outside = Path(self.tmp.name + "-outside.py")
        outside.write_text("x = 2\n")
        self.addCleanup(outside.unlink)
        (self.repo / "core.py").unlink()
        try:
            os.symlink(outside, self.repo / "core.py")
        except (OSError, NotImplementedError):
            self.skipTest("symlinks not available")
        linked = self.identity()
        self.assertNotEqual(linked["digest"], before)
        other = Path(self.tmp.name + "-other.py")
        other.write_text("x = 2\n")
        self.addCleanup(other.unlink)
        (self.repo / "core.py").unlink()
        os.symlink(other, self.repo / "core.py")
        self.assertNotEqual(self.identity()["digest"], linked["digest"])  # retargeting counts
        (self.repo / "core.py").unlink()
        os.symlink(self.repo / "missing.py", self.repo / "core.py")  # dangling link
        self.identity()
        code = ("import json,sys,runpy;sys.argv=['x'];"
                f"g=runpy.run_path({str(HOOK)!r});print(json.dumps(g['source_identity'](5)))")
        env = dict(os.environ, CLAUDE_PROJECT_DIR=str(self.repo))
        hook_identity = json.loads(subprocess.run([sys.executable, "-c", code], cwd=self.repo, env=env, capture_output=True, text=True, check=True).stdout)
        self.assertEqual(hook_identity, self.identity())

    def test_linked_ancestor_directories_are_never_read_through(self):
        (self.repo / "native").mkdir()
        (self.repo / "native" / "Host.cs").write_text("inside\n")
        git(self.repo, "add", "native")
        git(self.repo, "-c", "user.email=t@example.invalid", "-c", "user.name=t", "commit", "-qm", "native")
        outside = Path(self.tmp.name + "-native")
        outside.mkdir()
        self.addCleanup(shutil.rmtree, outside)
        (outside / "Host.cs").write_text("external\n")
        shutil.rmtree(self.repo / "native")
        try:
            os.symlink(outside, self.repo / "native", target_is_directory=True)
        except (OSError, NotImplementedError):
            self.skipTest("symlinks not available")
        first = self.identity()
        (outside / "Host.cs").write_text("changed external\n")
        self.assertEqual(self.identity(), first)  # external content is never hashed
        code = ("import json,sys,runpy;sys.argv=['x'];"
                f"g=runpy.run_path({str(HOOK)!r});print(json.dumps(g['source_identity'](5)))")
        env = dict(os.environ, CLAUDE_PROJECT_DIR=str(self.repo))
        hook_identity = json.loads(subprocess.run([sys.executable, "-c", code], cwd=self.repo, env=env, capture_output=True, text=True, check=True).stdout)
        self.assertEqual(hook_identity, first)

    def test_malformed_local_state_is_inconclusive(self):
        (self.repo / "core.py").write_text("x = 2\n")
        state = self.repo / "work" / "loop-memory" / "state" / "stop_gate.json"
        state.parent.mkdir(parents=True)
        for raw in ("[]", '{"s1": "broken"}', '{"s1": -1}', "{bad"):
            with self.subTest(raw=raw):
                state.write_text(raw)
                out = self.hook()
                self.assertNotIn("decision", out)
                self.assertIn("inconclusive", out["systemMessage"])
        state.unlink()
        self.write_review(self.identity(), verdict="findings", findings=[{"id": "F1", "severity": "blocker"}], dispositions=["F1"])
        self.assertEqual(self.hook(session_id="d")["decision"], "block")

    def test_malformed_input_and_configuration_are_inconclusive(self):
        (self.repo / "core.py").write_text("x = 2\n")
        env = dict(os.environ, CLAUDE_PROJECT_DIR=str(self.repo))
        r = subprocess.run([sys.executable, str(HOOK)], input="[]", capture_output=True, text=True, env=env, timeout=20)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("inconclusive", json.loads(r.stdout)["systemMessage"])
        (self.repo / "tools" / "agents" / "critical_paths.json").write_text('{"patterns": true}')
        out = self.hook()
        self.assertNotIn("decision", out)
        self.assertIn("inconclusive", out["systemMessage"])

    def test_unreadable_input_is_inconclusive(self):
        env = dict(os.environ, CLAUDE_PROJECT_DIR=str(self.repo))
        r = subprocess.run([sys.executable, str(HOOK)], input="{not json", capture_output=True, text=True, env=env, timeout=20)
        self.assertEqual(r.returncode, 0)
        self.assertIn("inconclusive", json.loads(r.stdout)["systemMessage"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
