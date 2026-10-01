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
        records, skipped = home.asks(self.fx.data, SID)
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


import io
import time
from contextlib import redirect_stderr, redirect_stdout
from datetime import datetime, timezone

from test_workbench import at
import ask
import progress


def call_main(module, argv, cwd, sid=SID):
    """Run a CLI in process, with only a fixture cwd and session environment."""
    env = dict(os.environ)
    if sid is None:
        env.pop("CLAUDE_CODE_SESSION_ID", None)
    else:
        env["CLAUDE_CODE_SESSION_ID"] = sid
    out, err = io.StringIO(), io.StringIO()
    with patch.object(Path, "cwd", return_value=cwd), patch.dict(os.environ, env, clear=True), \
            redirect_stdout(out), redirect_stderr(err):
        code = module.main(argv)
    return subprocess.CompletedProcess(argv, code, out.getvalue(), err.getvalue())


def files_under(root):
    return {str(p.relative_to(root)): p.read_bytes() for p in root.rglob("*") if p.is_file()}


def link_directory(test, target: Path, link: Path) -> None:
    """A junction (Windows) or directory symlink at `link` to `target`; skips the test when neither works."""
    link.parent.mkdir(parents=True, exist_ok=True)
    try:
        import _winapi
        _winapi.CreateJunction(str(target), str(link))
        return
    except (ImportError, AttributeError, OSError):
        pass
    try:
        link.symlink_to(target, target_is_directory=True)
    except (OSError, NotImplementedError):
        test.skipTest("directory links are unavailable")


class AskValidationTests(CliFixtureTests):
    def record(self, **fields):
        args = dict(sid=SID, question="q", context="", options=[], attachments=[], kind=None,
                    blocking=False, roots=paths.checkouts(self.fx.main), now=at(60))
        args.update(fields)
        return ask.build_record(**args)

    def image(self, name, root=None):
        path = (root or self.fx.main) / "work" / "gallery" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(PNG)
        return path

    def rejected(self, argv, cwd=None, sid=SID):
        before = files_under(self.fx.data)
        r = call_main(ask, argv, cwd or self.fx.main, sid)
        self.assertEqual(r.returncode, 2, r)
        self.assertEqual(r.stdout, "")
        self.assertEqual(len(r.stderr.splitlines()), 1, r.stderr)
        self.assertNotIn("Traceback", r.stderr)
        self.assertEqual(files_under(self.fx.data), before)
        return r

    def test_session_validation_precedes_checkout_and_writes_nothing(self):
        r = self.rejected(["--question", "q"], cwd=self.fx.projects, sid=None)
        self.assertEqual(r.stderr.strip(),
                         "ask.py runs in a Claude Code session (CLAUDE_CODE_SESSION_ID is not set)")
        for sid in ("", "../escape", "a/b", "x" * 65):
            with self.subTest(sid=sid):
                self.rejected(["--question", "q"], cwd=self.fx.projects, sid=sid)
        r = self.cli("ask.py", "--question", "q", "--session", SID, sid="")
        self.assertEqual(r.returncode, 0, r.stderr)

    def test_no_checkout_rejected_without_script_location_fallback(self):
        r = self.cli("ask.py", "--question", "q", cwd=self.fx.projects)
        self.assertEqual(r.returncode, 2, r.stderr)
        self.assertEqual(len(r.stderr.splitlines()), 1)
        self.assertFalse(self.fx.data.exists())

    def test_kind_inference_and_approve_options(self):
        for options, expected in (([], "question"), (["A"], "question"), (["A", "B"], "decision")):
            self.assertEqual(self.record(options=options)["kind"], expected)
        self.assertEqual(self.record(kind="question", options=["A", "B"])["kind"], "question")
        self.assertEqual(self.record(kind="approve")["options"], ["Approve", "Reject"])
        self.assertEqual(self.record(kind="approve", options=["Ship"])["options"], ["Ship"])
        images = [str(self.image("a.png")), str(self.image("b.png"))]
        self.assertEqual(self.record(attachments=images)["kind"], "question")
        self.rejected(["--question", "q", "--kind", "unknown"])

    def test_text_rules_and_utf8_limits(self):
        cases = [
            ["--question", "  "], ["--question", "q\n"], ["--question", "\rq"],
            ["--question", "é" * (home.QUESTION_MAX_BYTES // 2 + 1)],
            ["--question", "q", "--context", "é" * (home.CONTEXT_MAX_BYTES // 2 + 1)],
            ["--question", "q", "--option", "é" * (home.OPTION_MAX_BYTES // 2 + 1)],
            ["--question", "q", "--option", " "],
            ["--question", "q", "--option", " A ", "--option", "A"],
            ["--question", "q", "--option", "A\n"], ["--question", "q", "--option", "\rA"],
            ["--question", "q", *sum((["--option", str(i)] for i in range(home.OPTIONS_MAX + 1)), [])],
        ]
        for argv in cases:
            with self.subTest(argv=argv):
                self.rejected(argv)
        record = self.record(question="  " + "é" * (home.QUESTION_MAX_BYTES // 2) + "  ",
                             context=" \n" + "é" * (home.CONTEXT_MAX_BYTES // 2) + "\n ",
                             options=[" " + "é" * (home.OPTION_MAX_BYTES // 2) + " "])
        self.assertEqual(len(record["question"].encode("utf-8")), home.QUESTION_MAX_BYTES)
        self.assertEqual(len(record["context"].encode("utf-8")), home.CONTEXT_MAX_BYTES)
        self.assertEqual(len(record["options"][0].encode("utf-8")), home.OPTION_MAX_BYTES)
        self.assertEqual(self.record(context=" \n context\nmore \n ")["context"], "context\nmore")
        self.assertEqual(len(self.record(options=[str(i) for i in range(home.OPTIONS_MAX)])["options"]), home.OPTIONS_MAX)

    def test_decision_and_pick_rules(self):
        a, b = self.image("a.PNG", self.fx.wt), self.image("b.jpg", self.fx.wt)
        text = self.image("c.txt", self.fx.wt)
        for args in (("--kind", "decision"), ("--kind", "decision", "--option", "one"),
                     ("--kind", "pick"), ("--kind", "pick", "--attach", str(a)),
                     ("--kind", "pick", "--attach", str(a), "--attach", str(text)),
                     ("--kind", "pick", "--attach", str(a), "--attach", str(b), "--option", "A")):
            with self.subTest(args=args):
                self.rejected(["--question", "q", *args])
        r = self.cli("ask.py", "--question", "Choose", "--kind", "pick", "--attach", str(a),
                     "--attach", str(b), cwd=self.fx.wt)
        self.assertEqual(r.returncode, 0, r.stderr)
        (file,) = (self.fx.data / "asks" / SID).glob("*.json")
        record = json.loads(file.read_bytes())
        self.assertEqual((record["kind"], record["options"], record["attachments"]),
                         ("pick", [], [str(a.resolve()), str(b.resolve())]))
        self.assertFalse(paths.data_root(self.fx.wt).exists())
        for ext in ask.PICK_EXT:
            extra = self.image("other" + ext, self.fx.ext)
            self.assertEqual(self.record(kind="pick", attachments=[str(a), str(extra)])["kind"], "pick")

    def test_attachment_validation_and_resolved_duplicates(self):
        a = self.image("a.png")
        outside = Path(self.fx.tmp.name) / "outside.png"
        outside.write_bytes(PNG)
        sibling = self.image("sibling.png", self.fx.outsider)
        duplicate = a.parent / ".." / "gallery" / a.name
        for attachments in ([str(a.parent)], [str(a.parent / "missing.png")], [str(outside)], [str(sibling)],
                            [str(a), str(duplicate)], [str(a)] * (home.ATTACHMENTS_MAX + 1)):
            with self.subTest(attachments=attachments):
                self.rejected(["--question", "q", *sum((["--attach", p] for p in attachments), [])])
        images = [self.image(f"{i}.png") for i in range(home.ATTACHMENTS_MAX)]
        self.assertEqual(self.record(attachments=[str(p) for p in images])["attachments"],
                         [str(p.resolve()) for p in images])

    def test_attachment_symlink_escape_when_supported(self):
        outside = Path(self.fx.tmp.name) / "outside.png"
        outside.write_bytes(PNG)
        link = self.fx.main / "escape.png"
        try:
            link.symlink_to(outside)
        except (OSError, NotImplementedError):
            self.skipTest("fixture symlinks are unavailable")
        self.rejected(["--question", "q", "--attach", str(link)])

    def test_record_fields_timestamp_utf8_and_atomic_write(self):
        image = self.image("a.png")
        with patch.object(ask.time, "time", return_value=at(60)), \
                patch.object(ask.secrets, "token_hex", return_value="0a0b0c0d") as token:
            r = call_main(ask, ["--question", "  Ship é?  ", "--context", " \nContext.\n ", "--option", " A ",
                                "--kind", "question", "--attach", str(image), "--blocking"], self.fx.wt)
        self.assertEqual(r.returncode, 0, r.stderr)
        token.assert_called_once_with(4)
        self.assertEqual(r.stdout, "ask 20260930T120100Z-0a0b0c0d\n")
        (path,) = (self.fx.data / "asks" / SID).iterdir()
        raw = path.read_bytes()
        self.assertIn("é".encode("utf-8"), raw)
        record = json.loads(raw.decode("utf-8"))
        types = {"schema": int, "id": str, "session_id": str, "created": str, "kind": str, "blocking": bool,
                 "question": str, "context": str, "options": list, "attachments": list}
        self.assertEqual(set(record), set(types))
        for key, typ in types.items():
            self.assertIs(type(record[key]), typ, key)
        self.assertEqual(record["schema"], 1)
        self.assertEqual(record["session_id"], SID)
        self.assertIsNotNone(home.ID_RE.fullmatch(record["id"]))
        self.assertEqual(path.name, record["id"] + ".json")
        self.assertEqual(record["created"], "2026-09-30T12:01:00+00:00")
        self.assertEqual(datetime.fromisoformat(record["created"]).tzinfo, timezone.utc)
        self.assertIn(record["kind"], home.ASK_KINDS)
        self.assertEqual((record["question"], record["context"], record["options"], record["blocking"]),
                         ("Ship é?", "Context.", ["A"], True))
        for values in (record["options"], record["attachments"]):
            self.assertTrue(all(type(v) is str for v in values))
        self.assertTrue(all(Path(p).is_absolute() and Path(p).is_file() and paths.inside(Path(p), paths.checkouts(self.fx.main))
                            for p in record["attachments"]))
        self.assertLessEqual(len(raw), home.ASK_MAX_BYTES)
        self.assertEqual(set(files_under(self.fx.data)), {f"asks/{SID}/{path.name}".replace("/", os.sep)})

    def test_session_cap_in_process_and_serialized_size_limit(self):
        with patch.object(home, "ASKS_PER_SESSION", 2):
            for _ in range(2):
                r = call_main(ask, ["--question", "q"], self.fx.main)
                self.assertEqual(r.returncode, 0, r.stderr)
            self.rejected(["--question", "q"])
        self.assertEqual(len(list((self.fx.data / "asks" / SID).iterdir())), 2)
        record = self.record(sid="other-session")
        raw = (json.dumps(record, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
        with patch.object(home, "ASK_MAX_BYTES", len(raw)):
            path = ask.write_record(self.fx.data, record)
        self.assertEqual(path.read_bytes(), raw)
        before = files_under(self.fx.data)
        with patch.object(home, "ASK_MAX_BYTES", len(raw) - 1), self.assertRaises(ask.AskRejected):
            ask.write_record(self.fx.data, record)
        self.assertEqual(files_under(self.fx.data), before)
        with patch.object(home, "ASK_MAX_BYTES", 1):
            self.rejected(["--question", "q", "--session", "fresh-session"])
        self.assertFalse((self.fx.data / "asks" / "fresh-session").exists())

    def test_asks_are_never_written_through_a_link_out_of_the_checkout(self):
        outside = Path(self.fx.tmp.name) / "outside-asks"
        outside.mkdir()
        link_directory(self, outside, self.fx.data / "asks")
        r = self.rejected(["--question", "q"])
        self.assertIn("outside the checkout", r.stderr)
        self.assertEqual(list(outside.iterdir()), [])

    def test_an_ask_session_directory_linked_out_of_the_checkout_is_refused(self):
        outside = Path(self.fx.tmp.name) / "outside-asks"
        outside.mkdir()
        link_directory(self, outside, self.fx.data / "asks" / SID)
        self.rejected(["--question", "q"])
        self.assertEqual(list(outside.iterdir()), [])

    def test_writer_lists_a_bounded_number_of_entries(self):
        directory = self.fx.data / "asks" / SID
        directory.mkdir(parents=True)
        for i in range(3):
            (directory / f"note{i}.txt").write_text("", encoding="utf-8")
        with patch.object(home, "ASK_SCAN_MAX", 3):
            self.rejected(["--question", "q"])
        with patch.object(home, "ASK_SCAN_MAX", 4):
            self.assertEqual(call_main(ask, ["--question", "q"], self.fx.main).returncode, 0)

    def test_failed_replace_removes_temp_files(self):
        r = call_main(ask, ["--question", "q"], self.fx.main)
        self.assertEqual(r.returncode, 0, r.stderr)
        with patch.object(ask.os, "replace", side_effect=OSError("busy\ntry later")):
            self.rejected(["--question", "q"])
        self.assertTrue(all(p.suffix == ".json" for p in (self.fx.data / "asks" / SID).iterdir()))

    def test_wait_invalid_id_timeout_and_arguments(self):
        ask_id = "20260930T120100Z-0a0b0c0d"
        for argv in (["wait"], ["wait", "../escape"], ["wait", ask_id + "\n"], ["wait", ask_id.upper()],
                     *(["wait", ask_id, "--timeout", value] for value in ("0", "-1", str(ask.WAIT_MAX + 1), "1.5", "no"))):
            with self.subTest(argv=argv):
                self.rejected(argv)

    def test_wait_polls_until_deadline_and_never_writes_or_locks(self):
        clock, sleeps = [0.0], []

        def sleep(seconds):
            sleeps.append(seconds)
            clock[0] += seconds

        before = files_under(self.fx.data)
        with patch.object(ask, "WAIT_DEFAULT", 5), patch.object(ask.time, "monotonic", side_effect=lambda: clock[0]), \
                patch.object(ask.time, "sleep", side_effect=sleep), patch.object(notes, "_acquire", side_effect=AssertionError("lock")):
            r = call_main(ask, ["wait", "20260930T120100Z-0a0b0c0d"], self.fx.main)
        self.assertEqual((r.returncode, r.stdout, r.stderr), (3, "no answer yet\n", ""))
        self.assertEqual(sleeps, [ask.WAIT_POLL, ask.WAIT_POLL, 1.0])
        self.assertEqual(files_under(self.fx.data), before)
        self.assertFalse(self.fx.data.exists())

    def test_wait_skips_bad_lines_and_finds_queued_and_emitted_answers(self):
        ask_id = "20260930T120100Z-0a0b0c0d"
        inbox = self.fx.data / "inbox" / f"{SID}.jsonl"
        inbox.parent.mkdir(parents=True)
        lines = [b"{bad json", b"\xff", b"null", b"[]", b'{"text": 7}',
                 json.dumps({"text": "answer to ask 20260930T120100Z-00000000: another"}).encode(),
                 json.dumps({"text": f"quoted answer to ask {ask_id}: body"}).encode()]
        inbox.write_bytes(b"\n".join(lines) + b"\n")
        self.assertFalse(ask.answered(self.fx.data, SID, ask_id))
        nid = notes.append_note(self.fx.data, SID, f"answer to ask {ask_id}: private answer body")
        for emitted in (False, True):
            if emitted:
                inbox.with_name(f"{SID}.state.jsonl").write_text(
                    json.dumps({"id": nid, "emitted_at": "2026-09-30T12:02:00Z"}) + "\n", encoding="utf-8")
            before = files_under(self.fx.data)
            with patch.object(notes, "_acquire", side_effect=AssertionError("lock")):
                r = call_main(ask, ["wait", ask_id, "--timeout", "1"], self.fx.wt)
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertEqual(r.stdout, f"answer to ask {ask_id} is queued; it arrives with this tool result\n")
            self.assertNotIn("private answer body", r.stdout + r.stderr)
            self.assertEqual(files_under(self.fx.data), before)

    def test_answered_read_is_bounded_and_unreadable_inbox_is_unanswered(self):
        ask_id = "20260930T120100Z-0a0b0c0d"
        inbox = self.fx.data / "inbox" / f"{SID}.jsonl"
        inbox.parent.mkdir(parents=True)
        prefix = b'{"text": "unrelated"}\n'
        inbox.write_bytes(prefix + json.dumps({"text": f"answer to ask {ask_id}: secret"}).encode() + b"\n")
        with patch.object(notes, "INBOX_MAX_BYTES", len(prefix)):
            self.assertFalse(ask.answered(self.fx.data, SID, ask_id))
        self.assertTrue(ask.answered(self.fx.data, SID, ask_id))
        with patch.object(Path, "open", side_effect=PermissionError):
            self.assertFalse(ask.answered(self.fx.data, SID, ask_id))


class ProgressValidationTests(CliFixtureTests):
    status = ProgressCliTests.status

    def invoke(self, *args, root=None):
        return call_main(progress, list(args), root or self.fx.wt)

    def rejected(self, path, *args, root=None):
        before = path.read_bytes()
        siblings = files_under(path.parent)
        r = self.invoke(*args, root=root)
        self.assertEqual(r.returncode, 2, r)
        self.assertEqual(r.stdout, "")
        self.assertEqual(len(r.stderr.splitlines()), 1, r.stderr)
        self.assertNotIn("Traceback", r.stderr)
        self.assertEqual(path.read_bytes(), before)
        self.assertEqual(files_under(path.parent), siblings)
        return r

    def test_done_first_and_last_then_explicit_done_status(self):
        path = self.status()
        utc = time.gmtime(at(24 * 60 * 60))
        with patch.object(progress.time, "gmtime", return_value=utc):
            for item in ("harness", "runs"):
                r = self.invoke("done", "0.4.1-2", item, "--evidence", "tests/evidence.txt")
                self.assertEqual((r.returncode, r.stdout), (0, f"done 0.4.1-2/{item} (path)\n"), r.stderr)
                doc = json.loads(path.read_bytes())
                self.assertEqual(doc["steps"][1]["status"], "in_progress")
                self.assertEqual(doc["updated"], "2026-10-01")
        self.rejected(path, "done", "0.4.1-2", "harness", "--evidence", "tests/evidence.txt")
        r = self.invoke("status", "0.4.1-2", "done")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(json.loads(path.read_bytes())["steps"][1]["status"], "done")
        self.rejected(path, "status", "0.4.1-2", "not_started")
        self.rejected(path, "status", "0.4.1-2", "done")

    def test_blocked_stays_blocked_and_status_rejects_open_items_and_noops(self):
        path = self.status()
        self.rejected(path, "status", "0.4.1-2", "done")
        self.rejected(path, "status", "0.4.1-2", "not_started")
        self.rejected(path, "status", "0.4.1-1", "not_started")
        progress.set_status(self.fx.wt, "0.4.1-2", "blocked", "2026-10-02")
        self.assertEqual(json.loads(path.read_bytes())["updated"], "2026-10-02")
        for item in ("harness", "runs"):
            r = self.invoke("done", "0.4.1-2", item, "--evidence", "tests/evidence.txt")
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertEqual(json.loads(path.read_bytes())["steps"][1]["status"], "blocked")
        with self.assertRaises(progress.ProgressRejected):
            progress.set_status(self.fx.wt, "0.4.1-2", "unknown", "2026-10-02")
        self.rejected(path, "status", "missing", "in_progress")

    def test_evidence_unknown_names_and_already_done_reject_without_writes(self):
        path = self.status()
        (self.fx.wt / "work").mkdir(exist_ok=True)
        (self.fx.wt / "work" / "private.txt").write_text("private", encoding="utf-8")
        for ref in ("work/private.txt", "Work/private.txt", "../tests/evidence.txt", "tests/../evidence.txt",
                    "tests/missing.txt", "tests", "0000000000000000000000000000000000000000",
                    "https://github.com/other/repo/pull/1"):
            with self.subTest(ref=ref):
                self.rejected(path, "done", "0.4.1-2", "harness", "--evidence", ref)
        for step, item in (("missing", "harness"), ("0.4.1-2", "missing"), ("0.4.1-1", "a")):
            self.rejected(path, "done", step, item, "--evidence", "tests/evidence.txt")

    def test_parallel_tracks_steps_items_and_current(self):
        path = self.status()
        self.assertEqual(self.invoke("track", "design", "--title", "Design track").returncode, 0)
        doc = json.loads(path.read_bytes())
        self.assertEqual([t["id"] for t in doc["tracks"]], [t for t, _ in checklist.DEFAULT_TRACKS] + ["design"])
        self.assertEqual(path.read_bytes().replace(b"\r\n", b"\n").decode("utf-8"), checklist.dump(doc))
        self.assertIn(b"\r\n", path.read_bytes())
        r = self.invoke("step", "d.1", "--track", "design", "--title", "Greybox layouts", "--item", "g1:2:Layout A: grid",
                        "--item", "g2:1:Layout B", "--acceptance", "Every kept command is reachable.",
                        "--acceptance-source", "tests/evidence.txt")
        self.assertEqual((r.returncode, r.stdout), (0, "step d.1 added to design\n"), r.stderr)
        step = json.loads(path.read_bytes())["steps"][-1]
        self.assertEqual((step["status"], step["track"], step["items"][0]["title"], step["items"][0]["weight"]),
                         ("not_started", "design", "Layout A: grid", 2))
        for item in ("g1", "g2"):
            self.assertEqual(self.invoke("done", "d.1", item, "--evidence", "tests/evidence.txt").returncode, 0)
        self.assertEqual(self.invoke("status", "d.1", "done").returncode, 0)
        self.assertEqual(self.invoke("item", "d.1", "g3", "--title", "Layout C", "--weight", "3").returncode, 0)
        step = json.loads(path.read_bytes())["steps"][-1]
        self.assertEqual((step["status"], step["items"][-1]["done"]), ("in_progress", False))
        self.assertEqual(self.invoke("current", "d.1").returncode, 0)
        doc = json.loads(path.read_bytes())
        self.assertEqual(doc["current"], "d.1")
        self.assertEqual(checklist.validate(doc), [])
        view = home.progress_view(doc, [])
        self.assertEqual([t["id"] for t in view["tracks"]], ["0.4.1", "stage-2", "design"])
        self.assertEqual(view["tracks"][2]["current"]["id"], "d.1")

    def test_adding_rejects_bad_input_without_writes(self):
        path = self.status()
        self.assertEqual(self.invoke("track", "design", "--title", "Design").returncode, 0)
        for args in (("track", "design", "--title", "Again"), ("track", "Bad Id", "--title", "x"),
                     ("track", "t2", "--title", "\u8bbe\u8ba1"), ("track", "t2", "--title", "  "),
                     ("track", "t2", "--title", "see /home/user/x"), ("track", "t2", "--title", "a\nb"),
                     ("step", "d.1", "--track", "missing", "--title", "x", "--item", "a:1:x"),
                     ("step", "d.1", "--track", "design", "--title", "x"),
                     ("step", "d.1", "--track", "design", "--title", "x", "--item", "a:0:x"),
                     ("step", "d.1", "--track", "design", "--title", "x", "--item", "a:x"),
                     ("step", "d.1", "--track", "design", "--title", "x", "--item", "A:1:x"),
                     ("step", "d.1", "--track", "design", "--title", "x", "--item", "a:1:x", "--item", "a:1:y"),
                     ("step", "d.1", "--track", "design", "--title", "x", "--item", "a:1:x", "--acceptance", "y"),
                     ("step", "d.1", "--track", "design", "--title", "x", "--item", "a:1:x", "--acceptance", "y",
                      "--acceptance-source", "tests/missing.txt"),
                     ("step", "0.4.1-1", "--track", "design", "--title", "x", "--item", "a:1:x"),
                     ("item", "missing", "a", "--title", "x"), ("item", "0.4.1-2", "harness", "--title", "x"),
                     ("item", "0.4.1-2", "new", "--title", "x", "--weight", "0"),
                     ("current", "missing")):
            with self.subTest(args=args):
                self.rejected(path, *args)

    def test_validate_checks_the_track_list(self):
        doc = sample_status()
        self.assertEqual(checklist.tracks(doc), list(checklist.DEFAULT_TRACKS))
        good = {**doc, "tracks": [{"id": "0.4.1", "title": "0.4.1"}, {"id": "stage-2", "title": "Stage 2"}]}
        self.assertEqual(checklist.validate(good), [])
        for tracks, problem in (([], "tracks must be a non-empty list"), ("x", "tracks must be a non-empty list"),
                                ([{"id": "0.4.1", "title": "a"}, {"id": "0.4.1", "title": "b"}], "duplicate track id"),
                                ([{"id": "Bad", "title": "a"}], "track id 'Bad' is invalid"),
                                ([{"id": "0.4.1", "title": ""}], "title must be"),
                                ([{"id": "0.4.1", "title": "0.4.1"}], "unknown track")):
            with self.subTest(tracks=tracks):
                self.assertTrue(any(problem in p for p in checklist.validate({**doc, "tracks": tracks})))

    def test_pr_evidence_needs_no_subprocess(self):
        path = self.status()
        ref = "https://github.com/KonomiYuzu01/Magic-600-Cell/pull/20"
        with patch.object(progress.subprocess, "run", side_effect=AssertionError("subprocess")):
            r = self.invoke("done", "0.4.1-2", "harness", "--evidence", ref)
        self.assertEqual((r.returncode, r.stdout), (0, "done 0.4.1-2/harness (pr)\n"), r.stderr)
        self.assertEqual(json.loads(path.read_bytes())["steps"][1]["items"][0]["evidence"], ref)

    def test_sha_evidence_from_isolated_fixture_commit(self):
        path = self.status(self.fx.main)
        fixture_home = Path(self.fx.tmp.name) / "git-home"
        fixture_home.mkdir()
        env = {**os.environ, "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull, "HOME": str(fixture_home)}

        def git(*args):
            r = subprocess.run(["git", "-C", str(self.fx.main), *args], env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(r.returncode, 0, r.stderr)
            return r.stdout.strip()

        git("init", "--quiet")
        git("-c", "user.name=t", "-c", "user.email=t@example.invalid", "commit", "--allow-empty", "--quiet", "-m", "fixture")
        sha = git("rev-parse", "HEAD")
        with patch.dict(os.environ, env):
            r = self.cli("progress.py", "done", "0.4.1-2", "harness", "--evidence", sha)
            self.assertEqual((r.returncode, r.stdout), (0, "done 0.4.1-2/harness (sha)\n"), r.stderr)
            self.assertIsNone(progress.evidence_problem(self.fx.main, sha[:7]))
        self.assertEqual(json.loads(path.read_bytes())["steps"][1]["items"][0]["evidence"], sha)

    def test_sha_resolution_is_captured_timed_and_failure_is_a_problem(self):
        ref = "0123abc"
        with patch.object(progress.subprocess, "run", return_value=subprocess.CompletedProcess([], 0)) as run:
            self.assertIsNone(progress.evidence_problem(self.fx.wt, ref))
        run.assert_called_once_with(["git", "-C", str(self.fx.wt), "rev-parse", "--verify", "--quiet", ref + "^{commit}"],
                                    timeout=30, capture_output=True)
        for failure in (FileNotFoundError("git"), subprocess.TimeoutExpired("git", 30)):
            with patch.object(progress.subprocess, "run", side_effect=failure):
                self.assertIsNotNone(progress.evidence_problem(self.fx.wt, ref))

    def test_check_good_bad_missing_unreadable_oversized_and_non_json(self):
        root = self.fx.wt
        self.assertTrue(progress.check(root))
        r = self.invoke("check")
        self.assertEqual(r.returncode, 2)
        path = self.status()
        r = self.invoke("check")
        self.assertEqual((r.returncode, r.stdout, r.stderr), (0, "status.json ok\n", ""))
        before = path.read_bytes()
        with patch.object(Path, "open", side_effect=PermissionError):
            self.assertEqual(progress.check(root), ["cannot read status.json"])
        for raw in (b"x" * (progress.STATUS_MAX + 1), b"{not JSON", b"\xff", b"[]",
                    checklist.dump({**sample_status(), "schema": 1}).encode("utf-8")):
            with self.subTest(raw=raw[:50]):
                path.write_bytes(raw)
                problems = progress.check(root)
                self.assertTrue(problems)
                r = self.invoke("check")
                self.assertEqual((r.returncode, r.stdout), (2, "\n".join(problems) + "\n"))
                self.rejected(path, "done", "0.4.1-2", "harness", "--evidence", "tests/evidence.txt")
                self.rejected(path, "status", "0.4.1-2", "blocked")
        path.write_bytes(before)
        doc = json.loads(before)
        doc["steps"][0]["items"][0]["evidence"] = "tests/missing.txt"
        path.write_bytes(checklist.dump(doc).encode("utf-8"))
        problems = progress.check(root)
        self.assertEqual(len(problems), 1)
        self.assertTrue(problems[0].startswith("0.4.1-1/a: "), problems)
        r = self.rejected(path, "done", "0.4.1-2", "harness", "--evidence", "tests/evidence.txt")
        self.assertIn(problems[0], r.stderr)

    def test_all_done_evidence_is_rechecked_after_mutation(self):
        path = self.status()
        result = self.fx.wt / "tests" / "new.txt"
        result.write_text("new result", encoding="utf-8")
        original = progress.evidence_problem

        def disappearing_evidence(root, ref):
            if ref == "tests/new.txt":
                (root / "tests" / "evidence.txt").unlink(missing_ok=True)
            return original(root, ref)

        with patch.object(progress, "evidence_problem", side_effect=disappearing_evidence):
            r = self.rejected(path, "done", "0.4.1-2", "harness", "--evidence", "tests/new.txt")
        self.assertIn("0.4.1-1/a:", r.stderr)

    def test_resulting_size_limit_rejects_without_write(self):
        path = self.status()
        with patch.object(progress, "STATUS_MAX", len(path.read_bytes())):
            self.rejected(path, "done", "0.4.1-2", "harness", "--evidence", "tests/evidence.txt")

    def test_lf_layout_stays_lf_and_replace_failure_leaves_no_temp(self):
        path = self.status()
        path.write_bytes(path.read_bytes().replace(b"\r\n", b"\n"))
        for args in (("done", "0.4.1-2", "harness", "--evidence", "tests/evidence.txt"),
                     ("status", "0.4.1-2", "blocked")):
            r = self.invoke(*args)
            self.assertEqual(r.returncode, 0, r.stderr)
            raw = path.read_bytes()
            self.assertNotIn(b"\r", raw)
            self.assertEqual(raw.decode("utf-8"), checklist.dump(json.loads(raw)))
            self.assertEqual(list(path.parent.iterdir()), [path])
        with patch.object(progress.os, "replace", side_effect=OSError("busy\ntry later")):
            self.rejected(path, "done", "0.4.1-2", "runs", "--evidence", "tests/evidence.txt")
            self.rejected(path, "status", "0.4.1-2", "in_progress")

    def test_checkout_selection_and_main_edits_only_main(self):
        main_path, wt_path = self.status(self.fx.main), self.status()
        wt_before = wt_path.read_bytes()
        self.assertEqual(progress.checkout_root(main_path.parent), self.fx.main)
        self.assertEqual(progress.checkout_root(wt_path.parent), self.fx.wt)
        self.assertEqual(progress.checkout_root(self.fx.ext), self.fx.ext)
        self.assertIsNone(progress.checkout_root(self.fx.projects))
        with patch.object(paths, "checkouts", return_value=[self.fx.ext]):
            self.assertIsNone(progress.checkout_root(self.fx.main))
        r = self.cli("progress.py", "done", "0.4.1-2", "harness", "--evidence", "tests/evidence.txt", cwd=main_path.parent)
        self.assertEqual(r.returncode, 0, r.stderr)
        main_after = main_path.read_bytes()
        self.assertEqual(wt_path.read_bytes(), wt_before)
        r = self.cli("progress.py", "status", "0.4.1-2", "blocked", cwd=wt_path.parent)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(main_path.read_bytes(), main_after)
        r = self.invoke("check", root=self.fx.projects)
        self.assertEqual(r.returncode, 2)
        self.assertEqual(len(r.stderr.splitlines()), 1)

    def test_evidence_symlink_escape_when_supported(self):
        path = self.status()
        outside = Path(self.fx.tmp.name) / "outside.txt"
        outside.write_text("outside", encoding="utf-8")
        link = self.fx.wt / "tests" / "escape.txt"
        try:
            link.symlink_to(outside)
        except (OSError, NotImplementedError):
            self.skipTest("fixture symlinks are unavailable")
        self.rejected(path, "done", "0.4.1-2", "harness", "--evidence", "tests/escape.txt")

    def test_status_file_behind_a_link_out_of_the_checkout_is_refused(self):
        path = self.status()
        outside = Path(self.fx.tmp.name) / "outside-progress"
        shutil.move(str(path.parent), str(outside))
        link_directory(self, outside, path.parent)
        path = outside / "status.json"
        r = self.rejected(path, "done", "0.4.1-2", "harness", "--evidence", "tests/evidence.txt")
        self.assertIn("outside the checkout", r.stderr)
        self.rejected(path, "status", "0.4.1-2", "in_progress")
        r = self.invoke("check")
        self.assertEqual((r.returncode, r.stdout.strip()), (2, "status.json leads outside the checkout"))

    def test_status_file_linked_back_inside_from_a_linked_directory_is_refused(self):
        # docs/progress is a junction out of the checkout, and status.json there leads back inside:
        # the file resolves inside, but the temporary file would be created outside.
        self.status()
        prog = self.fx.wt / "docs" / "progress"
        backing = self.fx.wt / "backing.json"
        shutil.move(str(prog / "status.json"), str(backing))
        prog.rmdir()
        outside = Path(self.fx.tmp.name) / "outside-progress"
        outside.mkdir()
        (outside / "status.json").write_bytes(backing.read_bytes())
        link_directory(self, outside, prog)
        real = Path.resolve
        linked = real(outside / "status.json")

        def resolve(self, strict=False):   # stands in for a file symlink, which needs a privilege here
            r = real(self, strict)
            return real(backing, strict) if r == linked else r

        with patch.object(Path, "resolve", resolve):
            for args in (("done", "0.4.1-2", "harness", "--evidence", "tests/evidence.txt"),
                         ("status", "0.4.1-2", "in_progress")):
                with self.subTest(command=args[0]):
                    r = self.rejected(outside / "status.json", *args)
                    self.assertIn("outside the checkout", r.stderr)
        self.assertEqual(sorted(p.name for p in outside.iterdir()), ["status.json"])

    def test_in_process_garbage_arguments_return_2_without_traceback(self):
        path = self.status()
        for argv in ((), ("garbage",), ("done",), ("done", "s", "i"), ("check", "garbage"),
                     ("status", "s", "unknown"), ("garbage\nargument",)):
            with self.subTest(argv=argv):
                self.rejected(path, *argv)


if __name__ == "__main__":
    unittest.main(verbosity=2)
