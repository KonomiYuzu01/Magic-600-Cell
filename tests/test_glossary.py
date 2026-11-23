"""The glossary check and the language learning path (LP4-11): EV5.6, EV5.10, EV5.14, EV5.15 and EV5.18.

CT-EV5-02 (part: the integrated glossary covers chapter 04 section 6's rows, every entry has build evidence, each
sense of EV5.18's homonyms has its own row, each step's prerequisite concepts are taught before that step's task, and
the steps' prerequisites form a DAG; not run: that the proposed changes have been reviewed, because the
mathematician's review of LP4-11's integration is not recorded yet (each such row says so in `review`); that flow
goals need no undisclosed concept, because the disclosure data is LP4-11's part in c600-command; and BS3's branch of
EV5.9, which waits for LQ2's option set).

The check is `docs/progress/1.0/glossary/build.py --check`. These tests run it on the repository's data and on
planted documents, which shows that each refusal can fail: an unknown term, an ambiguous term or citation, a
definition above its level, a learner-facing definition that uses a row that is not learner-facing, a bare homonym,
a learning path with a cycle, a concept taught after its use, and a fixture that changed or whose answer is a word.

This is source evidence about the glossary's data and the learning path's fixtures. The human tests of the path wait
for the owner's form (EV5.12); nothing here is a learner's result.
"""
from __future__ import annotations

import copy
import importlib.util
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
GLOSSARY = ROOT / "docs/progress/1.0/glossary"
CHAPTER = ROOT / "docs/architecture/1.0/language/architecture/04-environment.md"

_spec = importlib.util.spec_from_file_location("glossary_build", GLOSSARY / "build.py")
build = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(build)


def chapter_section_6() -> tuple[list[str], list[str]]:
    """The term column of chapter 04 section 6's inventory, and the bold qualified names of EV5.18's table."""
    text = CHAPTER.read_text(encoding="utf-8")
    body = text[text.index("## 6. Glossary entries"):text.index("## 7. ")]
    terms, senses, table = [], [], None
    for line in body.split("\n"):
        if line.startswith("| Term | Proposed level"):
            table = "terms"
        elif line.startswith("| Word | Qualified names"):
            table = "senses"
        elif line.startswith("| ") and not line.startswith("| ---") and table:
            cells = [c.strip() for c in line.strip().strip("|").split(" | ")]
            if table == "terms":
                terms.append(cells[0])
            else:
                senses += re.findall(r"\*\*([^*]+)\*\*", cells[1])
    return terms, senses


def planted() -> dict:
    """A small glossary on its own: three levels, a homonym and a row that is not learner-facing."""
    return {
        "levels": [{"id": "L1", "name": "one", "workflow": "-"}, {"id": "L2", "name": "two", "workflow": "-"},
                   {"id": "L6", "name": "six", "workflow": "-"}],
        "citations": {"documents": ["Chapter", "Theory", "Partition"], "clauses": ["KN", "EV"]},
        "homonyms": [{"word": "Domain", "senses": ["Alignment domain", "Declared domain"], "bare": None}],
        "terms": [
            {"id": "cap", "term": "Cap", "level": "L1", "definition": "A grip and cut.", "source": "KN1.5"},
            {"id": "word", "term": "Word", "level": "L2", "definition": "Letters of a [[cap]] menu.",
             "source": "Chapter 01 KN2"},
            {"id": "alignment-domain", "term": "Alignment domain", "level": "L6",
             "definition": "A coset of [[words|Word]].", "source": "Theory Lemma 7"},
            {"id": "declared-domain", "term": "Declared domain", "level": "L6", "definition": "Defined inputs.",
             "source": "EV5.18"},
            {"id": "free-mode", "term": "Free mode", "level": "L6", "learner_facing": False,
             "definition": "An infinite alphabet.", "source": "Chapter 01 KN1"},
        ],
    }


class Planted(unittest.TestCase):
    """Each refusal of the term check, on a planted glossary with its own diagram folder."""

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.saved = build.DIAGRAMS
        build.DIAGRAMS = Path(self.tmp.name)

    def tearDown(self) -> None:
        build.DIAGRAMS = self.saved
        self.tmp.cleanup()

    def errors(self, doc: dict) -> list[str]:
        for t in doc["terms"]:
            (build.DIAGRAMS / f"{t['id']}.mmd").write_bytes(b"flowchart LR\n  A --> B\n")
        return build.term_errors(doc)

    def edited(self, tid: str, **fields) -> dict:
        doc = copy.deepcopy(planted())
        row = next(t for t in doc["terms"] if t["id"] == tid)
        row.update(fields)
        return doc

    def test_the_planted_glossary_passes(self) -> None:
        self.assertEqual(self.errors(planted()), [])

    def test_an_unknown_term_is_refused(self) -> None:
        errs = self.errors(self.edited("word", definition="Letters of a [[grip]] menu."))
        self.assertTrue(any("unknown term 'grip'" in e for e in errs), errs)

    def test_a_reference_to_a_homonym_without_a_bare_sense_is_ambiguous(self) -> None:
        errs = self.errors(self.edited("free-mode", definition="An infinite [[domain]]."))
        self.assertTrue(any("ambiguous term 'domain'" in e for e in errs), errs)

    def test_a_named_sense_resolves(self) -> None:
        self.assertEqual(self.errors(self.edited("free-mode", definition="An infinite [[domain|Declared domain]].")),
                         [])

    def test_a_bare_homonym_in_prose_is_refused_and_an_ordinary_word_mark_is_not(self) -> None:
        errs = self.errors(self.edited("cap", definition="A grip whose domain is a cut."))
        self.assertTrue(any("bare word 'Domain'" in e for e in errs), errs)
        self.assertEqual(self.errors(self.edited("cap", definition="A grip whose [[domain|~]] is a cut.")), [])

    def test_a_sense_without_a_row_is_refused(self) -> None:
        doc = planted()
        doc["homonyms"][0]["senses"].append("Chart domain")
        self.assertTrue(any("'Chart domain' has no row" in e for e in self.errors(doc)))

    def test_a_definition_above_its_level_is_refused_unless_recorded(self) -> None:
        doc = self.edited("cap", definition="A grip and cut of [[words|Word]].")
        self.assertTrue(any("above its own level L1" in e for e in self.errors(doc)))
        row = next(t for t in doc["terms"] if t["id"] == "cap")
        row["forward"] = [{"term": "Word", "why": "a contrast only"}]
        self.assertEqual(self.errors(doc), [])
        row["forward"] = [{"term": "Word", "why": ""}]
        self.assertTrue(any("gives no reason" in e for e in self.errors(doc)))
        row["definition"] = "A grip and cut."
        self.assertTrue(any("does not make" in e for e in self.errors(doc)))

    def test_a_learner_facing_definition_cannot_use_a_row_that_is_not(self) -> None:
        errs = self.errors(self.edited("alignment-domain", definition="A coset in [[free mode]]."))
        self.assertTrue(any("not learner-facing" in e for e in errs), errs)
        self.assertEqual(self.errors(self.edited("free-mode", definition="Infinite [[words|Word]].")), [])

    def test_ambiguous_citations_are_refused(self) -> None:
        for bad in ("Lemma 7", "M3", "default 10", "access policy: LQ14"):
            errs = self.errors(self.edited("cap", source=bad))
            self.assertTrue(any("ambiguous citation" in e for e in errs), (bad, errs))
        for good in ("Theory Lemma 7", "KN1.5", "Partition default 10", "Chapter 01 KN1.5; EV5.18",
                     "core.py PuzzleState", "Chapter 07 BND.14 (a reading; the term is used)"):
            self.assertEqual(self.errors(self.edited("cap", source=good)), [], good)
        errs = self.errors(self.edited("cap", source="research/no-such-file.tex, Lemma 3"))
        self.assertTrue(any("does not exist" in e for e in errs), errs)
        errs = self.errors(self.edited("cap", source="[card 6](../../no/such.md)"))
        self.assertTrue(any("names no file" in e for e in errs), errs)


class Repository(unittest.TestCase):
    """The repository's glossary and learning path."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.doc = build.load()
        cls.path = build.load(build.PATH)

    def test_the_check_passes(self) -> None:
        self.assertEqual(build.check(self.doc, self.path), [])

    def test_every_section_6_row_is_integrated(self) -> None:
        terms, _ = chapter_section_6()
        self.assertGreater(len(terms), 200)
        integrated = {t.get("chapter_term") for t in self.doc["terms"]}
        self.assertEqual([t for t in terms if t not in integrated], [])

    def test_each_sense_of_ev5_18_has_its_own_row(self) -> None:
        _, senses = chapter_section_6()
        self.assertGreater(len(senses), 40)
        names = build.index(self.doc)
        unresolved = [s for s in senses if build.resolve(names, s)[1]]
        self.assertEqual(unresolved, [])
        for h in self.doc["homonyms"]:
            ids = {build.resolve(names, s)[0] for s in h["senses"]}
            self.assertEqual(len(ids), len(h["senses"]), h["word"])

    def test_every_entry_has_its_diagram_evidence(self) -> None:
        manifest = json.loads(build.MANIFEST.read_text(encoding="utf-8"))
        self.assertEqual(manifest["tool"], "@mermaid-js/mermaid-cli")
        self.assertTrue(manifest["version"] and manifest["command"])
        self.assertEqual(set(manifest["diagrams"]), {t["id"] for t in self.doc["terms"]})
        self.assertEqual(build.manifest_errors(self.doc), [])

    def test_rows_that_wait_for_review_say_so(self) -> None:
        for t in self.doc["terms"]:
            if t.get("origin") != "M6" or "chapter_term" in t:
                self.assertIn("review", t["review"], t["id"])

    def test_generated_files_use_lf(self) -> None:
        for path in (build.PAGE, build.TERMS, build.PATH, build.MANIFEST):
            self.assertNotIn(b"\r", path.read_bytes(), path.name)

    def test_seven_steps_with_one_fixture_each(self) -> None:
        self.assertEqual([s["id"] for s in self.path["steps"]], [f"S{n}" for n in range(1, 8)])
        for s in self.path["steps"]:
            self.assertIn(s["fixture"]["answer"]["kind"], build.ANSWER_KINDS)
            self.assertTrue(s["fixture"]["reads"])


class PlantedPath(unittest.TestCase):
    """Each refusal of the learning-path check, on edited copies of the repository's path."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.doc = build.load()
        cls.path = build.load(build.PATH)

    def errors(self, edit) -> list[str]:
        path = copy.deepcopy(self.path)
        edit(path)
        return build.path_errors(self.doc, path)

    def test_a_cycle_is_refused(self) -> None:
        errs = self.errors(lambda p: p["steps"][0]["after"].append("S7"))
        self.assertTrue(any("form a cycle" in e for e in errs), errs)

    def test_a_concept_taught_after_its_use_is_refused(self) -> None:
        def move(p):
            p["steps"][0]["closure"].remove("Statement")
            p["steps"][6]["closure"].append("Statement")
        errs = self.errors(move)
        self.assertTrue(any("uses 'Statement', which no step up to S1 teaches" in e for e in errs), errs)

    def test_a_row_that_is_not_learner_facing_cannot_be_taught(self) -> None:
        errs = self.errors(lambda p: p["steps"][5]["concepts"].append("Free mode"))
        self.assertTrue(any("not learner-facing" in e for e in errs), errs)

    def test_a_fixture_never_answers_with_a_word_or_a_macro(self) -> None:
        for kind in ("word", "macro", "solving macro"):
            errs = self.errors(lambda p: p["steps"][3]["fixture"]["answer"].update(kind=kind))
            self.assertTrue(any("never chooses or outputs" in e for e in errs), (kind, errs))

    def test_a_changed_or_missing_instance_is_refused(self) -> None:
        errs = self.errors(lambda p: p["steps"][0]["fixture"]["reads"][0].update(sha256="0" * 64))
        self.assertTrue(any("differs from its recorded digest" in e for e in errs), errs)
        errs = self.errors(lambda p: p["steps"][0]["fixture"]["reads"][0].update(path="research/none.json"))
        self.assertTrue(any("does not exist" in e for e in errs), errs)
        errs = self.errors(lambda p: p["steps"][0]["fixture"]["reads"][0]["expect"][0].update(value=1))
        self.assertTrue(any("/run/phi" in e for e in errs), errs)

    def test_a_fixture_concept_must_be_taught_first(self) -> None:
        errs = self.errors(lambda p: p["steps"][0]["fixture"]["concepts"].append("Gauge"))
        self.assertTrue(any("'Gauge' is not taught before the task" in e for e in errs), errs)

    def test_an_assumed_term_comes_from_the_manual_or_the_solving_glossary(self) -> None:
        errs = self.errors(lambda p: p["assumed"].append("Reproducibility key"))
        self.assertTrue(any("neither a solving-glossary term nor used by the manual" in e for e in errs), errs)


if __name__ == "__main__":
    unittest.main()
