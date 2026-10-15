"""Independent recomputation of the depth-one source-cell atlas (3E registry data).
For one class P (the class of grip c0) and one frame gamma = a jumble element of Lambda_c0, every home region
A_s (177120 chambers) is posed as gamma(A_s); its class-P signature (the class-P caps that contain it) is read
from its vertices (floating point with a margin; any vertex within the margin is reported). Cells are the
distinct signatures; 'sources' count posed regions per cell, 'capacity' counts unposed home regions per cell.
Cells are grouped into orbits under H_P = {k in K+ : k preserves class P}. The core test checks each posed
region of the empty cell against the core alpha*P (vertex strictly inside the core: intersects; one core facet
separating all vertices: disjoint; otherwise undecided).
usage: atlas.py <research/jumbling> <classes.json> <c0> <out json>"""
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
ctx = sim.get_context()
d, reg, kp = ctx.data, ctx.regions, ctx.kplus
M = RS.menu('I_a')
cls = {int(k): v for k, v in json.load(open(sys.argv[2])).items()}
c0 = int(sys.argv[3]); OUT = sys.argv[4]
t0 = time.time()
P = cls[c0]
classP = np.array([c for c in range(600) if cls[c] == P])
JI = list(RS.jumble_items(M))
fm = lambda m: np.array([[float(v) for v in row] for row in m])
G = fm(RS.element(M, c0, JI[0]))
Kf = np.array([fm(kp.matrix(i)) for i in range(7200)])
repV = [np.array([[float(v) for v in vert] for vert in vs]) for vs in reg.rep_vertices]
NF = d.NF.astype(float) if hasattr(d.NF, 'astype') else np.array(d.NF, float)
KAP = float(d.KAPPA)
NP = len(reg.orbit_of)
print('pieces', NP, 'class', P, 'grips', len(classP), 'rep orbits', len(repV), '%.0fs' % (time.time() - t0), flush=True)
TOL = 1e-7
def sig_of(V):
    vals = V @ NF[classP].T - KAP           # (nv, 120)
    ins = (vals > -TOL).all(0) & (vals > TOL).any(0)
    outs = (vals < TOL).all(0)
    amb = ~(ins | outs)
    return tuple(classP[ins].tolist()), int(amb.sum())
cell_src, cell_cap = Counter(), Counter()
src_cell = {}
amb_src = amb_cap = 0
for s in range(NP):
    o, k = int(reg.orbit_of[s]), int(reg.transport[s])
    V = repV[o] @ Kf[k].T
    sg, a = sig_of(V @ G.T)
    amb_src += a
    cell_src[sg] += 1
    src_cell[s] = sg
    sg0, a0 = sig_of(V)
    amb_cap += a0
    cell_cap[sg0] += 1
print('cells (sources)', len(cell_src), 'cells (capacity)', len(cell_cap), 'ambiguous', amb_src, amb_cap,
      '%.0fs' % (time.time() - t0), flush=True)
# H_P and its action on class-P signatures
perms = kp.perms.astype(np.int64)
Pset = set(classP.tolist())
HP = [k for k in range(7200) if set(perms[k, classP].tolist()) == Pset]
cells = sorted(set(cell_src) | set(cell_cap), key=lambda t: (len(t), t))
idx = {c: i for i, c in enumerate(cells)}
orbit = [-1] * len(cells)
orbits = []
for c in cells:
    if orbit[idx[c]] >= 0:
        continue
    oid = len(orbits)
    members = set()
    for k in HP:
        img = tuple(sorted(perms[k, list(c)].tolist()))
        if img in idx:
            members.add(img)
    for m in members:
        orbit[idx[m]] = oid
    orbits.append(sorted(members))
rows = []
for oid, mem in enumerate(orbits):
    srcs = sorted({cell_src.get(m, 0) for m in mem}); caps = sorted({cell_cap.get(m, 0) for m in mem})
    rows.append({'orbit': oid, 'cells': len(mem), 'signature_size': len(mem[0]), 'sources_per_cell': srcs,
                 'capacity_per_cell': caps, 'total_sources': sum(cell_src.get(m, 0) for m in mem)})
    print(rows[-1], flush=True)
# core test on the empty-signature cell
NFall = NF
empty = [s for s, sg in src_cell.items() if len(sg) == 0]
inter = disj = und = 0
for s in empty:
    o, k = int(reg.orbit_of[s]), int(reg.transport[s])
    V = (repV[o] @ Kf[k].T) @ G.T
    vals = V @ NFall.T - KAP                 # (nv, 600)
    if ((vals < -TOL).all(1)).any():
        inter += 1
    elif ((vals > -TOL).all(0)).any():
        disj += 1
    else:
        und += 1
print('empty cell sources', len(empty), 'core: vertex inside', inter, 'facet-separated', disj, 'undecided', und,
      '%.0fs' % (time.time() - t0), flush=True)
json.dump({'class': int(P), 'grip': c0, 'frame_item': int(JI[0]), 'H_P_order': len(HP), 'cells': len(cells),
           'orbits': rows, 'ambiguous_vertex_signs': [amb_src, amb_cap],
           'empty_cell': {'sources': len(empty), 'vertex_inside_core': inter, 'facet_separated': disj,
                          'undecided': und}}, open(OUT, 'w'), indent=1)
