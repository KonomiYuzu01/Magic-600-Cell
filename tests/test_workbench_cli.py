"""Headless tests of the session CLIs ask.py and progress.py, with disposable synthetic fixtures only.

The tests in this file up to the marker pin the committed interface; the implementation
packet adds the rest (every validation rule, limits, atomic writes, git evidence)."""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

from test_workbench import Fixture, SID, WB, notes, paths
from test_workbench_home import png, sample_status
import checklist
import home

PNG = png(2, 2)


class CliFixtureTests(unittest.TestCase):
    def setUp(self):
        mkdir = os.mkdir

        def fixture_mkdir(path, mode=0o777, *, dir_fd=None):
            # Python 3.14's Windows 0700 ACL excludes the sandbox token; inherit fixture permissions.
            return mkdir(path, 0o777 if os.name == "nt" and mode == 0o700 else mode, dir_fd=dir_fd)

        with patch("tempfile._os.mkdir", fixture_mkdir):
            self.fx = Fixture()
        self.addCleanup(self.fx.cleanup)

    def cli(self, script, *args, cwd=None, sid=SID):
        env = {**os.environ, "CLAUDE_CODE_SESSION_ID": sid}
        # Run the copies inside the fixture so they resolve the fixture's main checkout.
        tools = self.fx.main / "tools" / "workbench"
        if not tools.is_dir():
            shutil.copytree(WB, tools, ignore=shutil.ignore_patterns("__pycache__", "qml"))
        return subprocess.run([sys.executable, str(tools / script), *args], capture_output=True, text=True,
                              cwd=str(cwd or self.fx.main), env=env, timeout=60)


class AskCliTests(CliFixtureTests):
    def test_ask_writes_one_record_readable_by_home(self):
        img = self.fx.main / "work" / "gallery" / "a.png"
        img.parent.mkdir(parents=True)
        img.write_bytes(PNG)
        r = self.cli("ask.py", "--question", "Ship it?", "--context", "One sentence.", "--kind", "approve",
                     "--attach", str(img), "--blocking")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertRegex(r.stdout.strip(), r"^ask \d{8}T\d{6}Z-[0-9a-f]{8}$")
        ask_id = r.stdout.split()[1]
        (path,) = (self.fx.data / "asks" / SID).glob("*.json")
        raw = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual((raw["schema"], raw["id"], raw["session_id"], raw["kind"], raw["blocking"], raw["options"]),
                         (1, ask_id, SID, "approve", True, ["Approve", "Reject"]))
        self.assertTrue(Path(raw["attachments"][0]).samefile(img))
        try:
            records, skipped = home.asks(self.fx.data, SID)
        except NotImplementedError:
            self.skipTest("home.asks is implemented in packet P1")
        self.assertEqual(skipped, 0)
        (rec,) = records
        self.assertEqual((rec["id"], rec["kind"], rec["attachments"][0]["index"]), (ask_id, "approve", 1))

    def test_wait_returns_once_the_answer_note_exists(self):
        r = self.cli("ask.py", "--question", "Ship it?", "--kind", "approve", "--blocking")
        ask_id = r.stdout.split()[1]
        r = self.cli("ask.py", "wait", ask_id, "--timeout", "1")
        self.assertEqual(r.returncode, 3, r.stderr)
        notes.append_note(self.fx.data, SID, f"answer to ask {ask_id}: Approve")
        r = self.cli("ask.py", "wait", ask_id, "--timeout", "5")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn("Approve", r.stdout)  # the answer arrives only through the owner-note channel

    def test_ask_refuses_invalid_input(self):
        outside = Path(self.fx.tmp.name) / "outside.png"
        outside.write_bytes(PNG)
        for args in ((),                                                           # no question
                     ("--question", "q" * (home.QUESTION_MAX_BYTES + 1)),
                     ("--question", "q", "--kind", "decision", "--option", "only one"),
                     ("--question", "q", "--attach", str(outside))):                   # outside every checkout
            r = self.cli("ask.py", *args)
            self.assertEqual(r.returncode, 2, args)
        self.assertFalse((self.fx.data / "asks").exists() and any((self.fx.data / "asks").rglob("*.json")))


class ProgressCliTests(CliFixtureTests):
    def status(self, root=None):
        """sample_status() in `root` (default the worktree), with the done item's evidence made resolvable there."""
        root = root or self.fx.wt
        (root / "tests").mkdir(parents=True, exist_ok=True)
        (root / "tests" / "evidence.txt").write_text("ok\n", encoding="utf-8")
        doc = sample_status()
        doc["steps"][0]["items"][0]["evidence"] = "tests/evidence.txt"
        prog = root / "docs" / "progress"
        prog.mkdir(parents=True, exist_ok=True)
        (prog / "status.json").write_bytes(checklist.dump(doc).replace("\n", "\r\n").encode("utf-8"))
        return prog / "status.json"

    def test_done_edits_only_the_current_checkout_and_keeps_the_layout(self):
        path = self.status()
        result = self.fx.wt / "tests" / "result.txt"
        result.write_text("ok\n", encoding="utf-8")
        before = path.read_bytes()
        r = self.cli("progress.py", "done", "0.4.1-2", "harness", "--evidence", "tests/result.txt", cwd=self.fx.wt)
        self.assertEqual(r.returncode, 0, r.stderr)
        after = path.read_bytes()
        self.assertEqual(before.count(b"\r\n"), after.count(b"\r\n"))
        doc = json.loads(after.decode("utf-8"))
        self.assertEqual(checklist.validate(doc), [])
        step = next(s for s in doc["steps"] if s["id"] == "0.4.1-2")
        item = next(i for i in step["items"] if i["id"] == "harness")
        self.assertEqual((item["done"], item["evidence"]), (True, "tests/result.txt"))
        self.assertEqual(step["status"], "in_progress")  # the first done item starts a not_started step
        self.assertEqual(after.decode("utf-8").replace("\r\n", "\n"), checklist.dump(doc))
        self.assertFalse((self.fx.main / "docs" / "progress" / "status.json").exists())

    def test_status_command_checks_the_items(self):
        path = self.status()
        before = path.read_bytes()
        r = self.cli("progress.py", "status", "0.4.1-1", "done", cwd=self.fx.wt)   # item b is open
        self.assertEqual(r.returncode, 2, r.stderr)
        self.assertEqual(path.read_bytes(), before)
        r = self.cli("progress.py", "status", "0.4.1-2", "blocked", cwd=self.fx.wt)
        self.assertEqual(r.returncode, 0, r.stderr)
        doc = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(next(s for s in doc["steps"] if s["id"] == "0.4.1-2")["status"], "blocked")

    def test_done_refuses_missing_evidence_and_unknown_items(self):
        path = self.status()
        before = path.read_bytes()
        for args in (("0.4.1-2", "harness", "--evidence", "tests/missing.txt"),
                     ("0.4.1-2", "no-such-item", "--evidence", "0123abc"),
                     ("0.4.1-2", "harness", "--evidence", "work/private.txt")):
            r = self.cli("progress.py", "done", *args, cwd=self.fx.wt)
            self.assertEqual(r.returncode, 2, args)
        self.assertEqual(path.read_bytes(), before)


# ---- implementation packet P2 adds its tests below this line ----


if __name__ == "__main__":
    unittest.main(verbosity=2)
