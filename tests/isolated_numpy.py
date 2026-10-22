"""Run a NumPy test module in one isolated worker process.

tests/test_jumbling_differential.py proves that its acceptance checks never
load NumPy or the J1 simulator. A test module that needs them sets
`_IN_PROCESS = __name__ == "__main__"` and returns `suite(...)` from
`load_tests` when `_IN_PROCESS` is false. Loaded by a shared unittest
command, its tests then run in a fresh worker and the caller stays free of
NumPy. Run directly (`python tests/<module>.py`, as the CI runner does), the
module keeps its tests in process.
"""

import sys

sys.dont_write_bytecode = True

import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[1]

_WORKER = "\n".join((
    "import importlib, json, sys, unittest",
    "module = importlib.import_module('tests.' + sys.argv[1])",
    "module._IN_PROCESS = True",
    "suite = unittest.defaultTestLoader.loadTestsFromModule(module)",
    "result = unittest.TextTestRunner(verbosity=0).run(suite)",
    "print(json.dumps({'tests': result.testsRun, 'skips': len(result.skipped)}))",
    "raise SystemExit(not result.wasSuccessful())",
))


def run(name, reason):
    """Run tests/<name>.py in a worker; skip with `reason` if it lacks NumPy."""
    # No time limit here: the CI runner limits each check, and a hard limit
    # close to the measured time would fail on a slower machine.
    completed = subprocess.run([sys.executable, "-B", "-c", _WORKER, name], cwd=ROOT,
                               capture_output=True, text=True)
    if completed.returncode:
        raise AssertionError(completed.stdout + completed.stderr)
    report = json.loads(completed.stdout.splitlines()[-1])
    if report["tests"] == 0 and report["skips"]:
        raise unittest.SkipTest(reason)
    if report["skips"]:
        raise AssertionError(f"the {name} worker skipped {report['skips']} test(s)")


def suite(name, description, reason):
    return unittest.TestSuite([unittest.FunctionTestCase(lambda: run(name, reason),
                                                         description=description)])
