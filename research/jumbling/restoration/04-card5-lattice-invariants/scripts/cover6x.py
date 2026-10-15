"""Height-6 and height-8 left positions along a reachable history (menu I_a). Replays the I_a W-J fixture journal;
after every record T in [T0, T1) (stride k), finds the off pieces of left height >= 4, the grips whose closed cap
might contain one of them wholly (float prefilter, margin 1e-6), and classifies those grips exactly
(State.classify). For every unstraddled such grip g, every jumble element y of Lambda_g and every distinct left
factor l (height >= 4) of a piece wholly inside cap g, the twist (g, y) is admissible after record T, so y_L * l is
the left position of a reachable pose. Collects the distinct vertices at distance 6 and 8, closes them under left
multiplication by 2I (Lemma E) and counts them. usage: cover6x.py <research/jumbling> <T0> <T1> <stride> <out json>"""
import json, sys, time, random
from collections import Counter
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim, wj, trees
import restore as RS
from trees import qmul, qconj, dist, factors, UNITS
ctx = sim.get_context(); d = ctx.data; reg = ctx.regions; kp = ctx.kplus
T0, T1, STRIDE = int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
fixture, M = wj.load(ctx, 'I_a')
recs = fixture['journal']['records']
T1 = min(T1, len(recs))
st = sim.State(ctx, menu=M)
t0 = time.time()
NF = np.array([[float(v) for v in row] for row in d.N]); KAP = float(d.KAPPA)
fm = lambda m: np.array([[float(v) for v in row] for row in m])
Kf = np.array([fm(kp.matrix(i)) for i in range(7200)])
repV = [np.array([[float(v) for v in vert] for vert in vs]) for vs in reg.rep_vertices]
orb = reg.orbit_of.astype(np.int64); tr = reg.transport.astype(np.int64)
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


V6, V8, V10 = VSet(), VSet(), VSet()
YL = {}


def yl(g, i):
    if (g, i) not in YL:
        YL[(g, i)] = factors(RS.element(M, g, i))[0]
    return YL[(g, i)]


LF = {}


def leftf(pid):
    m = st._pose_matrix(pid)
    k = repr(m)
    if k not in LF:
        l, r = factors(m)
        LF[k] = (l, dist(l))
    return LF[k]


hist, per_step, maxh = Counter(), [], Counter()
for T, rec in enumerate(recs[:T1]):
    assert st.apply(sim.Twist.from_record(ctx, rec)).applied
    if T + 1 < T0 or (T + 1 - T0) % STRIDE:
        continue
    Kp = st._kpose()[st.pose_id]
    off = np.flatnonzero(Kp < 0)
    hi = []
    hval = {}
    for pid in sorted(set(st.pose_id[off].tolist())):
        l, dl = leftf(pid)
        maxh[dl] += int((st.pose_id[off] == pid).sum())
        if dl >= 4:
            hval[pid] = (l, dl)
            hi.extend(off[st.pose_id[off] == pid].tolist())
    if not hi:
        per_step.append((T + 1, 0, 0, len(V6), len(V8)))
        continue
    cand = np.zeros(600, bool)
    for chunk in range(0, len(hi), 400):
        ps = hi[chunk:chunk + 400]
        for p in ps:
            V = (repV[orb[p]] @ Kf[tr[p]].T) @ fm(st.pose(p)).T
            cand |= ((V @ NF.T - KAP) > -1e-6).all(0)
    ng = 0
    for g in np.flatnonzero(cand).tolist():
        cl = st.classify(g, need_inside=True, stop_at_straddle=True)
        if cl.straddle is not None or cl.uncertain:
            continue
        ins = np.asarray(cl.inside, np.int64)
        pids = sorted({int(x) for x in st.pose_id[ins].tolist()} & set(hval))
        if not pids:
            continue
        ng += 1
        for y in JI:
            ylq = yl(g, y)
            for pid in pids:
                l, dl = hval[pid]
                q = qmul(ylq, l)
                dq = dist(q)
                hist[(dl, dq)] += 1
                if dq == 6:
                    V6.add(q)
                elif dq == 8:
                    V8.add(q)
                elif dq >= 10:
                    V10.add(q)
    per_step.append((T + 1, len(hi), ng, len(V6), len(V8)))
    if (T + 1) % 25 == 0 or ng:
        print('T', T + 1, 'high pieces', len(hi), 'admissible grips', ng, 'V6', len(V6), 'V8', len(V8), 'V10+', len(V10),
              '%.0fs' % (time.time() - t0), flush=True)


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
out = {'T0': T0, 'T1': T1, 'stride': STRIDE, 'height_transitions': {f'{a}->{b}': n for (a, b), n in sorted(hist.items())},
       'height6_seed': s6, 'height6_closure': close(V6), 'total_height6': 5120,
       'height8_seed': s8, 'height8_closure': close(V8), 'total_height8': 5 * 4 ** 7, 'height10plus_seed': len(V10),
       'last_step_height_histogram': dict(maxh), 'per_step': per_step, 'seconds': round(time.time() - t0)}
print(json.dumps({k: v for k, v in out.items() if k != 'per_step'}))
json.dump(out, open(sys.argv[5], 'w'), indent=1)
