"""Index of chapter 12 section 15's conformance tests, computed from the repository.

For one phase (default LP2) the tool reads the IDs from the frozen table in
`docs/architecture/1.0/language/architecture/12-phases-and-conformance.md` (section 15, the subsection named for the
phase), scans the test sources for PHS.62 markers and reads the hand-maintained notes file
`docs/progress/1.0/lp2-ct-index-notes.md` (LP3 would read `lp3-ct-index-notes.md`). Each ID gets exactly one state:

- whole: a marker `CT-XXX-NN (whole` names it, or a note says the parts run the whole definition by their union;
- part: a marker `CT-XXX-NN (part: ...; not run: ...)` names it and no marker says whole;
- named: the ID is mentioned without a marker (a test or a document names it, which is no run);
- missing: no mention, or a note says only documentation names it;
- not applicable: a note says so;
- waiting: a note says what it waits for (it overrides named and missing, never a marker).

A marker applies to the IDs before it on the same line, back to the previous marker on that line. It may name the
evidence kind first, one of the table's columns, as in `CT-XXX-NN (synthetic, part: ...)`.
Read only. Standard library only. Usage:

    python tools/language/ct_index.py [--phase LP2] [--summary | --markdown | --json] [--root DIR]

Exit codes: 0 done, 2 the notes file or the table is unusable (an ID outside the phase, an unknown note kind).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CHAPTER = Path("docs/architecture/1.0/language/architecture/12-phases-and-conformance.md")
NOTES_DIR = Path("docs/progress/1.0")  # a phase reads <phase in lower case>-ct-index-notes.md, if the file exists
COLUMNS = ("source", "synthetic", "windows", "performance", "human")
NOTE_KINDS = ("whole by union", "not applicable", "waiting", "missing despite name")
STATES = ("whole", "part", "named", "missing", "not applicable", "waiting")

ID_RE = re.compile(r"CT-[A-Z0-9]+-\d{2}")
MARK_RE = re.compile(r"\((?:(?:" + "|".join(COLUMNS) + r"), )?(whole|parts?)\b")
COMMENT_PREFIX = re.compile(r"^\s*(?:///|//!|//|#)\s?")
MAX_MARKER_TEXT = 1500


class NotesError(Exception):
    """The notes file or the table cannot be used."""


# ---------------------------------------------------------------- the table


def parse_table(text: str, phase: str) -> dict[str, str]:
    """Return {ID: column} for the phase's subsection of section 15, in table order."""
    lines = text.splitlines()
    start = None
    for number, line in enumerate(lines):
        if re.match(r"^### 15\.\d+\. " + re.escape(phase) + r"\s*$", line):
            start = number + 1
            break
    if start is None:
        raise NotesError(f"no section 15 subsection for phase {phase}")
    ids: dict[str, str] = {}
    for line in lines[start:]:
        if line.startswith("#"):
            break
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) != 1 + len(COLUMNS) or cells[0] in ("Component", "---"):
            continue
        for column, cell in zip(COLUMNS, cells[1:]):
            for found in ID_RE.findall(cell):
                if found in ids:
                    raise NotesError(f"{found} occurs twice in {phase}'s table")
                ids[found] = column
    if not ids:
        raise NotesError(f"phase {phase}'s table holds no IDs")
    return ids


# --------------------------------------------------------------- the scanner


def _marker_span(text: str, start: int) -> int:
    """End offset of the parenthesis that opens at `start` (bounded; unbalanced text ends at the bound)."""
    depth = 0
    limit = min(len(text), start + MAX_MARKER_TEXT)
    for pos in range(start, limit):
        char = text[pos]
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return pos + 1
    return limit


def _clean(span: str) -> str:
    parts = [COMMENT_PREFIX.sub("", line) for line in span.splitlines()]
    return " ".join(part.strip() for part in parts if part.strip())


def scan_text(text: str) -> dict[str, dict]:
    """Scan one file's text. Returns {ID: {"whole": bool, "part": bool, "named": bool, "not_run": [str]}}."""
    found: dict[str, dict] = {}

    def entry(test_id: str) -> dict:
        return found.setdefault(test_id, {"whole": False, "part": False, "named": False, "not_run": []})

    markers = []
    for match in MARK_RE.finditer(text):
        end = _marker_span(text, match.start())
        markers.append((match.start(), end, match.group(1), _clean(text[match.start():end])))
    inside = [(start, end) for start, end, _kind, _text in markers]
    offset = 0
    for line in text.splitlines(keepends=True):
        line_end = offset + len(line)
        line_markers = [m for m in markers if offset <= m[0] < line_end]
        for match in ID_RE.finditer(line):
            position = offset + match.start()
            test_id = match.group(0)
            if any(start <= position < end for start, end in inside):
                entry(test_id)["named"] = True
                continue
            following = [m for m in line_markers if m[0] >= offset + match.end()]
            if not following:
                entry(test_id)["named"] = True
                continue
            _start, _end, kind, span = following[0]
            item = entry(test_id)
            if kind == "whole":
                item["whole"] = True
            else:
                item["part"] = True
                lowered = span.lower()
                at = lowered.find("not run:")
                if at >= 0:
                    note = span[at + len("not run:"):].strip()
                    note = (note[:-1] if note.endswith(")") else note).strip()  # the marker's own parenthesis
                    if note:
                        item["not_run"].append(note)
        offset = line_end
    return found


SCAN_GLOBS = (
    "rust/c600-*/**/*.rs",
    "tests/*.py",
    "tools/**/*.py",
    "research/jumbling/differential/candidate/**/*.py",
)


# This tool's own test holds synthetic markers; they are no coverage.
SKIP = {"tests/test_ct_index.py", "tools/language/ct_index.py"}


def scan_files(root: Path) -> dict[str, dict]:
    """Scan the repository. Returns {ID: {"whole": [files], "part": [files], "named": [files], "not_run": [(file, text)]}}."""
    result: dict[str, dict] = {}
    seen: set[Path] = set()
    for pattern in SCAN_GLOBS:
        for path in sorted(root.glob(pattern)):
            if path in seen or not path.is_file():
                continue
            seen.add(path)
            relative = path.relative_to(root)
            if relative.as_posix() in SKIP or "vendor" in relative.parts or "target" in relative.parts:
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except (UnicodeDecodeError, OSError):
                continue
            if "CT-" not in text:
                continue
            name = relative.as_posix()
            for test_id, item in scan_text(text).items():
                slot = result.setdefault(test_id, {"whole": [], "part": [], "named": [], "not_run": []})
                for kind in ("whole", "part", "named"):
                    if item[kind]:
                        slot[kind].append(name)
                slot["not_run"].extend((name, note) for note in item["not_run"])
    return result


# --------------------------------------------------------------------- notes


def parse_notes(text: str, ids: dict[str, str]) -> dict[str, tuple[str, str]]:
    """Read the notes table. Returns {ID: (kind, text)}. Raises NotesError on an unknown ID or kind."""
    notes: dict[str, tuple[str, str]] = {}
    for number, line in enumerate(text.splitlines(), 1):
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|", 2)]
        if len(cells) != 3 or cells[0].lower() == "id" or set(cells[0]) <= set("-: "):
            continue
        test_id, kind, note = cells
        if test_id not in ids:
            raise NotesError(f"notes line {number}: {test_id!r} is not an ID of the phase")
        if kind not in NOTE_KINDS:
            raise NotesError(f"notes line {number}: unknown note kind {kind!r} (known: {', '.join(NOTE_KINDS)})")
        if test_id in notes:
            raise NotesError(f"notes line {number}: {test_id} has two notes")
        notes[test_id] = (kind, note)
    return notes


# --------------------------------------------------------------------- index


def build_index(ids: dict[str, str], scan: dict[str, dict], notes: dict[str, tuple[str, str]]) -> list[dict]:
    rows = []
    for test_id, column in ids.items():
        found = scan.get(test_id)
        files = {"whole": [], "part": [], "named": []}
        not_run: list[str] = []
        if found:
            for kind in files:
                files[kind] = list(found[kind])
            not_run = [f"{note} ({Path(name).name})" for name, note in found["not_run"]]
        if files["whole"]:
            state = "whole"
        elif files["part"]:
            state = "part"
        elif files["named"]:
            state = "named"
        else:
            state = "missing"
        by_union = False
        note_kind, note_text = notes.get(test_id, (None, ""))
        if note_kind == "whole by union":
            if state in ("part", "whole"):
                state, by_union = "whole", state == "part"
            # a union note on an ID without parts is ignored and shows in the warnings
        elif note_kind == "not applicable":
            state = "not applicable"
        elif note_kind == "waiting" and state in ("named", "missing"):
            state = "waiting"
        elif note_kind == "missing despite name" and state == "named":
            state = "missing"
        waiting_for = []
        if note_kind in ("waiting", "not applicable", "missing despite name") and note_text:
            waiting_for.append(note_text)
        if state == "part":
            waiting_for.extend(not_run)
        if state == "whole" and by_union and note_text:
            waiting_for.append(note_text)
        covered = files["whole"] + [f for f in files["part"] if f not in files["whole"]]
        covered = covered or files["named"]
        rows.append(
            {
                "id": test_id,
                "column": column,
                "state": state,
                "by_union": by_union,
                "whole_files": files["whole"],
                "part_files": files["part"],
                "named_files": files["named"],
                "covered_by": covered,
                "note_kind": note_kind,
                "not_run_or_waiting_for": " | ".join(waiting_for),
            }
        )
    return rows


def totals(rows: list[dict]) -> dict[str, int]:
    count = {state: 0 for state in STATES}
    for row in rows:
        count[row["state"]] += 1
    count["by_union"] = sum(1 for row in rows if row["by_union"])
    return count


def summary_line(phase: str, rows: list[dict]) -> str:
    count = totals(rows)
    return (
        f"{phase}: {len(rows)} IDs: {count['whole']} whole, of which {count['by_union']} by union; "
        f"{count['part']} part; {count['named']} named only; {count['missing']} missing; "
        f"{count['not applicable']} not applicable; {count['waiting']} waiting"
    )


def markdown(phase: str, rows: list[dict]) -> str:
    out = [f"# {phase} conformance index", "", summary_line(phase, rows), ""]
    out.append("| ID | column | state | covered by | not run or waiting for |")
    out.append("| --- | --- | --- | --- | --- |")
    for row in rows:
        state = row["state"] + (" (by union)" if row["by_union"] else "")
        files = ", ".join(f"`{name}`" for name in row["covered_by"])
        waiting = row["not_run_or_waiting_for"].replace("|", "/")
        out.append(f"| {row['id']} | {row['column']} | {state} | {files} | {waiting} |")
    return "\n".join(out) + "\n"


def compute(root: Path, phase: str) -> tuple[dict[str, str], dict[str, dict], dict[str, tuple[str, str]], list[dict]]:
    ids = parse_table((root / CHAPTER).read_text(encoding="utf-8"), phase)
    notes_path = root / NOTES_DIR / f"{phase.lower()}-ct-index-notes.md"
    notes = parse_notes(notes_path.read_text(encoding="utf-8"), ids) if notes_path.is_file() else {}
    scan = scan_files(root)
    return ids, scan, notes, build_index(ids, scan, notes)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--phase", default="LP2")
    parser.add_argument("--root", default=str(ROOT))
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--summary", action="store_true")
    mode.add_argument("--markdown", action="store_true")
    mode.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    root = Path(args.root)
    try:
        _ids, _scan, _notes, rows = compute(root, args.phase)
    except NotesError as error:
        print(f"ct_index: {error}", file=sys.stderr)
        return 2
    if args.json:
        print(json.dumps({"phase": args.phase, "totals": totals(rows), "rows": rows}, indent=2))
    elif args.markdown:
        sys.stdout.write(markdown(args.phase, rows))
    else:
        print(summary_line(args.phase, rows))
    return 0


if __name__ == "__main__":
    sys.exit(main())
