"""Jumbling study of the retained 600-cell-Full puzzle (cell caps, alpha = 121/125).

Exploratory research code. It reads assets/model.npz read-only, never opens a session and
writes research/jumbling/results.json. Run from the repository root:

    python research/jumbling/jumble_study.py

Sections (keys of results.json):
  shells          poles whose signatures meet one cap, by angle from the cap pole, and the
                  depth below which each shell interacts
  realignments    discrete jumble twists: rotations about the cell axis, outside A4, that
                  carry at least two independent interacting poles onto poles; classified
                  up to A4 x A4 double cosets
  snapping        for each class: does nearest-pole snapping give a bijection of the
                  interacting poles, and how many cap signatures map onto cap signatures
  automorphisms   all permutations of the interacting poles that fix the cap pole and map
                  the set of cap signatures onto itself, compared with the geometric T_d
  approx_profile  smallest worst-case landing error of a rotation at a given distance from A4
  sun_cube        the same landing error for the Sun Cube 45-degree face twist, as a baseline
"""
import collections
import itertools
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from h4 import axis_angle, frame_perp, h4_polytope, rot3_to_axis_angle, rot_about  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent / 'results.json'
SEED = 20261009


def kabsch(pa, pb):
    """Proper rotation R with R @ pa[i] = pb[i] for consistent pairs."""
    u, _, vt = np.linalg.svd(pa.T @ pb)
    d = np.sign(np.linalg.det(vt.T @ u.T))
    return vt.T @ np.diag([1, 1, d]) @ u.T


def orthonormalize(r):
    u, _, vt = np.linalg.svd(r)
    return u @ vt


def chord_to_deg(chord):
    return np.degrees(2 * np.arcsin(np.clip(chord / 2, 0, 1)))


def load_model():
    m = np.load(ROOT / 'assets' / 'model.npz')
    normals = m['normals'] / np.linalg.norm(m['normals'], axis=1)[:, None]
    return m, normals


def align_to_model(cells, normals):
    """Orthogonal Q with Q @ cells[i] = normals[c2m[i]] for all 600 cell poles."""
    def nearest4(x, i):
        return list(np.argsort(-(x @ x[i]))[1:5])
    a = [0] + nearest4(cells, 0)
    for perm in itertools.permutations(nearest4(normals, 0)):
        b = [0] + list(perm)
        u, _, vt = np.linalg.svd(cells[a].T @ normals[b])
        q = (u @ vt).T
        if np.abs(cells[a] @ q.T - normals[b]).max() > 1e-9:
            continue
        d = np.linalg.norm((cells @ q.T)[:, None, :] - normals[None], axis=2)
        if d.min(1).max() < 1e-9:
            return q, d.argmin(1)
    raise RuntimeError('no alignment between the quaternion cells and the model poles')


def shells_section(poly, cells, v):
    verts = poly['V']
    rin = float(np.linalg.norm(verts[list(poly['Cidx'][0])].mean(0)))
    ang = np.degrees(np.arccos(np.clip(cells @ v, -1, 1)))

    def max_min(u):
        # max over the polytope of min(x.u, x.v): a 2D hull problem over the 120 vertices
        a, b = verts @ u, verts @ v
        best = float(np.max(np.minimum(a, b)))
        da = a - b
        for i in np.where(da > 0)[0]:
            for j in np.where(da < 0)[0]:
                t = da[i] / (da[i] - da[j])
                best = max(best, float(a[i] + t * (a[j] - a[i])))
        return best

    rows = []
    for s in sorted(set(np.round(ang, 3)))[1:9]:
        members = np.where(np.abs(ang - s) < 1e-3)[0]
        rows.append({
            'angle_deg': float(s),
            'poles': int(len(members)),
            'interacts_iff_depth_below': round(max_min(cells[members[0]]) / rin, 5),
        })
    return {'inradius': rin, 'depth_unit': 'cut offset / facet distance', 'retained_depth': 121 / 125,
            'shells': rows}


class Local:
    """Interacting poles of one cap, projected to the cap pole's perpendicular 3-space."""

    def __init__(self, cells, v):
        ang = np.degrees(np.arccos(np.clip(cells @ v, -1, 1)))
        self.inter = np.where((ang > 1e-6) & (ang < 45))[0]
        d = cells[self.inter] @ frame_perp(v).T
        self.dn = d / np.linalg.norm(d, axis=1)[:, None]
        self.shell = np.round(ang[self.inter], 3)
        self.shells = sorted(set(self.shell))
        self.n = len(self.inter)
        self.a4 = self._a4()

    def landing(self, r3):
        """Per pole: 4D angle (deg) from its image to the nearest pole of its shell."""
        x = self.dn @ r3.T
        out = np.zeros(self.n)
        for s in self.shells:
            msk = self.shell == s
            dist = np.linalg.norm(x[msk][:, None, :] - self.dn[msk][None], axis=2).min(1)
            half = np.arcsin(np.clip(dist / 2, 0, 1))
            out[msk] = 2 * np.degrees(np.arcsin(np.sin(np.radians(s)) * np.sin(half)))
        return out

    def _a4(self):
        s1 = np.where(self.shell == self.shells[0])[0]
        a4 = [r for u1, u2 in itertools.permutations(s1, 2)
              for r in [kabsch(self.dn[s1[:2]], self.dn[[u1, u2]])]
              if self.landing(r).max() < 1e-5]
        assert len(a4) == 12, len(a4)
        return np.array(a4)

    def dist_from_a4(self, rs):
        tr = np.einsum('bij,kij->bk', rs, self.a4)
        return np.degrees(np.arccos(np.clip((tr.max(1) - 1) / 2, -1, 1)))


def realignment_section(loc):
    reps, seen = [], set()
    for i in range(loc.n):
        key = min(int(np.argmin(np.linalg.norm(loc.dn - a @ loc.dn[i], axis=1))) for a in loc.a4)
        if key not in seen:
            seen.add(key)
            reps.append(key)
    found = {}
    for w1 in reps:
        for w2 in range(loc.n):
            c12 = loc.dn[w1] @ loc.dn[w2]
            if abs(abs(c12) - 1) < 1e-9:
                continue
            for u1 in np.where(loc.shell == loc.shell[w1])[0]:
                for u2 in np.where(loc.shell == loc.shell[w2])[0]:
                    if abs(loc.dn[u1] @ loc.dn[u2] - c12) > 1e-9:
                        continue
                    r = kabsch(loc.dn[[w1, w2]], loc.dn[[u1, u2]])
                    if loc.landing(r).max() < 1e-5:
                        continue
                    found.setdefault(tuple(np.round(r, 7).ravel()), r)
    classes = []
    for r in found.values():
        for c in classes:
            if any(np.allclose(a @ r @ b, c['R'], atol=1e-6) for a in loc.a4 for b in loc.a4):
                c['members'] += 1
                break
        else:
            classes.append({'R': r, 'members': 1})
    rows = []
    for c in classes:
        r = c['R']
        d = loc.landing(r)
        best = min(((rot3_to_axis_angle(a @ r @ b)[1], a @ r @ b) for a in loc.a4 for b in loc.a4),
                   key=lambda t: t[0])
        rows.append({
            'min_rotation_angle_deg': round(float(np.degrees(best[0])), 4),
            'aligned_poles': int(np.sum(d < 1e-4)),
            'aligned_per_shell': [int(np.sum((d < 1e-4) & (loc.shell == s))) for s in loc.shells],
            'max_landing_error_deg': round(float(d.max()), 4),
            'R_perp': orthonormalize(best[1]).tolist(),
        })
    rows.sort(key=lambda r: (-r['aligned_poles'], r['max_landing_error_deg']))
    return {'rotations_found': len(found), 'orbit_representatives': len(reps), 'classes': rows}


def cap_signatures(m, v_m):
    mo, mv = m['mask_offsets'], m['mask_values']
    caps = [p for p in range(len(mo) - 1) if v_m in mv[mo[p]:mo[p + 1]]]
    sigs = [frozenset(mv[mo[p]:mo[p + 1]].tolist()) for p in caps]
    return caps, sigs


def snapping_section(classes, v, q, normals, sigs, orbit_of):
    sigset = set(sigs)
    poles = sorted(set().union(*sigs))
    out = []
    for row in classes:
        r4 = q @ rot_about(v, np.array(row['R_perp'])) @ q.T
        dist = np.linalg.norm((normals[poles] @ r4.T)[:, None, :] - normals[None], axis=2)
        tgt = dist.argmin(1).tolist()
        sigma = dict(zip(poles, tgt))
        bijective = sorted(tgt) == poles
        ok, failing = 0, collections.Counter()
        for k, x in enumerate(sigs):
            y = frozenset(sigma[p] for p in x)
            if len(y) == len(x) and y in sigset:
                ok += 1
            else:
                failing[orbit_of[k]] += 1
        out.append({
            'min_rotation_angle_deg': row['min_rotation_angle_deg'],
            'pole_bijection': bool(bijective),
            'cap_signatures_mapped_to_signatures': ok,
            'cap_signatures': len(sigs),
            'max_error_to_any_pole_deg': round(float(chord_to_deg(dist.min(1)).max()), 3),
            'orbits_with_failures': len(failing),
        })
    return out


def automorphism_section(sigs, v_m, normals):
    """Individualisation-refinement search for all signature-preserving pole permutations."""
    sigset = set(sigs)
    poles = sorted(set().union(*sigs))
    gi = {g: i for i, g in enumerate(poles)}
    n = len(poles)
    inc = [[] for _ in range(n)]
    for k, x in enumerate(sigs):
        for p in x:
            inc[gi[p]].append(k)

    def refine(col):
        while True:
            mcol = [tuple(sorted(col[gi[p]] for p in x)) for x in sigs]
            sig = [(col[i], tuple(sorted(mcol[k] for k in inc[i]))) for i in range(n)]
            keys = {s: j for j, s in enumerate(sorted(set(sig)))}
            new = [keys[s] for s in sig]
            if len(set(new)) == len(set(col)):
                return new
            col = new

    auts = []

    def search(ca, cb):
        classes = collections.defaultdict(list)
        for i, c in enumerate(ca):
            classes[c].append(i)
        if all(len(x) == 1 for x in classes.values()):
            inv = {c: i for i, c in enumerate(cb)}
            perm = tuple(poles[inv[ca[i]]] for i in range(n))
            if all(frozenset(perm[gi[p]] for p in x) in sigset for x in sigs):
                auts.append(perm)
            return
        c = min((k for k, x in classes.items() if len(x) > 1), key=lambda k: len(classes[k]))
        a = classes[c][0]
        for b in [i for i, x in enumerate(cb) if x == c]:
            top = max(max(ca), max(cb)) + 1
            na, nb = list(ca), list(cb)
            na[a] = top
            nb[b] = top
            na, nb = refine(na), refine(nb)
            if collections.Counter(na) == collections.Counter(nb):
                search(na, nb)

    col = refine([1 if p == v_m else 0 for p in poles])
    search(col, col)

    # geometric symmetries fixing the pole, by images of the four face-neighbour poles
    vv, g = normals[v_m], normals[poles]
    s1 = [i for i in range(n) if poles[i] != v_m and g[i] @ vv > np.cos(np.radians(16))]
    geo, geo_rot = set(), set()
    for img in itertools.permutations(s1, 4):
        mtx = np.linalg.lstsq(np.vstack([vv, g[s1]]), np.vstack([vv, g[list(img)]]), rcond=None)[0].T
        if np.abs(mtx @ mtx.T - np.eye(4)).max() > 1e-6:
            continue
        d = np.linalg.norm((g @ mtx.T)[:, None] - g[None], axis=2)
        if d.min(1).max() < 1e-6:
            t = tuple(poles[j] for j in d.argmin(1))
            geo.add(t)
            if np.linalg.det(mtx) > 0:
                geo_rot.add(t)
    return {
        'poles': n,
        'cap_signatures': len(sigs),
        'refined_colour_class_sizes': sorted(collections.Counter(col).values()),
        'automorphisms_fixing_cap_pole': len(auts),
        'geometric_symmetries_fixing_cap_pole': len(geo),
        'geometric_rotations_fixing_cap_pole': len(geo_rot),
        'automorphisms_equal_geometric_symmetries': set(auts) == geo,
    }


def quat_to_mat(q):
    q = q / np.linalg.norm(q, axis=-1, keepdims=True)
    w, x, y, z = np.moveaxis(q, -1, 0)
    return np.stack([
        np.stack([1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)], -1),
        np.stack([2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)], -1),
        np.stack([2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)], -1),
    ], -2)


def approx_section(loc, samples=200000):
    rng = np.random.default_rng(SEED)
    rs = quat_to_mat(rng.normal(size=(samples, 4)))
    x = np.einsum('bij,nj->bni', rs, loc.dn)
    worst = np.zeros(samples)
    for s in loc.shells:
        msk = loc.shell == s
        c = np.einsum('bni,mi->bnm', x[:, msk], loc.dn[msk]).max(-1)
        half = np.arccos(np.clip(c, -1, 1)) / 2
        worst = np.maximum(worst, (2 * np.degrees(np.arcsin(np.sin(np.radians(s)) * np.sin(half)))).max(-1))
    da = loc.dist_from_a4(rs)
    prof = {str(d): round(float(worst[da >= d].min()), 2) for d in (5, 10, 15, 20, 25, 30, 40, 50, 60)}
    return {'samples': samples, 'seed': SEED,
            'best_max_landing_error_deg_by_min_distance_from_A4_deg': prof,
            'nearest_pole_spacing_deg': float(loc.shells[0])}


def sun_cube_section():
    f = [np.array(p, float) for p in itertools.permutations((1, 0, 0))]
    f = np.unique(np.vstack([f, [-p for p in f]]), axis=0)
    e = np.unique(np.array([p for s in itertools.product([1, -1], repeat=2)
                            for p in itertools.permutations((s[0], s[1], 0))], float), axis=0) / np.sqrt(2)
    vtx = np.array(list(itertools.product([1, -1], repeat=3)), float) / np.sqrt(3)
    allg = np.vstack([f, e, vtx])
    r = axis_angle([1, 0, 0], np.pi / 4)
    moved = allg[allg[:, 0] > -1e-9]
    err = [float(np.degrees(np.arccos(np.clip((allg @ (r @ p)).max(), -1, 1)))) for p in moved]
    gram = np.clip(allg @ allg.T, -1, 1)
    np.fill_diagonal(gram, -1)
    return {'grips': {'F': len(f), 'E': len(e), 'V': len(vtx)},
            'max_landing_error_deg': round(max(err), 3),
            'nearest_grip_spacing_deg': round(float(np.degrees(np.arccos(gram.max()))), 3)}


def main():
    poly = h4_polytope()
    cells = poly['C']
    v = cells[0]
    m, normals = load_model()
    q, c2m = align_to_model(cells, normals)
    v_m = int(c2m[0])
    loc = Local(cells, v)
    caps, sigs = cap_signatures(m, v_m)
    orbit_of = [int(m['orbit_id'][p]) for p in caps]
    res = {'model_cap_pole_index': v_m, 'cap_pieces': len(caps)}
    res['shells'] = shells_section(poly, cells, v)
    res['realignments'] = realignment_section(loc)
    res['snapping'] = snapping_section(res['realignments']['classes'], v, q, normals, sigs, orbit_of)
    res['automorphisms'] = automorphism_section(sigs, v_m, normals)
    res['approx_profile'] = approx_section(loc)
    res['sun_cube'] = sun_cube_section()
    OUT.write_text(json.dumps(res, indent=1) + '\n')
    a = res['automorphisms']
    print(f"cap pieces {len(caps)}; interacting poles {loc.n}; realignment classes "
          f"{len(res['realignments']['classes'])}; bijective snappings "
          f"{sum(s['pole_bijection'] for s in res['snapping'])}; automorphisms {a['automorphisms_fixing_cap_pole']} "
          f"(geometric {a['geometric_symmetries_fixing_cap_pole']}, equal {a['automorphisms_equal_geometric_symmetries']})")


if __name__ == '__main__':
    main()
