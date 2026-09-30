"""Tests for tools/wiki/lint.py on the real wiki and on synthetic broken pages."""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools" / "wiki"))
import lint  # noqa: E402

PAGE = """---
id: {id}
type: concept
status: {status}
visibility: public
summary: Test page.
claims:
  - {{id: c1, evidence_kind: {kind}, path: {path}, sha256: {sha}}}
---

# Test

{body}
"""


class WikiLintTests(unittest.TestCase):
    def test_repository_wiki_is_clean(self):
        self.assertEqual(lint.lint(ROOT / "docs" / "wiki"), [])

    def make(self, page=None, index=True, log="## [2026-09-29] update | test\n", name="sample"):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        wiki = Path(tmp.name)
        (wiki / "concepts").mkdir()
        (wiki / "SCHEMA.md").write_text("schema\n", encoding="utf-8")
        (wiki / "log.md").write_text("# Log\n\n" + log, encoding="utf-8")
        (wiki / "index.md").write_text(f"- [{name}](concepts/{name}.md)\n" if index else "# Index\n", encoding="utf-8")
        page = page or PAGE.format(id=name, status="draft", kind="source", path="core.py", sha="null", body="Body.")
        (wiki / "concepts" / f"{name}.md").write_text(page, encoding="utf-8")
        return lint.lint(wiki)

    def assertFlags(self, errors, text):
        self.assertTrue(any(text in e for e in errors), errors)

    def test_clean_synthetic_page(self):
        self.assertEqual(self.make(), [])

    def test_detects_problems(self):
        base = dict(id="sample", status="draft", kind="source", path="core.py", sha="null", body="Body.")
        cases = [
            (dict(base, id="other"), "must equal the file name"),
            (dict(base, status="done"), "unknown status"),
            (dict(base, kind="vibes"), "unknown evidence_kind"),
            (dict(base, path="no/such/file.py"), "cites missing file"),
            (dict(base, sha="0" * 64), "digest changed"),
            (dict(base, body="See [x](missing.md)."), "broken link"),
            (dict(base, body="Saved in C:\\Users\\someone\\x"), "Windows user path"),
            (dict(base, body="Path /home/someone/x"), "absolute home path"),
            (dict(base, body="Mail someone@example.com"), "email address"),
            (dict(base, body="Key sk-abcdefghijklmnopqrstuv"), "credential-like token"),
            (dict(base, body="See private:notes/raw.md"), "private reference"),
            (dict(base, body="\u4e2d\u6587"), "non-English"),
        ]
        for fields, expected in cases:
            with self.subTest(expected=expected):
                self.assertFlags(self.make(PAGE.format(**fields)), expected)

    def test_stale_page_may_keep_an_old_digest(self):
        page = PAGE.format(id="sample", status="stale", kind="source", path="core.py", sha="0" * 64, body="Body.")
        self.assertEqual(self.make(page), [])

    def test_orphan_and_log_format(self):
        self.assertFlags(self.make(index=False), "not listed in index.md")
        self.assertFlags(self.make(log="## 2026-09-29 update test\n"), "bad entry heading")

    def test_missing_front_matter(self):
        self.assertFlags(self.make("# No front matter\n"), "missing front matter")


if __name__ == "__main__":
    unittest.main(verbosity=2)
