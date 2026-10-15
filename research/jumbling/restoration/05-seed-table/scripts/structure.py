"""Seed-table structure for 3E (menu I_a, height-2 off pieces).
(1) Classes: check that every k in K+ maps each grip class onto a grip class, and that K+ permutes the classes
    transitively (then a seed of class 0 is carried to every class by conjugation, Lemma E).
(2) Position classes: fix the frame gamma K+ (gamma = first jumble element at grip c0). An off piece in this frame
    occupies a posed region gamma A_s for a unique home s (its position). Conjugation by k in K+ preserves the
    frame exactly when gamma^-1 k gamma = k' is in K+, and then maps position s to k'(s). So seed classes are the
    orbits of the frame-compatible group S' = gamma^-1 H gamma (order 360) on the 177,120 homes. A seed (a
    replayed word from solved that ends with one off piece) restores every piece in its class, by conjugation.
(3) Registry orbit of each position (the orbit of its source cell, as in atlas2.py) and, for the empty cell
    (registry orbit 0), the core test of atlas2.py (floating point with exact-sign shortcuts).
(4) The class of the off piece of every single word given.
usage: structure.py <research/jumbling> <classes.json> <c0> <out json> [singles jsonl ...]"""
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
fm = lambda m: np.array([[float(v) for v in row] for row in m])
Kf = np.array([fm(kp.matrix(i)) for i in range(7200)])
key = lambda A: tuple(np.round(A, 6).ravel().tolist())
Kidx = {key(Kf[i]): i for i in range(7200)}
assert len(Kidx) == 7200
perms = kp.perms.astype(np.int64)
# (1) classes
names = sorted(set(cls.values()))
members = {P: np.array([c for c in range(600) if cls[c] == P]) for P in names}
block_ok, induced = True, set()
for k in range(7200):
    img = []
    for P in names:
        im = {cls[int(c)] for c in perms[k, members[P]]}
        if len(im) != 1:
            block_ok = False
            break
        img.append(im.pop())
    if not block_ok:
        break
    induced.add(tuple(img))
orbit0 = {t[0] for t in induced}
classes = {'classes': len(names), 'class_sizes': [len(members[P]) for P in names], 'block_system': block_ok,
           'induced_permutations': len(induced), 'orbit_of_first_class': sorted(orbit0),
           'transitive': block_ok and len(orbit0) == len(names)}
print('classes', classes, '%.0fs' % (time.time() - t0), flush=True)
# (2) frame-compatible group and position classes
P = cls[c0]
classP = members[P]
JI = list(RS.jumble_items(M))
G = fm(RS.element(M, c0, JI[0])); Gi = G.T
Pset = set(classP.tolist())
Sp = []
for k in range(7200):
    kk = Kidx.get(key(Gi @ Kf[k] @ G))
    if kk is not None:
        Sp.append(kk)
print('frame-compatible group order', len(Sp), '%.0fs' % (time.time() - t0), flush=True)
NP = len(reg.orbit_of)
homes = np.arange(NP)
orb, tr = reg.orbit_of.astype(np.int64), reg.transport.astype(np.int64)
lab = homes.copy()
for kk in Sp:
    img = reg.piece_of[orb, kp.compose_vec(np.full(NP, kk), tr)]
    np.minimum(lab, img, out=lab)
# lab[s] = min over the group orbit (the group contains the identity and is closed, so this is the orbit minimum)
cls_of = lab
nclass = len(np.unique(lab))
print('position classes', nclass, '%.0fs' % (time.time() - t0), flush=True)
# (3) registry orbits of positions
repV = [np.array([[float(v) for v in vert] for vert in vs]) for vs in reg.rep_vertices]
NF = np.array(d.NF, float); KAP = float(d.KAPPA); TOL = 1e-7
def sig_of(V):
    vals = V @ NF[classP].T - KAP
    ins = (vals > -TOL).all(0) & (vals > TOL).any(0)
    return tuple(classP[ins].tolist())
cell_src, cell_cap, src_cell = Counter(), Counter(), np.empty(NP, object)
for s in range(NP):
    V = repV[orb[s]] @ Kf[tr[s]].T
    sg = sig_of(V @ G.T)
    cell_src[sg] += 1; src_cell[s] = sg
    cell_cap[sig_of(V)] += 1
cells = sorted(cell_src, key=lambda t: (len(t), t))
cidx = {c: i for i, c in enumerate(cells)}
Hk = [k for k in range(7200) if Kidx.get(key(Gi @ Kf[k] @ G)) is not None]
corb = [-1] * len(cells); corbits = []
for c in cells:
    if corb[cidx[c]] >= 0:
        continue
    mem = {tuple(sorted(perms[k, list(c)].tolist())) for k in Hk}
    for m in mem:
        corb[cidx[m]] = len(corbits)
    corbits.append(sorted(mem))
REG = {(0, 5280, 5280): '0', (1, 385, 385): '1', (2, 86, 86): '2|5', (3, 111, 0): '3', (4, 87, 0): '4', (3, 111, 111): '6'}
corb_name = []
for oid, mem in enumerate(corbits):
    k_ = (len(mem[0]), cell_src[mem[0]], cell_cap.get(mem[0], 0))
    corb_name.append(REG.get(k_, '?') + ('' if REG.get(k_) != '2|5' else ('a' if sum(1 for n in corb_name if n.startswith('2|5')) == 0 else 'b')))
reg_of = np.array([corb_name[corb[cidx[src_cell[s]]]] for s in range(NP)], object)
print('cell orbits', [(corb_name[i], len(m), cell_src[m[0]], cell_cap.get(m[0], 0)) for i, m in enumerate(corbits)], '%.0fs' % (time.time() - t0), flush=True)
# core test for the empty cell (as atlas2.py)
allV = np.concatenate([repV[orb[s]] @ Kf[tr[s]].T for s in range(NP)])
nr = np.linalg.norm(allV, axis=1); Rmax = nr.max()
pv = []
for x in allV[nr > Rmax - 1e-6]:
    if all(np.linalg.norm(x - y) > 1e-6 for y in pv):
        pv.append(x)
core_v = (121 / 125) * (1 - 1e-6) * np.array(pv)
def fw_dist(A, B, it=200000):
    z = A[0] - B[0]
    for _ in range(it):
        sa = A[np.argmin(A @ z)]; sb = B[np.argmax(B @ z)]
        s_ = sa - sb
        if z @ s_ > 1e-12:
            return 'sep'
        dz = s_ - z; den = dz @ dz
        if den < 1e-30:
            break
        lam = min(1.0, max(0.0, -(z @ dz) / den)); z = z + lam * dz
        if np.linalg.norm(z) < 1e-10:
            return 'int'
    return 'touch'
core = {}
for s in np.flatnonzero(reg_of == '0').tolist():
    V = (repV[orb[s]] @ Kf[tr[s]].T) @ G.T
    vals = V @ NF.T - KAP
    if ((vals < -TOL).all(1)).any():
        core[s] = 'vertex_inside'
    elif ((vals > -TOL).all(0)).any():
        core[s] = 'facet_separated'
    else:
        core[s] = 'fw_' + fw_dist(V, core_v)
core_bad = {s for s, v in core.items() if v in ('vertex_inside', 'fw_int', 'fw_touch')}
print('core', Counter(core.values()), '%.0fs' % (time.time() - t0), flush=True)
table = defaultdict(lambda: {'positions': 0, 'classes': set(), 'core_forbidden_positions': 0, 'core_ok_classes': set()})
for s in range(NP):
    r = reg_of[s]
    t = table[r]
    t['positions'] += 1; t['classes'].add(int(cls_of[s]))
    if s in core_bad:
        t['core_forbidden_positions'] += 1
    else:
        t['core_ok_classes'].add(int(cls_of[s]))
# class sizes and purity (every class lies in one registry orbit, and either all or none of its positions are core-forbidden)
csize = Counter(cls_of.tolist())
mixed_reg = sum(1 for c in csize if len({reg_of[s] for s in np.flatnonzero(cls_of == c)[:50]}) > 1)
# (4) singles
TW = {}
def tw(l):
    if l not in TW:
        TW[l] = sim.Twist(ctx, l[0], RS.element(M, l[0], l[1]), 'menu', {'menu': M.name})
    return TW[l]
srows = []
for path in sys.argv[5:]:
    for line in open(path):
        if not line.strip():
            continue
        r = json.loads(line)
        st = sim.State(ctx, menu=M)
        for l in r['word']:
            assert st.apply(tw(tuple(l))).applied
        off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
        assert len(off) == 1
        p = int(off[0]); X = fm(st.pose(p))
        found = None
        for k in range(7200):
            rr = Kidx.get(key(Gi @ Kf[k] @ X))
            if rr is not None:
                found = (k, rr); break
        k, rr = found
        s = int(reg.piece_of[orb[p], kp.compose_vec(np.array([rr]), np.array([tr[p]]))[0]])
        srows.append({'name': r.get('name'), 'length': len(r['word']), 'piece': p, 'piece_orbit': int(d.orbit_id[p]),
                      'position': s, 'class': int(cls_of[s]), 'class_size': int(csize[int(cls_of[s])]),
                      'registry_orbit': reg_of[s], 'core': core.get(s)})
        print(srows[-1], flush=True)
summary = {r: {'positions': t['positions'], 'position_classes': len(t['classes']),
               'core_forbidden_positions': t['core_forbidden_positions'], 'core_ok_classes': len(t['core_ok_classes']),
               'classes_with_a_seed': len({x['class'] for x in srows if x['registry_orbit'] == r})}
           for r, t in sorted(table.items())}
out = {'classes': classes, 'frame': {'grip': c0, 'jumble_item': int(JI[0]), 'frame_compatible_order': len(Sp)},
       'position_classes_total': nclass, 'class_size_histogram': dict(sorted(Counter(csize.values()).items())),
       'classes_spanning_two_registry_orbits_(sampled)': mixed_reg, 'by_registry_orbit': summary,
       'core_test_counts': dict(Counter(core.values())), 'singles': srows,
       'distinct_seed_classes': len({x['class'] for x in srows}), 'seconds': round(time.time() - t0, 1)}
json.dump(out, open(OUT, 'w'), indent=1)
print(json.dumps({k: v for k, v in out.items() if k != 'singles'}, indent=1))
