"""Tests for the stage 2.2 disposition tools (docs/progress/1.0/dispositions/)."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIR = ROOT / "docs" / "progress" / "1.0" / "dispositions"
sys.path.insert(0, str(DIR))
import build_report  # noqa: E402
import build_units  # noqa: E402
import check_dispositions as cd  # noqa: E402

UNITS = {
    "U1-001": {"id": "U1-001", "shard": "s1", "names": ["Undo"], "rows": ["codex:S1-1"],
               "screening": ["X-02 (open)"], "screening_rows": {"X-02": ["codex:S1-1"]}},
    "U1-002": {"id": "U1-002", "shard": "s1", "names": ["Session tools"], "rows": ["codex:S1-2", "codex:S1-3", "claude:S1-2"],
               "screening": ["B-005 (needs verification)"], "screening_rows": {"B-005": ["codex:S1-3"]}},
    "U4-001": {"id": "U4-001", "shard": "s4", "names": ["Undo button"], "rows": ["codex:S4-1"],
               "screening": [], "screening_rows": {}},
}
FLOWS = {"F-EXECUTE", "F-SESSION", "F-PLATFORM"}


def row(unit, disposition="keep", flows=("F-EXECUTE",), same=(), reqs=("X-02: status stays fast",), **extra):
    r = {"unit": unit, "disposition": disposition, "purpose": "Return to the previous state.",
         "reason": "Every flow needs a safe way back.", "flows": list(flows),
         "same_purpose_as": list(same), "requirements": list(reqs)}
    r.update(extra)
    return r


def part(rows, disposition, flows=("F-SESSION",), reqs=()):
    return {"rows": list(rows), "disposition": disposition, "purpose": "A purpose of some rows.",
            "reason": "The reason for this part.", "flows": list(flows), "requirements": list(reqs)}


def split(unit, parts, same=()):
    return {"unit": unit, "disposition": "split", "purpose": "Several purposes in one unit.",
            "reason": "Its rows serve different purposes.", "same_purpose_as": list(same), "parts": parts}


U1_002_DELETE = row("U1-002", "delete", flows=[], reqs=[])


class CheckTests(unittest.TestCase):
    def check(self, data, name="dispositions-s1.json"):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / name
            p.write_text(json.dumps(data), encoding="utf-8")
            return cd.check_file(p, UNITS, FLOWS)

    def test_valid_file_passes(self):
        self.assertEqual(self.check({"shard": "s1", "rows": [row("U1-001", same=["U4-001"]), U1_002_DELETE]}), [])

    def test_missing_unit_and_unknown_disposition_fail(self):
        errors = self.check({"shard": "s1", "rows": [row("U1-001", "maybe")]})
        self.assertTrue(any("disposition must be" in e for e in errors))
        self.assertTrue(any("U1-002 has no row" in e for e in errors))

    def test_kept_unit_needs_a_flow_and_its_open_findings(self):
        errors = self.check({"shard": "s1", "rows": [row("U1-001", flows=[], reqs=[]), U1_002_DELETE]})
        self.assertTrue(any("at least one flow" in e for e in errors))
        self.assertTrue(any("X-02" in e for e in errors))

    def test_automated_purpose_also_carries_its_findings(self):
        errors = self.check({"shard": "s1", "rows": [row("U1-001", "automate", reqs=[]), U1_002_DELETE]})
        self.assertTrue(any("X-02" in e for e in errors))

    def test_bare_finding_id_is_not_a_requirement(self):
        errors = self.check({"shard": "s1", "rows": [row("U1-001", reqs=["X-02"]), U1_002_DELETE]})
        self.assertTrue(any("'<finding id>: <requirement>'" in e for e in errors))

    def test_unknown_flow_link_and_extra_field_fail(self):
        data = {"shard": "s1", "rows": [row("U1-001", flows=["F-NOPE"], same=["U9-999"]),
                                        row("U1-002", "delete", flows=[], reqs=[], extra=1)]}
        errors = self.check(data)
        self.assertTrue(any("flows must be" in e for e in errors))
        self.assertTrue(any("same_purpose_as" in e for e in errors))
        self.assertTrue(any("fields must be" in e for e in errors))

    def test_wrong_shard_duplicate_and_misnamed_file_fail(self):
        errors = self.check({"shard": "s1", "rows": [row("U1-001"), row("U1-001"), row("U4-001"), U1_002_DELETE]})
        self.assertTrue(any("duplicate" in e for e in errors))
        self.assertTrue(any("not a unit of s1" in e for e in errors))
        errors = self.check({"shard": "s1", "rows": [row("U1-001"), U1_002_DELETE]}, name="dispositions-s4.json")
        self.assertTrue(any("expected 's4'" in e for e in errors))

    def test_split_unit_decides_each_part(self):
        ok = split("U1-002", [part(["codex:S1-2", "claude:S1-2"], "redesign"),
                              part(["codex:S1-3", "claude:S1-2"], "automate", reqs=["B-005: answer outside the lock"])])
        self.assertEqual(self.check({"shard": "s1", "rows": [row("U1-001"), ok]}), [])
        missing_row = split("U1-002", [part(["codex:S1-2"], "redesign"), part(["claude:S1-2"], "delete", flows=[])])
        errors = self.check({"shard": "s1", "rows": [row("U1-001"), missing_row]})
        self.assertTrue(any("cover every inventory row" in e for e in errors))
        no_finding = split("U1-002", [part(["codex:S1-2", "claude:S1-2"], "delete", flows=[]), part(["codex:S1-3"], "keep")])
        errors = self.check({"shard": "s1", "rows": [row("U1-001"), no_finding]})
        self.assertTrue(any("U1-002/2" in e and "B-005" in e for e in errors))

    def test_all_requires_each_shard_once_and_every_unit(self):
        with tempfile.TemporaryDirectory() as d:
            s1 = {"shard": "s1", "rows": [row("U1-001"), U1_002_DELETE]}
            for n in range(1, 7):
                (Path(d) / f"dispositions-s{n}.json").write_text(json.dumps(s1), encoding="utf-8")
            errors, rows = cd.check_all(UNITS, FLOWS, directory=d)
            self.assertTrue(any("expected 's4'" in e for e in errors))
            self.assertTrue(any("U4-001" in e for e in errors))


class ReportTests(unittest.TestCase):
    def render(self, rows):
        return build_report.render(UNITS, [{"id": f, "title": f} for f in sorted(FLOWS)], rows)

    def test_mixed_purpose_groups_are_listed(self):
        rows = {"U1-001": row("U1-001", same=["U4-001"]), "U4-001": row("U4-001", "delete", flows=[], reqs=[]),
                "U1-002": row("U1-002", "automate", reqs=["B-005: answer outside the lock"])}
        text = self.render(rows)
        self.assertIn("U1-001 keep and U4-001 delete", text)
        self.assertIn("| U1-002 | automate |", text)

    def test_delete_against_automate_is_listed(self):
        rows = {"U1-001": row("U1-001", "automate", same=["U4-001"]), "U4-001": row("U4-001", "delete", flows=[], reqs=[]),
                "U1-002": U1_002_DELETE}
        self.assertIn("U1-001 automate and U4-001 delete", self.render(rows))

    def test_split_parts_reach_the_signature_table(self):
        rows = {"U1-001": row("U1-001"), "U4-001": row("U4-001", reqs=[]),
                "U1-002": split("U1-002", [part(["codex:S1-2", "claude:S1-2"], "redesign"),
                                           part(["codex:S1-3"], "automate", reqs=["B-005: answer outside the lock"])])}
        self.assertIn("| U1-002/2 | automate |", self.render(rows))


class UnitTests(unittest.TestCase):
    def test_units_cover_every_row_once_and_are_reproducible(self):
        built = build_units.build()
        members = [m for u in built["units"] for m in u["rows"]]
        self.assertEqual(len(members), len(set(members)))
        self.assertEqual(set(members), set(build_units.load_rows()))
        text = json.dumps(built, indent=1, ensure_ascii=False, sort_keys=True) + "\n"
        self.assertEqual(text, (DIR / "units.json").read_text(encoding="utf-8"))

    def test_units_never_cross_shards(self):
        for u in build_units.build()["units"]:
            self.assertEqual({m.split(":")[1].split("-")[0] for m in u["rows"]}, {"S" + u["shard"][1:]})

    def test_route_notations_resolve_across_layers(self):
        self.assertEqual(build_units.normalize("POST experiment/native-command action=undo"),
                         build_units.normalize("route POST /api/experiment/native-command action=undo"))
        self.assertEqual(build_units.normalize("native shell handler registered for command undo (native/ExperimentShell.cs)"),
                         "command undo")
        units = {u["id"]: u for u in build_units.build()["units"]}
        native_undo = next(uid for uid, u in units.items() if "codex:S6-51" in u["rows"])
        adapter_undo = next(uid for uid, u in units.items() if "codex:S3-53" in u["rows"])
        self.assertIn(adapter_undo, units[native_undo]["depends_on_units"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
