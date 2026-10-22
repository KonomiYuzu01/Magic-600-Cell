"""Retain the LL6 harness and prove the final LL7 and stills schedules."""
import contextlib
import io
import json
from pathlib import Path
import runpy
import subprocess
import unittest
from unittest.mock import patch
import ll6_harness_test as previous


check = previous.previous.check
lane = previous.module("looklab_ll7", "lane_ll7.py")
RESULTS = {
    "ll7-presets": "LOOKLAB_LL7_PRESETS_PASS presets=7 kinds=6 uniforms=bound checks=1\n",
    "ll7-spin": "LOOKLAB_LL7_SPIN_PASS global=passed inertia=passed local=passed compare=2 checks=1\n",
    "ll7-layout": "LOOKLAB_LL7_LAYOUT_PASS kinds=4 classes=5 texture=exact palette=passed checks=1\n",
    "ll7-framing": "LOOKLAB_LL7_FRAMING_PASS kinds=3 settled=1e-6 global=neutral checks=1\n",
}
FAULTS = [
    ["ll7-presets", "ll7-projection-clamped"],
    ["ll7-spin", "ll7-compare-spin-not-copied"],
    ["ll7-layout", "ll7-palette-old-colouring"],
    ["ll7-framing", "ll7-framing-ignores-spin"],
]
GEOMETRY = ("LOOKLAB_GEOMETRY_PASS cases=10 samples=3000 framed=1 target=RGBA32F\n"
            "LOOKLAB_LAYOUT_GEOMETRY_PASS cases=6 samples=1800 target=RGBA32F\n"
            "LOOKLAB_DRAW_COUNT_PASS instances=600 visible=600 primitives=6096000\n")


class LL7HarnessTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(__file__).resolve().parents[1]
        self.runs = []
        self.fault_code, self.fault_message, self.bad_fault = 1, None, None
        self.results = dict(RESULTS)

    def process(self, command, **kwargs):
        self.runs.append(command)
        self.assertIn("--headless", command)
        lines = ["LOOKLAB_DIR " + kind + " " + json.dumps(str(self.root / kind)) for kind in sorted(check.DIRECTORIES)]
        lines.append("LOOKLAB_DIRS_END")
        for line in lines:
            kwargs["guard"].line(line)
        mode_index = next(i for i, value in enumerate(command) if value in RESULTS)
        mode = command[mode_index]
        if len(command) == mode_index + 1:
            code, result = 0, self.results[mode]
        else:
            fault = command[-1]
            code = self.fault_code if self.bad_fault == fault else 1
            result = "LOOKLAB_FAIL " + (self.fault_message if self.bad_fault == fault and self.fault_message else fault) + ": failed check\n"
        return subprocess.CompletedProcess(command, code, "\n".join(lines) + "\n" + result)

    def run_lane(self):
        with patch.object(check, "run_process", side_effect=self.process), contextlib.redirect_stdout(io.StringIO()):
            lane.run("synthetic-godot", self.root / "synthetic-stage", self.root, {},
                     ["--", "checkout", "scratch"], check.godot_run, check.CheckFailure)

    def test_four_results_and_four_specific_faults(self):
        self.run_lane()
        self.assertEqual([command[command.index("scratch") + 1:] for command in self.runs],
                         [[mode] for mode in RESULTS] + FAULTS)

    def test_missing_repeated_or_wrong_count_results_fail(self):
        for mode, result in RESULTS.items():
            for invalid in ("", result + result, result.replace("checks=1", "checks=0")):
                with self.subTest(mode=mode, invalid=invalid):
                    self.results = dict(RESULTS, **{mode: invalid})
                    with self.assertRaisesRegex(check.CheckFailure, mode):
                        self.run_lane()

    def test_each_fault_requires_exit_one(self):
        for _, fault in FAULTS:
            for code in (0, 2):
                with self.subTest(fault=fault, code=code):
                    self.bad_fault, self.fault_code = fault, code
                    with self.assertRaisesRegex(check.CheckFailure, fault):
                        self.run_lane()

    def test_each_fault_requires_its_own_failure(self):
        for _, fault in FAULTS:
            with self.subTest(fault=fault):
                self.bad_fault, self.fault_message = fault, "unrelated"
                with self.assertRaisesRegex(check.CheckFailure, fault):
                    self.run_lane()


class CurrentHarnessTests(previous.CurrentHarnessTests):
    def schedule(self, mode, geometry=GEOMETRY, stills=""):
        godot = self.folder / check.GODOT_EXE
        (self.folder / "GodotSharp/Tools/nupkgs").mkdir(parents=True, exist_ok=True)
        calls, extensions = [], []

        def process(command, **kwargs):
            return subprocess.CompletedProcess(command, 0, check.GODOT_VERSION + "\n" if "--version" in command else "")

        def runner(executable, stage, temporary, env, arguments, **kwargs):
            calls.append((arguments, kwargs))
            output = ("LOOKLAB_STAGE0_CPU_PASS\nLOOKLAB_STAGE0_GPU_PASS\n"
                      "LOOKLAB_SMOKE_PASS instances=600 vertices=30480 slots=259800 parameters=42 bytes=123\n"
                      + geometry + stills + "".join(RESULTS.values()) + "".join(previous.RESULTS.values())
                      + "".join(previous.previous.RESULTS.values()))
            return subprocess.CompletedProcess(arguments, 0, output)

        real_run_path = runpy.run_path

        def extension(path):
            name = Path(path).name
            if name in ("lane_ll5.py", "lane_ll6.py", "lane_ll7.py"):
                actual = real_run_path(path)
                def call(key, *args):
                    extensions.append((name, key))
                    return actual[key](*args)
                return {key: (lambda *args, key=key: call(key, *args)) for key in ("headless", "graphics", "run")}
            return {key: (lambda *args, key=key: extensions.append((name, key))) for key in ("headless", "graphics", "run")}

        with patch.object(check, "find_godot", return_value=godot), patch.object(check, "stage_project", return_value=self.folder), \
             patch.object(check, "audit_packages"), patch.object(check, "run_process", side_effect=process), \
             patch.object(check, "godot_run", side_effect=runner), patch("runpy.run_path", side_effect=extension), \
             contextlib.redirect_stdout(io.StringIO()):
            check.godot_lane(mode, self.folder, {"NUGET_PACKAGES": str(self.folder)}, {})
        return calls, extensions

    def test_cpu_and_gpu_lane_schedules(self):
        calls, extensions = self.schedule("--godot")
        self.assertEqual(extensions, [("lane_ll3.py", "headless"), ("lane_ll4.py", "run"), ("lane_ll5.py", "headless"),
                                      ("lane_ll6.py", "run"), ("lane_ll7.py", "run")])
        self.assertEqual(len(calls), 29)
        self.assertTrue(all("--headless" in args for args, _ in calls))
        self.assertEqual([kwargs["expected_failure"] for _, kwargs in calls if "expected_failure" in kwargs],
                         ["skip-parameter", "wrong-instance-count", "changed-preset-byte", "ll5-disable-cvd", "ll5-disable-null-cost"]
                         + [fault for _, fault in previous.FAULTS] + [fault for _, fault in FAULTS])
        calls, extensions = self.schedule("--godot-gpu")
        self.assertEqual(extensions, [("lane_ll3.py", "graphics"), ("lane_ll5.py", "graphics")])
        self.assertEqual(len(calls), 10)
        self.assertEqual(calls[0][0], ["--headless", "--import"])
        self.assertTrue(all("--headless" not in args and kwargs["graphics"] for args, kwargs in calls[1:]))
        self.assertEqual([kwargs["expected_failure"] for _, kwargs in calls if "expected_failure" in kwargs],
                         ["float-offset", "transpose-q", "missing-instance", "missing-view-frame", "layout-no-blend", "ll5-wrong-matrix"])

    def test_gpu_geometry_requires_both_lines(self):
        for line in GEOMETRY.splitlines(keepends=True):
            with self.subTest(line=line), self.assertRaisesRegex(check.CheckFailure, "geometry or full draw-count"):
                self.schedule("--godot-gpu", geometry=GEOMETRY.replace(line, ""))

    def stills_fixture(self):
        root = self.folder / "checkout"
        presets = root / check.LAB / "presets"; presets.mkdir(parents=True)
        captures = root / "work/loop-memory/looklab/captures"; captures.mkdir(parents=True)
        lines = []
        names = ("hemispheres", "hopf-rings", "orthographic", "perspective", "ring-sheet", "stereographic", "stereographic-cap")
        for name in names:
            preset = "g1-" + name
            (presets / (preset + ".json")).write_text("{}", encoding="utf-8")
            for pose in ("rest", "mid"):
                path = captures / ("capture-" + preset + "-" + pose + ".png")
                path.write_bytes(b"synthetic capture fixture")
                lines.append(f"LOOKLAB_STILL preset={preset} pose={pose} path={path.relative_to(root).as_posix()}")
        lines.append("LOOKLAB_STILLS_PASS presets=7 stills=14 size=1600x1000")
        return root, "\n".join(lines) + "\n"

    def test_stills_schedule_uses_real_root_and_graphics(self):
        root, output = self.stills_fixture()
        with patch.object(check, "ROOT", root):
            calls, extensions = self.schedule("--stills", stills=output)
        self.assertEqual(extensions, [])
        self.assertEqual(calls, [(["--headless", "--import"], {}),
                                 (["--", str(root), str(self.folder), "stills"], {"graphics": True})])

    def test_stills_skip_core_and_preserve_graphics_unavailable_exit(self):
        with patch.object(check, "core_check") as core, patch.object(check, "godot_lane") as lane, \
             patch.object(check, "snapshot", return_value={}), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.main(["--stills"]), 0)
            self.assertEqual(lane.call_args.args[0], "--stills")
            core.assert_not_called()
            lane.side_effect = check.GraphicsUnavailable("graphics lane not run: window unavailable")
            self.assertEqual(check.main(["--stills"]), 3)

    def test_stills_missing_repeated_malformed_lines_or_pass_fail(self):
        root, output = self.stills_fixture()
        first, *rest = output.splitlines(keepends=True)
        last = rest[-1]
        invalid = ("".join(rest), output + first, output.replace("pose=rest", "pose=other", 1),
                   output.replace("size=1600x1000", "size=800x500"), output.replace(last, ""), output + last,
                   output + first.rstrip() + " extra\n")
        with patch.object(check, "ROOT", root):
            check.stills_report(output)
            for text in invalid:
                with self.subTest(text=text), self.assertRaisesRegex(check.CheckFailure, "stills:"):
                    check.stills_report(text)

    def test_stills_missing_capture_or_path_outside_root_fails(self):
        root, output = self.stills_fixture()
        path = next((root / "work/loop-memory/looklab/captures").iterdir())
        path.unlink()
        with patch.object(check, "ROOT", root), self.assertRaisesRegex(check.CheckFailure, "missing capture"):
            check.stills_report(output)
        outside = "work/loop-memory/looklab/captures/capture-../../../../../outside.png"
        first_path = output.splitlines()[0].split("path=")[1]
        with patch.object(check, "ROOT", root), self.assertRaisesRegex(check.CheckFailure, "missing capture"):
            check.stills_report(output.replace(first_path, outside))

    def test_stills_allow_only_app_output_changes_including_ignored(self):
        before = {"./": "directory", "work/": "directory", "tools/a.txt": "source"}
        saved = dict(before, **{"work/loop-memory/": "directory", "work/loop-memory/looklab/": "directory",
                               "work/loop-memory/looklab/captures/capture-x.png": "capture",
                               "work/gallery/": "directory", "work/gallery/looklab/capture-x.png": "mirror"})
        for after, expected in ((saved, 0), (dict(saved, **{"tools/a.txt": "changed"}), 1),
                                (dict(saved, **{"ignored.bin": "unexpected"}), 1),
                                (dict(saved, **{"work/gallery/other.png": "unexpected"}), 1)):
            with patch.object(check, "core_check"), patch.object(check, "godot_lane"), \
                 patch.object(check, "snapshot", side_effect=[before, after]), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(check.main(["--stills"]), expected)


if __name__ == "__main__":
    loader = unittest.TestLoader()
    suite = loader.loadTestsFromTestCase(CurrentHarnessTests)
    suite.addTests(previous.previous.LL5HarnessTests(name) for name in loader.getTestCaseNames(previous.previous.LL5HarnessTests)
                   if name.startswith("test_ll5_") and name != "test_ll5_combined_schedules_keep_existing_checks")
    suite.addTests(loader.loadTestsFromTestCase(previous.LL6HarnessTests))
    suite.addTests(loader.loadTestsFromTestCase(LL7HarnessTests))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    raise SystemExit(0 if result.wasSuccessful() else 1)
