"""Build the stage 2.2 disposition units from the stage 2.1 inventories.

A unit is one 0.4 function: the inventory rows that the integrator's
correspondences (merged.json) join, closed transitively. A row with no
correspondence is a unit of its own. Units never cross shards, because
correspondences are recorded only within a shard. Links between units of
different layers come from `depends_on` and are listed, not merged.

Run from the repository root: python docs/progress/1.0/dispositions/build_units.py
Running it twice gives byte-identical output.
"""
import json
import re
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
INV = HERE.parent / "inventory"
ROW_ID = re.compile(r"^S([1-6])-(\d+)$")
HANDLER = re.compile(r"^native shell handler registered for command ([\w-]+)\b")
ROUTE = re.compile(r"^(?:route\s+)?(GET|POST|PUT|DELETE)\s+/?(?:api/)?(\S.*)$", re.I)


def normalize(ref):
    """Map the notations the two inventories use for one entry point to one key.

    `route POST /api/x action=y`, `POST /api/x action=y` and `POST x action=y`
    name the same route; `native shell handler registered for command x (...)`
    names `command x`. Anything else is compared as written.
    """
    ref = " ".join(ref.split())
    m = HANDLER.match(ref)
    if m:
        return f"command {m.group(1)}"
    m = ROUTE.match(ref)
    if m:
        return f"route {m.group(1).upper()} {m.group(2)}"
    return ref


def _key(row_id):
    side, rid = row_id.split(":")
    m = ROW_ID.match(rid)
    return (int(m.group(1)), int(m.group(2)), side)


def load_rows():
    rows = {}
    for side, stem in (("codex", "bottom-up"), ("claude", "claude-source")):
        for n in range(1, 7):
            data = json.loads((INV / f"{stem}-s{n}.json").read_text(encoding="utf-8"))
            for r in data["functions"]:
                rows[f"{side}:{r['id']}"] = r
    return rows


def build():
    rows = load_rows()
    merged = json.loads((INV / "merged.json").read_text(encoding="utf-8"))
    parent = {k: k for k in rows}

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for e in merged["matched"]:
        ids = e["claude"] + e["codex"]
        for other in ids[1:]:
            a, b = find(ids[0]), find(other)
            if a != b:
                parent[max(a, b, key=_key)] = min(a, b, key=_key)

    groups = defaultdict(list)
    for k in rows:
        groups[find(k)].append(k)
    ordered = sorted((sorted(v, key=_key) for v in groups.values()), key=lambda v: _key(v[0]))

    unit_of, counters, units = {}, defaultdict(int), []
    for members in ordered:
        shard = _key(members[0])[0]
        counters[shard] += 1
        uid = f"U{shard}-{counters[shard]:03d}"
        for m in members:
            unit_of[m] = uid
        units.append((uid, shard, members))

    entry_owner = defaultdict(set)
    for rid, r in rows.items():
        for ep in r["entry_points"]:
            entry_owner[normalize(ep)].add(unit_of[rid])

    findings = defaultdict(set)
    finding_rows = defaultdict(lambda: defaultdict(list))
    for f in merged["screening"]:
        for rid in f.get("rows", []):
            if rid in unit_of:
                findings[unit_of[rid]].add(f"{f['id']} ({f['status']})")
                finding_rows[unit_of[rid]][f["id"]].append(rid)

    open_pairs = defaultdict(set)
    for c in merged["open_candidates"]:
        a, b = unit_of.get(c["claude"]), unit_of.get(c["codex"])
        if a and b and a != b:
            open_pairs[a].add(b)
            open_pairs[b].add(a)

    links_of, unresolved = {}, {}
    for uid, shard, members in units:
        links, missing = set(), set()
        for m in members:
            side = m.split(":")[0]
            for dep in rows[m]["depends_on"]:
                target = f"{side}:{dep}" if ROW_ID.match(dep) else None
                found = ({unit_of[target]} if target in unit_of else set()) | entry_owner.get(normalize(dep), set())
                if found:
                    links |= found
                else:
                    missing.add(dep)
        links.discard(uid)
        links_of[uid] = links
        unresolved[uid] = sorted(missing)
    used_by = defaultdict(set)
    for uid, links in links_of.items():
        for t in links:
            used_by[t].add(uid)

    out = []
    for uid, shard, members in units:
        out.append({
            "id": uid,
            "shard": f"s{shard}",
            "rows": members,
            "names": sorted({rows[m]["name"] for m in members}),
            "entry_points": sorted({ep for m in members for ep in rows[m]["entry_points"]}),
            "depends_on_units": sorted(links_of[uid]),
            "used_by_units": sorted(used_by[uid]),
            "open_candidates": sorted(open_pairs[uid]),
            "screening": sorted(findings[uid]),
            "screening_rows": {k: sorted(v, key=_key) for k, v in sorted(finding_rows[uid].items())},
            "unresolved_depends_on": unresolved[uid],
        })
    return {"format": "C600-DISPOSITION-UNITS-draft",
            "source": "docs/progress/1.0/inventory/merged.json and the twelve sealed inventory files",
            "units": out}


def main():
    text = json.dumps(build(), indent=1, ensure_ascii=False, sort_keys=True) + "\n"
    (HERE / "units.json").write_text(text, encoding="utf-8", newline="\n")
    units = json.loads(text)["units"]
    print(f"units: {len(units)}")
    for n in range(1, 7):
        mine = [u for u in units if u["shard"] == f"s{n}"]
        print(f"s{n}: {sum(1 for u in mine if u['depends_on_units'])} with links, "
              f"{sum(len(u['unresolved_depends_on']) for u in mine)} unresolved dependencies (module functions and free text)")


if __name__ == "__main__":
    main()
