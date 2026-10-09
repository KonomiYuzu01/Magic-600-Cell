"""Retain LL2 assertions and verify LL5's appended schedules and fault gates."""
import contextlib
import io
from pathlib import Path
import runpy
import subprocess
import unittest
from unittest.mock import patch

import harness_test

check = harness_test.check
lane = runpy.run_path(str(Path(__file__).resolve().parents[1] / "lane_ll5.py"))
RESULTS = {
    "ll5-cvd": "LOOKLAB_LL5_CVD_PASS modes=4 samples=52 backgrounds=5 tolerance=2e-6-linear\n",
    "ll5-cost": "LOOKLAB_LL5_COST_PASS nulls=10 measured=1 source=shown live=passed\n",
    "ll5-palette": "LOOKLAB_LL5_PALETTE_PASS modes=4 pairs=15 same_ring=600 gamut_failures=3 thresholds=caller\n",
    "ll5-cvd-gpu": "LOOKLAB_LL5_CVD_GPU_PASS modes=4 samples=52 target=RGBA32F tolerance=2e-6-linear\n",
}


class LL2HarnessTests(harness_test.HarnessTests):
    def test_cpu_and_gpu_lane_schedules(self):
        # This original fixture models only LL2. Keep all its assertions, with
        # the extension isolated; the combined schedules are checked below.
        with patch("runpy.run_path", return_value={"headless": lambda *args: None, "graphics": lambda *args: None}):
            super().test_cpu_and_gpu_lane_schedules()


class LL5HarnessTests(harness_test.HarnessTests):
    # Reuse the isolated folder and directory reports, without registering the
    # inherited LL2 test methods twice.
    def execute(self, graphics=False, bad_fault=None, bad_code=0, result_override=None):
        calls = []

        def process(command, **kwargs):
            arguments = command[command.index("--") + 3:]
            mode = arguments[0]
            fault = arguments[1] if len(arguments) > 1 else None
            reports = "".join(self.reports())
            for line in reports.splitlines():
                kwargs["guard"].line(line)
            evidence = 'LOOKLAB_GRAPHICS {"display":"windows","driver":"vulkan","adapter":"synthetic"}\n' if graphics else ""
            if fault:
                code, output = (bad_code, "LOOKLAB_FAIL unrelated\n") if fault == bad_fault else (1, "LOOKLAB_FAIL " + fault + ": mismatch\n")
            else:
                code, output = 0, (result_override or {}).get(mode, RESULTS[mode])
            calls.append(command)
            return subprocess.CompletedProcess(command, code, reports + evidence + output)

        with patch.object(check, "run_process", side_effect=process), contextlib.redirect_stdout(io.StringIO()):
            lane["graphics" if graphics else "headless"]("fake", self.folder, self.folder, {},
                ["--", "root", "scratch"], check.godot_run, check.CheckFailure)
        return calls

    def test_ll5_cpu_schedule_and_own_faults(self):
        calls = self.execute()
        self.assertEqual([c[c.index("--") + 3:] for c in calls], [
            ["ll5-cvd"], ["ll5-cvd", "ll5-disable-cvd"],
            ["ll5-cost"], ["ll5-cost", "ll5-disable-null-cost"], ["ll5-palette"]])
        self.assertTrue(all("--headless" in c for c in calls))

    def test_ll5_disabled_cvd_must_fail_its_check(self):
        for code in (0, 1):
            with self.assertRaisesRegex(check.CheckFailure, "did not fail its own"):
                self.execute(bad_fault="ll5-disable-cvd", bad_code=code)

    def test_ll5_disabled_null_branch_must_fail_its_check(self):
        for code in (0, 1):
            with self.assertRaisesRegex(check.CheckFailure, "did not fail its own"):
                self.execute(bad_fault="ll5-disable-null-cost", bad_code=code)

    def test_ll5_gpu_schedule_and_wrong_matrix(self):
        calls = self.execute(graphics=True)
        self.assertEqual([c[c.index("--") + 3:] for c in calls], [["ll5-cvd-gpu"], ["ll5-cvd-gpu", "ll5-wrong-matrix"]])
        self.assertTrue(all("--headless" not in c for c in calls))

    def test_ll5_wrong_matrix_must_fail_its_check(self):
        for code in (0, 1):
            with self.assertRaisesRegex(check.CheckFailure, "did not fail its own"):
                self.execute(graphics=True, bad_fault="ll5-wrong-matrix", bad_code=code)

    def test_ll5_missing_repeated_and_wrong_counts_refused(self):
        for mode, output in RESULTS.items():
            for malformed in ("", output + output, output.replace("modes=4", "modes=3").replace("nulls=10", "nulls=9")):
                with self.assertRaisesRegex(check.CheckFailure, "LL5 result"):
                    self.execute(graphics=mode == "ll5-cvd-gpu", result_override={mode: malformed})

    def test_ll5_combined_schedules_keep_existing_checks(self):
        godot = self.folder / check.GODOT_EXE
        (self.folder / "GodotSharp/Tools/nupkgs").mkdir(parents=True)
        calls = []

        def process(command, **kwargs):
            return subprocess.CompletedProcess(command, 0, check.GODOT_VERSION + "\n" if "--version" in command else "")

        def runner(executable, stage, temporary, env, arguments, **kwargs):
            calls.append((arguments, kwargs))
            output = ("LOOKLAB_STAGE0_CPU_PASS\nLOOKLAB_STAGE0_GPU_PASS\n"
                "LOOKLAB_SMOKE_PASS instances=600 vertices=30480 slots=259800 parameters=42 bytes=123\n"
                "LOOKLAB_GEOMETRY_PASS cases=9 samples=2700 target=RGBA32F\n"
                "LOOKLAB_DRAW_COUNT_PASS instances=600 visible=600 primitives=6096000\n" + "".join(RESULTS.values()))
            return subprocess.CompletedProcess(arguments, 0, output)

        real_run_path = runpy.run_path

        def only_ll5(path):
            # The other packets' lanes have their own schedule tests.
            if Path(path).name == "lane_ll5.py":
                return real_run_path(path)
            return {key: (lambda *args: None) for key in ("headless", "graphics", "run")}

        with patch.object(check, "find_godot", return_value=godot), patch.object(check, "stage_project", return_value=self.folder), \
             patch.object(check, "audit_packages"), patch.object(check, "run_process", side_effect=process), \
             patch.object(check, "godot_run", side_effect=runner), patch("runpy.run_path", side_effect=only_ll5), \
             contextlib.redirect_stdout(io.StringIO()):
            check.godot_lane("--godot", self.folder, {"NUGET_PACKAGES": str(self.folder)}, {})
            self.assertEqual(len(calls), 11)
            self.assertTrue(all("--headless" in args for args, _ in calls))
            self.assertEqual([kwargs["expected_failure"] for _, kwargs in calls if "expected_failure" in kwargs],
                ["skip-parameter", "wrong-instance-count", "changed-preset-byte", "ll5-disable-cvd", "ll5-disable-null-cost"])
            calls.clear()
            check.godot_lane("--godot-gpu", self.folder, {"NUGET_PACKAGES": str(self.folder)}, {})
            self.assertEqual(len(calls), 8)
            self.assertTrue(all("--headless" not in args and kwargs["graphics"] for args, kwargs in calls[1:]))
            self.assertEqual([kwargs["expected_failure"] for _, kwargs in calls if "expected_failure" in kwargs],
                ["float-offset", "transpose-q", "missing-instance", "ll5-wrong-matrix"])


if __name__ == "__main__":
    loader = unittest.TestLoader()
    suite = loader.loadTestsFromTestCase(LL2HarnessTests)
    suite.addTests(LL5HarnessTests(name) for name in loader.getTestCaseNames(LL5HarnessTests) if name.startswith("test_ll5_"))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    raise SystemExit(0 if result.wasSuccessful() else 1)
