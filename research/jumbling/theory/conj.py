"""J3 item 4 lead: conjugation words W = (0,q) (d,a) (0,q^-1) in J(S4) and J(I).
Which are admissible, which end on the lattice, and do lattice endpoints keep the retained
necessary invariants (centres fixed, even positional parity in every orbit)?"""
import sys, json, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))
import numpy as np
import sim
from exact import ZERO
from groups import build, lift

ctx = sim.get_context(); kp, reg, data = ctx.kplus, ctx.regions, ctx.data
menu_name = sys.argv[1]
ex = build()
def angle3(m):
    import math
    tr = float(sum((m[k][k] for k in range(3)), ZERO)); return round(math.degrees(math.acos(max(-1, min(1, (tr - 1) / 2)))))
target = 90 if menu_name == 'S4' else 72
m = next(m for m in ex[menu_name] if angle3(m) == target)
q = sim.Twist(ctx, 0, lift(m)); qi = q.inverse()
orbits = reg.orbit_of
def parity(perm):
    seen = np.zeros(len(perm), bool); odd = 0
    for i in range(len(perm)):
        if not seen[i]:
            j, L = i, 0
            while not seen[j]:
                seen[j] = True; j = perm[j]; L += 1
            odd ^= (L - 1) & 1
    return odd
def invariants(st):
    K = st._kpose()[st.pose_id]
    dest = reg.piece_of[orbits, kp.compose_vec(K, reg.transport)]
    kc = K[data.centre_pieces]
    out = {'centres_fixed': bool(np.all(kp.perms[kc, data.centre_poles] == data.centre_poles)), 'odd_orbits': []}
    for o in np.unique(orbits):
        mem = np.nonzero(orbits == o)[0]
        idx = {p: i for i, p in enumerate(mem.tolist())}
        perm = np.array([idx[int(x)] for x in dest[mem]])
        if parity(perm):
            out['odd_orbits'].append(int(o))
    return out
t0 = time.time()
rows = []
counts = {'second_blocked': 0, 'third_blocked': 0, 'off_lattice_end': 0, 'lattice_end': 0}
for d in range(1, 600):
    for i in range(1, 12):
        a = sim.a4_element(ctx, d, i)
        st = sim.State(ctx)
        st.apply(q)
        o = st.apply(a)
        if not o.applied:
            counts['second_blocked'] += 1; continue
        o = st.apply(qi)
        if not o.applied:
            counts['third_blocked'] += 1; continue
        if not st.is_lattice():
            counts['off_lattice_end'] += 1; continue
        counts['lattice_end'] += 1
        if st.digest() == sim.State(ctx).digest():
            continue
        inv = invariants(st)
        rows.append({'d': d, 'a': i, 'moved': int(st.moved_count()), **inv})
print(menu_name, counts, 'nontrivial lattice', len(rows), 'with odd orbits', sum(bool(r['odd_orbits']) for r in rows),
      'with moved centres', sum(not r['centres_fixed'] for r in rows), round(time.time() - t0), 's')
json.dump({'menu': menu_name, 'counts': counts, 'rows': rows}, open(sys.argv[2], 'w'))
