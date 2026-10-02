"""Runs the Taste Lab JavaScript tests (core and geometry) and checks the geometry fixture."""
from __future__ import annotations

import shutil
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NODE = shutil.which("node")
SUITES = ("tools/tastelab/core/tests", "tools/tastelab/page/tests")


@unittest.skipIf(NODE is None, "Node.js is not installed; the Taste Lab JavaScript tests were not run")
class TasteLabTests(unittest.TestCase):
    def run_suite(self, folder: str) -> None:
        files = sorted((ROOT / folder).glob("*.test.mjs"))
        self.assertTrue(files, f"no tests in {folder}")
        result = subprocess.run([NODE, "--test", *[f.relative_to(ROOT).as_posix() for f in files]],
                                cwd=ROOT, capture_output=True, text=True, timeout=600)
        self.assertEqual(result.returncode, 0, result.stdout[-4000:] + result.stderr[-2000:])

    def test_core(self):
        self.run_suite(SUITES[0])

    def test_geometry(self):
        self.run_suite(SUITES[1])


class GeometryFixtureTests(unittest.TestCase):
    def test_fixture_matches_its_builder(self):
        result = subprocess.run([sys.executable, "tools/tastelab/sim/geometry_fixture.py", "--check"],
                                cwd=ROOT, capture_output=True, text=True, timeout=600)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
