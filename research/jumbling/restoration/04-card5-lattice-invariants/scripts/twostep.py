"""Two-twist raise search around high clusters of a configuration (menu I_a), exact admissibility.

For every component of the 3-face graph (adj3 output) whose pieces have left height >= HMIN and every unstraddled
grip c whose closed cap wholly holds the component: apply each non-identity element of Lambda_c (exact simulator),
then for every grip c' whose cap wholly holds the moved component and whose tree branch differs from the
component's new left branch, test c' exactly; an unstraddled c' gives a raising second twist, which is applied
and the new left height recorded.
usage: twostep.py <research/jumbling> <pose table json> <adj npz> <hmin> <out json>"""
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
HMIN = int(sys.argv[4]); OUT = sys.argv[5]
fm = lambda m: np.array([[float(v) for v in row] for row in m])
NF = np.array(d.NF, float); KAP = float(d.KAPPA)
orb = reg.orbit_of.astype(np.int64); tr = reg.transport.astype(np.int64)
Kf = np.array([fm(kp.matrix(i)) for i in range(7200)])
repV = [np.array([[float(x) for x in v] for v in vs]) for vs in reg.rep_vertices]
Pex = [matrix_from_json(m) for m in tab['poses']]
pid0 = np.array(tab['pose_id'], np.int64)
st = sim.State(ctx, menu=M)
mp_ = [st._intern_matrix(m) for m in Pex]
st.pose_id[:] = np.array(mp_, np.int32)[pid0]
assert st.digest() == tab['digest']
gbr = json.load(open(Path(sys.argv[3]).parent / 'grip-branch.json'))
JI = set(RS.jumble_items(M))
hcache = {}


def lh(q):
    m = st.pose(q)
    k = json.dumps([[str(x) for x in r] for r in m])
    if k not in hcache:
        l, r = T.factors(m)
        hcache[k] = (T.dist(l), T.branch(l))
    return hcache[k]


def holding(mem):
    lo = np.full(600, np.inf)
    for q in mem:
        V = repV[orb[q]] @ Kf[tr[q]].T @ fm(st.pose(q)).T
        lo = np.minimum(lo, (V @ NF.T - KAP).min(0))
    return np.flatnonzero(lo > -1e-7).tolist()


def unstraddled(c):
    cl = st.classify(c, need_inside=True, stop_at_straddle=True)
    return cl.straddle is None


comp, hp = z['comp'], z['hp']
cs = Counter(comp.tolist())
main = cs.most_common(1)[0][0]
clusters = []
for c_, n in cs.items():
    if c_ == main:
        continue
    mem = np.flatnonzero(comp == c_)
    if hp[mem].min() >= HMIN:
        clusters.append(mem.tolist())
print(json.dumps({'clusters': len(clusters), 'sizes': sorted(len(m) for m in clusters), 'sec': round(time.time() - t0, 1)}),
      flush=True)
found = []; tried = Counter()
for mem in clusters:
    h0, b0 = lh(mem[0])
    caps = [c for c in holding(mem) if unstraddled(c)]
    for c in caps:
        for i in range(1, 60):
            tw = sim.Twist(ctx, c, RS.element(M, c, i), 'menu', {'menu': M.name})
            out = st.apply(tw, journal=True)
            if not out.applied:
                tried['first_blocked'] += 1
                continue
            tried['first'] += 1
            h1, b1 = lh(mem[0])
            for c2 in holding(mem):
                if gbr[c2] == b1 or c2 == c:
                    continue
                tried['second_candidates'] += 1
                if not unstraddled(c2):
                    tried['second_straddled'] += 1
                    continue
                for j in sorted(JI)[:1]:
                    tw2 = sim.Twist(ctx, c2, RS.element(M, c2, j), 'menu', {'menu': M.name})
                    o2 = st.apply(tw2, journal=True)
                    if o2.applied:
                        h2, b2 = lh(mem[0])
                        found.append({'cluster_size': len(mem), 'h0': h0, 'first': [c, i], 'h1': h1, 'second': [c2, j],
                                      'h2': h2})
                        print(json.dumps(found[-1]), flush=True)
                        st.undo()
            st.undo()
    print(json.dumps({'cluster': len(mem), 'h0': h0, 'caps': caps, **tried, 'found': len(found),
                      'sec': round(time.time() - t0, 1)}), flush=True)
Path(OUT).write_text(json.dumps({'found': found, 'tried': dict(tried)}, indent=1) + '\n')
