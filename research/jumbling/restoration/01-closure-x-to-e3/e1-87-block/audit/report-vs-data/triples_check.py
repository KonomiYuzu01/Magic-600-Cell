import json, sys
from collections import Counter
rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
print("rows", len(rows), "bad values", Counter(r["bad"] for r in rows).most_common(5))
own = [r for r in rows if (r["invB"], r["invA"], r["c_in"], r["pe_in"], r["e_in"]) == (True, True, True, True, False)]
print("owner-shaped", len(own))
ec = sorted({(r["e"], r["c"]) for r in own})
print("distinct (e,c)", len(ec), ec)
q = json.load(open(sys.argv[2]))
print("q87-ownerlike entries", len(q))
qec = [(x[1][0], x[2][0]) for x in q]
print("q87 (e,c) distinct", len(set(qec)), "== owner (e,c) set", set(qec) == set(ec))
# check each q87 Q appears among owner-shaped rows
ownQ = {json.dumps(r["Q"]) for r in own}
print("all q87 Q in owner-shaped rows", all(json.dumps(x) in ownQ for x in q))
print("landing_in_bad among owner-shaped", Counter(r.get("landing_in_bad") for r in own).most_common(5))
print("landing_in_bad among all", Counter(r.get("landing_in_bad") for r in rows).most_common(5))
