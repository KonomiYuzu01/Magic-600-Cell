"""Check stage 2.2 disposition files against units.json and flows.json.

  python docs/progress/1.0/dispositions/check_dispositions.py dispositions-s3.json
  python docs/progress/1.0/dispositions/check_dispositions.py --all

One file per shard, named dispositions-s<N>.json. Each unit of the shard gets
exactly one row. A row either decides the whole unit, or, when the unit's
rows serve different purposes, has disposition "split" and a list of parts,
each deciding a subset of the unit's inventory rows. --all also requires all
six files, each shard exactly once, and every unit of units.json.
"""
import argparse
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
DISPOSITIONS = ("keep", "redesign", "delete", "automate")
RETAINED = ("keep", "redesign", "automate")  # the purpose survives in 1.0
ROW_KEYS = {"unit", "disposition", "purpose", "reason", "flows", "same_purpose_as", "requirements"}
SPLIT_KEYS = {"unit", "disposition", "purpose", "reason", "same_purpose_as", "parts"}
PART_KEYS = {"rows", "disposition", "purpose", "reason", "flows", "requirements"}
OPTIONAL_KEYS = {"notes"}
UNIT_ID = re.compile(r"^U[1-6]-\d{3}$")
REQUIREMENT = re.compile(r"^([A-Z]+-\d+):\s*\S.{4,}$")
FILE_NAME = re.compile(r"^dispositions-(s[1-6])\.json$")


def _load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def _sentence(value):
    return isinstance(value, str) and len(value.strip()) >= 10


def decisions(row):
    """The decisions a row makes: (label, decision dict, inventory rows or None for all)."""
    if row.get("disposition") == "split":
        return [(f"{row['unit']}/{i + 1}", part, part.get("rows")) for i, part in enumerate(row.get("parts", []))]
    return [(row["unit"], row, None)]


def _check_decision(where, d, unit, rows, flow_ids, errors):
    if d["disposition"] not in DISPOSITIONS:
        errors.append(f"{where}: disposition must be one of {DISPOSITIONS}")
    for field in ("purpose", "reason"):
        if not _sentence(d[field]):
            errors.append(f"{where}: {field} must be a sentence")
    flows = d["flows"]
    if not isinstance(flows, list) or any(f not in flow_ids for f in flows) or len(set(flows)) != len(flows):
        errors.append(f"{where}: flows must be distinct ids from flows.json")
    elif not flows and d["disposition"] in ("keep", "redesign"):
        errors.append(f"{where}: a kept or redesigned purpose serves at least one flow")
    reqs = d["requirements"]
    if not isinstance(reqs, list) or any(not isinstance(r, str) or not r.strip() for r in reqs):
        errors.append(f"{where}: requirements must be a list of strings")
        return
    named = set()
    for r in reqs:
        m = REQUIREMENT.match(r.strip())
        if m:
            named.add(m.group(1))
        elif re.match(r"^[A-Z]+-\d+\b", r.strip()):
            errors.append(f"{where}: requirement {r!r} must read '<finding id>: <requirement>'")
    if d["disposition"] in RETAINED:
        statuses = {s.split(" ")[0]: s for s in unit.get("screening", [])}
        by_row = unit.get("screening_rows", {})
        needed = {f for f, s in statuses.items()
                  if ("(open)" in s or "(needs verification)" in s)
                  and (rows is None or set(by_row.get(f, [])) & set(rows))}
        if needed - named:
            errors.append(f"{where}: requirements must name open screening findings {sorted(needed - named)}")


def check_file(path, units, flow_ids, shard=None):
    errors = []
    data = _load(path)
    if not isinstance(data, dict) or set(data) != {"shard", "rows"} or not isinstance(data["rows"], list):
        return [f"{path}: top level must be exactly {{shard, rows}} with a list of rows"]
    m = FILE_NAME.match(Path(path).name)
    declared = data["shard"]
    if shard is None and m:
        shard = m.group(1)
    if shard is not None and declared != shard:
        return [f"{path}: declares shard {declared!r}, expected {shard!r}"]
    expected = {u["id"] for u in units.values() if u["shard"] == declared}
    if not expected:
        return [f"{path}: unknown shard {declared!r}"]
    seen = set()
    for i, row in enumerate(data["rows"]):
        where = f"{path}: row {i}"
        if not isinstance(row, dict):
            errors.append(f"{where}: not an object")
            continue
        split = row.get("disposition") == "split"
        required = SPLIT_KEYS if split else ROW_KEYS
        keys = set(row)
        if not required <= keys or keys - required - OPTIONAL_KEYS:
            errors.append(f"{where}: fields must be {sorted(required)} plus optional {sorted(OPTIONAL_KEYS)}")
            continue
        uid = row["unit"]
        where = f"{path}: {uid}"
        if uid not in expected:
            errors.append(f"{where}: not a unit of {declared}")
            continue
        if uid in seen:
            errors.append(f"{where}: duplicate")
        seen.add(uid)
        unit = units[uid]
        if "notes" in row and (not isinstance(row["notes"], str) or not row["notes"].strip()):
            errors.append(f"{where}: notes must be a non-empty string when present")
        same = row["same_purpose_as"]
        if not isinstance(same, list) or any(not isinstance(s, str) or not UNIT_ID.match(s) or s not in units or s == uid for s in same):
            errors.append(f"{where}: same_purpose_as must list other existing unit ids")
        if not split:
            _check_decision(where, row, unit, None, flow_ids, errors)
            continue
        for field in ("purpose", "reason"):
            if not _sentence(row[field]):
                errors.append(f"{where}: {field} must be a sentence")
        parts = row["parts"]
        if not isinstance(parts, list) or len(parts) < 2:
            errors.append(f"{where}: a split row has at least two parts")
            continue
        covered = set()
        for k, part in enumerate(parts):
            pwhere = f"{where}/{k + 1}"
            if not isinstance(part, dict) or set(part) != PART_KEYS:
                errors.append(f"{pwhere}: part fields must be {sorted(PART_KEYS)}")
                continue
            prow = part["rows"]
            if not isinstance(prow, list) or not prow or any(r not in unit["rows"] for r in prow):
                errors.append(f"{pwhere}: rows must be a non-empty subset of the unit's inventory rows")
                continue
            covered |= set(prow)
            _check_decision(pwhere, part, unit, prow, flow_ids, errors)
        if covered and covered != set(unit["rows"]):
            errors.append(f"{where}: parts must cover every inventory row of the unit; missing {sorted(set(unit['rows']) - covered)}")
    for uid in sorted(expected - seen):
        errors.append(f"{path}: {uid} has no row")
    return errors


def check_all(units, flow_ids, directory=HERE):
    errors, rows = [], {}
    for n in range(1, 7):
        p = Path(directory) / f"dispositions-s{n}.json"
        if not p.is_file():
            errors.append(f"{p.name}: missing")
            continue
        file_errors = check_file(p, units, flow_ids, shard=f"s{n}")
        errors += file_errors
        data = _load(p)
        if any("top level" in e or "declares shard" in e for e in file_errors):
            continue
        for row in data["rows"]:
            if isinstance(row, dict) and row.get("unit") in units:
                if row["unit"] in rows:
                    errors.append(f"{p.name}: {row['unit']} also decided in another file")
                rows[row["unit"]] = row
    missing = set(units) - set(rows)
    if missing:
        errors.append(f"{len(missing)} unit(s) of units.json have no row, for example {sorted(missing)[:5]}")
    return errors, rows


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="*")
    ap.add_argument("--all", action="store_true")
    args = ap.parse_args(argv)
    units = {u["id"]: u for u in _load(HERE / "units.json")["units"]}
    flow_ids = {f["id"] for f in _load(HERE / "flows.json")["flows"]}
    if args.all:
        errors, _ = check_all(units, flow_ids)
    elif args.files:
        errors = [e for f in args.files for e in check_file(f, units, flow_ids)]
    else:
        ap.error("give files or --all")
    for e in errors:
        print(e)
    print("dispositions: ok" if not errors else f"dispositions: {len(errors)} error(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
