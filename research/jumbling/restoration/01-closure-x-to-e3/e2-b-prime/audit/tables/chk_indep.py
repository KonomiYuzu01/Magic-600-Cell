"""Independent chain check of segment 3 (E2 -> E3) from the raw pose tables. Audit label 'tables'.

No simulator code is imported. Inputs: the raw JSON pose tables, and assets/model.npz of the research
worktree for the pole normals (converted to exact (p + q sqrt5)/2 here) and the piece signatures (mask CSR).

- sha256 of the E2 copies, E1 copies, X and E3.
- Every pose entry normalised exactly here; matrices compared as exact values.
- Table digest recomputed here from the canonical-digest definition (group pieces by exact pose, skip the
  identity, sha256 over sorted (json key, int32 member bytes)).
- K+ built here: exact BFS closure of two poses that are pole-preserving rotations (checked exactly),
  must have 7,200 elements; every distinct pose of every table is tested three ways: closure membership,
  and the geometric criterion (exact SO(4) and maps the 600 poles onto poles).
- Chamber of a lattice piece: the piece whose home signature is the image of its signature under the pole
  permutation of its pose (pole permutation computed here from the exact matrix).
- Sets: off sets, changed sets between tables, 17- and 87-blocks and their overlaps, holes.

Writes chk_indep.json (and chk_indep_sets.npz) next to this file.
"""
import hashlib
import json
import sys
import time
from fractions import Fraction
from itertools import permutations
from math import gcd, sqrt
from pathlib import Path

import numpy as np

T0 = time.time()
SP = Path('<work>')
HERE = SP / 'v26/B4/audit/tables'
MODEL = SP / 'wt-e3/assets/model.npz'
NP = 177120

PATHS = {
    'X': SP / 'pkg26/Claude-E-closure-X1-20261012/input/X-table.json',
    'E1': SP / 'v26/E2/c0/E1-table.json',
    'E2': SP / 'v26/B4/E2-table.json',
    'E3': SP / 'v26/B4/c3/E3-table.json',
}
E2_COPIES = {
    'pkg27/D87/E2-table.json': SP / 'pkg27/Claude-E-closure-E1E2-20261012/D87/E2-table.json',
    'v26/E2/c0/E2-table.json': SP / 'v26/E2/c0/E2-table.json',
    'v26/B3/E2-table.json': SP / 'v26/B3/E2-table.json',
    'v26/B4/E2-table.json': SP / 'v26/B4/E2-table.json',
}
E1_COPIES = {
    'v26/E2/c0/E1-table.json': SP / 'v26/E2/c0/E1-table.json',
    'pkg27/input/E1-table.json': SP / 'pkg27/Claude-E-closure-E1E2-20261012/input/E1-table.json',
    'pkg26/D17/end-table.json': SP / 'pkg26/Claude-E-closure-X1-20261012/D17/end-table.json',
}
BPRIME = [16245, 16249, 32088]


def log(**kw):
    kw['t'] = round(time.time() - T0, 1)
    print(json.dumps(kw), flush=True)


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


# ------------------------------------------------------------------ exact entries and matrices
def norm_entry(e):
    if not (isinstance(e, list) and len(e) == 3 and all(type(x) is int for x in e)):
        raise ValueError(f'bad entry {e!r}')
    a, b, d = e
    if d == 0:
        raise ValueError('zero denominator')
    if d < 0:
        a, b, d = -a, -b, -d
    g = gcd(gcd(a, b), d)
    if g > 1:
        a, b, d = a // g, b // g, d // g
    return (a, b, d)


def canon(m):
    """Canonical key of an exact 4x4 matrix: 16 lowest-terms (a, b, d) triples, row-major, flattened."""
    if not (isinstance(m, list) and len(m) == 4 and all(isinstance(r, list) and len(r) == 4 for r in m)):
        raise ValueError('matrix must be 4 x 4')
    out = []
    lowest = True
    for r in m:
        for e in r:
            t = norm_entry(e)
            if list(t) != e:
                lowest = False
            out.extend(t)
    return tuple(out), lowest


def frac_key(key):
    """Second canonical form: pairs of Fractions (a/d, b/d)."""
    return tuple((Fraction(key[3 * i], key[3 * i + 2]), Fraction(key[3 * i + 1], key[3 * i + 2])) for i in range(16))


def int_form(key):
    """(A, B, D): matrix = (A + B sqrt5) / D with D = lcm of the entry denominators (then gcd(A, B, D) = 1)."""
    ds = [key[3 * i + 2] for i in range(16)]
    D = 1
    for d in ds:
        D = D * d // gcd(D, d)
    A = np.array([key[3 * i] * (D // key[3 * i + 2]) for i in range(16)], np.int64).reshape(4, 4)
    B = np.array([key[3 * i + 1] * (D // key[3 * i + 2]) for i in range(16)], np.int64).reshape(4, 4)
    return A, B, D


def normalise_int(A, B, D):
    g = int(np.gcd.reduce(np.concatenate([A.ravel(), B.ravel(), [D]])))
    if D < 0:
        g = -g
    if g != 1:
        A, B, D = A // g, B // g, D // g
    return A, B, int(D)


def int_key(A, B, D):
    return A.tobytes() + B.tobytes() + int(D).to_bytes(8, 'little', signed=True)


def int_to_canon(A, B, D):
    out = []
    for i in range(4):
        for j in range(4):
            out.extend(norm_entry([int(A[i, j]), int(B[i, j]), int(D)]))
    return tuple(out)


def mul(x, y):
    A1, B1, D1 = x
    A2, B2, D2 = y
    return normalise_int(A1 @ A2 + 5 * (B1 @ B2), A1 @ B2 + B1 @ A2, D1 * D2)


def zmul(x, y):
    return (x[0] * y[0] + 5 * x[1] * y[1], x[0] * y[1] + x[1] * y[0])


def parity(p):
    s, p = 1, list(p)
    for i in range(len(p)):
        while p[i] != i:
            j = p[i]
            p[i], p[j] = p[j], p[i]
            s = -s
    return s


PERMS4 = [(p, parity(p)) for p in permutations(range(4))]


def det_is_one(A, B, D):
    tot = [0, 0]
    for p, s in PERMS4:
        t = (1, 0)
        for i in range(4):
            t = zmul(t, (int(A[i, p[i]]), int(B[i, p[i]])))
        tot[0] += s * t[0]
        tot[1] += s * t[1]
    return tot[0] == D ** 4 and tot[1] == 0


def orthogonal(A, B, D):
    return (np.array_equal(A.T @ A + 5 * (B.T @ B), D * D * np.eye(4, dtype=np.int64))
            and not np.any(A.T @ B + B.T @ A))


# ------------------------------------------------------------------ poles and signatures
def exact_half(x):
    """The unique (p, q) with |q| <= 12 and (p + q sqrt5)/2 within 1e-9 of x."""
    s5 = sqrt(5)
    hits = []
    for q in range(-12, 13):
        p = round(2 * x - q * s5)
        if abs((p + q * s5) / 2 - x) < 1e-9:
            hits.append((p, q))
    if len(hits) != 1:
        raise ValueError(f'no unique value for {x}: {hits}')
    return hits[0]


def load_model():
    m = np.load(MODEL)
    nf = m['normals']
    N1 = np.zeros((600, 4), np.int64)
    N2 = np.zeros((600, 4), np.int64)
    for i in range(600):
        for j in range(4):
            N1[i, j], N2[i, j] = exact_half(float(nf[i, j]))
    # |n|^2 = sum ((N1 + N2 r)/2)^2 = (sum(N1^2 + 5 N2^2) + 2 sum(N1 N2) r) / 4
    nn = {(int((N1[i] ** 2 + 5 * N2[i] ** 2).sum()), int(2 * (N1[i] * N2[i]).sum())) for i in range(600)}
    pole_key = {(tuple(N1[i].tolist()), tuple(N2[i].tolist())): i for i in range(600)}
    assert len(pole_key) == 600
    mo = m['mask_offsets'].astype(np.int64)
    mv = m['mask_values'].astype(np.int64)
    return N1, N2, nn, pole_key, mo, mv, sha(MODEL)


def pole_perm(ABD, N1, N2, pole_key):
    """Pole permutation of an exact matrix (perm[x] = y with M n_x = n_y), or None if some image is not a pole."""
    A, B, D = ABD
    Y1 = N1 @ A.T + 5 * (N2 @ B.T)
    Y2 = N2 @ A.T + N1 @ B.T
    if np.any(Y1 % D) or np.any(Y2 % D):
        return None
    Y1 //= D
    Y2 //= D
    out = np.empty(600, np.int64)
    r1, r2 = Y1.tolist(), Y2.tolist()
    for x in range(600):
        y = pole_key.get((tuple(r1[x]), tuple(r2[x])))
        if y is None:
            return None
        out[x] = y
    if len(set(out.tolist())) != 600:
        return None
    return out


# ------------------------------------------------------------------ tables
class Registry:
    def __init__(self):
        self.key_of = {}
        self.keys = []

    def gid(self, key):
        g = self.key_of.get(key)
        if g is None:
            g = len(self.keys)
            self.key_of[key] = g
            self.keys.append(key)
        return g


def load_table(path, reg):
    doc = json.loads(Path(path).read_text())
    info = {'top_keys': sorted(doc), 'poses_listed': len(doc['poses']), 'pieces': len(doc['pose_id'])}
    if len(doc['pose_id']) != NP:
        raise ValueError('pose_id length')
    pid = np.array(doc['pose_id'], np.int64)
    if pid.min() < 0 or pid.max() >= len(doc['poses']):
        raise ValueError('pose_id out of range')
    if not all(type(x) is int for x in doc['pose_id']):
        raise ValueError('non-integer pose id')
    gids = []
    lowest_all = True
    for m in doc['poses']:
        k, lowest = canon(m)
        lowest_all &= lowest
        gids.append(reg.gid(k))
    gids = np.array(gids, np.int64)
    info['entries_in_lowest_terms'] = lowest_all
    info['distinct_listed_matrices'] = int(len(np.unique(gids)))
    info['duplicate_listed_matrices'] = int(len(gids) - len(np.unique(gids)))
    piece = gids[pid]
    info['distinct_used_matrices'] = int(len(np.unique(piece)))
    info['unused_listed_entries'] = int(len(doc['poses']) - len(np.unique(pid)))
    return doc.get('digest'), piece, info


def matrix_json_of(key):
    return [[[key[3 * (4 * i + j)], key[3 * (4 * i + j) + 1], key[3 * (4 * i + j) + 2]] for j in range(4)]
            for i in range(4)]


def recompute_digest(piece, reg, identity_gid):
    order = np.argsort(piece, kind='stable')
    sp = piece[order]
    cuts = np.flatnonzero(np.diff(sp)) + 1
    items = []
    for seg in np.split(np.arange(NP), cuts):
        g = int(sp[seg[0]])
        if g == identity_gid:
            continue
        members = np.sort(order[seg]).astype('<i4')
        items.append((json.dumps(matrix_json_of(reg.keys[g])), members.tobytes()))
    items.sort()
    h = hashlib.sha256()
    for key, b in items:
        h.update(key.encode())
        h.update(b)
    return h.hexdigest()


def main():
    res = {'inputs': {k: str(v.relative_to(SP)) for k, v in PATHS.items()}}
    # 1. hashes
    res['sha256'] = {'E2_copies': {k: sha(v) for k, v in E2_COPIES.items()},
                     'E1_copies': {k: sha(v) for k, v in E1_COPIES.items()},
                     'X': sha(PATHS['X']), 'E3': sha(PATHS['E3'])}
    res['E2_copies_identical'] = len(set(res['sha256']['E2_copies'].values())) == 1
    res['E1_copies_identical'] = len(set(res['sha256']['E1_copies'].values())) == 1
    log(step='hashes', **{k: v for k, v in res.items() if k.endswith('identical')})

    # 2. model
    N1, N2, nn, pole_key, mo, mv, model_sha = load_model()
    res['model'] = {'sha256': model_sha, 'pole_norms_x4': sorted(nn)}
    sig_home = {}
    for p in range(NP):
        sig_home[np.sort(mv[mo[p]:mo[p + 1]]).astype(np.int16).tobytes()] = p
    res['model']['distinct_signatures'] = len(sig_home)
    log(step='model', distinct_signatures=len(sig_home), norms=sorted(nn))

    # 3. tables
    reg = Registry()
    ident = tuple(x for i in range(4) for j in range(4) for x in ((1, 0, 1) if i == j else (0, 0, 1)))
    identity_gid = reg.gid(ident)
    tabs = {}
    res['tables'] = {}
    for name, path in PATHS.items():
        dig, piece, info = load_table(path, reg)
        info['digest_field'] = dig
        info['digest_recomputed'] = recompute_digest(piece, reg, identity_gid)
        info['digest_field_matches_content'] = info['digest_recomputed'] == dig
        tabs[name] = piece
        res['tables'][name] = info
        log(step='table', name=name, **{k: v for k, v in info.items() if k != 'top_keys'})
    # parse the other E2 copies too (bytes are compared above; parse anyway)
    for name, path in E2_COPIES.items():
        dig, piece, info = load_table(path, reg)
        res['tables'].setdefault('E2_copies_parsed', {})[name] = {
            'digest_field': dig, 'same_poses_as_E2': bool(np.array_equal(piece, tabs['E2']))}
    for name, path in E1_COPIES.items():
        dig, piece, info = load_table(path, reg)
        res['tables'].setdefault('E1_copies_parsed', {})[name] = {
            'digest_field': dig, 'same_poses_as_E1': bool(np.array_equal(piece, tabs['E1'])),
            'digest_recomputed': recompute_digest(piece, reg, identity_gid)}
    # second canonical form agrees (bijection)
    fk = {frac_key(k) for k in reg.keys}
    res['distinct_matrices_all_tables'] = len(reg.keys)
    res['fraction_form_bijective'] = len(fk) == len(reg.keys)
    log(step='registry', distinct=len(reg.keys), frac_ok=res['fraction_form_bijective'])

    # 4. geometric criterion for every distinct matrix
    ints = [int_form(k) for k in reg.keys]
    so4 = np.zeros(len(ints), bool)
    perm_of = {}
    for g, ABD in enumerate(ints):
        so4[g] = orthogonal(*ABD) and det_is_one(*ABD)
        if so4[g]:
            pp = pole_perm(ABD, N1, N2, pole_key)
            if pp is not None:
                perm_of[g] = pp
    geo = np.array([g in perm_of for g in range(len(ints))])
    res['all_matrices_rotations'] = bool(so4.all())
    res['non_rotation_matrices'] = int((~so4).sum())
    log(step='geometric', rotations=int(so4.sum()), pole_preserving=int(geo.sum()))

    # 5. K+ closure from two pole-preserving rotations taken from E3's poses
    e3_used = sorted(set(np.unique(tabs['E3']).tolist()) - {identity_gid}, key=lambda g: reg.keys[g])
    cands = [g for g in e3_used if geo[g]]
    closure = None
    tried = []
    for i in range(len(cands)):
        for j in range(i + 1, len(cands)):
            gens = [ints[cands[i]], ints[cands[j]]]
            idm = (np.eye(4, dtype=np.int64), np.zeros((4, 4), np.int64), 1)
            seen = {int_key(*idm): idm}
            frontier = [idm]
            while frontier and len(seen) <= 20000:
                nxt = []
                for x in frontier:
                    for s in gens:
                        y = mul(s, x)
                        k = int_key(*y)
                        if k not in seen:
                            seen[k] = y
                            nxt.append(y)
                frontier = nxt
            tried.append((cands[i], cands[j], len(seen)))
            if len(seen) == 7200:
                closure = seen
                gen_pair = (cands[i], cands[j])
                break
        if closure is not None:
            break
        if len(tried) > 50:
            break
    res['closure'] = {'tried': [[int(a), int(b), n] for a, b, n in tried], 'order': len(closure) if closure else None}
    assert closure is not None, 'no pair generated 7,200 elements'
    res['closure']['generators'] = [matrix_json_of(reg.keys[g]) for g in gen_pair]
    res['closure']['generators_pole_preserving_rotations'] = bool(geo[gen_pair[0]] and geo[gen_pair[1]])
    # every closure element: rotation, pole preserving (inherited; checked anyway on all)
    cl_ok_rot = all(orthogonal(*v) for v in closure.values())
    cl_ok_pole = all(pole_perm(v, N1, N2, pole_key) is not None for v in closure.values())
    res['closure']['all_orthogonal'] = cl_ok_rot
    res['closure']['all_pole_preserving'] = cl_ok_pole
    closure_canon = {int_to_canon(*v) for v in closure.values()}
    inK = np.array([k in closure_canon for k in reg.keys])
    res['membership_agree_closure_vs_geometric'] = bool(np.array_equal(inK, geo))
    res['membership_disagreements'] = np.flatnonzero(inK != geo).tolist()
    log(step='closure', order=len(closure), rot=cl_ok_rot, pole=cl_ok_pole,
        agree=res['membership_agree_closure_vs_geometric'])
    # save closure keys for the sim comparison
    with open(HERE / 'closure_keys.json', 'w') as f:
        json.dump(sorted(list(k) for k in closure_canon), f)

    # 6. lattice flags, off sets, chambers
    lat = {n: inK[tabs[n]] for n in tabs}
    off = {n: np.flatnonzero(~lat[n]) for n in tabs}
    res['off_counts'] = {n: int(len(off[n])) for n in tabs}
    res['E2_off_set'] = off['E2'].tolist()
    res['E2_off_is_Bprime'] = off['E2'].tolist() == BPRIME
    bp = tabs['E2'][BPRIME]
    res['Bprime_one_pose_at_E2'] = bool(len(set(bp.tolist())) == 1)
    res['Bprime_pose_E2'] = matrix_json_of(reg.keys[int(bp[0])])
    res['Bprime_pose_E2_in_Kplus'] = bool(inK[int(bp[0])])
    res['Bprime_pose_E2_is_rotation'] = bool(so4[int(bp[0])])
    res['Bprime_pose_E2_pole_preserving'] = bool(geo[int(bp[0])])
    res['E3_off'] = int(len(off['E3']))
    res['Bprime_poses_E3'] = {str(p): matrix_json_of(reg.keys[int(tabs['E3'][p])]) for p in BPRIME}
    res['Bprime_E3_in_Kplus'] = [bool(inK[int(tabs['E3'][p])]) for p in BPRIME]
    res['Bprime_E3_distinct_poses'] = len(set(tabs['E3'][BPRIME].tolist()))

    def chambers(name):
        piece = tabs[name]
        ch = np.full(NP, -1, np.int64)
        miss = 0
        for p in np.flatnonzero(lat[name]).tolist():
            pp = perm_of.get(int(piece[p]))
            if pp is None:
                miss += 1
                continue
            img = np.sort(pp[mv[mo[p]:mo[p + 1]]]).astype(np.int16).tobytes()
            q = sig_home.get(img)
            if q is None:
                miss += 1
            else:
                ch[p] = q
        return ch, miss

    chs = {}
    res['chambers'] = {}
    for name in tabs:
        ch, miss = chambers(name)
        chs[name] = ch
        occ = ch[ch >= 0]
        u = np.unique(occ)
        holes = np.setdiff1d(np.arange(NP), u)
        res['chambers'][name] = {'lattice': int(lat[name].sum()), 'unmatched_images': miss,
                                 'occupied': int(len(u)), 'collisions': int(len(occ) - len(u)),
                                 'holes': holes.tolist() if len(holes) <= 50 else int(len(holes)),
                                 'hole_count': int(len(holes))}
        log(step='chambers', name=name, **{k: (v if not isinstance(v, list) else v[:10]) for k, v in
                                           res['chambers'][name].items()})

    # 7. changes between tables
    def changed(a, b):
        return np.flatnonzero(tabs[a] != tabs[b])
    ch_sets = {f'{a}_to_{b}': changed(a, b) for a, b in
               (('E2', 'E3'), ('E1', 'E3'), ('X', 'E3'), ('X', 'E1'), ('E1', 'E2'), ('X', 'E2'))}
    res['changed_counts'] = {k: int(len(v)) for k, v in ch_sets.items()}
    c23 = ch_sets['E2_to_E3']
    moved3 = c23[lat['E2'][c23]]
    res['E2_E3'] = {
        'changed': int(len(c23)),
        'changed_includes_Bprime': bool(np.isin(BPRIME, c23).all()),
        'changed_minus_Bprime_equals_lattice_moved': bool(np.array_equal(np.setdiff1d(c23, BPRIME), moved3)),
        'lattice_moved': int(len(moved3)),
        'lattice_moved_all_on_K_at_E3': bool(lat['E3'][moved3].all()),
        'changed_all_on_K_at_E3': bool(lat['E3'][c23].all()),
        'E2_lattice': int(lat['E2'].sum()),
        'E2_lattice_same_exact_pose': int(lat['E2'].sum() - len(moved3)),
        'E2_lattice_on_K_at_E3': int((lat['E2'] & lat['E3']).sum()),
        'collateral_E2_lattice_off_at_E3': int((lat['E2'] & ~lat['E3']).sum()),
        'lattice_moved_changed_chamber': int((chs['E2'][moved3] != chs['E3'][moved3]).sum()),
        'lattice_moved_same_chamber_ids': moved3[chs['E2'][moved3] == chs['E3'][moved3]].tolist(),
    }
    same_ch = moved3[chs['E2'][moved3] == chs['E3'][moved3]]
    res['E2_E3']['same_chamber_pieces_detail'] = [
        {'piece': int(p), 'chamber': int(chs['E2'][p]), 'home_chamber': int(p) == int(chs['E2'][p]),
         'signature_size': int(mo[p + 1] - mo[p])} for p in same_ch.tolist()]
    # blocks
    offX, offE1, offE2 = set(off['X'].tolist()), set(off['E1'].tolist()), set(off['E2'].tolist())
    b17 = sorted(offX - offE1)
    b87 = sorted(offE1 - set(BPRIME))
    m3 = set(moved3.tolist())
    res['blocks'] = {
        'E1_off_subset_X_off': offE1 <= offX, 'Bprime_subset_E1_off': set(BPRIME) <= offE1,
        'E2_off_subset_E1_off': offE2 <= offE1,
        'block17_size': len(b17), 'block87_size': len(b87),
        'block87_in_lattice_moved': len(set(b87) & m3), 'block17_in_lattice_moved': len(set(b17) & m3),
        'Bprime_in_lattice_moved': len(set(BPRIME) & m3),
        'block87_ids_in_lattice_moved': sorted(set(b87) & m3), 'block17_ids_in_lattice_moved': sorted(set(b17) & m3),
    }
    c13, cX3 = set(ch_sets['E1_to_E3'].tolist()), set(ch_sets['X_to_E3'].tolist())
    res['decomposition'] = {
        'E1_to_E3_equals_E1off_union_moved3': c13 == (offE1 | m3),
        'X_to_E3_equals_Xoff_union_moved3': cX3 == (offX | m3),
        'E1_to_E3_formula': len(offE1) + len(m3) - len(offE1 & m3),
        'X_to_E3_formula': len(offX) + len(m3) - len(offX & m3),
        'E1_to_E3_extra_not_in_union': sorted(c13 - (offE1 | m3))[:20],
        'E1_to_E3_union_not_changed': sorted((offE1 | m3) - c13)[:20],
        'X_to_E3_extra_not_in_union': sorted(cX3 - (offX | m3))[:20],
        'X_to_E3_union_not_changed': sorted((offX | m3) - cX3)[:20],
        'X_to_E1_equals_block17': set(ch_sets['X_to_E1'].tolist()) == set(b17),
        'E1_to_E2_equals_block87': set(ch_sets['E1_to_E2'].tolist()) == set(b87),
        'X_lattice_moved_in_seg1': sorted(set(ch_sets['X_to_E1'].tolist()) - offX)[:20],
        'E1_lattice_moved_in_seg2': sorted(set(ch_sets['E1_to_E2'].tolist()) - offE1)[:20],
    }
    # moved3 pieces that return to an older exact pose (would break the formula)
    res['decomposition']['moved3_back_to_E1_pose'] = int((tabs['E1'][moved3] == tabs['E3'][moved3]).sum())
    res['decomposition']['moved3_back_to_X_pose'] = int((tabs['X'][moved3] == tabs['E3'][moved3]).sum())
    np.savez(HERE / 'chk_indep_sets.npz', moved3=moved3, c23=c23, offX=off['X'], offE1=off['E1'], offE2=off['E2'],
             chE2=chs['E2'], chE3=chs['E3'])
    res['seconds'] = round(time.time() - T0, 1)
    (HERE / 'chk_indep.json').write_text(json.dumps(res, indent=1) + '\n')
    log(step='done', seconds=res['seconds'])


if __name__ == '__main__':
    main()
