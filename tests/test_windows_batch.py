"""Fixture checks for the Windows batch; no real step, PowerShell or GPU."""
from __future__ import annotations

import _thread
import importlib
import io
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import types
import unittest
import uuid
from datetime import datetime, timezone
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools/windows_batch"))
import batch_interface as I

NOW = datetime(2026, 10, 3, 8, 9, 10, tzinfo=timezone.utc)
STAMP = "20261003T080910Z"
SOURCE = {"head": "a" * 40, "paths": ["fixture.py"], "digest": "b" * 64}


class FakeStep:
    def __init__(self, name="native", result=None, *, missing=None, error=None):
        self.name = name
        self.title = "Fixture " + name
        self.result = result if result is not None else I.StepResult(I.PASS, lines=["OK"])
        self.missing = missing
        self.error = error
        self.contexts = []
        self.ran = False

    def unavailable(self, ctx):
        self.contexts.append(ctx)
        return self.missing

    def run(self, ctx):
        self.ran = True
        self.ctx = ctx
        if self.error is not None:
            raise self.error
        return self.result


class InterruptingOut(io.StringIO):
    """A console where the first write containing `trigger` raises KeyboardInterrupt."""

    def __init__(self, trigger):
        super().__init__()
        self.trigger = trigger
        self.fired = False

    def write(self, text):
        if not self.fired and self.trigger in text:
            self.fired = True
            raise KeyboardInterrupt
        return super().write(text)


class TemporaryRepo(unittest.TestCase):
    def setUp(self):
        # Default mkdir mode inherits a usable ACL in the Windows sandbox.
        self.base = Path(tempfile.gettempdir()) / ("m6wb-test-" + uuid.uuid4().hex)
        self.base.mkdir()
        self.addCleanup(shutil.rmtree, self.base)
        self.repo = self.base / "repo"
        self.repo.mkdir()
        self.temp = self.base / "temp"
        self.temp.mkdir()
        self.batch = importlib.import_module("run_all")
        self.guard = importlib.import_module("batch_guard")
        self.report = importlib.import_module("batch_report")
        self.source = self.enterContext(mock.patch.object(
            self.report._source_digest, "source_identity", return_value=SOURCE))
        self.enterContext(mock.patch.object(self.batch, "is_windows", return_value=True))
        self.admin = self.enterContext(mock.patch.object(
            self.batch, "is_elevated", return_value=False))
        self.temp_lookup = self.enterContext(mock.patch.object(
            tempfile, "gettempdir", return_value=str(self.temp)))
        self.enterContext(mock.patch.dict(os.environ, {
            "USERNAME": "FixtureOwner", "COMPUTERNAME": "FixtureComputer",
            "USERDOMAIN": "FixtureDomain"}))
        self.out = io.StringIO()
        self.input = mock.Mock(return_value="")

    def go(self, steps=None, argv=()):
        if steps is None:
            steps = [FakeStep()]
        return self.batch.main(list(argv), repo=self.repo, steps=steps,
                               input_fn=self.input, out=self.out, clock=lambda: NOW)

    def record(self):
        files = list((self.repo / "work/windows-batch").glob("*/batch.json"))
        self.assertEqual(len(files), 1)
        return json.loads(files[0].read_text(encoding="utf-8")), files[0].parent

    def context(self):
        private = self.base / "private"
        scratch = self.base / "scratch"
        private.mkdir()
        scratch.mkdir()
        return self.batch.BatchContext(self.repo, private, scratch, False,
                                       dict(I.DEFAULT_OPTIONS), self.input, self.out)


class ArgumentTests(TemporaryRepo):
    def test_only_preserves_module_order(self):
        steps = [FakeStep("renderer"), FakeStep("migration"), FakeStep("native")]
        self.assertEqual(self.go(steps, ["--only", "renderer,native,native"]), I.EXIT_PASS)
        record, _ = self.record()
        self.assertEqual([s["name"] for s in record["steps"]], ["native", "renderer"])
        self.assertEqual(record["options"]["only"], ["native", "renderer"])
        self.assertFalse(steps[1].ran)

    def test_unknown_only_is_usage_error_without_writes(self):
        self.assertEqual(self.go(argv=["--only", "native,unknown"]), I.EXIT_USAGE)
        self.assertEqual(list(self.repo.iterdir()), [])
        self.input.assert_not_called()
        self.source.assert_not_called()

    def test_scenes_validation_and_none(self):
        for text in ["", "w0", "w5", "w1,w1", "none,w1", "W1", "w1,"]:
            with self.subTest(invalid=text):
                self.assertEqual(self.go(argv=["--list", "--scenes", text]), I.EXIT_USAGE)
        for text, expected in [("none", ()), ("w4,w2,w1", ("w4", "w2", "w1"))]:
            with self.subTest(valid=text):
                step = FakeStep()
                self.assertEqual(self.go([step], ["--list", "--scenes", text]), I.EXIT_PASS)
                self.assertEqual(step.contexts[0].options["scenes"], expected)
        self.assertEqual(list(self.repo.iterdir()), [])

    def test_runs_bounds(self):
        for value in ["0", "11", "-1", "1.5", "word"]:
            with self.subTest(invalid=value):
                self.assertEqual(self.go(argv=["--list", "--runs", value]), I.EXIT_USAGE)
        for value in ["1", "10"]:
            step = FakeStep()
            self.assertEqual(self.go([step], ["--list", "--runs", value]), I.EXIT_PASS)
            self.assertEqual(step.contexts[0].options["runs"], int(value))

    def test_option_overrides_and_defaults(self):
        step = FakeStep()
        self.assertEqual(self.go([step], ["--no-faults", "--rebuild-probe",
                                         "--scenes", "none", "--runs", "10"]), 0)
        self.assertEqual(step.ctx.options, {
            "scenes": (), "runs": 10, "faults": False, "rebuild_probe": True})
        self.assertEqual(I.DEFAULT_OPTIONS["scenes"], ("w1", "w2", "w4"))
        record, _ = self.record()
        self.assertEqual(record["options"]["scenes"], [])
        self.assertFalse(record["options"]["faults"])
        self.assertTrue(record["options"]["rebuild_probe"])

    def test_list_on_other_os_writes_prompts_and_starts_nothing(self):
        step = FakeStep(missing="fixture unavailable")
        with mock.patch.object(self.batch, "is_windows", return_value=False), \
                mock.patch.object(self.batch.subprocess, "Popen") as popen:
            self.assertEqual(self.go([step], ["--list"]), I.EXIT_PASS)
        popen.assert_not_called()
        self.source.assert_not_called()
        self.temp_lookup.assert_not_called()
        self.input.assert_not_called()
        self.assertFalse(step.ran)
        self.assertEqual(list(self.repo.iterdir()), [])
        self.assertEqual(list(self.temp.iterdir()), [])
        self.assertIn("skip: fixture unavailable", self.out.getvalue())

    def test_non_windows_exits_before_loading_or_setup(self):
        with mock.patch.object(self.batch, "is_windows", return_value=False), \
                mock.patch.object(self.batch, "load_steps") as load:
            self.assertEqual(self.batch.main([], repo=self.repo, out=self.out), I.EXIT_USAGE)
        load.assert_not_called()
        self.input.assert_not_called()
        self.assertEqual(len(self.out.getvalue().splitlines()), 1)
        self.assertEqual(list(self.repo.iterdir()), [])


class FlowTests(TemporaryRepo):
    def test_step_status_exit_codes(self):
        for status, code in [(I.PASS, 0), (I.FAIL, 1), (I.ERROR, 1), (I.SKIPPED, 0)]:
            with self.subTest(status=status):
                self.repo = self.base / status
                self.repo.mkdir()
                step = FakeStep(result=I.StepResult(status, "fixture reason"))
                self.assertEqual(self.go([step]), code)
                record, _ = self.record()
                self.assertEqual(record["steps"][0]["status"], status)
                self.assertEqual(record["outcome"], "fail" if code else "pass")
                self.assertTrue(step.ctx.private.is_dir())
                self.assertFalse(step.ctx.scratch.parent.exists())

    def test_exception_does_not_stop_the_next_step(self):
        broken = FakeStep(error=ValueError("fixture failure"))
        later = FakeStep("migration")
        self.assertEqual(self.go([broken, later]), I.EXIT_FAIL)
        record, _ = self.record()
        self.assertEqual(record["steps"][0]["status"], I.ERROR)
        self.assertEqual(record["steps"][0]["reason"], "ValueError: fixture failure")
        self.assertTrue(later.ran)
        self.assertIn("native: error - ValueError: fixture failure", self.out.getvalue())
        self.assertIn("migration: pass - OK", self.out.getvalue())

    def test_interrupt_writes_results_and_skips_later_steps(self):
        first = FakeStep()
        interrupted = FakeStep("migration", error=KeyboardInterrupt())
        later = FakeStep("renderer")
        self.assertEqual(self.go([first, interrupted, later]), I.EXIT_INTERRUPTED)
        record, _ = self.record()
        self.assertEqual(record["outcome"], "interrupted")
        self.assertEqual([s["status"] for s in record["steps"]],
                         [I.PASS, I.INTERRUPTED, I.SKIPPED])
        self.assertEqual(record["steps"][2]["reason"], "not run: the batch was interrupted")
        self.assertFalse(later.ran)
        self.assertFalse(later.contexts[0].private.exists())
        self.assertFalse(interrupted.ctx.scratch.parent.exists())

    def test_initial_eof_is_an_interrupted_batch(self):
        self.input.side_effect = EOFError()
        first, later = FakeStep(), FakeStep("migration")
        self.assertEqual(self.go([first, later]), I.EXIT_INTERRUPTED)
        record, _ = self.record()
        self.assertEqual([s["status"] for s in record["steps"]],
                         [I.INTERRUPTED, I.SKIPPED])
        self.assertFalse(first.ran)
        self.assertFalse(later.ran)

    def test_interrupt_between_steps_keeps_completed_results(self):
        # Review finding WB-L2-03: Ctrl+C while the progress line of a finished step prints.
        self.out = InterruptingOut("native: pass")
        first, later, last = FakeStep(), FakeStep("migration"), FakeStep("renderer")
        self.assertEqual(self.go([first, later, last]), I.EXIT_INTERRUPTED)
        record, _ = self.record()
        self.assertEqual(record["outcome"], "interrupted")
        self.assertEqual([s["status"] for s in record["steps"]], [I.PASS, I.INTERRUPTED, I.SKIPPED])
        self.assertEqual(record["steps"][0]["lines"], ["OK"])
        self.assertEqual(record["steps"][1]["reason"], "interrupted between steps")
        self.assertEqual(record["steps"][2]["reason"], "not run: the batch was interrupted")
        self.assertTrue(first.ran)
        self.assertFalse(later.ran)
        self.assertFalse(last.ran)
        self.assertFalse(first.ctx.scratch.parent.exists())

    def test_interrupt_after_the_last_step_is_an_interrupted_batch(self):
        self.out = InterruptingOut("native: pass")
        self.assertEqual(self.go([FakeStep()]), I.EXIT_INTERRUPTED)
        record, _ = self.record()
        self.assertEqual(record["outcome"], "interrupted")
        self.assertEqual([s["status"] for s in record["steps"]], [I.PASS])

    def test_interrupt_during_source_identity_still_publishes(self):
        self.source.side_effect = KeyboardInterrupt()
        first, later = FakeStep(), FakeStep("migration")
        self.assertEqual(self.go([first, later]), I.EXIT_INTERRUPTED)
        record, _ = self.record()
        self.assertEqual(record["source"], {"error": "not checked: the batch was interrupted"})
        self.assertEqual([s["status"] for s in record["steps"]], [I.INTERRUPTED, I.SKIPPED])
        self.assertEqual(record["steps"][0]["reason"], "interrupted before the batch started")
        self.assertFalse(first.ran)

    def test_ctrl_c_while_a_result_is_recorded_keeps_it(self):
        # Review WB-L2-03, verification round 1: a real Ctrl+C after a step returned.
        normalize = self.batch.normalize
        tripped = []

        def tripping(result):
            if not tripped:
                tripped.append(True)
                _thread.interrupt_main()
            return normalize(result)

        first = FakeStep(result=I.StepResult(I.PASS, counts={"checks": 19}, lines=["OK"]))
        later, last = FakeStep("migration"), FakeStep("renderer")
        with mock.patch.object(self.batch, "normalize", tripping):
            self.assertEqual(self.go([first, later, last]), I.EXIT_INTERRUPTED)
        record, _ = self.record()
        self.assertEqual(record["outcome"], "interrupted")
        self.assertEqual([s["status"] for s in record["steps"]], [I.PASS, I.INTERRUPTED, I.SKIPPED])
        self.assertEqual(record["steps"][0]["counts"], {"checks": 19})
        self.assertEqual(record["steps"][0]["lines"], ["OK"])
        self.assertEqual(record["steps"][1]["reason"], "interrupted between steps")
        self.assertEqual(record["steps"][2]["reason"], "not run: the batch was interrupted")
        self.assertFalse(later.ran)
        self.assertFalse(last.ran)
        self.assertFalse(first.ctx.scratch.parent.exists())

    def test_ctrl_c_during_a_step_interrupts_it(self):
        class Tripping(FakeStep):
            def run(self, ctx):
                super().run(ctx)
                _thread.interrupt_main()
                return self.result  # not reached: the Ctrl+C is raised inside the step

        first, later = Tripping(), FakeStep("migration")
        self.assertEqual(self.go([first, later]), I.EXIT_INTERRUPTED)
        record, _ = self.record()
        self.assertEqual([s["status"] for s in record["steps"]], [I.INTERRUPTED, I.SKIPPED])
        self.assertEqual(record["steps"][0]["reason"], "interrupted by the owner")
        self.assertFalse(later.ran)
        self.assertFalse(first.ctx.scratch.parent.exists())

    def test_ctrl_c_while_planning_stops_before_the_start_prompt(self):
        def tripping(**kwargs):
            _thread.interrupt_main()
            return SOURCE

        self.source.side_effect = tripping
        first, later = FakeStep(), FakeStep("migration")
        self.assertEqual(self.go([first, later]), I.EXIT_INTERRUPTED)
        record, _ = self.record()
        self.assertEqual(record["source"]["head"], SOURCE["head"])
        self.assertEqual([s["status"] for s in record["steps"]], [I.INTERRUPTED, I.SKIPPED])
        self.assertEqual(record["steps"][0]["reason"], "interrupted before the batch started")
        self.input.assert_not_called()
        self.assertFalse(first.ran)

    def test_publication_and_cleanup_hold_ctrl_c(self):
        seen = []
        publish = self.report.publish

        def tripping(*args, **kwargs):
            handler = signal.getsignal(signal.SIGINT)
            seen.append((type(handler).__name__, handler.held))
            _thread.interrupt_main()
            return publish(*args, **kwargs)

        before = signal.getsignal(signal.SIGINT)
        with mock.patch.object(self.report, "publish", side_effect=tripping):
            self.assertEqual(self.go(), I.EXIT_PASS)
        self.assertEqual(seen, [("_InterruptHold", True)])
        record, _ = self.record()
        self.assertEqual(record["outcome"], "pass")
        self.assertEqual(list(self.temp.iterdir()), [])
        self.assertIs(signal.getsignal(signal.SIGINT), before)

    def test_hold_never_raises_in_the_batch_frame(self):
        hold = self.batch._InterruptHold()
        hold.held = False
        # A signal handled in the batch frame arrives after the step or prompt returned.
        hold(signal.SIGINT, types.SimpleNamespace(f_code=self.batch._run_batch.__code__))
        self.assertTrue(hold.pending)
        with self.assertRaises(KeyboardInterrupt):
            hold.release()
        self.assertFalse(hold.pending)
        with self.assertRaises(KeyboardInterrupt):
            hold(signal.SIGINT, sys._getframe())
        hold.held = True
        hold(signal.SIGINT, sys._getframe())
        self.assertTrue(hold.pending)

    def test_unavailable_has_no_step_directories(self):
        step = FakeStep(missing="tool not present")
        self.assertEqual(self.go([step]), I.EXIT_PASS)
        record, _ = self.record()
        self.assertEqual(record["steps"][0]["status"], I.SKIPPED)
        self.assertEqual(record["steps"][0]["reason"], "tool not present")
        self.assertFalse(step.ran)
        self.assertFalse(step.contexts[0].private.exists())
        self.assertFalse(step.contexts[0].scratch.exists())

    def test_unimportable_module_is_error_and_other_module_runs(self):
        modules = self.base / "modules"
        modules.mkdir()
        (modules / "step_native.py").write_text(
            "raise ImportError('fixture import failure')\n", encoding="ascii")
        (modules / "step_migration.py").write_text(
            "from batch_interface import StepResult, PASS\n"
            "class Fixture:\n"
            " name = 'migration'\n"
            " title = 'Fixture migration'\n"
            " def unavailable(self, ctx): return None\n"
            " def run(self, ctx): return StepResult(PASS)\n"
            "STEP = Fixture()\n", encoding="ascii")
        with mock.patch.object(self.batch, "HERE", modules), mock.patch.object(
                self.batch, "STEP_MODULES", ("step_native", "step_migration")):
            code = self.batch.main([], repo=self.repo, input_fn=self.input,
                                   out=self.out, clock=lambda: NOW)
        self.assertEqual(code, I.EXIT_FAIL)
        record, _ = self.record()
        self.assertEqual([s["status"] for s in record["steps"]], [I.ERROR, I.PASS])
        self.assertIn("ImportError: fixture import failure", record["steps"][0]["reason"])

    def test_result_limits_and_invalid_digests(self):
        step = FakeStep(result=I.StepResult(
            "invented", "x" * 400 + "\nprivate tail", counts={"checks": 3},
            digests={"good": "a" * 64, "upper": "A" * 64, "short": "b", "number": 3},
            details={"observed": True}, lines=["z" * 220 + "\nend"] * 20))
        self.assertEqual(self.go([step]), I.EXIT_FAIL)
        record, _ = self.record()
        result = record["steps"][0]
        self.assertEqual(result["status"], I.ERROR)
        self.assertLessEqual(len(result["reason"]), I.REASON_LIMIT)
        self.assertNotIn("\n", result["reason"])
        self.assertEqual(len(result["lines"]), I.MAX_LINES)
        self.assertTrue(all(len(line) <= 200 and "\n" not in line for line in result["lines"]))
        self.assertEqual(result["digests"], {"good": "a" * 64})
        self.assertEqual(result["counts"], {"checks": 3})
        self.assertEqual(result["details"], {"observed": True})

    def test_scratch_collision_adds_pid_and_keep_preserves_only_new_scratch(self):
        existing = self.temp / ("m6wb-" + STAMP)
        existing.mkdir()
        marker = existing / "old.txt"
        marker.write_text("old", encoding="ascii")
        step = FakeStep()
        self.assertEqual(self.go([step], ["--keep-scratch"]), I.EXIT_PASS)
        self.assertEqual(step.ctx.scratch.parent.name, "m6wb-" + STAMP + "-" + str(os.getpid()))
        self.assertTrue(step.ctx.scratch.is_dir())
        self.assertEqual(marker.read_text(encoding="ascii"), "old")

    def test_scratch_cleanup_failure_is_reported(self):
        step = FakeStep()
        with mock.patch.object(self.batch.shutil, "rmtree", side_effect=PermissionError("locked")):
            self.assertEqual(self.go([step]), I.EXIT_PASS)
        self.assertTrue(step.ctx.scratch.is_dir())
        self.assertIn("Scratch remains:", self.out.getvalue())


class ReportTests(TemporaryRepo):
    def private_text(self):
        return str(self.repo / "fresh.sqlite3") + " FixtureOwner S-1-5-21-123-456-789-1001"

    def test_sanitizes_reason_details_lines_and_machine_record(self):
        text = self.private_text()
        step = FakeStep(result=I.StepResult(I.PASS, text, counts={"checks": 2},
                                           digests={"fixture": "c" * 64},
                                           details={"path": text}, lines=[text]))
        self.assertEqual(self.go([step]), I.EXIT_PASS)
        record, directory = self.record()
        combined = (directory / "batch.json").read_text(encoding="utf-8")
        combined += (directory / "summary.md").read_text(encoding="utf-8")
        for secret in [str(self.repo), "FixtureOwner", "S-1-5-21-123-456-789-1001"]:
            self.assertNotIn(secret, combined)
        for placeholder in ["<repo>", "<private>", "<sid>"]:
            self.assertIn(placeholder, combined)
        self.assertEqual(self.report._sanitizer.leaks(combined), [])
        self.assertEqual(self.report._sanitizer.leaks_in(record), [])
        self.assertEqual(record["format"], I.FORMAT)
        self.assertEqual(record["source"], {"head": SOURCE["head"], "changed_paths": 1,
                                            "digest": SOURCE["digest"]})
        self.source.assert_called_once_with(timeout=30)
        self.assertEqual(set(record["machine"]),
                         {"os", "os_release", "os_build", "python", "python_bits", "elevated"})
        self.assertEqual(record["started_utc"], "2026-10-03T08:09:10Z")
        self.assertEqual(record["finished_utc"], "2026-10-03T08:09:10Z")
        self.assertEqual(record["private_records"],
                         "work/loop-memory/windows-batch/" + STAMP + "/ (not committed)")
        for filename in ["batch.json", "summary.md"]:
            raw = (directory / filename).read_bytes()
            self.assertNotIn(b"\r", raw)
            self.assertTrue(raw.endswith(b"\n"))
        self.assertIn("git add -f work/windows-batch/2026-10-03", self.out.getvalue())

    def test_leak_precedes_interrupted_outcome_and_writes_only_private_record(self):
        step = FakeStep(result=I.StepResult(I.INTERRUPTED, self.private_text()))
        with mock.patch.object(self.report._sanitizer, "sanitize", side_effect=lambda value, roots: value):
            self.assertEqual(self.go([step]), I.EXIT_USAGE)
        self.assertFalse((self.repo / "work/windows-batch").exists())
        raw = self.repo / "work/loop-memory/windows-batch" / STAMP / "batch-unpublished.json"
        self.assertTrue(raw.is_file())
        self.assertIn("FixtureOwner", raw.read_text(encoding="utf-8"))
        self.assertIn("sid", self.out.getvalue())
        self.assertIn("private-word", self.out.getvalue())
        self.assertNotIn("git add -f", self.out.getvalue())

    def test_placeholder_paths_use_portable_separators_even_with_spaces(self):
        path = str(self.repo / "two words" / "fresh.sqlite3")
        step = FakeStep(result=I.StepResult(I.PASS, details={"nested": [{"path": path}]}))
        self.assertEqual(self.go([step]), I.EXIT_PASS)
        record, _ = self.record()
        self.assertEqual(record["steps"][0]["details"]["nested"][0]["path"],
                         "<repo>/two words/fresh.sqlite3")

    def test_unsanitized_absolute_path_alone_blocks_publication(self):
        step = FakeStep(result=I.StepResult(I.PASS, r"C:\fixture\secret.sqlite3"))
        with mock.patch.object(self.report._sanitizer, "sanitize", side_effect=lambda value, roots: value):
            self.assertEqual(self.go([step]), I.EXIT_USAGE)
        self.assertFalse((self.repo / "work/windows-batch").exists())
        self.assertIn("absolute-path", self.out.getvalue())

    def test_existing_date_directory_is_not_overwritten(self):
        existing = self.repo / "work/windows-batch/2026-10-03"
        existing.mkdir(parents=True)
        (existing / "batch.json").write_text("existing", encoding="ascii")
        self.assertEqual(self.go(), I.EXIT_PASS)
        result = existing.with_name("2026-10-03-2")
        self.assertTrue((result / "batch.json").is_file())
        self.assertEqual((existing / "batch.json").read_text(encoding="ascii"), "existing")

    def test_one_page_summary_escapes_every_table_cell(self):
        steps = [FakeStep(name, I.StepResult(I.PASS, "ok | verified",
                                            counts={"cases|total": 2},
                                            lines=["fixture line"] * 30))
                 for name in ["native", "migration", "renderer"]]
        self.assertEqual(self.go(steps), I.EXIT_PASS)
        _, directory = self.record()
        summary = (directory / "summary.md").read_text(encoding="utf-8")
        self.assertLessEqual(len(summary.splitlines()), 60)
        self.assertEqual(summary.splitlines()[0], "# Windows batch 2026-10-03: pass (3 pass)")
        self.assertIn("Actual Windows on fresh synthetic data.", summary)
        self.assertIn("cases\\|total=2", summary)
        for line in summary.splitlines():
            if line.startswith("| "):
                self.assertEqual(len(re.findall(r"(?<!\\)\|", line)), 5)
        for step in steps:
            self.assertIn("## " + step.name, summary)
        self.assertEqual(summary.count("- fixture line"), 3 * I.MAX_LINES)
        self.assertIn("raw records are not committed", summary.splitlines()[-1].lower())

    def test_source_exception_is_clipped_and_sanitized(self):
        self.source.side_effect = RuntimeError(self.private_text() + " " + "x" * 400)
        self.assertEqual(self.go(), I.EXIT_PASS)
        record, _ = self.record()
        self.assertEqual(set(record["source"]), {"error"})
        error = record["source"]["error"]
        self.assertLessEqual(len(error), I.REASON_LIMIT)
        # clip() replaces the path before the cut, with the placeholder of the root that holds it.
        self.assertNotIn(str(self.repo), error)
        self.assertRegex(error, r"<(repo|temp|home|path)>")
        self.assertIn("<private>", error)
        self.assertEqual(self.report._sanitizer.leaks(error), [])

    def test_private_names_across_clip_boundaries_are_replaced_whole(self):
        # Review finding WB-L2-04: a cut through a name used to keep a fragment such as "Fixture...".
        step = FakeStep(result=I.StepResult(
            I.FAIL, "x" * 290 + "FixtureOwner",
            lines=["y" * 190 + "FixtureComputer", "z" * 190 + "FixtureDomain"]))
        step.title = "Boundary step"
        self.source.side_effect = RuntimeError("w" * 276 + "FixtureDomain")
        self.assertEqual(self.go([step]), I.EXIT_FAIL)
        record, directory = self.record()
        published = ((directory / "batch.json").read_text(encoding="utf-8") +
                     (directory / "summary.md").read_text(encoding="utf-8"))
        self.assertNotIn("fixture", published.lower())
        self.assertEqual(record["steps"][0]["reason"], "x" * 290 + "<private>")
        self.assertEqual(record["steps"][0]["lines"], ["y" * 190 + "<private>", "z" * 190 + "<private>"])
        self.assertEqual(record["source"]["error"], "RuntimeError: " + "w" * 276 + "<private>")


class ConsoleTests(TemporaryRepo):
    def test_manual_has_one_clear_prompt_and_one_input(self):
        ctx = self.context()
        ctx.manual("Start 0.4,\nwait, then close it.")
        prompts = [line for line in self.out.getvalue().splitlines() if line.startswith(">>> ")]
        self.assertEqual(prompts, [">>> Start 0.4, wait, then close it. Press Enter to continue."])
        self.assertIn("\a", self.out.getvalue())
        self.input.assert_called_once_with()

    def test_ask_empty_uses_default_and_answers_are_stripped(self):
        ctx = self.context()
        self.input.side_effect = ["", "  answer  "]
        self.assertEqual(ctx.ask("Continue?", "yes"), "yes")
        self.assertEqual(ctx.ask("Choose", "no"), "answer")
        self.assertIn("??? Continue? [yes]: ", self.out.getvalue())

    def test_eof_is_keyboard_interrupt(self):
        ctx = self.context()
        self.input.side_effect = EOFError()
        with self.assertRaises(KeyboardInterrupt):
            ctx.manual("Continue")
        with self.assertRaises(KeyboardInterrupt):
            ctx.ask("Continue?")

    def test_require_quiet_rechecks_and_never_prints_command_lines(self):
        ctx = self.context()
        processes = [{"pid": 123, "ppid": 0, "name": "cl.exe", "command": "DO-NOT-ECHO"}]
        with mock.patch.object(self.guard, "list_processes", side_effect=[processes, []]) as listing:
            self.assertIsNone(ctx.require_quiet())
        self.assertEqual(listing.call_count, 2)
        self.assertIn("  123 cl.exe: build", self.out.getvalue())
        self.assertNotIn("DO-NOT-ECHO", self.out.getvalue())
        self.input.assert_called_once_with()

    def test_require_quiet_skip_names_are_unique(self):
        ctx = self.context()
        processes = [{"pid": pid, "ppid": 0, "name": "cl.exe", "command": None}
                     for pid in [123, 124]]
        self.input.return_value = "  SKIP  "
        with mock.patch.object(self.guard, "list_processes", return_value=processes):
            self.assertEqual(ctx.require_quiet(),
                             "skipped by the owner: heavy work was running (cl.exe)")

    def test_require_quiet_retries_process_list_failure(self):
        ctx = self.context()
        with mock.patch.object(self.guard, "list_processes",
                               side_effect=[RuntimeError("fixture list unavailable"), []]):
            self.assertIsNone(ctx.require_quiet())
        self.assertIn("fixture list unavailable", self.out.getvalue())
        self.assertIn("Stop them and press Enter to check again, or type skip to skip the captures",
                      self.out.getvalue())


class ProcessTests(TemporaryRepo):
    def test_output_is_echoed_captured_and_logged_with_default_cwd(self):
        ctx = self.context()
        script = ("import os, sys; print(os.getcwd()); print('stdout'); "
                  "print('stderr', file=sys.stderr); "
                  "print('stdin=' + sys.stdin.read()); sys.stdout.buffer.write(b'\\xfftail\\n')")
        result = ctx.run([sys.executable, "-u", "-c", script], timeout=10)
        self.assertEqual(result.returncode, 0)
        self.assertFalse(result.timed_out)
        self.assertGreaterEqual(result.seconds, 0)
        self.assertIn(str(self.repo), result.output)
        self.assertIn("stdout", result.output)
        self.assertIn("stderr", result.output)
        self.assertIn("stdin=\n", result.output)
        self.assertIn("\ufffdtail", result.output)
        log = ctx.private / ("01-" + Path(sys.executable).stem + ".log")
        self.assertEqual(log.read_text(encoding="utf-8"), result.output)
        self.assertEqual(self.out.getvalue(), result.output)
        ctx.run([sys.executable, "-c", "print('second')"], timeout=10)
        self.assertTrue((ctx.private / ("02-" + Path(sys.executable).stem + ".log")).is_file())

    def test_cwd_and_env_override(self):
        ctx = self.context()
        script = "import os; print(os.getcwd()); print(os.environ['WB_FIXTURE'])"
        result = ctx.run([sys.executable, "-c", script], timeout=10,
                         cwd=ctx.scratch, env=dict(os.environ, WB_FIXTURE="override"))
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.output.splitlines(), [str(ctx.scratch), "override"])

    def test_timeout_ends_sleeping_child_within_twenty_seconds(self):
        ctx = self.context()
        start = time.monotonic()
        result = ctx.run([sys.executable, "-c", "import time; time.sleep(60)"], timeout=2)
        self.assertTrue(result.timed_out)
        self.assertIsNone(result.returncode)
        self.assertLess(time.monotonic() - start, 20)

    def test_interactive_inherits_stdio_and_has_no_output_log(self):
        ctx = self.context()
        proc = mock.Mock()
        proc.wait.return_value = 0
        with mock.patch.object(self.batch.subprocess, "Popen", return_value=proc) as popen:
            result = ctx.run(["fixture.exe"], timeout=2, interactive=True)
        self.assertEqual(result.output, "")
        self.assertEqual(result.returncode, 0)
        for stream in ["stdin", "stdout", "stderr"]:
            self.assertIsNone(popen.call_args.kwargs.get(stream))
        self.assertFalse(popen.call_args.kwargs.get("shell", False))
        self.assertEqual(list(ctx.private.iterdir()), [])

    def test_keyboard_interrupt_ends_child_and_propagates(self):
        ctx = self.context()
        proc = mock.Mock(stdout=io.BytesIO(b"fixture\n"))
        proc.wait.side_effect = KeyboardInterrupt()
        with mock.patch.object(self.batch.subprocess, "Popen", return_value=proc), \
                mock.patch.object(self.batch, "stop_tree") as stop:
            with self.assertRaises(KeyboardInterrupt):
                ctx.run(["fixture.exe"], timeout=2)
        stop.assert_called_once_with(proc)


class GuardTests(TemporaryRepo):
    def test_heavy_flags_only_specified_work_and_excludes_descendants(self):
        names = ["cl.exe", "Godot_v4.7.2-stable_win64.exe", "PresentMon-2.6.0-x64.exe",
                 "sb_probe.exe", "python.exe", "PresentMonService.exe", "codex.exe",
                 "python.exe", "node.exe", "powershell.exe", "ffmpeg.exe", "CMAKE.EXE"]
        commands = {4: "python codex_review.py --kind implement", 6: "codex.exe app-server",
                    7: "python other_script.py"}
        processes = [{"pid": n + 100, "ppid": 0, "name": name,
                      "command": commands.get(n)} for n, name in enumerate(names)]
        processes += [{"pid": 3, "ppid": 2, "name": "ninja.exe", "command": None},
                      {"pid": 2, "ppid": 1, "name": "cl.exe", "command": None},
                      {"pid": 1, "ppid": 0, "name": "sb_probe.exe", "command": None}]
        found = self.guard.heavy(processes, own_pid=1)
        self.assertEqual([pid for pid, _, _ in found], [100, 101, 102, 103, 104, 110, 111])
        self.assertEqual([why for _, _, why in found],
                         ["build", "Godot", "PresentMon capture", "project GPU program",
                          "Codex implementation call", "heavy tool", "build"])
        self.assertEqual(processes[-1]["name"], "sb_probe.exe")

    def test_process_list_accepts_single_object_and_list(self):
        item = {"ProcessId": 10, "ParentProcessId": 1, "Name": "fixture.exe", "CommandLine": None}
        for value in [item, [item], []]:
            with self.subTest(value=value):
                completed = subprocess.CompletedProcess([], 0, json.dumps(value), "")
                with mock.patch.object(self.guard.subprocess, "run", return_value=completed):
                    result = self.guard.list_processes(timeout=3)
                expected = [{"pid": 10, "ppid": 1, "name": "fixture.exe", "command": None}]
                self.assertEqual(result, expected if value else [])

    def test_process_list_failures_are_short(self):
        for result in [subprocess.CompletedProcess([], 1, "private commands", "failure"),
                       subprocess.CompletedProcess([], 0, "not JSON", ""),
                       subprocess.TimeoutExpired("fixture", 1)]:
            with self.subTest(result=result):
                behavior = {"side_effect": result} if isinstance(result, Exception) else {"return_value": result}
                with mock.patch.object(self.guard.subprocess, "run", **behavior):
                    with self.assertRaises(RuntimeError) as caught:
                        self.guard.list_processes()
                self.assertLess(len(str(caught.exception)), 200)
                self.assertNotIn("private commands", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
