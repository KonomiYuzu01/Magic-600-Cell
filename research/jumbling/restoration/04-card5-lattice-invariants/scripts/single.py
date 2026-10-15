"""Climb floating pieces of a configuration (menu I_a, exact simulator).

For each piece with no detected 3-face neighbour (adj3 npz degree 0): depth-first over raises. At each node, the
grips whose closed caps wholly hold the piece (float test, margin 1e-9) and whose tree branch differs from the
piece's left branch are classified exactly; at an unstraddled one, every jumble coset (4 left results) is applied
and the search recurses. Reports the best left height per piece and the blocking statistics.
usage: single.py <research/jumbling> <pose table json> <adj npz> <max depth> <out json>"""
import json, sys, time
from collections import Counter
from pathlib import Path
import numpy as np

J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
import restore as RS
from sim.kplus import matrix_from_json
import trees as T

t0 = time.time()
ctx = sim.get_context()
d, reg, kp = ctx.data, ctx.regions, ctx.kplus
M = RS.menu('I_a')
tab = json.load(open(sys.argv[2])); z = np.load(sys.argv[3]); MAXD = int(sys.argv[4]); OUT = sys.argv[5]
NP = len(reg.orbit_of)
fm = lambda m: np.array([[float(v) for v in row] for row in m])
NF = np.array(d.NF, float); KAP = float(d.KAPPA)
orb = reg.orbit_of.astype(np.int64); tr = reg.transport.astype(np.int64)
Pex = [matrix_from_json(m) for m in tab['poses']]
pid0 = np.array(tab['pose_id'], np.int64)
st = sim.State(ctx, menu=M)
st.pose_id[:] = np.array([st._intern_matrix(m) for m in Pex], np.int32)[pid0]
assert st.digest() == tab['digest']
JI = list(RS.jumble_items(M))
EL = {}


def el(c, i):
    if (c, i) not in EL:
        EL[(c, i)] = RS.element(M, c, i)
    return EL[(c, i)]


gbr = [T.branch(T.factors(el(c, JI[0]))[0]) for c in range(600)]
# one jumble per left coset at each grip: group jumble items by the left vertex y_L v0
cos = {}
for c in range(600):
    seen = {}
    for i in JI:
        l, r = T.factors(el(c, i))
        k = json.dumps([str(x) for x in T.qmul(l, (1, 0, 0, 0))])
        # left vertex label: use the tree action on the root via dist of l times a fixed probe
        key = T.dist(l), str(sorted(str(x) for x in l))
        seen.setdefault(i, i)
    cos[c] = JI
deg = np.zeros(NP, int)
for a, b in z['edges']:
    deg[a] += 1; deg[b] += 1
V0 = {}


def posedV(q):
    if q not in V0:
        V0[q] = np.array([[float(x) for x in v] for v in reg.rep_vertices[orb[q]]]) @ fm(kp.matrix(int(tr[q]))).T
    return V0[q] @ fm(st.pose(q)).T


def lh(q):
    l, r = T.factors(st.pose(q))
    return T.dist(l), T.branch(l)


stats = Counter()


def climb(q, depth, best, path):
    h, b = lh(q)
    if h > best[0]:
        best[0] = h; best[1] = list(path)
    if depth >= MAXD:
        return
    V = posedV(q)
    lo = (V @ NF.T - KAP).min(0)
    hold = [c for c in np.flatnonzero(lo > -1e-9).tolist() if gbr[c] != b or h == 0]
    for c in hold:
        cl = st.classify(c, need_inside=False, stop_at_straddle=True)
        if cl.straddle is not None:
            stats['blocked'] += 1
            continue
        stats['open'] += 1
        lefts = set()
        for i in JI:
            l, r = T.factors(el(c, i))
            out = st.apply(sim.Twist(ctx, c, el(c, i), 'menu', {'menu': M.name}), journal=True)
            if not out.applied:
                break
            l2, r2 = T.factors(st.pose(q))
            key = json.dumps([str(x) for x in l2])
            if T.dist(l2) > h and key not in lefts and len(lefts) < 4:
                lefts.add(key)
                path.append((c, i)); climb(q, depth + 1, best, path); path.pop()
            st.undo()


res = []
for q in np.flatnonzero(deg == 0).tolist():
    best = [0, []]
    climb(q, 0, best, [])
    h0, b0 = lh(q)
    res.append({'piece': q, 'orbit': int(d.orbit_id[q]), 'h0': h0, 'best': best[0], 'path': best[1]})
    print(json.dumps(res[-1] | {'stats': dict(stats), 'sec': round(time.time() - t0, 1)}), flush=True)
Path(OUT).write_text(json.dumps({'results': res, 'stats': dict(stats)}, indent=1) + '\n')
