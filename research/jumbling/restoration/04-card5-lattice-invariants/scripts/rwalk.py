"""Return-biased random walk from solved (menu I_a, exact simulator); tracks the maximal left height of all pieces.

Each step: with probability PRET an out grip (accumulated element since it last returned is not in K+) is returned
by the inverse of its accumulated element; otherwise a random grip gets a random menu element (jumble with
probability PJ). Every letter is checked by the simulator; blocked letters are skipped. The journal of the best
state is written when a new maximal height appears.
usage: rwalk.py <research/jumbling> <seed> <seconds> <out json> [pret] [pj]"""
import json, sys, time, random
from pathlib import Path
import numpy as np

J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
import restore as RS
from exact import matmul
from sim.kplus import to_tuple
import trees as T

t0 = time.time()
ctx = sim.get_context()
kp = ctx.kplus
M = RS.menu('I_a')
SEED = int(sys.argv[2]); SECS = float(sys.argv[3]); OUT = sys.argv[4]
PRET = float(sys.argv[5]) if len(sys.argv) > 5 else 0.45
PJ = float(sys.argv[6]) if len(sys.argv) > 6 else 0.5
rng = random.Random(SEED)
JI = list(RS.jumble_items(M)); RI = [i for i in range(1, 60) if i not in JI]
st = sim.State(ctx, menu=M)
acc = {}
hpose = {}
best = 0; steps = 0; applied = 0; hist = {}


def height_of(i):
    if i not in hpose:
        p = st._poses[i]
        if p.kidx >= 0:
            hpose[i] = 0
        else:
            l, r = T.factors(p.matrix)
            hpose[i] = T.dist(l)
    return hpose[i]


def tw(c, E):
    out = st.apply(sim.Twist(ctx, c, E, 'menu', {'menu': M.name}), journal=True)
    if out.applied:
        prev = acc.get(c)
        acc[c] = to_tuple(E) if prev is None else to_tuple(matmul(E, prev))
        if kp.index_of_matrix(acc[c]) >= 0:
            acc.pop(c)
    return out.applied


while time.time() - t0 < SECS:
    steps += 1
    ok = False
    if acc and rng.random() < PRET:
        c = rng.choice(list(acc))
        A = acc[c]
        ok = tw(c, tuple(tuple(A[j][i] for j in range(4)) for i in range(4)))
    if not ok:
        c = rng.randrange(600)
        i = rng.choice(JI) if rng.random() < PJ else rng.choice(RI)
        ok = tw(c, RS.element(M, c, i))
    if not ok:
        continue
    applied += 1
    ids = np.unique(np.asarray(st.pose_id)).tolist()
    h = max(height_of(i) for i in ids)
    hist[h] = hist.get(h, 0) + 1
    if h > best:
        best = h
        print(json.dumps({'seed': SEED, 'best': h, 'applied': applied, 'steps': steps, 'off': st.off_lattice_count(),
                          'outs': len(acc), 'sec': round(time.time() - t0, 1)}), flush=True)
        Path(OUT).write_text(json.dumps({'seed': SEED, 'best': h, 'journal': st.journal}) + '\n')
    if applied % 200 == 0:
        print(json.dumps({'seed': SEED, 'applied': applied, 'steps': steps, 'max_now': h, 'off': st.off_lattice_count(),
                          'outs': len(acc), 'poses': len(st._poses), 'sec': round(time.time() - t0, 1)}), flush=True)
print(json.dumps({'done': True, 'seed': SEED, 'best': best, 'applied': applied, 'steps': steps, 'hist': hist,
                  'sec': round(time.time() - t0, 1)}), flush=True)
