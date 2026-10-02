"""Format and coverage checks for the stage 2.1 source inventories.

Usage:
  python docs/progress/1.0/inventory/check_inventory.py <file.json> [...]
  python docs/progress/1.0/inventory/check_inventory.py --coverage

The first form checks the row format shared by the Codex shards and the Claude
inventory: allowed fields, value types, unique ids, notes where the evidence kind
needs them, and that every evidence entry is a repository-relative path with a
line number inside the file. It does not judge completeness.

`--coverage` checks `shards.json`: every tracked file under the owner's source
scope belongs to exactly one shard or is excluded with a reason, and every listed
file exists. Standard library only.
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
SHARDS = HERE / "shards.json"
TOP = {"shard", "functions", "internal_only", "unclear"}
FIELDS = {"id", "name", "behaviour", "entry_points", "evidence", "depends_on",
          "evidence_kind", "reads", "writes", "engine_call"}
OPTIONAL = {"notes"}
AUX_FIELDS = {"name", "evidence"}
AUX_OPTIONAL = {"reason"}
DATA = {"state", "protection", "journal", "checkpoints", "workspace", "preferences", "files", "none"}
KINDS = {"source", "source+test", "inferred"}
SHARD_RE = re.compile(r"^s[1-9]$")
EVIDENCE_RE = re.compile(r"^([A-Za-z0-9_.-][A-Za-z0-9_./-]*):(\d+)$")
PATH_RE = re.compile(r"[A-Za-z0-9_][A-Za-z0-9_./-]*\.(?:py|cs|js|mjs)\b")
_lines: dict[str, int | None] = {}


def line_count(rel: str) -> int | None:
    """Lines of a repository file, or None for a missing, absolute or escaping path."""
    if rel not in _lines:
        parts = Path(rel).parts
        path = ROOT / rel
        ok = not rel.startswith("/") and ".." not in parts and path.is_file()
        _lines[rel] = len(path.read_bytes().splitlines()) if ok else None
    return _lines[rel]


def is_test(rel: str) -> bool:
    """An existing repository file under a `tests` directory or named `test*`."""
    parts = Path(rel).parts
    return line_count(rel) is not None and ("tests" in parts[:-1] or parts[-1].startswith("test"))


def evidence_errors(where: str, evidence) -> list[str]:
    if not isinstance(evidence, list) or not evidence:
        return [f"{where}: evidence must be a non-empty list"]
    errors = []
    for ev in evidence:
        m = EVIDENCE_RE.match(ev) if isinstance(ev, str) else None
        if not m:
            errors.append(f"{where}: evidence '{ev}' is not a repository-relative path:line")
            continue
        count = line_count(m.group(1))
        if count is None:
            errors.append(f"{where}: evidence file '{m.group(1)}' does not exist in the repository")
        elif not 1 <= int(m.group(2)) <= count:
            errors.append(f"{where}: line {m.group(2)} outside '{m.group(1)}' (1..{count})")
    return errors


def row_errors(where: str, row: dict, shard: str, seen: set) -> list[str]:
    errors = []
    missing, extra = FIELDS - set(row), set(row) - FIELDS - OPTIONAL
    if missing:
        errors.append(f"{where}: missing {sorted(missing)}")
    if extra:
        errors.append(f"{where}: unknown fields {sorted(extra)}")
    if missing:
        return errors
    rid = row["id"]
    if not isinstance(rid, str) or not re.fullmatch(rf"S{shard[1:]}-\d+", rid) or rid in seen:
        errors.append(f"{where}: id must be a unique 'S{shard[1:]}-<n>'")
    if isinstance(rid, str):
        seen.add(rid)
    where = f"{where} ({rid})"
    for key in ("name", "behaviour", "evidence_kind"):
        if not isinstance(row[key], str) or not row[key].strip():
            errors.append(f"{where}: '{key}' must be a non-empty string")
    if "notes" in row and (not isinstance(row["notes"], str) or not row["notes"].strip()):
        errors.append(f"{where}: 'notes' must be a non-empty string when present")
    kind, notes = row["evidence_kind"], row.get("notes") if isinstance(row.get("notes"), str) else ""
    if not isinstance(kind, str) or kind not in KINDS:
        errors.append(f"{where}: evidence_kind must be one of {sorted(KINDS)}")
    elif kind == "inferred" and not notes.strip():
        errors.append(f"{where}: an inferred row explains its inference in 'notes'")
    elif kind == "source+test" and not any(is_test(p) for p in PATH_RE.findall(notes)):
        errors.append(f"{where}: a source+test row names an existing test file in 'notes'")
    for key in ("entry_points", "depends_on"):
        if not isinstance(row[key], list) or not all(isinstance(x, str) and x.strip() for x in row[key]):
            errors.append(f"{where}: '{key}' must be a list of non-empty strings")
    if isinstance(row["entry_points"], list) and not row["entry_points"]:
        errors.append(f"{where}: a user-visible function needs at least one entry point")
    for key in ("reads", "writes"):
        value = row[key]
        if (not isinstance(value, list) or not value or not all(isinstance(x, str) for x in value)
                or not set(value) <= DATA):
            errors.append(f"{where}: '{key}' must be a non-empty list drawn from {sorted(DATA)}")
    if row["engine_call"] is not None and not (isinstance(row["engine_call"], str) and row["engine_call"].strip()):
        errors.append(f"{where}: engine_call must be a non-empty string or null")
    return errors + evidence_errors(where, row["evidence"])


def check(path: Path) -> list[str]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        return [f"{path}: cannot read JSON ({exc})"]
    if not isinstance(data, dict):
        return [f"{path}: top level must be an object"]
    errors = []
    if set(data) != TOP:
        errors.append(f"{path}: top-level keys must be exactly {sorted(TOP)}")
    shard = data.get("shard")
    if not isinstance(shard, str) or not SHARD_RE.match(shard):
        errors.append(f"{path}: 'shard' must be 's1' to 's9'")
    for key in ("functions", "internal_only", "unclear"):
        if not isinstance(data.get(key), list):
            errors.append(f"{path}: '{key}' must be a list")
    if errors:
        return errors
    if not data["functions"]:
        errors.append(f"{path}: 'functions' is empty")
    seen: set[str] = set()
    for n, row in enumerate(data["functions"]):
        where = f"{path}: functions[{n}]"
        if not isinstance(row, dict):
            errors.append(f"{where}: not an object")
            continue
        errors += row_errors(where, row, shard, seen)
    for key in ("internal_only", "unclear"):
        for n, item in enumerate(data[key]):
            where = f"{path}: {key}[{n}]"
            if not isinstance(item, dict):
                errors.append(f"{where}: not an object")
                continue
            missing, extra = AUX_FIELDS - set(item), set(item) - AUX_FIELDS - AUX_OPTIONAL
            if missing or extra:
                errors.append(f"{where}: fields must be name, evidence and optional reason")
                continue
            if not isinstance(item["name"], str) or not item["name"].strip():
                errors.append(f"{where}: 'name' must be a non-empty string")
            if "reason" in item and (not isinstance(item["reason"], str) or not item["reason"].strip()):
                errors.append(f"{where}: 'reason' must be a non-empty string when present")
            errors += evidence_errors(where, item["evidence"])
    return errors


def tracked(scope: list[str]) -> set[str]:
    out = subprocess.run(["git", "-C", str(ROOT), "ls-files", "-z", "--", *scope],
                         capture_output=True, check=True).stdout.decode("utf-8")
    return {p for p in out.split("\0") if p}


def coverage() -> list[str]:
    try:
        spec = json.loads(SHARDS.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        return [f"{SHARDS.name}: cannot read JSON ({exc})"]
    errors, owner = [], {}
    for shard, entry in sorted(spec.get("shards", {}).items()):
        if not SHARD_RE.match(shard) or not isinstance(entry, dict) or not isinstance(entry.get("files"), list):
            errors.append(f"{SHARDS.name}: shard {shard!r} needs a 'files' list")
            continue
        for rel in entry["files"]:
            if rel in owner:
                errors.append(f"{rel}: in both {owner[rel]} and {shard}")
            owner[rel] = shard
    excluded = spec.get("excluded", {})
    for rel, reason in excluded.items():
        if not isinstance(reason, str) or not reason.strip():
            errors.append(f"{rel}: an excluded file needs a reason")
        if rel in owner:
            errors.append(f"{rel}: both excluded and in {owner[rel]}")
    files = tracked(spec.get("scope", []))
    listed = set(owner) | set(excluded)
    for rel in sorted(files - listed):
        errors.append(f"{rel}: in the source scope but in no shard and not excluded")
    for rel in sorted(listed - files):
        errors.append(f"{rel}: listed but not a tracked file in the source scope")
    if not errors:
        print(f"inventory coverage: ok ({len(owner)} files in {len(spec['shards'])} shards, {len(excluded)} excluded)")
    return errors


def main(argv: list[str]) -> int:
    if argv == ["--coverage"]:
        errors = coverage()
    elif argv and "--coverage" not in argv:
        errors = [e for arg in argv for e in check(Path(arg))]
        if not errors:
            print(f"inventory format: ok ({len(argv)} file(s))")
    else:
        print(__doc__.strip().splitlines()[2].strip())
        return 2
    for e in errors:
        print(e)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
