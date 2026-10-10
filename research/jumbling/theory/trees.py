"""J3-R tree model: the left and right quaternion factors of a pose on the two Bruhat-Tits trees at 2.

A rotation M of R^4 with entries in Q(sqrt5) is x -> l x conj(r) / s for quaternions l, r over Q(sqrt5)
(x = x0 + x1 i + x2 j + x3 k) and a scalar s. `factors` extracts (l, r) from the rank-one matrix
A[i][j] = l_i r_j = tr(E_ij^T M) / 4, where E_ij is the matrix of x -> e_i x conj(e_j), and every
extraction is checked exactly against M.

At the prime 2 (inert in Q(sqrt5)) the quaternion algebra splits, the icosian ring O is a maximal order,
and PGL_2(Q_4) acts on a 5-regular tree whose vertex v0 is O tensor Z_4. The tree distance of a
quaternion x is d(x) = v2(N(x')) for the primitive multiple x' = 2^-k x in O tensor Z_4; the branch of x
is the neighbour of v0 on the geodesic towards x v0, named by the tetrahedral subgroup of A5 = 2I/{+-1}
that fixes it. A pose is lattice (in K+) exactly when both distances are 0 (see restoration.md, T1).

Run from the repository root:
    python research/jumbling/theory/trees.py --check      exact checks and results/trees.json
    python research/jumbling/theory/trees.py --fixtures   height histograms of the W-J fixtures
"""
import itertools
import json
import random
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

from exact import Q5, ZERO, matmul, solve, transpose  # noqa: E402

HALF = Q5(1, 0, 2)
PHI = Q5(1, 1, 2)
ONEQ = (Q5(1), Q5(0), Q5(0), Q5(0))


# ------------------------------------------------------------------------------- quaternions
def qmul(p, q):
    a0, a1, a2, a3 = p
    b0, b1, b2, b3 = q
    return (a0 * b0 - a1 * b1 - a2 * b2 - a3 * b3,
            a0 * b1 + a1 * b0 + a2 * b3 - a3 * b2,
            a0 * b2 - a1 * b3 + a2 * b0 + a3 * b1,
            a0 * b3 + a1 * b2 - a2 * b1 + a3 * b0)


def qconj(p):
    return (p[0], -p[1], -p[2], -p[3])


def qnorm(p):
    return p[0] * p[0] + p[1] * p[1] + p[2] * p[2] + p[3] * p[3]


def qdot(p, q):
    return p[0] * q[0] + p[1] * q[1] + p[2] * q[2] + p[3] * q[3]


_E = [tuple(1 if k == i else 0 for k in range(4)) for i in range(4)]
_EM = [[None] * 4 for _ in range(4)]
for _i in range(4):
    for _j in range(4):
        _cols = [qmul(qmul(_E[_i], _E[_b]), qconj(_E[_j])) for _b in range(4)]
        _EM[_i][_j] = [[_cols[_b][_a] for _b in range(4)] for _a in range(4)]


def apply_lr(l, r, x):
    return qmul(qmul(l, x), qconj(r))


def factors(M, check=True):
    """(l, r) with M x proportional to l x conj(r), exactly; raises if the extraction fails."""
    A = [[ZERO] * 4 for _ in range(4)]
    for i in range(4):
        for j in range(4):
            s = ZERO
            e = _EM[i][j]
            for a in range(4):
                for b in range(4):
                    if e[a][b]:
                        s = s + M[a][b] * e[a][b]
            A[i][j] = s * Q5(1, 0, 4)
    l = next(tuple(A[i][j] for i in range(4)) for j in range(4) if any(not A[i][j].is_zero() for i in range(4)))
    r = next(tuple(A[i][j] for j in range(4)) for i in range(4) if any(not A[i][j].is_zero() for j in range(4)))
    if check and not reconstructs(M, l, r):
        raise ValueError('quaternion factor extraction failed')
    return l, r


def reconstructs(M, l, r):
    """M e_b = s * l e_b conj(r) for b = 0..3 with one common non-zero scalar s."""
    ratio = None
    for b in range(4):
        y = apply_lr(l, r, tuple(Q5(v) for v in _E[b]))
        for a in range(4):
            m = M[a][b]
            if m.is_zero():
                if not y[a].is_zero():
                    return False
                continue
            q = y[a] / m
            if ratio is None:
                ratio = q
            elif q != ratio:
                return False
    return ratio is not None and not ratio.is_zero()


# --------------------------------------------------------------------------- icosian units
def icosian_units():
    """The 120 unit icosians in standard coordinates (the binary icosahedral group 2I), exactly."""
    out = set()
    z = Q5(0)
    for i in range(4):
        for s in (1, -1):
            v = [z] * 4
            v[i] = Q5(s)
            out.add(tuple(v))
    for signs in itertools.product((1, -1), repeat=4):
        out.add(tuple(HALF * s for s in signs))
    a, b, c = PHI * HALF, HALF, (PHI - 1) * HALF
    even = [p for p in itertools.permutations(range(4))
            if sum(1 for x in range(4) for y in range(x + 1, 4) if p[x] > p[y]) % 2 == 0]
    for p in even:
        for s1, s2, s3 in itertools.product((1, -1), repeat=3):
            base = [a * s1, b * s2, c * s3, z]
            out.add(tuple(base[p[k]] for k in range(4)))
    units = sorted(out, key=lambda q: tuple(float(x) for x in q))
    assert len(units) == 120
    return units


UNITS = icosian_units()


# ------------------------------------------------------------------------------ 2-adic data
def _v2int(n):
    if n == 0:
        return None
    n, k = abs(n), 0
    while n % 2 == 0:
        n //= 2
        k += 1
    return k


def v2(x):
    """2-adic valuation on Q(sqrt5); 2 is inert, Z[phi] has basis (1, phi) and sqrt5 = 2 phi - 1."""
    x = Q5.of(x)
    if x.is_zero():
        raise ZeroDivisionError('v2(0)')
    a, b, d = x.a, x.b, x.d
    vals = [v for v in (_v2int(a - b), _v2int(2 * b)) if v is not None]
    return min(vals) - _v2int(d)


def in_zphi_half(x):
    """x in Z[phi][1/2]: (a + b sqrt5)/d = ((a - b) + 2 b phi)/d, and the odd part of d must divide both."""
    x = Q5.of(x)
    d = x.d
    while d % 2 == 0:
        d //= 2
    return (x.a - x.b) % d == 0 and (2 * x.b) % d == 0


def _det4(m):
    from sim.kplus import det4
    return det4(m)


def _local_basis():
    """Four unit icosians whose Gram determinant (reduced trace form) is a 2-adic unit. Their span is
    a sublattice of O tensor Z_(2)[phi] with unit discriminant, hence all of it (O is maximal with
    unit discriminant)."""
    rng = random.Random(0)
    while True:
        B = rng.sample(UNITS, 4)
        g = _det4([[qdot(a, b) * 2 for b in B] for a in B])
        if not g.is_zero() and v2(g) == 0:
            return B


BASIS = _local_basis()
_BT = [[BASIS[j][i] for j in range(4)] for i in range(4)]
_BINV = solve(_BT, [[Q5(1) if i == j else Q5(0) for j in range(4)] for i in range(4)])


def coords(x):
    return [sum((_BINV[i][k] * x[k] for k in range(4)), ZERO) for i in range(4)]


def kval(x):
    """Largest k with 2^-k x in O tensor Z_(2)[phi]."""
    return min(v2(c) for c in coords(x) if not c.is_zero())


def dist(x):
    """Tree distance from v0 to x v0."""
    return v2(qnorm(x)) - 2 * kval(x)


# ---------------------------------------------------------------------------------- branches
def qkey(q):
    for c in q:
        if not c.is_zero():
            s = 1 if c.sign() > 0 else -1
            return tuple(x * s for x in q)
    raise ValueError('zero quaternion')


PUNITS = sorted({qkey(u) for u in UNITS}, key=lambda q: tuple(float(x) for x in q))   # A5, 60


def _order(u):
    p, n = u, 1
    while qkey(p) != qkey(ONEQ):
        p, n = qmul(p, u), n + 1
    return n


def _closure(gens):
    S = {qkey(ONEQ)}
    frontier = list(S)
    while frontier:
        nxt = []
        for a in frontier:
            for g in gens:
                b = qkey(qmul(a, g))
                if b not in S:
                    S.add(b)
                    nxt.append(b)
        frontier = nxt
    return frozenset(S)


def _tetrahedral_subgroups():
    tets = set()
    inv2 = [u for u in PUNITS if _order(u) == 2]
    ord3 = [u for u in PUNITS if _order(u) == 3]
    for a in inv2:
        for b in inv2:
            if a != b and len(_closure([a, b])) == 4:
                for c in ord3:
                    H = _closure([a, b, c])
                    if len(H) == 12:
                        tets.add(H)
    tets = sorted(tets, key=lambda S: sorted(tuple(float(x) for x in q) for q in S))
    assert len(tets) == 5
    return tets


TETS = _tetrahedral_subgroups()


def branch(x):
    """Index (0..4) of the neighbour of v0 on the geodesic towards x v0, or -1 when d(x) = 0. The
    neighbour w is named by Stab(v0) n Stab(w), the elements u of A5 with d(x^-1 u x) <= 2 d(x) - 2."""
    d = dist(x)
    if d == 0:
        return -1
    xi = qconj(x)
    fix = {u for u in PUNITS if dist(qmul(qmul(xi, u), x)) <= 2 * d - 2}
    for t, S in enumerate(TETS):
        if fix == set(S):
            return t
    raise ValueError('branch not identified')


def pose_tree(M):
    """(d_L, d_R, branch_L, branch_R) of an exact rotation matrix."""
    l, r = factors(M)
    return dist(l), dist(r), branch(l), branch(r)


# ------------------------------------------------------------------------------------ checks
def _check():
    import sim
    import wj
    from sim.kplus import to_tuple
    ctx = sim.get_context()
    kp = ctx.kplus
    t0 = time.time()

    def is_unit_icosian(q):
        for u in UNITS:
            lam = None
            ok = True
            for a, b in zip(q, u):
                if b.is_zero():
                    if not a.is_zero():
                        ok = False
                        break
                    continue
                t = a / b
                if lam is None:
                    lam = t
                elif t != lam:
                    ok = False
                    break
            if ok:
                return True
        return False

    bad = 0
    for idx in range(len(kp.perms)):
        l, r = factors(kp.matrix(idx))
        if not (is_unit_icosian(l) and is_unit_icosian(r)) or dist(l) or dist(r):
            bad += 1
    out = {'kplus': {'elements': len(kp.perms), 'failures': bad,
                     'statement': 'every K+ element is x -> u x conj(w) with u, w unit icosians; both distances 0'}}
    print('K+ checked', len(kp.perms), 'failures', bad, round(time.time() - t0), 's', flush=True)
    menus = {name: wj.make_menu(ctx, name) for name in ('S4', 'I_a', 'I_b')}
    pattern = {name: {} for name in menus}
    kappa = {}
    structure_failures = []
    for c in range(600):
        fac = {}
        for name, M in menus.items():
            f = M.frame(c)
            counts = {}
            fac[name] = []
            for lab, m0 in M.items:
                g = to_tuple(matmul(matmul(f, m0), transpose(f)))
                if not all(in_zphi_half(x) for row in g for x in row):
                    structure_failures.append((name, c, 'entries outside Z[phi][1/2]'))
                l, r = factors(g)
                v = (dist(l), dist(r))
                fac[name].append((l, r, v))
                counts[v] = counts.get(v, 0) + 1
            key = str(sorted((k[0], k[1], n) for k, n in counts.items()))
            pattern[name][key] = pattern[name].get(key, 0) + 1
        # S4: every odd element inverts the same edge in each tree: d = (1, 1), its square fixes v0,
        # and all odd elements have the same branches (kappa_L(c), kappa_R(c)).
        odd = [(l, r) for l, r, v in fac['S4'] if v != (0, 0)]
        br = {(branch(l), branch(r)) for l, r in odd}
        sq = all(dist(qmul(l, l)) == 0 and dist(qmul(r, r)) == 0 for l, r in odd)
        if len(br) != 1 or not sq or any(v not in ((0, 0), (1, 1)) for _, _, v in fac['S4']):
            structure_failures.append(('S4', c))
        kappa[c] = br.pop() if len(br) == 1 else None
        # I_a fixes w_L = s_L v0 on the left and v0 on the right; I_b mirrors it (s = an odd S4 element).
        sl, sr = odd[0]
        for name, side in (('I_a', 0), ('I_b', 1)):
            s = (sl, sr)[side]
            si = qconj(s)
            for l, r, v in fac[name]:
                moved, fixed = (l, r) if side == 0 else (r, l)
                if dist(qmul(qmul(si, moved), s)) != 0 or dist(fixed) != 0:
                    structure_failures.append((name, c))
                    break
        if c % 100 == 99:
            print('caps checked', c + 1, round(time.time() - t0), 's', flush=True)
    classes = {}
    for c, k in kappa.items():
        classes[str(k)] = classes.get(str(k), 0) + 1
    out['menus'] = {name: {'distance_patterns': pattern[name]} for name in menus}
    out['S4_branch_classes'] = classes
    out['kappa'] = {str(c): list(k) if k else None for c, k in kappa.items()}
    out['structure_failures'] = structure_failures
    out['statements'] = [
        'S4_c: 12 elements at (0, 0) (A4_c) and 12 at (1, 1); every odd element inverts the edges '
        '{v0, w_L} and {v0, w_R} named by (kappa_L(c), kappa_R(c)); each of the 25 pairs occurs on 24 caps',
        'I_a_c fixes w_L(c) on the left tree and v0 on the right tree; I_b_c fixes v0 on the left and w_R(c) on the right',
        'every transported menu element has entries in Z[phi][1/2]',
    ]
    print('patterns', pattern, 'branch classes', classes, 'structure failures', len(structure_failures), flush=True)
    bad += len(structure_failures)
    res = HERE / 'results' / 'trees.json'
    res.write_text(json.dumps(out, indent=1) + '\n')
    print('wrote', res.relative_to(HERE.parent.parent.parent))
    return bad == 0


def _fixtures():
    """Height histograms of the W-J fixture states at fixed checkpoints (lead, exact states)."""
    import numpy as np
    import sim
    import wj
    ctx = sim.get_context()
    out = {}
    for name in ('S4', 'I_a', 'I_b'):
        fixture, M = wj.load(ctx, name)
        recs = fixture['journal']['records']
        marks = sorted({10, 20, 60, 150, 300, len(recs)} & set(range(1, len(recs) + 1)))
        st = sim.State(ctx, menu=M)
        memo = {}
        rows = []
        t0 = time.time()
        for t, rec in enumerate(recs, 1):
            if not st.apply(sim.Twist.from_record(ctx, rec)).applied:
                raise RuntimeError(f'{name}: fixture twist {t} refused')
            if t not in marks:
                continue
            cnt = np.bincount(st.pose_id, minlength=len(st._poses))
            pieces, classes = {}, {}
            for pid in np.nonzero(cnt)[0]:
                p = st._poses[int(pid)]
                if p.key not in memo:
                    memo[p.key] = (0, 0) if p.kidx >= 0 else tuple(pose_tree(st._pose_matrix(int(pid)))[:2])
                k = '%d,%d' % memo[p.key]
                pieces[k] = pieces.get(k, 0) + int(cnt[pid])
                classes[k] = classes.get(k, 0) + 1
            rows.append({'t': t, 'pieces_by_heights': dict(sorted(pieces.items())),
                         'pose_classes_by_heights': dict(sorted(classes.items()))})
            print(name, 't', t, rows[-1]['pieces_by_heights'], round(time.time() - t0), 's', flush=True)
        out[name] = rows
    res = HERE / 'results' / 'trees-fixtures.json'
    res.write_text(json.dumps(out, indent=1) + '\n')
    print('wrote', res.relative_to(HERE.parent.parent.parent))


if __name__ == '__main__':
    if '--check' in sys.argv:
        sys.exit(0 if _check() else 1)
    if '--fixtures' in sys.argv:
        _fixtures()
        sys.exit(0)
    print(__doc__)
