"""Orientation structure of the K+-invariant matchings of an orbit under local retained moves (geometry and K+
only, menu-independent). For each matching (edge orbit k of edgeorbits.py) build the graph on oriented edges
(p -> q) whose arcs are the retained letters (g, a), a in A4_g, applied to edges lying wholly in cap g. If no
component contains both orientations of an edge, there is an orientation preserved by every local retained move.
Also reports, per grip, how many edges of each matching lie wholly in its cap and how many chambers of the cap have
their partner outside, and whether some a in A4_g fixes an edge of cap g as a set while swapping its ends.
usage: orient.py <research/jumbling> <classes.json> <arrays.npz> <edges json> <out json>"""
import json, sys
from collections import Counter
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'e3s'))
sys.path.insert(0, str(Path(__file__).resolve().parent))  # package layout: e3lib.py next to this script
from e3lib import Lib  # noqa: E402
L = Lib(sys.argv[1], sys.argv[2], sys.argv[3])
d = L.d
E = json.load(open(sys.argv[4]))
edges = [tuple(e) for e in E['edges']]
lab = E['edge_orbit']
# stabilizer A4_g: the K+ elements fixing grip g (perms[k, g] == g)
perms = L.perms
res = {'orbit': E['orbit']}
for k in sorted(set(lab)):
    M = [e for e, l in zip(edges, lab) if l == k]
    partner = {}
    for a, b in M:
        partner[a] = b; partner[b] = a
    idx = {}
    for i, (a, b) in enumerate(M):
        idx[(a, b)] = 2 * i; idx[(b, a)] = 2 * i + 1
    par = list(range(2 * len(M)))

    def find(x):
        while par[x] != x:
            par[x] = par[par[x]]
            x = par[x]
        return x

    def union(x, y):
        x, y = find(x), find(y)
        if x != y:
            par[x] = y
    full, half, selfswap = Counter(), Counter(), 0
    for g in range(600):
        stab = np.flatnonzero(perms[:, g] == g)
        members = set(int(p) for p in d.cap_members(g) if partner.get(int(p)) is not None)
        inside = [(a, b) for a, b in M if a in members and b in members]
        full[len(inside)] += 1
        half[sum(1 for p in members if partner[p] not in members)] += 1
        if not inside:
            continue
        A = np.array([e[0] for e in inside]); B = np.array([e[1] for e in inside])
        for kk in stab.tolist():
            if kk == 0:
                continue
            ia, ib = L.act(kk, A), L.act(kk, B)
            for (a, b), x, y in zip(inside, ia.tolist(), ib.tolist()):
                assert (x, y) in idx, 'matching not preserved'
                union(idx[(a, b)], idx[(x, y)])
                union(idx[(b, a)], idx[(y, x)])
                if (x, y) == (b, a):
                    selfswap += 1
    comps = Counter(find(x) for x in range(2 * len(M)))
    both = sum(1 for i in range(len(M)) if find(2 * i) == find(2 * i + 1))
    res[f'matching{k}'] = {'edges': len(M), 'components': len(comps), 'component sizes': dict(Counter(comps.values())),
                           'edges with both orientations in one component': both,
                           'edges wholly in a cap (per grip)': dict(full), 'chambers with partner outside (per grip)': dict(half),
                           'local letters swapping the ends of an edge': selfswap}
    print(k, res[f'matching{k}'], flush=True)
json.dump(res, open(sys.argv[5], 'w'), indent=1)
