"""After one jumble twist (0, x) from solved, for every grip d != 0 whose cap contains a vacated slot of the given
orbits: is the twist at d admissible (no straddle, exact), and is its cap clean (no off piece wholly inside)?
usage: b45_access.py <research/jumbling> <out json> <orbit ...>"""
import json, sys, time
from collections import Counter, defaultdict
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
import restore as RS
ctx = sim.get_context(); d = ctx.data
M = RS.menu('I_a')
orbs = [int(x) for x in sys.argv[3:]]
NF = np.array([[float(v) for v in row] for row in d.N])
ang = np.degrees(np.arccos(np.clip(NF @ NF[0] / float(d.NN), -1, 1)))
members = [int(p) for p in d.cap_members(0)]
t0 = time.time()
rows, tally = [], Counter()
for x in RS.jumble_items(M):
    st = sim.State(ctx, menu=M)
    assert st.apply(sim.Twist(ctx, 0, RS.element(M, 0, x), 'menu', {'menu': M.name})).applied
    off = set(np.flatnonzero(st._kpose()[st.pose_id] < 0).tolist())
    grips = defaultdict(set)
    for p in members:
        o = int(d.orbit_id[p])
        if o in orbs:
            for c in d.signature_list(p):
                if int(c) != 0:
                    grips[int(c)].add(o)
    res = {}
    for c, os_ in sorted(grips.items()):
        cl = st.classify(c, need_inside=True, stop_at_straddle=True)
        adm = cl.straddle is None and not cl.uncertain
        clean = adm and not (set(cl.inside.tolist()) & off)
        key = (round(float(ang[c]), 2), 'clean' if clean else ('admissible' if adm else 'blocked'))
        tally[key] += 1
        res[c] = {'angle': key[0], 'orbits': sorted(os_), 'admissible': adm, 'clean': clean}
    rows.append({'item': int(x), 'grips': res})
    print('item', x, Counter((v['angle'], v['admissible'], v['clean']) for v in res.values()), '%.0fs' % (time.time() - t0), flush=True)
out = {'orbits': orbs, 'jumble_items': len(rows),
       'tally (angle, status)': {f'{a} {s}': n for (a, s), n in sorted(tally.items())}, 'rows': rows,
       'seconds': round(time.time() - t0, 1)}
json.dump(out, open(sys.argv[2], 'w'), indent=1)
print(json.dumps(out['tally (angle, status)']))
