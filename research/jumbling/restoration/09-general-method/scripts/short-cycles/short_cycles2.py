"""Two-level single-overlap search for the orbits that `short_cycles.py` leaves without a short pure three-cycle
(SENIOR-REVIEW.md section 11).

If supp X and supp Y meet in a set I of pieces, the commutator [X, Y] moves only pieces of I, X^-1(I) and Y^-1(I),
so at most 3|I| pieces.
- Level 1: Z = [X, Y^sigma] for spine nodes X and Y of the seed atlas and sigma in K+ whose supports meet in 2 to
  KMAX pieces (default 3), at least one of them in a target orbit. Z then moves at most 3 KMAX pieces. For each target orbit the
  search keeps the K1 shortest such pairs, computes Z exactly and keeps those that move a piece of a target orbit.
- Level 2: C = [Z, W^tau] for a kept Z, W a spine node or a kept Z, and tau in K+, whose supports meet in exactly one
  piece q of a target orbit. By the single-overlap lemma C is a three-cycle of positions, a pair twist or a single
  twist in q's orbit. The six shortest per orbit and kind are verified exactly as in `short_cycles.py`, and the
  shortest verified C per orbit and kind is written with its word, in the record format of `short_cycles.py`, so
  that `check_cycles.py` checks it.
usage (from the repository root): python3 -I -B short_cycles2.py MAXLEN TARGETS K1 out.json [KMAX]
  TARGETS: comma-separated orbit ids. Research only: exact synthetic geometry; no Windows, DirectX, input or
  performance claim.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import kplus as KP  # noqa: E402

MAXLEN, TARGETS, K1, OUT = int(sys.argv[1]), [int(x) for x in sys.argv[2].split(',')], int(sys.argv[3]), sys.argv[4]
KMAX = int(sys.argv[5]) if len(sys.argv) > 5 else 3
T0 = time.time()
NS, NP, sp, rot = KP.NS, KP.NP, KP.sp, KP.rot
orb = KP.m['orbit_id'].astype(np.int64)
ID = np.arange(NS, dtype=np.int64)
_, FIRST = np.unique(sp, return_index=True)
assert len(FIRST) == NP
TGT = np.isin(orb, TARGETS)


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
        P = prim(g)[P]
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


def a4_words(c):
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


def comm_word(x, y):
    return x + y + [-g for g in reversed(x)] + [-g for g in reversed(y)]


def element(word):
    """Support pieces, the pieces kept at their position, and the permutation of a word."""
    P = word_perm(word)
    PR.clear()
    S = np.unique(sp[np.flatnonzero(P != ID)])
    return S, S[sp[P[FIRST[S]]] == S], P


atlas = json.loads(Path('assets/seed_atlas.json').read_text())
nodes, seen = [], set()
for e in atlas:
    for nd in spine(e['ast']):
        k = json.dumps(nd)
        if k not in seen:
            seen.add(k)
            w = expand(nd)
            if len(w) <= MAXLEN:
                nodes.append({'name': f"spine node of O{e['orbit']:02d}, {len(w)} letters", 'word': w})
for d in nodes:
    d['S'], d['F'], _ = element(d['word'])
K = KP.group()
assert len(K) == 7200


def letter_table(words):
    used = sorted({abs(g) for w in words for g in w})
    cnt = np.zeros((len(words), len(used)), np.int64)
    col = {a: c for c, a in enumerate(used)}
    for i, w in enumerate(words):
        for g in w:
            cnt[i, col[abs(g)]] += 1
    return used, cnt


log(nodes=len(nodes), targets=TARGETS, kmax=KMAX)

# level 1: overlaps of two or three pieces with a target piece
M = np.zeros((len(nodes), NP), bool)
for i, d in enumerate(nodes):
    M[i, d['S']] = True
MT = M & TGT[None, :]
USED, CNT = letter_table([d['word'] for d in nodes])
lvl1 = {}  # target orbit -> up to K1 (length, i, j, sigma index)
for si, sig in enumerate(K):
    sinv = np.argsort(sig)
    tl = CNT @ np.array([len(conj_word(a, sig, sinv)) for a in USED])
    pm = KP.piece_map(sig)
    for j, dy in enumerate(nodes):
        img = pm[dy['S']]
        c_all = M[:, img].sum(axis=1)
        c_t = MT[:, img].sum(axis=1)
        for i in np.flatnonzero((c_all >= 2) & (c_all <= KMAX) & (c_t >= 1)):
            L = 2 * (len(nodes[i]['word']) + int(tl[j]))
            for o in set(orb[img[MT[i, img]]].tolist()):
                lst = lvl1.setdefault(o, [])
                if len(lst) < K1 or L < lst[-1][0]:
                    if (L, int(i), j, si) not in lst:
                        lst.append((L, int(i), j, si))
                        lst.sort()
                        del lst[K1:]
    if si % 1200 == 1199:
        log(level=1, sigma=si + 1, orbits=sorted(lvl1))
zs, zseen = [], set()
for o, lst in sorted(lvl1.items()):
    for L, i, j, si in lst:
        if (i, j, si) in zseen:
            continue
        zseen.add((i, j, si))
        w = comm_word(nodes[i]['word'], transport(nodes[j]['word'], K[si]))
        S, F, _ = element(w)
        if len(S) and TGT[S].any():
            zs.append({'name': f"[{nodes[i]['name']}, ({nodes[j]['name']})^sigma{si}]", 'word': w, 'S': S, 'F': F})
log(level=1, z_kept=len(zs), z_sizes=sorted(len(z['S']) for z in zs), z_lengths=sorted(len(z['word']) for z in zs))
if not zs:
    Path(OUT).write_text('{}')
    sys.exit(0)

# level 2: C = [Z, W^tau] with a single overlap in a target orbit
W = nodes + zs
MW = np.zeros((len(W), NP), bool)
for i, d in enumerate(W):
    MW[i, d['S']] = True
USED, CNT = letter_table([d['word'] for d in W])
KINDS = {(False, False): 'three-cycle', (True, False): 'pair twist', (False, True): 'pair twist',
         (True, True): 'single twist'}
cand = {}
for ti, tau in enumerate(K):
    tinv = np.argsort(tau)
    tl = CNT @ np.array([len(conj_word(a, tau, tinv)) for a in USED])
    pm = KP.piece_map(tau)
    pinv = np.empty_like(pm)
    pinv[pm] = np.arange(NP)
    for zi, z in enumerate(zs):
        pre = pinv[z['S']]  # W^tau moves piece p exactly when W moves pinv[p]
        cnt = MW[:, pre].sum(axis=1)
        for wi in np.flatnonzero(cnt == 1):
            k = int(np.flatnonzero(MW[wi, pre])[0])
            q = int(z['S'][k])
            if not TGT[q]:
                continue
            kind = KINDS[(bool(np.isin(q, z['F'])), bool(np.isin(pre[k], W[wi]['F'])))]
            L = 2 * (len(z['word']) + int(tl[wi]))
            lst = cand.setdefault((int(orb[q]), kind), [])
            if len(lst) < 6 or L < lst[-1][0]:
                lst.append((L, zi, int(wi), ti, q))
                lst.sort()
                del lst[6:]
    if ti % 1200 == 1199:
        log(level=2, tau=ti + 1, keys=sorted(f'{o}:{k}' for o, k in cand))
res = {}
for (o, kind), lst in sorted(cand.items()):
    for L, zi, wi, ti, q in lst:
        if f'{o}:{kind}' in res and res[f'{o}:{kind}']['C_len'] <= L:
            break
        z, w, tau = zs[zi], W[wi], K[ti]
        Sm = KP.slot_map(tau)
        Pw = word_perm(w['word'])
        Ws = np.empty_like(Pw)
        Ws[Sm] = Sm[Pw]
        wt = transport(w['word'], tau)
        exact = bool(np.array_equal(word_perm(wt), Ws))
        Z = word_perm(z['word'])
        C = inv(Ws)[inv(Z)[Ws[Z]]]
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
        word = comm_word(z['word'], wt)
        assert len(word) == L
        key = f'{o}:{got}'
        if key not in res or L < res[key]['C_len']:
            res[key] = {'orbit': o, 'kind': got, 'C_len': L, 'order': order, 'X_len': len(z['word']),
                        'Y_len': len(w['word']), 'X': z['name'], 'Y': w['name'], 'sigma_index': ti,
                        'overlap_piece': q, 'C_pieces': [int(p) for p in pcs], 'C_word': word, 'level': 2}
for key, r in sorted(res.items(), key=lambda kv: (kv[1]['orbit'], kv[1]['kind'])):
    log(**{k: v for k, v in r.items() if k != 'C_word'})
pure3 = sorted(r['orbit'] for r in res.values() if r['kind'] == 'three-cycle')
log(pure_three_cycle_orbits=pure3, missing=[o for o in TARGETS if o not in pure3])
Path(OUT).write_text(json.dumps(res))
