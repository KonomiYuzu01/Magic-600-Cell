"""ADR-D02 build steps B28 and B29 (ADR-D02.3, .4, .44 to .46, .49, .50, .52): the research pane's synthetic sample is well formed; never builds
or launches anything.

The sample (tools/looklab/data/samples/research/panel.json) holds one answer per research row the pane reads, each in
the panel's envelope. Its field names are the research panel's own (rust/c600-research/src/panel/answers.rs and
wire.rs; the revision's nine keys are the command layer's Revision, which wire.rs writes), except the body of the draft
row research.record.read, which is a row request and answers in the sample's own words, and the seed, depth and budget
of the draft row research.run.start (B29). The sample covers every verdict
the pane words, each long step's named parts sum to its length, and every claim group's record is in the inventory.
"""
from __future__ import annotations

import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SAMPLE = ROOT / "tools" / "looklab" / "data" / "samples" / "research" / "panel.json"
PANEL = ROOT / "rust" / "c600-research" / "src" / "panel"
REVISION = ROOT / "rust" / "c600-command" / "src" / "receipt.rs"
TABLE = ROOT / "docs" / "progress" / "1.0" / "command-table.json"
READ_ROWS = ["research.archive.inventory", "research.claims", "research.runs", "research.obligations"]
DRAFT_ROW = "research.record.read"
# B29: the run's declaration and start are draft rows; reproduction is a table row the sample answers for its own run.
RUN_DRAFTS = ["research.run.declare", "research.run.start"]
REPRODUCE = "research.run.reproduce"
# The start's own fields: what the run recorded of its declaration (ADR-D02.4, .52).
RUN_FIELDS = {"seed", "depth", "budget"}
ENVELOPE = ["row", "revision", "research", "answer"]
# The draft row's own fields: the record's finding and its folded tree of named parts (ADR-D02.49).
DRAFT_FIELDS = {"rows", "name", "family", "finding", "twists", "parts", "length"}
# What the pane words (ADR-D02.46): the latest check's verdicts and reading, else the preflight's kind.
VERDICTS = {"pass", "fail", "not_applicable", "cut_off", "ready", "unavailable"}


def load() -> dict:
    return json.loads(SAMPLE.read_text(encoding="utf-8"))


def keys(value, found: set[str]) -> set[str]:
    if isinstance(value, dict):
        for key, inner in value.items():
            found.add(key)
            keys(inner, found)
    elif isinstance(value, list):
        for inner in value:
            keys(inner, found)
    return found


def verdict(row: dict) -> str:
    """A claim group's words as the pane reads them (Research.cs, ResearchModel.VerdictOf)."""
    latest = row["latest_check"]
    if latest is None:
        return row["preflight"]["kind"]
    if latest["reading"]["kind"] != "complete":
        return latest["reading"]["kind"]
    kinds = [v["kind"] for v in latest["verdicts"] if isinstance(v, dict)]
    for kind in ("fail", "not_applicable"):
        if kind in kinds:
            return kind
    return "pass" if kinds and all(k == "pass" for k in kinds) else "no_report"


class ResearchSample(unittest.TestCase):
    def setUp(self):
        self.doc = load()
        self.answers = self.doc["answers"]

    def test_format_and_synthetic_evidence(self):
        self.assertEqual(self.doc["format"], "magic600-d02-research-sample/1")
        self.assertEqual(self.doc["evidence"], "synthetic")
        self.assertEqual(sorted(self.answers), sorted(READ_ROWS + [DRAFT_ROW, RUN_DRAFTS[1], REPRODUCE]))

    def test_every_answer_has_the_envelope_and_one_research_digest(self):
        for row, answer in self.answers.items():
            self.assertEqual(sorted(answer), sorted(ENVELOPE), row)
            self.assertEqual(answer["row"], row)
            self.assertRegex(answer["research"], r"^[0-9a-f]{64}$", row)
        self.assertEqual(len({answer["research"] for answer in self.answers.values()}), 1, "more than one research digest")
        self.assertEqual(len({json.dumps(a["revision"], sort_keys=True) for a in self.answers.values()}), 1, "more than one revision")

    def test_field_names_are_the_research_panels(self):
        rust = "".join((PANEL / name).read_text(encoding="utf-8") for name in ("answers.rs", "wire.rs"))
        names = set(re.findall(r"[A-Za-z_][A-Za-z0-9_]*", rust))
        revision = set(re.findall(r"pub (\w+):", REVISION.read_text(encoding="utf-8").split("pub struct Revision {", 1)[1].split("}", 1)[0]))
        self.assertEqual(len(revision), 9, "KN0.2's nine revision keys")
        for row, answer in self.answers.items():
            self.assertEqual(set(answer["revision"]), revision, row)
            body = keys(answer["answer"], set())
            own = DRAFT_FIELDS if row == DRAFT_ROW else RUN_FIELDS if row == RUN_DRAFTS[1] else set()
            self.assertEqual(sorted(body - names - own), [], f"{row}: field names not in answers.rs or wire.rs")

    def test_the_draft_row_is_no_table_row(self):
        table = json.loads(TABLE.read_text(encoding="utf-8"))
        ids = {command["id"] for command in table["commands"]}
        self.assertNotIn(DRAFT_ROW, ids)
        drafts = [command["id"] for command in self.doc["draft_rows"]["commands"]]
        self.assertEqual(drafts, [DRAFT_ROW] + RUN_DRAFTS)
        for row in drafts:
            self.assertNotIn(row, ids)
        self.assertIn(REPRODUCE, ids)
        for row in READ_ROWS:
            self.assertIn(row, ids)

    def test_every_verdict_kind_is_covered(self):
        rows = self.answers["research.claims"]["answer"]["rows"]
        self.assertEqual({verdict(row) for row in rows}, VERDICTS)
        fail = next(row for row in rows if verdict(row) == "fail")
        first = next(v for v in fail["latest_check"]["verdicts"] if v["kind"] == "fail")
        self.assertEqual((first["claim"], first["code"]), ("cycle.closes", "KN6:evidence.certificate_rejected"))
        reasons = [reason for row in rows if verdict(row) == "unavailable" for reason in row["preflight"]["reasons"]]
        self.assertIn("historical_check_incomplete", reasons)

    def test_named_parts_sum_to_their_length(self):
        def check(parts: list, length: int, where: str):
            if parts:
                self.assertEqual(sum(part["length"] for part in parts), length, where)
            for part in parts:
                check(part["parts"], part["length"], where + " › " + part["name"])

        records = self.answers[DRAFT_ROW]["answer"]["rows"]
        self.assertTrue(records)
        for record in records:
            self.assertTrue(record["parts"], record["name"])
            check(record["parts"], record["twists"], record["name"])

    def test_every_claim_and_record_is_in_the_inventory(self):
        inventory = {row["name"] for row in self.answers["research.archive.inventory"]["answer"]["rows"]}
        for row in self.answers["research.claims"]["answer"]["rows"]:
            path = row["group"].split(":", 1)[1].split("#", 1)[0]
            self.assertIn(path, inventory, row["group"])
        for record in self.answers[DRAFT_ROW]["answer"]["rows"]:
            self.assertIn(record["name"], inventory)

    def test_the_sample_run_is_cut_off_at_its_budget(self):
        """B29: the sample's one run has the cut-off run's form: no group, cut off with KN7's budget code, nothing checked;
        the source writes the declared seed, depth and budget into it."""
        rows = self.answers[RUN_DRAFTS[1]]["answer"]["rows"]
        self.assertEqual(len(rows), 1)
        run = rows[0]
        listed = self.answers["research.runs"]["answer"]["rows"][0]
        self.assertEqual(sorted(set(run) - RUN_FIELDS), sorted(listed))
        self.assertEqual((run["group"], run["outcome"], run["code"], run["reading"]["kind"], run["checks"]),
                         (None, "cut_off", "KN7:budget.steps", "cut_off", []))
        self.assertEqual(self.answers[REPRODUCE]["answer"]["answer"]["kind"], "same")


if __name__ == "__main__":
    unittest.main()
