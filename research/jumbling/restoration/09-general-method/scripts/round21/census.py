"""E21-3 of round 21 (SENIOR-REVIEW.md section 26): the off pieces, their left heights and the high units of a
reachable state, and for each high unit the caps of its own tree branch that hold it whole, whether their cuts are
clean, how many pieces and lattice pieces they hold, and which of their 60 menu items lower the unit by 2.
A high unit is a set of off pieces of height at least 4 with one pose and one inside vector over the 600 caps.
Run from the repository root:
    python3 -I -B census.py witness <witness.json> <prefix length> <label> <out.json>
    python3 -I -B census.py fixture <records> <label> <out.json>
The first form replays the first <prefix length> letters of a stage 10 witness from the solved state, the second the
first <records> records of the W-J fixture of I_a. Research only: exact synthetic geometry; no Windows, DirectX,
input or performance claim.
"""
import sys, json, time
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import numpy as np
import sim, restore as R, trees as T
from exact import matmul
from sim.kplus import to_tuple

t0 = time.time()
C = sim.get_context(); D = C.data; KP = C.kplus
kind = sys.argv[1]
if kind == 'witness':
    M = R.menu('I_a')
    W = json.load(open(sys.argv[2]))['word']; n = int(sys.argv[3]); label = sys.argv[4]; dest = sys.argv[5]
    s = sim.State(C, filtered=False, menu=M)
    for c, j in W[:n]:
        o = s.apply(sim.Twist(C, c, R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
        assert o.applied and not o.uncertain
else:
    import wj
    fx, M = wj.load(C, 'I_a')
    n = int(sys.argv[2]); label = sys.argv[3]; dest = sys.argv[4]
    s = sim.State(C, menu=M)
    for rec in fx['journal']['records'][:n]:
        assert s.apply(sim.Twist.from_record(C, rec)).applied
    s.journal = []
print('state %s ready; %.0fs' % (label, time.time() - t0), flush=True)
kpose = s._kpose(); Kp = kpose[s.pose_id]
off = np.flatnonzero(Kp < 0)
info = {}
for pid in np.unique(s.pose_id[off]).tolist():
    dl, dr, bl, br = T.pose_tree(s._pose_matrix(pid))
    info[pid] = (dl, dr, bl)
hgt = np.zeros(len(s.pose_id), np.int64)
for pid, (dl, dr, bl) in info.items():
    hgt[s.pose_id == pid] = dl
hist = {}
for p in off.tolist():
    hist[int(hgt[p])] = hist.get(int(hgt[p]), 0) + 1
assert all(v[1] == 0 for v in info.values()), 'a right height is not 0'
hmax = max(hist) if hist else 0
print('off %d pieces, %d off poses; left heights %s; max %d; %.0fs' % (len(off), len(info), dict(sorted(hist.items())), hmax, time.time() - t0), flush=True)
high = off[hgt[off] >= 4]
GB = {x: T.branch(tuple(D.N[x])) for x in range(600)}
# every cap: inside set, clean or not
ins = np.zeros((len(high), 600), bool)
clean = np.zeros(600, bool); capn = np.zeros(600, np.int64); caplat = np.zeros(600, np.int64)
lat = Kp >= 0
for x in range(600):
    cl = s.classify(x, need_inside=True, stop_at_straddle=False)
    assert not cl.uncertain
    clean[x] = cl.straddle is None
    capn[x] = len(cl.inside); caplat[x] = int(lat[cl.inside].sum())
    if len(high):
        ins[:, x] = np.isin(high, cl.inside)
print('caps classified: %d clean; %.0fs' % (int(clean.sum()), time.time() - t0), flush=True)
# units among high pieces: same pose and same inside vector
units = {}
for r, p in enumerate(high.tolist()):
    key = (int(s.pose_id[p]), ins[r].tobytes())
    units.setdefault(key, []).append(p)
out = {'label': label, 'off': int(len(off)), 'off_poses': len(info), 'heights': {str(k): v for k, v in sorted(hist.items())},
       'max_height': hmax, 'clean_caps': int(clean.sum()), 'units': []}
for (pid, _), ps in sorted(units.items(), key=lambda kv: (-info[kv[0][0]][0], kv[1][0])):
    dl, dr, bl = info[pid]
    r0 = int(np.flatnonzero(high == ps[0])[0])
    caps = np.flatnonzero(ins[r0]).tolist()
    Q = s._pose_matrix(pid)
    lower = []
    for x in caps:
        if GB[x] != bl:
            continue
        items = []
        for j in range(60):
            l2, _ = T.factors(to_tuple(matmul(R.element(M, x, j), Q)))
            if T.dist(l2) == dl - 2:
                items.append(j)
        lower.append({'grip': x, 'clean': bool(clean[x]), 'cap_pieces': int(capn[x]), 'cap_lattice': int(caplat[x]), 'items': items})
    u = {'pieces': ps, 'height': dl, 'branch': bl, 'caps_whole': len(caps), 'clean_caps_whole': int(clean[caps].sum()) if caps else 0,
         'lowering': lower}
    out['units'].append(u)
    print('unit %d pieces (first %d) height %d branch %d: whole in %d caps (%d clean); lowering grips %s'
          % (len(ps), ps[0], dl, bl, len(caps), u['clean_caps_whole'],
             [(l['grip'], 'clean' if l['clean'] else 'blocked', l['cap_pieces'], l['cap_lattice'], len(l['items'])) for l in lower]), flush=True)
json.dump(out, open(dest, 'w'))
print('done; %.0fs' % (time.time() - t0))
