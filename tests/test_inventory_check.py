"""Tests of the stage 2.1 inventory checker (docs/progress/1.0/inventory/check_inventory.py).

Synthetic inventories only; evidence points at small, stable repository files."""
from __future__ import annotations

import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("check_inventory", ROOT / "docs/progress/1.0/inventory/check_inventory.py")
ci = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci)

ROW = {"id": "S1-1", "name": "Undo", "behaviour": "Reverts the last committed turn.", "entry_points": ["POST /api/undo"],
       "evidence": ["server.py:1"], "depends_on": [], "evidence_kind": "source", "reads": ["journal"],
       "writes": ["state", "journal"], "engine_call": "Session.undo"}
GOOD = {"shard": "s1", "functions": [ROW], "internal_only": [{"name": "helper", "evidence": ["core.py:1"]}],
        "unclear": [{"name": "maybe", "evidence": ["core.py:2"], "reason": "no caller found"}]}


class InventoryCheckTests(unittest.TestCase):
    def errors(self, doc):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "inv.json"
            path.write_text(json.dumps(doc), encoding="utf-8")
            return ci.check(path)

    def mutated(self, fn):
        doc = copy.deepcopy(GOOD)
        fn(doc)
        return self.errors(doc)

    def test_good_inventory_passes(self):
        self.assertEqual(self.errors(GOOD), [])

    def test_auxiliary_items_get_full_evidence_and_field_checks(self):
        cases = {
            "missing file": lambda d: d["internal_only"].append({"name": "x", "evidence": ["missing-source.py:9"]}),
            "line out of range": lambda d: d["unclear"].append({"name": "x", "evidence": ["core.py:999999"]}),
            "malformed": lambda d: d["unclear"].append({"name": "x", "evidence": ["core.py"]}),
            "empty evidence": lambda d: d["internal_only"].append({"name": "x", "evidence": []}),
            "disposition field": lambda d: d["internal_only"].append({"name": "x", "evidence": ["core.py:1"],
                                                                     "disposition": "DELETE"}),
            "empty name": lambda d: d["unclear"].append({"name": " ", "evidence": ["core.py:1"]}),
        }
        for name, fn in cases.items():
            with self.subTest(name):
                self.assertTrue(self.mutated(fn))

    def test_rows_reject_unknown_fields_bad_paths_and_ids(self):
        cases = {
            "disposition": lambda d: d["functions"][0].update(disposition="keep"),
            "absolute path": lambda d: d["functions"][0].update(evidence=["/etc/hostname:1"]),
            "parent path": lambda d: d["functions"][0].update(evidence=["docs/../server.py:1"]),
            "line zero": lambda d: d["functions"][0].update(evidence=["server.py:0"]),
            "label": lambda d: d["functions"][0].update(evidence=["server.py:1 (route)"]),
            "bad kind": lambda d: d["functions"][0].update(evidence_kind="test"),
            "bad data": lambda d: d["functions"][0].update(reads=["database"]),
            "no entry point": lambda d: d["functions"][0].update(entry_points=[]),
            "id of another shard": lambda d: d["functions"][0].update(id="S2-1"),
            "duplicate id": lambda d: d["functions"].append(dict(ROW)),
            "extra top-level key": lambda d: d.update(dispositions={}),
            "bad shard": lambda d: d.update(shard="all"),
            "empty notes": lambda d: d["functions"][0].update(notes=""),
        }
        for name, fn in cases.items():
            with self.subTest(name):
                self.assertTrue(self.mutated(fn))

    def test_evidence_kind_needs_its_notes(self):
        self.assertTrue(self.mutated(lambda d: d["functions"][0].update(evidence_kind="inferred")))
        self.assertTrue(self.mutated(lambda d: d["functions"][0].update(evidence_kind="source+test")))
        self.assertTrue(self.mutated(lambda d: d["functions"][0].update(evidence_kind="source+test",
                                                                        notes="Covered by tests/test_missing_x.py")))
        self.assertEqual(self.mutated(lambda d: d["functions"][0].update(evidence_kind="source+test",
                                                                         notes="Covered by tests/test_core.py.")), [])
        self.assertEqual(self.mutated(lambda d: d["functions"][0].update(evidence_kind="inferred",
                                                                         notes="Reached only through the host.")), [])

    def test_any_existing_test_file_counts_for_source_and_test(self):
        for notes in ("Covered by 'tests/test_core.py'.", 'See "tests/test_core.py"',
                      "work/experiments/magic600-04/packaging/test_package_contract.py covers it"):
            with self.subTest(notes):
                self.assertEqual(self.mutated(lambda d: d["functions"][0].update(evidence_kind="source+test",
                                                                                 notes=notes)), [])
        self.assertTrue(self.mutated(lambda d: d["functions"][0].update(evidence_kind="source+test",
                                                                        notes="Exercised by core.py itself")))

    def test_wrong_types_are_reported_not_raised(self):
        for field, value in (("id", []), ("evidence_kind", {}), ("reads", [{}]), ("writes", [[]]),
                             ("entry_points", [1]), ("evidence", [3]), ("engine_call", 5), ("name", None)):
            with self.subTest(field):
                self.assertTrue(self.mutated(lambda d: d["functions"][0].update({field: value})))

    def test_unreadable_and_non_object_files_fail_cleanly(self):
        with tempfile.TemporaryDirectory() as tmp:
            bad = Path(tmp) / "bad.json"
            bad.write_text("{", encoding="utf-8")
            self.assertTrue(ci.check(bad))
            bad.write_text("[]", encoding="utf-8")
            self.assertTrue(ci.check(bad))
            self.assertTrue(ci.check(Path(tmp) / "missing.json"))

    def test_committed_coverage_map_is_complete(self):
        self.assertEqual(ci.coverage(), [])

    def test_coverage_reports_overlap_gaps_and_unknown_files(self):
        base = json.loads(ci.SHARDS.read_text(encoding="utf-8"))
        cases = {
            "overlap": lambda d: d["shards"]["s2"]["files"].append(d["shards"]["s1"]["files"][0]),
            "gap": lambda d: d["shards"]["s1"]["files"].pop(),
            "unknown": lambda d: d["shards"]["s1"]["files"].append("no_such_file.py"),
            "excluded and owned": lambda d: d["excluded"].update({d["shards"]["s1"]["files"][0]: "x"}),
            "no reason": lambda d: d["excluded"].update({next(iter(d["excluded"])): ""}),
        }
        for name, fn in cases.items():
            with self.subTest(name), tempfile.TemporaryDirectory() as tmp:
                doc = copy.deepcopy(base)
                fn(doc)
                path = Path(tmp) / "shards.json"
                path.write_text(json.dumps(doc), encoding="utf-8")
                with patch.object(ci, "SHARDS", path):
                    self.assertTrue(ci.coverage())


if __name__ == "__main__":
    unittest.main(verbosity=2)
