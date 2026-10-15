"""Independent chain check X -> E1 -> E2 from the raw pose tables (audit label "tables").

Own code: exact parsing and normalisation of the (a + b sqrt5)/d entries, own Q(sqrt5) arithmetic for the
orthogonality/determinant test, own pole-set membership test (integer numpy), own digest recomputation.
From the sim package only: the 600 exact poles (data.N) and the 7200 K+ matrices (kplus.matrix), both
re-verified here.

Usage: python3 -I chain_tables.py <work dir> <out json>
"""
import hashlib
import json
import sys
from math import gcd
from pathlib import Path

import numpy as np

SP = Path(sys.argv[1])
OUT = Path(sys.argv[2])
JUMB = SP / 'wt-e3/research/jumbling'
PKG = SP / 'pkg26/Claude-E-closure-X1-20261012'
C0 = SP / 'v26/E2/c0'
FILES = {
    'seg1_end': PKG / 'D17/end-table.json',
    'v26_E1': SP / 'v26/E1-table.json',
    'c0_E1': C0 / 'E1-table.json',
    'X': PKG / 'input/X-table.json',
    'E2': C0 / 'E2-table.json',
}
BPRIME = [16245, 16249, 32088]
res = {}


def norm(e):
    a, b, d = (int(x) for x in e)
    if d == 0:
        raise ValueError('zero denominator')
    if d < 0:
        a, b, d = -a, -b, -d
    g = gcd(gcd(a, b), d)
    if g > 1:
        a, b, d = a // g, b // g, d // g
    return (a, b, d)


def mkey(m):
    assert len(m) == 4 and all(len(r) == 4 for r in m)
    return tuple(tuple(norm(e) for e in r) for r in m)


# ---- exact Q(sqrt5) arithmetic with Fractions -------------------------------------------------------------
from fractions import Fraction as F  # noqa: E402


def q(e):
    return (F(e[0], e[2]), F(e[1], e[2]))


def qmul(x, y):
    return (x[0] * y[0] + 5 * x[1] * y[1], x[0] * y[1] + x[1] * y[0])


def qadd(x, y):
    return (x[0] + y[0], x[1] + y[1])


def qmat(k):
    return [[q(e) for e in r] for r in k]


def is_rotation(k):
    m = qmat(k)
    Z = (F(0), F(0))
    for i in range(4):
        for j in range(4):
            s = Z
            for r in range(4):
                s = qadd(s, qmul(m[r][i], m[r][j]))
            if s != ((F(1), F(0)) if i == j else Z):
                return False
    # determinant by cofactor expansion
    def det(a):
        n = len(a)
        if n == 1:
            return a[0][0]
        s = Z
        for j in range(n):
            minor = [row[:j] + row[j + 1:] for row in a[1:]]
            t = qmul(a[0][j], det(minor))
            s = qadd(s, t) if j % 2 == 0 else qadd(s, (-t[0], -t[1]))
        return s
    return det(m) == (F(1), F(0))


# ---- sha256 of files ----------------------------------------------------------------------------------------
sha = {k: hashlib.sha256(p.read_bytes()).hexdigest() for k, p in FILES.items()}
res['sha256'] = sha

# ---- parse tables -------------------------------------------------------------------------------------------
tables = {}
for k in ['seg1_end', 'v26_E1', 'c0_E1', 'X', 'E2']:
    d = json.loads(FILES[k].read_text())
    tables[k] = d
res['n_pieces'] = {k: len(t['pose_id']) for k, t in tables.items()}
res['n_poses'] = {k: len(t['poses']) for k, t in tables.items()}
res['digest_field'] = {k: t['digest'] for k, t in tables.items()}

# global matrix-key interning
KEYS = {}
KLIST = []


def intern(k):
    i = KEYS.get(k)
    if i is None:
        i = len(KLIST)
        KEYS[k] = i
        KLIST.append(k)
    return i


piece_key = {}
dup_poses = {}
for k, t in tables.items():
    ids = np.array([intern(mkey(m)) for m in t['poses']], np.int64)
    dup_poses[k] = int(len(ids) - len(set(ids.tolist())))
    pid = np.asarray(t['pose_id'], np.int64)
    assert pid.min() >= 0 and pid.max() < len(ids)
    piece_key[k] = ids[pid]
res['duplicate_pose_entries_in_table'] = dup_poses

# ---- K+ from the sim package, re-verified ---------------------------------------------------------------------
sys.path.insert(0, str(JUMB))
import sim  # noqa: E402

ctx = sim.get_context()
kp, data = ctx.kplus, ctx.data
kplus_keys = set()
for i in range(7200):
    m = kp.matrix(i)
    kplus_keys.add(tuple(tuple(norm((x.a, x.b, x.d)) for x in r) for r in m))
res['kplus_distinct_matrices'] = len(kplus_keys)
# poles in integer form n = (N1 + N2 sqrt5)/2
N1 = np.zeros((600, 4), np.int64)
N2 = np.zeros((600, 4), np.int64)
for i, row in enumerate(data.N):
    for c, x in enumerate(row):
        assert (2 * x.a) % x.d == 0 and (2 * x.b) % x.d == 0
        N1[i, c] = 2 * x.a // x.d
        N2[i, c] = 2 * x.b // x.d
pole_set = {(tuple(N1[i]), tuple(N2[i])) for i in range(600)}
assert len(pole_set) == 600


def preserves_poles(k):
    D = 1
    for r in k:
        for e in r:
            D = D * e[2] // gcd(D, e[2])
    g1 = np.array([[e[0] * (D // e[2]) for e in r] for r in k], np.int64)
    g2 = np.array([[e[1] * (D // e[2]) for e in r] for r in k], np.int64)
    r1 = N1 @ g1.T + 5 * (N2 @ g2.T)
    r2 = N2 @ g1.T + N1 @ g2.T
    if np.any(r1 % D) or np.any(r2 % D):
        return False
    r1 //= D
    r2 //= D
    imgs = {(tuple(r1[i]), tuple(r2[i])) for i in range(600)}
    return imgs == pole_set


# verify the K+ list: all rotations preserving poles, closed under a sample of products
kp_list = sorted(kplus_keys)
res['kplus_all_rotations'] = all(is_rotation(k) for k in kp_list)
res['kplus_all_preserve_poles'] = all(preserves_poles(k) for k in kp_list)
rng = np.random.default_rng(12)


def kmul(a, b):
    A, B = qmat(a), qmat(b)
    out = []
    for i in range(4):
        row = []
        for j in range(4):
            s = (F(0), F(0))
            for r in range(4):
                s = qadd(s, qmul(A[i][r], B[r][j]))
            # back to (a,b,d)
            d = s[0].denominator * s[1].denominator // gcd(s[0].denominator, s[1].denominator)
            row.append(norm((int(s[0] * d), int(s[1] * d), d)))
        out.append(tuple(row))
    return tuple(out)


samp = rng.integers(0, 7200, size=(3000, 2))
res['kplus_closed_on_3000_random_products'] = all(kmul(kp_list[a], kp_list[b]) in kplus_keys for a, b in samp)

# ---- lattice flags per table: two exact criteria ---------------------------------------------------------------
in_kp = np.array([k in kplus_keys for k in KLIST], bool)
crit2 = np.array([preserves_poles(k) and is_rotation(k) for k in KLIST], bool)
res['distinct_matrices_all_tables'] = len(KLIST)
res['membership_criteria_agree'] = bool(np.array_equal(in_kp, crit2))
res['all_table_matrices_are_rotations'] = all(is_rotation(k) for k in KLIST)
off = {k: ~in_kp[piece_key[k]] for k in tables}
offset = {k: set(np.flatnonzero(off[k]).tolist()) for k in tables}
res['off_counts'] = {k: len(v) for k, v in offset.items()}

# ---- the chain ------------------------------------------------------------------------------------------------------
X, E1, E2 = piece_key['X'], piece_key['c0_E1'], piece_key['E2']
res['E1_copies_entrywise_equal'] = bool(np.array_equal(piece_key['seg1_end'], piece_key['c0_E1'])
                                        and np.array_equal(piece_key['v26_E1'], piece_key['c0_E1']))
oX, oE1, oE2 = offset['X'], offset['c0_E1'], offset['E2']
B = set(BPRIME)
res['E2_off_set'] = sorted(oE2)
res['E2_off_is_Bprime'] = oE2 == B
d12 = set(np.flatnonzero(E1 != E2).tolist())
dX2 = set(np.flatnonzero(X != E2).tolist())
dX1 = set(np.flatnonzero(X != E1).tolist())
res['E1_E2_changed'] = len(d12)
res['E1_E2_changed_eq_offE1_minus_B'] = d12 == (oE1 - B)
res['X_E2_changed'] = len(dX2)
res['X_E2_changed_eq_offX_minus_B'] = dX2 == (oX - B)
res['X_E1_changed'] = len(dX1)
res['B_subset_offX_offE1'] = B <= oX and B <= oE1
res['Bprime_equal_X_E1_E2'] = all(X[p] == E1[p] == E2[p] for p in BPRIME)
res['Bprime_one_shared_pose'] = len({int(E2[p]) for p in BPRIME}) == 1
seventeen = oX - oE1
res['offE1_subset_offX'] = oE1 <= oX
res['block17_size'] = len(seventeen)
res['block17'] = sorted(seventeen)
res['X_E1_changed_eq_block17'] = dX1 == seventeen
res['block17_equal_E1_E2'] = all(E1[p] == E2[p] for p in seventeen)
latE1 = np.flatnonzero(~off['c0_E1'])
res['lattice_E1'] = int(len(latE1))
res['lattice_E1_on_lattice_E2'] = int((~off['E2'][latE1]).sum())
res['lattice_E1_same_matrix_E2'] = int((E1[latE1] == E2[latE1]).sum())
T = sorted(oE1 - B)
res['T_size'] = len(T)
res['T_on_lattice_E2'] = int((~off['E2'][T]).sum())
res['T_distinct_poses_E1'] = len({int(E1[p]) for p in T})
res['T_distinct_poses_E2'] = len({int(E2[p]) for p in T})
res['collateral_E1_lattice_off_at_E2'] = int(off['E2'][latE1].sum())
# B' matrices vs chain-check.json
cc = json.loads((C0 / 'chain-check.json').read_text())
res['Bprime_matrices_match_chain_check'] = all(mkey(cc['Bprime_poses_at_E2'][str(p)]) == KLIST[E2[p]] for p in BPRIME)
res['Bprime_matrix'] = [list(map(list, r)) for r in KLIST[E2[BPRIME[0]]]]
res['Bprime_matrix_is_rotation_not_Kplus'] = bool(is_rotation(KLIST[E2[BPRIME[0]]]) and not in_kp[E2[BPRIME[0]]])


# ---- digest recomputation (own code, per the definition in sim/state.py digest()) -----------------------------------
IDENT = tuple(tuple((1 if i == j else 0, 0, 1) for j in range(4)) for i in range(4))


def digest(pk):
    groups = {}
    for p in np.flatnonzero(np.array([KLIST[i] != IDENT for i in range(len(KLIST))])[pk]):
        groups.setdefault(int(pk[p]), []).append(int(p))
    items = []
    for kid, members in groups.items():
        key = json.dumps([[list(e) for e in r] for r in KLIST[kid]])
        items.append((key, np.sort(np.array(members, np.int32)).astype('<i4').tobytes()))
    items.sort()
    h = hashlib.sha256()
    for key, b in items:
        h.update(key.encode())
        h.update(b)
    return h.hexdigest()


res['digest_recomputed'] = {k: digest(piece_key[k]) for k in tables}
res['digest_recomputed_matches_field'] = {k: res['digest_recomputed'][k] == tables[k]['digest'] for k in tables}

# ---- holes via the sim region map (cross-check only) ----------------------------------------------------------------
reg = ctx.regions
kidx = {}
for i in range(7200):
    m = kp.matrix(i)
    kidx[tuple(tuple(norm((x.a, x.b, x.d)) for x in r) for r in m)] = i
kof = np.array([kidx.get(k, -1) for k in KLIST], np.int64)
holes = {}
for k in ['X', 'c0_E1', 'E2']:
    K = kof[piece_key[k]]
    lat = np.flatnonzero(K >= 0)
    ch = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    holes[k] = {'lattice': int(len(lat)), 'distinct_chambers': int(len(set(ch.tolist()))),
                'holes': int(177120 - len(set(ch.tolist())))}
    if k == 'E2':
        occ = np.zeros(177120, bool)
        occ[ch] = True
        holes[k]['hole_ids'] = np.flatnonzero(~occ).tolist()
res['holes'] = holes

res['chain_check_json'] = {k: v for k, v in cc.items() if k != 'Bprime_poses_at_E2'}
OUT.write_text(json.dumps(res, indent=1, default=int) + '\n')
print(json.dumps({k: v for k, v in res.items() if k not in ('block17', 'Bprime_matrix', 'chain_check_json')},
                 default=int))
