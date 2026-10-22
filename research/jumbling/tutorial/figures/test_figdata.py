"""Small failure-path and provenance tests; never rerun a reserved seed."""

import ast
import copy
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import types
import unittest
from unittest.mock import patch


HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("tutorial_figdata", HERE / "figdata.py")
G = importlib.util.module_from_spec(spec)
spec.loader.exec_module(G)


class FigdataTests(unittest.TestCase):
    def test_digest_mismatch_refuses_state_including_wrong_recorded_prefix(self):
        state = types.SimpleNamespace(digest=lambda: "a" * 64)
        self.assertEqual(G.check_digest(state, "a" * 64), "a" * 64)
        self.assertEqual(G.check_digest(state, "a" * 8), "a" * 64)
        for expected in ("b" * 64, "b" * 8, "", None):
            with self.subTest(expected=expected), self.assertRaisesRegex(ValueError, "digest"):
                G.check_digest(state, expected)

    def test_rebuild_checks_digest_before_returning_the_height_start(self):
        computation = G.Computation()
        state = types.SimpleNamespace(digest=lambda: "a" * 64, off_lattice_count=lambda: 0)
        record = {"seed": 7302, "history": [], "moves": [], "off_now": 0}
        with patch.object(computation, "state", return_value=state), \
             patch.object(computation.inputs, "json", return_value=record), \
             patch.object(computation.inputs, "lines", return_value=[("START", {"digest": "b" * 64})]):
            with self.assertRaisesRegex(ValueError, "state digest mismatch"):
                computation.height_start
        self.assertNotIn("height_start", computation.__dict__)

    def test_check_cli_detects_changed_value_without_writing(self):
        name = G.NAMES[0]
        fresh = {"format": G.FORMAT, "figures": {name: {"states": [{"state_digest": "a" * 64, "N_c": {"268": 42}}]}}}
        with tempfile.TemporaryDirectory(prefix="magic600-figdata-test-") as directory:
            path = Path(directory) / "figdata.json"
            path.write_text(json.dumps(fresh))
            with patch.object(G, "HERE", Path(directory)), patch.object(G, "compute", return_value=fresh), \
                 contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(G.main(["--check", "--only", name]), 0)
                changed = copy.deepcopy(fresh)
                changed["figures"][name]["states"][0]["N_c"]["268"] += 1
                path.write_text(json.dumps(changed))
                before = path.read_bytes()
                self.assertEqual(G.main(["--check", "--only", name]), 1)
                self.assertIn(name, output.getvalue())
                self.assertEqual(path.read_bytes(), before)

    def test_reserved_seeds_are_rejected_before_any_read(self):
        inputs = G.Inputs()
        with patch.object(Path, "read_bytes", side_effect=AssertionError("input was opened")) as read:
            for seed in range(9011, 9074):
                for name in (f"ia-k40-s{seed}.json", f"depth20-seed{seed}/run.json"):
                    with self.subTest(seed=seed, name=name), self.assertRaisesRegex(ValueError, "reserved seed"):
                        inputs.raw(G.CENSUS + name)
            read.assert_not_called()
        for name in G.NAMES:
            for path in G.evidence_paths(name):
                G.input_path(path)

    def test_failed_recomputation_preserves_saved_data(self):
        with tempfile.TemporaryDirectory(prefix="magic600-figdata-test-") as directory:
            path = Path(directory) / "figdata.json"
            path.write_bytes(b"previous data\n")
            with patch.object(G, "HERE", Path(directory)), \
                 patch.object(G, "compute", side_effect=ValueError("state digest mismatch")), \
                 contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(G.main([]), 1)
            self.assertEqual(path.read_bytes(), b"previous data\n")

    def test_input_paths_reject_escape_and_non_normalized_forms(self):
        for path in ("/tmp/state", "../state", "a/../b", "a//b", "C:\\state", "a/./b"):
            with self.subTest(path=path), self.assertRaises(ValueError):
                G.input_path(path)

    def test_loading_functions_does_not_run_a_script_cli(self):
        inputs = types.SimpleNamespace(text=lambda _: "raise RuntimeError('CLI ran')\ndef value(x):\n return x + 1\n")
        namespace = G.functions(inputs, "example.py", ["value"], {})
        self.assertEqual(namespace["value"](2), 3)
        with self.assertRaisesRegex(ValueError, "source functions mismatch"):
            G.functions(inputs, "example.py", ["missing"], {})

    def test_only_limits_computation_and_has_selection_independent_inputs(self):
        selected = G.NAMES[2]
        with patch.object(G.Computation, "prediction", return_value={"L0": 3, "F_h": {6: 0}}) as prediction, \
             patch.object(G.Computation, "height_counts", side_effect=AssertionError("unselected work")), \
             patch.object(G.Inputs, "raw", return_value=b"committed input"), \
             contextlib.redirect_stdout(io.StringIO()):
            fresh = G.compute([selected])
        prediction.assert_called_once()
        self.assertEqual(set(fresh["figures"]), {selected})
        self.assertEqual(fresh["figures"][selected]["F_h"], {"6": 0})
        self.assertEqual(set(fresh["figures"][selected]["inputs"]), set(G.evidence_paths(selected)))
        saved = copy.deepcopy(fresh)
        saved["figures"][G.NAMES[0]] = {"old": True}
        self.assertEqual(G.check(saved, fresh, [selected]), [])
        self.assertEqual(G.check(saved, fresh), [G.NAMES[0]])

    def test_unknown_selection_does_no_work(self):
        with patch.object(G.Computation, "bulk", side_effect=AssertionError("unexpected replay")):
            with self.assertRaisesRegex(ValueError, "unknown figure"):
                G.compute(["unknown"])

    def test_saved_observations_have_digests_and_resolvable_source_functions(self):
        document = json.loads((HERE / "figdata.json").read_bytes())
        self.assertEqual(document["format"], G.FORMAT)
        self.assertEqual(set(document["figures"]), set(G.NAMES))
        checked = set()

        def visit(value):
            if isinstance(value, dict):
                if "state_digest" in value:
                    self.assertRegex(value["state_digest"], r"^[0-9a-f]{64}$")
                    self.assertIn("source", value)
                for key, item in value.items():
                    if key in ("source", "N_source", "position_source") and item not in checked:
                        path, function = item.split(":")
                        body = ast.parse(G.input_path(path).read_text()).body
                        for part in function.split("."):
                            node = next((n for n in body if isinstance(n, (ast.FunctionDef, ast.ClassDef))
                                         and n.name == part), None)
                            self.assertIsNotNone(node, item)
                            body = node.body
                        checked.add(item)
                    visit(item)
            elif isinstance(value, list):
                for item in value:
                    visit(item)

        visit(document["figures"])
        self.assertGreaterEqual(len(checked), 7)


if __name__ == "__main__":
    unittest.main(verbosity=2)
