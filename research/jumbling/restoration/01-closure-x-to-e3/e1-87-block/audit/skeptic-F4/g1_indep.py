"""Independent G1 recompute for W2 = P2 Q2 P2^-1 (skeptic-F4).

G1 records, for X = Q2's third letter (115, 26), the pole X^-1 n_p of every other grip p of W2.
This recomputes it with its own exact arithmetic in Q(sqrt5) (pairs of Fractions), and finds the matching
normal by exact comparison against every cap normal, not through the simulator's POLE dictionary.
G1 depends only on the menu elements and the normals, not on the state; the table is only needed to open the menu.

Usage: python3 -I -B g1_indep.py <theory dir> <pose table> <W json> <out json>
"""
import json
import sys
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402
from restore import element  # noqa: E402

data = BL.CL.ctx.data


def num(x):
    """(rational part, sqrt5 part) as Fractions."""
    if isinstance(x, int):
        return (Fraction(x), Fraction(0))
    if hasattr(x, 'a') and hasattr(x, 'b') and hasattr(x, 'd'):
        return (Fraction(x.a, x.d), Fraction(x.b, x.d))
    if isinstance(x, (tuple, list)) and len(x) == 3:
        a, b, d = x
        return (Fraction(a, d), Fraction(b, d))
    raise TypeError(type(x))


def mul(u, v):
    return (u[0] * v[0] + 5 * u[1] * v[1], u[0] * v[1] + u[1] * v[0])


def add(u, v):
    return (u[0] + v[0], u[1] + v[1])


def mat(m):
    return [[num(x) for x in row] for row in m]


def tvec(m, v):
    """m^T v."""
    out = []
    for j in range(4):
        s = (Fraction(0), Fraction(0))
        for i in range(4):
            s = add(s, mul(m[i][j], v[i]))
        out.append(s)
    return out


def main():
    a = sys.argv[1:]
    R, M = BL.O.start(a[1])
    W = json.loads(Path(a[2]).read_text())
    P = [tuple(x) for x in W['P']]
    Q = [tuple(x) for x in W['Q']]
    gc, ix = Q[2]
    X = mat(element(M, gc, ix))
    normals = [[num(x) for x in row] for row in data.N]
    grips = sorted({c for c, _ in P + Q} - {gc})
    res = {'X': [gc, ix], 'n_normals': len(normals), 'G1': {}}
    for p in grips:
        v = tvec(X, normals[p])
        hits = [q for q, n in enumerate(normals) if n == v]
        res['G1'][str(p)] = hits
    # transpose of X must be the letter (115, 58) recorded as X^-1
    Xinv = mat(element(M, gc, 58))
    res['X58_equals_X_transpose'] = all(Xinv[i][j] == X[j][i] for i in range(4) for j in range(4))
    Path(a[3]).write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps(res))


if __name__ == '__main__':
    main()
