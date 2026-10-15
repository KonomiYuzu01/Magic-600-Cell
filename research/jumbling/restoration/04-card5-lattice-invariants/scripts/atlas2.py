"""Atlas refinement: (1) orbits of the source cells under the frame-compatible group
H_P(gamma) = {k in H_P : gamma^-1 k gamma in K+}, which maps posed regions of the frame gamma K+ to posed regions
of the same frame; (2) a Frank-Wolfe distance test between each undecided empty-cell posed region and the core
alpha*P (shrunk by 1e-6 so that touching is not counted as intersection).
usage: atlas2.py <research/jumbling> <classes.json> <c0> <atlas json> <out json>"""
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
ctx = sim.get_context()
d, reg, kp = ctx.data, ctx.regions, ctx.kplus
M = RS.menu('I_a')
cls = {int(k): v for k, v in json.load(open(sys.argv[2])).items()}
c0 = int(sys.argv[3]); OUT = sys.argv[5]
t0 = time.time()
P = cls[c0]
classP = np.array([c for c in range(600) if cls[c] == P])
JI = list(RS.jumble_items(M))
fm = lambda m: np.array([[float(v) for v in row] for row in m])
G = fm(RS.element(M, c0, JI[0])); Gi = G.T
Kf = np.array([fm(kp.matrix(i)) for i in range(7200)])
repV = [np.array([[float(v) for v in vert] for vert in vs]) for vs in reg.rep_vertices]
NF = np.array(d.NF, float)
KAP = float(d.KAPPA)
NP = len(reg.orbit_of)
TOL = 1e-7
NFn = NF / np.linalg.norm(NF, axis=1)[:, None]
def in_kplus(R):
    img = NFn @ R.T
    return np.allclose(np.sort(np.abs(img @ NFn.T).max(1)), 1.0, atol=1e-9)
perms = kp.perms.astype(np.int64)
Pset = set(classP.tolist())
HP = [k for k in range(7200) if set(perms[k, classP].tolist()) == Pset]
HPg = [k for k in HP if in_kplus(Gi @ Kf[k] @ G)]
print('H_P', len(HP), 'frame-compatible', len(HPg), '%.0fs' % (time.time() - t0), flush=True)
def sig_of(V):
    vals = V @ NF[classP].T - KAP
    ins = (vals > -TOL).all(0) & (vals > TOL).any(0)
    return tuple(classP[ins].tolist())
cell_src, cell_cap, src_cell = Counter(), Counter(), {}
for s in range(NP):
    o, k = int(reg.orbit_of[s]), int(reg.transport[s])
    V = repV[o] @ Kf[k].T
    sg = sig_of(V @ G.T)
    cell_src[sg] += 1; src_cell[s] = sg
    cell_cap[sig_of(V)] += 1
cells = sorted(cell_src, key=lambda t: (len(t), t))
idx = {c: i for i, c in enumerate(cells)}
orbit = [-1] * len(cells); orbits = []
for c in cells:
    if orbit[idx[c]] >= 0:
        continue
    mem = set()
    for k in HPg:
        img = tuple(sorted(perms[k, list(c)].tolist()))
        assert img in idx, 'source cell image is not a source cell'
        mem.add(img)
    for m in mem:
        orbit[idx[m]] = len(orbits)
    orbits.append(sorted(mem))
rows = []
for oid, mem in enumerate(orbits):
    rows.append({'orbit': oid, 'cells': len(mem), 'signature_size': len(mem[0]),
                 'sources_per_cell': sorted({cell_src[m] for m in mem}),
                 'capacity_per_cell': sorted({cell_cap.get(m, 0) for m in mem})})
    print(rows[-1], flush=True)
# core test
core_v = None
# core vertices: alpha * (600-cell vertices). The 600-cell P has facets n.x = |n|^2; its vertices are where
# 20 facets meet; get them as the vertices of the solved region union -- use the radial direction of 2I:
# take every home vertex at maximal norm (these are the polytope vertices), scale by alpha.
allV = np.concatenate([repV[o] @ Kf[k].T for o, k in ((int(reg.orbit_of[s]), int(reg.transport[s])) for s in range(NP))])
nr = np.linalg.norm(allV, axis=1); Rmax = nr.max()
cand = allV[nr > Rmax - 1e-6]
pv = []
for x in cand:
    if all(np.linalg.norm(x - y) > 1e-6 for y in pv):
        pv.append(x)
pv = np.array(pv)
alpha = 121 / 125
core_v = alpha * (1 - 1e-6) * pv
print('polytope vertices found', len(pv), 'circumradius', Rmax, flush=True)
def fw_dist(A, B, it=200000):
    """distance between conv(A) and conv(B) by Frank-Wolfe on the Minkowski difference; returns
    ('sep', margin) when a separating direction is certified, ('touch', dist) otherwise."""
    z = A[0] - B[0]
    for _ in range(it):
        sa = A[np.argmin(A @ z)]; sb = B[np.argmax(B @ z)]
        s = sa - sb
        if z @ s > 1e-12:
            return 'sep', float(z @ s / np.linalg.norm(z))
        dz = s - z
        den = dz @ dz
        if den < 1e-30:
            break
        lam = min(1.0, max(0.0, -(z @ dz) / den))
        z = z + lam * dz
        if np.linalg.norm(z) < 1e-10:
            return 'int', 0.0
    return 'touch', float(np.linalg.norm(z))
empty = [s for s, sg in src_cell.items() if len(sg) == 0]
res = Counter(); mind = []
for s in empty:
    o, k = int(reg.orbit_of[s]), int(reg.transport[s])
    V = (repV[o] @ Kf[k].T) @ G.T
    vals = V @ NF.T - KAP
    if ((vals < -TOL).all(1)).any():
        res['vertex_inside'] += 1
        continue
    if ((vals > -TOL).all(0)).any():
        res['facet_separated'] += 1
        continue
    r, x = fw_dist(V, core_v)
    res['fw_' + r] += 1
    if r != 'sep':
        mind.append(x)
print('empty cell', len(empty), dict(res), 'max residual distance among non-separated', max(mind) if mind else None,
      '%.0fs' % (time.time() - t0), flush=True)
json.dump({'class': int(P), 'grip': c0, 'H_P': len(HP), 'H_P_frame_compatible': len(HPg), 'orbits': rows,
           'empty_cell': {'sources': len(empty), **dict(res)}}, open(OUT, 'w'), indent=1)
