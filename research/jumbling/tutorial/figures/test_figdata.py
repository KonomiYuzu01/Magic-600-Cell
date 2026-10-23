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

    def test_point_model_refuses_wrong_rebuilt_digest_before_loading_the_model(self):
        computation = G.Computation()
        state = types.SimpleNamespace(digest=lambda: "a" * 64)
        record = {"seed": 7102, "history": [], "moves": []}
        with patch.object(computation, "state", return_value=state), \
             patch.object(computation, "play", return_value=None), \
             patch.object(computation.inputs, "json", return_value=record), \
             patch.object(computation.inputs, "text", return_value=json.dumps({"start_digest": "b" * 64})), \
             patch.object(G, "functions", side_effect=AssertionError("loaded a model before checking the state")):
            with self.assertRaisesRegex(ValueError, "state digest mismatch"):
                computation.point_model()
        self.assertNotIn("point_model_start", computation.__dict__)

    def test_class_cells_refuse_wrong_rebuilt_digest_before_loading_the_model(self):
        computation = G.Computation()
        record = {"input": {"table": G.CLASS_TABLE, "sha256": "c" * 64, "digest": "b" * 64}}
        state = types.SimpleNamespace(digest=lambda: "a" * 64)
        computation.inputs.digests[G.CLASS_TABLE] = record["input"]["sha256"]
        computation.__dict__["mechanics"] = types.SimpleNamespace(
            orient=types.SimpleNamespace(set_pose_table=lambda *args: None))
        with patch.object(computation, "state", return_value=state), \
             patch.object(computation.inputs, "json", side_effect=[record, {"digest": "b" * 64}]), \
             patch.object(G, "functions", side_effect=AssertionError("loaded a model before checking the state")):
            with self.assertRaisesRegex(ValueError, "state digest mismatch"):
                computation.class_cells()
        self.assertNotIn("class_cells_start", computation.__dict__)

    def test_class_cells_refuse_changed_input_bytes_before_importing_the_table(self):
        computation = G.Computation()
        record = {"input": {"table": G.CLASS_TABLE, "sha256": "c" * 64, "digest": "b" * 64}}
        computation.inputs.digests[G.CLASS_TABLE] = "d" * 64
        with patch.object(computation.inputs, "json", side_effect=[record, {"digest": "b" * 64}]), \
             patch.object(computation, "state", side_effect=AssertionError("used an unverified table")):
            with self.assertRaisesRegex(ValueError, "class-cell table digest mismatch"):
                computation.class_cells()

    def test_each_concept_check_detects_a_changed_observation_without_writing(self):
        saved = json.loads((HERE / "figdata.json").read_bytes())
        for name in G.CONCEPTS:
            fresh = {"format": G.FORMAT, "figures": {name: saved["figures"][name]}}
            changed = copy.deepcopy(fresh)
            if name == "ch04-point-model":
                changed["figures"][name]["trace"][0]["label"] += 1
            else:
                changed["figures"][name]["cells"][0]["points"][0]["owner"]["pieces"] += 1
            with self.subTest(name=name), tempfile.TemporaryDirectory(prefix="magic600-concept-check-") as directory:
                path = Path(directory) / "figdata.json"
                path.write_text(json.dumps(fresh))
                with patch.object(G, "HERE", Path(directory)), patch.object(G, "compute", return_value=fresh), \
                     contextlib.redirect_stdout(io.StringIO()) as output:
                    self.assertEqual(G.main(["--check", "--only", name]), 0)
                    path.write_text(json.dumps(changed))
                    before = path.read_bytes()
                    self.assertEqual(G.main(["--check", "--only", name]), 1)
                    self.assertEqual(path.read_bytes(), before)
                self.assertIn(name, output.getvalue())

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
        inputs = types.SimpleNamespace(text=lambda _: "raise RuntimeError('CLI ran')\nclass Example:\n value = 7\n")
        namespace = G.functions(inputs, "example.py", ["Example"], {})
        self.assertEqual(namespace["Example"]().value, 7)

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
                    if key in ("source", "N_source", "position_source", "bad_source") and item not in checked:
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

    def test_each_appendix_data_set_check_detects_a_changed_value_without_writes(self):
        for name in (n for n in G.NAMES if n.startswith("appa-")):
            fresh = {"format": G.FORMAT, "figures": {name: {"letters": 2, "before_digest": "a" * 64}}}
            if name == "appa-late-cycle-tree":
                fresh["word"] = [[12, 1], [12, 2]]
            with self.subTest(name=name), tempfile.TemporaryDirectory(prefix="magic600-appa-check-") as directory:
                path = Path(directory) / "figdata.json"
                changed = copy.deepcopy(fresh)
                changed["figures"][name]["letters"] = 3
                path.write_text(json.dumps(changed))
                before = path.read_bytes()
                with patch.object(G, "HERE", Path(directory)), patch.object(G, "compute", return_value=fresh), \
                     contextlib.redirect_stdout(io.StringIO()) as output:
                    self.assertEqual(G.main(["--check", "--only", name]), 1)
                self.assertIn(name, output.getvalue())
                self.assertEqual(path.read_bytes(), before)
                if "word" in fresh:
                    changed = copy.deepcopy(fresh)
                    changed["word"][0][1] = 3
                    self.assertEqual(G.check(changed, fresh, [name]), [name])

    def test_u2_disagreeing_committed_digest_is_refused(self):
        computation = G.Computation()
        original = computation.inputs.json
        tape = original(G.U2)
        tape["digest"] = "0" * 64
        with patch.object(computation.inputs, "json", return_value=tape):
            with self.assertRaisesRegex(ValueError, "U2 input digest mismatch"):
                computation.appendix_u2()

    def test_h4_pair_disagreeing_cloud_boundary_is_refused(self):
        computation = G.Computation()
        original = computation.inputs.lines

        def changed(path):
            rows = original(path)
            if path.endswith("end_summary.txt"):
                row = next(row for tag, row in rows if tag == "LETTER" and row["n"] == 20)
                row["digest"] = "0" * 64
            return rows

        with patch.object(computation.inputs, "lines", side_effect=changed):
            with self.assertRaisesRegex(ValueError, "h4 pair input digest mismatch"):
                computation.appendix_h4_pair()

    def test_pure_wrong_pose_state_is_refused_before_playing(self):
        computation = G.Computation()
        state = types.SimpleNamespace(digest=lambda: "0" * 64)
        computation.__dict__["mechanics"] = types.SimpleNamespace(
            orient=types.SimpleNamespace(set_pose_table=lambda *args: None))
        with patch.object(computation, "state", return_value=state), \
             patch.object(computation, "play", side_effect=AssertionError("played before checking the input")):
            with self.assertRaisesRegex(ValueError, "state digest mismatch"):
                computation.appendix_pure()

    def test_correction_target_and_library_digest_mismatches_are_refused(self):
        for field in ("target", "library"):
            computation = G.Computation()
            original = computation.inputs.json

            def changed(path):
                value = original(path)
                if path.endswith("restore-summary.json"):
                    if field == "target":
                        value["runs"][0]["T_digest"] = "0" * 64
                    else:
                        value["library"]["sha256"] = "0" * 64
                return value

            with self.subTest(field=field), patch.object(computation.inputs, "json", side_effect=changed):
                with self.assertRaisesRegex(ValueError, "digest mismatch"):
                    computation.appendix_correction()

    def test_late_program_word_digest_is_checked_before_replay(self):
        computation = G.Computation()
        computation.__dict__["mechanics"] = types.SimpleNamespace(E=types.SimpleNamespace(sim=None), C=None, M=None)
        chapter = {"E_late": {"steps": [{"orbit": 6, "factors": 2}], "orbit6": {"word_sha256": "0" * 64}}}
        doc = {"nodes": [], "solutions": {"recorded": [[0, 1], [1, 1]]}}
        module = types.SimpleNamespace(Program=lambda *args: types.SimpleNamespace(expand=lambda *args: [1]),
                                       ObserverLetters=lambda *args: types.SimpleNamespace(primitives={1: 1}))
        values = {G.T + "/records/ch06-examples.json": chapter, G.LATE: doc, "assets/execution_trees.json.gz": []}
        with patch.dict("sys.modules", {"literal_lattice": module}), \
             patch.object(computation.inputs, "json", side_effect=lambda path: values[path]), \
             patch.object(computation, "state", side_effect=AssertionError("replayed before validating the word")):
            with self.assertRaisesRegex(ValueError, "late orbit word digest mismatch"):
                computation.appendix_late_cycle()

    def test_saved_appendix_words_and_runner_trees_have_matching_lengths(self):
        document = json.loads((HERE / "figdata.json").read_bytes())
        api = G.load_diagrams().tree_api()
        for name in (n for n in G.NAMES if n.startswith("appa-") and n != "appa-inventory"):
            data = document["figures"][name]
            with self.subTest(name=name):
                word = data.get("word")
                if word is None:
                    word = G.Inputs().json(data["word_file"])["word"]
                api.check_tree(data["tree"], len(word))
        self.assertEqual(document["word"], document["figures"]["appa-late-cycle-tree"]["word"])
        self.assertEqual(len(document["word"]), 810)
        late = document["figures"]["appa-late-cycle-tree"]
        self.assertEqual(late["misplaced_positions"], [6, 8, 6])
        self.assertEqual(late["off_before"], late["off_after"])

    def test_inventory_refuses_an_obsolete_missing_example_claim(self):
        computation = G.Computation()
        with patch.object(G, "inventory_paths", return_value=["new-development-record.json"]), \
             patch.object(computation.inputs, "json", return_value={"steps": [{"rule": "lock"}]}):
            with self.assertRaisesRegex(ValueError, "missing-example claim no longer holds"):
                computation.appendix_inventory()


if __name__ == "__main__":
    unittest.main(verbosity=2)
