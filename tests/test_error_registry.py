"""Chapter 13's error registry, read in place: the registry halves of CT-ERR-01 to CT-ERR-17 (LP2).

Chapter 13 and the chapters it reconciles are frozen (LP1, 15 November 2026), so these tests read them where they are
and never write them. The module parses chapter 13's envelope, category table, registry tables (section 5) and
reconciliation tables (section 6), and the `errors:` lines of chapters 01 to 12, and checks them against chapter 13's
own rules. Each check also runs on a planted counterexample, which shows that it can fail.

This is source evidence about the registry text. No error envelope, session store, command layer, runtime process or
renderer exists in LP2, so each test names the run-time half of its row that it does not run;
`tests/test_language_scans.py` checks the codes the Rust crates emit. Nothing here is Windows, DirectX, input,
long-session or performance evidence.
"""
from __future__ import annotations

import dataclasses
import hashlib
import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ARCHITECTURE = ROOT / "docs/architecture/1.0/language/architecture"
REGISTRY_PATH = ROOT / "docs/architecture/1.0/language/architecture/13-errors.md"
RENDERER_MATRIX_PATH = ROOT / "docs/architecture/1.0/language/renderer-feature-matrix.md"
HANDOFF_PATH = ROOT / "docs/architecture/1.0/language/closure/HANDOFF.md"
MANIFESTS = ROOT / "docs/architecture/1.0/language/closure/manifests"
LP1_EDITION = ROOT / "tests/fixtures/error_registry/lp1-edition.json"

# Frozen-text items that these tests tolerate and report as PHS.52 change items; nothing here edits them. Both are
# empty since chapter 13's amendment at LP3's command-table freeze (section 9; owner, 16 and 20 November 2026) moved
# LB6's detached closure row into ERR.31's table and added KN0.9 to the filter-freshness row's rule source.
KNOWN_DETACHED_ROWS: dict[str, str] = {}
KNOWN_RULE_SOURCE_GAPS: dict[str, str] = {}
# The codes that amendment registers; every other row is the LP1 edition's.
LP3_AMENDMENT = frozenset({
    "KN2:consistency.star_certificate", "KN2:input.named_cap_blocked_or_closed", "KN2:input.menu_order_unpinned",
    "KN4:input.session_exists", "KN4:busy.session_locked", "KN5:input.unknown_row", "KN5:input.argument_form",
    "KN5:input.row_not_built", "KN5:input.overlay_row_conflict", "KN5:input.saved_set_exists",
    "KN5:consistency.unadmitted_journal_action", "KN5:capability.preview_shown",
    "KN5:capability.outside_solver_command",
})

SOURCE = r"[A-Z]{2,3}[0-9]?"
FULL_CODE = re.compile(rf"^({SOURCE}):([a-z_]+)\.([a-z0-9_]+)$")
FULL_WILDCARD = re.compile(rf"^({SOURCE}):([a-z_]+)\.\*$")
SHORT_CODE = re.compile(r"^([a-z_]+)\.([a-z0-9_]+)$")
SHORT_WILDCARD = re.compile(r"^([a-z_]+)\.\*$")
CODE_PARTS = re.compile(r"^([^:]*):([^.]*)\.(.*)$")
COMPONENT_HEADING = re.compile(rf"^## \d+\. ({SOURCE}): ")
SOURCE_HEADING = re.compile(rf"^### ({SOURCE}): ")
BLOCK_ERRORS = re.compile(r"^\s*errors:\s+(.*)$")
INLINE_ERRORS = re.compile(r"`errors: ([^`]+)`")
BACKTICKED = re.compile(r"`([^`\s]+)`")

# ERR.3: lower-case snake case that starts with a letter, with no empty parts; at most 40 characters.
SLUG = re.compile(r"^[a-z][a-z0-9]*(?:_[a-z0-9]+)*$")
SLUG_LIMIT = 40
# ERR.3 and DT-J1 A5: a slug names the failed fact, never a remedy, a selected twist or a correction. Remedies are
# matched as whole slug parts; a twist, letter or correction name is a letter with an optional number, or two letters
# with a number (the shape of menu letters, generators and repair pairs).
REMEDY_PARTS = frozenset({
    "retry", "refresh", "reconnect", "reload", "rerun", "resend", "restart", "choose", "select", "apply", "fix",
    "repair", "correct", "try", "should", "instead", "suggest", "suggested", "recommended", "please",
})
LETTER_NAME = re.compile(r"^(?:[a-z][0-9]*|[a-z]{2}[0-9]+)$")
# A message that tells the solver which twist or correction to make (ERR.13, DT-J1 A5).
SELECTS_A_MOVE = re.compile(r"\b(?:apply|choose|select|use|try|make|perform)\b[^.;]*\b(?:twist|correction|letter)s?\b",
                            re.IGNORECASE)


@dataclasses.dataclass(frozen=True)
class Category:
    name: str
    recoverable: bool
    retry: str
    message_data: str
    raising: str


@dataclasses.dataclass(frozen=True)
class Row:
    """One row of chapter 13's section 5, under the heading of its raising source."""
    source: str
    code: str
    form: str
    meaning: str
    retry: str
    rule_source: str
    used_by: str
    line: int

    @property
    def prefix(self) -> str:
        return split_code(self.code)[0]

    @property
    def category(self) -> str:
        return split_code(self.code)[1]

    @property
    def slug(self) -> str:
        return split_code(self.code)[2]

    @property
    def forms(self) -> list[str]:
        """The forms the Form cell names, primary first: each `;` part starts with one form (ERR.9)."""
        return [part.strip().split(" ", 1)[0] for part in self.form.split(";")]

    @property
    def status(self) -> str:
        """`active`, or the status the Meaning cell opens with (ERR.5, ERR.23 to ERR.27)."""
        if self.meaning.startswith("Withdrawn before first use"):
            return "withdrawn"
        if self.meaning.startswith("Retired"):
            return "retired"
        if self.meaning.startswith("Reserved"):
            return "reserved"
        return "active"


@dataclasses.dataclass(frozen=True)
class Reconciliation:
    """One row of section 6: a spelling, the registry code it became, and what changed."""
    spelling: str
    code: str
    change: str
    line: int
    table: str | None  # "ERR.28", "ERR.31", or None for a row outside both tables


@dataclasses.dataclass(frozen=True)
class Registry:
    envelope_fields: tuple[str, ...]
    forms: tuple[str, ...]
    retries: tuple[str, ...]
    form_table: tuple[str, ...]
    categories: dict[str, Category]
    sources: tuple[str, ...]
    rows: tuple[Row, ...]
    reconciliation: tuple[Reconciliation, ...]

    def row(self, code: str) -> Row | None:
        return next((row for row in self.rows if row.code == code), None)

    def active(self) -> list[Row]:
        return [row for row in self.rows if row.status == "active"]

    def retry(self, row: Row) -> str:
        """The row's retry value: its own, or its category's default when the cell is blank (ERR.23)."""
        return row.retry or self.categories[row.category].retry


@dataclasses.dataclass(frozen=True)
class ErrorsEntry:
    """One `errors:` line of a chapter, with its wrapped continuation lines, and the component section it is in."""
    path: str
    line: int
    component: str | None
    items: tuple[str, ...]


def split_code(code: str) -> tuple[str, str, str]:
    match = CODE_PARTS.match(code)
    return match.groups() if match else ("", "", "")


def unquote(cell: str) -> str:
    if len(cell) >= 2 and cell[0] == cell[-1] == "`" and cell.count("`") == 2:
        return cell[1:-1]
    return cell


def cells(line: str) -> list[str]:
    inner = line.strip()
    if not (inner.startswith("|") and inner.endswith("|")):
        raise ValueError(f"not a table row: {line!r}")
    return [cell.strip() for cell in inner[1:-1].split("|")]


def sections(lines: list[str]) -> dict[str, tuple[int, int]]:
    """Map each `## N.` heading number ("5") to the index range of its lines."""
    starts = [(i, line) for i, line in enumerate(lines) if line.startswith("## ")]
    found = {}
    for n, (i, line) in enumerate(starts):
        end = starts[n + 1][0] if n + 1 < len(starts) else len(lines)
        number = line[3:].split(".", 1)[0]
        found[number] = (i + 1, end)
    return found


def table(lines: list[str], start: int, end: int, header: str) -> list[tuple[int, list[str]]]:
    """The rows of the first table in lines[start:end] whose header row starts with `header`, with line numbers."""
    for i in range(start, end):
        if lines[i].startswith(header):
            width = len(cells(lines[i]))
            if not lines[i + 1].startswith("| ---"):
                raise ValueError(f"line {i + 2}: no delimiter row under {header!r}")
            rows, j = [], i + 2
            while j < end and lines[j].startswith("|"):
                row = cells(lines[j])
                if len(row) != width:
                    raise ValueError(f"line {j + 1}: {len(row)} cells, expected {width}")
                rows.append((j + 1, row))
                j += 1
            return rows
    raise ValueError(f"no table headed {header!r}")


def parse_registry(text: str) -> Registry:
    lines = text.split("\n")
    parts = sections(lines)

    start, end = parts["2"]
    opening = next(i for i in range(start, end) if lines[i].strip() == "Error = {")
    fields, forms, retries = [], (), ()
    for i in range(opening + 1, end):
        if lines[i].strip() == "}":
            break
        name, value = lines[i].strip().split(":", 1)
        fields.append(name)
        if name == "form":
            forms = tuple(v.strip() for v in value.split("|"))
        elif name == "retry":
            retries = tuple(v.strip() for v in value.split("|"))
    form_table = tuple(unquote(row[0]) for _, row in table(lines, start, end, "| Form | Contract |"))

    start, end = parts["3"]
    categories = {}
    for _, row in table(lines, start, end, "| Category | Meaning |"):
        recoverable, _, retry = row[2].partition(";")
        name = unquote(row[0])
        categories[name] = Category(name, recoverable.strip() == "yes", unquote(retry.strip()), row[3], row[4])

    start, end = parts["5"]
    sources, rows, current, i = [], [], None, start
    while i < end:
        line = lines[i]
        heading = SOURCE_HEADING.match(line)
        if heading:
            current = heading.group(1)
            sources.append(current)
        elif line.startswith("| Code |"):
            i += 2
            while i < end and lines[i].startswith("|"):
                c = cells(lines[i])
                if len(c) != 6:
                    raise ValueError(f"line {i + 1}: {len(c)} cells, expected 6")
                rows.append(Row(current, unquote(c[0]), c[1], c[2], unquote(c[3]), c[4], c[5], i + 1))
                i += 1
            continue
        elif line.startswith("|"):
            raise ValueError(f"line {i + 1}: a registry row outside a table")
        i += 1

    start, end = parts["6"]
    reconciliation, current = [], None
    i = start
    while i < end:
        line = lines[i]
        if line.startswith("| Spelling first used |"):
            current, i = "ERR.28", i + 2
            continue
        if line.startswith("| Local or earlier spelling |"):
            current, i = "ERR.31", i + 2
            continue
        if line.startswith("|"):
            c = cells(line)
            if len(c) != 3:
                raise ValueError(f"line {i + 1}: {len(c)} cells, expected 3")
            reconciliation.append(Reconciliation(unquote(c[0]), unquote(c[1]), c[2], i + 1, current))
        else:
            current = None
        i += 1

    return Registry(tuple(fields), forms, retries, form_table, categories, tuple(sources), tuple(rows),
                    tuple(reconciliation))


def errors_entries(text: str, path: str) -> list[ErrorsEntry]:
    """The errors lines of one chapter: code-block lines with their wrapped continuations, and inline `errors: ...`."""
    lines = text.split("\n")
    entries, component, i = [], None, 0
    while i < len(lines):
        line = lines[i]
        if line.startswith("## "):
            heading = COMPONENT_HEADING.match(line)
            component = heading.group(1) if heading else None
        block = BLOCK_ERRORS.match(line)
        if block and not line.lstrip().startswith("|"):
            body, first = block.group(1).strip(), i
            while body.endswith(",") and i + 1 < len(lines):
                i += 1
                body += " " + lines[i].strip()
            entries.append(ErrorsEntry(path, first + 1, component, tuple(x.strip() for x in body.split(","))))
        else:
            for inline in INLINE_ERRORS.finditer(line):
                items = tuple(x.strip() for x in inline.group(1).split(","))
                entries.append(ErrorsEntry(path, i + 1, component, items))
        i += 1
    return entries


def chapter_paths() -> list[Path]:
    """Chapters 01 to 12. The overview's errors line is the template that defines the field; 13 is the registry."""
    paths = []
    for number in range(1, 13):
        found = sorted(ARCHITECTURE.glob(f"{number:02d}-*.md"))
        if len(found) != 1:
            raise ValueError(f"chapter {number:02d}: {found}")
        paths.append(found[0])
    return paths


def expand(entry: ErrorsEntry) -> tuple[list[tuple[str, str]], list[tuple[str, str, str]], list[str]]:
    """ERR.4: (concrete codes, wildcards as (item, prefix, category), problems) of one errors line."""
    concrete, wildcards, problems = [], [], []
    where = f"{entry.path}:{entry.line}"
    for item in entry.items:
        full, family = FULL_CODE.match(item), FULL_WILDCARD.match(item)
        short, short_family = SHORT_CODE.match(item), SHORT_WILDCARD.match(item)
        if full:
            concrete.append((item, item))
        elif family:
            wildcards.append((item, family.group(1), family.group(2)))
        elif (short or short_family) and entry.component is None:
            problems.append(f"{where}: short form {item!r} outside a component section (ERR.4)")
        elif short:
            concrete.append((item, f"{entry.component}:{item}"))
        elif short_family:
            wildcards.append((item, entry.component, short_family.group(1)))
        else:
            problems.append(f"{where}: {item!r} is no code or wildcard (ERR.4)")
    return concrete, wildcards, problems


def errors_line_problems(entries: list[ErrorsEntry], registry: Registry) -> list[str]:
    """CT-ERR-04: every concrete code expands to an active row and every wildcard reaches an active row."""
    problems = []
    active = {row.code for row in registry.active()}
    for entry in entries:
        concrete, wildcards, found = expand(entry)
        problems += found
        where = f"{entry.path}:{entry.line}"
        for item, code in concrete:
            row = registry.row(code)
            if row is None:
                problems.append(f"{where}: {item!r} expands to {code}, which has no registry row")
            elif code not in active:
                problems.append(f"{where}: {code} is {row.status} and cannot be returned")
        for item, prefix, category in wildcards:
            if not any(split_code(code)[:2] == (prefix, category) for code in active):
                problems.append(f"{where}: {item!r} ({prefix}:{category}.*) reaches no registered code")
    return problems


def code_form_problems(registry: Registry) -> list[str]:
    """CT-ERR-01: each row's code has the form source:category.slug, with its heading's source and a category."""
    problems = []
    if len(set(registry.sources)) != len(registry.sources):
        problems.append(f"a source heading repeats: {registry.sources}")
    for row in registry.rows:
        prefix, category, _ = split_code(row.code)
        if not FULL_CODE.match(row.code):
            problems.append(f"line {row.line}: {row.code!r} is not source:category.slug (ERR.2)")
        if prefix not in registry.sources:
            problems.append(f"line {row.line}: {row.code} has an unregistered source {prefix!r} (ERR.2, ERR.23)")
        elif prefix != row.source:
            problems.append(f"line {row.line}: {row.code} under the {row.source} heading (ERR.2, ERR.23)")
        if category not in registry.categories:
            problems.append(f"line {row.line}: {row.code} has an unregistered category {category!r} (ERR.14)")
    return problems


def slug_problems(code: str) -> list[str]:
    """CT-ERR-02: ERR.3's slug rules for one code."""
    prefix, _, slug = split_code(code)
    problems = []
    if not SLUG.match(slug):
        problems.append(f"{code}: slug is not lower-case snake case with nonempty parts")
    if len(slug) > SLUG_LIMIT:
        problems.append(f"{code}: slug has {len(slug)} characters, over {SLUG_LIMIT}")
    parts = slug.split("_")
    if prefix.startswith("LB") and prefix.lower() in parts:
        problems.append(f"{code}: slug repeats its library {prefix.lower()!r}")
    for part in parts:
        if part in REMEDY_PARTS:
            problems.append(f"{code}: slug names a remedy ({part!r})")
        elif part and LETTER_NAME.match(part):
            problems.append(f"{code}: slug names a twist, letter or correction ({part!r})")
    return problems


def form_retry_problems(registry: Registry) -> list[str]:
    """CT-ERR-05, registry half: admitted forms and retry values in every row and category."""
    problems = []
    for row in registry.rows:
        for form in row.forms:
            if form not in registry.forms:
                problems.append(f"line {row.line}: {row.code} names form {form!r} (ERR.8, ERR.9)")
        if row.retry and row.retry not in registry.retries:
            problems.append(f"line {row.line}: {row.code} has retry {row.retry!r} (ERR.8, ERR.11)")
    for category in registry.categories.values():
        if category.retry not in registry.retries:
            problems.append(f"category {category.name} defaults to retry {category.retry!r}")
        if not category.recoverable and category.retry != "never":
            problems.append(f"category {category.name} is not recoverable but defaults to {category.retry!r}")
    return problems


def row_field_problems(registry: Registry) -> list[str]:
    """CT-ERR-04: every row has a Rule source and a Used by entry (ERR.23)."""
    return [f"line {row.line}: {row.code} lacks {name}" for row in registry.rows
            for name, value in (("Rule source", row.rule_source), ("Used by", row.used_by)) if not value]


def edition_row(row: Row) -> dict:
    meaning = " ".join(row.meaning.split())
    return {"status": row.status, "form": row.form, "retry": row.retry,
            "meaning_sha256": hashlib.sha256(meaning.encode("utf-8")).hexdigest()}


def compare_editions(previous: dict[str, dict], registry: Registry) -> list[str]:
    """CT-ERR-03: a later edition keeps every code, meaning and retirement of an earlier one (ERR.5, ERR.27)."""
    problems, current = [], {}
    for row in registry.rows:
        if row.code in current:
            problems.append(f"{row.code}: registered twice (lines {current[row.code].line} and {row.line}; ERR.5)")
        current[row.code] = row
    for code, old in sorted(previous.items()):
        row = current.get(code)
        if row is None:
            problems.append(f"{code}: dropped; retirement keeps the row and its meaning (ERR.5)")
            continue
        new = edition_row(row)
        if old["status"] in ("retired", "withdrawn") and new["status"] != old["status"]:
            problems.append(f"{code}: a {old['status']} code is now {new['status']} (ERR.5: never reused)")
        for key in ("meaning_sha256", "form", "retry"):
            if new[key] != old[key]:
                problems.append(f"{code}: {key} changed since the earlier edition (ERR.5)")
    for row in registry.rows:
        if row.status not in ("retired", "withdrawn"):
            continue
        unassigned = "never active; edition unassigned" in row.used_by
        stated = re.search(r"retired in edition [A-Za-z0-9.\-]+", row.used_by)
        if "before first use" in row.meaning and not unassigned:
            problems.append(f"{row.code}: pre-use {row.status} row without its explicit unassigned edition (ERR.5)")
        elif not unassigned and not stated:
            problems.append(f"{row.code}: {row.status} row without its retirement edition (ERR.5)")
    return problems


def reconciliation_problems(registry: Registry) -> list[str]:
    """CT-ERR-12: one row per spelling; each names a registry code except ERR.22's withdrawn spelling."""
    problems, seen = [], {}
    for rec in registry.reconciliation:
        if rec.spelling in seen:
            problems.append(f"{rec.spelling}: reconciled twice (lines {seen[rec.spelling]} and {rec.line})")
        seen[rec.spelling] = rec.line
        if rec.table is None and KNOWN_DETACHED_ROWS.get(rec.spelling) is None:
            problems.append(f"line {rec.line}: {rec.spelling} stands outside both reconciliation tables")
        withdrawn = rec.change.startswith("Withdrawn spelling only, no registry row")
        if withdrawn:
            if registry.row(rec.code) is not None:
                problems.append(f"line {rec.line}: withdrawn {rec.code} has a registry row (ERR.22)")
        elif registry.row(rec.code) is None:
            problems.append(f"line {rec.line}: {rec.spelling} maps to {rec.code}, which has no registry row")
    return problems


def load_registry() -> Registry:
    return parse_registry(REGISTRY_PATH.read_text(encoding="utf-8"))


def load_entries() -> list[ErrorsEntry]:
    entries = []
    for path in chapter_paths():
        entries += errors_entries(path.read_text(encoding="utf-8"), path.relative_to(ROOT).as_posix())
    return entries


def replace_row(registry: Registry, which: str, **changes) -> Registry:
    rows = tuple(dataclasses.replace(row, **changes) if row.code == which else row for row in registry.rows)
    return dataclasses.replace(registry, rows=rows)


def drop_row(registry: Registry, which: str) -> Registry:
    return dataclasses.replace(registry, rows=tuple(row for row in registry.rows if row.code != which))


def add_rows(registry: Registry, *rows: Row) -> Registry:
    return dataclasses.replace(registry, rows=registry.rows + rows)


class RegistryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.registry = load_registry()
        cls.entries = load_entries()

    def row(self, code: str) -> Row:
        row = self.registry.row(code)
        self.assertIsNotNone(row, code)
        return row

    def test_the_parse_covers_the_whole_registry(self):
        """The parse finds chapter 13's sixteen categories, five forms, five retry values and every row (ERR.14)."""
        r = self.registry
        self.assertEqual(len(r.categories), 16)
        self.assertEqual(r.forms, ("diagnostic", "refusal", "stop", "freeze", "status"))
        self.assertEqual(r.retries, ("never", "after_refresh", "after_user_change", "after_reconnect",
                                     "after_user_request"))
        self.assertEqual(r.sources, ("KN1", "KN2", "KN4", "KN5", "KN6", "KN7", "LG1", "LG2", "LG3", "LG4", "LG5",
                                     "LB1", "LB2", "LB4", "LB5", "LB6", "LB7", "EV1", "EV2", "EV3", "EV4", "PS1",
                                     "PS2", "PS3", "EVD"))
        self.assertGreaterEqual(len(r.rows), 150)
        self.assertGreaterEqual(len(r.reconciliation), 115)
        self.assertGreaterEqual(len(self.entries), 40)
        lines = REGISTRY_PATH.read_text(encoding="utf-8").split("\n")
        parts = sections(lines)
        for number, parsed in (("5", r.rows), ("6", r.reconciliation)):
            start, end = parts[number]
            self.assertEqual(sum(1 for line in lines[start:end] if line.startswith("| `")), len(parsed), number)

    def test_ct_err_01_codes_carry_a_registered_source_and_category(self):
        """CT-ERR-01 (part: the registry half; not run: envelope.category and the reported origin of a raised
        envelope, and forwarding at run time, since no V06 envelope exists before LP3's command layer; the codes the
        crates emit are checked in tests/test_language_scans.py): every registry row has the form
        source:category.slug, its prefix is the heading it sits under (one per raising source), and its category is
        one of section 3's sixteen. Controls: an unknown prefix, a misplaced row and an unknown category fail."""
        self.assertEqual(code_form_problems(self.registry), [])
        template = self.row("KN1:input.unknown_model")
        planted = add_rows(self.registry,
                           dataclasses.replace(template, source="KN9", code="KN9:input.unknown_model", line=-1),
                           dataclasses.replace(template, code="KN2:input.misplaced", line=-2),
                           dataclasses.replace(template, code="KN1:bogus.category", line=-3),
                           dataclasses.replace(template, code="KN1:input", line=-4))
        found = code_form_problems(planted)
        for line, expected in ((-1, "unregistered source 'KN9'"), (-2, "under the KN1 heading"),
                               (-3, "unregistered category 'bogus'"), (-4, "is not source:category.slug")):
            self.assertTrue(any(p.startswith(f"line {line}:") and expected in p for p in found), (expected, found))

    def test_ct_err_02_slugs_follow_err_3(self):
        """CT-ERR-02 (whole): every registry code, every reconciled registry code and every code an errors line
        names has a lower-case snake-case slug that starts with a letter, has no empty part and at most 40
        characters, repeats no LB library its source already names, and names no remedy (a fixed vocabulary of
        remedy words) and no twist, letter or correction (a letter with an optional number). Controls: one planted
        code per rule fails."""
        codes = {row.code for row in self.registry.rows}
        codes |= {rec.code for rec in self.registry.reconciliation}
        for entry in self.entries:
            codes |= {code for _, code in expand(entry)[0]}
        self.assertGreaterEqual(len(codes), 150)
        self.assertEqual([p for code in sorted(codes) for p in slug_problems(code)], [])
        planted = {
            "KN1:input.Unknown_model": "snake case", "KN1:input.1model": "snake case",
            "KN1:input.unknown__model": "snake case", "KN1:input.unknown_": "snake case",
            "KN1:input.": "snake case", "KN1:input." + "a" * 41: "characters",
            "LB5:input.lb5_outside_certified_domain": "library", "KN5:stale.retry_after_refresh": "remedy",
            "KN5:input.choose_jumble_off": "remedy", "KN2:input.twist_a3_required": "twist",
            "KN5:input.correction_x_needed": "twist",
        }
        for code, rule in planted.items():
            with self.subTest(code=code):
                self.assertTrue(any(rule in p for p in slug_problems(code)), slug_problems(code))

    def test_ct_err_03_editions_keep_codes_and_retirements(self):
        """CT-ERR-03 (whole): the registry is compared with the pinned LP1 edition
        (tests/fixtures/error_registry/lp1-edition.json, one entry per row: status, form, retry and the digest of
        the meaning). No code is dropped, registered twice, reused after retirement or withdrawal, or changed in
        meaning, form or retry; every retired or withdrawn row keeps its meaning, and a pre-use withdrawal or
        retirement states its edition as explicitly unassigned. The codes added since LP1 are exactly those of chapter
        13's amendment at LP3's command-table freeze (LP3_AMENDMENT). Controls: synthetic next editions that drop a
        retired row, change a meaning, revive a withdrawn code, register a code twice and invent a retirement
        edition for a pre-use withdrawal each fail."""
        lp1 = json.loads(LP1_EDITION.read_text(encoding="utf-8"))
        self.assertEqual(lp1["edition"], "LP1")
        self.assertEqual({row.code for row in self.registry.rows} - set(lp1["rows"]), LP3_AMENDMENT)
        self.assertEqual(len(lp1["rows"]) + len(LP3_AMENDMENT), len(self.registry.rows))
        self.assertEqual(compare_editions(lp1["rows"], self.registry), [])
        retired = [row for row in self.registry.rows if row.status in ("retired", "withdrawn")]
        self.assertEqual(sorted(row.code for row in retired), [
            "KN7:budget.not_declared", "KN7:budget.over_ceiling", "LG2:type.chart_subset_violation",
            "LG3:effect.flow_refused"])

        r = self.registry
        dropped = dataclasses.replace(r, rows=tuple(row for row in r.rows if row.code != "KN7:budget.not_declared"))
        changed = replace_row(r, "KN5:stale.token_used", meaning="A token was used.")
        revived = replace_row(r, "LG3:effect.flow_refused", meaning="Flow refused by labels.")
        twice = add_rows(r, self.row("KN5:stale.token_used"))
        invented = replace_row(r, "LG2:type.chart_subset_violation", used_by="10 former chart admission; "
                               "retired in edition LP1")
        for name, edition, expected in (("dropped", dropped, "dropped"), ("changed", changed, "meaning_sha256"),
                                        ("revived", revived, "now active"), ("twice", twice, "registered twice"),
                                        ("invented", invented, "explicit unassigned edition")):
            with self.subTest(edition=name):
                found = compare_editions(lp1["rows"], edition)
                self.assertTrue(any(expected in p for p in found), found)

    def test_ct_err_04_errors_lines_expand_to_registered_codes(self):
        """CT-ERR-04 (whole): the errors lines of chapters 01 to 12, block lines with their wrapped continuations
        and inline `errors: ...` spans, expand under ERR.4 with the prefix of the component section they sit in.
        Every concrete code has an active row, every wildcard, qualified or not, reaches an active code, every row
        has Rule source and Used by, and a forwarded code keeps its full prefix (a short form expands to its own
        component and so fails). Controls: a planted KN5 section whose wrapped line hides an empty wildcard, a
        forwarded code without its prefix, an unknown source and a short form outside any component section fail,
        and so does a row without Rule source or Used by."""
        self.assertEqual(errors_line_problems(self.entries, self.registry), [])
        self.assertEqual(row_field_problems(self.registry), [])
        wrapped = [e for e in self.entries if "unknown_outcome.*" in e.items and e.component == "KN5"]
        self.assertEqual(len(wrapped), 1, "chapter 01's commit errors line continues on a second line")
        self.assertTrue(any(e.component == "KN4" and "input.*" in e.items for e in self.entries))
        files = {e.path for e in self.entries}
        self.assertEqual(len(files), 9, files)

        planted = "\n".join([
            "## 6. KN5: command layer", "", "  ```text", "  operation x()",
            "    errors:  input.*, KN2:protection.*, syntax.invalid_encoding,",
            "             KN4:consistency.*, KN9:input.unknown_model", "  ```", "",
            "## 9. Notes", "", "Prose with `errors: stale.*`.", ""])
        found = errors_line_problems(errors_entries(planted, "planted.md"), self.registry)
        self.assertEqual(len(found), 4, found)
        for expected in ("KN5:syntax.invalid_encoding", "KN4:consistency.*", "KN9:input.unknown_model",
                         "outside a component section"):
            self.assertTrue(any(expected in p for p in found), (expected, found))
        blank = replace_row(self.registry, "KN1:input.unknown_model", rule_source="", used_by="")
        self.assertEqual(len(row_field_problems(blank)), 2)

    def test_ct_err_05_forms_retries_and_the_encoding_row(self):
        """CT-ERR-05 (part: the registry half; not run: V06 fields on a raised envelope (no envelope before LP3's
        command layer), EV1.10's range service (LP4), the renderer's contextual forms at run time (LP3) and
        activation of a draft (EV1.11, LP4); the LG1 encoding half runs in rust/c600-syntax/tests/errors.rs): the
        envelope keeps V06's fields plus form and span, every row names only admitted forms and retry values, the
        encoding row is a span-less diagnostic under ERR.10, renderer loss keeps its contextual forms, and the
        ERR.12 and RM.30 example ranges agree with a reference UTF-8, UTF-16 and scalar conversion written here.
        Controls: an unknown form, an unknown retry value and a wrong example range fail."""
        r = self.registry
        self.assertEqual(r.envelope_fields, ("code", "category", "recoverable", "retry", "context", "summary",
                                             "diagnostic", "form", "span"))
        self.assertEqual(r.form_table, r.forms)
        self.assertEqual(form_retry_problems(r), [])
        planted = replace_row(replace_row(r, "KN1:input.unknown_model", form="warning"),
                              "KN1:input.level_not_held", retry="after_a_while")
        self.assertEqual(len(form_retry_problems(planted)), 2)

        encoding = self.row("LG1:syntax.invalid_encoding")
        self.assertEqual((encoding.forms, encoding.status), (["diagnostic"], "active"))
        self.assertIn("span absent", encoding.meaning)
        self.assertIn("ERR.10", encoding.rule_source)
        renderer = self.row("EV3:busy.renderer_unavailable")
        self.assertEqual(renderer.form, "refusal before acceptance; stop after acceptance before installation")
        self.assertIn("Before acceptance return refused(Error); accepted preparation returns failed(Error), with "
                      "error(Error) for its job. Installed publications keep completed(view_published)",
                      renderer.meaning)

        # ERR.12's EV1.10 example: a, U+1F600, U+00E9, LF, z.
        source = "a\U0001F600é\nz"
        self.assertEqual(convert(source, (1, 5)), (1, 3))
        self.assertEqual(convert(source, (9, 9)), (6, 6))
        self.assertIsNone(convert(source, (2, 2)))
        self.assertIsNone(convert(source, (2, 2), unit="utf16"))
        self.assertNotEqual(convert(source, (1, 5)), (1, 4), "control: a wrong example range fails")
        # RM.30's example, applied to the EV3 rows by CT-ERR-12.
        rm30 = RENDERER_MATRIX_PATH.read_text(encoding="utf-8")
        for text in ("a\U0001F600b", "a\U0001F600c"):
            self.assertIn(hashlib.sha256(text.encode("utf-8")).hexdigest(), rm30)
        self.assertEqual(convert("a\U0001F600b", (5, 6)), (3, 4))
        self.assertEqual(convert("a\U0001F600b", (5, 6), target="scalar"), (2, 3))
        self.assertEqual(convert("a\U0001F600b", (5, 5)), (3, 3))
        self.assertIsNone(convert("a\U0001F600b", (2, 5)))

    def test_ct_err_06_codes_not_text_decide_handling(self):
        """CT-ERR-06 (part: the registry half; not run: changing and translating summaries and diagnostics of a
        raised envelope, which the command layer carries as `code` and `detail` only, with no translation seam; the
        crates' handling is scanned in tests/test_language_scans.py): ERR.6 binds tests and handling to the code, every row's code
        is unique so no handling needs its meaning text to tell two failures apart, and every reconciled spelling
        maps to one code. Control: two rows with one code fail."""
        text = REGISTRY_PATH.read_text(encoding="utf-8")
        self.assertIn("Tests and handling assert `code`, never `summary` or `diagnostic`.", " ".join(text.split()))
        codes = [row.code for row in self.registry.rows]
        self.assertEqual(len(codes), len(set(codes)))
        twice = add_rows(self.registry, dataclasses.replace(self.row("KN5:stale.token_used"), meaning="other"))
        self.assertNotEqual(len({row.code for row in twice.rows}), len(twice.rows))

    def test_ct_err_07_only_the_kernel_freezes(self):
        """CT-ERR-07 (part: the registry half; not run: replay, cache, persistence, bundle and runtime-fault
        fixtures against a session, which need the session store (M6) and the runtime process (LP3)): every
        consistency row comes from KN1 to KN7 with form freeze, and the five non-kernel disagreements that drafts
        first spelled as consistency (replay, cache, persistence digest, display bundle, runtime fault) now map to
        codes that neither use consistency nor freeze. Controls: a planted EV2 consistency row and a refusal-form
        KN consistency row fail."""
        self.assertEqual(consistency_problems(self.registry), [])
        self.assertGreaterEqual(len([r for r in self.registry.active() if r.category == "consistency"]), 2)
        moved = {"consistency.replay_mismatch", "consistency.cache_entry_mismatch", "consistency.persistence_digest",
                 "consistency.display_bundle_mismatch", "consistency.runtime_failed"}
        for rec in self.registry.reconciliation:
            if rec.spelling in moved:
                row = self.row(rec.code)
                self.assertNotEqual(row.category, "consistency", rec.spelling)
                self.assertNotIn("freeze", row.forms, rec.spelling)
                moved.remove(rec.spelling)
        self.assertEqual(moved, set())
        template = self.row("KN5:consistency.state_hash_mismatch")
        planted = add_rows(self.registry, dataclasses.replace(template, source="EV2", code="EV2:consistency.replay"),
                           dataclasses.replace(template, code="KN5:consistency.other", form="refusal"))
        self.assertEqual(len(consistency_problems(planted)), 2)

    def test_ct_err_08_budget_codes_need_a_declared_allowance(self):
        """CT-ERR-08 (part: the registry half; not run: the fixed-ceiling 100/101 publication, RM.37's straddler
        payload and the 100/101 output-byte publication, which need the renderer (LP3) and the command layer's
        cancelled(budget) (LP3); the meter's stops are CT-KN7 and CT-DET tests): every active budget row is a KN7
        stop on an allowance the job declared; a missing declaration, a host/profile ceiling and a wall-clock stop
        use input and resource_limit codes; the retired budget spellings stay inactive; the fixed display ceiling is
        EV3:resource_limit.display_plan_ceiling (admit equality, refuse excess, stop after acceptance), never the
        reserved KN7:budget.display_plan_limit; the output allowance cuts off with cancelled(budget); physical
        exhaustion is failed(KN7:resource_limit.host_exhausted). Controls: planted budget rows outside KN7, as a
        refusal, or without a declared allowance fail."""
        self.assertEqual(budget_problems(self.registry), [])
        r = self.registry
        self.assertEqual(self.row("KN7:input.budget_not_declared").category, "input")
        for code in ("KN7:resource_limit.budget_ceiling", "KN7:resource_limit.safety_stop",
                     "KN7:resource_limit.host_exhausted", "EV3:resource_limit.display_plan_ceiling"):
            self.assertEqual((self.row(code).category, self.row(code).status), ("resource_limit", "active"), code)
        for code in ("KN7:budget.not_declared", "KN7:budget.over_ceiling"):
            self.assertEqual(self.row(code).status, "retired")
        ceiling = self.row("EV3:resource_limit.display_plan_ceiling")
        self.assertEqual(ceiling.forms, ["refusal", "stop"])
        self.assertIn("Admit equality; refuse excess before acceptance.", ceiling.meaning)
        self.assertIn("Context names field, requested count and ceiling.", ceiling.meaning)
        plan = self.row("KN7:budget.display_plan_limit")
        self.assertEqual(plan.status, "reserved")
        self.assertIn("never reports a fixed renderer ceiling", plan.meaning)
        output = self.row("KN7:budget.output")
        self.assertIn("cut_off", output.meaning)
        self.assertIn("cancelled(budget)", output.meaning)
        host = self.row("KN7:resource_limit.host_exhausted")
        self.assertIn("failed(Error)", host.meaning)
        self.assertIn("No deterministic cut-off claim", host.meaning)
        template = self.row("KN7:budget.steps")
        planted = add_rows(r, dataclasses.replace(template, source="EV3", code="EV3:budget.display_plan"),
                           dataclasses.replace(template, code="KN7:budget.refused", form="refusal"),
                           dataclasses.replace(template, code="KN7:budget.ceiling", meaning="Host ceiling reached."))
        self.assertEqual(len(budget_problems(planted)), 3)

    def test_ct_err_09_commit_failures_keep_their_codes(self):
        """CT-ERR-09 (part: the registry half; the commit-step fixtures and the unchanged state on refusal run in
        c600-command/tests/layer.rs and kn5.rs): the six commit-step failures (authenticator,
        token reuse, replaced preview, bound content, moved revision, equal-revision head hash) have six distinct
        active codes with their own forms, retries and commit steps (ERR.29). Control: a merged pair fails."""
        self.assertEqual(commit_problems(self.registry, COMMIT_STEPS), [])
        self.assertEqual(len({(form, retry) for form, retry, _ in COMMIT_STEPS.values()}), 3)
        merged = drop_row(self.registry, "KN5:stale.token_mismatch")
        self.assertTrue(any("no active row" in p for p in commit_problems(merged, COMMIT_STEPS)))
        retried = replace_row(self.registry, "KN5:stale.token_used", retry="after_reconnect")
        self.assertTrue(commit_problems(retried, COMMIT_STEPS))

    def test_ct_err_10_unknown_outcomes_never_retry(self):
        """CT-ERR-10 (part: the registry half; read-only recovery after an unknown commit fate runs in
        c600-command/tests/layer.rs; not run: an unknown import publication fate, which has no producer yet): the
        unknown_outcome category is not recoverable
        and defaults to never; its three rows (commit fate and import publication freeze, the further request during
        recovery refuses) keep that default, so no retry follows from a missing receipt. Control: a planted
        unknown_outcome row with its own retry value fails."""
        self.assertEqual(unknown_outcome_problems(self.registry), [])
        planted = replace_row(self.registry, "KN5:unknown_outcome.commit_fate", retry="after_refresh")
        self.assertTrue(unknown_outcome_problems(planted))

    def test_ct_err_12_reconciliation_and_closure_additions(self):
        """CT-ERR-12 (part: the registry half; not run: RM.30's pick and navigation outcomes at run time, which need
        the display (LP3) and EV1's source services (LP4)): every spelling is reconciled once into a registry code,
        and the withdrawn outside-domain spelling has no registry row, errors line or rename; merges into one code
        are equal-meaning merges or forwards, and the distinct facts named in ERR.20, ERR.28 and ERR.29 keep
        distinct codes; H15.16's 23 closure additions (the manifests' new codes) are active rows citing their
        clauses, with ERR.31 rows for their component-local spellings, the six final KN origins and filter
        freshness, and both JI.41 forwards into existing EV3 rows; branch-path, void-exit and geometry failures are
        input facts that name their context and no frame; the EV3 pick row precedes the source-link row as RM.10
        and RM.30 require; and every full code the chapters name in backticks is registered. Controls: a duplicated
        spelling, a rename of the withdrawn spelling, a merge of distinct facts and an unregistered chapter code
        fail."""
        r = self.registry
        self.assertEqual(reconciliation_problems(r), [])
        detached = [rec for rec in r.reconciliation if rec.table is None]
        self.assertEqual({rec.spelling for rec in detached}, set(KNOWN_DETACHED_ROWS))

        withdrawn = [rec for rec in r.reconciliation if rec.code == "LB5:input.outside_certified_domain"]
        self.assertEqual([rec.spelling for rec in withdrawn], ["input.lb5_outside_certified_domain"])
        self.assertIn("This is not a rename to an active code.", withdrawn[0].change)
        self.assertIsNone(r.row("LB5:input.outside_certified_domain"))
        expanded = {code for entry in self.entries for _, code in expand(entry)[0]}
        self.assertNotIn("LB5:input.outside_certified_domain", expanded)

        self.assertEqual(merge_problems(r), [])
        self.assertEqual(distinct_problems(r), [])

        closure = closure_codes()
        self.assertEqual(len(closure), 23)
        self.assertEqual(closure_problems(r, closure), [])
        handoff = handoff_finals()
        self.assertEqual(len(handoff), 8)
        local = {rec.spelling: rec for rec in r.reconciliation if rec.table in ("ERR.31", None)}
        for draft, final in handoff.items():
            spelling = draft.split(":", 1)[1]
            self.assertEqual(local[spelling].code, final, draft)
            if final.startswith("KN"):
                self.assertIn("H15.16", self.row(final).rule_source, final)
                self.assertIn(final, closure)
        self.assertEqual(sorted(c for c in handoff.values() if c.startswith("KN")), sorted(SIX_KN_ORIGINS))
        self.assertIn("KN5:stale.filter_preview", closure)
        for spelling, code in (("input.projection_chain", "EV3:input.display_unsupported"),
                               ("input.render_payload", "EV3:input.display_bundle_mismatch")):
            self.assertEqual((local[spelling].code, self.row(code).status), (code, "active"))
            self.assertIn("JI.41 forwards the existing", local[spelling].change)
        for code in ("KN4:input.branch_path_unproved", "KN5:input.exit_record_voided",
                     "KN2:input.exposed_geometry_unavailable"):
            row = self.row(code)
            self.assertEqual(row.category, "input", code)
            self.assertIn("Context names", row.meaning, code)
            self.assertNotIn("frame", row.meaning.lower(), code)
        self.assertEqual([row.code for row in r.active() if row.category == "unverified_reference"],
                         ["KN2:unverified_reference.frame_ambiguous"])

        link, pick = self.row("EV3:stale.source_link"), self.row("EV3:stale.display_pick")
        self.assertTrue(link.meaning.startswith("After pick-binding checks succeed"))
        self.assertIn("Binding failures use EV3:stale.display_pick first, even if source also differs.", link.meaning)
        self.assertIn("RM.10", link.rule_source)
        self.assertEqual((pick.category, pick.forms), ("stale", ["refusal"]))

        mentions = chapter_code_problems(r)
        self.assertEqual(mentions, [])

        rec = r.reconciliation
        twice = dataclasses.replace(r, reconciliation=rec + (rec[0],))
        renamed = dataclasses.replace(r, reconciliation=tuple(
            dataclasses.replace(x, code="LB5:evidence.reduction_scope_mismatch", change="Prefix only.")
            if x.spelling == "input.lb5_outside_certified_domain" else x for x in rec))
        self.assertTrue(any("reconciled twice" in p for p in reconciliation_problems(twice)))
        self.assertTrue(withdrawn_problems(renamed))
        merged = replace_row(r, "LG4:evidence.reproducibility_mismatch", code="EV2:evidence.replay_mismatch")
        self.assertTrue(distinct_problems(merged))
        joined = dataclasses.replace(r, reconciliation=rec + (Reconciliation(
            "consistency.bundle_digest", "EV3:input.display_bundle_mismatch", "Digest disagreement.", -1, "ERR.28"),))
        self.assertTrue(merge_problems(joined))
        self.assertTrue(chapter_code_problems(r, extra="Text naming `KN5:stale.never_registered` here."))

    def test_ct_err_13_category_content_rules(self):
        """CT-ERR-13 (part: the registry half; not run: the typed context, retained drafts and message data of
        raised envelopes, which exist only from LP3's command layer and LP4's editor services; the crates' message
        texts are scanned in tests/test_language_scans.py): each of the sixteen categories states its required
        message data, including retained drafts, typed protected set, level and coverage, limits with unit and
        actual value, budget coverage, and that a failed check alone refutes nothing; the incomplete protection
        assurance issues no safe verdict; and no row's meaning tells the solver which twist, letter or correction to
        make. Controls: a category without its data and a meaning that selects a twist fail."""
        self.assertEqual(content_problems(self.registry), [])
        self.assertIn("issue no safe verdict", self.row("LB2:protection.incomplete_coverage").meaning)
        self.assertIn("No correction is selected.", self.row("KN5:input.jumble_off_nu_nonzero").meaning)
        self.assertIn("without changing another claim", self.row("EV3:evidence.chart_check_failed").meaning)
        categories = dict(self.registry.categories)
        categories["evidence"] = dataclasses.replace(categories["evidence"], message_data="Failed support.")
        planted = replace_row(dataclasses.replace(self.registry, categories=categories),
                              "KN5:input.jumble_off_nu_nonzero", meaning="Apply the twist A3 correction first.")
        self.assertEqual(len(content_problems(planted)), 2)

    def test_ct_err_14_runtime_loss_is_no_freeze(self):
        """CT-ERR-14 (part: the registry half; killing the runtime runs in c600-runtime/tests/faults.rs against a fake
        session; not run: the same under the command layer and the session store): the runtime-loss code is
        an active resource_limit stop, citing KN7.13 and ONE.70, that preserves session and pending preview, and
        Round 3's consistency spelling for the same fault was moved to it. Control: a consistency version fails."""
        self.assertEqual(runtime_loss_problems(self.registry), [])
        planted = replace_row(self.registry, "KN7:resource_limit.runtime_lost", form="freeze")
        self.assertTrue(runtime_loss_problems(planted))

    def test_ct_err_16_rotation_requests_are_kn2_codes(self):
        """CT-ERR-16 (part: the registry half; the three refusals at run time with their typed failed fact (ERR.8) are
        run in rust/c600-jumble/tests/kn3.rs and rust/c600-command/tests/twist_apply.rs (a draft form of the fact:
        `request::Fact`); that no crate emits a KN3 code is checked in tests/test_language_scans.py): the
        exact-request format, menu-binding and free-search-cap failures keep their three KN2 codes with refusal form,
        after_user_change retry and the KN3.7/KN3.8 rule sources, and KN3 has no registry section, row, reconciled
        code or errors-line code (ERR.23). Controls: a planted KN3 row and a retry change fail."""
        self.assertEqual(rotation_problems(self.registry, self.entries), [])
        template = self.row("KN2:input.rotation_request_malformed")
        planted = add_rows(self.registry, dataclasses.replace(template, source="KN3", code="KN3:input.not_exact"))
        self.assertTrue(rotation_problems(planted, self.entries))
        changed = replace_row(self.registry, "KN2:input.menu_binding_mismatch", retry="")
        self.assertTrue(rotation_problems(changed, self.entries))

    def test_ct_err_17_stage_and_view_rows(self):
        """CT-ERR-17 (part: the registry half; not run: stage admission, compilation, quarantine, raster bounds and
        view open/close at run time, which need the display and renderer (LP3); that no crate emits the withdrawn
        code is checked in tests/test_language_scans.py): stage-rule diagnostics have
        LG2:type.stage_rule_violation; the withdrawn chart-subset row keeps its meaning, cannot be named by an errors
        line and has no assigned edition; stage compile failures and quarantine refuse before acceptance and stop
        after it; raster-bound excess is a status report naming that no user-stage fragment work ran; view
        open/close refusals keep their two input codes and the input category's retry. Controls: a changed form and
        an errors line naming the withdrawn code fail."""
        self.assertEqual(stage_problems(self.registry), [])
        planted = replace_row(self.registry, "EV3:input.stage_quarantined", form="refusal")
        self.assertTrue(stage_problems(planted))
        named = errors_entries("## 9. Notes\n\n    errors:  LG2:type.chart_subset_violation\n", "planted.md")
        self.assertTrue(errors_line_problems(named, self.registry))


COMMIT_STEPS = {
    # code: (form, retry, commit step in the rule source)
    "KN5:auth.token_invalid": ("refusal", "after_reconnect", "ONE commit step 3"),
    "KN5:stale.token_used": ("refusal", "after_refresh", "ONE commit step 3"),
    "KN5:stale.preview_replaced": ("refusal", "after_refresh", "ONE commit step 5"),
    "KN5:stale.token_mismatch": ("refusal", "after_refresh", "ONE commit step 5"),
    "KN5:stale.revision_changed": ("refusal", "after_refresh", "ONE commit step 6"),
    "KN5:consistency.state_hash_mismatch": ("freeze", "never", "ONE commit step 7"),
}
SIX_KN_ORIGINS = ("KN1:input.unsupported_dimension", "KN2:input.named_object_invalid",
                  "KN5:input.scramble_policy_unset", "KN4:input.branch_path_unproved", "KN5:input.exit_record_voided",
                  "KN2:input.exposed_geometry_unavailable")
# Distinct facts that reconciliation must not merge (ERR.20, ERR.28, ERR.29).
DISTINCT = (
    ("EV2:evidence.replay_mismatch", "LG4:evidence.reproducibility_mismatch"),
    ("KN5:auth.token_invalid", "KN5:stale.token_used"),
    ("KN5:stale.revision_changed", "KN5:consistency.state_hash_mismatch"),
    ("LG2:type.exact_required", "LG2:type.exactness_violation"),
    ("PS2:input.persistence_version", "PS2:input.edition_unsupported"),
    ("PS1:resource_limit.persistence_size", "PS1:resource_limit.record_too_large"),
    ("LG3:capability.not_granted", "LG3:capability.grant_mismatch"),
    ("KN1:input.identity_mismatch", "LG2:type.identity_mismatch"),
    ("LG2:type.verdict_required", "LG2:type.verdict_as_boolean"),
    ("EV2:input.replay_dependency_missing", "LG4:input.repro_input_missing"),
    ("PS1:input.digest_mismatch", "KN5:consistency.state_hash_mismatch"),
    ("KN2:stale.revision_gone", "KN2:stale.epoch_changed"),
    ("LG1:syntax.edition_unsupported", "PS2:input.edition_unsupported"),
)
REQUIRED_MESSAGE_DATA = {
    "input": ("retained draft",), "unverified_reference": ("disambiguating frame",),
    "protection": ("level", "unchecked coverage"), "stale": ("preserved inputs",),
    "cancelled": ("coverage", "no claim of emptiness"), "resource_limit": ("Limit, unit and actual value",),
    "consistency": ("retained draft",), "unknown_outcome": ("retained draft", "no assertion that the operation failed"),
    "auth": ("renew",), "busy": ("No blind replay",), "syntax": ("retained valid version",), "type": ("levels",),
    "effect": ("call path",), "capability": ("effective profile", "failed scope"),
    "budget": ("coverage", "No unexamined domain is reported empty or negative"),
    "evidence": ("A failed check alone is no refutation",),
}


def convert(text: str, span: tuple[int, int], unit: str = "utf8", target: str = "utf16") -> tuple[int, int] | None:
    """A reference conversion of a half-open range between UTF-8 bytes, UTF-16 units and scalars; None if an endpoint
    is not on a scalar boundary. It checks the frozen example values only; EV1.10's service is LP4's."""
    def boundaries(kind: str) -> list[int]:
        offsets, total = [0], 0
        for ch in text:
            total += {"utf8": len(ch.encode("utf-8")), "utf16": len(ch.encode("utf-16-le")) // 2, "scalar": 1}[kind]
            offsets.append(total)
        return offsets
    source, goal = boundaries(unit), boundaries(target)
    if span[0] not in source or span[1] not in source or span[0] > span[1]:
        return None
    return goal[source.index(span[0])], goal[source.index(span[1])]


def consistency_problems(registry: Registry) -> list[str]:
    """ERR.18: only KN1 to KN7 raise consistency, and every such row has form freeze."""
    problems = []
    for row in registry.rows:
        if row.category != "consistency":
            continue
        if not re.fullmatch(r"KN[1-7]", row.prefix):
            problems.append(f"line {row.line}: {row.code} raises consistency outside KN1 to KN7")
        if row.forms != ["freeze"]:
            problems.append(f"line {row.line}: {row.code} has form {row.form!r}, not freeze")
    return problems


def budget_problems(registry: Registry) -> list[str]:
    """ERR.16: an active budget row is a KN7 stop on an allowance the job declared."""
    problems = []
    for row in registry.active():
        if row.category != "budget":
            continue
        if row.prefix != "KN7":
            problems.append(f"line {row.line}: {row.code} is a budget code outside KN7")
        if row.forms != ["stop"]:
            problems.append(f"line {row.line}: {row.code} has form {row.form!r}, not stop")
        if not row.meaning.startswith("Declared"):
            problems.append(f"line {row.line}: {row.code} names no allowance the job declared")
    return problems


def commit_problems(registry: Registry, steps: dict[str, tuple[str, str, str]]) -> list[str]:
    problems = []
    if len(steps) != 6:
        problems.append(f"{len(steps)} commit-step codes, not six distinct ones")
    for code, (form, retry, step) in steps.items():
        row = registry.row(code)
        if row is None or row.status != "active":
            problems.append(f"{code}: no active row")
            continue
        if (row.form, registry.retry(row)) != (form, retry):
            problems.append(f"{code}: form {row.form!r} and retry {registry.retry(row)!r}, not {form!r}, {retry!r}")
        if step not in row.rule_source:
            problems.append(f"{code}: rule source {row.rule_source!r} names no {step}")
    return problems


def unknown_outcome_problems(registry: Registry) -> list[str]:
    category = registry.categories["unknown_outcome"]
    problems = [] if (category.recoverable, category.retry) == (False, "never") else ["unknown_outcome retries"]
    expected = {"KN5:unknown_outcome.commit_fate": ["freeze"], "KN5:unknown_outcome.read_only_recovery": ["refusal"],
                "PS3:unknown_outcome.import_publication": ["freeze"]}
    found = {row.code: row for row in registry.active() if row.category == "unknown_outcome"}
    if set(found) != set(expected):
        problems.append(f"unknown_outcome rows {sorted(found)}")
    for code, row in found.items():
        if row.forms != expected.get(code) or row.retry:
            problems.append(f"line {row.line}: {code} has form {row.form!r} and retry {row.retry!r}")
    return problems


def equal_meaning(rec: Reconciliation) -> bool:
    return (rec.change.startswith(("Same", "Prefix")) or "merge the synonym" in rec.change
            or "forwards the existing" in rec.change)


def merge_problems(registry: Registry) -> list[str]:
    """ERR.28: when several spellings meet in one code, all but the code's own origin join it as an equal-meaning
    merge, a prefix-only change or a forward of the existing code."""
    problems, groups = [], {}
    for rec in registry.reconciliation:
        groups.setdefault(rec.code, []).append(rec)
    for code, recs in groups.items():
        others = [rec for rec in recs if not equal_meaning(rec)]
        if len(recs) > 1 and len(others) > 1:
            names = ", ".join(f"{rec.spelling} (line {rec.line})" for rec in others)
            problems.append(f"{code}: {names} join it without an equal-meaning reason")
    return problems


def distinct_problems(registry: Registry) -> list[str]:
    problems = []
    for first, second in DISTINCT:
        a, b = registry.row(first), registry.row(second)
        if a is None or b is None:
            problems.append(f"{first} or {second} has no registry row")
        elif first == second or a is b or a.code == b.code:
            problems.append(f"{first} and {second} are merged")
    codes = [row.code for row in registry.rows]
    for first, second in DISTINCT:
        if codes.count(first) > 1 or codes.count(second) > 1:
            problems.append(f"{first} or {second} is registered twice")
    return problems


def withdrawn_problems(registry: Registry) -> list[str]:
    """ERR.22: the outside-domain spelling is withdrawn with no registry row, never renamed into an active code."""
    recs = [rec for rec in registry.reconciliation if rec.spelling == "input.lb5_outside_certified_domain"]
    if len(recs) != 1:
        return [f"{len(recs)} rows for the withdrawn outside-domain spelling"]
    rec = recs[0]
    problems = []
    if rec.code != "LB5:input.outside_certified_domain" or registry.row(rec.code) is not None:
        problems.append(f"line {rec.line}: the withdrawn spelling maps to {rec.code}")
    if not rec.change.startswith("Withdrawn spelling only, no registry row"):
        problems.append(f"line {rec.line}: the withdrawn spelling is reconciled as {rec.change!r}")
    return problems


def closure_codes() -> dict[str, str]:
    """The closure manifests' new error codes and their originating clauses (H15.16's 23)."""
    codes = {}
    for path in sorted(MANIFESTS.glob("*.json")):
        for entry in json.loads(path.read_text(encoding="utf-8")).get("new_error_codes", []):
            if entry["code"] in codes:
                raise ValueError(f"{path.name}: {entry['code']} is proposed twice")
            codes[entry["code"]] = entry["clause"]
    return codes


def handoff_finals() -> dict[str, str]:
    """H15.16's draft JI codes and their final registry codes."""
    lines = HANDOFF_PATH.read_text(encoding="utf-8").split("\n")
    start = lines.index("### H15.16 Error code proposals (RS3-11)")
    end = next(i for i in range(start + 1, len(lines)) if lines[i].startswith("### "))
    return {unquote(row[0]): unquote(row[1].removeprefix("existing ").strip())
            for _, row in table(lines, start, end, "| Draft code |")}


def closure_problems(registry: Registry, closure: dict[str, str]) -> list[str]:
    problems = []
    local = {}
    for rec in registry.reconciliation:
        if rec.table == "ERR.31" or KNOWN_DETACHED_ROWS.get(rec.spelling) == "ERR.31":
            local.setdefault(rec.code, []).append(rec)
    for code, clause in sorted(closure.items()):
        row = registry.row(code)
        if row is None or row.status != "active":
            problems.append(f"{code}: no active row for a closure addition")
            continue
        if clause not in row.rule_source and KNOWN_RULE_SOURCE_GAPS.get(code) != clause:
            problems.append(f"line {row.line}: {code} does not cite its originating clause {clause}")
        if len(local.get(code, [])) != 1:
            problems.append(f"{code}: {len(local.get(code, []))} ERR.31 rows for its local spelling")
    return problems + withdrawn_problems(registry)


def chapter_code_problems(registry: Registry, extra: str = "") -> list[str]:
    """Every full code chapters 01 to 12 name in backticks is registered, or is ERR.22's withdrawn spelling. A token
    counts as a code when its prefix is a registry source or its category a registry category; other identifiers of
    the same shape, such as chapter 06's stored collision names (`EF:collision.*`), are no error codes."""
    problems = []
    texts = [(path.relative_to(ROOT).as_posix(), path.read_text(encoding="utf-8")) for path in chapter_paths()]
    texts.append(("planted.md", extra))
    for name, text in texts:
        for number, line in enumerate(text.split("\n"), 1):
            for token in BACKTICKED.findall(line):
                prefix, category, _ = split_code(token)
                if not FULL_CODE.match(token) or (prefix not in registry.sources
                                                  and category not in registry.categories):
                    continue
                if registry.row(token) is None:
                    if token == "LB5:input.outside_certified_domain" and "withdr" in line.lower():
                        continue
                    problems.append(f"{name}:{number}: {token} is not registered")
    return problems


def content_problems(registry: Registry) -> list[str]:
    problems = []
    for name, needles in REQUIRED_MESSAGE_DATA.items():
        data = registry.categories[name].message_data
        for needle in needles:
            if needle not in data:
                problems.append(f"category {name}: message data lacks {needle!r}")
    if set(registry.categories) != set(REQUIRED_MESSAGE_DATA):
        problems.append(f"categories {sorted(registry.categories)}")
    for row in registry.rows:
        if SELECTS_A_MOVE.search(row.meaning):
            problems.append(f"line {row.line}: {row.code}'s meaning selects a twist or correction")
    return problems


def runtime_loss_problems(registry: Registry) -> list[str]:
    row = registry.row("KN7:resource_limit.runtime_lost")
    if row is None:
        return ["no runtime-loss row"]
    problems = []
    if (row.status, row.category, row.forms) != ("active", "resource_limit", ["stop"]):
        problems.append(f"line {row.line}: {row.status} {row.category} {row.form}")
    for clause in ("KN7.13", "ONE.70"):
        if clause not in row.rule_source:
            problems.append(f"line {row.line}: no {clause}")
    if "session and pending engine preview preserved" not in row.meaning:
        problems.append(f"line {row.line}: the meaning keeps no session and preview")
    moved = [rec for rec in registry.reconciliation if rec.spelling == "consistency.runtime_failed"]
    if [rec.code for rec in moved] != [row.code]:
        problems.append("Round 3's consistency.runtime_failed is not moved to the runtime-loss code")
    return problems


def rotation_problems(registry: Registry, entries: list[ErrorsEntry]) -> list[str]:
    """CT-ERR-16: KN2's three rotation-request codes, and no KN3 raising family anywhere (ERR.23)."""
    problems = []
    expected = {"KN2:input.rotation_request_malformed": ("input", ("KN3.7",)),
                "KN2:input.menu_binding_mismatch": ("input", ("KN3.7", "KN3.8")),
                "KN2:resource_limit.rotation_search_bound": ("resource_limit", ("KN3.7",))}
    for code, (category, clauses) in expected.items():
        row = registry.row(code)
        if row is None:
            problems.append(f"{code}: no row")
            continue
        if (row.status, row.category, row.forms, row.retry) != ("active", category, ["refusal"], "after_user_change"):
            problems.append(f"line {row.line}: {code} is {row.status}, {row.form}, retry {row.retry!r}")
        problems += [f"line {row.line}: {code} does not cite {c}" for c in clauses if c not in row.rule_source]
    named = {row.code for row in registry.rows} | {rec.code for rec in registry.reconciliation}
    named |= {code for entry in entries for _, code in expand(entry)[0]}
    named |= {f"{prefix}:{category}.*" for entry in entries for _, prefix, category in expand(entry)[1]}
    if "KN3" in registry.sources:
        problems.append("KN3 has a registry section")
    problems += [f"{code} is a KN3 code" for code in sorted(named) if code.startswith("KN3:")]
    return problems


def stage_problems(registry: Registry) -> list[str]:
    problems = []

    def check(code, status, forms, retry=None, needles=()):
        row = registry.row(code)
        if row is None:
            problems.append(f"{code}: no row")
            return
        if (row.status, row.forms) != (status, forms):
            problems.append(f"line {row.line}: {code} is {row.status} with form {row.form!r}")
        if retry is not None and registry.retry(row) != retry:
            problems.append(f"line {row.line}: {code} retries {registry.retry(row)!r}")
        problems.extend(f"line {row.line}: {code} lacks {n!r}" for n in needles if n not in row.meaning + row.used_by)

    check("LG2:type.stage_rule_violation", "active", ["diagnostic"], "after_user_change")
    check("LG2:type.chart_subset_violation", "withdrawn", ["diagnostic"], None,
          ("this code keeps its old meaning", "never active; edition unassigned"))
    for code in ("EV3:input.stage_compile_failed", "EV3:input.stage_quarantined"):
        check(code, "active", ["refusal", "stop"], "after_user_change")
        row = registry.row(code)
        if row and row.form != "refusal before acceptance; stop after acceptance":
            problems.append(f"line {row.line}: {code} has form {row.form!r}")
    check("EV3:resource_limit.stage_work_exceeded", "active", ["status"], "after_user_change",
          ("No user-stage fragment work of that frame ran.",))
    for code in ("EV3:input.view_id_in_use", "EV3:input.view_not_closable"):
        check(code, "active", ["refusal"], registry.categories["input"].retry)
    return problems


if __name__ == "__main__":
    unittest.main()
