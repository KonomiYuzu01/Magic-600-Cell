"""Build the 0.4 function atlas: one self-contained HTML page from the stage 2.1 and 2.2 data.

Usage (from the repository root):
  python docs/progress/1.0/atlas/build.py [--out work/atlas/atlas.html]

Reads only committed files under docs/progress/1.0 and fails if the data disagree
(an unknown flow, a missing inventory row, or decision counts that differ from dispositions.md).
"""
import argparse
import glob
import json
import os
import re
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
root = os.path.normpath(os.path.join(HERE, "..", "..", "..", ".."))
ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
ap.add_argument("--out", default=os.path.join(root, "work", "atlas", "atlas.html"))
args = ap.parse_args()
template = os.path.join(HERE, "template.html")
out = args.out
P = os.path.join(root, "docs/progress/1.0")


def load(rel):
    with open(os.path.join(P, rel), encoding="utf-8") as fh:
        return json.load(fh)


units = load("dispositions/units.json")["units"]
flows = load("dispositions/flows.json")
shards = load("inventory/shards.json")["shards"]
commands = load("command-table.json")
topdown = {f["id"]: f for f in load("inventory/top-down.json")["functions"]}
screening = load("inventory/screening-map.json")["findings"]

disp = {}
for path in sorted(glob.glob(os.path.join(P, "dispositions/dispositions-s*.json"))):
    with open(path, encoding="utf-8") as fh:
        for row in json.load(fh)["rows"]:
            disp[row["unit"]] = row

rows = {}
for s in range(1, 7):
    for author, fname in (("claude", f"claude-source-s{s}.json"), ("codex", f"bottom-up-s{s}.json")):
        for f in load(f"inventory/{fname}")["functions"]:
            rows[f"{author}:{f['id']}"] = f


def row_view(rid):
    f = rows.get(rid)
    if not f:
        raise SystemExit(f"inventory row {rid} not found")
    return {
        "id": rid,
        "name": f.get("name"),
        "behaviour": f.get("behaviour"),
        "evidence": f.get("evidence", [])[:8],
        "evidence_more": max(0, len(f.get("evidence", [])) - 8),
        "engine_call": f.get("engine_call"),
        "reads": f.get("reads", []),
        "writes": f.get("writes", []),
        "kind": f.get("evidence_kind"),
        "notes": f.get("notes"),
    }


out_units = []
for u in units:
    d = disp[u["id"]]
    parts = []
    if d["disposition"] == "split":
        for i, p in enumerate(d["parts"], 1):
            parts.append({
                "n": i,
                "d": p["disposition"],
                "purpose": p["purpose"],
                "reason": p["reason"],
                "flows": p.get("flows", []),
                "req": p.get("requirements", []),
                "rows": p.get("rows", []),
            })
        dset = sorted({p["d"] for p in parts})
        uflows = []
        for p in parts:
            for fl in p["flows"]:
                if fl not in uflows:
                    uflows.append(fl)
    else:
        dset = [d["disposition"]]
        uflows = d.get("flows", [])
    out_units.append({
        "id": u["id"],
        "name": u["names"][0],
        "names": u["names"],
        "shard": u["shard"],
        "d": d["disposition"],
        "dset": dset,
        "purpose": d["purpose"],
        "reason": d["reason"],
        "flows": uflows,
        "req": d.get("requirements", []),
        "notes": d.get("notes"),
        "same": d.get("same_purpose_as", []),
        "parts": parts,
        "dep": u["depends_on_units"],
        "used": u["used_by_units"],
        "unres": u["unresolved_depends_on"],
        "entry": u["entry_points"],
        "scr": [s.split(" ")[0] for s in u["screening"]],
        "rows": [row_view(r) for r in u["rows"]],
    })

flow_ids = {f["id"] for f in flows["flows"]}
for u in out_units:
    for fl in u["flows"]:
        if fl not in flow_ids:
            raise SystemExit(f"{u['id']} names unknown flow {fl}")

decisions = {}
for u in out_units:
    for d in ([p["d"] for p in u["parts"]] or [u["d"]]):
        decisions[d] = decisions.get(d, 0) + 1

# The signed report's "all" row must match what the page shows.
with open(os.path.join(P, "dispositions/dispositions.md"), encoding="utf-8") as fh:
    m = re.search(r"^\| all \| (\d+) \| (\d+) \| (\d+) \| (\d+) \| (\d+) \|$", fh.read(), re.M)
if not m:
    raise SystemExit("dispositions.md has no 'all' count row")
want = dict(zip(("keep", "redesign", "delete", "automate"), map(int, m.groups()[:4])))
got = {k: decisions.get(k, 0) for k in want}
if got != want or sum(decisions.values()) != int(m.group(5)):
    raise SystemExit(f"decision counts {decisions} differ from dispositions.md {want}")

cmd = []
for c in commands["commands"]:
    cmd.append({
        "id": c["id"], "label": c["label"], "kind": c["kind"], "gates": c["gates"],
        "preview": c["preview"], "undo": c["undo"], "contexts": c["contexts"],
        "repeat": c["repeat"], "job": c["job"], "key": c["key_0_4"],
        "disp": c["disposition"], "note": c["note"],
        "inv": [{"id": t, "name": topdown[t]["name"], "area": topdown[t]["area"]} if t in topdown else {"id": t}
                for t in c["inventory"]],
    })

scr = []
for f in screening:
    scr.append({
        "id": f["id"], "sev": f["severity"], "status": f["status"], "area": f["screening_shard"],
        "summary": f["summary"],
        "where": [f"{c['path']}:{c['start']}-{c['end']} {c.get('symbol') or ''}".strip() for c in f["current"]],
        "units": [u["id"] for u in out_units if f["id"] in u["scr"]],
    })

def git(*args):
    return subprocess.run(["git", "-C", root, *args], capture_output=True, text=True, check=True).stdout.strip()

data = {
    "stamp": {"commit": git("rev-parse", "--short", "HEAD"), "date": git("log", "-1", "--format=%cd", "--date=short")},
    "flows": flows["flows"],
    "flowStatus": flows["status"],
    "shards": {k: {"title": v["title"], "files": len(v["files"])} for k, v in shards.items()},
    "units": out_units,
    "decisions": decisions,
    "commands": cmd,
    "cmdVocab": commands["vocabularies"],
    "notCommands": commands["not_commands"],
    "screening": scr,
}
blob = json.dumps(data, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")
with open(template, encoding="utf-8") as fh:
    html = fh.read()
if html.count("/*__DATA__*/null") != 1:
    raise SystemExit("template.html must contain the data placeholder exactly once")
html = html.replace("/*__DATA__*/null", blob)
os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
with open(out, "w", encoding="utf-8") as fh:
    fh.write(html)
print("units", len(out_units), "decisions", decisions, "commands", len(cmd), "screening", len(scr), "bytes", len(html))
