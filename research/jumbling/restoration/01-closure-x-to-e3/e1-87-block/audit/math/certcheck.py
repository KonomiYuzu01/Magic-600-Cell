"""Math audit (d): independent check of the peeling certificate of segment 2 against Theorem B's hypotheses.

From E1-table.json only (plus the certificate's rows, which are the object checked):
- SAFE-ALL grips at E1 by a per-piece exact classification of every off piece against every cut (reference path,
  no grouping), compared with pre.json;
- letters = (grip, menu index) with element in K+ minus identity, acting on chambers of the cap by the K+ element;
- G-orbits of the target chambers (min-label propagation over all letters);
- for each row j: every letter of E_j is SAFE-ALL, pi_{E_j}(y_j) = t_j, pi_{E_j} fixes P_j pointwise, y_j not in T;
- routes: reachability of y_j inside its orbit by free letters (caps avoid P_j), macros E_k (k >= j, both ways) and the
  row's conjugate edges (each checked: SAFE-ALL letters, from -> to, fixes P_j pointwise); u_j and thresholds h_Delta;
- holes per orbit at E1 against h_Delta.
Usage: python3 -I certcheck.py <theory dir> <v26 dir> <out json>
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
V26 = Path(sys.argv[2]).resolve()
OUT = Path(sys.argv[3])
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import orient as O  # noqa: E402
import restore  # noqa: E402
from exact import identity  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
C0 = V26 / 'E2' / 'c0'


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


R, M = O.start(str(C0 / 'E1-table.json'))
st = R.st
ctx = st.ctx
data, kp, reg = ctx.data, ctx.kplus, ctx.regions
NP = len(st.pose_id)
K = st._kpose()[st.pose_id]
offp = np.flatnonzero(K < 0)
out = {'digest': st.digest(), 'off': int(len(offp))}

# ---- SAFE-ALL grips, per piece
safe, blocked = [], {}
for e in range(600):
    evs, strad, inside = {}, 0, 0
    for p in offp.tolist():
        pid = int(st.pose_id[p])
        ev = evs.get(pid)
        if ev is None:
            ev = evs[pid] = Evaluator(pullback_form(st._pose_matrix(pid), data.N[e]))
        r = ev.classify(reg.vform(p))[0]
        if r == 'straddle':
            strad += 1
        elif r == 'in':
            inside += 1
    if strad == 0 and inside == 0:
        safe.append(e)
    else:
        blocked[e] = (strad, inside)
pre = json.loads((C0 / 'pre.json').read_text())
out['safe_count'] = len(safe)
out['safe_equals_pre'] = safe == pre['menu']['safe_grips']
out['blocked'] = {str(k): v for k, v in blocked.items()}
log(safe=len(safe))

# ---- letters
kid = kp.index_of_matrix(to_tuple(identity(4)))
ORB, TRN, POF = np.asarray(reg.orbit_of), np.asarray(reg.transport), reg.piece_of
allc = np.arange(NP)


def img(k, z):
    z = np.asarray(z, np.int64)
    return POF[ORB[z], kp.compose_vec(np.full(len(z), k, np.int64), TRN[z])]


capmask = {}
letters = {}
for c in safe:
    capmask[c] = data.in_sig(allc, np.full(NP, c, np.int64))
    for i in range(len(M.items)):
        k = kp.index_of_matrix(restore.element(M, c, i))
        if k >= 0 and k != kid:
            letters[(c, i)] = int(k)
out['letters'] = len(letters)
by_grip = {}
for (c, i), k in letters.items():
    by_grip.setdefault(c, []).append(k)
safe_set = set(safe)
log(letters=len(letters))


def track(word, pts):
    pts = np.array(pts, np.int64)
    for c, i in word:
        k = letters[(c, i)]
        m = capmask[c][pts]
        if m.any():
            pts[m] = img(k, pts[m])
    return pts


# ---- orbits (min-label propagation over all letters)
lab = np.arange(NP)
caps_x = {c: np.flatnonzero(capmask[c]) for c in safe}
lett_imgs = [(c, img(k, caps_x[c])) for (c, i), k in letters.items()]
while True:
    old = lab.copy()
    for c, ys in lett_imgs:
        xs = caps_x[c]
        mm = np.minimum(lab[xs], lab[ys])
        np.minimum.at(lab, xs, mm)
        np.minimum.at(lab, ys, mm)
    while True:
        nl = lab[lab]
        if np.array_equal(nl, lab):
            break
        lab = nl
    if np.array_equal(lab, old):
        break
log(orbits=int(len(np.unique(lab))))

cert = json.loads((C0 / 'certificate.json').read_text())
T = np.asarray(cert['target_chambers'], np.int64)
out['target_equals_pre_badW'] = sorted(T.tolist()) == sorted(pre['ghost']['badW'])
inT = np.zeros(NP, bool)
inT[T] = True
# holes at E1 from the pose table
lat = np.flatnonzero(K >= 0)
occ = np.zeros(NP, bool)
occ[img(0, lat) if False else POF[ORB[lat], kp.compose_vec(K[lat], TRN[lat])]] = True
holes = np.flatnonzero(~occ)
out['holes'] = int(len(holes))
orbT = sorted(set(lab[T].tolist()))
osz = {o: int((lab == o).sum()) for o in orbT}
out['target_orbits'] = {str(o): {'size': osz[o], 'target': int((lab[T] == o).sum()),
                                  'holes': int((lab[holes] == o).sum()),
                                  'type_sizes': sorted(set(int((ORB == t).sum()) for t in ORB[lab == o]))}
                        for o in orbT}
out['target_fixed'] = int(sum(1 for o in lab[T] if osz[o] == 1))

# ---- rows
rows = cert['rows']
order = [r['t'] for r in rows]
out['order_is_permutation_of_T'] = sorted(order) == sorted(T.tolist())
# macro permutations on the union of target orbits
U = np.flatnonzero(np.isin(lab, orbT))
pos = np.full(NP, -1, np.int64)
pos[U] = np.arange(len(U))
macro = []
for r in rows:
    w = [tuple(x) for x in r['word']]
    macro.append(track(w, U))
# free-letter edges per orbit, by grip
edges = {}
for o in orbT:
    pts = np.flatnonzero(lab == o)
    src, dst, grp = [], [], []
    for c in safe:
        xs = pts[capmask[c][pts]]
        if not len(xs):
            continue
        for k in by_grip[c]:
            ys = img(k, xs)
            keep = ys != xs
            src.append(xs[keep]); dst.append(ys[keep]); grp.append(np.full(int(keep.sum()), c))
    edges[o] = (np.concatenate(src), np.concatenate(dst), np.concatenate(grp))
log(edges={str(o): int(len(v[0])) for o, v in edges.items()})

problems = []
u_rows, hneed = [], {}
for j, r in enumerate(rows):
    t, y = int(r['t']), int(r['y'])
    P = np.asarray(order[:j], np.int64)
    w = [tuple(x) for x in r['word']]
    if not all(x in letters for x in w):
        problems.append((j, 'E_j has a non-SAFE-ALL letter'))
    if int(track(w, [y])[0]) != t:
        problems.append((j, 'pi_E(y) != t'))
    if len(P) and not np.array_equal(track(w, P), P):
        problems.append((j, 'E_j moves a point of P_j'))
    if inT[y]:
        problems.append((j, 'y_j in T'))
    o = int(lab[t])
    if int(lab[y]) != o:
        problems.append((j, 'y_j not in the orbit of t_j'))
    # graph for row j
    src, dst, grp = edges[o]
    Pmask_grips = set()
    for p in P.tolist():
        for c in safe:
            if capmask[c][p]:
                Pmask_grips.add(c)
    keep = ~np.isin(grp, list(Pmask_grips)) if Pmask_grips else np.ones(len(grp), bool)
    S, D = [src[keep]], [dst[keep]]
    pts = np.flatnonzero(lab == o)
    for kk in range(j, len(rows)):
        im = macro[kk][pos[pts]]
        mv = im != pts
        S += [pts[mv], im[mv]]
        D += [im[mv], pts[mv]]
    for e in r['conjugate_edges']:
        cw = [tuple(x) for x in e['word']]
        if not all(x in letters for x in cw):
            problems.append((j, 'conjugate edge has a non-SAFE-ALL letter'))
            continue
        if int(track(cw, [e['from']])[0]) != int(e['to']):
            problems.append((j, 'conjugate edge does not map from -> to'))
            continue
        if len(P) and not np.array_equal(track(cw, P), P):
            problems.append((j, 'conjugate edge moves P_j'))
            continue
        S += [np.array([e['from'], e['to']])]
        D += [np.array([e['to'], e['from']])]
    S, D = np.concatenate(S), np.concatenate(D)
    ordr = np.argsort(S, kind='stable')
    S, D = S[ordr], D[ordr]
    start = np.searchsorted(S, np.arange(NP + 1))
    seen = np.zeros(NP, bool)
    seen[y] = True
    front = np.array([y])
    while len(front):
        idx = np.concatenate([np.arange(start[x], start[x + 1]) for x in front.tolist()])
        nb = np.unique(D[idx]) if len(idx) else np.zeros(0, np.int64)
        nb = nb[~seen[nb]]
        seen[nb] = True
        front = nb
    if seen[P].any() if len(P) else False:
        problems.append((j, 'route set meets P_j'))
    cand = pts[~np.isin(pts, P)]
    unreached = cand[(~seen[cand]) & (cand != t)]
    u = int(len(unreached))
    u_rows.append(u)
    need = int(np.isin(P, pts).sum()) + 1 + u
    hneed[o] = max(hneed.get(o, 0), need)
    if u != r['u_j']:
        problems.append((j, f'u_j recomputed {u} != recorded {r["u_j"]}'))
out['u_j_max'] = max(u_rows)
out['u_j_all_zero'] = all(u == 0 for u in u_rows)
out['thresholds'] = {str(o): {'h': hneed[o], 'target': int((lab[T] == o).sum()), 'holes': int((lab[holes] == o).sum())}
                     for o in orbT}
out['thresholds_met'] = all(int((lab[holes] == o).sum()) >= hneed[o] for o in orbT)
out['problems'] = [list(map(str, x)) for x in problems][:50]
out['n_problems'] = len(problems)
out['seconds'] = round(time.time() - T0, 1)
OUT.write_text(json.dumps(out, indent=1) + '\n')
log(done=True, problems=len(problems))
