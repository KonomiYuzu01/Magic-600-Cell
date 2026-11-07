"""MC-07 controls over copied current texts. Mutates memory only; writes nothing.

These controls test reference rejection, not corpus-wide closure, content review,
implementation, or freeze readiness. Other packets may still move definitions.
"""
import sys

sys.dont_write_bytecode = True

import unittest
from unittest.mock import patch

import check_closure
import check_p7


class DefinitionChecks(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.corpus = check_closure.texts()
        cls.owner = check_closure.LANGUAGE / "architecture" / "05-persistence.md"

    def assert_reference(self, corpus, name, accepted):
        # Reconciliation imports the shared checker; exercise that copy too.
        for entry, checker in (("closure", check_closure), ("reconciliation", check_p7.closure)):
            with self.subTest(entry=entry, reference=name), patch.object(checker, "texts", return_value=corpus), \
                    patch.object(checker, "errors", []):
                ids = checker.defined_ids()
                checker.check_vector("MC-07", "definition", {
                    "given": "A vector cites an ID whose defining marker may be missing.",
                    "expected": "Accept only while the definition exists, regardless of remaining citations.",
                    "clause": name,
                }, ids)
                self.assertEqual(name in ids, accepted, f"{entry}: definition membership for {name}")
                if accepted:
                    self.assertEqual(checker.errors, [])
                else:
                    self.assertEqual(checker.errors, [f"MC-07: vector definition cites unknown clause {name!r}"])

    def test_current_definitions_are_accepted(self):
        for name in ("PS1.2", "CT-PS1-01"):
            self.assert_reference(self.corpus, name, True)

    def test_removed_clause_marker_is_not_replaced_by_a_bold_citation(self):
        marker = "**PS1.2 [D]**"
        original = dict(self.corpus)[self.owner]
        self.assertEqual(original.count(marker), 1)
        corpus = [(path, text.replace(marker, "PS1.2 [D]") if path == self.owner else text)
                  for path, text in self.corpus]
        self.assertTrue(any("**PS1.2 retained JSON**" in text for path, text in corpus if path != self.owner))
        self.assert_reference(corpus, "PS1.2", False)

    def test_requirement_cited_only_outside_its_defining_table_is_rejected(self):
        original = dict(self.corpus)[self.owner]
        rows = [line for line in original.splitlines(keepends=True) if line.startswith("| CT-PS1-01 |")]
        self.assertEqual(len(rows), 1)
        corpus = [(path, text.replace(rows[0], "") if path == self.owner else text)
                  for path, text in self.corpus]
        self.assertTrue(any("CT-PS1-01" in text for path, text in corpus if path != self.owner))
        self.assert_reference(corpus, "CT-PS1-01", False)


if __name__ == "__main__":
    unittest.main()
