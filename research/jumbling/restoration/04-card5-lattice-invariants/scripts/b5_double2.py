"""Doubling through an admissible cap that holds off pieces (orbit 5 target).
For C = (0, x) and a grip g whose cap is unstraddled after C (off pieces of C may lie wholly inside it), and a
retained half-turn a at g: w = U (g, a) U^-1 with U = C C^a. If U acts as two independent copies and a moves every
hole and every off piece of C in cap g, the shape after U is a-symmetric inside cap g, so U^-1 stays admissible and
every off piece q of C inside cap g ends at a(home q) with pose a: on the lattice. Predicted nu: the sum of
b_orbit over the holes of C in cap g and over the off pieces of C wholly inside cap g. Every candidate is replayed
from solved; only replayed results count.
usage: b5_double2.py <research/jumbling> <repo root> <nucoords dir> <out jsonl> [orbit]"""
import json, sys, time
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
OUT = sys.argv[4]
TARGET = int(sys.argv[5]) if len(sys.argv) > 5 else 5
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
    n = apply_word(st2, w)
    if n != len(w):
        return None, n
    if not st2.is_lattice():
        return None, -1
    return {str(k): list(x) for k, x in sorted(NC.nu(np.asarray(st2.lattice_stickers()['labels'], np.int64)).items())}, n
fout = open(OUT, 'a')
t0 = time.time()
stats = {'grips': 0, 'admissible': 0, 'pred_target': 0, 'replays': 0, 'stopped_at': {}, 'off_lattice': 0, 'ok': 0, 'pred_ok': 0, 'hits': 0}
for x in JI:
    st = sim.State(ctx, menu=M)
    assert apply_word(st, [(0, x)]) == 1
    H = [int(h) for h in holes(st).tolist()]; Q = set(off_of(st).tolist())
    capH = {}
    for h in H:
        for g in d.signature_list(h):
            if int(g) != 0:
                capH.setdefault(int(g), []).append(h)
    for g, hs in sorted(capH.items()):
        stats['grips'] += 1
        cl = st.classify(g, need_inside=True, stop_at_straddle=True)
        if cl.straddle is not None or cl.uncertain:
            continue
        stats['admissible'] += 1
        qin = sorted(set(cl.inside.tolist()) & Q)
        pred = {}
        for p in hs + qin:
            o = int(d.orbit_id[p]); pred[o] = pred.get(o, 0) ^ 1
        pv = sorted(o for o, b in pred.items() if b)
        if TARGET not in pv:
            continue
        stats['pred_target'] += 1
        for a in halfturns(g):
            img = images(kidx((g, a)), hs).tolist()
            if any(int(y) == h for y, h in zip(img, hs)):
                continue
            A = RS.element(M, g, a)
            perm = kp.perms[kp.index_of_matrix(A)]
            C = [(0, x)]
            Ca = [(int(perm[c]), item(int(perm[c]), matmul(matmul(A, RS.element(M, c, j)), transpose(A)))) for c, j in C]
            U = C + Ca
            w = U + [(g, a)] + winv(U)
            stats['replays'] += 1
            nu, n = replay_nu(w)
            if nu is None:
                if n == -1:
                    stats['off_lattice'] += 1
                else:
                    stats['stopped_at'][n] = stats['stopped_at'].get(n, 0) + 1
                continue
            stats['ok'] += 1
            nv = sorted(int(o) for o, (s, c) in nu.items() if s & 1)
            if nv == pv and not any(c for s, c in nu.values()):
                stats['pred_ok'] += 1
            hit = str(TARGET) in nu and nu[str(TARGET)][0] & 1
            print('x', x, 'g', g, 'a', a, 'holes', len(hs), 'off inside', len(qin), 'pred', pv, 'nu', nu, 'hit', bool(hit), '%.0fs' % (time.time() - t0), flush=True)
            if hit:
                stats['hits'] += 1
                fout.write(json.dumps({'kind': 'double-offcap', 'nu': nu, 'length': len(w), 'prep': [list(l) for l in C], 'grip': g,
                                       'a_item': a, 'holes_in_cap': hs, 'off_inside': qin, 'pred_odd_orbits': pv,
                                       'word': [list(l) for l in w]}) + '\n'); fout.flush()
    print('x', x, 'stats', stats, '%.0fs' % (time.time() - t0), flush=True)
    if stats['hits'] >= 2:
        break
print('end', stats, '%.0fs' % (time.time() - t0))
