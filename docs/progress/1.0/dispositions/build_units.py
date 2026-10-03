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
            entry_owner[ep].add(unit_of[rid])

    findings = defaultdict(set)
    for f in merged["screening"]:
        for rid in f.get("rows", []):
            if rid in unit_of:
                findings[unit_of[rid]].add(f"{f['id']} ({f['status']})")

    open_pairs = defaultdict(set)
    for c in merged["open_candidates"]:
        a, b = unit_of.get(c["claude"]), unit_of.get(c["codex"])
        if a and b and a != b:
            open_pairs[a].add(b)
            open_pairs[b].add(a)

    links_of = {}
    for uid, shard, members in units:
        links = set()
        for m in members:
            side = m.split(":")[0]
            for dep in rows[m]["depends_on"]:
                target = f"{side}:{dep}" if ROW_ID.match(dep) else None
                if target in unit_of:
                    links.add(unit_of[target])
                links |= entry_owner.get(dep, set())
        links.discard(uid)
        links_of[uid] = links
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
        })
    return {"format": "C600-DISPOSITION-UNITS-draft",
            "source": "docs/progress/1.0/inventory/merged.json and the twelve sealed inventory files",
            "units": out}


def main():
    text = json.dumps(build(), indent=1, ensure_ascii=False, sort_keys=True) + "\n"
    (HERE / "units.json").write_text(text, encoding="utf-8", newline="\n")
    print(f"units: {len(json.loads(text)['units'])}")


if __name__ == "__main__":
    main()
