"""(refjz: refj.py plus a full Lemma D check, over every type, of every root, W1 and W2 whose Bad set misses the given
types; written with --full.)
Depth-two certificate for commutator refinements of a unit's words, by the refinement identity (PROOFS-U
Proposition J), with every depth-one prediction checked against the exact ghost.

For a root W0 (rows as in refcert.py: dfam2 q rows, word rows, and with --singles every single letter) that lands T
with D(W0) empty in the given types:
- every candidate W1 = [W0, k1] (k1 a non-identity K+ letter at a grip f whose cap holds the landing L of W0, k1(L)
  disjoint from L) is decided by Proposition J from the data of W0: landing iff k1~(L) misses Bad(W0^-1), and
  |Bad(W1)_t| = |D(W0)_t| + #{w in B'_t : k1~(w) not in B'} + #{y in S(W0)_t : k1(y) dies or is not in S(W0)};
- every W1 predicted to land is then ghosted exactly (`check` counts the agreements of landing and of |Bad(W1)_t|);
- for every W1 with D(W1) empty in the given types, every W2 = [W1, k2] is decided the same way from the data of W1.
A W2 with excess 0 that lands is written to `swap_words` and ghosted exactly.

usage: refj.py <theory> <echain> <table> <target json> <out json> <rows>... --types 7,9 [--singles] [--part i/n]
"""
import gzip
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
import conjprep as CP  # noqa: E402
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
from exact import identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
table, tfile, out = sys.argv[3:6]
a = sys.argv[6:]
cut = min([a.index(x) for x in a if x.startswith('--')] + [len(a)])
files = a[:cut]
TYPES = [int(x) for x in a[a.index('--types') + 1].split(',')]
PI, PN = (int(x) for x in a[a.index('--part') + 1].split('/')) if '--part' in a else (0, 1)
C = GL.Ctx(table)
M, data = C.M, GL.data
kp = GL.BL.CL.ctx.kplus
KID = kp.index_of_matrix(to_tuple(identity(4)))
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
RARE = np.isin(TYPE, TYPES)
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
need = {t: int((TYPE[T] == t).sum()) for t in TYPES}
_kl = {}


def kletters(c):
    if c not in _kl:
        res = []
        for i in range(len(M.items)):
            k = kp.index_of_matrix(GL.restore.element(M, c, i))
            if k >= 0 and k != KID:
                res.append((i, int(k), int(CP.inverse_word(M, [(c, i)])[0][1])))
        _kl[c] = res
    return _kl[c]


NT_ = int(TYPE.max()) + 1
FULL = open(a[a.index('--full') + 1], 'w') if '--full' in a else None
HOLES = np.bincount(TYPE[~C.occ], minlength=NT_)


def full_check(W, kind):
    """|Bad(W)| per type over all chambers of W's caps against the holes per type at the state (Lemma D)."""
    bad, _, _ = C.bad(W)
    bt = np.bincount(TYPE[bad], minlength=NT_)
    d = np.maximum(bt - HOLES, 0)
    row = {'kind': kind, 'W': [list(x) for x in W], 'bad': int(len(bad)), 'occ': int(C.occ[bad].sum()),
           'deficit': int(d.sum()), 'deficit_types': {str(t): [int(bt[t]), int(HOLES[t])] for t in np.flatnonzero(d)}}
    if FULL:
        FULL.write(json.dumps(row) + '\n')
        FULL.flush()
    return row


def rare_in_caps(W):
    grips = sorted({c for c, _ in W})
    return np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in grips]) & RARE)


def forward(W):
    """Landing L of T (None if T does not land), D(W) and S(W) in the given types."""
    R = rare_in_caps(W)
    g, m, _ = C.ghost(W, T, R)
    if not (g.alive[:m].all() and (g.dom[:m] == 0).all()):
        return None
    al, dm, qq = g.alive[m:], g.dom[m:], g.q[m:]
    off = al & (dm != 0)
    D = R[~al]
    S = (dm[off].copy(), qq[off].copy(), TYPE[R[off]].copy())
    return g.q[:m].copy(), D, S


def candidates(L):
    Lset = set(L.tolist())
    caps = np.flatnonzero(data.sig_matrix(L).sum(axis=0) == len(L)).tolist()
    res = []
    for f in caps:
        for i, k, j in kletters(f):
            kL = FG.chamber_image(k, L)
            if Lset & set(kL.tolist()):
                continue
            res.append((f, i, k, j, kL))
    return res


def predict(W, L, D, S):
    """Proposition J for every candidate refinement of W: list of (f, i, j, by-type Bad counts)."""
    cand = candidates(L)
    if not cand:
        return []
    R = rare_in_caps(W)
    ch = np.unique(np.concatenate([R] + [c[4] for c in cand]))
    gi, _, _ = C.ghost(CP.inverse_word(M, W), (), ch)
    bad = ~gi.alive | (gi.dom != 0)
    Bset = set(ch[bad].tolist())
    Bp = ch[bad & RARE[ch]]
    Dc = {t: int((TYPE[D] == t).sum()) for t in TYPES}
    Sset = set(zip(S[0].tolist(), S[1].tolist()))
    res = []
    for f, i, k, j, kL in cand:
        if Bset & set(kL.tolist()):
            continue
        kB = np.where(GL.cap(f)[Bp], FG.chamber_image(k, Bp), Bp)  # k~: k on cap f, identity outside
        lat = Counter(int(TYPE[w]) for w, w2 in zip(Bp.tolist(), kB.tolist()) if w2 not in Bset)
        sterm = Counter()
        if len(S[0]):
            gs = FG.Ghost(C.F, S[0], S[1])
            gs.apply(f, C.E(f, i), (f, i))
            for n in range(len(S[0])):
                if not gs.alive[n] or (int(gs.dom[n]), int(gs.q[n])) not in Sset:
                    sterm[int(S[2][n])] += 1
        res.append((f, i, j, {t: Dc[t] + lat[t] + sterm[t] for t in TYPES}))
    return res


def excess(cnt):
    return sum(max(0, cnt[t] - need[t]) for t in TYPES)


roots = []
for fn in files:
    op = gzip.open if fn.endswith('.gz') else open
    rows = [json.loads(x) for x in op(fn, 'rt') if x.strip()]
    if rows and 'Q' in rows[0]:
        tmax = max(r['T_on'] for r in rows)
        rows = [{'W': r['Q']} for r in rows if r['T_dead'] == 0 and r['T_on'] == tmax]
    roots += [r['W'] for r in rows]
n_files = len(roots)
if '--singles' in a:
    for c in range(600):
        for i in range(len(M.items)):
            roots.append([[c, i]])
seen, uniq = set(), []
for W in roots:
    key = json.dumps(W)
    if key not in seen:
        seen.add(key)
        uniq.append(W)
roots = uniq[PI::PN]
st, h1, h2 = Counter(), Counter(), Counter()
swaps, mism = [], []
for nw, Wl in enumerate(roots):
    W0 = [(int(c), int(i)) for c, i in Wl]
    r0 = forward(W0)
    if r0 is None:
        st['root_not_landing'] += 1
        continue
    st['root_landing'] += 1
    L0, D0, S0 = r0
    if not len(D0) and not len(S0[0]):
        st['root_full_deficit0'] += int(full_check(W0, 'root')['deficit'] == 0)
    if len(D0):
        st['root_closed_by_dead'] += 1
        continue
    Wi0 = CP.inverse_word(M, W0)
    for f, i, j, cnt in predict(W0, L0, D0, S0):
        st['w1_predicted_landing'] += 1
        W1 = W0 + [(f, i)] + Wi0 + [(f, j)]
        r1 = forward(W1)
        if r1 is None:
            mism.append({'W1': [list(x) for x in W1], 'why': 'predicted landing, ghost does not land'})
            continue
        L1, D1, S1 = r1
        # exact Bad(W1) in the given types: D(W1) plus the off contents S(W1)
        exact = {t: int((TYPE[D1] == t).sum()) + int((S1[2] == t).sum()) for t in TYPES}
        if exact != cnt:
            mism.append({'W1': [list(x) for x in W1], 'pred': cnt, 'exact': exact})
        else:
            st['w1_check_ok'] += 1
        h1[excess(exact)] += 1
        if excess(exact) == 0:
            st['w1_full_deficit0'] += int(full_check(W1, 'w1')['deficit'] == 0)
        if len(D1):
            st['w1_closed_by_dead'] += 1
            continue
        st['w1_open'] += 1
        Wi1 = CP.inverse_word(M, W1)
        for f2, i2, j2, cnt2 in predict(W1, L1, D1, S1):
            st['w2_landing'] += 1
            ex2 = excess(cnt2)
            h2[ex2] += 1
            if ex2 == 0:
                W2 = W1 + [(f2, i2)] + Wi1 + [(f2, j2)]
                r2 = forward(W2)
                ok = r2 is not None and {t: int((TYPE[r2[1]] == t).sum()) + int((r2[2][2] == t).sum())
                                         for t in TYPES} == cnt2
                fc = full_check(W2, 'w2')
                st['w2_full_deficit0'] += int(fc['deficit'] == 0)
                swaps.append({'W2': [list(x) for x in W2], 'by_type': cnt2, 'ghost_confirms': bool(ok),
                              'full_deficit': fc['deficit']})
    if nw % 200 == 0:
        print(json.dumps({'root': nw, 'of': len(roots), **st, 'mismatches': len(mism), 'h1': sorted(h1.items()),
                          'h2': sorted(h2.items()), 'swaps': len(swaps), 'sec': round(time.time() - T0)}), flush=True)
doc = {'unit': int(T.min()), 'unit_by_type': need, 'types': TYPES, 'part': [PI, PN], 'roots_from_files': n_files,
       'singles': '--singles' in a, 'roots_this_part': len(roots), **st, 'mismatches': len(mism),
       'mismatch_examples': mism[:10], 'w1_excess_histogram': sorted(h1.items()),
       'w2_excess_histogram': sorted(h2.items()), 'swap_words_depth2': len(swaps), 'swap_examples': swaps[:10],
       'seconds': round(time.time() - T0)}
Path(out).write_text(json.dumps(doc) + '\n')
print(json.dumps({k_: v for k_, v in doc.items() if k_ not in ('swap_examples', 'mismatch_examples')}))
