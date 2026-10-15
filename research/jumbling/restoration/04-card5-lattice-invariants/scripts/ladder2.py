"""Zero-residue commutator ladder for a single off-lattice piece.
Base: a word V admissible from solved whose end state has exactly one off-lattice piece p (left height h).
Search a commutator W = x k x^-1 k^-1, admissible after V, such that after V W the only off-lattice piece is p,
at left height h + 2:
  x = (d, jumble) with p wholly in cap d and class(d) != outer class of p (so x raises p by 2, Lemma 5.2);
  k = (e, item) with p wholly in cap e after x, no other off-lattice piece in cap e, and k carrying p wholly
      outside cap d (float pre-test only; the exact simulator decides everything).
Then x^-1 misses p and k^-1 returns p to its raised pose x P. Every success is replayed from solved.
usage: ladder2.py <research/jumbling> <classes.json> <base json> <out json> <budget s> <steps> [items: retained|all]"""
import json, sys, time
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim, trees
import restore as RS
from exact import transpose
from sim.kplus import to_tuple
ctx = sim.get_context()
d, reg, kp = ctx.data, ctx.regions, ctx.kplus
M = RS.menu('I_a')
cls = {int(k): v for k, v in json.load(open(sys.argv[2])).items()}
base = json.load(open(sys.argv[3]))
OUT, budget, steps = sys.argv[4], float(sys.argv[5]), int(sys.argv[6])
mode = sys.argv[7] if len(sys.argv) > 7 else 'retained'
JI = list(RS.jumble_items(M))
ident = RS.element(M, 0, 0)
RI = [i for i in range(len(M.items)) if i not in JI and RS.element(M, 0, i) != ident]
KI_items = RI if mode == 'retained' else [i for i in range(len(M.items)) if RS.element(M, 0, i) != ident]
cb = {}
for c in range(600):
    if cls[c] not in cb:
        cb[cls[c]] = trees.pose_tree(RS.element(M, c, JI[0]))[2]
bclass = {v: k for k, v in cb.items()}
NF = np.array([[float(v) for v in row] for row in d.N]); NN = NF / np.linalg.norm(NF, axis=1)[:, None]
KAP = float(d.KAPPA)
inv_cache, TW, FM = {}, {}, {}
def inv(l):
    if l not in inv_cache:
        c, i = l
        T = to_tuple(transpose(RS.element(M, c, i)))
        inv_cache[l] = (c, next(j for j in range(len(M.items)) if RS.element(M, c, j) == T))
    return inv_cache[l]
def tw(l):
    if l not in TW:
        TW[l] = sim.Twist(ctx, l[0], RS.element(M, l[0], l[1]), 'menu', {'menu': M.name})
    return TW[l]
def fmat(m):
    return np.array([[float(v) for v in row] for row in m])
def fm(l):
    if l not in FM:
        FM[l] = fmat(RS.element(M, l[0], l[1]))
    return FM[l]
V = [tuple(l) for l in base['word']]
st = sim.State(ctx, menu=M)
for l in V:
    assert st.apply(tw(l)).applied
off0 = np.flatnonzero(st._kpose()[st.pose_id] < 0)
assert len(off0) == 1, off0
p = int(off0[0])
HV = np.array([[float(v) for v in vert] for vert in reg.vertices(p)])
def pverts():
    return HV @ fmat(st.pose(p)).T
def height():
    dl, dr, bl, br = trees.pose_tree(st.pose(p))
    return dl, bclass.get(bl)
t0 = time.time()
res = {'piece': p, 'orbit': int(d.orbit_id[p]), 'steps': []}
for step in range(steps):
    h0, outer = height()
    print('step', step, 'piece', p, 'height', h0, 'outer class', outer, 'word length', len(V), flush=True)
    pv = pverts(); cen = pv.mean(0); cen /= np.linalg.norm(cen)
    near_d = [c for c in np.argsort(-(NN @ cen))[:40].tolist() if cls[c] != outer]
    ds = []
    for c in near_d:
        if (pv @ NF[c] > KAP + 1e-9).all():
            cl = st.classify(c, need_inside=True, stop_at_straddle=True)
            if cl.straddle is None and not cl.uncertain and p in set(cl.inside.tolist()):
                ds.append(c)
    print('  catching grips', ds, '%.0fs' % (time.time() - t0), flush=True)
    found = None
    tried = 0
    for c in ds:
        for xi in JI:
            if found or time.time() - t0 > budget:
                break
            x = (c, xi)
            assert st.apply(tw(x)).applied
            if height()[0] != h0 + 2:
                assert st.apply(tw(inv(x))).applied
                continue
            pv1 = pverts(); cen1 = pv1.mean(0); cen1 /= np.linalg.norm(cen1)
            offset = set(np.flatnonzero(st._kpose()[st.pose_id] < 0).tolist())
            for e in np.argsort(-(NN @ cen1))[:30].tolist():
                if e == c or not (pv1 @ NF[e] > KAP + 1e-9).all():
                    continue
                cl = st.classify(e, need_inside=True, stop_at_straddle=True)
                if cl.straddle is not None or cl.uncertain:
                    continue
                ins = set(cl.inside.tolist())
                if p not in ins or (ins & offset) != {p}:
                    continue
                for ki in KI_items:
                    k = (e, ki)
                    pv2 = pv1 @ fm(k).T
                    if not (pv2 @ NF[c] < KAP - 1e-9).all():
                        continue
                    W = [k, inv(x), inv(k)]
                    n = 0
                    for l in W:
                        if not st.apply(tw(l)).applied:
                            break
                        n += 1
                    tried += 1
                    ok = False
                    if n == len(W):
                        offn = np.flatnonzero(st._kpose()[st.pose_id] < 0)
                        ok = len(offn) == 1 and int(offn[0]) == p and height()[0] == h0 + 2
                    for l in reversed(W[:n]):
                        assert st.apply(tw(inv(l))).applied
                    if ok:
                        found = [x] + W
                        break
                if found:
                    break
            assert st.apply(tw(inv(x))).applied
        if found:
            break
    print('  tried', tried, 'found', found, '%.0fs' % (time.time() - t0), flush=True)
    if not found:
        break
    for l in found:
        assert st.apply(tw(l)).applied
    V = V + found
    res['steps'].append({'height': h0 + 2, 'word_tail': [list(l) for l in found], 'length': len(V)})
    res['word'] = [list(l) for l in V]
    json.dump(res, open(OUT, 'w'))
# independent replay from solved
if res['steps']:
    st2 = sim.State(ctx, menu=M)
    for l in V:
        assert st2.apply(tw(l)).applied
    offn = np.flatnonzero(st2._kpose()[st2.pose_id] < 0)
    dl = trees.pose_tree(st2.pose(p))[0]
    res['replay'] = {'off': offn.tolist(), 'height': int(dl), 'length': len(V)}
    json.dump(res, open(OUT, 'w'))
    print('REPLAY off', offn.tolist(), 'height', dl, 'length', len(V), flush=True)
