"""Clear-and-raise search around a high floating cluster (menu I_a, exact simulator).

For a cluster K (component from an adj3 npz) and every grip T whose closed cap wholly holds K with a tree branch
other than K's: list the pieces that straddle plane T (float), then look for a word of at most two letters at caps
that do not touch K, after which plane T is clean (float pre-test, then exact), and then apply a jumble at T
(exact): K rises by 2.
usage: clear.py <research/jumbling> <pose table json> <adj npz> <cluster size> <out json>"""
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
tab = json.load(open(sys.argv[2])); z = np.load(sys.argv[3]); CS = int(sys.argv[4]); OUT = sys.argv[5]
NP = len(reg.orbit_of)
fm = lambda m: np.array([[float(v) for v in row] for row in m])
NF = np.array(d.NF, float); KAP = float(d.KAPPA)
orb = reg.orbit_of.astype(np.int64); tr = reg.transport.astype(np.int64)
Kf = np.array([fm(kp.matrix(i)) for i in range(7200)])
repV = [np.array([[float(x) for x in v] for v in vs]) for vs in reg.rep_vertices]
HV = [repV[orb[q]] @ Kf[tr[q]].T for q in range(NP)]
nv = np.array([len(v) for v in HV]); off_ = np.concatenate([[0], np.cumsum(nv)]); HVall = np.concatenate(HV)
owner = np.repeat(np.arange(NP), nv)
Pex = [matrix_from_json(m) for m in tab['poses']]
pid0 = np.array(tab['pose_id'], np.int64)
st = sim.State(ctx, menu=M)
st.pose_id[:] = np.array([st._intern_matrix(m) for m in Pex], np.int32)[pid0]
assert st.digest() == tab['digest']
JI = list(RS.jumble_items(M))
gbr = np.array([T.branch(T.factors(RS.element(M, c, JI[0]))[0]) for c in range(600)])
pfc = {}


def posed_all():
    pid = np.asarray(st.pose_id)
    U = np.unique(pid)
    Pm = np.zeros((int(U.max()) + 1, 4, 4))
    for i in U.tolist():
        if i not in pfc:
            pfc[i] = fm(st._pose_matrix(i))
        Pm[i] = pfc[i]
    return np.einsum('nij,nj->ni', Pm[pid[owner]], HVall)


def mm(vals):
    return np.minimum.reduceat(vals, off_[:-1], axis=0), np.maximum.reduceat(vals, off_[:-1], axis=0)


def lh(q):
    l, r = T.factors(st.pose(q))
    return T.dist(l), T.branch(l)


comp = z['comp']
cs = Counter(comp.tolist())
cid = [c for c, n in cs.items() if n == CS][0]
K = np.flatnonzero(comp == cid)
hK, bK = lh(int(K[0]))
X = posed_all()
vals = X @ NF.T - KAP
lo, hi = mm(vals)
holdK = np.flatnonzero(lo[K].min(0) > -1e-9)
targets = [int(t) for t in holdK if gbr[t] != bK]
print(json.dumps({'cluster': CS, 'height': hK, 'branch': int(bK), 'holding': holdK.tolist(), 'targets': targets}), flush=True)
strad = (lo < -1e-9) & (hi > 1e-9)
touchK = (hi[K] > 1e-9).any(0)          # caps whose open half-space meets K
res = []
for Tg in targets:
    S = np.flatnonzero(strad[:, Tg])
    hs = Counter(lh(int(q))[0] for q in S[:400])
    # caps F holding all straddlers wholly, not touching K, currently clean
    capsF = [F for F in range(600) if not touchK[F] and (lo[S, F] > -1e-9).all() and not strad[:, F].any()]
    found = []
    for F in capsF:
        inF = np.flatnonzero(lo[:, F] > -1e-9)
        idx = np.concatenate([np.arange(off_[q], off_[q + 1]) for q in inF.tolist()])
        for i in range(1, 60):
            E = fm(RS.element(M, F, i))
            v = (X[idx] @ E.T) @ NF[Tg] - KAP
            st_ = np.minimum.reduceat(v, np.concatenate([[0], np.cumsum(nv[inF])[:-1]]))
            sh_ = np.maximum.reduceat(v, np.concatenate([[0], np.cumsum(nv[inF])[:-1]]))
            if ((st_ < -1e-9) & (sh_ > 1e-9)).any():
                continue
            # exact: apply F, then a jumble at T
            o1 = st.apply(sim.Twist(ctx, F, RS.element(M, F, i), 'menu', {'menu': M.name}), journal=True)
            if not o1.applied:
                break
            o2 = st.apply(sim.Twist(ctx, Tg, RS.element(M, Tg, JI[0]), 'menu', {'menu': M.name}), journal=True)
            if o2.applied:
                h2 = lh(int(K[0]))[0]
                found.append({'F': F, 'i': i, 'T': Tg, 'height_after': h2})
                print(json.dumps(found[-1]), flush=True)
                st.undo()
            st.undo()
    res.append({'T': Tg, 'branch': int(gbr[Tg]), 'straddlers': len(S), 'straddler_heights': dict(hs),
                'single_F_caps': len(capsF), 'found': found})
    print(json.dumps({k: v for k, v in res[-1].items() if k != 'found'} | {'found': len(found), 'sec': round(time.time() - t0, 1)}), flush=True)
Path(OUT).write_text(json.dumps({'cluster': CS, 'height': hK, 'targets': res}, indent=1) + '\n')
