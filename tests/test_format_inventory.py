"""CT-PS2-06 (whole): the format inventory of the migration format freeze (chapter 05, PS1.12 and PS2.10).

The test reads the frozen texts in place and writes nothing: the format packet's FP.4 identifier table, chapter 05's
PS1.12 format catalogue, PS2.10's data-core list and PS3.3's path B targets, the packet's reference reader
(`check_vectors.py`, FP.10 dispatch) and the identifiers the Rust crates declare for syntax and IR. It checks that
- the source extension is `.6cs` in PS1.12, in FP.4's note and for the parser's pinned fixtures;
- every FP.4 row wholly or partly in the freeze carries no `-draft` suffix, every row marked No keeps it
  (`c600-source-draft` among them), and every identifier keeps the `c600-` prefix;
- the reference reader dispatches exactly the frozen identifiers plus playback's draft, refuses the former draft
  identifier of every frozen row and has no reader for the reserved draft identifiers;
- the freeze covers every data-core format of PS1.12 and PS2.10, the facts envelope and every path B target of PS3.3;
- it excludes source syntax, the IR, module manifests and lockfiles, recordings and the proof-obligation export.
Negative controls mutate copies of the texts and expect each rule to report.

Source evidence only: no Windows/DirectX, input, long-session or performance claim.
"""
from __future__ import annotations

import importlib.util
import re
import sys
import unittest
from pathlib import Path

sys.dont_write_bytecode = True  # importing the frozen reference reader must not write a cache next to it
ROOT = Path(__file__).resolve().parents[1]
PACKET = ROOT / "docs/architecture/1.0/language/format-packet"
PERSISTENCE = ROOT / "docs/architecture/1.0/language/architecture/05-persistence.md"
IR_LIB = ROOT / "rust/c600-ir/src/lib.rs"
SYNTAX_LIB = ROOT / "rust/c600-syntax/src/lib.rs"
SYNTAX_FIXTURES = ROOT / "rust/c600-syntax/tests/fixtures"

FP4_HEADER = "| Record | Exact `format` | Payload contract | In the freeze |"
CATALOGUE_HEADER = "| Format | Identifier | Version | Writer | Reader | Data core under LQ10 |"
TARGET_HEADER = "| Target | Read from staged 0.4 data | Becomes | Checks before publication |"

# PS2.10: the language-neutral data core the freeze fixes, phrase by phrase, as the FP.4 records that hold it.
DATA_CORE = {
    "words/recipes": {"c600-word", "c600-recipe"},
    "macro/protection sets": {"c600-macro", "c600-macro-set", "c600-protection-set"},
    "filters/preferences": {"c600-filter", "c600-preferences"},
    "checkpoints with replay paths": {"c600-checkpoint", "c600-replay-path"},
    "bindings/key maps": {"c600-binding", "c600-key-map"},
    "the provenance field of words/macros/workspace records": {"c600-word", "c600-macro", "c600-workspace"},
    "the envelope's identity, immutable scope and facts payload": {"c600-evidence"},
}
# PS3.3: every path B target, as the FP.4 records it becomes. The carrier is FP.24's manifest and session record.
PATH_B_TARGETS = {
    "Recipes": {"c600-recipe"},
    "Macro sets": {"c600-macro", "c600-macro-set"},
    "Protected orbits": {"c600-protection-set"},
    "Filters": {"c600-filter"},
    "Preferences": {"c600-preferences", "c600-workspace"},
    "Checkpoints": {"c600-checkpoint", "c600-replay-path"},
    "0.4 twist keys": {"c600-binding", "c600-key-map"},
    "0.4 command keys": {"c600-binding", "c600-key-map"},
}
PATH_B_CARRIER = {"C600-MIGRATE", "c600-session-record"}
# PS2.10 and FP.4: what the freeze excludes, by the draft identifier FP.4 reserves for it.
EXCLUDED = {
    "source syntax": "c600-source-draft",
    "module semantics": "c600-module-draft",
    "module lockfile": "c600-lock-draft",
    "recordings": "c600-recording-draft",
    "proof-obligation export": "c600-proof-obligation-draft",
}
# FP.37: the one draft identifier with an active reference reader; FP.4's last three rows reserve the others only.
DISPATCHED_DRAFT = "c600-playback-draft"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def table(text: str, header: str) -> list[list[str]]:
    """The rows of the Markdown table whose header line is `header`, each as a list of stripped cells."""
    lines = text.splitlines()
    starts = [index for index, line in enumerate(lines) if line.strip() == header]
    if len(starts) != 1:
        raise AssertionError(f"expected one table headed {header!r}, found {len(starts)}")
    rows = []
    for line in lines[starts[0] + 2:]:
        if not line.startswith("| "):
            break
        rows.append([cell.strip() for cell in line.strip().strip("|").split("|")])
    return rows


def ticked(cell: str) -> list[str]:
    return re.findall(r"`([^`]+)`", cell)


def clause(text: str, label: str) -> str:
    """One numbered clause (`- **PS2.10 [B]** ...`) up to the next list item or heading, whitespace collapsed."""
    match = re.search(r"^- \*\*" + re.escape(label) + r" \[[A-Z]\]\*\*(.*?)(?=^- \*\*|^#|\Z)", text, re.M | re.S)
    if match is None:
        raise AssertionError(f"clause {label} not found")
    return " ".join(match.group(1).split())


def fp4_split(fp_text: str) -> tuple[list[tuple[str, list[str], str]], set[str], set[str]]:
    """FP.4's rows as (record, identifiers, in the freeze), the identifiers in the freeze and those outside it."""
    rows = [(record, ticked(formats), freeze) for record, formats, _contract, freeze in table(fp_text, FP4_HEADER)]
    frozen = {name for _record, names, freeze in rows if freeze != "No" for name in names}
    drafts = {name for _record, names, freeze in rows if freeze == "No" for name in names}
    return rows, frozen, drafts


def inventory_problems(fp_text: str, ps_text: str) -> list[str]:
    """Every way the texts break CT-PS2-06's inventory rules; empty when they hold."""
    problems = []
    rows, frozen, drafts = fp4_split(fp_text)
    for record, names, freeze in rows:
        if not names:
            problems.append(f"FP.4 row {record!r} names no identifier")
        for name in names:
            if not name.lower().startswith("c600-"):
                problems.append(f"FP.4 {name}: lost the c600- prefix")
            if freeze == "No" and not name.endswith("-draft"):
                problems.append(f"FP.4 {name}: a row outside the freeze keeps its -draft suffix")
            if freeze != "No" and "-draft" in name:
                problems.append(f"FP.4 {name}: a row in the freeze carries no -draft suffix")
            if re.search(r"(?:^|-)(?:ir|syntax)(?:-|$)", name.lower()):
                problems.append(f"FP.4 {name}: syntax and IR have no FP.4 record")
    # PS1.12 and FP.4's note: the source extension.
    if "The source extension is `.6cs`" not in ps_text:
        problems.append("PS1.12 no longer says the source extension is .6cs")
    if "`.6cs`" not in fp_text:
        problems.append("FP.4's note no longer names the extension .6cs")
    source = [row for row in table(ps_text, CATALOGUE_HEADER) if row[0] == "Source"]
    if len(source) != 1 or "extension `.6cs`" not in source[0][1] or "`c600-source-draft`" not in source[0][1]:
        problems.append("PS1.12's Source row no longer gives descriptor c600-source-draft and extension .6cs")
    # PS1.12: a c600 format in the data core is frozen in FP.4; one outside it keeps its draft identifier.
    for row in table(ps_text, CATALOGUE_HEADER):
        fmt, identifier, core = row[0], row[1], row[5]
        for name in ticked(identifier):
            if not name.startswith("c600-"):
                continue
            if name not in frozen | drafts:
                problems.append(f"PS1.12 {fmt}: {name} is not an FP.4 identifier")
            elif core.startswith("No") and name in frozen:
                problems.append(f"PS1.12 {fmt}: {name} is outside the data core but frozen in FP.4")
            elif not core.startswith("No") and name in drafts:
                problems.append(f"PS1.12 {fmt}: {name} is in the data core but outside the FP.4 freeze")
    # PS2.10: the data core and the facts envelope are frozen.
    core_text = clause(ps_text, "PS2.10")
    for phrase, names in DATA_CORE.items():
        if phrase not in core_text:
            problems.append(f"PS2.10 no longer lists {phrase!r}; the test's DATA_CORE needs review")
        for name in sorted(names - frozen):
            problems.append(f"PS2.10 {phrase}: {name} is not frozen in FP.4")
    # PS3.3: every path B target and the carrier are frozen.
    targets = {row[0] for row in table(ps_text, TARGET_HEADER)}
    if targets != set(PATH_B_TARGETS):
        problems.append(f"PS3.3 targets {sorted(targets)} differ from the mapped targets {sorted(PATH_B_TARGETS)}")
    for target, names in PATH_B_TARGETS.items():
        for name in sorted(names - frozen):
            problems.append(f"PS3.3 target {target}: {name} is not frozen in FP.4")
    for name in sorted(PATH_B_CARRIER - frozen):
        problems.append(f"path B carrier {name} is not frozen in FP.4")
    # PS2.10: source syntax, module semantics, recordings and the proof-obligation export stay outside.
    for what, name in EXCLUDED.items():
        if name not in drafts:
            problems.append(f"{what}: {name} must be an FP.4 row outside the freeze")
    return problems


def load_reader():
    """The format packet's reference reader (FP.10), imported from its frozen place without writing anything."""
    spec = importlib.util.spec_from_file_location("format_packet_reference_reader", PACKET / "check_vectors.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def rust_constant(path: Path, name: str) -> str:
    match = re.search(r'pub const ' + name + r': &str = "([^"]+)";', read(path))
    if match is None:
        raise AssertionError(f"{path.relative_to(ROOT)}: constant {name} not found")
    return match.group(1)


FP_TEXT = read(PACKET / "README.md")
PS_TEXT = read(PERSISTENCE)


class FormatInventoryTests(unittest.TestCase):
    def test_ct_ps2_06_frozen_texts_keep_the_inventory_rules(self) -> None:
        """CT-PS2-06 (whole): FP.4, PS1.12, PS2.10 and PS3.3 agree on suffixes, prefix, core and exclusions."""
        self.assertEqual(inventory_problems(FP_TEXT, PS_TEXT), [])
        _rows, frozen, drafts = fp4_split(FP_TEXT)
        self.assertIn("c600-source-draft", drafts)
        self.assertEqual(len(drafts), 8, sorted(drafts))
        self.assertFalse(frozen & drafts)

    def test_ct_ps2_06_reference_reader_dispatches_only_frozen_identifiers(self) -> None:
        """CT-PS2-06 (whole): FP.10's reader knows the frozen identifiers and refuses every former draft spelling."""
        reader = load_reader()
        _rows, frozen, drafts = fp4_split(FP_TEXT)
        records = {name for name in frozen if name.startswith("c600-")}
        self.assertEqual({f"c600-{kind}" for kind in reader.FROZEN_KINDS}, records)
        self.assertEqual(set(reader.KNOWN_FORMATS), records | {DISPATCHED_DRAFT})
        for name in sorted(records):
            reader.supported({"format": name, "version": 1})
            with self.assertRaises(reader.Refusal, msg=name + "-draft"):
                reader.supported({"format": name + "-draft", "version": 1})
        for name in sorted(drafts - {DISPATCHED_DRAFT}):
            with self.assertRaises(reader.Refusal, msg=name):
                reader.supported({"format": name, "version": 1})

    def test_ct_ps2_06_code_keeps_syntax_and_ir_outside_the_freeze(self) -> None:
        """CT-PS2-06 (whole): parser fixtures use .6cs; the syntax edition and IR identifier stay drafts outside FP.4."""
        fixtures = sorted(path.name for path in SYNTAX_FIXTURES.iterdir() if path.is_file())
        self.assertTrue(fixtures)
        self.assertEqual([name for name in fixtures if not name.endswith(".6cs")], [])
        edition = rust_constant(SYNTAX_LIB, "EDITION")
        ir_version = rust_constant(IR_LIB, "IR_VERSION")
        self.assertTrue(edition.startswith("6cs-") and "-draft-" in edition, edition)
        self.assertIn("-draft-", ir_version)
        _rows, frozen, drafts = fp4_split(FP_TEXT)
        reader = load_reader()
        for name in (edition, ir_version):
            self.assertNotIn(name, frozen | drafts)
            self.assertNotIn(name, reader.KNOWN_FORMATS)

    def test_ct_ps2_06_negative_controls(self) -> None:
        """CT-PS2-06 (whole): each rule reports when a copy of the texts breaks it."""
        mutations = [
            ("frozen row with -draft", "fp", "| Word | `c600-word` |", "| Word | `c600-word-draft` |",
             "c600-word-draft"),
            ("descriptor without -draft", "fp", "`c600-source-draft`, `c600-module-draft`",
             "`c600-source`, `c600-module-draft`", "c600-source"),
            ("core format outside the freeze", "fp",
             "| Filter, preferences | `c600-filter`, `c600-preferences` | FP.15 | Yes |",
             "| Filter, preferences | `c600-filter`, `c600-preferences` | FP.15 | No |", "c600-filter"),
            ("recording in the freeze", "fp", "`c600-recording-draft`, `c600-run-draft` | PS1.27/PS1.30/PS1.31 | No |",
             "`c600-recording-draft`, `c600-run-draft` | PS1.27/PS1.30/PS1.31 | Yes |", "c600-recording-draft"),
            ("unmapped path B target", "ps", "| Filters |", "| Templates |", "PS3.3 targets"),
            ("extension renamed", "ps", "The source extension is `.6cs`", "The source extension is `.c6s`", ".6cs"),
            ("data-core list shortened", "ps", "checkpoints with replay paths, bindings/key maps",
             "checkpoints with replay paths, bindings", "bindings/key maps"),
        ]
        for label, which, old, new, expected in mutations:
            with self.subTest(label):
                fp_text, ps_text = FP_TEXT, PS_TEXT
                source = fp_text if which == "fp" else ps_text
                self.assertEqual(source.count(old), 1, old)
                if which == "fp":
                    fp_text = fp_text.replace(old, new)
                else:
                    ps_text = ps_text.replace(old, new)
                problems = inventory_problems(fp_text, ps_text)
                self.assertTrue(any(expected in problem for problem in problems), problems)


if __name__ == "__main__":
    unittest.main(verbosity=2)
