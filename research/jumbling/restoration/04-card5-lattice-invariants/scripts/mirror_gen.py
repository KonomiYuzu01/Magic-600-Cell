"""Harvest new nu generators from random class-P preparations by two vacancy-formula constructions.
State: a preparation C (admissible from solved) with off set Q and hole set H, from random commutator cores and
greedy refill descent (as singles4.py).
(a) Doubling. For a grip d whose cap holds no piece of Q and is not straddled, and a retained half-turn a at d
    that moves every hole of C in cap d: w = U (d,a) U^-1 with U = C C^a. If C^a acts after C as from solved,
    U has holes H and a(H), (d,a) swaps each hole h of C in cap d with a(h), and the vacancy formula predicts
    nu(w) = sum of b_orbit(h) over the holes of C in cap d (frames cancel because a^2 = 1).
(b) Frame loop. For a hole h of a frame orbit, a breadth-first search over the K+ poses of the vacancy at h for
    a retained word R that returns it to h with a nontrivial stabiliser rotation, using only grips whose caps
    hold no piece of Q, no other hole of C and no straddle: nu(C R C^-1) is a pure frame of orbit(h).
Only candidates whose predicted vector leaves the current span (or gives a C5 frame) are replayed. Every kept
word is replayed from solved in the exact simulator and nu is read from the lattice stickers.
usage: mirror_gen.py <research/jumbling> <repo root> <i23 scripts> <classes.json> <class P> <seed> <budget s>
       <out jsonl> <library json> <frame orbits comma list> <gens jsonl ...> [-- name=coord,coord ...]"""
import json, sys, time, random
from collections import deque
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
cls = {int(k): v for k, v in json.load(open(sys.argv[4])).items()}
P, seed, budget, OUT = int(sys.argv[5]), int(sys.argv[6]), float(sys.argv[7]), sys.argv[8]
LIB = sys.argv[9]
FRAME_ORB = set(int(x) for x in sys.argv[10].split(',') if x)
rest = sys.argv[11:]
extras = []
if '--' in rest:
    i = rest.index('--'); extras = rest[i + 1:]; rest = rest[:i]
GENS = rest
rng = random.Random(seed)
BFS_CAP = 3000
NI = len(M.items)
JI = list(RS.jumble_items(M)); ident = RS.element(M, 0, 0)
RIall = [i for i in range(NI) if i not in JI and RS.element(M, 0, i) != ident]
C2F = [0, 3, 6, 8, 11, 22, 25, 32]
names = [f'b{o}' for o in range(35)] + [f'f{o}' for o in C2F]
def vec(nu):
    v = 0
    for o, (s, c) in nu.items():
        o = int(o)
        if s & 1: v |= 1 << o
        if o in C2F and c & 1: v |= 1 << (35 + C2F.index(o))
    return v
basis = {}
def reduce(v):
    while v:
        p = v.bit_length() - 1
        if p in basis: v ^= basis[p]
        else: return v
    return 0
def insert(v):
    v = reduce(v)
    if v:
        basis[v.bit_length() - 1] = v
        return True
    return False
for m in json.load(open(LIB))['macros']:
    insert(vec(m['nu']))
c5 = set()
for path in GENS:
    for line in open(path):
        if line.strip():
            r = json.loads(line); insert(vec(r['nu']))
            for o in ('17', '28'):
                if o in r['nu'] and r['nu'][o][1] % 5: c5.add(int(o))
for e in extras:
    nm, coords = e.split('=')
    v = 0
    for t in coords.split(','): v |= 1 << names.index(t)
    insert(v)
print('start rank', len(basis), 'c5', sorted(c5), flush=True)
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
def comp(a, b):
    return int(kp.compose_vec(np.array([a]), np.array([b]))[0])
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
def undo(st, W, n):
    for l in reversed(W[:n]):
        assert st.apply(tw(inv(l))).applied
def holes(st):
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    comp_ = kp.compose_vec(K[lat], reg.transport[lat])
    dest = reg.piece_of[reg.orbit_of[lat], comp_]
    occ = np.zeros(len(K), bool); occ[dest] = True
    return np.flatnonzero(~occ)
off_of = lambda st: np.flatnonzero(st._kpose()[st.pose_id] < 0)
NF = np.array([[float(v) for v in row] for row in d.N]); NN = NF / np.linalg.norm(NF, axis=1)[:, None]
cosang = NN @ NN.T
classP = [c for c in range(600) if cls[c] == P]
def replay_nu(w):
    st2 = sim.State(ctx, menu=M)
    if apply_word(st2, w) != len(w) or not st2.is_lattice():
        return None
    return {str(k): list(x) for k, x in sorted(NC.nu(np.asarray(st2.lattice_stickers()['labels'], np.int64)).items())}
fout = open(OUT, 'a')
t0 = time.time()
stats = {'states': 0, 'dbl_pred_new': 0, 'dbl_ok': 0, 'dbl_mismatch': 0, 'frame_try': 0, 'frame_ok': 0}
def keep(kind, C, w, nu, extra):
    v = vec(nu)
    new = insert(v)
    newc5 = [int(o) for o in ('17', '28') if o in nu and nu[o][1] % 5 and int(o) not in c5]
    for o in newc5: c5.add(o)
    if new or newc5:
        row = {'kind': kind, 'nu': nu, 'length': len(w), 'prep_length': len(C), 'word': [list(l) for l in w]}
        row.update(extra)
        fout.write(json.dumps(row) + '\n'); fout.flush()
        print('NEW', kind, nu, 'len', len(w), 'rank', len(basis), 'c5', sorted(c5), '%.0fs' % (time.time() - t0), flush=True)
    return new or newc5
def try_state(st, C):
    stats['states'] += 1
    H = holes(st); Q = set(off_of(st).tolist())
    Hl = [int(h) for h in H.tolist()]; Hs = set(Hl)
    capH = {}
    for h in Hl:
        for g in d.signature_list(h):
            capH.setdefault(int(g), []).append(h)
    okc = {}
    def clean(g):
        if g not in okc:
            cl = st.classify(g, need_inside=True, stop_at_straddle=True)
            okc[g] = cl.straddle is None and not cl.uncertain and not (set(cl.inside.tolist()) & Q)
        return okc[g]
    # (a) doubling
    cands = []
    for g, hs in capH.items():
        pred = 0
        for h in hs:
            pred ^= 1 << int(d.orbit_id[h])
        if not reduce(pred):
            continue
        cands.append((len(hs), g, hs, pred))
    rng.shuffle(cands); cands.sort(key=lambda x: x[0])
    tried = 0
    for _, g, hs, pred in cands:
        if tried >= 3 or not reduce(pred):
            continue
        if not clean(g):
            continue
        for a in halfturns(g):
            img = images(kidx((g, a)), hs).tolist()
            if any(int(x) == h for x, h in zip(img, hs)):
                continue
            A = RS.element(M, g, a)
            perm = kp.perms[kp.index_of_matrix(A)]
            Ca = [(int(perm[c]), item(int(perm[c]), matmul(matmul(A, RS.element(M, c, j)), transpose(A)))) for c, j in C]
            U = C + Ca
            w = U + [(g, a)] + winv(U)
            stats['dbl_pred_new'] += 1; tried += 1
            nu = replay_nu(w)
            if nu is None:
                break
            if vec(nu) == pred:
                stats['dbl_ok'] += 1
            else:
                stats['dbl_mismatch'] += 1
            keep('double', C, w, nu, {'grip': g, 'a_item': a, 'holes_in_cap': hs,
                                      'hole_orbits': [int(d.orbit_id[h]) for h in hs], 'pred': [names[i] for i in range(43) if pred >> i & 1]})
            break
    # (b) frame loops
    for h in Hl:
        o = int(d.orbit_id[h])
        if o not in FRAME_ORB:
            continue
        if o in C2F and not reduce(1 << (35 + C2F.index(o))):
            continue
        if o in (17, 28) and o in c5:
            continue
        if o not in C2F and o not in (17, 28):
            continue
        stats.setdefault('bfs', 0); stats['bfs'] += 1
        others = Hs - {h}
        gok = {}
        def grip_ok(c):
            if c not in gok:
                gok[c] = clean(c) and not any(x in others for x in capH.get(c, []))
            return gok[c]
        ro = int(reg.orbit_of[h]); g0 = int(reg.transport[h])
        prev = {g0: None}; dq = deque([(g0, 0)]); found = None
        while dq and found is None and len(prev) < BFS_CAP:
            gg, dep = dq.popleft()
            if dep >= 6:
                continue
            s = int(reg.piece_of[ro, gg])
            for c in d.signature_list(s):
                c = int(c)
                if not grip_ok(c):
                    continue
                for i in RIall:
                    g2 = comp(kidx((c, i)), gg)
                    if g2 in prev:
                        continue
                    prev[g2] = (gg, (c, i))
                    if int(reg.piece_of[ro, g2]) == h:
                        found = g2; break
                    dq.append((g2, dep + 1))
                if found is not None:
                    break
        if found is None:
            continue
        R = []; gg = found
        while prev[gg] is not None:
            gg, l = prev[gg]; R.append(l)
        R.reverse()
        w = C + R + winv(C)
        stats['frame_try'] += 1
        nu = replay_nu(w)
        if nu is None:
            continue
        stats['frame_ok'] += 1
        keep('frame', C, w, nu, {'hole': h, 'orbit': o, 'R': [list(l) for l in R]})
while time.time() - t0 < budget:
    b = rng.choice(classP)
    nb = [a for a in classP if a != b and cosang[b, a] > 0.80]
    a1, a2 = rng.sample(nb, 2)
    k1, x, k2 = (a1, rng.choice(RIall if rng.random() < 0.5 else JI)), (b, rng.choice(JI)), (a2, rng.choice(RIall))
    C0 = [k1, x, inv(k1), inv(x)]
    C = C0 if rng.random() < 0.5 else [k2] + C0 + [inv(k2)] + winv(C0)
    st = sim.State(ctx, menu=M)
    if apply_word(st, C) != len(C) or len(off_of(st)) == 0:
        continue
    for level in range(5):
        try_state(st, C)
        if time.time() - t0 > budget:
            break
        H = holes(st); Hs = set(H.tolist()); off = set(off_of(st).tolist())
        cnt = {}
        for h in H.tolist():
            for g in d.signature_list(int(h)):
                cnt.setdefault(int(g), []).append(int(h))
        desc = []
        for g, hs in cnt.items():
            for a in RIall:
                pre = images(kidx(inv((g, a))), hs)
                Rr = [h for h, q in zip(hs, pre.tolist()) if q not in Hs]
                if len(Rr) >= 2:
                    desc.append((len(Rr), rng.random(), g, a))
        desc.sort()
        moved = False
        for r, _, g, a in desc[:12]:
            cl = st.classify(g, need_inside=True, stop_at_straddle=True)
            if cl.straddle is not None or cl.uncertain or (set(cl.inside.tolist()) & off):
                continue
            W = [(g, a)] + winv(C)
            n = apply_word(st, W)
            if n == len(W) and len(off_of(st)) == r:
                C = C + W; moved = True
                break
            undo(st, W, n)
        if not moved:
            break
    if stats['states'] % 20 == 0:
        print('stats', stats, 'rank', len(basis), '%.0fs' % (time.time() - t0), flush=True)
print('done rank', len(basis), 'c5', sorted(c5), stats, '%.0fs' % (time.time() - t0))
