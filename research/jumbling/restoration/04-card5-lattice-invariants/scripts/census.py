"""Seed census: locate the off piece of every single-hole word in the depth-one source-cell atlas.
For each single (exactly one off piece p, pose X, left height 2, outer class Q), compute the class-Q signature of
the posed region X(A_p), the number of sources of that cell for the frame X K+ (posed regions X A_s), and the
number of homes in the cell (capacity). The (signature size, sources, capacity) triple identifies the registry
orbit, except that the two 360-cell orbits with 86/86 (registry orbits 2 and 5) are not separated here.
The inverse of the single word restores p from that source position on the pure single input.
usage: census.py <research/jumbling> <classes.json> <out json> <singles jsonl> [...]"""
import json, sys, time
from collections import Counter
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim, trees
import restore as RS
ctx = sim.get_context()
d, reg, kp = ctx.data, ctx.regions, ctx.kplus
M = RS.menu('I_a')
cls = {int(k): v for k, v in json.load(open(sys.argv[2])).items()}
OUT = sys.argv[3]
JI = list(RS.jumble_items(M))
cb = {}
for c in range(600):
    if cls[c] not in cb:
        cb[cls[c]] = trees.pose_tree(RS.element(M, c, JI[0]))[2]
bclass = {v: k for k, v in cb.items()}
fm = lambda m: np.array([[float(v) for v in row] for row in m])
Kf = np.array([fm(kp.matrix(i)) for i in range(7200)])
repV = [np.array([[float(v) for v in vert] for vert in vs]) for vs in reg.rep_vertices]
NF = np.array(d.NF, float); KAP = float(d.KAPPA); TOL = 1e-7
NP = len(reg.orbit_of)
HOME = [repV[int(reg.orbit_of[s])] @ Kf[int(reg.transport[s])].T for s in range(NP)]
def sig(V, cp):
    vals = V @ NF[cp].T - KAP
    ins = (vals > -TOL).all(0) & (vals > TOL).any(0)
    straddle = ~(ins | (vals < TOL).all(0))
    assert not straddle.any()
    return tuple(cp[ins].tolist())
REG = {(0, 5280, 5280): '0', (1, 385, 385): '1', (2, 86, 86): '2 or 5', (3, 111, 0): '3', (4, 87, 0): '4',
       (3, 111, 111): '6'}
capcache = {}
rows = []
TW = {}
def tw(l):
    if l not in TW:
        TW[l] = sim.Twist(ctx, l[0], RS.element(M, l[0], l[1]), 'menu', {'menu': M.name})
    return TW[l]
for path in sys.argv[4:]:
    for line in open(path):
        if not line.strip():
            continue
        r = json.loads(line)
        st = sim.State(ctx, menu=M)
        for l in r['word']:
            assert st.apply(tw(tuple(l))).applied
        off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
        assert len(off) == 1
        p = int(off[0])
        X = st.pose(p)
        dl, dr, bl, br = trees.pose_tree(X)
        Q = bclass[bl]
        cp = np.array([c for c in range(600) if cls[c] == Q])
        Xf = fm(X)
        s_p = sig(HOME[p] @ Xf.T, cp)
        if Q not in capcache:
            capcache[Q] = Counter(sig(V, cp) for V in HOME)
        nsrc = sum(1 for V in HOME if sig(V @ Xf.T, cp) == s_p)
        cap = capcache[Q][s_p]
        key = (len(s_p), nsrc, cap)
        row = {'file': Path(path).name, 'length': len(r['word']), 'piece': p, 'piece_orbit': int(d.orbit_id[p]),
               'height': int(dl), 'outer_class': int(Q), 'cell_signature': list(s_p), 'cell_sources': nsrc,
               'cell_capacity': int(cap), 'registry_orbit': REG.get(key, '?'), 'hole': r['holes'][0],
               'hole_orbit': r['hole_orbit'][0]}
        rows.append(row)
        print(row, flush=True)
json.dump(rows, open(OUT, 'w'), indent=1)
