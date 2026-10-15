"""Relocate shorter elements onto the cascade's buffers and certify them as replacement seeds
(SENIOR-REVIEW.md section 12).

For every orbit o the committed cascade (stage 03, `kit.py`) uses c_o, the atlas seed conjugated by its relocation:
a three-cycle A -> B -> C -> A of positions on orbit o whose collateral lies on orbits solved after o. Candidates
for a shorter replacement:
- the pure three-cycles of `data/short-cycles.json.gz` and `data/short-cycles-2.json.gz` (S1, S2);
- the spine nodes of the orbit's own seed whose action on orbit o is a three-cycle of positions and whose
  collateral lies on orbits solved after o (M2-06).
A candidate P on the positions (p_i, p_j, p_k) is relocated by g = w1 w2 w3: w1 is a shortest word moving p_i to A,
w2 a shortest word of letters that fix position A moving the image of p_j to B, and w3 the inverse of the tree path
to the image of p_k, which fixes A and B. The replacement is c' = g^-1 P g (chronological), or its inverse when its
direction is A -> C -> B, freely reduced. Every chosen word is verified on all 259,800 slots: its orbit-o part is
the position three-cycle A -> B -> C -> A, its collateral orbits are solved after o, and the fibre maps are recorded
(aligned = it maps the label slots of A to B, B to C and C to A index by index, as the committed c_o does). Any
fibres are accepted: on trivial and abelian H_o the committed kit needs no alignment, and on D5 and A5
`restore_realigned.py` re-indexes the pairs (SENIOR-REVIEW.md section 12). The bound of M2-04 is then recomputed
with the replaced lengths.
usage (from the repository root): python3 -I -B realign.py out.json
Research only: exact synthetic geometry; no Windows, DirectX, input or performance claim.
"""
import gzip
import json
import sys
from fractions import Fraction as F
from itertools import permutations
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
GM = HERE.parents[1]                      # 09-general-method
sys.path.insert(0, str(GM.parent / '03-retained-group-and-lattice-restorer' / 'scripts'))
from rlib import Retained, NS, invword   # noqa: E402

OUT = sys.argv[1]
R = Retained('.')
ID = np.arange(NS, dtype=np.int64)
HG = {0: 'C2', 3: 'C2', 8: 'C2', 11: 'C2', 22: 'C2', 25: 'C2', 32: 'C2', 17: 'C5', 28: 'C5', 6: 'D5', 34: 'A5'}
FIRST = R.psorted[R.poff[:-1]]           # piece -> its first slot
LETTERS = [m for a in range(1, 1201) for m in (a, -a)]


def reduce_word(w):
    out = []
    for m in w:
        if out and out[-1] == -m:
            out.pop()
        else:
            out.append(int(m))
    return out


def expand(t):
    if t[0] == 'g':
        return [t[1]]
    if t[0] == 'pow':
        return expand(t[1]) * t[2]
    x, y = expand(t[1]), expand(t[2])
    return x + y + invword(x) + invword(y)


def spine(t):
    out = []
    while t[0] in ('comm', 'pow'):
        out.append(t)
        t = t[1]
    return out[::-1]


# ---- the committed order (as restore_lattice.py lines 49 to 60) ------------------------------
coll = {}
for o in range(35):
    P = R.word_perm(R.trees[o]['seed'])
    coll[o] = set(int(x) for x in R.so[np.flatnonzero(P != ID)]) - {o}
order, done = [], set()
while len(order) < 35:
    for o in [o for o in range(35) if o not in done and coll[o] <= done]:
        order.append(o)
        done.add(o)
cascade = order[::-1]
after = {o: set(cascade[i + 1:]) for i, o in enumerate(cascade)}

cyc = {}
for path in (GM / 'data' / 'short-cycles.json.gz', GM / 'data' / 'short-cycles-2.json.gz'):
    for r in json.loads(gzip.decompress(path.read_bytes())).values():
        if r['kind'] == 'three-cycle':
            cyc.setdefault(r['orbit'], []).append(r)


def orbit_tools(o):
    t, at = R.trees[o], R.atlas[o]
    A, B, C = int(t['buffers'][0]), int(t['buffers'][1]), int(t['third'])
    pcs = R.orbit_pieces(o)
    ix = np.full(R.np, -1, np.int64)
    ix[pcs] = np.arange(len(pcs))
    fs = FIRST[pcs]
    pm = {}
    for m in LETTERS:
        s, d = R.move(m)
        j = np.minimum(np.searchsorted(s, fs), len(s) - 1)
        hit = s[j] == fs
        img = fs.copy()
        img[hit] = d[j[hit]]
        pm[m] = ix[R.sp[img]]
    assert all((v >= 0).all() for v in pm.values())
    lab = {A: np.array(at['frames'][0], np.int64), B: np.array(at['frames'][1], np.int64)}
    node_at = {}
    for n, p in enumerate(t['positions']):
        if p not in node_at:
            node_at[p] = n
            lab[p] = np.array(t['frames'][n], np.int64)
    assert node_at[C] == 0 and set(lab) == set(int(p) for p in pcs)
    return dict(A=A, B=B, C=C, pcs=pcs, ix=ix, pm=pm, lab=lab, node_at=node_at, t=t)


def bfs_to(T, pm, n, letters):
    """dist[x], nxt[x]: a shortest word from position index x to T uses nxt[x] first."""
    inv = {m: np.argsort(pm[m]) for m in letters}
    dist = np.full(n, -1, np.int64)
    nxt = np.zeros(n, np.int64)
    dist[T] = 0
    front = np.array([T])
    k = 0
    while len(front):
        new = []
        for m in letters:
            y = inv[m][front]
            y = y[dist[y] < 0]
            dist[y] = k + 1
            nxt[y] = m
            new.append(y)
        front = np.unique(np.concatenate(new)) if new else np.zeros(0, np.int64)
        k += 1
    return dist, nxt


def walk(x, T, pm, nxt):
    w = []
    while x != T:
        m = int(nxt[x])
        w.append(m)
        x = int(pm[m][x])
    return w


def tree_path(t, n):
    w = []
    while t['parent'][n] >= 0:
        w.append(int(t['move'][n]))
        n = t['parent'][n]
    return w[::-1]


def apply_pos(w, x, pm):
    for m in w:
        x = int(pm[m][x])
    return x


def three_cycle(P, o):
    """Positions (p, P(p), P^2(p)) if the orbit-o part of P moves exactly three pieces cyclically, else None."""
    mv = np.flatnonzero(P != ID)
    on = mv[R.so[mv] == o]
    pcs = np.unique(R.sp[on])
    if len(pcs) != 3:
        return None
    img = {int(p): int(R.sp[P[FIRST[p]]]) for p in pcs}
    p = int(pcs[0])
    if sorted(img.values()) != sorted(img) or img[p] == p or img[img[img[p]]] != p or img[img[p]] == p:
        return None
    return [p, img[p], img[img[p]]]


def certify(o, T, P_word, src):
    """Relocate P_word onto (A, B, C) and verify exactly. Returns a record or None."""
    pm, ix, A, B, C = T['pm'], T['ix'], T['A'], T['B'], T['C']
    P = R.word_perm(P_word)
    trip = three_cycle(P, o)
    if trip is None:
        return None
    best = None
    for pi, pj, pk in permutations(trip):
        a, b, c = int(ix[pi]), int(ix[pj]), int(ix[pk])
        if T['dA'][a] < 0:
            continue
        w1 = walk(a, int(ix[A]), pm, T['nA'])
        b1, c1 = apply_pos(w1, b, pm), apply_pos(w1, c, pm)
        if T['dB'][b1] < 0:
            continue
        w2 = walk(b1, int(ix[B]), pm, T['nB'])
        c2 = apply_pos(w2, c1, pm)
        w3 = invword(tree_path(T['t'], T['node_at'][int(T['pcs'][c2])]))
        g = w1 + w2 + w3
        if best is None or len(g) < len(best):
            best = g
    if best is None:
        return None
    word = reduce_word(invword(best) + list(P_word) + best)
    Q = R.word_perm(word)
    pos = {X: int(R.sp[Q[FIRST[X]]]) for X in (A, B, C)}
    if pos[A] == C:
        word = invword(word)
        Q = np.argsort(Q)
        pos = {X: int(R.sp[Q[FIRST[X]]]) for X in (A, B, C)}
    assert pos == {A: B, B: C, C: A}, (o, pos)
    mv = np.flatnonzero(Q != ID)
    on = mv[R.so[mv] == o]
    assert set(R.sp[on].tolist()) == {A, B, C}, o
    col = sorted(set(int(x) for x in R.so[mv]) - {o})
    lab = T['lab']
    aligned = all(np.array_equal(Q[lab[x]], lab[y]) for x, y in ((A, B), (B, C), (C, A)))
    cube = np.array_equal(Q[Q[Q[on]]], on)
    return {'orbit': o, 'source': src, 'P_len': len(P_word), 'g_len': len(best), 'len': len(word),
            'collateral': col, 'legal': set(col) <= after[o], 'pure': not col, 'aligned_fibres': bool(aligned),
            'cube_identity_on_orbit': bool(cube), 'word': word}


res = {}
for o in range(35):
    t = R.trees[o]
    T = orbit_tools(o)
    n = len(T['pcs'])
    T['dA'], T['nA'] = bfs_to(int(T['ix'][T['A']]), T['pm'], n, LETTERS)
    fixA = [m for m in LETTERS if T['pm'][m][T['ix'][T['A']]] == T['ix'][T['A']]]
    T['dB'], T['nB'] = bfs_to(int(T['ix'][T['B']]), T['pm'], n, fixA)
    for X, nd in T['node_at'].items():
        u = tree_path(t, nd)
        assert apply_pos(u, int(T['ix'][T['C']]), T['pm']) == T['ix'][X]
        assert apply_pos(u, int(T['ix'][T['A']]), T['pm']) == T['ix'][T['A']]
        assert apply_pos(u, int(T['ix'][T['B']]), T['pm']) == T['ix'][T['B']]
    s0 = len(t['seed']) + 2 * len(t['relocation'])
    cands = []
    for r in cyc.get(o, []):
        c = certify(o, T, r['C_word'], 'pure three-cycle (%s), level %d' % (r['X'][:60], r.get('level', 1)))
        if c:
            cands.append(c)
    for nd in spine(R.atlas[o]['ast']):
        w = expand(nd)
        if len(w) >= s0:
            continue
        c = certify(o, T, w, 'spine node of O%02d, %d letters' % (o, len(w)))
        if c and c['legal']:
            cands.append(c)
    ok = [c for c in cands if c['legal']]
    best = min(ok, key=lambda c: c['len']) if ok else None
    use = best if best and best['len'] < s0 else None
    res[o] = {'orbit': o, 'H': HG.get(o, '1'), 'N': n, 'seed_s0': s0, 'tree_depth': max(t['depth']),
              'replaced': use is not None, 's': use['len'] if use else s0,
              'reindex': bool(use and not use['aligned_fibres'] and HG.get(o) in ('D5', 'A5')),
              'choice': {k: v for k, v in use.items()} if use else None,
              'candidates': [{k: v for k, v in c.items() if k != 'word'} for c in cands]}
    print(json.dumps({'orbit': o, 'H': HG.get(o, '1'), 'seed_s0': s0, 's': res[o]['s'],
                      'source': use['source'] if use else 'seed', 'g_len': use['g_len'] if use else None,
                      'aligned': use['aligned_fibres'] if use else True, 'pure': use['pure'] if use else None,
                      'n_candidates': len(cands)}), flush=True)

# ---- the bound of M2-04 with the replaced lengths --------------------------------------------
Hn = {0: 2, 3: 2, 6: 10, 8: 2, 11: 2, 17: 5, 22: 2, 25: 2, 28: 5, 32: 2, 34: 60}
h = {0: 5, 3: 3, 6: 4, 8: 3, 11: 3, 17: 2, 22: 3, 25: 2, 28: 2, 32: 2, 34: 3}
bound = {}
for mode in ('committed', 'replaced'):
    total, expect, create = 0, F(0), 0
    for o in range(35):
        N, d = res[o]['N'], res[o]['tree_depth']
        s = res[o]['seed_s0'] if mode == 'committed' else res[o]['s']
        w = s + 2 * d
        ES = F(N - 5) + sum((F(1, j) for j in range(1, N + 1)), F(0)) + F(3, N) + F(1, N * (N - 1))
        total += (3 * N // 2 - 3) * w
        expect += ES * w
        create += 2 * s
        if o in Hn:
            P4 = 4 * s + 2 * h[o] + 4 * d
            PB = 2 * s + 2 * h[o]
            SC = 4 * s + (16 if o == 6 else 10) if o in (6, 34) else 0
            total += (N - 2) * P4 + PB + SC
            expect += F(Hn[o] - 1, Hn[o]) * ((N - 2) * P4 + PB)
            if o in (6, 34):
                expect += (F(4, 5) if o == 6 else F(59, 60)) * SC
    bound[mode] = {'worst': total, 'expected_ceiling': -(-expect.numerator // expect.denominator),
                   'creation_70': create}
print(json.dumps({'bound': bound, 'replaced_orbits': sorted(o for o in res if res[o]['replaced'])}), flush=True)
Path(OUT).write_text(json.dumps({'cascade_order': cascade, 'orbits': [res[o] for o in range(35)], 'bound': bound}))
