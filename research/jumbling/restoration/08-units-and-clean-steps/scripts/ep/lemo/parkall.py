"""Every parking word of a unit's family, with the per-type swap condition (certificate, menu I_a).

dfam2.py parks a family word Q as W = P Q P^-1 with P = [[X, Y], Z] = X, Y, X^-1, Y^-1, Z, Y, X, Y^-1, X^-1, Z^-1,
where X is Q's letter at grip c, Y a non-identity K+ letter at a cap p != c that holds all of Bad(Q) and is a pole of
X's frame, and Z a letter outside K+ at any grip with Z n_e = n_c (dfam2 also asks Z n_p = n_p unless --relaxed, and
admissible grips; this script drops both, so it covers more words than dfam2 makes in any state, and it takes every
family word with a lens cap, not only the few dfam2 picks).

For every such W whose ghost lands T, the script counts Bad(W) in the given types (exact frame ghost of the lattice
chambers of those types in the caps of W's grips: a chamber outside every cap of W is not moved) and the excess
sum_t max(0, |Bad(W)_t| - |T_t|) of Corollary D''. Excess 0 is needed to restore T as the last carrier of these types.

usage: parkall.py <theory> <echain> <table> <target json {pieces}> <lens json (lens.py)> <out json> --types 7,9
"""
import json
import sys
import time
from collections import Counter
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa: E402
from exact import matvec, transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
table, tfile, lfile, out = sys.argv[3:7]
a = sys.argv[7:]
TYPES = [int(x) for x in a[a.index('--types') + 1].split(',')]
C = GL.Ctx(table)
M, E, data, kp = C.M, C.E, GL.data, GL.BL.CL.ctx.kplus
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
RARE = np.isin(TYPE, TYPES)
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
need = {t: int((TYPE[T] == t).sum()) for t in TYPES}
n_items = len(M.items)
ident = to_tuple(E(0, 0))
inK = lambda g: kp.index_of_matrix(to_tuple(g)) >= 0  # noqa: E731
_inv, _kit, _Z = {}, {}, {}


def inv(c, i):
    if (c, i) not in _inv:
        t = to_tuple(transpose(E(c, i)))
        _inv[(c, i)] = next(j for j in range(n_items) if to_tuple(E(c, j)) == t)
    return _inv[(c, i)]


def kit(c):
    if c not in _kit:
        _kit[c] = [i for i in range(n_items) if inK(E(c, i)) and to_tuple(E(c, i)) != ident]
    return _kit[c]


def pole(g, e):
    return data.POLE.get(tuple(matvec(transpose(g), data.N[e])), -1)


def img(g, e):
    return data.POLE.get(tuple(matvec(g, data.N[e])), -1)


def zs(e, c):
    if (e, c) not in _Z:
        _Z[(e, c)] = [(z, i) for z in range(600) for i in range(n_items)
                      if not inK(E(z, i)) and img(E(z, i), e) == c]
    return _Z[(e, c)]


def ecx(Q):
    if len(Q) == 10:
        return Q[1][0], Q[2][0], Q[2][1]
    assert len(Q) == 4
    return Q[1][0], Q[0][0], Q[0][1]


words = json.loads(Path(lfile).read_text())['with_lens_words']
hist, swaps = Counter(), []
n_park = n_land = 0
by_q = []
for nq, w in enumerate(words):
    Q = [(int(c), int(i)) for c, i in w['Q']]
    e, c, Xi = ecx(Q)
    Xm = E(c, Xi)
    Ps = [p for p in w['lens'] if p != c and pole(Xm, p) >= 0]
    Zl = zs(e, c)
    nq_park = nq_land = 0
    best = None
    for p in Ps:
        for y in kit(p):
            for (z, i) in Zl:
                J1 = [(c, Xi), (p, y), (c, inv(c, Xi)), (p, inv(p, y))]
                J1i = [(p, y), (c, Xi), (p, inv(p, y)), (c, inv(c, Xi))]
                P = J1 + [(z, i)] + J1i + [(z, inv(z, i))]
                Pinv = [(cc, inv(cc, ii)) for cc, ii in reversed(P)]
                W = P + Q + Pinv
                grips = sorted({g_ for g_, _ in W})
                ch = np.flatnonzero(np.logical_or.reduce([GL.cap(g_) for g_ in grips]) & RARE)
                g, m, U = C.ghost(W, T, ch)
                n_park += 1
                nq_park += 1
                if not (g.alive[:m].all() and (g.dom[:m] == 0).all()):
                    continue
                n_land += 1
                nq_land += 1
                bad = U[~g.alive[m:] | (g.dom[m:] != 0)]
                cnt = {t: int((TYPE[bad] == t).sum()) for t in TYPES}
                ex = sum(max(0, cnt[t] - need[t]) for t in TYPES)
                hist[ex] += 1
                best = ex if best is None else min(best, ex)
                if ex == 0:
                    swaps.append({'W': [list(x) for x in W], 'by_type': cnt})
    by_q.append([nq, len(Ps), len(Zl), nq_park, nq_land, best])
    if nq % 5 == 0:
        print(json.dumps({'q': nq, 'of': len(words), 'parking_words': n_park, 'landing': n_land,
                          'hist': sorted(hist.items()), 'sec': round(time.time() - T0)}), flush=True)
doc = {'unit': int(T.min()), 'unit_by_type': need, 'family_words_with_lens': len(words), 'parking_words': n_park,
       'landing': n_land, 'excess_histogram': sorted(hist.items()), 'swap_words': len(swaps), 'examples': swaps[:10], 'per_q': by_q}
Path(out).write_text(json.dumps(doc) + '\n')
print(json.dumps({k: v for k, v in doc.items() if k not in ('examples', 'per_q')}))
