"""Floating components of a configuration (from adj3.py output): caps that hold them, straddle status, tree branches.
usage: flt.py <research/jumbling> <pose table json> <adj npz> <out json>"""
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
tab = json.load(open(sys.argv[2]))
z = np.load(sys.argv[3])
OUT = sys.argv[4]
NP = len(reg.orbit_of)
fm = lambda m: np.array([[float(v) for v in row] for row in m])
NF = np.array(d.NF, float); KAP = float(d.KAPPA)
orb = reg.orbit_of.astype(np.int64); tr = reg.transport.astype(np.int64)
Kf = np.array([fm(kp.matrix(i)) for i in range(7200)])
Pex = [matrix_from_json(m) for m in tab['poses']]
Pf = np.array([fm(m) for m in Pex])
pid = np.array(tab['pose_id'], np.int64)
repV = [np.array([[float(x) for x in v] for v in vs]) for vs in reg.rep_vertices]
st = sim.State(ctx, menu=M)
mp_ = [st._intern_matrix(m) for m in Pex]
st.pose_id[:] = np.array(mp_, np.int32)[pid]
assert st.digest() == tab['digest'], 'state rebuild'
# grip branches
JI = list(RS.jumble_items(M))
gbr = []
for c in range(600):
    l, r = T.factors(RS.element(M, c, JI[0]))
    gbr.append(T.branch(l))
lb = []
for m in Pex:
    l, r = T.factors(m)
    lb.append((T.dist(l), T.branch(l)))
comp, anch, hp = z['comp'], z['anch'], z['hp']
cs = Counter(comp.tolist())
main = cs.most_common(1)[0][0]
res = []
cls_cache = {}
for c_, n in cs.most_common():
    if c_ == main:
        continue
    mem = np.flatnonzero(comp == c_)
    if anch[mem].any():
        continue
    # posed vertices of all members
    lo = np.full(600, np.inf); hi = np.full(600, -np.inf)
    for q in mem.tolist():
        V = repV[orb[q]] @ Kf[tr[q]].T @ Pf[pid[q]].T
        vals = V @ NF.T - KAP
        lo = np.minimum(lo, vals.min(0)); hi = np.maximum(hi, vals.max(0))
    whole = np.flatnonzero(lo > 1e-7).tolist()
    amb = np.flatnonzero((np.abs(lo) <= 1e-7)).tolist()
    part = np.flatnonzero((hi > 1e-7) & (lo < -1e-7)).tolist()
    caps = []
    for g in whole + amb:
        if g not in cls_cache:
            cl = st.classify(g, need_inside=True, stop_at_straddle=True)
            cls_cache[g] = None if cl.straddle is None else int(cl.straddle['piece'])
        caps.append({'grip': g, 'branch': gbr[g], 'straddled_by': cls_cache[g], 'float_margin_ambiguous': g in amb})
    br = Counter((int(lb[pid[q]][0]), int(lb[pid[q]][1])) for q in mem.tolist())
    res.append({'size': int(n), 'heights_branches': {f'{h}/{b}': k for (h, b), k in sorted(br.items())},
                'orbits': dict(Counter(d.orbit_id[mem].tolist())), 'caps_wholly': caps, 'caps_partly': part[:40],
                'n_caps_partly': len(part), 'members_head': mem[:12].tolist()})
    print(json.dumps({k: v for k, v in res[-1].items() if k not in ('orbits', 'caps_partly', 'members_head')}), flush=True)
Path(OUT).write_text(json.dumps({'digest': tab['digest'], 'grip_branch_counts': dict(Counter(gbr)),
                                 'unanchored_floating': res, 'sec': round(time.time() - t0, 1)}, indent=1) + '\n')
