"""Check stage 2.2 disposition files against units.json and flows.json.

  python docs/progress/1.0/dispositions/check_dispositions.py dispositions-s3.json
  python docs/progress/1.0/dispositions/check_dispositions.py --all

One file per shard. Each unit of the shard gets exactly one row; --all also
requires all six files and checks links between shards.
"""
import argparse
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
DISPOSITIONS = ("keep", "redesign", "delete", "automate")
ROW_KEYS = {"unit", "disposition", "purpose", "reason", "flows", "same_purpose_as", "requirements"}
OPTIONAL_KEYS = {"notes"}
UNIT_ID = re.compile(r"^U[1-6]-\d{3}$")


def _load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def check_file(path, units, flow_ids):
    errors = []
    data = _load(path)
    if not isinstance(data, dict) or set(data) != {"shard", "rows"}:
        return [f"{path}: top level must be exactly {{shard, rows}}"]
    shard = data["shard"]
    expected = {u["id"] for u in units.values() if u["shard"] == shard}
    if not expected:
        return [f"{path}: unknown shard {shard!r}"]
    seen = set()
    for i, row in enumerate(data["rows"]):
        where = f"{path}: row {i}"
        if not isinstance(row, dict):
            errors.append(f"{where}: not an object")
            continue
        keys = set(row)
        if not ROW_KEYS <= keys or keys - ROW_KEYS - OPTIONAL_KEYS:
            errors.append(f"{where}: fields must be {sorted(ROW_KEYS)} plus optional {sorted(OPTIONAL_KEYS)}")
            continue
        uid = row["unit"]
        where = f"{path}: {uid}"
        if uid not in expected:
            errors.append(f"{where}: not a unit of {shard}")
        if uid in seen:
            errors.append(f"{where}: duplicate")
        seen.add(uid)
        if row["disposition"] not in DISPOSITIONS:
            errors.append(f"{where}: disposition must be one of {DISPOSITIONS}")
        for field in ("purpose", "reason"):
            if not isinstance(row[field], str) or len(row[field].strip()) < 10:
                errors.append(f"{where}: {field} must be a sentence")
        if "notes" in row and (not isinstance(row["notes"], str) or not row["notes"].strip()):
            errors.append(f"{where}: notes must be a non-empty string when present")
        flows = row["flows"]
        if not isinstance(flows, list) or any(f not in flow_ids for f in flows) or len(set(flows)) != len(flows):
            errors.append(f"{where}: flows must be distinct ids from flows.json")
        elif not flows and row["disposition"] in ("keep", "redesign"):
            errors.append(f"{where}: a kept or redesigned unit serves at least one flow")
        same = row["same_purpose_as"]
        if not isinstance(same, list) or any(not isinstance(s, str) or not UNIT_ID.match(s) or s not in units or s == uid for s in same):
            errors.append(f"{where}: same_purpose_as must list other existing unit ids")
        reqs = row["requirements"]
        if not isinstance(reqs, list) or any(not isinstance(r, str) or not r.strip() for r in reqs):
            errors.append(f"{where}: requirements must be a list of strings")
        else:
            needed = {s.split(" ")[0] for s in units.get(uid, {}).get("screening", []) if "(open)" in s or "(needs verification)" in s}
            named = {r.split(":")[0].strip() for r in reqs}
            if row["disposition"] in ("keep", "redesign") and needed - named:
                errors.append(f"{where}: requirements must name open screening findings {sorted(needed - named)}")
    for uid in sorted(expected - seen):
        errors.append(f"{path}: {uid} has no row")
    return errors


def check_all(units, flow_ids):
    errors, rows = [], {}
    for n in range(1, 7):
        p = HERE / f"dispositions-s{n}.json"
        if not p.is_file():
            errors.append(f"{p.name}: missing")
            continue
        errors += check_file(p, units, flow_ids)
        for row in _load(p).get("rows", []):
            if isinstance(row, dict) and "unit" in row:
                rows[row["unit"]] = row
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
