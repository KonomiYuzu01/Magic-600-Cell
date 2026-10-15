"""Compare exact pose matrices between pose tables (pure JSON reads, no simulator)."""
import json, sys, hashlib

def load(p):
    d = json.load(open(p))
    return d

def canon(m):
    # canonicalize entries [a,b,d] -> reduced fraction representation (a/d, b/d) as tuples of ints
    from fractions import Fraction
    return tuple(tuple((Fraction(e[0], e[2]), Fraction(e[1], e[2])) for e in row) for row in m)

paths = dict(X=sys.argv[1], E1=sys.argv[2], E2=sys.argv[3])
extra = sys.argv[4:]  # optional label=path
for e in extra:
    k, v = e.split("=", 1); paths[k] = v
T = {}
for k, p in paths.items():
    d = load(p)
    keys = sorted(d.keys())
    poses = [canon(m) for m in d["poses"]]
    pid = d["pose_id"]
    T[k] = (poses, pid)
    print(k, "keys", keys, "n_poses", len(poses), "n_pieces", len(pid), "digest", d.get("digest"))

def mat(k, p):
    poses, pid = T[k]
    return poses[pid[p]]

n = len(T["E1"][1])
def changed(a, b):
    return [p for p in range(n) if mat(a, p) != mat(b, p)]

pre = json.load(open(sys.argv[0].replace("tables_check.py", "") + "../../E2/c0/pre.json"))
blk87 = sorted(pre["target_block"]); bprime = sorted(pre["protected_off"])
mech = json.load(open("<work>/pkg26/Claude-E-closure-X1-20261012/input/X-mechanism.json"))["X"]
blk17 = sorted(mech["blocks"][1]["pieces"])
print("87-block from pre == X-mechanism block 0", blk87 == sorted(mech["blocks"][0]["pieces"]))
c12 = changed("E1", "E2")
print("E1->E2 changed", len(c12), "== 87-block", sorted(c12) == blk87)
cx2 = changed("X", "E2")
print("X->E2 changed", len(cx2), "== 87+17 blocks", sorted(cx2) == sorted(blk87 + blk17))
cx1 = changed("X", "E1")
print("X->E1 changed", len(cx1), "== 17-block", sorted(cx1) == blk17)
for k in T:
    print("B' at", k, [mat(k, p) == mat("E2", bprime[0]) for p in bprime])
print("17-block same E1/E2", all(mat("E1", p) == mat("E2", p) for p in blk17))
# B' poses vs chain-check and pre.json
cc = json.load(open(sys.argv[0].replace("tables_check.py", "") + "../../E2/c0/chain-check.json"))
for p in bprime:
    print("chain-check B'", p, canon(cc["Bprime_poses_at_E2"][str(p)]) == mat("E2", p),
          "pre.json", canon(pre["protected_poses"][str(p)]) == mat("E1", p))
