"""J3 lead: exact random walks of J(Lambda) for S4_0 and the icosahedral menus.
Tracks the pose table, entry heights and blocked counts. Leads only, not proofs."""
import sys, json, time, random
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))
import numpy as np
import sim
from groups import build, lift
from sim.twists import TwistMenu

name, steps, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
ctx = sim.get_context()
ex = build()
base = [(f'{name}[{i}]', lift(m)) for i, m in enumerate(ex[name])]
menu = TwistMenu(ctx, name, base, close=True)
assert menu.invariant and len(menu) == len(ex[name])
rng = random.Random(seed)
st = sim.State(ctx, menu=menu)
jumble = [i for i, (lab, m) in enumerate(menu.items) if not any(m == ctx.kplus.matrix(s) for s in ctx.kplus.stab0)]
log = []
t0 = time.time()
tries = blocked = 0
def height():
    h = 0
    for pid, p in enumerate(st._poses):
        if p.kidx >= 0:
            continue
        for r in st._pose_matrix(pid):
            for x in r:
                h = max(h, abs(x.d), abs(x.a), abs(x.b))
    return h
applied = 0
while applied < steps and time.time() - t0 < float(sys.argv[4]):
    c = rng.randrange(600)
    i = rng.choice(jumble)
    lab, m0 = menu.items[i]
    f = menu.frame(c)
    from exact import matmul, transpose
    from sim.kplus import to_tuple
    g = to_tuple(matmul(matmul(f, m0), transpose(f)))
    tw = sim.Twist(ctx, c, g, 'menu', {'menu': name, 'label': lab})
    tries += 1
    out = st.apply(tw)
    if out.status == 'blocked':
        blocked += 1
        continue
    if not out.applied:
        print('status', out.status); continue
    applied += 1
    if applied % 5 == 0 or applied <= 5:
        row = {'step': applied, 'tries': tries, 'blocked': blocked, 'poses': len(st._poses),
               'off': int(st.off_lattice_count()), 'height': height(), 't': round(time.time() - t0, 1)}
        log.append(row); print(json.dumps(row), flush=True)
json.dump({'menu': name, 'seed': seed, 'log': log, 'journal': st.journal_json()}, open(f'{sys.argv[5]}', 'w'))
