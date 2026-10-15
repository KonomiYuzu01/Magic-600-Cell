"""All-depth certificate for one root, by pair orbits (menu I_a).

Every iterated refinement W_n = [W_(n-1), k_n] (n >= 1) of a root W0 is, as a partial map of contents, a composition
of the maps k and g_k = W0^-1 k W0 and their inverses, for K+ letters k at the grips used. So the statements below,
checked over every composition of these generators, hold for every W_n of every depth.

For one type t (normally 9, where T has one piece u) and a chamber x of type t with x in Bad(W0) \\ L:
  (A) from the pair (u, l(x)), no composition reaches a pair (a, b) with a and b both lattice contents, nor a pair
      (l(x), b);
  (B) from the pairs (lam, l(x)), lam any lattice content of type t in Omega, no composition reaches (u, mu) with mu a
      lattice content.
If W_n lands T, then W_n(u) is a lattice content, so by (A) W_n(l(x)) is not (x in Bad(W_n)) and by (B)
W_n^-1(l(x)) is not (x in Bad(W_n^-1)); W_n^-1(l(W_n(u))) = u is off, and W_n(u) is not l(x) by (A). So
|Bad(W_n^-1)_t| >= 2, and |Bad(W_n)_t| = |Bad(W_n^-1)_t| >= 2 > 1 = |T_t| (PROOFS-U Lemma K(a)): no W_n is a swap
word in type t.

The grips G*: the refinement letters of W_n sit at grips whose caps hold the landing of W_(n-1), in particular its
type-t landing chamber. G* starts with the grips holding the landing of W0 in type t and grows until it holds every
lattice content reachable from u (first coordinates of (A)). Omega: the type-t lattice contents in the caps of W0's
grips and of G*, and u, closed under W0, W0^-1 and the letters at G*. A content that dies stays dead (a pair with a
dead coordinate cannot violate (A) or (B)).

usage: pairorb.py <theory> <echain> <table> <target json> <rows.jsonl[.gz]> <out json> --type 9
       [--rows a:b] [--max-grips N] [--max-omega N]
"""
import gzip
import json
import sys
import time
from collections import deque
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import conjprep as CP  # noqa: E402
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
from exact import identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
table, tfile, rf, out = sys.argv[3:7]
a = sys.argv[7:]
TT = int(a[a.index('--type') + 1])
TRK = [int(x) for x in a[a.index('--track') + 1].split(',')] if '--track' in a else [TT]
RA, RB = (int(x) for x in a[a.index('--rows') + 1].split(':')) if '--rows' in a else (0, 10 ** 9)
MAXG = int(a[a.index('--max-grips') + 1]) if '--max-grips' in a else 120
MAXO = int(a[a.index('--max-omega') + 1]) if '--max-omega' in a else 200000
C = GL.Ctx(table)
M, data = C.M, GL.data
kp = GL.BL.CL.ctx.kplus
KID = kp.index_of_matrix(to_tuple(identity(4)))
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
ISTT = TYPE == TT
ISTRK = np.isin(TYPE, TRK)
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
Tt = T[ISTT[T]]
assert len(Tt) == 1, 'the method needs exactly one piece of the type'
Ttr = T[ISTRK[T]]
ud, uq = C.contents(Tt)
U0 = (int(ud[0]), int(uq[0]))
trd, trq = C.contents(Ttr)
TRK0 = list(zip(trd.tolist(), trq.tolist()))
_kl = {}


def kletters(c):
    if c not in _kl:
        _kl[c] = [i for i in range(len(M.items))
                  if kp.index_of_matrix(GL.restore.element(M, c, i)) not in (-1, KID)]
    return _kl[c]


def inv_letter(c, i):
    return int(CP.inverse_word(M, [(c, i)])[0][1])


def run(word, dom, q):
    g = FG.Ghost(C.F, dom, q)
    for c, i in word:
        g.apply(c, C.E(c, i), (c, i))
    return g


def grips_holding(ch):
    return set(np.flatnonzero(data.sig_matrix(np.asarray(ch, np.int64)).sum(axis=0) == len(ch)).tolist())


def certify(W0):
    W0i = CP.inverse_word(M, W0)
    g0 = run(W0, ud, uq)
    if not (g0.alive[0] and g0.dom[0] == 0):
        return {'status': 'root does not land u'}
    l0 = int(g0.q[0])
    gt = run(W0, trd, trq)
    if not (gt.alive.all() and (gt.dom == 0).all()):
        return {'status': 'root does not land the tracked pieces'}
    ltr = gt.q.tolist()
    capsW = sorted({c for c, _ in W0})
    R = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in capsW]) & ISTT)
    gb = run(W0, np.zeros(len(R), np.int64), R)
    bad = R[~gb.alive | (gb.dom != 0)]
    if (~gb.alive).any():
        return {'status': 'root has a dead lattice content of the type (closed by Lemma I)'}
    xs = [int(x) for x in bad.tolist() if int(x) != l0]
    if not xs:
        return {'status': 'no copy chamber', 'bad': bad.tolist()}
    Gs = grips_holding(ltr)
    idx, order = {}, []

    def add(key):
        if key not in idx:
            idx[key] = len(order)
            order.append(key)

    maps = {}

    def close(Gs):
        lat = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in sorted(set(capsW) | Gs)]) & ISTRK)
        for q in lat.tolist():
            add((0, q))
        add(U0)
        for c_ in TRK0:
            add(c_)
        for x in xs:
            add((0, x))
        ops = [('W0', W0), ('W0i', W0i)] + [(('k', f, i), [(f, i)]) for f in sorted(Gs) for i in kletters(f)]
        done = {name: maps.get(name, {}) for name, _ in ops}
        changed = True
        while changed:
            changed = False
            for name, word in ops:
                res = done[name]
                todo = [j for j in range(len(order)) if j not in res]
                if not todo:
                    continue
                changed = True
                g = run(word, np.array([order[j][0] for j in todo], np.int64),
                        np.array([order[j][1] for j in todo], np.int64))
                for n_, j in enumerate(todo):
                    if g.alive[n_]:
                        key = (int(g.dom[n_]), int(g.q[n_]))
                        add(key)
                        res[j] = key
                    else:
                        res[j] = None
            if len(order) > MAXO:
                return None
        maps.update(done)
        N = len(order)
        arr = {}
        for name, res in maps.items():
            v = np.full(N, -1, np.int64)
            for j, img in res.items():
                if img is not None:
                    v[j] = idx[img]
            arr[name] = v
        return arr

    while True:
        arr = close(Gs)
        if arr is None:
            return {'status': 'omega too large', 'grips': len(Gs)}
        N = len(order)
        isl = np.array([d == 0 for d, _ in order])
        gens = []
        for f in sorted(Gs):
            for i in kletters(f):
                K, Ki = arr[('k', f, i)], arr[('k', f, inv_letter(f, i))]
                gens.append(K)
                # g_k = W0^-1 k W0 as a map: apply W0, then k, then W0^-1
                for Kx in (K, Ki):
                    y = arr['W0'].copy()
                    ok = y >= 0
                    y[ok] = Kx[y[ok]]
                    ok = y >= 0
                    y[ok] = arr['W0i'][y[ok]]
                    gens.append(y)
                gens.append(Ki)
        Gm = np.stack(gens)          # generator maps, -1 = dead
        DEAD = N                     # index of the dead coordinate
        ext = np.concatenate([Gm, np.full((len(Gm), 1), DEAD, np.int64)], axis=1)
        ext[ext < 0] = DEAD
        isl_ext = np.concatenate([isl, [False]])

        def tuple_orbit(start):
            """BFS over tuples of Omega indices under every generator; a tuple with a dead coordinate is dropped.
            Returns (visited bitmap over base-N codes, base, list of visited tuples with every coordinate lattice)."""
            m = start.shape[1]
            base = np.array([N ** (m - 1 - j) for j in range(m)], np.int64)
            vis = np.zeros(N ** m, bool)
            vis[start @ base] = True
            lat = [p for p in start.tolist() if isl_ext[p].all()]
            frontier = start
            while len(frontier):
                parts = []
                for g in range(len(ext)):
                    nx = ext[g][frontier]
                    nx = nx[(nx != DEAD).all(axis=1)]
                    if not len(nx):
                        continue
                    c = nx @ base
                    fresh = ~vis[c]
                    if fresh.any():
                        c, first = np.unique(c[fresh], return_index=True)
                        nx = nx[fresh][first]
                        vis[c] = True
                        parts.append(nx)
                frontier = np.concatenate(parts) if parts else np.zeros((0, m), np.int64)
                if len(frontier):
                    lat += frontier[isl_ext[frontier].all(axis=1)].tolist()
            return vis, base, lat

        start = np.array([[idx[c_] for c_ in TRK0]], np.int64)
        vis_t, _, tland = tuple_orbit(start)
        tcount = int(vis_t.sum())
        del vis_t
        newg = set()
        for p in tland:
            newg |= grips_holding([order[j][1] for j in p])
        if not newg <= Gs:
            if len(Gs | newg) > MAXG:
                return {'status': 'too many grips', 'grips': len(Gs | newg)}
            Gs = Gs | newg
            continue
        lx = [idx[(0, x)] for x in xs]
        res = {'status': None, 'l0': l0, 'copy_chambers': xs, 'grips': sorted(Gs), 'omega': N}
        okx = None
        for x, ix in zip(xs, lx):
            visA, baseA, _ = tuple_orbit(np.array([[idx[U0], ix]], np.int64))
            pa = np.flatnonzero(visA)
            pa = np.stack([pa // N, pa % N], axis=1)
            A_bad = int((isl_ext[pa[:, 0]] & isl_ext[pa[:, 1]]).sum())
            A_lx = int((pa[:, 0] == ix).sum())
            lam_idx = np.flatnonzero(isl)
            visB, _, _ = tuple_orbit(np.stack([lam_idx, np.full(len(lam_idx), ix)], axis=1))
            pb = np.flatnonzero(visB)
            pb = np.stack([pb // N, pb % N], axis=1)
            B_bad = int(((pb[:, 0] == idx[U0]) & isl_ext[pb[:, 1]]).sum())
            res.setdefault('per_x', []).append({'x': x, 'pairs_A': len(pa), 'A_violations': A_bad,
                                                'A_lands_on_x': A_lx, 'pairs_B': len(pb), 'B_violations': B_bad,
                                                'tracked_orbit': tcount, 'tracked_landings': len(tland)})
            if A_bad == 0 and A_lx == 0 and B_bad == 0:
                okx = x
                break
        res['status'] = 'certified' if okx is not None else 'not certified'
        res['x'] = okx
        return res


rows = []
op = gzip.open if rf.endswith('.gz') else open
for line in op(rf, 'rt'):
    if line.strip():
        rows.append(json.loads(line))
if rows and 'Q' in rows[0]:
    tmax = max(r['T_on'] for r in rows)
    rows = [{'W': r['Q']} for r in rows if r['T_dead'] == 0 and r['T_on'] == tmax]
rows = rows[RA:RB]
results = []
for n, r in enumerate(rows):
    W0 = [(int(c), int(i)) for c, i in r['W']]
    t1 = time.time()
    res = certify(W0)
    res['row'] = RA + n
    res['sec'] = round(time.time() - t1, 1)
    results.append(res)
    print(json.dumps(res), flush=True)
from collections import Counter  # noqa: E402
doc = {'type': TT, 'rows': [RA, RA + len(rows)], 'status': dict(Counter(r['status'] for r in results)),
       'results': results, 'seconds': round(time.time() - T0)}
Path(out).write_text(json.dumps(doc) + '\n')
print(json.dumps({k: v for k, v in doc.items() if k != 'results'}))
