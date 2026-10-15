"""Height-6 left positions: replay the I_a W-J fixture for T records; for the pieces of left height 4 lying wholly
in the admissible cap of grip E (exact classify), form y_L * l for every jumble element y of Lambda_E (left factor
y_L) and every distinct height-4 left factor l. Keep those at distance 6, close under left multiplication by 2I
(global K+ conjugation of the history), and count distinct vertices (key: distances to fixed reference vertices,
collisions resolved by the exact test dist(a^-1 b) = 0). Tree has 5*4^5 = 5120 vertices at distance 6."""
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
ctx = sim.get_context()
T, E = int(sys.argv[2]), int(sys.argv[3])
fixture, M = wj.load(ctx, 'I_a')
recs = fixture['journal']['records']
st = sim.State(ctx, menu=M)
t0 = time.time()
for rec in recs[:T]:
    assert st.apply(sim.Twist.from_record(ctx, rec)).applied
cl = st.classify(E, need_inside=True, stop_at_straddle=True)
assert cl.straddle is None and not cl.uncertain
ins = cl.inside
pids = sorted(set(st.pose_id[ins].tolist()))
L4 = []
for pid in pids:
    P = st._poses[pid]
    if P.kidx >= 0:
        continue
    l, r = factors(st._pose_matrix(pid))
    if dist(l) == 4:
        L4.append(l)
print('replayed', T, 'inside cap', E, len(ins), 'pieces; distinct height-4 left factors', len(L4), '%.0fs' % (time.time() - t0), flush=True)
rng = random.Random(5)
refs = []
for k in range(14):
    q = rng.choice(UNITS)
    for _ in range(rng.randint(1, 3)):
        q = qmul(q, qmul(rng.choice(UNITS), (trees.Q5(1), trees.Q5(1), trees.Q5(0), trees.Q5(0))))
    refs.append(qconj(q))
key = lambda x: tuple(dist(qmul(r, x)) for r in refs)
V = {}
def add(x):
    k = key(x)
    lst = V.setdefault(k, [])
    for y in lst:
        if dist(qmul(qconj(y), x)) == 0:
            return False
    lst.append(x)
    return True
seeds = []
for i in RS.jumble_items(M):
    yl = factors(RS.element(M, E, i))[0]
    for l in L4:
        q = qmul(yl, l)
        dq = dist(q)
        if dq == 6 and add(q):
            seeds.append(q)
        elif dq != 6:
            pass
n_seed = sum(len(v) for v in V.values())
print('distinct height-6 vertices from the cap', n_seed, '%.0fs' % (time.time() - t0), flush=True)
frontier = list(seeds)
while frontier:
    nxt = []
    for x in frontier:
        for u in UNITS:
            q = qmul(u, x)
            if add(q):
                nxt.append(q)
    frontier = nxt
    print('closure so far', sum(len(v) for v in V.values()), '%.0fs' % (time.time() - t0), flush=True)
tot = sum(len(v) for v in V.values())
print('after 2I closure', tot, 'of 5120')
json.dump({'T': T, 'grip': E, 'height4_left_factors': len(L4), 'seed_vertices': n_seed, 'closure': tot, 'total_height6': 5120}, open(sys.argv[4], 'w'))
