"""Targeted doubling for the signs of orbits 4 and 5.
C = (0, x) (e, r): a jumble at grip 0, then a retained twist at a clean grip e whose cap holds vacated slots of
orbit 4 or 5 (this moves those holes inside cap e). For every grip g whose cap holds an odd number of holes of
orbit 4 or of orbit 5, whose cap is clean, and every retained half-turn a at g moving every hole of C in cap g:
w = U (g, a) U^-1 with U = C C^a. Corollary D predicts nu(w) = sum of b_orbit(h) over the holes of C in cap g
when U is admissible with H and a(H) disjoint; every candidate is replayed from solved and nu is read from the
lattice stickers, so only replayed words count.
usage: b45_double.py <research/jumbling> <repo root> <nucoords dir> <seed> <budget s> <out jsonl> [max per orbit]
       [mode: clean|admissible] [orbit ...]   (mode admissible: e only needs to be unstraddled)"""
import json, sys, time, random
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
sys.path.insert(0, sys.argv[3])
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import restore as RS
import sim
from exact import matmul, transpose
from sim.kplus import to_tuple
from nucoords import NuCoords
ctx = RS.ctx
d, reg, kp = ctx.data, ctx.regions, ctx.kplus
M = RS.menu('I_a')
NC = NuCoords(sys.argv[2])
rng = random.Random(int(sys.argv[4]))
budget, OUT = float(sys.argv[5]), sys.argv[6]
PER = int(sys.argv[7]) if len(sys.argv) > 7 else 2
MODE = sys.argv[8] if len(sys.argv) > 8 else 'clean'
WANT = [int(o) for o in sys.argv[9:]] or [4, 5]
NI = len(M.items)
JI = list(RS.jumble_items(M)); ident = RS.element(M, 0, 0)
RIall = [i for i in range(NI) if i not in JI and RS.element(M, 0, i) != ident]
item_of = {}
def item(c, A):
    if c not in item_of:
        item_of[c] = {RS.element(M, c, j): j for j in range(NI)}
    return item_of[c][to_tuple(A)]
def inv(l):
    c, j = l
    return (c, item(c, transpose(RS.element(M, c, j))))
winv = lambda w: [inv(l) for l in reversed(w)]
TW, KI = {}, {}
def tw(l):
    if l not in TW:
        TW[l] = sim.Twist(ctx, l[0], RS.element(M, l[0], l[1]), 'menu', {'menu': M.name})
    return TW[l]
def kidx(l):
    if l not in KI:
        KI[l] = int(kp.index_of_matrix(RS.element(M, l[0], l[1])))
    return KI[l]
def images(k, slots):
    s = np.asarray(slots, np.int64)
    return reg.piece_of[reg.orbit_of[s], kp.compose_vec(np.full(len(s), k), reg.transport[s])]
HT = {}
def halfturns(c):
    if c not in HT:
        HT[c] = [i for i in RIall if to_tuple(matmul(RS.element(M, c, i), RS.element(M, c, i))) == RS.element(M, c, 0)]
    return HT[c]
def apply_word(st, W):
    n = 0
    for l in W:
        if not st.apply(tw(l)).applied:
            return n
        n += 1
    return n
def holes(st):
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    comp_ = kp.compose_vec(K[lat], reg.transport[lat])
    dest = reg.piece_of[reg.orbit_of[lat], comp_]
    occ = np.zeros(len(K), bool); occ[dest] = True
    return np.flatnonzero(~occ)
off_of = lambda st: np.flatnonzero(st._kpose()[st.pose_id] < 0)
def replay_nu(w):
    st2 = sim.State(ctx, menu=M)
    if apply_word(st2, w) != len(w) or not st2.is_lattice():
        return None
    return {str(k): list(x) for k, x in sorted(NC.nu(np.asarray(st2.lattice_stickers()['labels'], np.int64)).items())}
def clean_fn(st, Q, mode='clean'):
    okc = {}
    def clean(g):
        if g not in okc:
            cl = st.classify(g, need_inside=True, stop_at_straddle=True)
            adm = cl.straddle is None and not cl.uncertain
            okc[g] = adm and (mode == 'admissible' or not (set(cl.inside.tolist()) & Q))
        return okc[g]
    return clean
def cap_holes(H):
    capH = {}
    for h in H:
        for g in d.signature_list(h):
            capH.setdefault(int(g), []).append(h)
    return capH
fout = open(OUT, 'a')
t0 = time.time()
got = {o: 0 for o in WANT}
stats = {'preps': 0, 'cands': 0, 'clean': 0, 'replays': 0, 'not_admissible_or_off_lattice': 0, 'pred_ok': 0, 'pred_mismatch': 0}
xs = JI[:]; rng.shuffle(xs)
for x in xs:
    if time.time() - t0 > budget or min(got.values()) >= PER:
        break
    st0 = sim.State(ctx, menu=M)
    assert apply_word(st0, [(0, x)]) == 1
    H0 = [int(h) for h in holes(st0).tolist()]; Q0 = set(off_of(st0).tolist())
    clean0 = clean_fn(st0, Q0, MODE)
    E = sorted(g for g, hs in cap_holes(H0).items() if g != 0 and any(int(d.orbit_id[h]) in WANT for h in hs) and clean0(g))
    for e in E:
        for r in RIall:
            if time.time() - t0 > budget or min(got.values()) >= PER:
                break
            C = [(0, x), (e, r)]
            st = sim.State(ctx, menu=M)
            if apply_word(st, C) != 2:
                continue
            stats['preps'] += 1
            H = [int(h) for h in holes(st).tolist()]; Q = set(off_of(st).tolist())
            clean = clean_fn(st, Q)
            capH = cap_holes(H)
            cands = []
            for g, hs in capH.items():
                odd = [o for o in WANT if sum(1 for h in hs if int(d.orbit_id[h]) == o) % 2 and got[o] < PER]
                if odd:
                    cands.append((len(hs), g, hs, odd))
            cands.sort()
            stats['cands'] += len(cands)
            tries = 0
            for _, g, hs, odd in cands:
                if tries >= 4:
                    break
                if not clean(g):
                    continue
                stats['clean'] += 1
                for a in halfturns(g):
                    img = images(kidx((g, a)), hs).tolist()
                    if any(int(y) == h for y, h in zip(img, hs)):
                        continue
                    A = RS.element(M, g, a)
                    perm = kp.perms[kp.index_of_matrix(A)]
                    Ca = [(int(perm[c]), item(int(perm[c]), matmul(matmul(A, RS.element(M, c, j)), transpose(A)))) for c, j in C]
                    U = C + Ca
                    w = U + [(g, a)] + winv(U)
                    tries += 1; stats['replays'] += 1
                    nu = replay_nu(w)
                    if nu is None:
                        stats['not_admissible_or_off_lattice'] += 1
                        break
                    pred = {}
                    for h in hs:
                        o = int(d.orbit_id[h]); pred[o] = pred.get(o, 0) ^ 1
                    pv = sorted(o for o, b in pred.items() if b)
                    nv = sorted(int(o) for o, (s, c) in nu.items() if s & 1)
                    stats['pred_ok' if pv == nv and not any(c for s, c in nu.values()) else 'pred_mismatch'] += 1
                    hit = [o for o in WANT if str(o) in nu and nu[str(o)][0] & 1]
                    print('cand x', x, 'e', e, 'r', r, 'g', g, 'a', a, 'len', len(w), 'pred', pv, 'nu', nu, 'hit', hit, '%.0fs' % (time.time() - t0), flush=True)
                    if hit:
                        for o in hit:
                            got[o] += 1
                        fout.write(json.dumps({'kind': 'double45', 'nu': nu, 'length': len(w), 'prep': [list(l) for l in C], 'grip': g,
                                               'a_item': a, 'holes_in_cap': hs, 'hole_orbits': [int(d.orbit_id[h]) for h in hs],
                                               'pred_odd_orbits': pv, 'word': [list(l) for l in w]}) + '\n'); fout.flush()
                    break
    print('x', x, 'done', stats, got, '%.0fs' % (time.time() - t0), flush=True)
print('end', stats, got, '%.0fs' % (time.time() - t0))
