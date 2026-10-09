"""J3: exact S4_0 and the icosahedral groups containing A4_0 (cap-frame 3x3 and lifted 4x4)."""
import sys, math
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import numpy as np
import sim
from exact import Q5, ZERO, ONE, dot, matmul, matvec, transpose, identity
from sim.twists import cap_frame
from sim.kplus import to_tuple, is_rotation

ctx = sim.get_context()
n = ctx.data.N[0]; nn = dot(n, n); u = cap_frame(n)
PHI = (1 + 5 ** 0.5) / 2
VALS = {}
for s in (1, -1):
    VALS[s * 1.0] = Q5(s); VALS[s * 0.5] = Q5(s, 0, 2)
    VALS[s * PHI / 2] = Q5(s, s, 4); VALS[s / (2 * PHI)] = Q5(-s, s, 4)
VALS[0.0] = ZERO

def exact_of(f):
    for k, v in VALS.items():
        if abs(f - k) < 1e-12:
            return v
    raise ValueError(f)

def to3(g):
    return [[dot(u[i], matvec(g, u[j])) / nn for j in range(3)] for i in range(3)]

def lift(r):
    inv = nn.inv()
    return to_tuple([[(n[a] * n[b] + sum((r[i][j] * u[i][a] * u[j][b] for i in range(3) for j in range(3)), ZERO)) * inv
                      for b in range(4)] for a in range(4)])

def fclose(gens, cap=200):
    key = lambda m: tuple(np.round(m, 9).ravel())
    seen = {key(np.eye(3)): np.eye(3)}
    fr = [np.eye(3)]
    while fr and len(seen) <= cap:
        nx = []
        for a in fr:
            for g in gens:
                c = g @ a
                if key(c) not in seen:
                    seen[key(c)] = c; nx.append(c)
        fr = nx
    return list(seen.values())

def rot(axis, deg):
    a = np.asarray(axis, float); a /= np.linalg.norm(a)
    t = math.radians(deg); K = np.array([[0, -a[2], a[1]], [a[2], 0, -a[0]], [-a[1], a[0], 0]])
    return np.eye(3) + math.sin(t) * K + (1 - math.cos(t)) * K @ K

A4_4 = [sim.a4_element(ctx, 0, i).matrix for i in range(12)]
A4_3 = [np.array([[float(x) for x in r] for r in to3([list(r) for r in g])]) for g in A4_4]

def build():
    out = {'S4': fclose(A4_3 + [rot((1, 0, 0), 90)])}
    out['I_a'] = fclose(A4_3 + [rot((0, 1, PHI), 72)])
    out['I_b'] = fclose(A4_3 + [rot((0, PHI, 1), 72)])
    ex = {}
    for k, v in out.items():
        assert len(v) in (24, 60), (k, len(v))
        ex[k] = [to_tuple([[exact_of(x) for x in r] for r in m]) for m in v]
        for m in ex[k]:
            g = lift(m)
            assert is_rotation(g) and matvec(g, n) == list(n)
        # the float search only proposes; the group property is checked exactly
        keys = set(ex[k])
        assert len(keys) == len(v)
        assert all(to_tuple(matmul(a, b)) in keys for a in ex[k] for b in ex[k]), f'{k} is not closed'
        assert all(to_tuple(to3([list(r) for r in g])) in keys for g in A4_4), f'{k} misses A4_0'
    return ex

if __name__ == '__main__':
    ex = build()
    for k, v in ex.items():
        print(k, len(v), 'exact rotations fixing n0: ok')
