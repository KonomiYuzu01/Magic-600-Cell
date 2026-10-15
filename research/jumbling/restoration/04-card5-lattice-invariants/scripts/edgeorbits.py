"""K+-orbits on the edges of the 'pair' graph of an orbit (two chambers of the orbit sharing all but one cap, the
sub-signature shared by exactly these two). If the edges split into two K+-orbits that are perfect matchings, the
matching is K+-invariant. usage: edgeorbits.py <research/jumbling> <classes.json> <arrays.npz> <orbit> <out json>"""
import json, sys
from collections import Counter, defaultdict
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'e3s'))
sys.path.insert(0, str(Path(__file__).resolve().parent))  # package layout: e3lib.py next to this script
from e3lib import Lib  # noqa: E402
L = Lib(sys.argv[1], sys.argv[2], sys.argv[3])
d = L.d
oid = np.asarray(d.orbit_id)
o = int(sys.argv[4])
ch = np.flatnonzero(oid == o)
sig = {int(p): frozenset(int(c) for c in d.signature_list(int(p))) for p in ch}
bysub = defaultdict(list)
for p, s in sig.items():
    for c in s:
        bysub[s - {c}].append(p)
edges = sorted(tuple(sorted(ps)) for ps in bysub.values() if len(ps) == 2)
eidx = {e: i for i, e in enumerate(edges)}
A = np.array([e[0] for e in edges]); B = np.array([e[1] for e in edges])
lab = -np.ones(len(edges), np.int64)
norb = 0
for i in range(len(edges)):
    if lab[i] >= 0:
        continue
    # orbit of edge i under all of K+
    imgsA = L.reg.piece_of[L.orb[np.full(7200, A[i])], L.kp.compose_vec(np.arange(7200), np.full(7200, L.tr[A[i]]))]
    imgsB = L.reg.piece_of[L.orb[np.full(7200, B[i])], L.kp.compose_vec(np.arange(7200), np.full(7200, L.tr[B[i]]))]
    js = {eidx[tuple(sorted((int(a), int(b))))] for a, b in zip(imgsA.tolist(), imgsB.tolist())}
    for j in js:
        lab[j] = norb
    norb += 1
sizes = Counter(lab.tolist())
cover = {}
for k in range(norb):
    deg = Counter()
    for i in np.flatnonzero(lab == k).tolist():
        deg[edges[i][0]] += 1; deg[edges[i][1]] += 1
    cover[k] = {'edges': sizes[k], 'vertices covered': len(deg), 'perfect matching': len(deg) == len(ch) and set(deg.values()) == {1}}
print('orbit', o, 'edges', len(edges), 'K+ edge orbits', norb, cover)
json.dump({'orbit': o, 'edges': [list(e) for e in edges], 'edge_orbit': lab.tolist(), 'orbits': cover}, open(sys.argv[5], 'w'))
