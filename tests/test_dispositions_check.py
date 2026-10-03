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
    "U1-001": {"id": "U1-001", "shard": "s1", "names": ["Undo"], "screening": ["X-02 (open)"]},
    "U1-002": {"id": "U1-002", "shard": "s1", "names": ["Auto demo"], "screening": []},
    "U4-001": {"id": "U4-001", "shard": "s4", "names": ["Undo button"], "screening": []},
}
FLOWS = {"F-EXECUTE", "F-PLATFORM"}


def row(unit, disposition="keep", flows=("F-EXECUTE",), same=(), reqs=("X-02: status stays fast",), **extra):
    r = {"unit": unit, "disposition": disposition, "purpose": "Return to the previous state.",
         "reason": "Every flow needs a safe way back.", "flows": list(flows),
         "same_purpose_as": list(same), "requirements": list(reqs)}
    r.update(extra)
    return r


class CheckTests(unittest.TestCase):
    def check(self, data):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "dispositions-s1.json"
            p.write_text(json.dumps(data), encoding="utf-8")
            return cd.check_file(p, UNITS, FLOWS)

    def test_valid_file_passes(self):
        data = {"shard": "s1", "rows": [row("U1-001", same=["U4-001"]),
                                        row("U1-002", "delete", flows=[], reqs=[])]}
        self.assertEqual(self.check(data), [])

    def test_missing_unit_and_unknown_disposition_fail(self):
        errors = self.check({"shard": "s1", "rows": [row("U1-001", "maybe")]})
        self.assertTrue(any("disposition must be" in e for e in errors))
        self.assertTrue(any("U1-002 has no row" in e for e in errors))

    def test_kept_unit_needs_a_flow_and_its_open_findings(self):
        errors = self.check({"shard": "s1", "rows": [row("U1-001", flows=[], reqs=[]),
                                                     row("U1-002", "delete", flows=[], reqs=[])]})
        self.assertTrue(any("at least one flow" in e for e in errors))
        self.assertTrue(any("X-02" in e for e in errors))

    def test_unknown_flow_link_and_extra_field_fail(self):
        data = {"shard": "s1", "rows": [row("U1-001", flows=["F-NOPE"], same=["U9-999"]),
                                        row("U1-002", "delete", flows=[], reqs=[], extra=1)]}
        errors = self.check(data)
        self.assertTrue(any("flows must be" in e for e in errors))
        self.assertTrue(any("same_purpose_as" in e for e in errors))
        self.assertTrue(any("fields must be" in e for e in errors))

    def test_wrong_shard_and_duplicate_fail(self):
        errors = self.check({"shard": "s1", "rows": [row("U1-001"), row("U1-001"), row("U4-001"),
                                                     row("U1-002", "delete", flows=[], reqs=[])]})
        self.assertTrue(any("duplicate" in e for e in errors))
        self.assertTrue(any("not a unit of s1" in e for e in errors))


class ReportTests(unittest.TestCase):
    def test_mixed_purpose_group_is_listed(self):
        rows = {"U1-001": row("U1-001", same=["U4-001"]), "U4-001": row("U4-001", "delete", flows=[], reqs=[]),
                "U1-002": row("U1-002", "automate", flows=[], reqs=[])}
        text = build_report.render(UNITS, [{"id": f, "title": f} for f in sorted(FLOWS)], rows)
        self.assertIn("U1-001 keep, U4-001 delete", text)
        self.assertIn("| U1-002 | automate |", text)


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


if __name__ == "__main__":
    unittest.main(verbosity=2)
