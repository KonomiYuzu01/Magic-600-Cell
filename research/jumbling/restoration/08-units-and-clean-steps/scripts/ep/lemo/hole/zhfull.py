"""(zhfull: refcert.py plus a full Lemma D check of every refinement and root that misses the given types.)
Certificate for every depth of commutator refinement of a unit's words (menu I_a).

Roots: the given rows (dfam2 q rows with key "Q", only those whose ghost lands T; word rows with key "W") and, with
--singles, every single letter (c, i) at any grip whose ghost lands T. For a root W0 the script follows the exact frame
ghost of T and of every lattice chamber of the given types in the caps of its grips and records D(W0), the chambers
of those types whose ghost dies. If D(W0) is empty, every refinement W1 = [W0, k] that lands T (every grip f whose cap
holds the landing L of W0, every non-identity K+ letter k at f with k(L) disjoint from L) is scored the same way:
its excess sum_t max(0, |Bad_t| - |T_t|) and D(W1). Rows W1 with D(W1) empty are written to --open for the next
depth (every N-th one with --open-every N).

By PROOFS-U Lemma I, every iterated refinement of a word W that lands T has |Bad_t| >= |D(W)_t| + |T_t|, so a root
or a W1 with a dead chamber of a type t closes the whole subtree for the swap condition of Corollary D'' in type t.

usage: refcert.py <theory> <echain> <table> <target json {pieces}> <out json> <rows.jsonl[.gz]>... --types 7,9
       [--singles] [--open <jsonl> [--open-every N]] [--part i/n]
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
OPEN = a[a.index('--open') + 1] if '--open' in a else None
OEVERY = int(a[a.index('--open-every') + 1]) if '--open-every' in a else 1
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


NT_ = int(np.asarray(GL.restore.ctx.regions.orbit_of).max()) + 1
_TY = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
FULL = open(a[a.index('--full') + 1], 'w') if '--full' in a else None


def full_check(W, kind):
    """|Bad(W)| per type over all chambers of W's caps against the holes per type at the state (Lemma D)."""
    holes = np.bincount(_TY[~C.occ], minlength=NT_)
    bad, _, _ = C.bad(W)
    bt = np.bincount(_TY[bad], minlength=NT_)
    d = np.maximum(bt - holes, 0)
    row = {'kind': kind, 'W': [list(x) for x in W], 'bad': int(len(bad)), 'occ': int(C.occ[bad].sum()),
           'deficit': int(d.sum()), 'deficit_types': {str(t): [int(bt[t]), int(holes[t])] for t in np.flatnonzero(d)}}
    if FULL:
        FULL.write(json.dumps(row) + '\n')
        FULL.flush()
    return row


def score(W):
    """(landing chambers or None, Bad count per type, dead count per type)."""
    grips = sorted({c for c, _ in W})
    ch = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in grips]) & RARE)
    g, m, U = C.ghost(W, T, ch)
    if not (g.alive[:m].all() and (g.dom[:m] == 0).all()):
        return None, None, None
    al, dm = g.alive[m:], g.dom[m:]
    bad = U[~al | (dm != 0)]
    dead = U[~al]
    return (g.q[:m].copy(), {t: int((TYPE[bad] == t).sum()) for t in TYPES},
            {t: int((TYPE[dead] == t).sum()) for t in TYPES})


roots = []
for f in files:
    op = gzip.open if f.endswith('.gz') else open
    rows = [json.loads(x) for x in op(f, 'rt') if x.strip()]
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
st = Counter()
hist, swaps, root_dead = Counter(), [], Counter()
fh = open(OPEN, 'w') if OPEN else None
for nw, Wl in enumerate(roots):
    W0 = [(int(c), int(i)) for c, i in Wl]
    L, cnt0, dead0 = score(W0)
    if L is None:
        st['root_not_landing'] += 1
        continue
    st['root_landing'] += 1
    st['root_len_%d' % len(W0)] += 1
    ex0 = sum(max(0, cnt0[t] - need[t]) for t in TYPES)
    st['root_excess0'] += int(ex0 == 0)
    if ex0 == 0:
        st['root_full_deficit0'] += int(full_check(W0, 'root')['deficit'] == 0)
    if sum(dead0.values()):
        st['root_closed_by_dead'] += 1
        root_dead[tuple(dead0[t] for t in TYPES)] += 1
        continue
    Lset = set(L.tolist())
    caps = np.flatnonzero(data.sig_matrix(L).sum(axis=0) == len(L)).tolist()
    Wi = CP.inverse_word(M, W0)
    for f in caps:
        for i, k, j in kletters(f):
            if Lset & set(FG.chamber_image(k, L).tolist()):
                continue
            W1 = W0 + [(f, i)] + Wi + [(f, j)]
            st['refinements_scored'] += 1
            L1, cnt, dead = score(W1)
            if L1 is None:
                continue
            st['refinements_landing'] += 1
            ex = sum(max(0, cnt[t] - need[t]) for t in TYPES)
            hist[ex] += 1
            if ex == 0:
                swaps.append({'W': [list(x) for x in W1], 'by_type': cnt})
                st['w1_full_deficit0'] += int(full_check(W1, 'w1')['deficit'] == 0)
            if sum(dead.values()):
                st['w1_closed_by_dead'] += 1
            else:
                st['w1_open'] += 1
                if fh and st['w1_open'] % OEVERY == 0:
                    fh.write(json.dumps({'W': [list(x) for x in W1], 'by_type': cnt, 'excess': ex}) + '\n')
    if nw % 200 == 0:
        print(json.dumps({'root': nw, 'of': len(roots), **st, 'hist': sorted(hist.items()),
                          'sec': round(time.time() - T0)}), flush=True)
if fh:
    fh.close()
doc = {'unit': int(T.min()), 'unit_by_type': need, 'types': TYPES, 'part': [PI, PN],
       'roots_from_files': n_files, 'singles': '--singles' in a, 'roots_this_part': len(roots), **st,
       'w1_excess_histogram': sorted(hist.items()), 'swap_words': len(swaps), 'swap_examples': swaps[:10],
       'root_dead_by_type': sorted([[list(k_), v] for k_, v in root_dead.items()]),
       'seconds': round(time.time() - T0)}
Path(out).write_text(json.dumps(doc) + '\n')
print(json.dumps({k_: v for k_, v in doc.items() if k_ != 'swap_examples'}))
