"""One more twist after the height-6 event: replay the I_a W-J fixture for T records, apply each admissible jumble
twist (E, i), and then, for every grip g within 55.7 degrees of E whose cap is not straddled (exact classify), form
y_L * l for every jumble element y of Lambda_g and every distinct left factor l of the off-lattice pieces wholly
inside cap g. Every twist (g, y) with an unstraddled cap is admissible, so each y_L * l is the left position of a
reachable pose. Records the heights reached and the distinct height-6 and height-8 vertices, then closes under
left multiplication by 2I (global K+ conjugation of the history) and counts them.
usage: h6_more.py <research/jumbling> <T> <E> <out json>"""
import json, sys, time, random
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim, wj, trees
import restore as RS
from trees import qmul, qconj, dist, factors, UNITS
from exact import transpose
from sim.kplus import to_tuple
ctx = sim.get_context()
T, E = int(sys.argv[2]), int(sys.argv[3])
fixture, M = wj.load(ctx, 'I_a')
recs = fixture['journal']['records']
st = sim.State(ctx, menu=M)
t0 = time.time()
for rec in recs[:T]:
    assert st.apply(sim.Twist.from_record(ctx, rec)).applied
JI = list(RS.jumble_items(M))
rng = random.Random(5)
refs = []
for k in range(14):
    q = rng.choice(UNITS)
    for _ in range(rng.randint(1, 3)):
        q = qmul(q, qmul(rng.choice(UNITS), (trees.Q5(1), trees.Q5(1), trees.Q5(0), trees.Q5(0))))
    refs.append(qconj(q))
key = lambda x: tuple(dist(qmul(r, x)) for r in refs)
class VSet:
    def __init__(self):
        self.V = {}
    def add(self, x):
        lst = self.V.setdefault(key(x), [])
        for y in lst:
            if dist(qmul(qconj(y), x)) == 0:
                return False
        lst.append(x)
        return True
    def __len__(self):
        return sum(len(v) for v in self.V.values())
    def items(self):
        return [x for v in self.V.values() for x in v]
V6, V8 = VSet(), VSet()
YL = {}
def yl(g, i):
    if (g, i) not in YL:
        YL[(g, i)] = factors(RS.element(M, g, i))[0]
    return YL[(g, i)]
hist = {}
inv_item = {}
for i in JI:
    T_ = to_tuple(transpose(RS.element(M, E, i)))
    inv_item[i] = next(j for j in range(len(M.items)) if RS.element(M, E, j) == T_)
near = [int(g) for g in RS.NEAR[E]]
trials = 0
for i in JI:
    tw = sim.Twist(ctx, E, RS.element(M, E, i), 'menu', {'menu': M.name})
    if not st.apply(tw).applied:
        continue
    trials += 1
    for g in near:
        cl = st.classify(g, need_inside=True, stop_at_straddle=True)
        if cl.straddle is not None or cl.uncertain:
            continue
        pids = sorted(set(st.pose_id[cl.inside].tolist()))
        Ls = []
        for pid in pids:
            if st._poses[pid].kidx >= 0:
                continue
            l, r = factors(st._pose_matrix(pid))
            Ls.append((dist(l), l))
        if not Ls:
            continue
        for y in JI:
            ylq = yl(g, y)
            for d0, l in Ls:
                q = qmul(ylq, l)
                dq = dist(q)
                hist[(d0, dq)] = hist.get((d0, dq), 0) + 1
                if dq == 6:
                    V6.add(q)
                elif dq == 8:
                    V8.add(q)
    back = sim.Twist(ctx, E, RS.element(M, E, inv_item[i]), 'menu', {'menu': M.name})
    assert st.apply(back).applied
    print('item', i, 'V6', len(V6), 'V8', len(V8), '%.0fs' % (time.time() - t0), flush=True)
def close(vs):
    frontier = vs.items()
    while frontier:
        nxt = []
        for x in frontier:
            for u in UNITS:
                q = qmul(u, x)
                if vs.add(q):
                    nxt.append(q)
        frontier = nxt
    return len(vs)
s6, s8 = len(V6), len(V8)
c6 = close(V6)
c8 = close(V8)
out = {'T': T, 'E': E, 'admissible_first_twists': trials, 'height_transitions': {f'{a}->{b}': n for (a, b), n in sorted(hist.items())},
       'height6_seed_vertices': s6, 'height6_after_2I_closure': c6, 'total_height6': 5120,
       'height8_seed_vertices': s8, 'height8_after_2I_closure': c8, 'total_height8': 5 * 4 ** 7, 'seconds': round(time.time() - t0)}
print(json.dumps(out))
json.dump(out, open(sys.argv[4], 'w'), indent=1)
