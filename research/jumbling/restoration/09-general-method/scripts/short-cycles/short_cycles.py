"""Short pure elements of the retained group from single-overlap commutators (SENIOR-REVIEW.md section 11).

Let supp X be the set of pieces that an element X moves, to another position or in place. If supp X and supp Y meet
in exactly one piece q, the commutator C = [X, Y] = X Y X^-1 Y^-1 moves at most three pieces, all in the orbit of q:
- 'three-cycle': X and Y both move q to another position; C permutes the positions of X^-1(q), q and Y^-1(q)
  cyclically. It is pure when C^3 = 1 on all slots, which holds whenever the orbit's frame group H_o is trivial;
- 'pair twist': exactly one of X and Y keeps q at its position; C twists two pieces in place;
- 'single twist': both keep q at its position; C twists q in place (only when H_o is not abelian).
X and Y range over the spine nodes of the seed atlas (`assets/seed_atlas.json`) of at most MAXLEN letters, and Y is
placed by every sigma in K+ (7,200 placements, `kplus.py`): Y^sigma is written as the transported word, each letter
at cap c with rotation r becoming a shortest word at cap sigma(c) for sigma r sigma^-1. The search ranks the pairs
by the exact length of C in primitives and keeps the six shortest per orbit and kind. It then verifies each kept pair
exactly (the transported word's permutation equals the conjugate of Y by the slot map, the support, the kind, and
C^3 = 1 for a pure three-cycle) and records the shortest verified C per orbit and kind with its word.
`check_cycles.py` verifies the recorded words independently from the primitives alone.

usage (from the repository root): python3 -I -B short_cycles.py MAXLEN out.json
The committed result `data/short-cycles.json.gz` came from MAXLEN 12286 (all 122 spine nodes).
Research only: exact synthetic geometry in headless Linux runs; no Windows, DirectX, input or performance claim.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import kplus as KP  # noqa: E402

MAXLEN, OUT = int(sys.argv[1]), sys.argv[2]
T0 = time.time()
NS, NP, sp, rot = KP.NS, KP.NP, KP.sp, KP.rot
orb = KP.m['orbit_id'].astype(np.int64)
ID = np.arange(NS, dtype=np.int64)
_, FIRST = np.unique(sp, return_index=True)  # one slot of every piece


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), flush=True)


def inv(P):
    Q = np.empty_like(P)
    Q[P] = ID
    return Q


PR = {}


def prim(g):
    if g not in PR:
        P = KP.prim_perm(abs(g))
        PR[g] = P if g > 0 else inv(P)
    return PR[g]


def word_perm(w):
    P = ID
    for g in w:
        P = prim(g)[P]  # first P, then g
    return P


def expand(t):
    if t[0] == 'g':
        return [t[1]]
    if t[0] == 'pow':
        return expand(t[1]) * t[2]
    x, y = expand(t[1]), expand(t[2])
    return x + y + [-g for g in reversed(x)] + [-g for g in reversed(y)]


def spine(t):
    out = []
    while t[0] in ('comm', 'pow'):
        out.append(t)
        t = t[1]
    return out[::-1]


atlas = json.loads(Path('assets/seed_atlas.json').read_text())
nodes, seen = [], set()
for e in atlas:
    for nd in spine(e['ast']):
        k = json.dumps(nd)
        if k not in seen:
            seen.add(k)
            w = expand(nd)
            if len(w) <= MAXLEN:
                nodes.append({'ast': nd, 'word': w, 'from_orbit': e['orbit']})
M = np.zeros((len(nodes), NP), bool)  # M[i, q]: node i moves piece q
FIX = np.zeros((len(nodes), NP), bool)  # FIX[i, q]: node i twists piece q in place
for i, d in enumerate(nodes):
    P = word_perm(d['word'])
    S = np.unique(sp[np.flatnonzero(P != ID)])
    d['S'] = S
    M[i, S] = True
    FIX[i, S[sp[P[FIRST[S]]] == S]] = True
    PR.clear()
USED = sorted({abs(g) for d in nodes for g in d['word']})
CNT = np.zeros((len(nodes), len(USED)), np.int64)  # letters of each used primitive, either sign
for i, d in enumerate(nodes):
    for g in d['word']:
        CNT[i, USED.index(abs(g))] += 1
log(nodes=len(nodes), used_primitives=len(USED))


def a4_words(c):
    """Shortest words in H_c, T_c and T_c^-1 for the 12 rotations of cap c, keyed by their cap permutations."""
    gens = {2 * c + 1: rot[2 * c], 2 * c + 2: rot[2 * c + 1], -(2 * c + 2): np.argsort(rot[2 * c + 1])}
    ident = np.arange(600, dtype=np.int64)
    best = {ident.tobytes(): []}
    frontier = [(ident, [])]
    while frontier:
        nxt = []
        for x, w in frontier:
            for g, r in gens.items():
                y = r[x]
                if y.tobytes() not in best:
                    best[y.tobytes()] = w + [g]
                    nxt.append((y, w + [g]))
        frontier = nxt
    assert len(best) == 12
    return best


A4 = [a4_words(c) for c in range(600)]


def conj_word(g, sig, sinv):
    a = abs(g)
    r = rot[a - 1] if g > 0 else np.argsort(rot[a - 1])
    return A4[int(sig[(a - 1) // 2])][sig[r[sinv]].tobytes()]


def transport(word, sig):
    sinv = np.argsort(sig)
    return [h for g in word for h in conj_word(g, sig, sinv)]


K = KP.group()
assert len(K) == 7200
KINDS = {(False, False): 'three-cycle', (True, False): 'pair twist', (False, True): 'pair twist',
         (True, True): 'single twist'}
cand = {}  # (orbit, kind) -> up to six (length, i, j, sigma index, q), shortest first
hits = 0
for si, sig in enumerate(K):
    sinv = np.argsort(sig)
    ell = np.array([len(conj_word(a, sig, sinv)) for a in USED])
    tl = CNT @ ell  # tl[j]: length of the transported word of node j
    pm = KP.piece_map(sig)
    for j, dy in enumerate(nodes):
        img = pm[dy['S']]
        cnt = M[:, img].sum(axis=1)
        for i in np.flatnonzero(cnt == 1):
            hits += 1
            k = int(np.flatnonzero(M[i, img])[0])
            q = int(img[k])
            kind = KINDS[(bool(FIX[i, q]), bool(FIX[j, dy['S'][k]]))]
            L = 2 * (len(nodes[i]['word']) + int(tl[j]))
            lst = cand.setdefault((int(orb[q]), kind), [])
            if len(lst) < 6 or L < lst[-1][0]:
                lst.append((L, int(i), j, si, q))
                lst.sort()
                del lst[6:]
    if si % 1200 == 1199:
        log(sigma=si + 1, hits=hits, keys=len(cand))
res = {}
for (o, kind), lst in sorted(cand.items()):
    for L, i, j, si, q in lst:
        dx, dy, sig = nodes[i], nodes[j], K[si]
        if f'{o}:{kind}' in res and res[f'{o}:{kind}']['C_len'] <= L:
            break
        Sm = KP.slot_map(sig)
        Py = word_perm(dy['word'])
        Ys = np.empty_like(Py)
        Ys[Sm] = Sm[Py]  # Y conjugated by the slot map
        wt = transport(dy['word'], sig)
        exact = bool(np.array_equal(word_perm(wt), Ys))
        X = word_perm(dx['word'])
        C = inv(Ys)[inv(X)[Ys[X]]]  # X, then Ys, then X^-1, then Ys^-1
        PR.clear()
        mv = np.flatnonzero(C != ID)
        pcs = np.unique(sp[mv])
        stay = [bool(np.all(sp[C[mv[sp[mv] == p]]] == p)) for p in pcs]
        if len(pcs) == 3 and not any(stay):
            got = 'three-cycle'
        elif len(pcs) in (1, 2) and all(stay):
            got = {1: 'single twist', 2: 'pair twist'}[len(pcs)]
        else:
            got = 'other'
        pure = bool(np.array_equal(C[C[C]], ID)) if got == 'three-cycle' else True
        assert exact and got == kind and q in pcs and np.all(orb[pcs] == o), (o, kind, got, exact)
        if not pure:
            got = 'twisted three-cycle'
        order, P = 1, C
        while not np.array_equal(P, ID):
            P, order = C[P], order + 1
        word = dx['word'] + wt + [-g for g in reversed(dx['word'])] + [-g for g in reversed(wt)]
        assert len(word) == L
        key = f'{o}:{got}'
        if key not in res or L < res[key]['C_len']:
            res[key] = {'orbit': o, 'kind': got, 'C_len': L, 'order': order, 'X_len': len(dx['word']),
                        'Y_len': len(dy['word']), 'X_from_orbit': dx['from_orbit'], 'Y_from_orbit': dy['from_orbit'],
                        'X': dx['ast'], 'Y': dy['ast'], 'sigma_index': si, 'overlap_piece': q,
                        'C_pieces': [int(p) for p in pcs], 'C_word': word}
for key, r in sorted(res.items(), key=lambda kv: (kv[1]['orbit'], kv[1]['kind'])):
    log(**{k: v for k, v in r.items() if k not in ('X', 'Y', 'C_word')})
pure3 = sorted(r['orbit'] for r in res.values() if r['kind'] == 'three-cycle')
log(pure_three_cycle_orbits=pure3, count=len(pure3), missing=[o for o in range(35) if o not in pure3])
Path(OUT).write_text(json.dumps(res))
