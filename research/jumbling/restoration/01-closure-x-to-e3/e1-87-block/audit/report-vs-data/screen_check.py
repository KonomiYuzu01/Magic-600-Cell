import json, sys
d = sys.argv[1]
rows = [json.loads(l) for l in open(d + "/screen-87.jsonl")]
cands = json.load(open(d + "/cands-87.json"))
print("screen rows", len(rows), "cands", len(cands))
keys = set()
for r in rows: keys |= set(r)
print("keys", sorted(keys))
for r in rows:
    print(r.get("n"), r.get("source"), "T_on", r.get("T_on"), "prot", r.get("prot_exact"), "badW", r.get("badW"),
          "occ", r.get("badW_occupied"), "land_in", r.get("landing_in_badW"), "badPinv", r.get("badPinv"),
          "TPQinbad", r.get("T_after_PQ_in_badPinv"), "N1", r.get("N1"), "N2", r.get("N2"),
          "hit", r.get("orbits_hit"), "tight", r.get("tight_orbits"), "sec", r.get("sec"),
          "eq_land", sorted(r.get("landing_W", [])) == sorted(r.get("badW_chambers", [])),
          "Y", r["P"][1], "Z", r["P"][4], "Qa", r["Q"][0], "Qe", r["Q"][1], "X", r["P"][0])
# match with cands
print("cands match screen order", [c["source"] for c in cands] == [r["source"] for r in rows])
# distinct Q among candidates
print("distinct Q", len({json.dumps(c["Q"]) for c in cands}))
print("distinct P", len({json.dumps(c["P"]) for c in cands}))
# c0, c2 indices
for lab in ["c0","c2","c5","c6","c13"]:
    w = json.load(open(d + "/w87-%s.json" % lab))
    idx = [i for i, c in enumerate(cands) if c["P"] == w["P"] and c["Q"] == w["Q"]]
    print(lab, "source", w["source"], "cand index", idx)
