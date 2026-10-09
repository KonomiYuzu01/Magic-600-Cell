"""Exact arithmetic in Q(sqrt 5) and exact double description for the jumbling witness.

A number is stored as (a + b sqrt5) / d with Python integers and d > 0, kept in lowest terms.
Nothing here uses floating point; floats appear only where a caller converts a result for
display.
"""
from fractions import Fraction
from math import gcd


class Q5:
    __slots__ = ('a', 'b', 'd')

    def __init__(self, a=0, b=0, d=1):
        if d < 0:
            a, b, d = -a, -b, -d
        g = gcd(gcd(a, b), d)
        if g > 1:
            a, b, d = a // g, b // g, d // g
        self.a, self.b, self.d = a, b, d

    @staticmethod
    def of(x):
        if isinstance(x, Q5):
            return x
        if isinstance(x, int):
            return Q5(x, 0, 1)
        if isinstance(x, Fraction):
            return Q5(x.numerator, 0, x.denominator)
        raise TypeError(type(x))

    def __add__(self, o):
        o = Q5.of(o)
        return Q5(self.a * o.d + o.a * self.d, self.b * o.d + o.b * self.d, self.d * o.d)

    __radd__ = __add__

    def __neg__(self):
        return Q5(-self.a, -self.b, self.d)

    def __sub__(self, o):
        return self + (-Q5.of(o))

    def __rsub__(self, o):
        return Q5.of(o) - self

    def __mul__(self, o):
        o = Q5.of(o)
        return Q5(self.a * o.a + 5 * self.b * o.b, self.a * o.b + self.b * o.a, self.d * o.d)

    __rmul__ = __mul__

    def inv(self):
        n = self.a * self.a - 5 * self.b * self.b  # norm times d^2
        if n == 0:
            raise ZeroDivisionError('Q5 zero')
        return Q5(self.a * self.d, -self.b * self.d, n)

    def __truediv__(self, o):
        return self * Q5.of(o).inv()

    def __rtruediv__(self, o):
        return Q5.of(o) * self.inv()

    def sign(self):
        a, b = self.a, self.b
        if b == 0:
            return (a > 0) - (a < 0)
        if a == 0:
            return (b > 0) - (b < 0)
        if (a > 0) == (b > 0):
            return 1 if a > 0 else -1
        t = a * a - 5 * b * b
        s = (t > 0) - (t < 0)
        return s if a > 0 else -s

    def is_zero(self):
        return self.a == 0 and self.b == 0

    def __eq__(self, o):
        o = Q5.of(o)
        return self.a == o.a and self.b == o.b and self.d == o.d

    def __hash__(self):
        return hash((self.a, self.b, self.d))

    def __lt__(self, o):
        return (self - o).sign() < 0

    def __le__(self, o):
        return (self - o).sign() <= 0

    def __float__(self):
        return (self.a + self.b * 5 ** 0.5) / self.d

    def __repr__(self):
        return f'Q5({self.a},{self.b},{self.d})'


ZERO, ONE = Q5(0), Q5(1)


def from_float(x, max_q=12):
    """The unique (p + q sqrt5)/2 with |q| <= max_q within 1e-9 of x; raises otherwise."""
    s5 = 5 ** 0.5
    hits = []
    for q in range(-max_q, max_q + 1):
        p = round(2 * x - q * s5)
        if abs((p + q * s5) / 2 - x) < 1e-9:
            hits.append(Q5(p, q, 2))
    if len(hits) != 1:
        raise ValueError(f'no unique Q(sqrt5) value for {x}: {hits}')
    return hits[0]


def dot(u, v):
    s = ZERO
    for x, y in zip(u, v):
        s = s + x * y
    return s


def matvec(m, v):
    return [dot(row, v) for row in m]


def matmul(a, b):
    bt = list(zip(*b))
    return [[dot(r, c) for c in bt] for r in a]


def transpose(m):
    return [list(r) for r in zip(*m)]


def identity(n):
    return [[ONE if i == j else ZERO for j in range(n)] for i in range(n)]


def solve(m, rhs):
    """Exact Gaussian elimination: solve m x = rhs (square, nonsingular); rhs is a matrix."""
    n = len(m)
    a = [list(m[i]) + list(rhs[i]) for i in range(n)]
    for col in range(n):
        piv = next((r for r in range(col, n) if not a[r][col].is_zero()), None)
        if piv is None:
            raise ZeroDivisionError('singular')
        a[col], a[piv] = a[piv], a[col]
        inv = a[col][col].inv()
        a[col] = [x * inv for x in a[col]]
        for r in range(n):
            if r != col and not a[r][col].is_zero():
                f = a[r][col]
                a[r] = [x - f * y for x, y in zip(a[r], a[col])]
    return [row[n:] for row in a]


def rank(rows):
    a = [list(r) for r in rows]
    if not a:
        return 0
    rk, cols = 0, len(a[0])
    for col in range(cols):
        piv = next((r for r in range(rk, len(a)) if not a[r][col].is_zero()), None)
        if piv is None:
            continue
        a[rk], a[piv] = a[piv], a[rk]
        inv = a[rk][col].inv()
        a[rk] = [x * inv for x in a[rk]]
        for r in range(len(a)):
            if r != rk and not a[r][col].is_zero():
                f = a[r][col]
                a[r] = [x - f * y for x, y in zip(a[r], a[rk])]
        rk += 1
    return rk


def normalize_ray(r):
    """Positive rescaling: vertices get t = 1, directions get first nonzero entry of absolute value 1."""
    if not r[0].is_zero():
        inv = r[0].inv()
        return [x * inv for x in r]
    for x in r:
        if not x.is_zero():
            inv = x.inv() if x.sign() > 0 else (-x).inv()
            return [y * inv for y in r]
    raise ValueError('zero ray')


def double_description(constraints, dim=4):
    """Exact vertices of {x : a.x <= b for (a, b) in constraints}, which must be bounded and
    full-dimensional. Works on the homogenised cone {(t, x) : t >= 0, b t - a.x >= 0} in
    dimension dim + 1 with the combinatorial adjacency test. Returns (vertices, active sets);
    each active set lists the constraint indices tight at that vertex.
    """
    rows = [[Q5.of(b)] + [-Q5.of(x) for x in a] for a, b in constraints]
    rows.append([ONE] + [ZERO] * dim)  # t >= 0, index len(constraints)
    m = len(rows)
    # initial basis: greedy independent rows
    basis = []
    for i in [m - 1] + list(range(m - 1)):
        if rank([rows[j] for j in basis + [i]]) == len(basis) + 1:
            basis.append(i)
        if len(basis) == dim + 1:
            break
    if len(basis) < dim + 1:
        raise ValueError('constraints do not determine a pointed cone')
    inv = solve([rows[i] for i in basis], identity(dim + 1))  # columns are extreme rays
    rays = []
    for k in range(dim + 1):
        r = normalize_ray([inv[i][k] for i in range(dim + 1)])
        z = frozenset(basis[j] for j in range(dim + 1) if j != k)
        rays.append((r, z))
    done = set(basis)
    order = [i for i in range(m) if i not in done]
    for i in order:
        h = rows[i]
        vals = [dot(h, r) for r, _ in rays]
        plus = [k for k, v in enumerate(vals) if v.sign() > 0]
        minus = [k for k, v in enumerate(vals) if v.sign() < 0]
        zero = [k for k, v in enumerate(vals) if v.sign() == 0]
        if not minus:
            rays = [(r, z | {i}) if vals[k].sign() == 0 else (r, z) for k, (r, z) in enumerate(rays)]
            done.add(i)
            continue
        new = []
        for p in plus:
            for q in minus:
                common = rays[p][1] & rays[q][1]
                if len(common) < dim - 1:
                    continue
                if any(k not in (p, q) and common <= rays[k][1] for k in range(len(rays))):
                    continue
                rp, rq = rays[p][0], rays[q][0]
                vp, vq = vals[p], vals[q]
                r = [vp * y - vq * x for x, y in zip(rp, rq)]
                new.append((normalize_ray(r), common | {i}))
        rays = [rays[k] for k in plus] + [(rays[k][0], rays[k][1] | {i}) for k in zero] + new
        done.add(i)
    verts, act = [], []
    for r, z in rays:
        if r[0].is_zero():
            raise ValueError('region is unbounded')
        verts.append(r[1:])
        act.append(sorted(j for j in z if j < len(constraints)))
    return verts, act
