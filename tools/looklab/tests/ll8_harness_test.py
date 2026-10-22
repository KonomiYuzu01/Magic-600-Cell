"""Retain LL7 checks and exercise LL8 scheduling, provenance and shot reports."""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import runpy
import shutil
import struct
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from uuid import uuid4
import zlib
import ll7_harness_test as previous


check = previous.check
lane = previous.previous.module("looklab_ll8", "lane_ll8.py")
IDENTITY = {"commit": "a" * 40, "clean": True, "staged": "b" * 64}
RESULTS = {"ll8-shot": lane.HEADLESS + "\n", "ll8-capture": lane.GRAPHICS + "\n"}


class CurrentHarnessTests(previous.CurrentHarnessTests):
    def schedule(self, mode, geometry=previous.GEOMETRY, stills="", stems=()):
        godot = self.folder / check.GODOT_EXE
        (self.folder / "GodotSharp/Tools/nupkgs").mkdir(parents=True, exist_ok=True)
        calls, extensions, self.call_envs = [], [], []
        self.shot_reports = []

        def process(command, **kwargs):
            return subprocess.CompletedProcess(command, 0, check.GODOT_VERSION + "\n" if "--version" in command else "")

        def runner(executable, stage, temporary, env, arguments, **kwargs):
            calls.append((arguments, kwargs)); self.call_envs.append(dict(env))
            output = ("LOOKLAB_STAGE0_CPU_PASS\nLOOKLAB_STAGE0_GPU_PASS\n"
                      "LOOKLAB_SMOKE_PASS instances=600 vertices=30480 slots=259800 parameters=42 bytes=123\n"
                      + geometry + stills + "".join(RESULTS.values()) + "".join(previous.RESULTS.values())
                      + "".join(previous.previous.RESULTS.values()) + "".join(previous.previous.previous.RESULTS.values()))
            return subprocess.CompletedProcess(arguments, 0, output)

        real_run_path = runpy.run_path

        def extension(path):
            name = Path(path).name
            if name in ("lane_ll5.py", "lane_ll6.py", "lane_ll7.py", "lane_ll8.py"):
                actual = real_run_path(str(Path(__file__).resolve().parents[1] / name))
                def call(key, *args):
                    extensions.append((name, key))
                    if key == "shot":
                        def report(*values):
                            self.shot_reports.append(values)
                            return "g2-20261022T120000Z-0123abcd"
                        with patch.dict(actual[key].__globals__, source_identity=lambda c: dict(IDENTITY), shot_report=report):
                            return actual[key](*args)
                    return actual[key](*args)
                return {key: (lambda *args, key=key: call(key, *args)) for key in ("headless", "graphics", "run", "shot")}
            return {key: (lambda *args, key=key: extensions.append((name, key))) for key in ("headless", "graphics", "run", "shot")}

        with patch.object(check, "find_godot", return_value=godot), patch.object(check, "stage_project", return_value=self.folder), \
             patch.object(check, "audit_packages"), patch.object(check, "run_process", side_effect=process), \
             patch.object(check, "godot_run", side_effect=runner), patch("runpy.run_path", side_effect=extension), \
             contextlib.redirect_stdout(io.StringIO()):
            check.godot_lane(mode, self.folder, {"NUGET_PACKAGES": str(self.folder)}, {}, stems=stems)
        return calls, extensions

    def test_cpu_and_gpu_lane_schedules(self):
        calls, extensions = self.schedule("--godot")
        self.assertEqual(extensions, [("lane_ll3.py", "headless"), ("lane_ll4.py", "run"), ("lane_ll5.py", "headless"),
                                      ("lane_ll6.py", "run"), ("lane_ll7.py", "run"), ("lane_ll8.py", "run")])
        self.assertEqual(len(calls), 33)
        self.assertTrue(all("--headless" in args for args, _ in calls))
        self.assertEqual([kwargs["expected_failure"] for _, kwargs in calls if "expected_failure" in kwargs],
                         ["skip-parameter", "wrong-instance-count", "changed-preset-byte", "ll5-disable-cvd", "ll5-disable-null-cost"]
                         + [fault for _, fault in previous.previous.FAULTS] + [fault for _, fault in previous.FAULTS] + list(lane.SHOT_FAULTS))
        calls, extensions = self.schedule("--godot-gpu")
        self.assertEqual(extensions, [("lane_ll3.py", "graphics"), ("lane_ll5.py", "graphics"), ("lane_ll8.py", "graphics")])
        self.assertEqual(len(calls), 13)
        self.assertEqual(calls[0], (["--headless", "--import"], {}))
        self.assertTrue(all("--headless" not in args and kwargs["graphics"] for args, kwargs in calls[1:]))
        self.assertEqual([kwargs["expected_failure"] for _, kwargs in calls if "expected_failure" in kwargs],
                         ["float-offset", "transpose-q", "missing-instance", "missing-view-frame", "layout-no-blend", "ll5-wrong-matrix"] + list(lane.CAPTURE_FAULTS))
        self.assertTrue(all(kwargs["timeout"] == 600 for _, kwargs in calls[-3:]))

    def shot_root(self):
        root = self.folder / "shot-checkout"
        presets = root / check.LAB / "presets"; presets.mkdir(parents=True, exist_ok=True)
        for name in ("z-last", "default", "g1-hopf-rings"):
            (presets / (name + ".json")).write_text("{}", encoding="utf-8")
        return root

    def test_shot_schedule_stems_context_graphics_and_timeout(self):
        root = self.shot_root()
        for explicit, resolved in (((), ["default", "g1-hopf-rings", "z-last"]), (("z-last", "default"), ["z-last", "default"])):
            with self.subTest(explicit=explicit), patch.object(check, "ROOT", root):
                calls, extensions = self.schedule("--shot", stems=explicit)
            self.assertEqual(extensions, [("lane_ll8.py", "shot")])
            self.assertEqual(calls, [(["--headless", "--import"], {}),
                                     (["--", str(root), str(self.folder), "shot", *resolved], {"graphics": True, "timeout": 1800})])
            command = "python tools/looklab/check.py --shot" + (" " + " ".join(explicit) if explicit else "")
            self.assertEqual(json.loads(self.call_envs[-1]["LOOKLAB_SHOT_CONTEXT"]), {"command": command, "source": IDENTITY})
            self.assertNotIn("LOOKLAB_SHOT_CONTEXT", self.call_envs[0])
            self.assertEqual(self.shot_reports[0][2:], (resolved, IDENTITY, command))

    def test_shot_bad_stems_refused_before_any_mkdir(self):
        root = self.shot_root()
        with patch.object(check, "ROOT", root), patch.object(check.Path, "mkdir") as mkdir, contextlib.redirect_stderr(io.StringIO()):
            for stems in (("G2",), ("default", "default"), ("missing",), ("../default",), ("a/b",), ("-default",), ("default\n",)):
                with self.subTest(stems=stems):
                    self.assertEqual(check.main(["--shot", *stems]), 2)
            mkdir.assert_not_called()

    def test_shot_skips_core_passes_stems_and_keeps_unavailable_exit(self):
        root = self.shot_root()
        with patch.object(check, "ROOT", root), patch.object(check, "core_check") as core, patch.object(check, "godot_lane") as run_lane, \
             patch.object(check, "snapshot", return_value={}), contextlib.redirect_stdout(io.StringIO()):
            for stems in ((), ("g1-hopf-rings", "default")):
                self.assertEqual(check.main(["--shot", *stems]), 0)
                self.assertEqual(run_lane.call_args.args[0], "--shot")
                self.assertEqual(run_lane.call_args.kwargs, {"stems": stems})
            core.assert_not_called()
            run_lane.side_effect = check.GraphicsUnavailable("window unavailable")
            self.assertEqual(check.main(["--shot"]), 3)

    def test_shot_only_app_outputs_may_change(self):
        before = {"./": "directory", "work/": "directory", "tools/a.txt": "source"}
        saved = dict(before, **{"work/loop-memory/": "directory", "work/loop-memory/looklab/": "directory",
                               "work/loop-memory/looklab/shots/run/default.png": "still", "work/loop-memory/looklab/shots/run/manifest.json": "manifest",
                               "work/gallery/": "directory", "work/gallery/looklab/shots/run/default.png": "mirror"})
        for after, expected in ((saved, 0), (dict(saved, **{"tools/a.txt": "changed"}), 1),
                                (dict(saved, **{"ignored.bin": "unexpected"}), 1), (dict(saved, **{"work/gallery/other.png": "unexpected"}), 1)):
            with patch.object(check, "core_check"), patch.object(check, "godot_lane"), \
                 patch.object(check, "snapshot", side_effect=[before, after]), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(check.main(["--shot"]), expected)


class LL8HarnessTests(unittest.TestCase):
    def setUp(self):
        self.folder = Path(tempfile.gettempdir()) / ("looklab-ll8-test-" + uuid4().hex)
        self.folder.mkdir()
        self.runs, self.number = [], 0
        self.results, self.bad_fault, self.fault_code, self.fault_message = dict(RESULTS), None, 1, None

    def tearDown(self):
        shutil.rmtree(self.folder)

    def process(self, command, **kwargs):
        self.runs.append((command, kwargs))
        reports = ["LOOKLAB_DIR " + kind + " " + json.dumps(str(self.folder / kind)) for kind in sorted(check.DIRECTORIES)] + ["LOOKLAB_DIRS_END"]
        for line in reports:
            kwargs["guard"].line(line)
        mode_index = next(i for i, value in enumerate(command) if value in RESULTS)
        mode = command[mode_index]
        if len(command) == mode_index + 1:
            code, result = 0, self.results[mode]
        else:
            fault = command[-1]
            code = self.fault_code if self.bad_fault == fault else 1
            result = "LOOKLAB_FAIL " + (self.fault_message if self.bad_fault == fault and self.fault_message else fault) + ": failed check\n"
        graphics = 'LOOKLAB_GRAPHICS {"display":"windows","driver":"vulkan","adapter":"synthetic"}\n' if mode == "ll8-capture" else ""
        return subprocess.CompletedProcess(command, code, "\n".join(reports) + "\n" + graphics + result)

    def run_lane(self, graphics=False):
        with patch.object(check, "run_process", side_effect=self.process), contextlib.redirect_stdout(io.StringIO()):
            (lane.graphics if graphics else lane.run)("synthetic-godot", self.folder / "stage", self.folder, {},
                ["--", "checkout", "scratch"], check.godot_run, check.CheckFailure)

    def test_lane_results_and_specific_faults(self):
        for graphics, mode, faults in ((False, "ll8-shot", lane.SHOT_FAULTS), (True, "ll8-capture", lane.CAPTURE_FAULTS)):
            self.runs.clear(); self.run_lane(graphics)
            self.assertEqual([cmd[cmd.index("scratch") + 1:] for cmd, _ in self.runs], [[mode]] + [[mode, fault] for fault in faults])
            self.assertTrue(all(("--headless" not in cmd) == graphics for cmd, _ in self.runs))
            self.assertTrue(all(kwargs["timeout"] == (600 if graphics else check.GODOT_TIMEOUT) for _, kwargs in self.runs))

    def test_missing_repeated_or_wrong_count_results_fail(self):
        for graphics, mode in ((False, "ll8-shot"), (True, "ll8-capture")):
            result = RESULTS[mode]
            for invalid in ("", result + result, result.replace("checks=1", "checks=0"), result.rstrip() + " extra\n"):
                with self.subTest(mode=mode, invalid=invalid):
                    self.results = dict(RESULTS, **{mode: invalid})
                    with self.assertRaisesRegex(check.CheckFailure, mode):
                        self.run_lane(graphics)

    def test_faults_require_exit_one_and_their_own_id(self):
        for graphics, faults in ((False, lane.SHOT_FAULTS), (True, lane.CAPTURE_FAULTS)):
            for fault in faults:
                for code, message in ((0, None), (2, None), (1, "unrelated")):
                    with self.subTest(fault=fault, code=code, message=message):
                        self.bad_fault, self.fault_code, self.fault_message = fault, code, message
                        with self.assertRaisesRegex(check.CheckFailure, fault):
                            self.run_lane(graphics)

    def source_tree(self, name):
        root = self.folder / name; lab = root / check.LAB
        for relative, data in (("Directory.Build.props", "<Project />"), ("app/project.godot", 'config/name="LookLab"\n'),
                               ("app/Renderer.cs", "renderer one"), ("app/shaders/test.gdshader", "shader"), ("core/Rules.cs", "rules"),
                               ("app/bin/ignored.dll", "ignored"), ("core/obj/cache", "ignored"), ("app/.godot/import", "ignored")):
            path = lab / relative; path.parent.mkdir(parents=True, exist_ok=True); path.write_text(data, encoding="utf-8")
        return root

    def test_source_digest_matches_staged_file_list_and_changes_with_renderer(self):
        a = self.source_tree("a"); b = self.source_tree("b")
        with patch.object(check, "ROOT", a):
            sources = list(check.staged_sources()); digest = check.source_digest()
            temp = self.folder / "staging"; temp.mkdir()
            stage = check.stage_project(temp, self.folder / "packages", "LookLabCheck-test")
            names = [name for name, _ in sources]
            self.assertEqual(names, sorted(names))
            self.assertEqual(names, sorted(p.relative_to(stage).as_posix() for p in stage.rglob("*") if p.is_file() and p.name != "nuget.config"))
            expected = "".join(name + "\0" + hashlib.sha256(path.read_bytes()).hexdigest() + "\n" for name, path in sources)
            self.assertEqual(digest, hashlib.sha256(expected.encode("utf-8")).hexdigest())
            self.assertEqual(names, ["Directory.Build.props", "app/Renderer.cs", "app/project.godot", "app/shaders/test.gdshader", "core/Rules.cs"])
        with patch.object(check, "ROOT", b):
            self.assertEqual(check.source_digest(), digest)
            (b / check.LAB / "app/Renderer.cs").write_text("renderer two", encoding="utf-8")
            self.assertNotEqual(check.source_digest(), digest)

    def test_source_identity_clean_dirty_and_git_contract(self):
        for status in ("", " M tools/looklab/app/Renderer.cs\n"):
            responses = [subprocess.CompletedProcess([], 0, "a" * 40 + "\n"), subprocess.CompletedProcess([], 0, status)]
            with patch.object(lane.subprocess, "run", side_effect=responses) as git, patch.object(check, "source_digest", return_value="b" * 64):
                self.assertEqual(lane.source_identity(check), dict(IDENTITY, clean=not status))
            self.assertEqual(git.call_args_list[0].args[0], ["git", "rev-parse", "HEAD"])
            self.assertEqual(git.call_args_list[1].args[0], ["git", "--no-optional-locks", "status", "--porcelain", "--untracked-files=all", "--", "tools/looklab"])
            for call in git.call_args_list:
                self.assertEqual(call.kwargs, {"cwd": check.ROOT, "env": check.scoped_env(), "capture_output": True, "text": True, "timeout": 30})

    def test_source_identity_git_failures_and_malformed_commit(self):
        for response in (subprocess.CompletedProcess([], 1, ""), subprocess.CompletedProcess([], 0, "A" * 40),
                         subprocess.CompletedProcess([], 0, "short"), OSError("synthetic git failure"), subprocess.TimeoutExpired("git", 30)):
            with self.subTest(response=response), patch.object(lane.subprocess, "run", side_effect=[response]), self.assertRaises(check.CheckFailure):
                lane.source_identity(check)
        with patch.object(lane.subprocess, "run", side_effect=[subprocess.CompletedProcess([], 0, "a" * 40), subprocess.CompletedProcess([], 1, "")]), self.assertRaises(check.CheckFailure):
            lane.source_identity(check)

    @staticmethod
    def png(width=64, height=64):
        def chunk(kind, body):
            return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))
        header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
        rows = (b"\0" + bytes((30, 60, 90, 255)) * width) * height
        return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")

    def fixture(self, clean=True):
        self.number += 1; root = self.folder / ("fixture-" + str(self.number)); root.mkdir()
        def write(relative, data):
            path = root / relative; path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(data)
        shot_data = {"format": "magic600-look-shot", "version": 1, "id": "g2", "name": "fixture", "size": [64, 64], "seconds": .25,
                     "heroFrame": 2, "ease": [1 / 3, 1 / 3], "camera": [{"seconds": .25, "dragX": .5, "dragY": .15}], "turns": []}
        write(lane.SHOT, json.dumps(shot_data).encode())
        for path in (lane.TURN, lane.PARAMETERS, lane.SHADER):
            write(path, ("fixture " + path).encode())
        write("tools/looklab/lane_ll3.py", (Path(__file__).resolve().parents[1] / "lane_ll3.py").read_bytes())
        stems = ["default", "g1-hopf-rings"]; name = "g2-20261022T120000Z-0123abcd"
        identity = dict(IDENTITY, clean=clean); command = "python tools/looklab/check.py --shot " + " ".join(stems)
        folder = root / lane.RUNS / name; mirror = root / lane.GALLERY / name
        digest = lambda relative: hashlib.sha256((root / relative).read_bytes()).hexdigest()
        presets = []
        for stem in stems:
            path = lane.PRESETS + "/" + stem + ".json"; write(path, b"{}")
            still = lane.RUNS + "/" + name + "/" + stem + ".png"; write(still, self.png())
            if clean:
                write(lane.GALLERY + "/" + name + "/" + stem + ".png", self.png())
            presets.append({"stem": stem, "path": path, "sha256": digest(path), "name": stem, "framesRendered": 6,
                            "still": {"file": stem + ".png", "sha256": digest(still)}, "clip": None, "spot": "identical"})
        manifest = {"format": "magic600-look-shot-run", "version": 1, "run": name, "created": "2026-10-22T12:00:00Z", "command": command,
                    "source": dict(identity), "formal": clean,
                    "shot": {"id": "g2", "path": lane.SHOT, "sha256": digest(lane.SHOT), "frames": 6, "frameRate": 24, "heroFrame": 2,
                             "spotFrames": [0, 2, 5], "size": [64, 64], "track": "c" * 64},
                    "inputs": {"modelId": "fixture model", **{key: {"path": path, "sha256": digest(path)} for key, path in
                               (("turn", lane.TURN), ("parameters", lane.PARAMETERS), ("shader", lane.SHADER))}},
                    "tools": {"godot": "fixture Godot", "dotnet": "fixture .NET", "driver": "fixture driver", "adapter": "fixture adapter"},
                    "capture": {"waits": 3, "spotWaits": 8}, "presets": presets, "clips": lane.CLIPS}
        output = "\n".join([*(f"LOOKLAB_SHOT preset={s} still={s}.png spot=identical" for s in stems),
                           f"LOOKLAB_SHOT_RUN manifest={lane.RUNS}/{name}/manifest.json formal={int(clean)}",
                           "LOOKLAB_SHOT_PASS presets=2 stills=2 clips=0 frames=6 size=64x64"]) + "\n"
        return {"root": root, "folder": folder, "mirror": mirror, "manifest": manifest, "identity": identity, "command": command, "stems": stems, "output": output, "name": name}

    def report(self, fixture):
        (fixture["folder"] / "manifest.json").write_text(json.dumps(fixture["manifest"]), encoding="utf-8")
        with patch.object(check, "ROOT", fixture["root"]):
            return lane.shot_report(check, fixture["output"], fixture["stems"], fixture["identity"], fixture["command"])

    def test_shot_report_formal_and_draft(self):
        for clean in (True, False):
            fixture = self.fixture(clean)
            self.assertEqual(self.report(fixture), fixture["name"])

    def test_shot_report_run_preset_and_pass_line_defects(self):
        original = self.fixture()["output"]; a, b, run, passed = original.splitlines(keepends=True)
        invalid = (original.replace(run, ""), original + run, original.replace("manifest.json formal=1", "other.json formal=1"),
                   original.replace("g2-20261022T120000Z-0123abcd", "g2-bad-run"), original.replace(a, ""), original + a,
                   b + a + run + passed, original.replace("still=default.png", "still=other.png"), original.replace("spot=identical", "spot=other", 1),
                   original.replace(passed, ""), original + passed, original.replace("presets=2 stills=2", "presets=1 stills=1"),
                   original.replace("frames=6", "frames=5"), original.replace("size=64x64", "size=64x62"), original.replace("clips=0", "clips=1"),
                   original.replace("formal=1", "formal=0"), original + run.rstrip() + " extra\n")
        for output in invalid:
            with self.subTest(output=output):
                fixture = self.fixture(); fixture["output"] = output
                with self.assertRaisesRegex(check.CheckFailure, "shot:"):
                    self.report(fixture)

    def test_shot_report_manifest_field_and_digest_defects(self):
        defects = [(('shot', 'sha256'), '0' * 64), (('shot', 'track'), 'invalid'), (('shot', 'frames'), 240), (('shot', 'heroFrame'), 3),
                   (('shot', 'spotFrames'), [0, 3, 5]), (('shot', 'size'), [1600, 1000]), (('presets', 0, 'sha256'), '0' * 64),
                   (('presets', 0, 'still', 'sha256'), '0' * 64), (('presets', 0, 'clip'), 'default.mp4'), (('presets', 0, 'spot'), 'different'),
                   (('presets', 0, 'still', 'file'), '../default.png'), (('presets', 0, 'framesRendered'), 240), (('presets', 0, 'name'), None),
                   (('source', 'commit'), 'd' * 40), (('source', 'staged'), 'e' * 64), (('source', 'clean'), False), (('command',), 'other command'),
                   (('formal',), False), (('formal',), 1), (('format',), 'other'), (('version',), 2), (('run',), 'other'),
                   (('created',), '2026-10-22T12:00:01Z'), (('capture', 'waits'), 2), (('capture', 'spotWaits'), 3), (('clips',), 'made clips'),
                   (('inputs', 'modelId'), ''), *[(("inputs", k, "sha256"), '0' * 64) for k in ('turn', 'parameters', 'shader')],
                   *[(("tools", k), '') for k in ('godot', 'dotnet', 'driver', 'adapter')]]
        for path, value in defects:
            with self.subTest(path=path, value=value):
                fixture = self.fixture(); target = fixture["manifest"]
                for key in path[:-1]:
                    target = target[key]
                target[path[-1]] = value
                with self.assertRaisesRegex(check.CheckFailure, "shot:"):
                    self.report(fixture)

    def test_shot_report_extra_or_missing_keys_at_every_level(self):
        for path in ((), ('source',), ('shot',), ('inputs',), ('inputs', 'turn'), ('tools',), ('capture',), ('presets', 0), ('presets', 0, 'still')):
            for extra in (True, False):
                with self.subTest(path=path, extra=extra):
                    fixture = self.fixture(); target = fixture["manifest"]
                    for key in path:
                        target = target[key]
                    if extra:
                        target["extra"] = True
                    else:
                        del target[next(iter(target))]
                    with self.assertRaisesRegex(check.CheckFailure, "shot:"):
                        self.report(fixture)

    def test_shot_report_invalid_png_and_run_files(self):
        fixture = self.fixture(); path = fixture["folder"] / "default.png"; path.write_bytes(b"invalid PNG")
        fixture["manifest"]["presets"][0]["still"]["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        with self.assertRaisesRegex(check.CheckFailure, "shot:.*PNG"):
            self.report(fixture)
        fixture = self.fixture(); (fixture["folder"] / "extra.bin").write_bytes(b"extra")
        with self.assertRaisesRegex(check.CheckFailure, "shot:"):
            self.report(fixture)
        fixture = self.fixture(); (fixture["folder"] / "default.png").unlink()
        with self.assertRaisesRegex(check.CheckFailure, "shot:"):
            self.report(fixture)

    def test_shot_report_gallery_must_match_formal_state_and_bytes(self):
        fixture = self.fixture(); shutil.rmtree(fixture["mirror"])
        with self.assertRaisesRegex(check.CheckFailure, "shot:"):
            self.report(fixture)
        fixture = self.fixture(False); fixture["mirror"].mkdir(parents=True)
        with self.assertRaisesRegex(check.CheckFailure, "shot:"):
            self.report(fixture)
        fixture = self.fixture(); (fixture["mirror"] / "default.png").write_bytes(b"different")
        with self.assertRaisesRegex(check.CheckFailure, "shot:"):
            self.report(fixture)
        fixture = self.fixture(); (fixture["mirror"] / "extra.bin").write_bytes(b"extra")
        with self.assertRaisesRegex(check.CheckFailure, "shot:"):
            self.report(fixture)


if __name__ == "__main__":
    loader = unittest.TestLoader()
    suite = loader.loadTestsFromTestCase(CurrentHarnessTests)
    suite.addTests(previous.previous.previous.LL5HarnessTests(name) for name in loader.getTestCaseNames(previous.previous.previous.LL5HarnessTests)
                   if name.startswith("test_ll5_") and name != "test_ll5_combined_schedules_keep_existing_checks")
    suite.addTests(loader.loadTestsFromTestCase(previous.previous.LL6HarnessTests))
    suite.addTests(loader.loadTestsFromTestCase(previous.LL7HarnessTests))
    suite.addTests(loader.loadTestsFromTestCase(LL8HarnessTests))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    raise SystemExit(0 if result.wasSuccessful() else 1)
