"""Frame generators from single-hole words. For a single v (one off piece q, one hole h in orbit o whose slot
stabiliser is nontrivial), search breadth-first for a retained word R that carries the vacancy at h around a
loop back to h with a nontrivial stabiliser rotation, using only grips whose caps neither hold nor straddle q in
the state after v. The vacancy formula predicts nu(v R v^-1) = -nu(R|_{h}) = a pure frame value of orbit o.
Every candidate is replayed from solved in the exact simulator and nu is read from the lattice stickers.
usage: frame_track.py <research/jumbling> <repo root> <i23 scripts> <singles jsonl> <out jsonl> <max depth> [orbit ...]"""
import json, sys, time
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
from exact import transpose
from sim.kplus import to_tuple
from nucoords import NuCoords
ctx = RS.ctx
d, reg, kp = ctx.data, ctx.regions, ctx.kplus
M = RS.menu('I_a')
NC = NuCoords(sys.argv[2])
maxdep = int(sys.argv[6])
want = set(int(x) for x in sys.argv[7:]) or {8, 11, 17, 22, 25, 28, 32}
NI = len(M.items)
ident0 = RS.element(M, 0, 0)
stab0 = [kp.matrix(s) for s in kp.stab0]
RI = [i for i in range(NI) if RS.element(M, 0, i) in stab0 and RS.element(M, 0, i) != ident0]
item_of = {}
def inv_letter(l):
    c, j = l
    if c not in item_of:
        item_of[c] = {RS.element(M, c, x): x for x in range(NI)}
    return (c, item_of[c][to_tuple(transpose(RS.element(M, c, j)))])
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
rows = [json.loads(x) for x in open(sys.argv[4]) if x.strip()]
rows.sort(key=lambda r: r['length'])
fout = open(sys.argv[5], 'a')
done = set()
t0 = time.time()
for r in rows:
    if len(r['holes']) != 1:
        continue
    o, h = r['hole_orbit'][0], r['holes'][0]
    if o not in want or o in done:
        continue
    v = [tuple(x) for x in r['word']]
    st = sim.State(ctx, menu=M)
    for l in v:
        assert st.apply(tw(l)).applied
    q = set(np.flatnonzero(st._kpose()[st.pose_id] < 0).tolist())
    okc = {}
    def grip_ok(c):
        if c not in okc:
            cl = st.classify(c, need_inside=True, stop_at_straddle=True)
            okc[c] = cl.straddle is None and not cl.uncertain and not (set(cl.inside.tolist()) & q)
        return okc[c]
    ro = int(reg.orbit_of[h])
    g0 = int(reg.transport[h])
    prev = {g0: None}
    dq = deque([(g0, 0)])
    found = None
    while dq and found is None:
        g, dep = dq.popleft()
        if dep >= maxdep:
            continue
        s = int(reg.piece_of[ro, g])
        for c in d.signature_list(s):
            if not grip_ok(int(c)):
                continue
            for i in RI:
                g2 = comp(kidx((int(c), i)), g)
                if g2 in prev:
                    continue
                prev[g2] = (g, (int(c), i))
                if int(reg.piece_of[ro, g2]) == h:
                    found = g2
                    break
                dq.append((g2, dep + 1))
            if found is not None:
                break
    if found is None:
        print('orbit', o, 'hole', h, 'no loop within depth', maxdep, 'states', len(prev), '%.0fs' % (time.time() - t0), flush=True)
        continue
    R = []
    g = found
    while prev[g] is not None:
        g, l = prev[g]
        R.append(l)
    R.reverse()
    w = v + R + [inv_letter(l) for l in reversed(v)]
    st2 = sim.State(ctx, menu=M)
    n = 0
    for l in w:
        if not st2.apply(tw(l)).applied:
            break
        n += 1
    ok = n == len(w)
    lat = ok and bool(st2.is_lattice())
    nu = None
    if lat:
        nu = {str(k): list(x) for k, x in sorted(NC.nu(np.asarray(st2.lattice_stickers()['labels'], np.int64)).items())}
    print('orbit', o, 'hole', h, 'R', R, 'len', len(w), 'admissible', ok, 'lattice', lat, 'nu', nu, '%.0fs' % (time.time() - t0), flush=True)
    if lat and nu:
        fout.write(json.dumps({'orbit': o, 'nu': nu, 'length': len(w), 'R': [list(l) for l in R], 'hole': h,
                               'word': [list(l) for l in w]}) + '\n')
        fout.flush()
        if set(nu) == {str(o)}:
            done.add(o)
print('done', sorted(done), '%.0fs' % (time.time() - t0))
