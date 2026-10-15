"""Orbit-targeted single-hole preparations by greedy refill descent.
For a preparation C with hole set H and a retained twist (g,a) whose cap holds no off piece, the word
C (g,a) C^-1 leaves exactly the pieces that (g,a) moves into holes off the lattice: the refill set
R(g,a) = {h in H_g : a^-1(h) not in H}, one piece per refilled hole, of that hole's orbit. R is computed
exactly from slot images, without replay. If some |R(g,a)| = 1 with the refilled hole in a wanted orbit, the
conjugate is a single of that orbit; otherwise descend to C (g,a) C^-1 with the smallest refill set that still
contains a wanted orbit. Every single is confirmed by the exact simulator before it is written.
usage: singles4.py <research/jumbling> <class P> <seed> <budget s> <classes.json> <out jsonl> <cap> [orbit ...]"""
import json, sys, time, random
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
import restore as RS
from exact import transpose
from sim.kplus import to_tuple
ctx = sim.get_context()
d, reg, kp = ctx.data, ctx.regions, ctx.kplus
M = RS.menu('I_a')
P, seed, budget = int(sys.argv[2]), int(sys.argv[3]), float(sys.argv[4])
cls = {int(k): v for k, v in json.load(open(sys.argv[5])).items()}
OUT, CAP = sys.argv[6], int(sys.argv[7])
want = set(int(x) for x in sys.argv[8:])
rng = random.Random(seed)
JI = list(RS.jumble_items(M)); ident = RS.element(M, 0, 0)
RI = [i for i in range(len(M.items)) if i not in JI and RS.element(M, 0, i) != ident]
NF = np.array([[float(v) for v in row] for row in d.N]); NN = NF / np.linalg.norm(NF, axis=1)[:, None]
cosang = NN @ NN.T
inv_cache, TW, KI = {}, {}, {}
def inv(l):
    if l not in inv_cache:
        c, i = l
        T = to_tuple(transpose(RS.element(M, c, i)))
        inv_cache[l] = (c, next(j for j in range(len(M.items)) if RS.element(M, c, j) == T))
    return inv_cache[l]
winv = lambda w: [inv(l) for l in reversed(w)]
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
def apply_word(st, W):
    n = 0
    for l in W:
        if not st.apply(tw(l)).applied:
            return n
        n += 1
    return n
def undo(st, W, n):
    for l in reversed(W[:n]):
        assert st.apply(tw(inv(l))).applied
def holes(st):
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    comp = kp.compose_vec(K[lat], reg.transport[lat])
    dest = reg.piece_of[reg.orbit_of[lat], comp]
    occ = np.zeros(len(K), bool); occ[dest] = True
    return np.flatnonzero(~occ)
off_of = lambda st: np.flatnonzero(st._kpose()[st.pose_id] < 0)
classP = [c for c in range(600) if cls[c] == P]
fout = open(OUT, 'a')
got = {}
t0 = time.time()
okc = {}
def admissible_clean(st, g, off):
    cl = st.classify(g, need_inside=True, stop_at_straddle=True)
    return cl.straddle is None and not cl.uncertain and not (set(cl.inside.tolist()) & off)
def open_orbits():
    return {o for o in want if got.get(o, 0) < CAP}
while time.time() - t0 < budget and open_orbits():
    b = rng.choice(classP)
    nb = [a for a in classP if a != b and cosang[b, a] > 0.80]
    a1, a2 = rng.sample(nb, 2)
    k1, x, k2 = (a1, rng.choice(RI if rng.random() < 0.5 else JI)), (b, rng.choice(JI)), (a2, rng.choice(RI))
    C0 = [k1, x, inv(k1), inv(x)]
    C = C0 if rng.random() < 0.5 else [k2] + C0 + [inv(k2)] + winv(C0)
    st = sim.State(ctx, menu=M)
    if apply_word(st, C) != len(C) or len(off_of(st)) == 0:
        continue
    for level in range(7):
        wo = open_orbits()
        if not wo:
            break
        H = holes(st); Hs = set(H.tolist()); off = set(off_of(st).tolist())
        horb = {int(h): int(d.orbit_id[h]) for h in H.tolist()}
        if not (set(horb.values()) & wo):
            break
        cnt = {}
        for h in H.tolist():
            for g in d.signature_list(int(h)):
                cnt.setdefault(int(g), []).append(int(h))
        singles, desc = [], []
        for g, hs in cnt.items():
            if not (set(horb[h] for h in hs) & wo):
                continue
            for a in RI:
                pre = images(kidx(inv((g, a))), hs)
                R = [h for h, q in zip(hs, pre.tolist()) if q not in Hs]
                if not R:
                    continue
                ro = set(horb[h] for h in R)
                if not (ro & wo):
                    continue
                if len(R) == 1:
                    singles.append((g, a, R[0]))
                else:
                    desc.append((len(R), g, a))
        rng.shuffle(singles)
        tried = set()
        for g, a, h in singles:
            o = horb[h]
            if got.get(o, 0) >= CAP or (g, o) in tried:
                continue
            tried.add((g, o))
            if g not in okc:
                okc[g] = admissible_clean(st, g, off)
            if not okc[g]:
                continue
            W = [(g, a)] + winv(C)
            n = apply_word(st, W)
            if n == len(W):
                o2 = off_of(st)
                if len(o2) == 1:
                    H2 = holes(st)
                    row = {'class': P, 'word': [list(z) for z in C + W], 'length': len(C) + len(W), 'off': o2.tolist(),
                           'off_orbit': [int(d.orbit_id[q]) for q in o2], 'holes': H2.tolist(),
                           'hole_orbit': [int(d.orbit_id[q]) for q in H2], 'conj_grip': g, 'seed_hole': h,
                           'level': level, 't': round(time.time() - t0)}
                    fout.write(json.dumps(row) + '\n'); fout.flush()
                    got[o] = got.get(o, 0) + 1
                    print('SINGLE orbit', o, 'hole orbit', row['hole_orbit'], 'len', row['length'], 'level', level,
                          'got', dict(sorted(got.items())), '%.0fs' % (time.time() - t0), flush=True)
            undo(st, W, n)
        okc.clear()
        desc.sort()
        moved = False
        for r, g, a in desc[:12]:
            if not admissible_clean(st, g, off):
                continue
            W = [(g, a)] + winv(C)
            n = apply_word(st, W)
            if n == len(W) and len(off_of(st)) == r:
                C = C + W
                moved = True
                print('class', P, 'level', level + 1, 'residue', r, 'len', len(C), '%.0fs' % (time.time() - t0), flush=True)
                break
            undo(st, W, n)
        if not moved:
            break
print('done', dict(sorted(got.items())), '%.0fs' % (time.time() - t0))
