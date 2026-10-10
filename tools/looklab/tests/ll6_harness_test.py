"""LL6 scheduling, result counts and each negative control without Godot."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch
import runpy
import ll5_harness_test as previous


def module(name, relative):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parents[1] / relative)
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


check = module("looklab_check_ll6", "check.py")
lane = module("looklab_ll6", "lane_ll6.py")
RESULTS = {
    "ll6-views": "LOOKLAB_LL6_VIEWS_PASS layouts=4 sizes=2 modes=2 placements=2 arrangements=32 hidden=2 checks=1\n",
    "ll6-link": "LOOKLAB_LL6_LINK_PASS drags=2 steps=9 boundaries=2 checks=1\n",
    "ll6-colour": "LOOKLAB_LL6_COLOUR_PASS modes=4 states=12 late=passed checks=1\n",
    "ll6-framing": "LOOKLAB_LL6_FRAMING_PASS settled=1e-6 steps=120 caps=passed global=neutral checks=1\n",
}
FAULTS = [
    ["ll6-views", "ll6-local-hidden"],
    ["ll6-views", "ll6-panel-covers-local"],
    ["ll6-link", "ll6-local-drag-ignored"],
    ["ll6-link", "ll6-local-clock"],
    ["ll6-colour", "ll6-global-only-cvd"],
    ["ll6-framing", "ll6-neutral-local-frame"],
]


class LL6HarnessTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(__file__).resolve().parents[1]
        self.stage = self.root / "synthetic-stage"
        self.runs = []
        self.fault_code, self.fault_message, self.bad_fault = 1, None, None
        self.results = dict(RESULTS)

    def process(self, command, **kwargs):
        self.runs.append(command)
        self.assertIn("--headless", command)
        guard = kwargs["guard"]
        lines = ["LOOKLAB_DIR " + kind + " " + json.dumps(str(self.root / kind)) for kind in sorted(check.DIRECTORIES)]
        lines.append("LOOKLAB_DIRS_END")
        for line in lines:
            guard.line(line)
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
            lane.run("synthetic-godot", self.stage, self.root, {}, ["--", "checkout", "scratch"], check.godot_run, check.CheckFailure)

    def test_four_results_and_six_specific_faults(self):
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


class CurrentHarnessTests(previous.harness_test.HarnessTests):
    def test_cpu_and_gpu_lane_schedules(self):
        # The LL2/LL5 fixtures pin the old schedules. Retain their other cases
        # unchanged and verify the final combined schedule here.
        current = previous.check
        godot = self.folder / current.GODOT_EXE
        (self.folder / "GodotSharp/Tools/nupkgs").mkdir(parents=True)
        calls, extensions = [], []

        def process(command, **kwargs):
            return subprocess.CompletedProcess(command, 0, current.GODOT_VERSION + "\n" if "--version" in command else "")

        def runner(executable, stage, temporary, env, arguments, **kwargs):
            calls.append((arguments, kwargs))
            output = ("LOOKLAB_STAGE0_CPU_PASS\nLOOKLAB_STAGE0_GPU_PASS\n"
                      "LOOKLAB_SMOKE_PASS instances=600 vertices=30480 slots=259800 parameters=42 bytes=123\n"
                      "LOOKLAB_GEOMETRY_PASS cases=10 samples=3000 framed=1 target=RGBA32F\n"
                      "LOOKLAB_DRAW_COUNT_PASS instances=600 visible=600 primitives=6096000\n"
                      + "".join(RESULTS.values()) + "".join(previous.RESULTS.values()))
            return subprocess.CompletedProcess(arguments, 0, output)

        real_run_path = runpy.run_path

        def extension(path):
            name = Path(path).name
            if name in ("lane_ll5.py", "lane_ll6.py"):
                actual = real_run_path(path)
                def call(key, *args):
                    extensions.append((name, key))
                    return actual[key](*args)
                return {key: (lambda *args, key=key: call(key, *args)) for key in ("headless", "graphics", "run")}
            return {key: (lambda *args, key=key: extensions.append((name, key))) for key in ("headless", "graphics", "run")}

        with patch.object(current, "find_godot", return_value=godot), patch.object(current, "stage_project", return_value=self.folder), \
             patch.object(current, "audit_packages"), patch.object(current, "run_process", side_effect=process), \
             patch.object(current, "godot_run", side_effect=runner), patch("runpy.run_path", side_effect=extension), \
             contextlib.redirect_stdout(io.StringIO()):
            current.godot_lane("--godot", self.folder, {"NUGET_PACKAGES": str(self.folder)}, {})
            self.assertEqual(extensions, [("lane_ll3.py", "headless"), ("lane_ll4.py", "run"), ("lane_ll5.py", "headless"), ("lane_ll6.py", "run")])
            self.assertEqual(len(calls), 21)
            self.assertTrue(all("--headless" in args for args, _ in calls))
            self.assertEqual([kwargs["expected_failure"] for _, kwargs in calls if "expected_failure" in kwargs],
                             ["skip-parameter", "wrong-instance-count", "changed-preset-byte", "ll5-disable-cvd", "ll5-disable-null-cost"]
                             + [fault for _, fault in FAULTS])
            calls.clear(); extensions.clear()
            current.godot_lane("--godot-gpu", self.folder, {"NUGET_PACKAGES": str(self.folder)}, {})
            self.assertEqual(extensions, [("lane_ll3.py", "graphics"), ("lane_ll5.py", "graphics")])
            self.assertEqual(len(calls), 9)
            self.assertEqual(calls[0][0], ["--headless", "--import"])
            self.assertTrue(all("--headless" not in args and kwargs["graphics"] for args, kwargs in calls[1:]))
            self.assertEqual([kwargs["expected_failure"] for _, kwargs in calls if "expected_failure" in kwargs],
                             ["float-offset", "transpose-q", "missing-instance", "missing-view-frame", "ll5-wrong-matrix"])


if __name__ == "__main__":
    loader = unittest.TestLoader()
    suite = loader.loadTestsFromTestCase(CurrentHarnessTests)
    suite.addTests(previous.LL5HarnessTests(name) for name in loader.getTestCaseNames(previous.LL5HarnessTests)
                   if name.startswith("test_ll5_") and name != "test_ll5_combined_schedules_keep_existing_checks")
    suite.addTests(loader.loadTestsFromTestCase(LL6HarnessTests))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    raise SystemExit(0 if result.wasSuccessful() else 1)
