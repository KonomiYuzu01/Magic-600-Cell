"""Headless differential-oracle checks, using fresh temporary data only."""
from __future__ import annotations

import contextlib
import copy
import gzip
import hashlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import numpy as np

from core import Model, PuzzleState, canonical, invrecipe
from tools.oracle import compare, make_cases, replay_trace


class OracleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix="c600-oracle-tests-", dir=ROOT)
        cls.addClassCleanup(cls.temporary.cleanup)
        cls.directory = Path(cls.temporary.name)
        cls.model = Model()
        paths = make_cases.write_cases(cls.directory / "first", cls.model, small=True)
        cls.paths = {path.name[:3]: path for path in paths}
        cls.cases = {name: compare.read_document(path) for name, path in cls.paths.items()}
        cls.traces = {}
        primitive_model = Model()
        with mock.patch.object(primitive_model, "net", side_effect=AssertionError("Used net")), \
                mock.patch.object(primitive_model, "star_net", side_effect=AssertionError("Used star cache")), \
                mock.patch.object(primitive_model, "word_net", side_effect=AssertionError("Used word net")):
            for name, case in cls.cases.items():
                cls.traces[name] = replay_trace.replay_case(case, primitive_model)
        if primitive_model.star_cache or primitive_model.base:
            raise AssertionError("Primitive replay populated a star cache")

    def compare_cli(self, case, trace):
        path = self.directory / "candidate.json"
        path.write_text(canonical(trace), encoding="utf-8")
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            code = compare.main([str(self.paths[case]), str(path)])
        return code, json.loads(output.getvalue())

    def test_generator_deterministic_seeded_subsets(self):
        paths = make_cases.write_cases(self.directory / "second", self.model, small=True)
        self.assertEqual(set(self.cases), {"O%02d" % index for index in range(1, 8)})
        for path in paths:
            with self.subTest(case=path.name):
                data = path.read_bytes()
                self.assertEqual(data, self.paths[path.name[:3]].read_bytes())
                self.assertEqual(data[4:8], b"\0" * 4)
                self.assertFalse(data[3] & 8, "gzip must contain no filename")
                payload = gzip.decompress(data).decode("utf-8")
                case = json.loads(payload)
                self.assertEqual(payload, canonical(case))
                self.assertNotIn(str(ROOT), payload)
                compare.validate_case(case)
                generator = case["generator"]
                self.assertEqual(generator["counts"]["steps"], len(case["steps"]))
                self.assertEqual(generator["counts"]["initial_moves"], len(case["initial"]["moves"]))
                for field, source in (("script_sha256", "tools/oracle/make_cases.py"),
                                      ("core_sha256", "core.py"), ("session_sha256", "session.py")):
                    self.assertEqual(generator[field], hashlib.sha256((ROOT / source).read_bytes()).hexdigest())
                self.assertEqual(generator["numpy"], np.__version__)

    def test_full_suite_recipe_coverage(self):
        inputs = {row[0]: row for row in make_cases.case_inputs(self.model)}
        o01 = inputs["O01"][-1]
        self.assertEqual(len(o01), 2400)
        self.assertEqual([recipe[0]["moves"][0] for recipe in o01],
                         [sign * move for move in range(1, 1201) for sign in (1, -1)])
        self.assertEqual(inputs["O01"][2], "independent")
        self.assertEqual(len(inputs["O02"][-1]), 500)
        expected_stars = [make_cases.star(orbit, node, sign)
                          for orbit in range(35)
                          for node in (0, len(self.model.trees[orbit]["positions"]) // 2,
                                       len(self.model.trees[orbit]["positions"]) - 1)
                          for sign in (1, -1)]
        self.assertEqual(inputs["O03"][-1], [[star] for star in expected_stars])
        self.assertEqual(len(inputs["O03"][3]), make_cases.SCRAMBLE_MOVES)
        mixed = inputs["O04"][-1]
        self.assertEqual(len(mixed), make_cases.MIXED_RECIPES)
        self.assertEqual({len(recipe) for recipe in mixed}, set(range(2, 7)))
        self.assertTrue(all({part["kind"] for part in recipe} == {"word", "star"} for recipe in mixed))
        self.assertEqual(inputs["O05"][-1],
                         [item for recipe in mixed for item in (recipe, invrecipe(recipe))])
        self.assertEqual(len(inputs["O07"][-1]), 10)
        initial = self.cases["O05"]["initial"]["state_hash"]
        for step in self.cases["O05"]["steps"][1::2]:
            self.assertEqual(step["expect"]["state_hash"], initial)

    def test_replay_matches_every_generated_step_without_caches(self):
        for name, case in self.cases.items():
            with self.subTest(case=name):
                code, report = compare.compare_documents(case, self.traces[name])
                self.assertEqual(code, 0, report)
                self.assertEqual(report["status"], "equal")
                for expected, actual in zip(case["steps"], self.traces[name]["steps"]):
                    self.assertEqual(expected["index"], actual["index"])
                    self.assertEqual(expected["expect"],
                                     {field: actual[field] for field in compare.FIELDS})

    def test_comparator_first_mismatch_and_all_differences(self):
        for field in ("state_hash", "net_sha256", "accepted", "conflicts"):
            name = "O06" if field == "conflicts" else "O02"
            trace = copy.deepcopy(self.traces[name])
            if field == "conflicts":
                row = trace["steps"][1][field][0]
                trace["steps"][1][field][0] = dict(row, stickers=row["stickers"] + 1)
            elif field == "accepted":
                trace["steps"][1][field] = not trace["steps"][1][field]
            else:
                trace["steps"][1][field] = "0" * 64
            trace["steps"][-1]["state_hash"] = "1" * 64
            with self.subTest(field=field):
                code, report = self.compare_cli(name, trace)
                self.assertEqual(code, 1)
                self.assertEqual(report["case_id"], name)
                self.assertEqual(report["index"], 1)
                self.assertEqual(report["recipe"], self.cases[name]["steps"][1]["recipe"])
                self.assertEqual([diff["field"] for diff in report["differences"]], [field])

        trace = copy.deepcopy(self.traces["O02"])
        changed = ("accepted", "state_hash", "net_sha256", "net_moved_stickers",
                   "net_moved_pieces", "expansion_sha256", "primitive_count", "support",
                   "conflicts", "progress_sha256")
        step = trace["steps"][0]
        step["accepted"] = False
        for field in ("state_hash", "net_sha256", "expansion_sha256", "progress_sha256"):
            step[field] = "0" * 64
        for field in ("net_moved_stickers", "net_moved_pieces", "primitive_count"):
            step[field] += 1
        step["support"][0]["pieces"] += 1
        step["conflicts"] = [dict(step["support"][0])]
        code, report = self.compare_cli("O02", trace)
        self.assertEqual(code, 1)
        self.assertEqual([diff["field"] for diff in report["differences"]], list(changed))

    def test_comparator_identity_and_missing_steps_exit_two(self):
        for field in ("model_id", "manifest_sha256"):
            for missing in (False, True):
                trace = copy.deepcopy(self.traces["O02"])
                if missing:
                    del trace["model"][field]
                else:
                    trace["model"][field] = "different-model" if field == "model_id" else "0" * 64
                with self.subTest(field=field, missing=missing):
                    code, report = self.compare_cli("O02", trace)
                    self.assertEqual(code, 2)
                    self.assertIn(field, report["error"])
        for index in (0, 3, -1):
            trace = copy.deepcopy(self.traces["O02"])
            trace["steps"].pop(index)
            trace["steps"][0]["state_hash"] = "0" * 64
            with self.subTest(missing_step=index):
                code, report = self.compare_cli("O02", trace)
                self.assertEqual(code, 2)
                self.assertEqual(report["error"], "Missing step")

    def test_comparator_rows_order_omissions_and_malformed_files(self):
        case = copy.deepcopy(self.cases["O06"])
        trace = copy.deepcopy(self.traces["O06"])
        trace["steps"].reverse()
        for step in trace["steps"]:
            for field in ("support", "conflicts"):
                step[field].reverse()
                if step[field]:
                    step[field].append(dict(step[field][0]))
        self.assertEqual(compare.compare_documents(case, trace)[0], 0)
        del trace["steps"][0]["progress_sha256"]
        code, report = compare.compare_documents(case, trace)
        self.assertEqual(code, 1)
        self.assertEqual(report["differences"][0]["actual"], {"missing": True})
        case["optional_fields"] = {trace["candidate"]["name"]: ["progress_sha256"]}
        code, report = compare.compare_documents(case, trace)
        self.assertEqual(code, 0)
        self.assertEqual(report["status"], "match-with-omissions")
        self.assertEqual(report["omitted_fields"], [{"index": 5, "field": "progress_sha256"}])
        trace["steps"][0]["progress_sha256"] = "0" * 64
        self.assertEqual(compare.compare_documents(case, trace)[0], 1)

        for mutation in ("format", "case_id", "duplicate-index", "extra-step", "boolean-index", "bad-field"):
            bad = copy.deepcopy(self.traces["O02"])
            if mutation in ("format", "case_id"):
                bad[mutation] = "wrong"
            elif mutation == "duplicate-index":
                bad["steps"].append(dict(bad["steps"][0]))
            elif mutation == "extra-step":
                bad["steps"].append(dict(bad["steps"][0], index=999))
            elif mutation == "boolean-index":
                bad["steps"][0]["index"] = False
            else:
                bad["steps"][0]["primitive_count"] = True
            with self.subTest(malformed=mutation):
                self.assertEqual(self.compare_cli("O02", bad)[0], 2)
        corrupt_gzip = bytearray(gzip.compress(b"{}", mtime=0))
        corrupt_gzip[10:12] = b"\xff\xff"
        for data in (b"not JSON", b"[]", b'{"format":1,"format":2}', b'{"x":NaN}',
                     b"\x1f\x8btruncated", bytes(corrupt_gzip)):
            path = self.directory / "malformed.json"
            path.write_bytes(data)
            with self.subTest(data=data), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(compare.main([str(self.paths["O02"]), str(path)]), 2)
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(compare.main([str(self.paths["O02"]),
                                           str(self.directory / "absent.json")]), 2)

    def test_expansion_digest_vector(self):
        recipe = make_cases.word([1, -1])
        expected = "b15348c8f462384c01e83b6d499c6faf3f96808f5aa07c6bab4b65b36b4445d4"
        self.assertEqual(make_cases.expansion_digest(self.model, recipe), (expected, 2))
        _, replayed = replay_trace.replay_step(self.model, PuzzleState(self.model), recipe, set())
        self.assertEqual(replayed["expansion_sha256"], expected)
        self.assertEqual(replayed["primitive_count"], 2)
        self.assertEqual(replayed["net_sha256"], hashlib.sha256(b"").hexdigest())

    def test_streaming_candidate_fails_invalid_suffixes(self):
        case = self.cases["O07"]
        labels = self.model.ids.copy()
        for move in case["initial"]["moves"]:
            source, destination = self.model.move(move)
            labels[destination] = labels[source]
        initial = PuzzleState(self.model, labels, trusted=True)
        for step in case["steps"]:
            self.assertFalse(step["expect"]["accepted"])
            self.assertEqual(step["expect"]["error"], "invalid-input")
            self.assertEqual(step["expect"]["state_hash"], initial.hash)
            self.assertEqual(step["expect"]["progress_sha256"], make_cases.progress_digest(initial))
            self.assertTrue(all(step["expect"][field] is None for field in compare.EFFECT_FIELDS))

        for index in (8, 9):
            state = PuzzleState(self.model, labels, trusted=True)
            with self.assertRaises(ValueError):
                # Deliberate bug: validate each move/part only after applying its prefix.
                for part in case["steps"][index]["recipe"]:
                    moves = part["moves"] if part["kind"] == "word" else self.model.expand([part])
                    for move in moves:
                        source, destination = self.model.move(move)
                        state.apply(source, destination)
            trace = copy.deepcopy(self.traces["O07"])
            trace["candidate"]["name"] = "deliberately-streaming"
            trace["steps"][index]["state_hash"] = state.hash
            trace["steps"][index]["progress_sha256"] = make_cases.progress_digest(state)
            with self.subTest(invalid_suffix=index):
                self.assertNotEqual(state.hash, initial.hash)
                code, report = self.compare_cli("O07", trace)
                self.assertEqual(code, 1)
                self.assertEqual(report["index"], index)
                self.assertIn("state_hash", [diff["field"] for diff in report["differences"]])

    def test_protected_refusal_and_net_cancellation(self):
        case = self.cases["O06"]
        previous = case["initial"]["state_hash"]
        previous_progress = make_cases.progress_digest(PuzzleState(self.model))
        accepted = refused = False
        for step in case["steps"]:
            expected = step["expect"]
            if not expected["accepted"]:
                refused = True
                self.assertEqual(expected["error"], "protected")
                self.assertEqual(expected["state_hash"], previous)
                self.assertEqual(expected["progress_sha256"], previous_progress)
                self.assertTrue(expected["conflicts"])
            else:
                accepted = True
                self.assertEqual(expected["conflicts"], [])
            previous, previous_progress = expected["state_hash"], expected["progress_sha256"]
        self.assertTrue(accepted and refused)
        collateral = case["steps"][1]
        self.assertEqual(len(case["protected"]), 1)
        self.assertNotIn(collateral["recipe"][0]["orbit"], case["protected"])
        self.assertEqual([row["orbit"] for row in collateral["expect"]["conflicts"]], case["protected"])
        cancellation = case["steps"][3]
        move, inverse = cancellation["recipe"][0]["moves"]
        self.assertEqual(inverse, -move)
        source, _ = self.model.move(move)
        self.assertTrue(any(row["orbit"] in case["protected"] for row in self.model.support(source)))
        self.assertTrue(cancellation["expect"]["accepted"])
        self.assertEqual(cancellation["expect"]["net_moved_stickers"], 0)
        self.assertEqual(cancellation["expect"]["primitive_count"], 2)

    def test_generator_refuses_session_output(self):
        out = self.directory / "session-output"
        out.mkdir()
        database = out / "session.sqlite3"
        database.write_bytes(b"untouched sentinel")
        with mock.patch.object(make_cases, "Model", side_effect=AssertionError("Opened reference")):
            with self.assertRaisesRegex(ValueError, "session.sqlite3"):
                make_cases.write_cases(out, small=True)
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(make_cases.main(["--out", str(out)]), 2)
        self.assertEqual(list(out.iterdir()), [database])
        self.assertEqual(database.read_bytes(), b"untouched sentinel")
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as raised:
            make_cases.main(["--out", str(out), "--data-directory", str(out)])
        self.assertEqual(raised.exception.code, 2)

    def test_generator_refuses_linked_or_non_empty_output(self):
        sentinel = self.directory / "sentinel.bin"
        sentinel.write_bytes(b"untouched sentinel")
        for kind in ("symlink", "hardlink"):
            with self.subTest(kind=kind):
                out = self.directory / ("linked-" + kind)
                out.mkdir()
                entry = out / "O01.json.gz"
                if kind == "symlink":
                    entry.symlink_to(sentinel)
                else:
                    os.link(sentinel, entry)
                with mock.patch.object(make_cases, "Model", side_effect=AssertionError("Opened reference")):
                    with self.assertRaisesRegex(ValueError, "non-empty"):
                        make_cases.write_cases(out, small=True)
                self.assertEqual(sentinel.read_bytes(), b"untouched sentinel")
        linked = self.directory / "linked-directory"
        linked.symlink_to(self.directory / "linked-symlink", target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "linked output directory"):
            make_cases.check_output_directory(linked)

    def test_protected_independent_case_is_refused(self):
        with self.assertRaisesRegex(ValueError, "chain mode"):
            make_cases.make_case(self.model, "O06", "probe", "independent", [], [0], [[]])

    def test_command_line_replay_and_compare(self):
        environment = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
        replay = subprocess.run([sys.executable, "tools/oracle/replay_trace.py", str(self.paths["O02"])],
                                cwd=ROOT, env=environment, capture_output=True, text=True, timeout=30)
        self.assertEqual(replay.returncode, 0, replay.stderr)
        self.assertEqual(json.loads(replay.stdout), self.traces["O02"])
        path = self.directory / "command-line-trace.json"
        path.write_text(replay.stdout, encoding="utf-8")
        result = subprocess.run([sys.executable, "tools/oracle/compare.py", str(self.paths["O02"]), str(path)],
                                cwd=ROOT, env=environment, capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["status"], "equal")


if __name__ == "__main__":
    unittest.main(verbosity=2)
