"""Deterministic comparison of the two sealed stage 2.1 inventories.

Usage:
  python docs/progress/1.0/inventory/merge_inventories.py           write merged.json and merged.md
  python docs/progress/1.0/inventory/merge_inventories.py --check   verify both are current

Inputs (this folder): the twelve sealed files (`bottom-up-sN.json` by Codex, `claude-source-sN.json` by
Claude) with their digests in `sealed.json`, `census.json`, `screening-map.json` and the integrator's
`correspondence.json` and `screening-attachments.json`. Shared entry-point keys and shared evidence lines only propose candidate pairs, and only between rows of
the same shard (rows of different shards describe different layers);
a match exists only where `correspondence.json` records it with a reason. Field differences of matched
rows (behaviour text included) are listed, never reconciled. No row carries a keep, redesign or delete judgement.
Standard library only. Running it twice gives byte-identical output.
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SIDES = {"codex": "bottom-up-s{}.json", "claude": "claude-source-s{}.json"}
SHARDS = range(1, 7)
OUT_JSON, OUT_MD = HERE / "merged.json", HERE / "merged.md"
# Packaged programs and their Python entry scripts (packaging/package_contract.py:70, check_package.py:316).
EXE_ALIASES = {"launcher.py": {"magic600cell.exe"}, "engine_entry.py": {"magic600engine.exe"}}
COMPARED = ("behaviour", "reads", "writes", "engine_call", "evidence_kind")


def load(name: str):
    return json.loads((HERE / name).read_text(encoding="utf-8"))


def sealed_errors() -> list[str]:
    seals = load("sealed.json")["files"]
    errors = []
    for side, pattern in SIDES.items():
        for n in SHARDS:
            name = pattern.format(n)
            digest = hashlib.sha256((HERE / name).read_bytes()).hexdigest()
            if seals.get(name, {}).get("sha256") != digest:
                errors.append(f"{name}: digest differs from sealed.json")
    return errors


def norm(entry: str) -> str:
    e = re.sub(r"\s+", " ", entry.strip().lower())
    e = re.sub(r"^key bank ", "key ", e)
    e = e.replace(" (all banks)", "")
    return e


def keys(row: dict) -> set[str]:
    """Candidate keys: command IDs, routes with their action, CLI entries and exact evidence lines."""
    out = set()
    for e in map(norm, row["entry_points"]):
        if e.startswith("command "):
            out.add("command " + e.split()[1])
        elif e.startswith("route "):
            parts = e.split()
            action = re.search(r"action=([\w-]+)", e)
            if len(parts) >= 3:
                out.add(f"route {parts[1]} {parts[2]}" + (f" action={action.group(1)}" if action else ""))
        elif e.startswith("cli "):
            out.add(e)
    out.update("evidence " + ev for ev in row["evidence"])
    return out


def rows() -> dict[str, dict]:
    out = {}
    for side, pattern in SIDES.items():
        for n in SHARDS:
            for r in load(pattern.format(n))["functions"]:
                out[f"{side}:{r['id']}"] = r
    return out


def census_missing(all_rows: dict[str, dict], side: str) -> list[str]:
    entries = [norm(e) for rid, r in all_rows.items() if rid.startswith(side + ":") for e in r["entry_points"]]
    commands = {e.split()[1] for e in entries if e.startswith("command ") and len(e.split()) > 1}
    actions = {a for e in entries for a in re.findall(r"action=([\w-]+)", e)}
    missing = []
    for item in load("census.json")["items"]:
        k, i = item["kind"], item["id"]
        if k == "command":
            hit = i.lower() in commands
        elif k == "action":
            hit = i.lower() in actions
        elif k == "route":
            path = i.lower().rstrip("*")
            hit = any(e.startswith("route ") and any(t.startswith(path) if i.endswith("*") else t == path
                                                      for t in e.split()[1:3]) for e in entries)
        else:
            prog, opt = i.lower().split(" ", 1)
            names = {prog} | EXE_ALIASES.get(prog, set())
            hit = any(e.startswith("cli ") and e.split()[1].rsplit("/", 1)[-1] in names and opt in e.split()[2:]
                      for e in entries if len(e.split()) > 2)
        if not hit:
            missing.append(f"{k} {i}")
    return missing


def attachments(all_rows: dict[str, dict]) -> list[dict]:
    """Reviewed attachments from `screening-attachments.json`; evidence lines inside a finding's mapped
    range are kept only as `line_candidates`, a lead that never attaches a finding by itself."""
    reviewed = load("screening-attachments.json")["findings"]
    out = []
    for f in load("screening-map.json")["findings"]:
        if f["id"] not in reviewed:
            raise SystemExit(f"screening-attachments.json has no entry for {f['id']}")
        r = reviewed[f["id"]]
        unknown = [i for i in r["codex"] + r["claude"] if i not in all_rows]
        if unknown:
            raise SystemExit(f"screening-attachments.json names unknown rows: {unknown}")
        hits = set()
        for c in f["current"]:
            for rid, row in all_rows.items():
                for ev in row["evidence"]:
                    path, line = ev.rsplit(":", 1)
                    if path == c["path"] and c["start"] <= int(line) <= c["end"]:
                        hits.add(rid)
        rows_ = sorted(r["codex"]) + sorted(r["claude"])
        out.append({"id": f["id"], "status": f["status"], "severity": f["severity"], "rows": rows_,
                    "reason": r["reason"], "line_candidates": sorted(hits),
                    "unattached": [] if rows_ else f["unattached"], "attached": bool(rows_)})
    extra = sorted(set(reviewed) - {f["id"] for f in load("screening-map.json")["findings"]})
    if extra:
        raise SystemExit(f"screening-attachments.json names unknown findings: {extra}")
    return out


def build() -> dict:
    errors = sealed_errors()
    if errors:
        raise SystemExit("\n".join(errors))
    all_rows = rows()
    by_key: dict[str, set[str]] = {}
    for rid, r in all_rows.items():
        for k in keys(r):
            by_key.setdefault(k, set()).add(rid)
    candidates = {}
    for k, ids in by_key.items():
        cx = sorted(i for i in ids if i.startswith("codex:"))
        cl = sorted(i for i in ids if i.startswith("claude:"))
        for a in cx:
            for b in cl:
                if a.split(":")[1].split("-")[0] == b.split(":")[1].split("-")[0]:
                    # Rows of different shards describe different layers (for example the native button and
                    # the adapter action it sends); a shared key there is a dependency, not a match.
                    candidates.setdefault((a, b), set()).add(k)
    corr = load("correspondence.json")["pairs"] if (HERE / "correspondence.json").is_file() else []
    matched, used = [], set()
    for c in corr:
        unknown = [i for i in c["codex"] + c["claude"] if i not in all_rows]
        if unknown:
            raise SystemExit(f"correspondence names unknown rows: {unknown}")
        diffs = {}
        for field in COMPARED:
            a = sorted({json.dumps(all_rows[i][field], sort_keys=True) for i in c["codex"]})
            b = sorted({json.dumps(all_rows[i][field], sort_keys=True) for i in c["claude"]})
            if a != b:
                diffs[field] = {"codex": [json.loads(x) for x in a], "claude": [json.loads(x) for x in b]}
        ea = sorted({norm(e) for i in c["codex"] for e in all_rows[i]["entry_points"]})
        eb = sorted({norm(e) for i in c["claude"] for e in all_rows[i]["entry_points"]})
        if ea != eb:
            diffs["entry_points"] = {"codex_only": sorted(set(ea) - set(eb)), "claude_only": sorted(set(eb) - set(ea))}
        matched.append({"codex": c["codex"], "claude": c["claude"], "kind": c["kind"], "reason": c["reason"],
                        "differences": diffs})
        used.update(c["codex"] + c["claude"])
    unmatched_candidates = [{"codex": a, "claude": b, "shared": sorted(k)} for (a, b), k in sorted(candidates.items())
                            if a not in used or b not in used]
    only = {side: [{"id": rid, "name": r["name"]} for rid, r in sorted(all_rows.items())
                   if rid.startswith(side + ":") and rid not in used] for side in SIDES}
    return {"format": "C600-INVENTORY-MERGE-draft",
            "counts": {"codex_rows": sum(1 for i in all_rows if i.startswith("codex:")),
                       "claude_rows": sum(1 for i in all_rows if i.startswith("claude:")),
                       "correspondences": len(matched), "codex_only": len(only["codex"]),
                       "claude_only": len(only["claude"]), "open_candidates": len(unmatched_candidates)},
            "matched": matched, "only_codex": only["codex"], "only_claude": only["claude"],
            "open_candidates": unmatched_candidates,
            "census_missing": {side: census_missing(all_rows, side) for side in SIDES},
            "screening": attachments(all_rows)}


def markdown(doc: dict) -> str:
    c = doc["counts"]
    out = ["# Stage 2.1 inventories: comparison", "",
           "Generated by `merge_inventories.py`; do not edit. No row carries a keep, redesign or delete judgement. "
           "Method: [README](README.md).", "", "## Counts", "", "| | |", "|---|---|"]
    out += [f"| {k.replace('_', ' ')} | {v} |" for k, v in c.items()]
    out += ["", "## Census omissions", "",
            "Entry points the source census found that a side's inventory does not name as an entry point.", ""]
    for side, items in doc["census_missing"].items():
        out += [f"### {side.capitalize()} ({len(items)})", ""] + ([f"- `{i}`" for i in items] or ["- none"]) + [""]
    out += ["## Screening findings", "",
            "Attachments are the integrator's reviewed mapping (`screening-attachments.json`), by affected behaviour.", "",
            "| Finding | Status | Rows | Reason |", "|---|---|---|---|"]
    for s in doc["screening"]:
        rows_ = ", ".join(f"`{r}`" for r in s["rows"]) if s["rows"] else "none"
        out.append(f"| {s['id']} | {s['status']} | {rows_} | {s['reason']} |")
    for side in ("codex", "claude"):
        items = doc[f"only_{side}"]
        out += ["", f"## Only in the {side.capitalize()} inventory ({len(items)})", ""]
        out += [f"- `{i['id']}` {i['name']}" for i in items] or ["- none"]
    out += ["", f"## Matched rows ({len(doc['matched'])})", "", "| Codex | Claude | Kind | Differing fields | Reason |",
            "|---|---|---|---|---|"]
    for m in doc["matched"]:
        out.append(f"| {', '.join(m['codex'])} | {', '.join(m['claude'])} | {m['kind']} | "
                   f"{', '.join(sorted(m['differences'])) or 'none'} | {m['reason']} |")
    return "\n".join(out) + "\n"


def main(argv: list[str]) -> int:
    doc = build()
    j = json.dumps(doc, indent=1, sort_keys=True, ensure_ascii=False) + "\n"
    md = markdown(doc)
    if argv == ["--check"]:
        ok = OUT_JSON.is_file() and OUT_JSON.read_text(encoding="utf-8") == j and OUT_MD.read_text(encoding="utf-8") == md
        print("merge: ok" if ok else "merged.json or merged.md is stale; run merge_inventories.py")
        return 0 if ok else 1
    if argv:
        print(__doc__.strip().splitlines()[2].strip())
        return 2
    OUT_JSON.write_text(j, encoding="utf-8")
    OUT_MD.write_text(md, encoding="utf-8")
    print("merge:", doc["counts"])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
