"""Height-4 left positions: enumerate all 320 vertices at tree distance 4, find which ones occur as left positions
of poses in the I_a W-J fixture (at every 25th record and the end), and close under the left action of 2I (global
K+ conjugation of a reachable history maps a reachable left position t to u t). Exact quaternion arithmetic."""
import json, sys, time
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
M = RS.menu('I_a')
JI = RS.jumble_items(M)
cls = {int(k): v for k, v in json.load(open(sys.argv[2])).items()}
rep = {}
for c in range(600):
    rep.setdefault(cls[c], c)
same = lambda a, b: dist(qmul(qconj(a), b)) == 0
# frames: 4 per class
frames = []                     # (class, quaternion)
jl = {}
for P, c in sorted(rep.items()):
    ls = [factors(RS.element(M, c, i))[0] for i in JI]
    jl[P] = ls
    fr = []
    for l in ls:
        assert dist(l) == 2
        if not any(same(f, l) for f in fr):
            fr.append(l)
    assert len(fr) == 4
    frames += [(P, f) for f in fr]
print('frames', len(frames), flush=True)
def frame_of(q):
    hits = [k for k, (P, f) in enumerate(frames) if dist(qmul(qconj(f), q)) == 2]
    return hits
V4 = {}                         # frame index -> list of reps
for k, (P1, f) in enumerate(frames):
    for P2 in rep:
        if P2 == P1:
            continue
        for y in jl[P2]:
            q = qmul(y, f)
            if dist(q) != 4:
                continue
            fk = frame_of(q)
            assert len(fk) == 1, fk
            lst = V4.setdefault(fk[0], [])
            if not any(same(r, q) for r in lst):
                lst.append(q)
allv = [(k, i) for k in sorted(V4) for i in range(len(V4[k]))]
print('height-4 vertices', len(allv), 'per frame', sorted({len(v) for v in V4.values()}), flush=True)
def ident(q):
    assert dist(q) == 4
    fk = frame_of(q)
    assert len(fk) == 1
    for i, r in enumerate(V4[fk[0]]):
        if same(r, q):
            return (fk[0], i)
    raise ValueError('unidentified')
# fixture
fixture, Mf = wj.load(ctx, 'I_a')
recs = fixture['journal']['records']
st = sim.State(ctx, menu=Mf)
seen = {}
memo = {}
t0 = time.time()
for t, rec in enumerate(recs, 1):
    assert st.apply(sim.Twist.from_record(ctx, rec)).applied
    if t % 25 and t != len(recs):
        continue
    cnt = np.bincount(st.pose_id, minlength=len(st._poses))
    for pid in np.nonzero(cnt)[0]:
        P = st._poses[int(pid)]
        if P.kidx >= 0 or P.key in memo:
            continue
        l, r = factors(st._pose_matrix(int(pid)))
        dl = dist(l)
        memo[P.key] = dl
        if dl == 4:
            v = ident(l)
            seen.setdefault(v, t)
        elif dl > 4:
            print('HEIGHT', dl, 'at record', t, flush=True)
print('fixture: distinct height-4 left positions', len(seen), '%.0fs' % (time.time() - t0), flush=True)
# closure under 2I
reach = set(seen)
frontier = list(reach)
qof = lambda v: V4[v[0]][v[1]]
while frontier:
    nxt = []
    for v in frontier:
        q = qof(v)
        for u in UNITS:
            w = ident(qmul(u, q))
            if w not in reach:
                reach.add(w); nxt.append(w)
    frontier = nxt
print('after 2I closure', len(reach), 'of', len(allv))
# 2I orbits on all 320
orbits = []
left = set(allv)
while left:
    v = min(left)
    orb = {ident(qmul(u, qof(v))) for u in UNITS}
    orbits.append(sorted(orb))
    left -= orb
print('2I orbits on height-4 vertices:', [len(o) for o in orbits])
print('orbits hit by the fixture:', [i for i, o in enumerate(orbits) if set(o) & set(seen)])
json.dump({'n_height4': len(allv), 'orbit_sizes': [len(o) for o in orbits],
           'fixture_hits': {f'{k}': v for k, v in seen.items()}, 'closure': len(reach),
           'orbits_hit': [i for i, o in enumerate(orbits) if set(o) & set(seen)]}, open(sys.argv[3], 'w'), indent=1)
