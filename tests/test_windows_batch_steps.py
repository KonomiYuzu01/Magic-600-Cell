"""Windows batch step contracts, using synthetic files and fake processes only."""
from __future__ import annotations

import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest import mock
import uuid

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools/windows_batch"))
import batch_interface as wb
import step_migration as migration
import step_native as native
import step_renderer as renderer

REFERENCE = json.loads((ROOT / "docs/progress/1.0/migration-probe-results.json").read_text(encoding="utf-8"))
BUILD = "ab" * 32


class FakeContext:
    def __init__(self, base):
        self.repo = base / "repo"
        self.private = base / "private"
        self.scratch = base / "scratch"
        for path in (self.repo, self.private, self.scratch):
            path.mkdir()
        self.elevated = True
        self.options = dict(wb.DEFAULT_OPTIONS, scenes=("w1",), faults=False)
        self.events = []
        self.handlers = []
        self.answers = []
        self.quiet = []

    def say(self, line):
        self.events.append(("say", line))

    def manual(self, line):
        self.events.append(("manual", line))

    def ask(self, question, default=""):
        self.events.append(("ask", question, default))
        return self.answers.pop(0) if self.answers else default

    def require_quiet(self):
        self.events.append(("quiet",))
        return self.quiet.pop(0) if self.quiet else None

    def run(self, argv, **kwargs):
        self.events.append(("run", list(argv), kwargs))
        if not self.handlers:
            raise AssertionError("unexpected process: %s" % argv)
        return self.handlers.pop(0)(list(argv), kwargs)


class StepTests(unittest.TestCase):
    def setUp(self):
        # Plain mkdir inherits the temporary directory ACL in the Windows sandbox.
        self.base = Path(tempfile.gettempdir()) / ("wb-steps-" + uuid.uuid4().hex)
        self.base.mkdir()
        self.addCleanup(shutil.rmtree, self.base)
        self.ctx = FakeContext(self.base)
        self.repo = self.ctx.repo
        self.windir = self.base / "windows"
        self.program_files = self.base / "program-files"
        environment = mock.patch.dict(os.environ, {"WINDIR": str(self.windir),
                                                   "SystemRoot": str(self.windir),
                                                   "ProgramFiles": str(self.program_files)})
        environment.start()
        self.addCleanup(environment.stop)
        for module in (native, migration, renderer):
            platform = mock.patch.object(module, "IS_WINDOWS", True)
            platform.start()
            self.addCleanup(platform.stop)

    def write(self, path, content="fixture", encoding="utf-8"):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding=encoding, newline="\n")
        return path

    def write_json(self, path, content, encoding="utf-8"):
        return self.write(path, json.dumps(content), encoding)

    def engine(self):
        return self.write(self.repo / "tools/.venv/engine/Scripts/python.exe")

    def runs(self):
        return [event for event in self.ctx.events if event[0] == "run"]

    def kinds(self):
        return [event[0] for event in self.ctx.events]

    def assert_unavailable(self, step, reason):
        before = sorted(str(path.relative_to(self.base)) for path in self.base.rglob("*"))
        self.assertEqual(step.unavailable(self.ctx), reason)
        self.assertEqual(self.ctx.events, [])
        self.assertEqual(before, sorted(str(path.relative_to(self.base)) for path in self.base.rglob("*")))


class NativeTests(StepTests):
    def setUp(self):
        super().setUp()
        self.bootstrap = self.write(self.repo / "native/bootstrap.py")
        self.source = self.write(self.repo / "tests/native/NativeHostRegression.cs")
        self.compiler = self.write(self.windir / "Microsoft.NET/Framework/v4.0.30319/csc.exe")
        self.engine_python = self.engine()

    def native_output(self, passed=True, failed=(), error=None, returncode=0):
        def handler(argv, kwargs):
            data = Path(argv[argv.index("--data") + 1])
            self.assertEqual(data, self.ctx.scratch / "data")
            self.assertFalse(data.exists())
            diagnostics = data / "native-host-0.3/diagnostics"
            checks = [{"name": name, "passed": name not in failed} for name in ("layout", "startup", "closing")]
            self.write_json(diagnostics / "winforms-self-test.json", {
                "passed": passed, "scope": "WinForms fixture", "clr": "4.0.30319.42000", "process_bits": 32,
                "checks": checks, "error": error, "os": "Windows", "original_splitter_exception": None,
            }, "utf-8-sig")
            self.write_json(diagnostics / "build-info.json", {"source_sha256": BUILD.upper()})
            self.write(diagnostics / "winforms-self-test.log", "fixture log")
            self.write(diagnostics / "NativeHostRegression.exe", "fixture exe")
            return wb.Completed(returncode, "fixture last output\n", 5)
        return handler

    def test_name_and_available(self):
        self.assertEqual(native.STEP.name, "native")
        self.assert_unavailable(native.STEP, None)

    def test_windows_only(self):
        with mock.patch.object(native, "IS_WINDOWS", False):
            self.assert_unavailable(native.STEP, "Windows only")

    def test_missing_native_sources(self):
        for path in (self.bootstrap, self.source):
            with self.subTest(path=path.name):
                path.unlink()
                self.assert_unavailable(native.STEP, "native host regression not in this checkout")
                self.write(path)

    def test_missing_compiler(self):
        self.compiler.unlink()
        self.assert_unavailable(native.STEP, ".NET Framework 4.x compiler not found")

    def test_32_bit_fallback(self):
        self.engine_python.unlink()
        with mock.patch.object(native.struct, "calcsize", return_value=4):
            self.assert_unavailable(native.STEP, "needs 64-bit Python")

    def test_engine_preferred_over_32_bit_fallback(self):
        with mock.patch.object(native.struct, "calcsize", return_value=4):
            self.assert_unavailable(native.STEP, None)

    def test_pass_command_environment_and_evidence(self):
        self.ctx.handlers = [self.native_output()]
        # The sanitiser behind wb.clip() needs the home directory, as in every real environment.
        home = {key: value for key, value in os.environ.items()
                if key.upper() in ("USERPROFILE", "HOMEDRIVE", "HOMEPATH", "HOME")}
        environment = {"LIB": "bad", "lib": "also bad", "LiB": "bad too", "KEEP": "kept", **home}
        with mock.patch.object(native.os, "environ", environment):
            result = native.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.PASS)
        self.assertEqual(result.counts, {"checks": 3, "checks_passed": 3, "checks_failed": 0})
        self.assertEqual(self.kinds(), ["manual", "run"])
        argv, kwargs = self.runs()[0][1:]
        self.assertEqual(argv, [str(self.engine_python), str(self.bootstrap), "--self-test-only", "--data",
                                str(self.ctx.scratch / "data")])
        self.assertEqual(kwargs["timeout"], 900)
        self.assertEqual(kwargs["env"], {"KEEP": "kept", "PYTHONUTF8": "1", **home})
        self.assertEqual(environment["LIB"], "bad")
        self.assertEqual(result.digests["NativeHostRegression.cs"], hashlib.sha256(b"fixture").hexdigest())
        self.assertEqual(result.digests["host_sources"], BUILD)
        diagnostics = self.ctx.scratch / "data/native-host-0.3/diagnostics"
        for key, path in (("report", diagnostics / "winforms-self-test.json"),
                          ("NativeHostRegression.exe", diagnostics / "NativeHostRegression.exe")):
            self.assertEqual(result.digests[key], hashlib.sha256(path.read_bytes()).hexdigest())
        self.assertEqual(sorted(path.name for path in self.ctx.private.iterdir()),
                         ["build-info.json", "winforms-self-test.json", "winforms-self-test.log"])
        self.assertEqual(result.details["failed_checks"], [])
        self.assertEqual(result.details["scope"], "WinForms fixture")
        self.assertEqual(result.details["process_bits"], 32)
        self.assertIn("3 of 3", result.lines[0])

    def test_fallback_interpreter(self):
        self.engine_python.unlink()
        self.ctx.handlers = [self.native_output()]
        self.assertEqual(native.STEP.run(self.ctx).status, wb.PASS)
        self.assertEqual(self.runs()[0][1][0], sys.executable)

    def test_fail_with_failed_checks(self):
        self.ctx.handlers = [self.native_output(passed=False, failed=("layout", "closing"), returncode=1)]
        result = native.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.FAIL)
        self.assertEqual(result.reason, "layout, closing")
        self.assertEqual(result.counts["checks_failed"], 2)
        self.assertEqual(result.details["failed_checks"], ["layout", "closing"])

    def test_fail_keeps_only_first_error_line(self):
        self.ctx.handlers = [self.native_output(passed=False, error="FixtureError: broken\nprivate stack trace", returncode=1)]
        result = native.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.FAIL)
        self.assertEqual(result.reason, "FixtureError: broken")

    def test_pass_report_with_nonzero_exit_fails(self):
        self.ctx.handlers = [self.native_output(returncode=1)]
        result = native.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.FAIL)
        self.assertEqual(result.reason, "fixture last output")

    def test_timeout(self):
        self.ctx.handlers = [lambda argv, kwargs: wb.Completed(None, "", 900, timed_out=True)]
        result = native.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.reason, "native self-test timed out after 900 s")

    def test_missing_report_keeps_compile_error(self):
        self.ctx.handlers = [lambda argv, kwargs: wb.Completed(1, "Compiling...\nerror CS1619: long path\n\n", 2)]
        result = native.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.reason, "error CS1619: long path")

    def test_missing_report_without_output(self):
        self.ctx.handlers = [lambda argv, kwargs: wb.Completed(0, "", 2)]
        result = native.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.reason, "no self-test report")

    def test_unreadable_report(self):
        def handler(argv, kwargs):
            self.write(self.ctx.scratch / "data/native-host-0.3/diagnostics/winforms-self-test.json", "{")
            return wb.Completed(0, "", 1)
        self.ctx.handlers = [handler]
        self.assertEqual(native.STEP.run(self.ctx).status, wb.ERROR)

    def test_invalid_host_digest_is_not_reported(self):
        create = self.native_output()
        def handler(argv, kwargs):
            completed = create(argv, kwargs)
            self.write_json(self.ctx.scratch / "data/native-host-0.3/diagnostics/build-info.json", {"source_sha256": "bad"})
            return completed
        self.ctx.handlers = [handler]
        result = native.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.PASS)
        self.assertNotIn("host_sources", result.digests)


class MigrationTests(StepTests):
    def setUp(self):
        super().setUp()
        self.runner = self.write(self.repo / "tools/migration_probes/run_probes.py", "runner")
        self.write(self.repo / "tools/migration_probes/fixtures.py", "fixtures")
        self.engine_python = self.engine()
        self.run_root = self.repo / "work/migration-probes/runs/20261003T005400Z"

    def public(self):
        run = REFERENCE["probes"]["p1"]["run"]
        return {"run": self.run_root.name, "environment": copy.deepcopy(REFERENCE["runs"][run]),
                "probes": {name: copy.deepcopy(REFERENCE["probes"][name]["result"]) for name in migration.PROBES},
                "commands": copy.deepcopy(REFERENCE["commands"])}

    def migration_output(self, public=None, returncode=0):
        if public is None:
            public = self.public()
        def handler(argv, kwargs):
            self.write_json(self.run_root / "public.json", public)
            self.write_json(self.run_root / "raw.json", {"private": "ignored by the step"})
            return wb.Completed(returncode, "probes finished\n", 840)
        return handler

    def test_name_and_available(self):
        self.assertEqual(migration.STEP.name, "migration")
        self.assert_unavailable(migration.STEP, None)

    def test_windows_only(self):
        with mock.patch.object(migration, "IS_WINDOWS", False):
            self.assert_unavailable(migration.STEP, "Windows only")

    def test_missing_runner(self):
        self.runner.unlink()
        self.assert_unavailable(migration.STEP, "migration probes not in this checkout")

    def test_missing_engine(self):
        self.engine_python.unlink()
        self.assert_unavailable(migration.STEP, "engine environment tools/.venv/engine is missing (docs/DEVELOPMENT.md)")

    def test_reference_pass_command_and_evidence(self):
        # Existing evidence must not be mistaken for this invocation's fresh run.
        self.write_json(self.run_root.parent / "20261002T000000Z/public.json", {"old": True})
        self.ctx.handlers = [self.migration_output()]
        result = migration.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.PASS)
        self.assertEqual(self.kinds(), ["say", "run"])
        argv, kwargs = self.runs()[0][1:]
        self.assertEqual(argv, [str(self.engine_python), str(self.runner), "p3", "pipeline", "p1", "p2"])
        self.assertNotIn("--out", argv)
        self.assertEqual(kwargs["timeout"], 7200)
        self.assertEqual(kwargs["cwd"], self.repo)
        self.assertEqual(kwargs["env"]["PYTHONUTF8"], "1")
        self.assertEqual(result.counts, {
            "p3_cases": 6, "p3_passed": 6, "pipeline_cases": 6, "pipeline_passed": 6,
            "p1_c3_attempts": 12, "p1_c3_passed_attempts": 12, "p1_controls": 7, "p1_controls_failed": 0,
            "p2_interruptions": 36, "p2_interruptions_passed": 36,
        })
        self.assertEqual(result.details["p1_chosen"], "C3")
        self.assertEqual(result.details["p1_verdicts"]["C1a"], "fail")
        self.assertEqual(result.details["cases"]["p3"]["P3a"], True)
        self.assertEqual(result.details["run"], self.run_root.name)
        self.assertEqual(result.details["seconds"], 840)
        expected_sources = ("fixtures.py\0" + hashlib.sha256(b"fixtures").hexdigest() + "\n"
                            + "run_probes.py\0" + hashlib.sha256(b"runner").hexdigest() + "\n")
        self.assertEqual(result.digests["probe_sources"], hashlib.sha256(expected_sources.encode()).hexdigest())
        self.assertEqual(result.digests["public_json"], hashlib.sha256((self.run_root / "public.json").read_bytes()).hexdigest())
        self.assertEqual(list(self.ctx.private.iterdir()), [])
        self.assertIn("14 min", result.lines[-1])

    def test_case_failures(self):
        for probe in ("p3", "pipeline"):
            with self.subTest(probe=probe):
                public = self.public()
                case = next(iter(public["probes"][probe]))
                public["probes"][probe][case]["passed"] = False
                self.ctx.handlers = [self.migration_output(public)]
                result = migration.STEP.run(self.ctx)
                self.assertEqual(result.status, wb.FAIL)
                self.assertIn(probe + ": " + case, result.reason)
                self.assertEqual(result.counts[probe + "_passed"], 5)
                shutil.rmtree(self.run_root)

    def test_c3_failure(self):
        public = self.public()
        public["probes"]["p1"]["verdicts"]["C3"]["verdict"] = "fail"
        self.ctx.handlers = [self.migration_output(public)]
        result = migration.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.FAIL)
        self.assertIn("p1: C3 fail", result.reason)

    def test_control_failure(self):
        public = self.public()
        public["probes"]["p1"]["controls"]["negative-control"]["passed"] = False
        self.ctx.handlers = [self.migration_output(public)]
        result = migration.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.FAIL)
        self.assertEqual(result.counts["p1_controls_failed"], 1)
        self.assertEqual(result.details["failed_controls"], ["negative-control"])
        self.assertIn("negative-control", result.reason)

    def test_none_control_passes(self):
        public = self.public()
        public["probes"]["p1"]["controls"]["negative-control"]["passed"] = None
        self.ctx.handlers = [self.migration_output(public)]
        self.assertEqual(migration.STEP.run(self.ctx).status, wb.PASS)

    def test_p2_failure(self):
        public = self.public()
        public["probes"]["p2"]["passed"] = False
        public["probes"]["p2"]["interruptions"][0]["passed"] = False
        self.ctx.handlers = [self.migration_output(public)]
        result = migration.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.FAIL)
        self.assertIn("p2", result.reason)
        self.assertEqual(result.counts["p2_interruptions_passed"], 35)

    def test_exit_2(self):
        self.ctx.handlers = [self.migration_output(returncode=2)]
        result = migration.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.reason, "run_probes.py exited 2: no public result (private text after sanitising, or bad arguments)")

    def test_other_nonzero_exit(self):
        self.ctx.handlers = [lambda argv, kwargs: wb.Completed(1, "running\nFixtureError: stopped\n", 3)]
        result = migration.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.reason, "FixtureError: stopped")

    def test_timeout(self):
        self.ctx.handlers = [lambda argv, kwargs: wb.Completed(None, "", 7200, timed_out=True)]
        result = migration.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertIn("7200", result.reason)

    def test_no_new_directory(self):
        self.ctx.handlers = [lambda argv, kwargs: wb.Completed(0, "", 3)]
        result = migration.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.reason, "could not identify the probe run directory (0 new)")

    def test_two_new_directories(self):
        create = self.migration_output()
        def handler(argv, kwargs):
            completed = create(argv, kwargs)
            (self.run_root.parent / "20261003T005401Z").mkdir()
            return completed
        self.ctx.handlers = [handler]
        result = migration.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.reason, "could not identify the probe run directory (2 new)")

    def test_missing_public(self):
        def handler(argv, kwargs):
            self.run_root.mkdir(parents=True)
            return wb.Completed(0, "", 3)
        self.ctx.handlers = [handler]
        result = migration.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.reason, "missing public.json")

    def test_unreadable_public(self):
        def handler(argv, kwargs):
            self.write(self.run_root / "public.json", "{")
            return wb.Completed(0, "", 3)
        self.ctx.handlers = [handler]
        result = migration.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertIn("unreadable public.json", result.reason)

    def test_missing_probe(self):
        public = self.public()
        del public["probes"]["p2"]
        self.ctx.handlers = [self.migration_output(public)]
        result = migration.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.reason, "missing probes: p2")


class RendererTests(StepTests):
    def setUp(self):
        super().setUp()
        self.probe_dir = self.repo / "work/experiments/renderer-sb/probe"
        self.script = self.write(self.probe_dir / "run_scene.ps1")
        self.build_cmd = self.write(self.probe_dir / "build.cmd")
        self.executable = self.write(self.probe_dir / "build/sb_probe.exe")
        self.gate = self.write(self.repo / "tools/perf/renderer_gate.py")
        self.presentmon = self.write(self.program_files / "Intel/PresentMon/PresentMonConsoleApplication/PresentMon-2.6.0-x64.exe")
        self.captures = self.repo / "work/loop-memory/perf/renderer/sb"
        self.serial = 0

    def stamp(self):
        self.serial += 1
        return "20261003T080000%03dZ" % self.serial

    def scene_output(self, verdict=None, valid_runs=3, returncode=0, invalid=(), unreadable=(),
                     missing_summary=False, build=BUILD, timed_out=False):
        def handler(argv, kwargs):
            scene = argv[argv.index("-Scene") + 1]
            stamp = self.stamp()
            runs = []
            for index in range(1, valid_runs + 1):
                run_id = "%s-%s-%s" % (stamp, scene, index)
                self.write_json(self.captures / run_id / "run.json", {"run_id": run_id, "scene": scene})
                runs.append({"run_id": run_id, "build_identity": build, "fps": 812.4, "p99_ms": 1.432,
                             "vram_peak_mb": 81.7, "met": True})
            summary_dir = self.captures / (stamp + "-" + scene + "-summary")
            summary_dir.mkdir(parents=True)
            if not missing_summary:
                self.write_json(summary_dir / "summary.json", {
                    "scenes": [{"scene": scene, "gate_scene": scene == "w3", "verdict": verdict or (
                        "met" if scene == "w3" else "attribution"), "runs": runs,
                        "pooled": {"n": 1000, "fps": 812.4, "p99_ms": 1.432, "max_ms": 2, "met": True},
                        "vram_peak_mb": 81.7}],
                    "invalid_runs": list(invalid), "unreadable": list(unreadable),
                })
            return wb.Completed(None if timed_out else returncode, "", 600, timed_out=timed_out)
        return handler

    def fault_output(self, status="fail", returncode=0, missing=False, timed_out=False, applied=True,
                     gate="refused"):
        """One run_scene.ps1 -Inject invocation. gate: "refused" (label check), "other" (other reasons
        only), "accepted" (a valid run) or None (no summary.json, as after a script failure)."""
        def handler(argv, kwargs):
            stamp = self.stamp()
            run_id = stamp + "-w3-1"
            run_dir = self.captures / run_id
            run_dir.mkdir(parents=True)
            # run_scene.ps1 creates the summary directory before its runs.
            summary_dir = self.captures / (stamp + "-w3-summary")
            summary_dir.mkdir()
            if not missing:
                self.write_json(run_dir / "run.json", {"run_id": run_id, "injection_applied": applied, "label_check": {
                    "status": status, "mismatches": 1, "late_adoptions": 2, "binding_mismatches": 3,
                    "missing": 0, "revisions": 4, "copies": 4,
                }})
            if gate is not None:
                scene = {"scene": "w3", "gate_scene": True, "verdict": "no-data", "runs": [], "pooled": None,
                         "vram_peak_mb": None}
                invalid = []
                if gate == "accepted":
                    scene["runs"].append({"run_id": run_id, "build_identity": BUILD})
                else:
                    reasons = ["capture-short", "label-check-failed"] if gate == "refused" else ["capture-short"]
                    invalid.append({"run_id": run_id, "candidate": "S-B", "scene": "w3", "reasons": reasons})
                self.write_json(summary_dir / "summary.json", {"scenes": [scene], "invalid_runs": invalid,
                                                               "unreadable": []})
            return wb.Completed(None if timed_out else returncode, "", 10, timed_out=timed_out)
        return handler

    def test_name_and_available(self):
        self.assertEqual(renderer.STEP.name, "renderer")
        self.assert_unavailable(renderer.STEP, None)

    def test_windows_only(self):
        with mock.patch.object(renderer, "IS_WINDOWS", False):
            self.assert_unavailable(renderer.STEP, "Windows only")

    def test_missing_script_or_gate(self):
        for path in (self.script, self.gate):
            with self.subTest(path=path.name):
                path.unlink()
                self.assert_unavailable(renderer.STEP, "S-B capture script not in this checkout")
                self.write(path)

    def test_missing_build_files(self):
        self.executable.unlink()
        self.build_cmd.unlink()
        self.assert_unavailable(renderer.STEP, "S-B probe build files missing")

    def test_unelevated(self):
        self.ctx.elevated = False
        self.assert_unavailable(renderer.STEP, "needs an administrator PowerShell (PresentMon)")

    def test_missing_presentmon(self):
        self.presentmon.unlink()
        self.assert_unavailable(renderer.STEP, "PresentMon 2.6.0 not found at the pinned location")

    def test_no_requests(self):
        self.ctx.options.update(scenes=(), faults=False)
        self.assert_unavailable(renderer.STEP, "no captures requested")

    def test_build_when_executable_missing(self):
        self.executable.unlink()
        def build(argv, kwargs):
            self.write(self.executable, "rebuilt")
            return wb.Completed(0, "built", 5)
        self.ctx.handlers = [build, self.scene_output()]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.PASS)
        argv, kwargs = self.runs()[0][1:]
        self.assertEqual(argv, ["cmd.exe", "/d", "/c", str(self.build_cmd)])
        self.assertEqual(kwargs, {"cwd": self.probe_dir, "timeout": 1800})
        self.assertEqual(self.kinds()[:4], ["say", "run", "manual", "quiet"])
        self.assertEqual(result.digests["sb_probe_exe"], hashlib.sha256(b"rebuilt").hexdigest())

    def test_requested_rebuild(self):
        self.ctx.options["rebuild_probe"] = True
        self.ctx.handlers = [lambda argv, kwargs: wb.Completed(0, "built", 5), self.scene_output()]
        self.assertEqual(renderer.STEP.run(self.ctx).status, wb.PASS)
        self.assertEqual(self.runs()[0][1][0], "cmd.exe")

    def test_failed_build(self):
        self.executable.unlink()
        self.ctx.handlers = [lambda argv, kwargs: wb.Completed(1, "building\ncompiler unavailable\n", 5)]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.reason, "probe build failed: compiler unavailable")
        self.assertNotIn("manual", self.kinds())

    def test_successful_build_without_executable(self):
        self.executable.unlink()
        self.ctx.handlers = [lambda argv, kwargs: wb.Completed(0, "", 5)]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertIn("probe build failed", result.reason)

    def test_quiet_skip_before_preparation(self):
        self.ctx.quiet = ["heavy work still running"]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.SKIPPED)
        self.assertEqual(result.reason, "heavy work still running")
        self.assertEqual(self.kinds(), ["manual", "quiet"])
        self.assertEqual(result.counts["scenes_complete"], 0)

    def test_quiet_skip_before_first_scene_also_skips_faults(self):
        self.ctx.options["faults"] = True
        self.ctx.quiet = [None, "owner skipped"]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.SKIPPED)
        self.assertEqual(self.runs(), [])
        self.assertEqual(self.kinds(), ["manual", "quiet", "ask", "ask", "manual", "quiet"])

    def test_scene_command_and_complete_attribution(self):
        self.ctx.answers = ["none running", "custom vendor mode"]
        self.ctx.handlers = [self.scene_output()]
        with mock.patch.dict(os.environ, {"PATH": "old-path"}):
            result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.PASS)
        self.assertEqual(self.kinds(), ["manual", "quiet", "ask", "ask", "manual", "quiet", "run"])
        argv, kwargs = self.runs()[0][1:]
        powershell = self.windir / "System32/WindowsPowerShell/v1.0/powershell.exe"
        self.assertEqual(argv, [str(powershell), "-NoProfile", "-File", str(self.script), "-Scene", "w1",
                                "-Runs", "3", "-Overlays", "none running", "-Declare", "vendor_mode=custom vendor mode"])
        self.assertNotIn("-ExecutionPolicy", argv)
        self.assertTrue(kwargs["interactive"])
        self.assertEqual(kwargs["timeout"], 2400)
        self.assertEqual(kwargs["cwd"], self.repo)
        self.assertEqual(kwargs["env"]["PATH"], str(Path(sys.executable).parent) + os.pathsep + "old-path")
        self.assertEqual(result.counts, {"scenes_requested": 1, "scenes_complete": 1, "valid_runs": 3,
                                         "faults_run": 0, "faults_caught": 0})
        self.assertEqual(result.details["declared"], {"overlays": "none running",
                                                      "vendor_mode": renderer.PRIVATE_ANSWER})
        self.assertEqual(json.loads((self.ctx.private / "declared.json").read_text(encoding="utf-8")),
                         {"overlays": "none running", "vendor_mode": "custom vendor mode"})
        scene = result.details["scenes"]["w1"]
        self.assertEqual(scene["pooled"], {"n": 1000, "fps": 812.4, "p99_ms": 1.432})
        self.assertEqual(scene["build_identities"], [BUILD])
        self.assertEqual(len(scene["run_ids"]), 3)
        for key in ("sb_probe_exe", "run_scene_ps1", "renderer_gate_py"):
            self.assertEqual(result.digests[key], hashlib.sha256(b"fixture").hexdigest())
        summary = next(self.captures.glob("*-summary/summary.json"))
        self.assertEqual(result.digests["summary_w1"], hashlib.sha256(summary.read_bytes()).hexdigest())
        self.assertEqual(result.digests["build_identity"], BUILD)
        self.assertIn("812.40 fps", result.lines[0])
        self.assertIn("p99 1.432 ms", result.lines[0])

    def test_gate_met(self):
        self.ctx.options["scenes"] = ("w3",)
        self.ctx.handlers = [self.scene_output(verdict="met")]
        self.assertEqual(renderer.STEP.run(self.ctx).status, wb.PASS)

    def test_gate_not_met(self):
        self.ctx.options["scenes"] = ("w3",)
        self.ctx.handlers = [self.scene_output(verdict="not-met")]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.FAIL)
        self.assertEqual(result.reason, "W3: not-met")
        self.assertEqual(result.counts["scenes_complete"], 1)

    def test_invalid_run(self):
        self.ctx.handlers = [self.scene_output(invalid=[{"scene": "w1", "run_id": "invalid", "reasons": ["label-check"]}])]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.counts["scenes_complete"], 0)
        self.assertIn("label-check", result.reason)
        self.assertEqual(result.details["scenes"]["w1"]["invalid_run_reasons"], ["label-check"])

    def test_unscoped_unreadable_run(self):
        self.ctx.handlers = [self.scene_output(unreadable=[{"reason": "unreadable fixture"}])]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertIn("unreadable fixture", result.reason)

    def test_other_scene_invalid_entry_does_not_reject_capture(self):
        self.ctx.handlers = [self.scene_output(invalid=[{"scene": "w2", "reasons": ["other scene"]}],
                                               unreadable=[{"scene": "w2", "reason": "other scene"}])]
        self.assertEqual(renderer.STEP.run(self.ctx).status, wb.PASS)

    def test_missing_summary(self):
        self.ctx.handlers = [self.scene_output(missing_summary=True)]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertIn("missing summary.json", result.reason)

    def test_no_new_capture(self):
        self.ctx.handlers = [lambda argv, kwargs: wb.Completed(1, "", 1)]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertIn("no complete capture", result.reason)

    def test_fewer_valid_runs(self):
        self.ctx.handlers = [self.scene_output(valid_runs=2)]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.counts["valid_runs"], 2)
        self.assertIn("2 of 3 runs valid", result.reason)

    def test_nonzero_scene_exit(self):
        self.ctx.handlers = [self.scene_output(returncode=1)]
        self.assertEqual(renderer.STEP.run(self.ctx).status, wb.ERROR)

    def test_scene_timeout(self):
        self.ctx.handlers = [self.scene_output(timed_out=True)]
        self.assertEqual(renderer.STEP.run(self.ctx).status, wb.ERROR)

    def test_insufficient_gate_runs(self):
        self.ctx.options["scenes"] = ("w3",)
        self.ctx.handlers = [self.scene_output(verdict="insufficient-runs", valid_runs=2)]
        self.assertEqual(renderer.STEP.run(self.ctx).status, wb.ERROR)

    def test_different_builds_across_scenes_omit_identity(self):
        self.ctx.options["scenes"] = ("w1", "w2")
        self.ctx.handlers = [self.scene_output(), self.scene_output(build="cd" * 32)]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.PASS)
        self.assertNotIn("build_identity", result.digests)
        self.assertEqual(result.counts["scenes_complete"], 2)

    def test_short_build_identity_is_not_a_digest(self):
        self.ctx.handlers = [self.scene_output(build="short")]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.PASS)
        self.assertNotIn("build_identity", result.digests)

    def test_all_faults_caught_and_commands(self):
        self.ctx.options.update(scenes=(), faults=True)
        self.ctx.handlers = [self.fault_output() for _ in range(4)]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.PASS)
        self.assertEqual(result.counts["faults_run"], 4)
        self.assertEqual(result.counts["faults_caught"], 4)
        self.assertEqual(self.kinds(), ["manual", "quiet", "ask", "ask", "manual"] + ["quiet", "run"] * 4)
        self.assertIn("need no answers", self.ctx.events[4][1])
        for event, fault in zip(self.runs(), renderer.FAULTS):
            argv, kwargs = event[1:]
            self.assertEqual(argv[4:], ["-Scene", "w3", "-Runs", "1", "-Duration", "10", "-Inject", fault])
            self.assertNotIn("-ExecutionPolicy", argv)
            self.assertFalse(kwargs.get("interactive", False))
            self.assertEqual(kwargs["timeout"], 900)
            detail = result.details["faults"][fault]
            self.assertEqual(detail["status"], "fail")
            self.assertEqual(detail["counts"], {"mismatches": 1, "late_adoptions": 2, "binding_mismatches": 3,
                                                  "missing": 0, "revisions": 4, "copies": 4})
            self.assertTrue(detail["injection_applied"])
            self.assertTrue(detail["gate_refused"])
        self.assertEqual(result.lines, ["Faults: 4 of 4 caught by the label check and refused by the gate"])

    def test_fault_not_caught(self):
        self.ctx.options.update(scenes=(), faults=True)
        self.ctx.handlers = [self.fault_output(status="pass")] + [self.fault_output() for _ in range(3)]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.FAIL)
        self.assertEqual(result.counts["faults_caught"], 3)
        self.assertIn("corrupt-label", result.reason)

    def test_fault_missing_run_json(self):
        self.ctx.options.update(scenes=(), faults=True)
        self.ctx.handlers = [self.fault_output(missing=True)] + [self.fault_output() for _ in range(3)]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.counts["faults_run"], 4)
        self.assertIn("missing run.json", result.reason)

    def test_fault_missing_status(self):
        self.ctx.options.update(scenes=(), faults=True)
        self.ctx.handlers = [self.fault_output(status=None)] + [self.fault_output() for _ in range(3)]
        self.assertEqual(renderer.STEP.run(self.ctx).status, wb.ERROR)

    def fault_result(self, first, *, reason):
        # Review finding WB-L2-01: the first fault run is judged; the other three are caught.
        self.ctx.options.update(scenes=(), faults=True)
        self.ctx.handlers = [first] + [self.fault_output() for _ in range(3)]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.counts["faults_run"], 4)
        self.assertEqual(result.counts["faults_caught"], 3)
        self.assertIn("fault corrupt-label: " + reason, result.reason)
        return result

    def test_fault_after_timeout_is_error(self):
        result = self.fault_result(self.fault_output(timed_out=True, gate=None),
                                   reason="capture incomplete (script timed out)")
        self.assertEqual(result.status, wb.ERROR)

    def test_fault_with_failed_script_is_error(self):
        result = self.fault_result(self.fault_output(returncode=1), reason="capture incomplete (script exit 1)")
        self.assertEqual(result.status, wb.ERROR)

    def test_missed_fault_fails_even_when_the_capture_failed(self):
        result = self.fault_result(self.fault_output(status="pass", returncode=1, gate=None),
                                   reason="label check passed")
        self.assertEqual(result.status, wb.FAIL)

    def test_fault_not_applied_is_error(self):
        result = self.fault_result(self.fault_output(applied=False), reason="the probe did not apply the fault")
        self.assertEqual(result.status, wb.ERROR)

    def test_fault_without_gate_summary_is_error(self):
        result = self.fault_result(self.fault_output(gate=None), reason="missing gate summary.json")
        self.assertEqual(result.status, wb.ERROR)

    def test_fault_refused_only_for_other_reasons_is_error(self):
        result = self.fault_result(self.fault_output(gate="other"),
                                   reason="the gate did not refuse the run for its label check")
        self.assertEqual(result.status, wb.ERROR)
        self.assertFalse(result.details["faults"]["corrupt-label"]["gate_refused"])

    def test_fault_accepted_by_the_gate_fails(self):
        result = self.fault_result(self.fault_output(gate="accepted"), reason="the gate accepted the run")
        self.assertEqual(result.status, wb.FAIL)
        self.assertFalse(result.details["faults"]["corrupt-label"]["gate_refused"])

    def test_guard_before_every_fault_run(self):
        # Review finding WB-L2-02: heavy work starts after the first fault run.
        self.ctx.options.update(scenes=(), faults=True)
        self.ctx.quiet = [None, None, "skipped by the owner: heavy work was running (cl.exe)"]
        self.ctx.handlers = [self.fault_output()]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(self.kinds(), ["manual", "quiet", "ask", "ask", "manual", "quiet", "run", "quiet"])
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(result.counts["faults_run"], 1)
        self.assertEqual(result.counts["faults_caught"], 1)
        self.assertIn("cl.exe", result.reason)

    def test_default_declarations_are_published(self):
        self.ctx.handlers = [self.scene_output()]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.PASS)
        self.assertEqual(result.details["declared"], {"overlays": renderer.OVERLAYS_DEFAULT,
                                                      "vendor_mode": renderer.VENDOR_DEFAULT})

    def test_free_text_declarations_stay_private(self):
        # Review finding WB-L2-05: diagnostic text typed at the declaration questions.
        diagnostic = ("DxDiag System Information: System Model: FixtureLaptop; SMBIOS UUID: "
                      "01234567-89ab-cdef-0123-456789abcdef; BIOS Serial Number: FIXTURE-SERIAL-4917")
        self.ctx.answers = [diagnostic, diagnostic]
        self.ctx.handlers = [self.scene_output()]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.PASS)
        self.assertEqual(result.details["declared"], {"overlays": renderer.PRIVATE_ANSWER,
                                                      "vendor_mode": renderer.PRIVATE_ANSWER})
        public = json.dumps([result.reason, result.counts, result.digests, result.details, result.lines])
        for secret in ("FixtureLaptop", "01234567-89ab", "FIXTURE-SERIAL-4917"):
            self.assertNotIn(secret, public)
        self.assertEqual(json.loads((self.ctx.private / "declared.json").read_text(encoding="utf-8")),
                         {"overlays": diagnostic, "vendor_mode": diagnostic})
        argv = self.runs()[0][1]
        self.assertEqual(argv[argv.index("-Overlays") + 1], diagnostic)

    def test_scene_and_fault_prompt_order(self):
        self.ctx.options["faults"] = True
        self.ctx.handlers = [self.scene_output()] + [self.fault_output() for _ in range(4)]
        self.assertEqual(renderer.STEP.run(self.ctx).status, wb.PASS)
        self.assertEqual(self.kinds(), ["manual", "quiet", "ask", "ask", "manual", "quiet", "run",
                                        "manual"] + ["quiet", "run"] * 4)

    def test_partial_scene_skip_is_error_and_skips_faults(self):
        self.ctx.options.update(scenes=("w1", "w2"), faults=True)
        self.ctx.quiet = [None, None, "heavy work restarted"]
        self.ctx.handlers = [self.scene_output()]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertEqual(len(self.runs()), 1)
        self.assertEqual(result.counts["scenes_complete"], 1)
        self.assertEqual(result.counts["faults_run"], 0)
        self.assertIn("heavy work restarted", result.reason)

    def test_skip_faults_after_scene_is_error(self):
        self.ctx.options["faults"] = True
        self.ctx.quiet = [None, None, "owner skipped faults"]
        self.ctx.handlers = [self.scene_output()]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.ERROR)
        self.assertIn("owner skipped faults", result.reason)

    def test_gate_fail_takes_precedence_over_incomplete_capture(self):
        self.ctx.options["scenes"] = ("w3",)
        self.ctx.handlers = [self.scene_output(verdict="not-met", returncode=1)]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.FAIL)
        self.assertIn("W3: not-met", result.reason)
        self.assertIn("no complete capture", result.reason)

    def test_fault_fail_takes_precedence_over_missing_report(self):
        self.ctx.options.update(scenes=(), faults=True)
        self.ctx.handlers = [self.fault_output(status="pass"), self.fault_output(missing=True),
                              self.fault_output(), self.fault_output()]
        result = renderer.STEP.run(self.ctx)
        self.assertEqual(result.status, wb.FAIL)
        self.assertIn("corrupt-label", result.reason)
        self.assertIn("swap-same-colour", result.reason)
        self.assertIn("missing run.json", result.reason)


if __name__ == "__main__":
    unittest.main()
