import json, sys
p = sys.argv[1]
rows = [json.loads(l) for l in open(p)]
print("rows", len(rows), "prefixes", rows[0]["prefix"], rows[-1]["prefix"],
      "contiguous", [r["prefix"] for r in rows] == list(range(len(rows))))
print("all depth_ok", all(r["depth_ok"] for r in rows),
      "any failed domains", any(r["depth_failed_domains"] for r in rows))
keys = set()
for r in rows: keys |= set(r)
print("keys", sorted(keys))
# stage boundaries
stages = {}
for r in rows[1:]:
    stages.setdefault(r["file"], []).append(r)
for f, rs in stages.items():
    print(f, "n", len(rs), "prefix", rs[0]["prefix"], "-", rs[-1]["prefix"])
ends = {"V": 590, "P": 600, "Q": 610, "Pinv": 620, "Vinv": 1210}
starts = {"V": 0, "P": 590, "Q": 600, "Pinv": 610, "Vinv": 620}
for f in ["V", "P", "Q", "Pinv", "Vinv"]:
    e = rows[ends[f]]
    inside = [r["off"] for r in rows[starts[f]+1:ends[f]+1]]
    incl = [r["off"] for r in rows[starts[f]:ends[f]+1]]
    print(f, "end digest", e["digest"], "off", e["off"], "restored", e["restored"],
          "collateral", e["collateral"], "pose_changed", e["pose_changed"],
          "off range excl start", min(inside), max(inside), "incl start", min(incl), max(incl))
coll = [r["prefix"] for r in rows if r["collateral"] > 0]
print("collateral>0 prefixes", coll[0] if coll else None, coll[-1] if coll else None, len(coll),
      "contiguous", coll == list(range(coll[0], coll[-1]+1)) if coll else None,
      "max", max(r["collateral"] for r in rows))
pc = [(r["prefix"], r["pose_changed"]) for r in rows if r["pose_changed"] > 0]
print("pose_changed>0 count", len(pc), "max", max(x[1] for x in pc) if pc else None)
print("pose_changed prefixes", pc)
print("restored values by prefix 595..625", [(r["prefix"], r["restored"], r["collateral"], r["off"]) for r in rows[590:622]])
# letters check vs word-full
wf = json.load(open(sys.argv[2]))
print("word-full keys", list(wf)[:10] if isinstance(wf, dict) else type(wf))
