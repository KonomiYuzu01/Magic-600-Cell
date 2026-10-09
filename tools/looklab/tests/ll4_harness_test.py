"""LL4 scheduling and fault requirements without Godot or nested builds."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch


def module(name, relative):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parents[1] / relative)
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


check = module("looklab_check", "check.py")
lane = module("looklab_ll4", "lane_ll4.py")
RESULTS = {
    "ll4-metrics": "LOOKLAB_LL4_METRICS_PASS steps=14 checks=1\n",
    "ll4-refusal": "LOOKLAB_LL4_REFUSAL_PASS command=target.next.activate context=any checks=1\n",
    "ll4-greybox": "LOOKLAB_LL4_GREYBOX_PASS layouts=2 contexts=16 commands=86 placements=180 checks=1\n",
}


class LL4HarnessTests(unittest.TestCase):
    def setUp(self):
        # No files are created: this exercises the real result/directory checks.
        self.root = Path(__file__).resolve().parents[1]
        self.stage = self.root / "synthetic-stage"
        self.runs = []
        self.fault_code, self.fault_message = 1, None
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
            code = self.fault_code
            result = "LOOKLAB_FAIL " + (self.fault_message or fault) + ": failed check\n"
        return subprocess.CompletedProcess(command, code, "\n".join(lines) + "\n" + result)

    def run_lane(self):
        with patch.object(check, "run_process", side_effect=self.process), contextlib.redirect_stdout(io.StringIO()):
            lane.run("synthetic-godot", self.stage, self.root, {}, ["--", "checkout", "scratch"], check.godot_run, check.CheckFailure)

    def test_three_results_and_two_specific_faults(self):
        self.run_lane()
        self.assertEqual([command[command.index("scratch") + 1:] for command in self.runs], [
            ["ll4-metrics"], ["ll4-refusal"], ["ll4-greybox"],
            ["ll4-refusal", "skip-flow-refusal"], ["ll4-greybox", "skip-catalogue-filter"],
        ])

    def test_missing_repeated_or_wrong_count_results_fail(self):
        for mode, result in RESULTS.items():
            for invalid in ("", result + result, result.replace("checks=1", "checks=0")):
                with self.subTest(mode=mode, invalid=invalid):
                    self.results = dict(RESULTS, **{mode: invalid})
                    with self.assertRaisesRegex(check.CheckFailure, mode):
                        self.run_lane()

    def test_fault_exit_zero_is_not_a_pass(self):
        self.fault_code = 0
        with self.assertRaisesRegex(check.CheckFailure, "skip-flow-refusal"):
            self.run_lane()

    def test_unrelated_failure_is_not_a_negative_control(self):
        self.fault_message = "unrelated"
        with self.assertRaisesRegex(check.CheckFailure, "skip-flow-refusal"):
            self.run_lane()

    def test_catalogue_fault_also_requires_failure(self):
        original = self.process

        def disabled_fault(command, **kwargs):
            self.fault_code = 0 if command[-1] == "skip-catalogue-filter" else 1
            return original(command, **kwargs)

        with patch.object(check, "run_process", side_effect=disabled_fault), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(check.CheckFailure, "skip-catalogue-filter"):
                lane.run("synthetic-godot", self.stage, self.root, {}, ["--", "checkout", "scratch"], check.godot_run, check.CheckFailure)


if __name__ == "__main__":
    unittest.main(verbosity=2)
