"""ADR-D02.43 (build step B27): every term of art in D02's words is a glossary row or one of the record's requested
additions; never builds or launches anything.

D02's words are what its surfaces show: Look Lab's jumble-words.json (sentences, names, synonyms, row display names
and the build's own words), the draft rows' labels in command-rows-d02.json and the held seat entries' labels in
keymap.json. The rows' notes are help prose that K2 replaces, so they are not read. Each word is read, longest phrase
first, as one of these:

- a glossary row (LP4-11's terms.json, by its term or an alias, a plural included), through build.py's own index;
- one of the eleven additions ADR-D02 section 15 requests (PENDING_TERMS), until the glossary has the row;
- a reading of one glossary row that D02's words use (READINGS), or a homonym's sense in D02 (CONTEXT_SENSE);
- a proper name (NAMES), a placeholder variable (VARIABLES), or a plain word (PLAIN_WORDS).

A plain word is ordinary English or general geometry, never a glossary name. A word that is none of these fails the
check: it is a term of art the glossary lacks, and it needs a row or a request. Every list here must be used.
"""
from __future__ import annotations

import importlib.util
import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DESIGN = ROOT / "tools" / "looklab" / "data" / "design"
GLOSSARY = ROOT / "docs" / "progress" / "1.0" / "glossary"
ADR = ROOT / "docs" / "architecture" / "1.0" / "design" / "adr" / "ADR-D02-jumble-research.md"

# ADR-D02 section 15: the eleven glossary additions the record requests, with their levels. A term leaves this list
# when the glossary gets its row.
PENDING_TERMS = {
    "Straddler": "L1", "Cap status": "L1", "Cap class": "L1", "Named cap": "L1", "Meeting cap": "L1", "Contact class": "L1",
    "Repair": "L1", "Twist check": "L1", "Word check": "L2", "Step flag": "L6", "Undefined": "L1",
}

# D02's readings of one glossary row each: the words on the left name the row on the right.
READINGS = {
    "jumble": "Jumble on and Jumble off",  # the mode's own name, as in "Jumble entry slot"
    "jumble on": "Jumble on and Jumble off",
    "jumble off": "Jumble on and Jumble off",
    "F_c": "Floor",  # the floor of cap c, beside N_c
    "ΔN": "N",  # the change in N a repair or a twist makes
    "Δu": "u",
    "Δn_h": "Height counts",  # the change in the height counts
    "rev": "Revision",
    "derived": "Derived state",  # the research verdict: the state is derived
    "reachability": "Reachability claim",  # "reachability open": the derived state's claim is open
}

# Homonyms without a bare sense: the sense D02's words use.
CONTEXT_SENSE = {
    "twist": ("Twist, jumbling puzzle", "D02 shows the jumbled puzzle's twists"),
    "projection": ("Display projection", "Global's projection chain (ADR-D02.8) is the display projection's"),
    "covers": ("Cap cover", "the Cap covers section's caption"),
}

NAMES = {
    "600-cell-full": "the puzzle's model profile",
    "hopf": "Heinz Hopf: the Hopf rings are the Hopf fibration's great circles",
    "global": "the name of the first view",
    "f1": "the key that opens the command palette (Local's default-cap hint names it)",
}

# Placeholder letters in the words' templates (k of n, x d, y z); case matters, so N stays the glossary's N.
VARIABLES = set("dijkmnprxyz")

PLAIN_WORDS = {
    "a", "above", "after", "and", "antipode", "applicable", "apply", "are", "at", "beside", "by", "byte", "bytes",
    "certifies", "chain", "checked", "checker", "clear", "click", "clip", "compare", "covered", "cut", "declare", "differs",
    "does", "entry", "equal", "export", "exposed", "face", "faces", "fails", "file", "focus", "free", "from", "hold",
    "holds", "holes", "in", "input", "into", "it", "least", "live", "lowers", "map", "mark",
    "marks", "mode", "model", "name", "need", "needs", "neighbours", "no", "none", "not", "numbered", "of", "on", "one",
    "only", "or", "output", "pairs", "part", "pass", "pinned", "predicted", "preset", "radius", "raw", "read",
    "re-executed", "record", "refuse", "refused", "release", "replayed", "reproduced", "research", "revert", "reviewed",
    "ring", "rings", "row", "run", "save", "set", "sheet", "shell", "simulation", "slice", "slot", "slots", "solving",
    "split", "stage", "start", "step", "swap", "the", "this", "tint", "to", "tried", "try", "tuning",
    "un-name", "unavailable", "uses", "whole", "within", "selector",
}

TOKEN = re.compile(r"[\w]+(?:[-'’][\w]+)*")
PLACEHOLDER = re.compile(r"<[^<>]*>")


def build_module():
    spec = importlib.util.spec_from_file_location("glossary_build", GLOSSARY / "build.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def shown_words() -> list[tuple[str, str]]:
    """(where, text) for every word D02's surfaces show."""
    words = load(DESIGN / "jumble-words.json")
    out = [("sentence " + s["id"], s["text"]) for s in words["sentences"]]
    out += [("name " + n["id"], n["text"]) for n in words["names"]]
    out += [("synonym of " + k, v) for k, vs in words["synonyms"].items() for v in vs]
    out += [("row " + k, v) for k, v in words["rows"].items()]
    out += [("built " + b["id"], b["text"]) for b in words.get("built", [])]
    rows = load(DESIGN / "command-rows-d02.json")
    out += [("label " + c["id"], c["label"]) for c in rows["commands"]]
    keymap = load(DESIGN / "keymap.json")
    out += [("seat " + str(h["seat"]), h["label"]) for h in keymap.get("seats", {}).get("held", [])]
    return out


def tokens(text: str) -> list[str]:
    found = []
    for t in TOKEN.findall(PLACEHOLDER.sub(" ", text)):
        t = re.sub(r"['’]s$", "", t)
        if not t.isdigit():
            found.append(t)
    return found


def symbolic(token: str) -> bool:
    """A symbol whose case matters: one letter, or one with an underscore or a non-Latin letter."""
    return len(token) == 1 or "_" in token or any(not ("a" <= c.lower() <= "z" or c.isdigit() or c in "-'") for c in token)


def singular(word: str) -> list[str]:
    forms = [word]
    if word.endswith("es"):
        forms.append(word[:-2])
    if word.endswith("s"):
        forms.append(word[:-1])
    return forms


class Reader:
    def __init__(self):
        self.build = build_module()
        self.doc = self.build.load()
        self.index = self.build.index(self.doc)
        self.by_id = {t["id"]: t for t in self.doc["terms"]}
        self.exact = {}
        for t in self.doc["terms"]:
            for name in [t["term"]] + list(t.get("aliases", [])):
                self.exact.setdefault(name, set()).add(t["id"])
        self.pending = {p.lower() for p in PENDING_TERMS}
        self.used: dict[str, set] = {"pending": set(), "readings": set(), "sense": set(), "names": set(), "plain": set()}

    def phrase(self, parts: list[str]) -> str | None:
        """The reading of these tokens as one phrase, or None; an ambiguous glossary name raises."""
        joined = " ".join(parts)
        if len(parts) == 1 and symbolic(parts[0]):
            for form in singular(joined):
                if form in READINGS:
                    self.used["readings"].add(form)
                    return "reading:" + READINGS[form]
                if form in self.exact:
                    return "glossary:" + ",".join(sorted(self.exact[form]))
            return None
        for form in singular(joined.lower()):
            if form in self.pending:
                self.used["pending"].add(form)
                return "pending:" + form
            if form in READINGS:
                self.used["readings"].add(form)
                return "reading:" + READINGS[form]
            ids = self.index.get(form)
            if ids:
                if len(ids) == 1:
                    return "glossary:" + ids[0]
                if form in CONTEXT_SENSE:
                    self.used["sense"].add(form)
                    return "sense:" + CONTEXT_SENSE[form][0]
                raise ValueError(f"'{joined}' is an ambiguous glossary name ({', '.join(sorted(ids))}); name its sense in CONTEXT_SENSE")
        return None

    def read(self, text: str) -> list[str]:
        """The words of one text that are none of the kinds above."""
        parts = tokens(text)
        unknown = []
        i = 0
        while i < len(parts):
            for n in range(min(4, len(parts) - i), 0, -1):
                if self.phrase(parts[i:i + n]) is not None:
                    i += n
                    break
            else:
                word = parts[i]
                low = word.lower()
                if low in NAMES:
                    self.used["names"].add(low)
                elif len(word) == 1 and word in VARIABLES:
                    pass
                elif low in PLAIN_WORDS:
                    self.used["plain"].add(low)
                else:
                    unknown.append(word)
                i += 1
        return unknown


def requested_additions() -> dict[str, str]:
    """Section 15's glossary additions as written in the record: term -> level."""
    text = " ".join(ADR.read_text(encoding="utf-8").split())
    match = re.search(r"\*\*Glossary \(LP4-11's file\):\*\* (\w+) additions \(([^()]*)\)", text)
    if not match:
        raise AssertionError("ADR-D02 section 15 has no glossary additions line")
    found = {}
    for group in match.group(2).split(";"):
        names, level = group.strip().rsplit(" at ", 1)
        for name in names.split(","):
            found[name.strip()] = level.strip()
    return found


class D02WordsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.reader = Reader()
        cls.words = shown_words()
        cls.unknown = {where: cls.reader.read(text) for where, text in cls.words}

    def test_every_term_of_art_is_a_glossary_row_or_requested(self):
        missing = {where: words for where, words in self.unknown.items() if words}
        self.assertEqual(missing, {}, "words that are no glossary row, request, reading, name or plain word")

    def test_pending_terms_are_the_records_requests_not_yet_rows(self):
        requested = requested_additions()
        self.assertEqual(len(requested), 11, requested)
        for term, level in PENDING_TERMS.items():
            self.assertEqual(requested.get(term), level, f"{term} is not requested at {level} in section 15")
        names = self.reader.index
        for term in requested:
            if term in PENDING_TERMS:
                self.assertFalse(term.lower() in names, f"{term} is a glossary row now: move it out of PENDING_TERMS")
            else:
                self.assertTrue(term.lower() in names, f"section 15 requests {term}, which is neither pending nor a row")

    def test_readings_and_senses_name_one_row_each(self):
        terms = {t["term"]: t["id"] for t in self.reader.doc["terms"]}
        for word, term in READINGS.items():
            self.assertIn(term, terms, f"the reading of '{word}' names no glossary row")
            self.assertFalse(word.lower() in self.reader.index, f"'{word}' is a glossary name itself; drop its reading")
        for word, (sense, _why) in CONTEXT_SENSE.items():
            ids = self.reader.index.get(word, [])
            self.assertGreater(len(ids), 1, f"'{word}' is not an ambiguous glossary name")
            self.assertIn(terms.get(sense), ids, f"{sense} is not a sense of '{word}'")

    def test_plain_words_and_names_are_no_glossary_names(self):
        for word in sorted(PLAIN_WORDS | set(NAMES)):
            self.assertFalse(word in self.reader.index, f"'{word}' is a glossary name, not a plain word")
            self.assertFalse(word in self.reader.pending, f"'{word}' is a requested term, not a plain word")

    def test_every_list_entry_is_used(self):
        self.assertEqual(sorted(set(READINGS) - self.reader.used["readings"]), [])
        self.assertEqual(sorted(set(CONTEXT_SENSE) - self.reader.used["sense"]), [])
        self.assertEqual(sorted(set(NAMES) - self.reader.used["names"]), [])
        self.assertEqual(sorted(PLAIN_WORDS - self.reader.used["plain"]), [])

    def test_the_check_refuses_an_unknown_term_and_an_unnamed_sense(self):
        reader = Reader()
        self.assertEqual(reader.read("Holonomy of the cap"), ["Holonomy"])
        with self.assertRaises(ValueError):
            reader.read("level")
        self.assertEqual(reader.read("Seat 3 · N_c of cap 7 · Straddlers"), [])


if __name__ == "__main__":
    unittest.main()
