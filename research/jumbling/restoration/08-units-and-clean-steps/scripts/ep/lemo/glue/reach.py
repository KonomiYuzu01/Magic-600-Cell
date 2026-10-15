"""Which rotations can a carrier reach by admissible K+ letters, the other carriers fixed, and do they put its glued
holes onto the rare Bad set of a word (PROOFS-U Proposition GH)?

For a word V of admissible K+ letters that moves only carrier B (the other off blocks stay at their poses), B moves
rigidly by some g in K+, and its glued holes move to g(H_B). A letter at grip c is admissible and moves B iff no off
piece straddles cut c and cap c holds all of g(B); it then moves nothing else off iff cap c holds no piece of the
other blocks. The cut status of g(B) at grip c is the status of B at g^-1(c) (K+ maps cuts to cuts), so one exact
status vector per block gives the whole breadth-first search over g. For each word W of carrier A (rows), the
needed set is the rare part of Bad(W) minus A's glued holes; the script lists the g in K+ with g(H_B) equal to it and
whether the search reaches one of them, with a letter path.

usage: reach.py <theory> <echain> <table> <glued json> <out json> --types 7,9 --rows <A>:<rows.jsonl> ...
"""
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
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
from exact import matmul  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
table, gfile, out = sys.argv[3:6]
a = sys.argv[6:]
TYPES = [int(x) for x in a[a.index('--types') + 1].split(',')]
specs = a[a.index('--rows') + 1:]
CTX = GL.BL.CL.ctx
data, reg = CTX.data, CTX.regions
C = GL.Ctx(table)
st = C.st
kp = FG.kp
NK, NG = len(kp.perms), len(data.N)
IDK = FG.IDK
TYPE = np.asarray(reg.orbit_of, np.int64)
RARE = np.isin(TYPE, TYPES)
PERM = kp.perms.astype(np.int64)  # PERM[g][e] = g(e) on grip normals
INV = np.asarray(kp.inv, np.int64)


def status(pieces, poses=None):
    """grip -> 0 outside, 1 inside, 2 straddle, 3 both sides without a straddling piece."""
    groups = {}
    for n, p in enumerate(pieces):
        g = st.pose(int(p)) if poses is None else poses[n]
        groups.setdefault(to_tuple(g), (g, []))[1].append(int(p))
    res = []
    for e in range(NG):
        seen = set()
        for g, pcs in groups.values():
            ev = Evaluator(pullback_form(g, data.N[e]))
            for p in pcs:
                r = ev.classify(reg.vform(int(p)))[0]
                seen.add(r)
                if r == 'straddle':
                    break
            if 'straddle' in seen:
                break
        res.append(2 if 'straddle' in seen else (3 if len(seen) > 1 else (1 if 'in' in seen else 0)))
    return np.array(res)


blocks = {int(min(b)): np.asarray(sorted(b), np.int64) for b in C.bl.blocks if len(b)}
STAT = {u: status(b.tolist()) for u, b in blocks.items()}
print(json.dumps({'statuses': {str(u): np.bincount(s, minlength=4).tolist() for u, s in STAT.items()},
                  'sec': round(time.time() - T0)}), flush=True)

# self-check of the equivariance on one letter per block: status of k(B) recomputed exactly
STAB = {c: [int(k) for k in np.flatnonzero(PERM[:, c] == c) if k != IDK] for c in range(NG)}
chk = {}
for u, b in blocks.items():
    c = int(np.flatnonzero(STAT[u] == 1)[0])
    k = STAB[c][0]
    M = kp.matrix(k)
    moved = status(b.tolist(), [matmul(M, st.pose(int(p))) for p in b.tolist()])
    pred = STAT[u][PERM[INV[k]]]
    chk[str(u)] = bool(np.array_equal(moved, pred))
print(json.dumps({'equivariance_check': chk, 'sec': round(time.time() - T0)}), flush=True)
assert all(chk.values())

g = json.loads(Path(gfile).read_text())
H = {}
for r in g['rows']:
    H.setdefault(int(r['glued_to'][0]), []).append(int(r['hole']))
H = {u: np.array(sorted(v), np.int64) for u, v in H.items()}


def bfs(B, fixed):
    """Rotations g of B reachable by admissible K+ letters that move no other off piece; parent links."""
    sB = STAT[B]
    free = np.ones(NG, bool)
    for u in fixed:
        free &= STAT[u] == 0  # cap holds no piece of u and u does not straddle the cut
    par = {IDK: None}
    q = deque([IDK])
    while q:
        x = q.popleft()
        s = sB[PERM[INV[x]]]  # status of x(B) at every grip
        for c in np.flatnonzero((s == 1) & free).tolist():
            for k in STAB[c]:
                y = int(kp.compose(k, x))
                if y not in par:
                    par[y] = (x, c, k)
                    q.append(y)
    return par


def path(par, y):
    out_ = []
    while par[y] is not None:
        x, c, k = par[y]
        out_.append([c, k])
        y = x
    return out_[::-1]


IMGS = {u: np.array([np.sort(FG.chamber_image(k, h)) for k in range(NK)]) for u, h in H.items()}
REACH = {}
res = []
for spec in specs:
    A, rf = spec.split(':', 1)
    A = int(A)
    others = sorted(u for u in H if u != A)
    for B in others:
        fixed = [u for u in blocks if u != B]
        if (A, B) not in REACH:
            REACH[(A, B)] = bfs(B, fixed)
            print(json.dumps({'A': A, 'B': B, 'reachable_rotations': len(REACH[(A, B)]),
                              'sec': round(time.time() - T0)}), flush=True)
    for n, line in enumerate(open(rf)):
        if not line.strip():
            continue
        W = [(int(c), int(i)) for c, i in json.loads(line)['W']]
        grips = sorted({c for c, _ in W})
        ch = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in grips]) & RARE)
        bad, _, _ = C.bad(W, ch)
        need = np.array(sorted(set(bad.tolist()) - set(H[A].tolist())), np.int64)
        rec = {'A': A, 'row': n, 'need': need.tolist(), 'by_other': {}}
        for B in others:
            hit = [k for k in range(NK)] if len(need) == 0 else \
                np.flatnonzero((IMGS[B] == need[None, :]).all(axis=1)).tolist() if len(need) == len(H[B]) else []
            par = REACH[(A, B)]
            reach = [k for k in hit if k in par]
            rec['by_other'][str(B)] = {'rotations_onto_need': len(hit), 'reachable': len(reach),
                                       'example': {'g': reach[0], 'letters': path(par, reach[0])} if reach else None,
                                       'all': [{'g': k, 'letters': path(par, k)} for k in reach]}
        res.append(rec)
        print(json.dumps({'A': A, 'row': n, **{b: (v['rotations_onto_need'], v['reachable'])
                                              for b, v in rec['by_other'].items()}}), flush=True)
doc = {'table': Path(table).name, 'types': TYPES, 'statuses': {str(u): np.bincount(s, minlength=4).tolist()
                                                              for u, s in STAT.items()},
       'equivariance_check': chk, 'reachable': {f'{A}->{B}': len(p) for (A, B), p in REACH.items()},
       'words': len(res), 'words_with_reachable_cover': sum(1 for r in res if any(v['reachable']
                                                                                  for v in r['by_other'].values())),
       'rows': res, 'seconds': round(time.time() - T0)}
Path(out).write_text(json.dumps(doc) + '\n')
print(json.dumps({k: v for k, v in doc.items() if k != 'rows'}))
