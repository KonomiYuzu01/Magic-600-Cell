"""Point-location probe: for pieces with few detected 3-face neighbours, step off each posed facet and find the
piece that contains the probe point. Floating point; diagnostic only.
usage: ploc.py <research/jumbling> <pose table json> <adj npz> <out json>"""
import json, sys, time
from collections import Counter
from pathlib import Path
import numpy as np

J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
from sim.kplus import matrix_from_json
import trees as T

t0 = time.time()
ctx = sim.get_context()
d, reg, kp = ctx.data, ctx.regions, ctx.kplus
tab = json.load(open(sys.argv[2])); z = np.load(sys.argv[3]); OUT = sys.argv[4]
NP = len(reg.orbit_of)
fm = lambda m: np.array([[float(v) for v in row] for row in m])
NF = np.array(d.NF, float); NN = float(d.NN); KAP = float(d.KAPPA); RIN = NN ** 0.5; NFu = NF / RIN
orb = reg.orbit_of.astype(np.int64); tr = reg.transport.astype(np.int64)
Kf = np.array([fm(kp.matrix(i)) for i in range(7200)])
Pex = [matrix_from_json(m) for m in tab['poses']]
Pf = np.array([fm(m) for m in Pex]); pid = np.array(tab['pose_id'], np.int64)
repV = [np.array([[float(x) for x in v] for v in vs]) for vs in reg.rep_vertices]
# rep H-rep (all facets, any plane): from the cut/body planes tight on >= 4 affinely independent vertices
RU, RD, RK = [], [], []
for o, V in enumerate(repV):
    us, ds, ks = [], [], []
    for kind, off in (('cut', KAP), ('body', NN)):
        vals = V @ NF.T - off
        for c in range(600):
            t = np.abs(vals[:, c]) < 1e-7
            if t.sum() < 4 or np.linalg.matrix_rank(V[t] - V[t][0], tol=1e-9) != 3:
                continue
            rest = vals[~t, c]
            if kind == 'body':
                us.append(NFu[c]); ds.append(RIN); ks.append(('body', c))
            elif (rest > 0).all():
                us.append(-NFu[c]); ds.append(-KAP / RIN); ks.append(('cut-in', c))
            else:
                us.append(NFu[c]); ds.append(KAP / RIN); ks.append(('cut-out', c))
    RU.append(np.array(us)); RD.append(np.array(ds)); RK.append(ks)
    # sanity: every rep vertex satisfies all constraints
    assert (V @ np.array(us).T - np.array(ds) < 1e-7).all()
Mq = np.einsum('qij,qjk->qik', Pf[pid], Kf[tr])
# posed vertices and bounding boxes
PV = [repV[orb[q]] @ Mq[q].T for q in range(NP)]
lo = np.array([v.min(0) for v in PV]); hi = np.array([v.max(0) for v in PV])
lf = [T.factors(m)[0] for m in Pex]
hp = np.array([T.dist(l) for l in lf])[pid]
deg = np.zeros(NP, int)
for a, b in z['edges']:
    deg[a] += 1; deg[b] += 1


def locate(x):
    cand = np.flatnonzero(np.all((lo - 1e-9 <= x) & (x <= hi + 1e-9), axis=1))
    hits = []
    for q in cand.tolist():
        U = RU[orb[q]] @ Mq[q].T
        if (U @ x - RD[orb[q]] < 1e-9).all():
            hits.append(q)
    return hits


targets = np.flatnonzero(deg <= 1).tolist()
comp = z['comp']; cs = Counter(comp.tolist()); main = cs.most_common(1)[0][0]
cl78 = [c for c, n in cs.items() if n == 78]
if cl78:
    targets += np.flatnonzero(comp == cl78[0])[:6].tolist()
res = []
for q in targets[:60]:
    V = PV[q]; U = RU[orb[q]] @ Mq[q].T; D = RD[orb[q]]
    rows = []
    for k in range(len(U)):
        tight = np.abs(V @ U[k] - D[k]) < 1e-7
        cen = V[tight].mean(0)
        x = cen + 1e-6 * U[k]
        inbody = True
        incore = (NFu @ x <= KAP / RIN + 1e-12).all()
        hits = locate(x) if inbody and not incore else []
        # does the hit piece have an opposite facet on the same hyperplane?
        info = []
        for h in hits:
            Uh = RU[orb[h]] @ Mq[h].T; Dh = RD[orb[h]]
            same = np.flatnonzero((np.abs(Uh + U[k]).max(1) < 1e-9) & (np.abs(Dh + D[k]) < 1e-9))
            l1 = lf[pid[q]]; l2 = lf[pid[h]]
            info.append({'piece': h, 'height': int(hp[h]), 'coplanar_facet': len(same) > 0,
                         'left_dist': T.dist(T.qmul(T.qconj(l1), l2)), 'comp_same': bool(comp[h] == comp[q])})
        rows.append({'facet': list(RK[orb[q]][k]), 'outside_body': bool(not inbody), 'in_core': bool(incore), 'hits': info})
    res.append({'piece': q, 'orbit': int(d.orbit_id[q]), 'height': int(hp[q]), 'deg': int(deg[q]), 'facets': rows})
    print(json.dumps({'piece': q, 'height': int(hp[q]), 'deg': int(deg[q]),
                      'facets': [(r['facet'][0], r['outside_body'], r['in_core'],
                                  [(h['piece'], h['height'], h['coplanar_facet'], h['left_dist']) for h in r['hits']]) for r in rows]}),
          flush=True)
Path(OUT).write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps({'done': True, 'sec': round(time.time() - t0, 1)}))
