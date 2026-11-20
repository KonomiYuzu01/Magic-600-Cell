"""Write CT-LB1-01's tree reference: exact answers of research/jumbling/theory/trees.py (pinned) as JSON.

Usage, from the repository root: python rust/c600-library/tests/fixtures/trees_reference.py . > \
    rust/c600-library/tests/fixtures/trees-reference.json

Read-only: it imports trees.py and exact.py and writes nothing but standard output. Every Q(sqrt 5) value is an exact
triple [a, b, d] for (a + b sqrt 5)/d in exact.py's canonical form (lowest terms, d > 0). A quaternion is a list of four
values (x0 + x1 i + x2 j + x3 k), and a 4x4 matrix is a list of rows: m[a][b] is coefficient a of the image of e_b.
"""
import hashlib
import json
import sys
from pathlib import Path

root = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(root / 'research' / 'jumbling' / 'theory'))
import trees  # noqa: E402
from exact import Q5  # noqa: E402

PINS = {
    'research/jumbling/theory/trees.py': '8431eedfb778a68b04e5938f3bcfee92feac8e54e718471ea33d6ccedb55a642',
    'research/jumbling/exact.py': '763dc7803c82971d76520c53cfb2a492e230e901dcf9cfa6a8fbdf34308bc7a6',
}
for rel, pin in PINS.items():
    got = hashlib.sha256((root / rel).read_bytes()).hexdigest()
    if got != pin:
        sys.exit(f'{rel}: sha256 {got} is not the pinned {pin}')


def t(x):
    x = Q5.of(x)
    return [x.a, x.b, x.d]


def quat(q):
    return [t(c) for c in q]


def q(*cs):
    return tuple(Q5.of(c) for c in cs)


def matrix(l, r, s=Q5(1)):
    cols = [trees.apply_lr(l, r, tuple(Q5(v) for v in trees._E[b])) for b in range(4)]
    return [[cols[b][a] * s for b in range(4)] for a in range(4)]


ONE = q(1, 0, 0, 0)
I = q(0, 1, 0, 0)
ONE_I = q(1, 1, 0, 0)
ONE_J = q(1, 0, 1, 0)
ONE_K = q(1, 0, 0, 1)
PHI = trees.PHI
SQRT5 = Q5(0, 1, 1)


def chain(*parts):
    p = ONE
    for x in parts:
        p = trees.qmul(p, x)
    return p


U = trees.UNITS


def deep(n, offset):
    """A product u_1 (1 + i) u_2 (1 + i) ... of n non-backtracking steps: its distance is n."""
    x = ONE
    for step in range(1, n + 1):
        for k in range(len(U)):
            y = trees.qmul(trees.qmul(x, U[(offset + k) % len(U)]), ONE_I)
            if trees.dist(y) == step:
                x = y
                break
        else:
            raise ValueError(f'no step {step}')
    return x


def with_branch(x, b):
    """A unit times x whose geodesic leaves v0 through branch b (a unit fixes v0 and permutes its neighbours)."""
    for v in U:
        y = trees.qmul(v, x)
        if trees.branch(y) == b:
            return y
    raise ValueError(f'no branch {b}')


cases = [
    ('identity', ONE, ONE, Q5(1)),
    ('unit icosians u7, u100', U[7], U[100], Q5(1)),
    ('unit icosians u33, u61', U[33], U[61], Q5(1)),
    ('left 1 + i', ONE_I, ONE, Q5(1)),
    ('right 1 + j', ONE, ONE_J, Q5(1)),
    ('1 + i and 1 + k', ONE_I, ONE_K, Q5(1)),
    ('left (1 + i)(1 + j)', chain(ONE_I, ONE_J), ONE, Q5(1)),
    ('left (1 + i)(1 + j)(1 + k)', chain(ONE_I, ONE_J, ONE_K), U[7], Q5(1)),
    ('left (1 + i)(1 + j)(1 + i)(1 + j)', chain(ONE_I, ONE_J, ONE_I, ONE_J), ONE_K, Q5(1)),
    ('six steps', chain(ONE_I, ONE_J, ONE_I, ONE_J, ONE_I, ONE_J), chain(ONE_K, ONE_I, ONE_K), Q5(1)),
    ('Q(sqrt 5) coefficients', q(PHI, 1, 0, 0), q(2, SQRT5, 0, 1), Q5(1)),
    ('integer quaternions', q(1, 2, 3, 4), q(2, 0, 1, 1), Q5(1)),
    ('scaled by 3/2 + sqrt 5', chain(ONE_I, ONE_J), q(PHI, 1, 0, 0), Q5(3, 2, 2)),
    ('scaled by 1/8', ONE_I, U[33], Q5(1, 0, 8)),
    ('depth 2 on both sides, branches 1 and 2', with_branch(deep(2, 5), 1), with_branch(deep(2, 40), 2), Q5(1)),
    ('depth 4 on the left, branch 3', with_branch(deep(4, 11), 3), ONE, Q5(1)),
    ('depth 3 on the right, branch 4', U[2], with_branch(deep(3, 70), 4), Q5(1)),
    ('depths 6 and 5, branches 4 and 1', with_branch(deep(6, 17), 4), with_branch(deep(5, 90), 1), Q5(1)),
]

poses = []
quats = {}
for name, l0, r0, s in cases:
    m = matrix(l0, r0, s)
    l, r = trees.factors(m)
    assert trees.reconstructs(m, l, r)
    row = {'name': name, 'm': [[t(x) for x in rw] for rw in m], 'l': quat(l), 'r': quat(r), 'reconstructs': True,
           'dist_l': trees.dist(l), 'dist_r': trees.dist(r), 'branch_l': trees.branch(l), 'branch_r': trees.branch(r)}
    poses.append(row)
    for x in (l, r):
        quats[tuple(map(tuple, quat(x)))] = x

# A wrong right factor: the matrix of (1 + i, 1 + k) with r = 1 + j does not reconstruct.
m_bad = matrix(ONE_I, ONE_K)
l_bad, r_bad = trees.factors(m_bad)
assert not trees.reconstructs(m_bad, l_bad, ONE_J)
not_reconstructed = [{'name': 'right factor 1 + j for the matrix of (1 + i, 1 + k)',
                      'm': [[t(x) for x in rw] for rw in m_bad],
                      'l': quat(l_bad), 'r': quat(ONE_J), 'reconstructs': False}]

quaternions = []
for key, x in quats.items():
    quaternions.append({'x': quat(x), 'kval': trees.kval(x), 'dist': trees.dist(x), 'branch': trees.branch(x),
                        'norm_v2': trees.v2(trees.qnorm(x))})

v2_values = [Q5(1), Q5(2), Q5(4), Q5(1, 0, 2), Q5(3, 0, 8), Q5(-12), SQRT5, PHI, Q5(1, 1, 1), Q5(2, 2, 1),
             Q5(3, 1, 1), Q5(5, 1, 4), Q5(7, 3, 16), Q5(1, 0, 1024), Q5(3 * 2 ** 40), Q5(0, 6, 5)]
v2 = [{'x': t(x), 'v2': trees.v2(x)} for x in v2_values]

punits = list(trees.PUNITS)
index = {p: i for i, p in enumerate(punits)}
out = {
    'reference': PINS,
    'command': 'python rust/c600-library/tests/fixtures/trees_reference.py . > '
               'rust/c600-library/tests/fixtures/trees-reference.json',
    'constants': {
        'binv': [[t(c) for c in row] for row in trees._BINV],
        'punits': [quat(p) for p in punits],
        'tets': [sorted(index[u] for u in S) for S in trees.TETS],
    },
    'v2': v2,
    'v2_refused': [{'x': [0, 0, 1], 'code': 'LG4:input.pointwise_failed'}],
    'quaternions': quaternions,
    'poses': poses,
    'not_reconstructed': not_reconstructed,
}
sys.stdout.buffer.write((json.dumps(out, indent=1) + '\n').encode('utf-8'))
