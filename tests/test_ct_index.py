"""Tests for tools/language/ct_index.py: the table parser, the marker scanner, the notes and the repository index."""
from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools" / "language"))
import ct_index  # noqa: E402

TABLE = """## 15. The conformance suite by first phase

### 15.3. LP2

| Component | source | synthetic | windows | performance | human |
| --- | --- | --- | --- | --- | --- |
| KN1 | CT-KN1-01, CT-KN1-02 | CT-KN1-03 | none | none | none |
| ONE | CT-ONE-01 | none | none | CT-ONE-02 | none |

### 15.4. LP3

| Component | source | synthetic | windows | performance | human |
| --- | --- | --- | --- | --- | --- |
| LB1 | CT-LB1-01 | none | none | none | none |
"""

IDS = {"CT-KN1-01": "source", "CT-KN1-02": "source", "CT-KN1-03": "synthetic", "CT-ONE-01": "source", "CT-ONE-02": "performance"}


class TableTests(unittest.TestCase):
    def test_columns_are_kept_and_other_phases_ignored(self):
        self.assertEqual(ct_index.parse_table(TABLE, "LP2"), IDS)
        self.assertEqual(ct_index.parse_table(TABLE, "LP3"), {"CT-LB1-01": "source"})

    def test_unknown_phase_and_duplicates_are_refused(self):
        with self.assertRaises(ct_index.NotesError):
            ct_index.parse_table(TABLE, "LP9")
        with self.assertRaises(ct_index.NotesError):
            ct_index.parse_table(TABLE.replace("CT-ONE-02", "CT-ONE-01"), "LP2")


class ScannerTests(unittest.TestCase):
    def test_shared_marker(self):
        found = ct_index.scan_text("/// CT-KN1-01, CT-KN1-02 and CT-KN1-03 (whole: all three)\n")
        for test_id in ("CT-KN1-01", "CT-KN1-02", "CT-KN1-03"):
            self.assertTrue(found[test_id]["whole"], test_id)
            self.assertFalse(found[test_id]["part"])

    def test_part_text_keeps_not_run_across_lines(self):
        text = (
            "/// CT-KN1-01 (part: the first half; not run: the second\n"
            "/// half, which waits for M8) and CT-KN1-02 (whole)\n"
        )
        found = ct_index.scan_text(text)
        self.assertTrue(found["CT-KN1-01"]["part"])
        self.assertEqual(found["CT-KN1-01"]["not_run"], ["the second half, which waits for M8"])
        self.assertTrue(found["CT-KN1-02"]["whole"])
        self.assertFalse(found["CT-KN1-02"]["part"])

    def test_part_without_not_run_text(self):
        found = ct_index.scan_text("# CT-ONE-01 (part: one case)\n")
        self.assertTrue(found["CT-ONE-01"]["part"])
        self.assertEqual(found["CT-ONE-01"]["not_run"], [])

    def test_named_only_and_ids_after_or_inside_a_marker(self):
        found = ct_index.scan_text("// CT-KN1-01 is described here\n// CT-ONE-01 (part: x; not run: CT-ONE-02) then CT-KN1-03\n")
        self.assertEqual((found["CT-KN1-01"]["named"], found["CT-KN1-01"]["whole"], found["CT-KN1-01"]["part"]), (True, False, False))
        self.assertTrue(found["CT-ONE-01"]["part"])
        self.assertTrue(found["CT-ONE-02"]["named"] and not found["CT-ONE-02"]["part"])
        self.assertTrue(found["CT-KN1-03"]["named"] and not found["CT-KN1-03"]["whole"])

    def test_plural_part_marker_and_other_parentheses(self):
        found = ct_index.scan_text("/// CT-KN1-01 to CT-KN1-02 (parts; their rows run elsewhere), CT-KN1-03 (see above)\n")
        self.assertTrue(found["CT-KN1-01"]["part"] and found["CT-KN1-02"]["part"])
        self.assertTrue(found["CT-KN1-03"]["named"] and not found["CT-KN1-03"]["part"])

    def test_marker_may_name_the_evidence_kind_first(self):
        text = "/// CT-DET-28 (synthetic, part: x; not run: y (z)), CT-DET-29 (source, whole) and CT-DET-30 (other, part)\n"
        found = ct_index.scan_text(text)
        self.assertTrue(found["CT-DET-28"]["part"])
        self.assertEqual(found["CT-DET-28"]["not_run"], ["y (z)"])
        self.assertTrue(found["CT-DET-29"]["whole"])
        self.assertTrue(found["CT-DET-30"]["named"] and not found["CT-DET-30"]["part"])


class NotesTests(unittest.TestCase):
    def test_notes_table(self):
        text = "intro\n\n| ID | note kind | text |\n| --- | --- | --- |\n| CT-KN1-03 | waiting | for M8 |\n"
        self.assertEqual(ct_index.parse_notes(text, IDS), {"CT-KN1-03": ("waiting", "for M8")})

    def test_refusals(self):
        for row in ("| CT-LB1-01 | waiting | outside the phase |", "| CT-KN1-03 | pending | unknown kind |"):
            with self.assertRaises(ct_index.NotesError, msg=row):
                ct_index.parse_notes(row + "\n", IDS)
        twice = "| CT-KN1-03 | waiting | a |\n| CT-KN1-03 | waiting | b |\n"
        with self.assertRaises(ct_index.NotesError):
            ct_index.parse_notes(twice, IDS)

    def test_states(self):
        scan = {
            "CT-KN1-01": {"whole": [], "part": ["a.rs"], "named": [], "not_run": [("a.rs", "the rest")]},
            "CT-KN1-02": {"whole": ["b.rs"], "part": [], "named": [], "not_run": []},
            "CT-ONE-01": {"whole": [], "part": [], "named": ["docs.rs"], "not_run": []},
        }
        notes = {
            "CT-KN1-01": ("whole by union", "with b"),
            "CT-ONE-01": ("missing despite name", "only docs"),
            "CT-KN1-03": ("waiting", "M8"),
            "CT-ONE-02": ("not applicable", "none here"),
        }
        rows = {row["id"]: row for row in ct_index.build_index(IDS, scan, notes)}
        self.assertEqual({k: v["state"] for k, v in rows.items()}, {
            "CT-KN1-01": "whole", "CT-KN1-02": "whole", "CT-KN1-03": "waiting", "CT-ONE-01": "missing", "CT-ONE-02": "not applicable"})
        self.assertTrue(rows["CT-KN1-01"]["by_union"])
        self.assertFalse(rows["CT-KN1-02"]["by_union"])
        plain = {row["id"]: row["state"] for row in ct_index.build_index(IDS, scan, {})}
        self.assertEqual((plain["CT-KN1-01"], plain["CT-ONE-01"], plain["CT-KN1-03"]), ("part", "named", "missing"))
        line = ct_index.summary_line("LP2", list(rows.values()))
        self.assertIn(
            "2 whole, of which 1 by union; 0 part; 0 named only; 1 missing; 1 not applicable; 1 waiting; "
            "0 indexed elsewhere", line)

    def test_waiting_and_indexed_elsewhere_override_part_but_never_whole(self):
        part = {"whole": [], "part": ["a.rs"], "named": [], "not_run": [("a.rs", "the rest")]}
        scan = {"CT-KN1-01": part, "CT-KN1-02": dict(part), "CT-KN1-03": {**part, "whole": ["c.rs"]}}
        notes = {
            "CT-KN1-01": ("waiting", "LQ2"),
            "CT-KN1-02": ("indexed elsewhere", "LP3's host"),
            "CT-KN1-03": ("indexed elsewhere", "LP3's host"),
            "CT-ONE-01": ("indexed elsewhere", "LP4"),
        }
        rows = {row["id"]: row for row in ct_index.build_index(IDS, scan, notes)}
        self.assertEqual(
            [rows[i]["state"] for i in ("CT-KN1-01", "CT-KN1-02", "CT-KN1-03", "CT-ONE-01")],
            ["waiting", "indexed elsewhere", "whole", "indexed elsewhere"])
        # the marker's not-run text stays beside the note
        self.assertEqual(rows["CT-KN1-02"]["not_run_or_waiting_for"], "LP3's host | the rest (a.rs)")
        self.assertEqual(rows["CT-KN1-01"]["not_run_or_waiting_for"], "LQ2 | the rest (a.rs)")
        self.assertIn("1 waiting; 2 indexed elsewhere", ct_index.summary_line("LP2", list(rows.values())))


class RepositoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.ids, cls.scan, cls.notes, cls.rows = ct_index.compute(ROOT, "LP2")

    def test_every_lp2_id_has_exactly_one_state(self):
        self.assertEqual(len(self.ids), 248)
        self.assertEqual([row["id"] for row in self.rows], list(self.ids))
        self.assertTrue(all(row["state"] in ct_index.STATES for row in self.rows))
        count = ct_index.totals(self.rows)
        self.assertEqual(sum(count[state] for state in ct_index.STATES), 248)

    def test_notes_name_only_lp2_ids(self):
        self.assertTrue(set(self.notes) <= set(self.ids))

    def test_markers_name_ids_of_section_15(self):
        text = (ROOT / ct_index.CHAPTER).read_text(encoding="utf-8")
        known = set()
        for phase in ("LP0", "LP1", "LP2", "LP3", "LP4", "LP5", "LP6"):
            try:
                known |= set(ct_index.parse_table(text, phase))
            except ct_index.NotesError:
                pass  # LP0's table holds no IDs
        marked = {i for i, item in self.scan.items() if item["whole"] or item["part"]}
        strays = sorted(marked - known)
        if strays:
            print(f"warning: markers name IDs outside section 15: {' '.join(strays)}", file=sys.stderr)

    def test_other_phase_runs(self):
        _ids, _scan, _notes, rows = ct_index.compute(ROOT, "LP3")
        self.assertTrue(rows)


if __name__ == "__main__":
    unittest.main()
