"""Minimal exact witness E0-E4 of research/jumbling/state-contract.md (section 6).

Run from the repository root (about 15-40 minutes on four cores):

    python research/jumbling/witness.py

Reads assets/model.npz read-only; writes research/jumbling/witness-results.json. All
classifications use exact Q(sqrt 5) arithmetic from exact.py. Floating point is used only
for the E1 comparison with the retained floating-point slot centres, and for display.
"""
import json
import math
import sys
import time
from fractions import Fraction
from multiprocessing import Pool
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from exact import (ONE, ZERO, Q5, double_description, dot, from_float, identity, matmul,  # noqa: E402
                   matvec, rank, solve, transpose)

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent / 'witness-results.json'
ALPHA = Q5(121, 0, 125)
M = np.load(ROOT / 'assets' / 'model.npz')
NF = M['normals']
N = [[from_float(float(x)) for x in row] for row in NF]
NN = [dot(n, n) for n in N]                      # |n|^2, also the facet offset
KAPPA = [ALPHA * x for x in NN]                  # cut offset alpha |n|^2
POLE = {tuple(n): i for i, n in enumerate(N)}
MO, MV = M['mask_offsets'], M['mask_values']
FO, FV = M['face_offsets'], M['face_values']


def signature(p):
    return frozenset(int(x) for x in MV[MO[p]:MO[p + 1]])


def hosts(p):
    return [int(x) for x in FV[FO[p]:FO[p + 1]]]


def h(e, x):
    """Signed offset of point x from the cut of pole e, as a fraction of the facet distance."""
    return dot(N[e], x) / NN[e] - ALPHA


def cap_region(c):
    """Exact vertices of U_c-closure intersected with the polytope, certified against all facets."""
    near = [e for e in range(600) if NF[e] @ NF[c] > 0.6 * (NF[c] @ NF[c])]
    cons = [([-x for x in N[c]], -KAPPA[c])] + [(N[e], NN[e]) for e in near]
    verts, _ = double_description(cons)
    for v in verts:  # certificate: every facet constraint holds, so the superset is exact
        for e in range(600):
            assert (dot(N[e], v) - NN[e]).sign() <= 0
    return verts


def implied(verts):
    """Poles whose cut every vertex satisfies on the outer side, and facets that every vertex
    satisfies: these constraints are implied for every subset of the region."""
    cut_out = {e for e in range(600) if all((dot(N[e], v) - KAPPA[e]).sign() <= 0 for v in verts)}
    return cut_out


def piece_constraints(p, cand_poles, cand_facets):
    sig = signature(p)
    cons, tags = [], []
    for e in sorted(sig):
        cons.append(([-x for x in N[e]], -KAPPA[e]))
        tags.append(('in', e))
    for e in cand_poles:
        if e not in sig:
            cons.append((N[e], KAPPA[e]))
            tags.append(('out', e))
    for e in cand_facets:
        cons.append((N[e], NN[e]))
        tags.append(('facet', e))
    return cons, tags


def affine_rank(points):
    if len(points) < 2:
        return 0
    base = points[0]
    return rank([[x - y for x, y in zip(q, base)] for q in points[1:]])


def build_region(args):
    p, cand_poles, cand_facets = args
    cons, tags = piece_constraints(p, cand_poles, cand_facets)
    verts, act = double_description(cons)
    full = affine_rank(verts) == 4
    host_dim = {}
    for e in cand_facets:
        on = [v for v in verts if (dot(N[e], v) - NN[e]).is_zero()]
        host_dim[e] = affine_rank(on) if on else -1
    return p, verts, full, host_dim


def to_json_q5(x):
    return [x.a, x.b, x.d]


# ---------------------------------------------------------------------------------------------
# exact symmetries

def matrix_from_pole_map(src, dst):
    """The linear map sending N[src[i]] to N[dst[i]] for four independent poles, exactly."""
    a = [N[i] for i in src]           # rows
    b = [N[i] for i in dst]
    # X a_i = b_i  <=>  A X^T = B  with A rows a_i
    xt = solve(a, b)
    return transpose(xt)


def is_rotation(r):
    return matmul(transpose(r), r) == identity(4) and det4(r) == ONE


def det4(m):
    # Laplace expansion along the first row (exact)
    def det3(a):
        return (a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
                - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
                + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]))
    s = ZERO
    for j in range(4):
        minor = [[m[i][k] for k in range(4) if k != j] for i in range(1, 4)]
        term = m[0][j] * det3(minor)
        s = s + term if j % 2 == 0 else s - term
    return s


def pole_perm(r):
    """Pole permutation of an exact matrix, or None if some pole is not mapped onto a pole."""
    out = []
    for n in N:
        img = tuple(matvec(r, n))
        if img not in POLE:
            return None
        out.append(POLE[img])
    return out


def basis_poles(c):
    """c and three face-neighbour poles: an independent set."""
    nb = sorted(range(600), key=lambda e: -float(NF[e] @ NF[c]))[1:5]
    return [c] + nb[:3]


def a4(c):
    """The 12 exact rotations fixing n_c that permute the poles."""
    src = basis_poles(c)
    nb = sorted(range(600), key=lambda e: -float(NF[e] @ NF[c]))[1:5]
    out = []
    import itertools
    for img in itertools.permutations(nb, 3):
        r = matrix_from_pole_map(src, [c] + list(img))
        if is_rotation(r) and pole_perm(r) is not None:
            out.append(r)
    assert len(out) == 12, len(out)
    return out


def generator_matrix(k):
    """Exact rotation of retained generator k, from its recorded pole permutation."""
    perm = [int(x) for x in M['rotperms'][k]]
    src = basis_poles(k // 2)
    r = matrix_from_pole_map(src, [perm[i] for i in src])
    assert is_rotation(r) and pole_perm(r) == perm
    return r


def witness_rotation(c, d, degrees=10, max_den=1000):
    """g = P + C (I - P) + t J fixing n_c and n_d, with C^2 + m t^2 = 1 exactly."""
    a, b = N[c], N[d]
    gram = [[dot(a, a), dot(a, b)], [dot(b, a), dot(b, b)]]
    m = gram[0][0] * gram[1][1] - gram[0][1] * gram[1][0]
    # projector P = A G^-1 A^T, with A = [a b] as columns
    ginv = solve(gram, identity(2))
    cols = [a, b]
    p = [[sum((cols[i][r] * ginv[i][j] * cols[j][s] for i in range(2) for j in range(2)), ZERO)
          for s in range(4)] for r in range(4)]
    # Hodge dual J_rs = sum_kl eps_rskl a_k b_l
    import itertools
    def eps(*idx):
        perm = list(idx)
        if len(set(perm)) < 4:
            return 0
        inv = sum(1 for i in range(4) for j in range(i + 1, 4) if perm[i] > perm[j])
        return -1 if inv % 2 else 1
    jm = [[sum((Q5(eps(r, s, k, l)) * a[k] * b[l] for k in range(4) for l in range(4)), ZERO)
           for s in range(4)] for r in range(4)]
    s_float = math.tan(math.radians(degrees) / 2) / math.sqrt(float(m))
    s = Fraction(s_float).limit_denominator(max_den)
    s = Q5(s.numerator, 0, s.denominator)
    cc = (ONE - m * s * s) / (ONE + m * s * s)
    tt = (Q5(2) * s) / (ONE + m * s * s)
    ident = identity(4)
    g = [[p[r][q] + cc * (ident[r][q] - p[r][q]) + tt * jm[r][q] for q in range(4)] for r in range(4)]
    assert is_rotation(g), 'witness rotation is not exactly orthogonal with det 1'
    assert matvec(g, a) == list(a) and matvec(g, b) == list(b)
    angle = math.degrees(math.acos(max(-1.0, min(1.0, float(cc)))))
    return g, angle, s


# ---------------------------------------------------------------------------------------------
# configurations and admissibility (state-contract.md section 3)

class Config:
    def __init__(self, regions, sigs):
        self.regions = regions          # piece -> exact home vertices (only pieces that may leave the lattice)
        self.sigs = sigs                # piece -> signature (all pieces in play)
        self.pose = {}                  # piece -> exact 4x4 pose; identity when absent
        self.lattice = {}               # piece -> True when the pose is a retained-move product
        self._perm_cache = {}

    def snapshot(self):
        return {p: (tuple(map(tuple, r)), self.lattice[p]) for p, r in self.pose.items()}

    def _perm(self, r):
        key = tuple(tuple(row) for row in r)
        if key not in self._perm_cache:
            self._perm_cache[key] = pole_perm(r)
        return self._perm_cache[key]

    def classify(self, e):
        """Exact class of every piece in play for the cut of pole e, with certificates."""
        inside, outside, straddling = [], [], []
        for p, sig in self.sigs.items():
            r = self.pose.get(p)
            if r is None or self.lattice[p]:
                # lattice piece: exact by its (transported) complete signature
                tsig = sig if r is None else {self._perm(r)[x] for x in sig}
                (inside if e in tsig else outside).append(p)
                continue
            hs = [h(e, v) for v in (matvec(r, v) for v in self.regions[p])]
            signs = [x.sign() for x in hs]
            if min(signs) >= 0:
                inside.append(p)
            elif max(signs) <= 0:
                outside.append(p)
            else:
                lo = signs.index(-1)
                hi = signs.index(1)
                straddling.append({'piece': p, 'vertex_below': lo, 'h_below': float(hs[lo]),
                                   'vertex_above': hi, 'h_above': float(hs[hi])})
        return inside, outside, straddling

    def apply(self, e, r, retained):
        """Apply twist (e, r) if admissible; otherwise leave the configuration unchanged."""
        inside, _, straddling = self.classify(e)
        if straddling:
            return False, straddling[0]
        for p in inside:
            old = self.pose.get(p)
            new = r if old is None else matmul(r, old)
            if new == identity(4):
                self.pose.pop(p, None)
                self.lattice.pop(p, None)
            else:
                self.pose[p] = new
                self.lattice[p] = retained and self.lattice.get(p, True)
        return True, len(inside)


def classify_grouped(cfg, e, group_bounds):
    """Like Config.classify, but skips the per-piece test when the posed superset of a whole
    pose group lies on one side of the cut. group_bounds maps a pose key to the exact posed
    vertices of a region containing every piece with that pose."""
    straddling, inside, outside = [], [], []
    by_pose = {}
    for p in cfg.sigs:
        r = cfg.pose.get(p)
        if r is None or cfg.lattice[p]:
            continue
        by_pose.setdefault(tuple(tuple(x) for x in r), []).append(p)
    for key, pieces in by_pose.items():
        signs = [h(e, v).sign() for v in group_bounds[key]]
        if min(signs) >= 0:
            inside += pieces
            continue
        if max(signs) <= 0:
            outside += pieces
            continue
        r = [list(x) for x in key]
        for p in pieces:
            hs = [h(e, matvec(r, v)) for v in cfg.regions[p]]
            sg = [x.sign() for x in hs]
            if min(sg) >= 0:
                inside.append(p)
            elif max(sg) <= 0:
                outside.append(p)
            else:
                lo, hi = sg.index(-1), sg.index(1)
                straddling.append({'piece': int(p), 'vertex_below': lo, 'h_below': float(hs[lo]),
                                   'vertex_above': hi, 'h_above': float(hs[hi])})
    return inside, outside, straddling


def classify_values(hmin, hmax):
    """The section 3 rule on exact extreme values (used by the E0 controls)."""
    if hmin.sign() >= 0:
        return 'inside'
    if hmax.sign() <= 0:
        return 'outside'
    return 'straddling'


def main():
    t0 = time.time()
    res = {'contract': 'research/jumbling/state-contract.md section 6', 'alpha': '121/125'}
    c = 0
    d = sorted(range(600), key=lambda e: -float(NF[e] @ NF[c]))[1]
    res['poles'] = {'c': c, 'd': d, 'angle_c_d_deg': round(math.degrees(math.acos(
        float(NF[c] @ NF[d]) / float(NF[c] @ NF[c]))), 4)}

    # cap regions and candidate constraints
    caps = {}
    for x in (c, d):
        verts = cap_region(x)
        near = [e for e in range(600) if NF[e] @ NF[x] > 0.6 * (NF[x] @ NF[x])]
        out = implied(verts)
        caps[x] = {'verts': verts, 'facets': near, 'poles': [e for e in range(600) if e not in out and e != x]}
    res['cap_regions'] = {str(x): {'vertices': len(v['verts']), 'candidate_poles': len(v['poles']),
                                   'candidate_facets': len(v['facets'])} for x, v in caps.items()}

    in_play = [p for p in range(len(MO) - 1) if c in signature(p) or d in signature(p)]
    sigs = {p: signature(p) for p in in_play}
    jobs = []
    for p in in_play:
        x = c if c in sigs[p] else d
        jobs.append((p, caps[x]['poles'], caps[x]['facets']))
    with Pool(4) as pool:
        built = pool.map(build_region, jobs, chunksize=32)
    regions = {p: v for p, v, _, _ in built}

    # E1: regions against retained data
    e1 = {'pieces': len(in_play), 'full_dimensional': sum(f for _, _, f, _ in built)}
    host_ok = 0
    host_bad = []
    for p, _, _, hd in built:
        three = sorted(e for e, k in hd.items() if k == 3)
        if three == sorted(hosts(p)):
            host_ok += 1
        else:
            host_bad.append({'piece': p, 'computed': three, 'retained': hosts(p)})
    e1['host_patches_match'] = host_ok
    e1['host_patch_mismatches'] = host_bad[:10]
    sp = M['slot_piece']
    sc = M['slot_centers']
    slots = np.where(np.isin(sp, np.array(in_play)))[0]
    nf = NF / np.linalg.norm(NF, axis=1)[:, None] ** 2          # n / |n|^2
    hv = sc[slots] @ nf.T - 121 / 125                            # float offsets of every slot centre
    margin = float(np.abs(hv).min())
    sig_ok = 0
    for k, s in enumerate(slots):
        if frozenset(np.where(hv[k] > 0)[0].tolist()) == sigs[int(sp[s])]:
            sig_ok += 1
    on_host = 0
    for k, s in enumerate(slots):
        p = int(sp[s])
        fac = np.where(np.abs(sc[s] @ nf.T - 1) < 1e-9)[0].tolist()
        if len(fac) == 1 and fac[0] in hosts(p):
            on_host += 1
    e1.update({'slots': int(len(slots)), 'slot_centre_signature_matches_piece': sig_ok,
               'slot_centre_on_one_host_facet': on_host,
               'min_abs_float_cut_offset_of_slot_centres': margin,
               'note': 'slot-centre checks compare with retained floating-point data; region checks are exact'})

    # retained generators of cap c against the exact geometry
    agree = {}
    for k in (2 * c, 2 * c + 1):
        r = generator_matrix(k)
        src, dst = M['move_src'][k], M['move_dst'][k]
        fwd = bwd = 0
        for a, b in zip(src, dst):
            va = {tuple(matvec(r, v)) for v in regions[int(a)]}
            vb = {tuple(v) for v in regions[int(b)]}
            fwd += va == vb
            vb2 = {tuple(matvec(r, v)) for v in regions[int(b)]}
            bwd += vb2 == {tuple(v) for v in regions[int(a)]}
        agree[str(k)] = {'moves': int(len(src)), 'region_of_src_maps_to_dst': fwd, 'region_of_dst_maps_to_src': bwd}
    res['E1'] = e1
    res['E1_retained_generators'] = agree

    # E0 controls
    tiny = Q5(-5, 0, 10 ** 10)
    e0 = {
        'shallow_below': classify_values(tiny, Q5(1, 0, 1000)),
        'shallow_above': classify_values(Q5(-1, 0, 1000), Q5(5, 0, 10 ** 10)),
    }
    cfg = Config(regions, sigs)
    # exact contact: home regions of cap c pieces against the cut of c
    cap_c = [p for p in in_play if c in sigs[p]]
    signs = [[h(c, v).sign() for v in regions[p]] for p in cap_c]
    e0['cap_c_pieces'] = len(cap_c)
    e0['all_inside_exactly'] = all(min(sg) >= 0 for sg in signs)
    e0['pieces_touching_cut_exactly'] = sum(0 in sg for sg in signs)
    a4c = a4(c)
    adm = 0
    for a in a4c:
        trial = Config(regions, sigs)
        for p in in_play:  # treat every piece geometrically to exercise the exact rule
            trial.pose[p] = identity(4)
            trial.lattice[p] = False
        ok, _ = trial.apply(c, a, retained=True)
        adm += ok
    e0['retained_twists_admissible_from_solved'] = f'{adm}/12'
    res['E0'] = e0

    # E2 jumble twist
    g, angle, s = witness_rotation(c, d)
    g_in_a4 = any(g == a for a in a4c)
    res['E2'] = {'angle_deg': round(angle, 6), 'parameter_s': f'{s.a}/{s.d}', 'g_in_A4_c': g_in_a4,
                 'g': [[to_json_q5(x) for x in row] for row in g]}
    ginv = transpose(g)
    t_d = next(a for a in a4(d) if sum((a[i][i] for i in range(4)), ZERO) == ONE)  # order 3: trace 1
    t_d_inv = transpose(t_d)

    def bounds_for(cfg_):
        out = {}
        for p in cfg_.sigs:
            r = cfg_.pose.get(p)
            if r is None or cfg_.lattice[p]:
                continue
            key = tuple(tuple(x) for x in r)
            if key not in out:
                home = c if c in cfg_.sigs[p] else d
                out[key] = [matvec(r, v) for v in caps[home]['verts']]
        return out

    def survey(cfg_):
        gb = bounds_for(cfg_)
        rows = {}
        for e in range(600):
            ins, outs, st = classify_grouped(cfg_, e, gb)
            if st:
                rows[e] = {'status': 'blocked', 'straddling': len(st), 'certificate': st[0]}
            else:
                rows[e] = {'status': 'admissible'}
        return rows

    ok, n_in = cfg.apply(c, g, retained=False)
    assert ok
    after_g = survey(cfg)
    nb = sorted(set().union(*(sigs[p] for p in in_play if c in sigs[p])) - {c})
    res['E2']['moved_pieces'] = n_in
    res['E2']['admissible_after_g'] = {
        'all_poles': sum(r['status'] == 'admissible' for r in after_g.values()),
        'interacting_poles_of_c': {str(e): after_g[e]['status'] for e in [c] + nb},
        'blocked': sorted(e for e, r in after_g.items() if r['status'] == 'blocked'),
    }
    # negative control: a certified blocked grip is rejected and nothing changes
    blocked = res['E2']['admissible_after_g']['blocked']
    neg = {}
    if blocked:
        e_bad = blocked[0]
        before = cfg.snapshot()
        a_bad = a4(e_bad)[1]
        okb, cert = cfg.apply(e_bad, a_bad, retained=True)
        neg = {'pole': e_bad, 'applied': okb, 'unchanged': cfg.snapshot() == before,
               'certificate': cert}
    res['E3_negative_control'] = neg

    # E3 turn at the neighbour
    ok, n_in = cfg.apply(d, t_d, retained=True)
    after_gt = survey(cfg)
    res['E3'] = {'turn_at_d_admissible': ok, 'moved_pieces': n_in,
                 'off_lattice_pieces': sum(1 for p in cfg.pose if not cfg.lattice[p]),
                 'admissible_after_g_then_t': sum(r['status'] == 'admissible' for r in after_gt.values()),
                 'c_after_g_then_t': after_gt[c]['status'],
                 'interacting_poles_of_c_after_g_then_t': {str(e): after_gt[e]['status'] for e in [c] + nb},
                 'blocked_after_g_then_t': sorted(e for e, r in after_gt.items() if r['status'] == 'blocked'),
                 'certificates': {str(e): r['certificate'] for e, r in list(after_gt.items()) if r['status'] == 'blocked'}}

    # E4 reverse
    ok1, _ = cfg.apply(d, t_d_inv, retained=True)
    ok2, _ = cfg.apply(c, ginv, retained=False)
    res['E4'] = {'reverse_t_admissible': ok1, 'reverse_g_admissible': ok2,
                 'all_poses_identity_exactly': ok1 and ok2 and not cfg.pose}
    res['runtime_s'] = round(time.time() - t0, 1)
    OUT.write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: res[k] for k in ('poles', 'E0', 'E1_retained_generators', 'E4')}, indent=1))
    print('E1', {k: v for k, v in res['E1'].items() if k != 'host_patch_mismatches'})
    print('E2 admissible after g:', res['E2']['admissible_after_g']['all_poles'], 'blocked', len(blocked),
          '| E3 admissible after g,t:', res['E3']['admissible_after_g_then_t'], '| neg', {k: v for k, v in neg.items() if k != 'certificate'})


if __name__ == '__main__':
    main()
