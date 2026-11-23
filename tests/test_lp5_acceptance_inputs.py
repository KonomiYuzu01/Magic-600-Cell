"""LP5's fourth acceptance item: no new search is counted as acceptance (PHS.31; LP5 plan, section 2).

The plan asks for a scan of every LP5 acceptance command and its inputs against the Git-blob manifest and the plan:
no acceptance input is produced by a search, a strategy or an enlarged admitted domain, and the searches of
CT-LB5-07 and CT-LB5-08, which run on small synthetic fixtures, test the record contract only and put nothing into
research evidence (section 9, reading B). This scan reads the sources of LP5's acceptance tests and checks:

- every repository path under `research/` that they read names files that existed at the research archive's frozen
  commit (the Git-blob manifest's `frozen_commit`) and are unchanged since, so each input is an archived artifact and
  none is a new output;
- LP5's line changes nothing under `research/` (its own outputs are data under `rust/` and `docs/progress/`);
- no acceptance test starts a process on a file under `research/`, so no producer, search or strategy of the archive
  runs;
- the synthetic search tests of CT-LB5-07 and CT-LB5-08 reach no evidence writer and read no archived record.

Everything here is source evidence. It reads Git and files only; it builds nothing, runs no Rust and starts no search.
"""
from __future__ import annotations

import json
import re
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "work/experiments/language-lx/envelope-facts/MANIFEST.json"

# The sources of LP5's acceptance tests (LP5 plan, sections 2 and 5): the archive import, the KN6 checkers, the
# research library's entries, the obligation translator, the research host and the archive driver.
SOURCES = (
    "rust/c600-archive/tests",
    "rust/c600-checkers/tests",
    "rust/c600-research/tests",
    "rust/c600-library/tests/research_*.rs",
    "rust/c600-evidence/tests/obligation.rs",
    "tests/test_lp5_archive.py",
)
SEARCH_TESTS = ("rust/c600-library/tests/research_ct_lb5_07.rs", "rust/c600-library/tests/research_ct_lb5_08.rs")
# What would put a search's output into research evidence: the evidence crate, an admission or envelope writer, and
# the research host and its check job, whose reports alone write the independent-checker value.
EVIDENCE_WRITERS = (
    r"\bc600_evidence\b",
    r"\badmit\w*\s*\(",
    r"\bEnvelope\b",
    r"\bc600_research\b",
    r"\bCheckJob\b",
)
LITERAL = re.compile(r"[\"'](research/[^\"'{}]*)")
# Base folders that a test names only to join a further path to it, never to read the folder: the archive driver's
# `RESEARCH`, the restoration root that also holds stages 1 to 13 and later notes LP5 does not read. A base is no input
# itself; each join `NAME + "..."` is read as the literal it builds.
BASES = {"RESEARCH": "research/jumbling/restoration/"}
JOIN = re.compile(r"\b(" + "|".join(BASES) + r")\s*\+\s*[\"']([^\"'{}]*)")
PROCESS = re.compile(r"Command::new|subprocess\.|os\.system|Popen")
# Literals that name no tracked file are not inputs (a test's deliberately absent path); at most this many.
ABSENT_LIMIT = 3


def git(*args: str) -> str:
    return subprocess.run(["git", "-C", str(ROOT), *args], check=True, capture_output=True, text=True).stdout


def lines(text: str) -> list[str]:
    return [line for line in text.splitlines() if line]


def source_files() -> list[Path]:
    files: list[Path] = []
    for pattern in SOURCES:
        path = ROOT / pattern
        if "*" in pattern:
            files.extend(sorted(path.parent.glob(path.name)))
        elif path.is_dir():
            files.extend(sorted(path.rglob("*.rs")))
        else:
            files.append(path)
    return files


def research_prefixes(files: list[Path]) -> dict[str, list[str]]:
    """Each research path literal (cut at a format placeholder), with the files that name it."""
    found: dict[str, list[str]] = {}
    for file in files:
        text = file.read_bytes().decode("utf-8")
        name = file.relative_to(ROOT).as_posix()
        for match in LITERAL.finditer(text):
            if match.group(1) not in BASES.values():
                found.setdefault(match.group(1), []).append(name)
        for match in JOIN.finditer(text):
            found.setdefault(BASES[match.group(1)] + match.group(2), []).append(name)
    return found


def under(paths: list[str], prefix: str) -> list[str]:
    """The paths a literal names: the file itself, a folder's files, or the files whose names a cut-off literal
    begins (`wj-` before a `{name}` placeholder)."""
    return sorted(path for path in paths if path.startswith(prefix))


class AcceptanceInputs(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.frozen = json.loads(MANIFEST.read_bytes())["frozen_commit"]
        cls.files = source_files()
        cls.prefixes = research_prefixes(cls.files)
        cls.tracked = lines(git("ls-files", "--", "research/"))
        cls.archived = lines(git("ls-tree", "-r", "--name-only", cls.frozen, "--", "research/"))
        cls.changed = lines(git("diff", "--name-only", cls.frozen, "HEAD", "--", "research/"))

    def test_the_scan_reads_every_acceptance_source(self) -> None:
        self.assertGreaterEqual(len(self.files), 30, "the scan found too few LP5 test sources")
        for file in self.files:
            self.assertTrue(file.is_file(), file)
        self.assertGreaterEqual(len(self.prefixes), 10, "the scan found too few research inputs")

    def test_every_research_input_is_an_archived_file_unchanged_since_the_frozen_commit(self) -> None:
        absent = []
        for prefix, users in sorted(self.prefixes.items()):
            with self.subTest(prefix=prefix):
                tracked = under(self.tracked, prefix)
                if not tracked:
                    # A literal that names no tracked file cannot be read as an input: a test's deliberately absent
                    # path (a missing dependency or an ungranted file). It is counted, never taken as an input.
                    absent.append(prefix)
                    continue
                self.assertEqual(under(self.archived, prefix), tracked, f"{prefix} ({users[0]}) is not the archive's")
                self.assertEqual(under(self.changed, prefix), [], f"{prefix} changed after the frozen commit")
        self.assertLessEqual(len(absent), ABSENT_LIMIT, f"research literals that name no file: {absent}")

    def test_lp5_changes_nothing_under_research(self) -> None:
        # The line's own changes, against the default branch it started from. Without that ref the check cannot be
        # made here, which is reported as a skip, never as a pass.
        try:
            base = git("merge-base", "HEAD", "origin/main").strip()
        except subprocess.CalledProcessError:
            self.skipTest("no origin/main to compare with")
        self.assertEqual(lines(git("diff", "--name-only", base, "HEAD", "--", "research/")), [])

    def test_no_acceptance_test_starts_a_process_on_research_files(self) -> None:
        for file in self.files:
            for number, line in enumerate(file.read_bytes().decode("utf-8").splitlines(), 1):
                if PROCESS.search(line) and "research/" in line:
                    self.fail(f"{file.relative_to(ROOT).as_posix()}:{number} starts a process on a research file")

    def test_the_synthetic_searches_put_nothing_into_evidence(self) -> None:
        for name in SEARCH_TESTS:
            text = (ROOT / name).read_bytes().decode("utf-8")
            code = "\n".join(line for line in text.splitlines() if not line.lstrip().startswith("//"))
            for writer in EVIDENCE_WRITERS:
                with self.subTest(test=name, writer=writer):
                    self.assertIsNone(re.search(writer, code), writer)
            self.assertNotIn("research/", code, f"{name} reads no archived record: its fixtures are synthetic")


if __name__ == "__main__":
    unittest.main()
