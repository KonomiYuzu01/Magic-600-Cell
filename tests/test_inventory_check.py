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


class GeneratedFilesTests(unittest.TestCase):
    """census.json, screening-map.json and merged.json/md are current and reproducible."""

    def load(self, name):
        spec_ = importlib.util.spec_from_file_location(name, ROOT / "docs/progress/1.0/inventory" / f"{name}.py")
        mod = importlib.util.module_from_spec(spec_)
        spec_.loader.exec_module(mod)
        return mod

    def test_census_screening_map_and_merge_are_current(self):
        for name in ("census", "screening_map", "merge_inventories"):
            with self.subTest(name):
                self.assertEqual(self.load(name).main(["--check"]), 0)

    def test_sealed_files_pass_the_format_check(self):
        folder = ROOT / "docs/progress/1.0/inventory"
        for f in sorted(folder.glob("*-s[1-6].json")):
            with self.subTest(f.name):
                self.assertEqual(ci.check(f), [])

    def test_merge_refuses_a_changed_sealed_file(self):
        merge = self.load("merge_inventories")
        seals = json.loads((merge.HERE / "sealed.json").read_text(encoding="utf-8"))
        seals["files"]["bottom-up-s1.json"]["sha256"] = "0" * 64
        with patch.object(merge, "load", side_effect=lambda n: seals if n == "sealed.json" else json.loads(
                (merge.HERE / n).read_text(encoding="utf-8"))):
            self.assertTrue(merge.sealed_errors())

    def test_correspondence_ids_are_unique_and_within_one_shard(self):
        corr = json.loads((ROOT / "docs/progress/1.0/inventory/correspondence.json").read_text(encoding="utf-8"))
        ids = [i for p in corr["pairs"] for i in p["codex"] + p["claude"]]
        self.assertEqual(len(ids), len(set(ids)))
        for p in corr["pairs"]:
            self.assertEqual(len({i.split(":")[1].split("-")[0] for i in p["codex"] + p["claude"]}), 1)
            self.assertTrue(p["reason"].strip())

    def test_symbol_ranges_follow_real_definitions(self):
        sm = self.load("screening_map")
        py = ["class A:", "    def f(self):", "        def g():", "            return 1", "        x = g()",
              "        return x", "", "    def h(self):", "        pass"]
        self.assertEqual(sm.enclosing(py, "a.py", 5)[0], "A.f")
        self.assertEqual(sm.enclosing(py, "a.py", 4)[0], "A.f.g")
        self.assertEqual(sm.locate(py, "a.py", "A.f.g"), (3, 4))
        cs = ["class Shell{", " void One(){if(x){Run(\"}\");}}", "  int Two(){", "   return 2;", "  }",
              " Shell(){Init(a,", "  b);}", "}"]
        self.assertEqual(sm.enclosing(cs, "a.cs", 2), ("Shell.One", 2, 2))
        self.assertEqual(sm.enclosing(cs, "a.cs", 4), ("Shell.Two", 3, 5))
        self.assertEqual(sm.enclosing(cs, "a.cs", 7)[0], "Shell.Shell")

    def test_reviewed_screening_attachments_cover_every_finding(self):
        merge = self.load("merge_inventories")
        doc = json.loads(merge.OUT_JSON.read_text(encoding="utf-8"))
        self.assertEqual({s["id"] for s in doc["screening"]},
                         {f["id"] for f in json.loads((merge.HERE / "screening-map.json").read_text(encoding="utf-8"))["findings"]})
        for s in doc["screening"]:
            self.assertTrue(s["reason"].strip(), s["id"])
        by_id = {s["id"]: set(s["rows"]) for s in doc["screening"]}
        self.assertLessEqual({"codex:S6-19", "codex:S6-22", "codex:S6-23", "codex:S6-24"}, by_id["F-01"])
        self.assertFalse({"codex:S6-44", "codex:S6-46"} & by_id["F-01"])
        self.assertFalse({"codex:S3-32", "claude:S3-282"} & by_id["C-01"])

    def test_behaviour_only_change_shows_in_the_comparison(self):
        merge = self.load("merge_inventories")
        rows = merge.rows()
        rid = next(i for p in json.loads((merge.HERE / "correspondence.json").read_text(encoding="utf-8"))["pairs"]
                   for i in p["codex"])
        before = merge.build()
        rows[rid] = dict(rows[rid], behaviour="A completely different behaviour.")
        with patch.object(merge, "rows", return_value=rows):
            after = merge.build()
        self.assertNotEqual(before["matched"], after["matched"])

    def test_puzzle_command_is_not_reported_side_only(self):
        doc = json.loads((ROOT / "docs/progress/1.0/inventory/merged.json").read_text(encoding="utf-8"))
        group = next(m for m in doc["matched"] if "codex:S6-83" in m["codex"])
        self.assertIn("claude:S6-41", group["claude"])
        diff = group["differences"].get("entry_points", {})
        self.assertNotIn("command puzzle", diff.get("codex_only", []) + diff.get("claude_only", []))

if __name__ == "__main__":
    unittest.main(verbosity=2)
