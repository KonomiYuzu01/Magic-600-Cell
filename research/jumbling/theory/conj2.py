"""J3: are the lattice endpoints of W = (0,q)(d,a)(0,q^-1) single retained twists?"""
import sys, json, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))
import numpy as np
import sim
from exact import ZERO
from groups import build, lift
ctx = sim.get_context()
single = {}
t0 = time.time()
for c in range(600):
    for i in range(1, 12):
        st = sim.State(ctx); st.apply(sim.a4_element(ctx, c, i)); single[st.digest()] = (c, i)
print('single retained digests', len(single), round(time.time() - t0), 's', flush=True)
ex = build()
import math
def angle3(m):
    tr = float(sum((m[k][k] for k in range(3)), ZERO)); return round(math.degrees(math.acos(max(-1, min(1, (tr - 1) / 2)))))
out = {}
for name, target in (('S4', 90), ('I_a', 72)):
    m = next(m for m in ex[name] if angle3(m) == target)
    q = sim.Twist(ctx, 0, lift(m)); qi = q.inverse()
    rows = json.load(open(f'{sys.argv[1]}/conj-{name}.json'))['rows']
    matched, unmatched = 0, []
    for r in rows:
        st = sim.State(ctx)
        for tw in (q, sim.a4_element(ctx, r['d'], r['a']), qi):
            assert st.apply(tw).applied
        k = single.get(st.digest())
        if k is not None:
            matched += 1
        else:
            unmatched.append({'d': r['d'], 'a': r['a'], 'moved': r['moved'],
                              'checkpoint': st.checkpoint()['checkpoint'],
                              'centres_ok': st.checkpoint().get('reason') != 'a centre has moved to another chamber'})
    out[name] = {'lattice_endpoints': len(rows), 'equal_to_single_retained_twist': matched, 'other': unmatched}
    print(name, out[name]['lattice_endpoints'], matched, unmatched[:10], flush=True)
json.dump(out, open(f'{sys.argv[1]}/conj2.json', 'w'), indent=1)
