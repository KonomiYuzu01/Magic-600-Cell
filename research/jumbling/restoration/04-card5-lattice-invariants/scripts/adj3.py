"""3-face adjacency of the posed pieces of one configuration (menu I_a), floating point (evidence, not a certificate).

For every piece: posed facets (home facets of its region, carried by its pose matrix). Two pieces are 3-face
adjacent when they have opposite facets on one hyperplane whose relative interiors overlap (both facets shrunk by
1e-3 toward their centroids, then Wolfe's minimum-norm-point distance < 1e-9). A piece is anchored when one of its
posed facet normals is a pole (its facet lies on a fixed cut or facet hyperplane). Reports: components, anchoring,
Lemma L on every edge (left tree distance <= 2), chain distance to an anchored piece against left height.
usage: adj3.py <research/jumbling> <pose table json> <out json> [--limit-pieces N]"""
import json, sys, time
from collections import Counter, defaultdict, deque
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
tab = json.load(open(sys.argv[2]))
OUT = sys.argv[3]
NP = len(reg.orbit_of)
fm = lambda m: np.array([[float(v) for v in row] for row in m])
NF = np.array(d.NF, float)
NN = float(d.NN); KAP = float(d.KAPPA); RIN = NN ** 0.5
NFu = NF / RIN
orb = reg.orbit_of.astype(np.int64); tr = reg.transport.astype(np.int64)
Kf = np.array([fm(kp.matrix(i)) for i in range(7200)])
Pex = [matrix_from_json(m) for m in tab['poses']]
Pf = np.array([fm(m) for m in Pex])
pid = np.array(tab['pose_id'], np.int64)
print(json.dumps({'pieces': NP, 'poses': len(Pex), 'sec': round(time.time() - t0, 1)}), flush=True)

# exact left heights per pose
lf = []
hpose = []
for m in Pex:
    l, r = T.factors(m)
    lf.append(l); hpose.append(T.dist(l))
hpose = np.array(hpose)
hp = hpose[pid]


def mnp(P, maxit=500):
    """Wolfe's minimum-norm point of conv(P) (rows)."""
    nrm = (P * P).sum(1)
    i0 = int(np.argmin(nrm))
    S = [i0]; w = np.array([1.0]); x = P[i0].copy()
    scale = max(1.0, nrm.max())
    for _ in range(maxit):
        vals = P @ x
        j = int(np.argmin(vals))
        if x @ x - vals[j] <= 1e-12 * scale or j in S:
            return x
        S.append(j); w = np.append(w, 0.0)
        for _m in range(50):
            Q = P[S]; k = len(S)
            A = np.zeros((k + 1, k + 1)); A[:k, :k] = Q @ Q.T; A[:k, k] = 1; A[k, :k] = 1
            b = np.zeros(k + 1); b[k] = 1
            try:
                sol = np.linalg.solve(A, b)
            except np.linalg.LinAlgError:
                sol = np.linalg.lstsq(A, b, rcond=None)[0]
            al = sol[:k]
            if (al > 1e-12).all():
                w = al; x = al @ Q
                break
            th = 1.0
            for i in range(k):
                if al[i] <= 1e-12 and w[i] - al[i] > 1e-15:
                    th = min(th, w[i] / (w[i] - al[i]))
            w = w + th * (al - w)
            keep = w > 1e-12
            if not keep.any():
                keep[int(np.argmax(w))] = True
            S = [S[i] for i in range(k) if keep[i]]; w = w[keep]; w = w / w.sum()
            x = w @ P[S]
    return x


# representative facets
repV = [np.array([[float(x) for x in v] for v in vs]) for vs in reg.rep_vertices]
rfac = []        # per orbit: list of (u (4,), delta, vertex index array, kind)
ball_orbit = []
for o, V in enumerate(repV):
    fl = []
    for kind, off in (('cut', KAP), ('body', NN)):
        vals = V @ NF.T - off
        for c in range(600):
            t = np.abs(vals[:, c]) < 1e-7
            if t.sum() < 4:
                continue
            W = V[t] - V[t][0]
            if np.linalg.matrix_rank(W, tol=1e-9) != 3:
                continue
            rest = vals[~t, c]
            if kind == 'body':
                assert (rest < 0).all()
                u, dl = NFu[c], RIN
            elif (rest > 0).all():
                u, dl = -NFu[c], -KAP / RIN
            else:
                assert (rest < 0).all()
                u, dl = NFu[c], KAP / RIN
            fl.append((u, dl, np.flatnonzero(t), kind))
    rfac.append(fl)
    xm = mnp(V)
    ball_orbit.append(float(np.linalg.norm(xm)) < RIN - 1e-9)
print(json.dumps({'kplus_orbits': len(repV), 'facets_per_orbit': [len(f) for f in rfac],
                  'ball_orbits': [o for o in range(len(repV)) if ball_orbit[o]], 'sec': round(time.time() - t0, 1)}),
      flush=True)

# posed facets
pole_keys = {tuple(np.round(v, 6) + 0.0) for v in NFu}
recs_piece, recs_k, keys = [], [], []
anch = np.zeros(NP, bool)
Mq = np.einsum('qij,qjk->qik', Pf[pid], Kf[tr])
for o in range(len(repV)):
    qs = np.flatnonzero(orb == o)
    for k, (u, dl, idx, kind) in enumerate(rfac[o]):
        U = Mq[qs] @ u
        Ur = np.round(U, 6) + 0.0
        for q, ur in zip(qs.tolist(), map(tuple, Ur.tolist())):
            if ur in pole_keys:
                anch[q] = True
        key = np.hstack([np.round(U * 1e6).astype(np.int64), np.full((len(qs), 1), int(round(dl * 1e6)))])
        recs_piece.append(qs); recs_k.append(np.full(len(qs), k)); keys.append(key)
fp = np.concatenate(recs_piece); fk = np.concatenate(recs_k); K = np.concatenate(keys)
# canonical orientation: compare key with its negation lexicographically
neg = -K
flip = np.zeros(len(K), bool)
for col in range(5):
    undecided = ~flip & (np.all(K[:, :col] == neg[:, :col], axis=1) if col else np.ones(len(K), bool))
    flip |= undecided & (K[:, col] < neg[:, col])
C = np.where(flip[:, None], neg, K)
side = np.where(flip, -1, 1)
_, hid = np.unique(C, axis=0, return_inverse=True)
hid = hid.ravel()
print(json.dumps({'facets': len(fp), 'hyperplanes': int(hid.max()) + 1, 'anchored_pieces': int(anch.sum()),
                  'sec': round(time.time() - t0, 1)}), flush=True)

order = np.argsort(hid, kind='stable')
bounds = np.flatnonzero(np.diff(hid[order])) + 1
groups = [g for g in np.split(order, bounds) if (side[g] == 1).any() and (side[g] == -1).any()]
RU = [np.array([f[0] for f in rfac[o]]) for o in range(len(repV))]
RD = [np.array([f[1] for f in rfac[o]]) for o in range(len(repV))]
SH = 1e-3


def fverts(i):
    q, k = int(fp[i]), int(fk[i])
    V = repV[orb[q]][rfac[orb[q]][k][2]]
    return V @ Mq[q].T


def cons(q, k):
    """posed constraints of piece q other than its facet k: (U, delta) with region in U x <= delta."""
    o = orb[q]
    U = RU[o] @ Mq[q].T
    keep = np.arange(len(U)) != k
    return U[keep], RD[o][keep]


def work(gs):
    out = []; st = Counter()
    for g in gs:
        a = g[side[g] == 1]; b = g[side[g] == -1]
        VA = [fverts(i) for i in a]; VB = [fverts(i) for i in b]
        loA = np.array([v.min(0) for v in VA]); hiA = np.array([v.max(0) for v in VA])
        loB = np.array([v.min(0) for v in VB]); hiB = np.array([v.max(0) for v in VB])
        ov = np.all((loA[:, None, :] < hiB[None, :, :] - 1e-9) & (loB[None, :, :] < hiA[:, None, :] - 1e-9), axis=2)
        CA = {}; CB = {}
        for ia, ib in zip(*np.nonzero(ov)):
            qa, qb = int(fp[a[ia]]), int(fp[b[ib]])
            if qa == qb:
                continue
            st['cand'] += 1
            if ia not in CA:
                CA[ia] = cons(qa, int(fk[a[ia]]))
            if ib not in CB:
                CB[ib] = cons(qb, int(fk[b[ib]]))
            Ua, da = CA[ia]; Ub, db = CB[ib]
            A_ = VA[ia]; B_ = VB[ib]
            ca, cb = A_.mean(0), B_.mean(0)
            if (Ua @ cb - da < -1e-9).all() or (Ub @ ca - db < -1e-9).all():
                out.append((min(qa, qb), max(qa, qb))); st['centroid'] += 1
                continue
            A_ = ca + (1 - SH) * (A_ - ca); B_ = cb + (1 - SH) * (B_ - cb)
            if ((Ua @ B_.T - da[:, None]) > 1e-12).all(1).any() or ((Ub @ A_.T - db[:, None]) > 1e-12).all(1).any():
                st['face_sep'] += 1
                continue
            D = (A_[:, None, :] - B_[None, :, :]).reshape(-1, 4)
            st['mnp'] += 1
            if np.linalg.norm(mnp(D)) < 1e-9:
                out.append((min(qa, qb), max(qa, qb))); st['mnp_edge'] += 1
    return out, st


import multiprocessing as mp
NPROC = 4
chunks = [groups[i::NPROC * 8] for i in range(NPROC * 8)]
edges = set(); stats = Counter()
with mp.get_context('fork').Pool(NPROC) as pool:
    for n, (e_, s_) in enumerate(pool.imap_unordered(work, chunks)):
        edges.update(e_); stats.update(s_)
        print(json.dumps({'chunk': n + 1, 'of': len(chunks), 'edges': len(edges), **stats,
                          'sec': round(time.time() - t0, 1)}), flush=True)
tests = stats['mnp']
print(json.dumps({'edges': len(edges), 'tests': tests, 'sec': round(time.time() - t0, 1)}), flush=True)

# graph analysis
adj = defaultdict(list)
for x, y in edges:
    adj[x].append(y); adj[y].append(x)
par = list(range(NP))


def find(x):
    while par[x] != x:
        par[x] = par[par[x]]; x = par[x]
    return x


for x, y in edges:
    rx, ry = find(x), find(y)
    if rx != ry:
        par[rx] = ry
comp = np.array([find(x) for x in range(NP)])
cs = Counter(comp.tolist())
ballp = np.array([ball_orbit[o] for o in orb])
comp_info = []
for c, n in cs.most_common():
    mem = np.flatnonzero(comp == c)
    comp_info.append({'size': n, 'anchored': int(anch[mem].sum()), 'ball': int(ballp[mem].sum()),
                      'max_height': int(hp[mem].max()), 'orbits': sorted(set(d.orbit_id[mem].tolist()))[:12]})
# Lemma L on edges
lcache = {}
bad_L = 0; Lhist = Counter()
for x, y in edges:
    a_, b_ = int(pid[x]), int(pid[y])
    if a_ == b_:
        Lhist[0] += 1; continue
    kk = (min(a_, b_), max(a_, b_))
    if kk not in lcache:
        lcache[kk] = T.dist(T.qmul(T.qconj(lf[kk[0]]), lf[kk[1]]))
    Lhist[lcache[kk]] += 1
    bad_L += lcache[kk] > 2
# chain distance to anchored pieces
dist = np.full(NP, -1)
dq = deque()
for q in np.flatnonzero(anch).tolist():
    dist[q] = 0; dq.append(q)
while dq:
    x = dq.popleft()
    for y in adj[x]:
        if dist[y] < 0:
            dist[y] = dist[x] + 1; dq.append(y)
hd = Counter()
viol = 0
for q in range(NP):
    hd[(int(hp[q]), int(dist[q]))] += 1
    if dist[q] >= 0 and hp[q] > 2 + 2 * dist[q]:
        viol += 1
deg = Counter(len(adj[q]) for q in range(NP))
res = {'table': sys.argv[2].split('/')[-1], 'digest': tab.get('digest'), 'pieces': NP, 'edges': len(edges),
       'components': len(cs), 'component_sizes_top': [c['size'] for c in comp_info[:10]],
       'components_without_anchor': sum(1 for c in comp_info if c['anchored'] == 0),
       'components_without_ball_piece': sum(1 for c in comp_info if c['ball'] == 0),
       'small_components': comp_info[1:60], 'anchored_pieces': int(anch.sum()),
       'height_hist': dict(Counter(hp.tolist())),
       'lemmaL_edge_left_distance_hist': dict(sorted(Lhist.items())), 'lemmaL_violations': bad_L,
       'height_vs_anchor_distance': {f'{h}:{dd}': n for (h, dd), n in sorted(hd.items())},
       'height_bound_violations': viol, 'degree_hist': dict(sorted(deg.items())),
       'ball_orbits': [o for o in range(len(repV)) if ball_orbit[o]],
       'unreached_from_anchor': int((dist < 0).sum()), 'sec': round(time.time() - t0, 1)}
Path(OUT).write_text(json.dumps(res, indent=1) + '\n')
np.savez_compressed(OUT[:-5] + '.npz', edges=np.array(sorted(edges), np.int64), comp=comp, anch=anch, dist=dist, hp=hp, pid=pid)
print(json.dumps({k: v for k, v in res.items() if k != 'small_components'}), flush=True)
