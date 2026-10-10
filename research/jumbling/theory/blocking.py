"""J3 item 2: blocked grips after one twist (c, g) from solved, predicted from the cap
polytope K_c = cl(U_c cap P) alone, compared with the J1 survey. Exact throughout."""
import sys, json, random, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))
import numpy as np
import sim
from exact import Q5, ZERO, ONE, dot, matvec, transpose, double_description
from sim.twists import cayley
from groups import build, lift

ctx = sim.get_context(); data = ctx.data
N, NN, KAPPA = data.N, data.NN, data.KAPPA
pole_key = {tuple(v): i for i, v in enumerate(N)}

def cap_polytope(c):
    nf = np.array([[float(x) for x in v] for v in N])
    near = [e for e in range(600) if nf[e] @ nf[c] > 0.5 * float(NN)]
    cons = [(N[e], NN) for e in near] + [([-x for x in N[c]], -KAPPA)]
    verts, _ = double_description(cons)
    for v in verts:                       # superset from a constraint subset: verify all 600 facets
        assert all(dot(N[e], v) <= NN for e in range(600))
        assert dot(N[c], v) >= KAPPA
    return verts

def predict(c, g, verts):
    gi = transpose(g)
    out = {}
    for d in range(600):
        if d == c:
            out[d] = 'admissible'; continue
        w = matvec(gi, N[d])
        if tuple(w) in pole_key:
            out[d] = 'admissible'; continue
        vals = [dot(w, v) for v in verts]
        below = any((x - KAPPA).sign() < 0 for x in vals)
        above = any((x - KAPPA).sign() > 0 for x in vals)
        out[d] = 'blocked' if (below and above) else 'admissible'
    return out

if __name__ == '__main__':
    t0 = time.time()
    verts = cap_polytope(0)
    print('cap polytope K_0 vertices', len(verts), round(time.time() - t0, 1), 's')
    ex = build()
    twists = {'plane10_0_13': sim.plane(ctx, 0, 13, degrees=10),
              'S4_q90_u1': cayley(ctx, 0, [1, 0, 0])}
    for name in ('I_a', 'I_b'):
        for i, m in enumerate(ex[name]):
            g = lift(m)
            tr = sum((m[k][k] for k in range(3)), ZERO)
            if abs(float(tr) - (2 * 0.30901699437494745 * 1 + 1)) < 1e-9:   # 72 degrees: 1 + 2 cos 72
                twists[f'{name}_72'] = sim.Twist(ctx, 0, g); break
    rng = random.Random(7)
    for i in range(4):
        w = [Q5(rng.randint(-9, 9), 0, rng.randint(1, 9)) for _ in range(3)]
        twists[f'cayley_rand{i}'] = cayley(ctx, 0, w)
    res = {}
    for name, tw in twists.items():
        st = sim.State(ctx)
        assert st.apply(tw).applied
        sv = st.survey()
        pred = predict(0, tw.matrix, verts)
        agree = sum(pred[d] == sv[d]['status'] for d in range(600))
        nb = sum(v['status'] == 'blocked' for v in sv.values())
        realigned = sum(tuple(matvec(transpose(tw.matrix), N[d])) in pole_key for d in range(600))
        res[name] = {'angle': tw.angle_deg(), 'blocked_J1': nb, 'agree': agree, 'realigned_poles': realigned}
        print(name, res[name], flush=True)
    json.dump(res, open(sys.argv[1], 'w'), indent=1)
