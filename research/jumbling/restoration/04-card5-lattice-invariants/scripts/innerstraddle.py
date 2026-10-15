"""After one jumble (0, x) from solved: which grips are straddled by the inner cohort (anchored inner pieces, orbits
0,1,2,3,6) and by each orbit's off pieces? Float with margin, exact at near ties. Groups the 48 jumble items by the
left coset x A4_0. usage: innerstraddle.py <research/jumbling> <out json>"""
import json, sys, time
from collections import Counter, defaultdict
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
import restore as RS
from exact import matmul, transpose
from sim.kplus import to_tuple
ctx = sim.get_context(); d = ctx.data; reg = ctx.regions; kp = ctx.kplus
M = RS.menu('I_a')
NF = np.array([[float(v) for v in row] for row in d.N])
NN = float(d.NN)
KAP = float(d.KAPPA)
ALPHA_NN = KAP   # h_e(x) sign = n_e . x - kappa (kappa = alpha |n|^2)
ang = np.degrees(np.arccos(np.clip(NF @ NF[0] / NN, -1, 1)))
INNER = {0, 1, 2, 3, 6}
oid = np.asarray(d.orbit_id)
members = [int(p) for p in d.cap_members(0)]
items = list(RS.jumble_items(M))
mats = {x: RS.element(M, 0, x) for x in items}
# cosets x A4_0
coset = {}
reps = []
for x in items:
    for r in reps:
        if kp.index_of_matrix(to_tuple(matmul(transpose(mats[r]), mats[x]))) >= 0:
            coset[x] = r
            break
    else:
        reps.append(x); coset[x] = x
print('cosets', len(reps), Counter(coset.values()))
MARG = 1e-9
def straddled_by(G, pieces):
    """grips straddled by some piece of `pieces` posed by G (float, exact at ties)."""
    Gf = np.array([[float(v) for v in row] for row in G])
    res = defaultdict(set)
    for p in pieces:
        V = reg.vertices(p)
        Vf = np.array([[float(v) for v in x] for x in V]) @ Gf.T
        vals = Vf @ NF.T - KAP
        mx, mn = vals.max(axis=0), vals.min(axis=0)
        cand = np.nonzero((mx > -MARG) & (mn < MARG))[0]
        for e in cand.tolist():
            if mx[e] > MARG and mn[e] < -MARG:
                res[e].add(p); continue
            # exact at ties
            Vx = [tuple(sum((G[a][b] * x[b] for b in range(4)), d.N[0][0] * 0) for a in range(4)) for x in V]
            hs = [d.h(e, x).sign() for x in Vx]
            if min(hs) < 0 < max(hs):
                res[e].add(p)
    return res
out = {}
t0 = time.time()
for r in reps:
    G = mats[r]
    inner = [p for p in members if int(oid[p]) in INNER]
    st_in = straddled_by(G, inner)
    allp = straddled_by(G, members)
    tal_in = Counter(round(float(ang[e]), 2) for e in st_in if e != 0)
    tal_all = Counter(round(float(ang[e]), 2) for e in allp if e != 0)
    out[r] = {'inner_straddled': sorted(int(e) for e in st_in if e != 0),
              'all_straddled': sorted(int(e) for e in allp if e != 0),
              'inner_by_angle': dict(sorted(tal_in.items())), 'all_by_angle': dict(sorted(tal_all.items()))}
    print('coset rep', r, 'inner', dict(sorted(tal_in.items())), 'all', dict(sorted(tal_all.items())),
          '%.0fs' % (time.time() - t0), flush=True)
# coset invariance check on all items (inner straddles)
inv_ok = True
for x in items:
    if x in reps:
        continue
    st_in = straddled_by(mats[x], [p for p in members if int(oid[p]) in INNER])
    if sorted(int(e) for e in st_in if e != 0) != out[coset[x]]['inner_straddled']:
        inv_ok = False
print('coset invariance of inner straddles', inv_ok)
json.dump({'reps': reps, 'coset_of_item': {str(k): v for k, v in coset.items()}, 'by_coset': {str(k): v for k, v in out.items()},
           'coset_invariant': inv_ok}, open(sys.argv[2], 'w'), indent=1)
