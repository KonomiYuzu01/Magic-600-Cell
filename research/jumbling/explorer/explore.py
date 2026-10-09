"""Four-dimensional grip-orbit explorer for rigid jumbling of the 600-cell (workstream J4).

Research code, written from the published formal definition of grip theory
(https://hypercubing.xyz/theory/grip-theory/formal/#jumbling) and from this repository only.
It needs the Python standard library and NumPy. It never reads assets/ or a session.

A grip is a rotation P in SO(4): P maps the base pole n_0 and its base frame to the grip's
position P n_0 and frame. A menu M is a finite set of rotations fixing n_0, given in the base
frame; the twist of grip P by m is P m P^-1. Menus are closed under inverses and under
conjugation by the base cap group A4, so the twists of a lattice grip do not depend on which of
its twelve K+ frames is used. Grips are compared modulo right multiplication by A4.

Closure: starting from the 600 lattice grips, a twist of grip c by m moves every grip e whose
position lies within the interaction angle of c's position (a ball approximation, default 46.8
degrees, between the 44.478 and 49.118 degree shells), and the images are added. Two closures
are computed:

  lattice  only the 600 world-fixed lattice grips twist (state-contract.md section 1 and the
           formal definition's fixed set of allowed axes); a moved grip is a stored grip and is
           identified by its position alone;
  all      every grip in the closure twists with its own frame-conjugated menu; grips are
           identified by position and frame class.

Both closures are invariant under K+ (the 7200 rotations of the 600-cell), because the menu is
K+-conjugated. The search therefore keeps one representative per K+-orbit, in the Voronoi cell
of n_0 and a fundamental domain of A4, and counts grips as the sum of the orbit sizes.

Examples (from the repository root):

    python research/jumbling/explorer/explore.py --list-menus
    python research/jumbling/explorer/explore.py --menu class-05 --depth 3 --budget 200000
    python research/jumbling/explorer/explore.py --preset   # every menu, both closures

The preset writes explorer-results.json (numbers) and orbit-points.json (viewer samples) next
to this file.
"""
import argparse
import base64
import itertools
import json
import math
import random
import sys
import time
from fractions import Fraction
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
from exact import ONE, ZERO, Q5, dot, identity, matmul, matvec, solve, transpose  # noqa: E402
from h4 import frame_perp, h4_polytope, rot_about  # noqa: E402

RESULTS = HERE.parent / 'results.json'
OUT_RESULTS = HERE / 'explorer-results.json'
OUT_POINTS = HERE / 'orbit-points.json'

DEFAULT_THRESHOLD = 46.8      # degrees; between the 44.478 and 49.118 degree shells
POS_TOL = 1e-9                # chord distance under which two float positions coincide
FRAME_TOL = 1e-8              # entrywise distance under which two float frames coincide
TIE_EPS = 1e-9                # score difference treated as a tie in canonicalisation
HASH_CELL = 1e-6              # dedup hash cell (chord units)
SEPARATION_MAX_REPS = 50000   # separation is computed for complete levels up to this size
SEED = 20261009


# ---------------------------------------------------------------------------------------------
# exact helpers

def q5_from_float(x, den=4, max_q=8):
    """The unique (p + q sqrt5)/den with |q| <= max_q within 1e-9 of x."""
    s5 = 5 ** 0.5
    hits = []
    for q in range(-max_q, max_q + 1):
        p = round(den * x - q * s5)
        if abs((p + q * s5) / den - x) < 1e-9:
            hits.append(Q5(p, q, den))
    if len(hits) != 1:
        raise ValueError(f'no unique Q(sqrt5) value for {x}')
    return hits[0]


def qmul_x(a, b):
    a0, a1, a2, a3 = a
    b0, b1, b2, b3 = b
    return [a0 * b0 - a1 * b1 - a2 * b2 - a3 * b3,
            a0 * b1 + a1 * b0 + a2 * b3 - a3 * b2,
            a0 * b2 - a1 * b3 + a2 * b0 + a3 * b1,
            a0 * b3 + a1 * b2 - a2 * b1 + a3 * b0]


def kmat_x(l, r):
    """Exact matrix of x -> l x r (quaternion product); column k is l e_k r."""
    cols = []
    for k in range(4):
        e = [ONE if i == k else ZERO for i in range(4)]
        cols.append(qmul_x(qmul_x(l, e), r))
    return [[cols[k][i] for k in range(4)] for i in range(4)]


def det3(a):
    return (a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
            - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]))


def det4(m):
    s = ZERO
    for j in range(4):
        minor = [[m[i][k] for k in range(4) if k != j] for i in range(1, 4)]
        term = m[0][j] * det3(minor)
        s = s + term if j % 2 == 0 else s - term
    return s


def cross4(a, b, c):
    """Vector v orthogonal to a, b, c with det[a; b; c; v] = |v|^2 (exact)."""
    rows = [a, b, c]
    out = []
    for i in range(4):
        minor = [[r[k] for k in range(4) if k != i] for r in rows]
        d = det3(minor)
        out.append(d if (3 + i) % 2 == 0 else -d)
    return out


def is_rotation_x(r):
    return matmul(transpose(r), r) == identity(4) and det4(r) == ONE


def to_float(m):
    return np.array([[float(x) for x in row] for row in m])


def key_x(m):
    return tuple((x.a, x.b, x.d) for row in m for x in row)


def q5_json(x):
    return [x.a, x.b, x.d]


# ---------------------------------------------------------------------------------------------
# float quaternion helpers

def qmul_f(a, b):
    a0, a1, a2, a3 = np.moveaxis(a, -1, 0)
    b0, b1, b2, b3 = np.moveaxis(b, -1, 0)
    return np.stack([a0 * b0 - a1 * b1 - a2 * b2 - a3 * b3,
                     a0 * b1 + a1 * b0 + a2 * b3 - a3 * b2,
                     a0 * b2 - a1 * b3 + a2 * b0 + a3 * b1,
                     a0 * b3 + a1 * b2 - a2 * b1 + a3 * b0], -1)


def angle_deg(u, v):
    return math.degrees(math.acos(max(-1.0, min(1.0, float(np.dot(u, v))))))


def chord_to_deg(d):
    return math.degrees(2 * math.asin(min(1.0, d / 2)))


# ---------------------------------------------------------------------------------------------
# geometry: the 600 poles, K+ and the base cap group A4

class Geometry:
    """Poles (cell centres of the quaternion 600-cell, as in h4.py and jumble_study.py), the 7200
    rotations of K+ as pairs (l, r) of binary icosahedral units, a K+ frame per pole, and A4."""

    def __init__(self):
        poly = h4_polytope()
        verts = poly['V']
        self.verts_x = [[q5_from_float(float(x)) for x in row] for row in verts]
        self.poles = poly['C']                                  # 600 x 4, unit
        self.poles_x = []                                       # exact, unnormalised (same norm)
        for cell in poly['Cidx']:
            s = [ZERO] * 4
            for i in cell:
                s = [a + b for a, b in zip(s, self.verts_x[i])]
            self.poles_x.append(s)
        n2 = dot(self.poles_x[0], self.poles_x[0])
        assert all(dot(p, p) == n2 for p in self.poles_x)
        self.pole_norm2_x = n2
        self.n0 = self.poles[0]
        self.p0_x = self.poles_x[0]

        # K+: x -> l x r for l, r in 2I; (l, r) and (-l, -r) agree, so keep one sign of l
        neg = [int(np.argmin(np.linalg.norm(verts + v, axis=1))) for v in verts]
        lidx = [i for i in range(120) if i < neg[i]]
        pairs = [(li, ri) for li in lidx for ri in range(120)]
        L = qmul_f(verts[lidx][:, None, :], np.eye(4)[None])          # (60, k, 4): l e_k
        M = qmul_f(L[:, None, :, :], verts[None, :, None, :])         # (60, 120, k, 4): l e_k r
        G = np.swapaxes(M, -1, -2).reshape(-1, 4, 4)                   # [i][k]
        self.kplus = G
        self.kplus_pairs = pairs
        assert len(G) == 7200
        flat = np.round(G.reshape(7200, 16), 9)
        assert len(np.unique(flat, axis=0)) == 7200
        self._kx = {}

        img = G @ self.n0                                              # (7200, 4)
        d = np.abs(img @ self.poles.T - 1)                             # (7200, 600)
        to_pole = np.argmin(d, axis=1)
        assert d[np.arange(7200), to_pole].max() < 1e-9
        self.a4_idx = [g for g in range(7200) if to_pole[g] == 0]
        assert len(self.a4_idx) == 12
        ident = int(np.argmin(np.abs(G - np.eye(4)).reshape(7200, 16).max(1)))
        assert np.abs(G[ident] - np.eye(4)).max() < 1e-12
        self.a4_idx.remove(ident)
        self.a4_idx.insert(0, ident)                                   # A4[0] = identity
        frame = []
        for c in range(600):
            gs = [g for g in range(7200) if to_pole[g] == c]
            assert len(gs) == 12
            frame.append(ident if c == 0 else gs[0])
        self.frame_idx = frame                                         # P_c with P_c n_0 = n_c
        self.P = G[frame]                                              # (600, 4, 4)
        self.A4 = G[self.a4_idx]                                       # (12, 4, 4)
        self.A4_x = [self.kx(g) for g in self.a4_idx]
        self.P_x = {}
        for a in self.A4_x:
            assert is_rotation_x(a) and matvec(a, self.p0_x) == self.p0_x
        # the canonicalising elements A4_a P_c^T as K+ indices: they carry n_c to n_0
        lookup = {tuple(np.round(m, 6).ravel()): g for g, m in enumerate(G)}
        self.GA = np.einsum('aij,ckj->acik', self.A4, self.P)          # (12, 600, 4, 4)
        self.ga_idx = np.array([[lookup[tuple(np.round(self.GA[a, c], 6).ravel())]
                                 for c in range(600)] for a in range(12)])
        self.fperp = frame_perp(self.n0)                               # 3 x 4, for projection

    def kx(self, g):
        """Exact K+ element number g."""
        if g not in self._kx:
            li, ri = self.kplus_pairs[g]
            self._kx[g] = kmat_x(self.verts_x[li], self.verts_x[ri])
        return self._kx[g]

    def frame_x(self, c):
        if c not in self.P_x:
            m = self.kx(self.frame_idx[c])
            assert matvec(m, self.p0_x) == self.poles_x[c]
            self.P_x[c] = m
        return self.P_x[c]

    def face_neighbour(self):
        return int(np.argsort(-(self.poles @ self.n0))[1])

    def stereo(self, x):
        """Stereographic projection from -n_0 into the frame of n_0-perp: n_0 goes to the origin."""
        x = np.atleast_2d(x)
        return (x @ self.fperp.T) / (1 + x @ self.n0)[:, None]


# ---------------------------------------------------------------------------------------------
# menus

class Menu:
    """A4-closed, inverse-closed menu of rotations fixing n_0. elements[0:12] are A4."""

    def __init__(self, geo, name, label, family, gens_f, gens_x, info):
        self.name, self.label, self.family, self.info = name, label, family, info
        self.exact = gens_x is not None
        els_f = list(geo.A4)
        els_x = list(geo.A4_x) if self.exact else None
        seen = {tuple(np.round(m, 8).ravel()) for m in els_f}
        for gi, g in enumerate(gens_f):
            for inv in (False, True):
                for ai, a in enumerate(geo.A4):
                    gf = g.T if inv else g
                    m = a @ gf @ a.T
                    k = tuple(np.round(m, 8).ravel())
                    if k in seen:
                        continue
                    seen.add(k)
                    els_f.append(m)
                    if self.exact:
                        gx = transpose(gens_x[gi]) if inv else gens_x[gi]
                        ax = geo.A4_x[ai]
                        els_x.append(matmul(matmul(ax, gx), transpose(ax)))
        self.f = np.array(els_f)
        self.x = els_x
        self.size = len(els_f)
        self.jumble = list(range(12, self.size))
        assert np.abs(self.f @ geo.n0 - geo.n0).max() < 1e-12
        if self.exact:
            for mf, mx in zip(self.f, self.x):
                assert np.abs(to_float(mx) - mf).max() < 1e-12
            assert len({key_x(m) for m in self.x}) == self.size

    def describe(self):
        d = {'name': self.name, 'label': self.label, 'family': self.family,
             'exact_q_sqrt5': self.exact, 'size': self.size, 'jumble_elements': len(self.jumble)}
        d.update(self.info)
        return d


def rotation_angle(m4, n0):
    """Rotation angle (degrees) of a rotation fixing n0, from its trace on n0-perp."""
    tr = np.trace(m4) - 1.0
    return math.degrees(math.acos(max(-1.0, min(1.0, (tr - 1) / 2))))


def realignment_menus(geo):
    """One exact generator per realignment class of research/jumbling/results.json."""
    classes = json.loads(RESULTS.read_text())['realignments']['classes']
    n0 = geo.n0
    near = [i for i in range(1, 600) if angle_deg(geo.poles[i], n0) < 45]
    menus = []
    for k, row in enumerate(classes):
        r4 = rot_about(n0, np.array(row['R_perp']))
        img = geo.poles[near] @ r4.T
        d = np.linalg.norm(img[:, None, :] - geo.poles[None], axis=2)
        hit = d.min(1) < 1e-6
        pairs = [(near[i], int(d[i].argmin())) for i in np.where(hit)[0]]
        # two aligned poles whose n_0-perp components are independent
        perp = {w: geo.poles[w] - (geo.poles[w] @ n0) * n0 for w, _ in pairs}
        w1, w2 = None, None
        for (a, ua), (b, ub) in itertools.combinations(pairs, 2):
            pa, pb = perp[a], perp[b]
            if abs(pa @ pb) / (np.linalg.norm(pa) * np.linalg.norm(pb)) < 0.999:
                (w1, u1), (w2, u2) = (a, ua), (b, ub)
                break
        assert w1 is not None, f'class {k}: no independent aligned pair'
        px = geo.poles_x
        bw = [geo.p0_x, px[w1], px[w2], cross4(geo.p0_x, px[w1], px[w2])]
        bu = [geo.p0_x, px[u1], px[u2], cross4(geo.p0_x, px[u1], px[u2])]
        rx = transpose(solve(bw, bu))          # R bw_i = bu_i  <=>  bw R^T = bu (rows)
        assert is_rotation_x(rx), f'class {k}: exact realignment is not a rotation'
        assert matvec(rx, geo.p0_x) == geo.p0_x
        for w, u in pairs:
            assert matvec(rx, px[w]) == px[u], f'class {k}: aligned pair {w}->{u} not exact'
        rf = to_float(rx)
        assert np.abs(rf - r4).max() < 1e-9, f'class {k}: exact and float rotations differ'
        info = {'class_index': k,
                'min_rotation_angle_deg': row['min_rotation_angle_deg'],
                'aligned_poles': row['aligned_poles'],
                'aligned_per_shell': row['aligned_per_shell'],
                'max_landing_error_deg': row['max_landing_error_deg'],
                'generator_angle_deg': round(rotation_angle(rf, n0), 6),
                'exact_aligned_pairs_checked': len(pairs)}
        label = f"class {k:02d}: {row['min_rotation_angle_deg']:.2f}°, {row['aligned_poles']} aligned"
        menus.append(Menu(geo, f'class-{k:02d}', label, 'realignment', [rf], [rx], info))
    return menus


def plane_rotation(geo, d, degrees, exact=True, max_den=1000):
    """Rotation fixing n_0 and n_d pointwise by about `degrees` in the orthogonal plane.

    Exact version: g = P + C (I - P) + t J with Cayley parameter s in Q (bounded denominator),
    C = (1 - m s^2)/(1 + m s^2), t = 2 s/(1 + m s^2), as in witness.py; the realised angle is
    reported. Float version: the requested angle exactly, float entries only."""
    a, b = geo.p0_x, geo.poles_x[d]
    gram = [[dot(a, a), dot(a, b)], [dot(b, a), dot(b, b)]]
    m = gram[0][0] * gram[1][1] - gram[0][1] * gram[1][0]
    ginv = solve(gram, identity(2))
    cols = [a, b]
    p = [[sum((cols[i][r] * ginv[i][j] * cols[j][s] for i in range(2) for j in range(2)), ZERO)
          for s in range(4)] for r in range(4)]

    def eps(*idx):
        if len(set(idx)) < 4:
            return 0
        inv = sum(1 for i in range(4) for j in range(i + 1, 4) if idx[i] > idx[j])
        return -1 if inv % 2 else 1
    jm = [[sum((Q5(eps(r, s, k, l)) * a[k] * b[l] for k in range(4) for l in range(4)), ZERO)
           for s in range(4)] for r in range(4)]
    ident = identity(4)
    if exact:
        s_float = math.tan(math.radians(degrees) / 2) / math.sqrt(float(m))
        s = Fraction(s_float).limit_denominator(max_den)
        sx = Q5(s.numerator, 0, s.denominator)
        cc = (ONE - m * sx * sx) / (ONE + m * sx * sx)
        tt = (Q5(2) * sx) / (ONE + m * sx * sx)
        g = [[p[r][q] + cc * (ident[r][q] - p[r][q]) + tt * jm[r][q] for q in range(4)]
             for r in range(4)]
        assert is_rotation_x(g)
        assert matvec(g, a) == list(a) and matvec(g, b) == list(b)
        gf = to_float(g)
        return gf, g, {'cayley_s': [s.numerator, s.denominator],
                       'realised_angle_deg': round(math.degrees(math.acos(float(cc))), 6)}
    pf, jf = to_float(p), to_float(jm)
    th = math.radians(degrees)
    gf = pf + math.cos(th) * (np.eye(4) - pf) + math.sin(th) / math.sqrt(float(m)) * jf
    assert np.abs(gf.T @ gf - np.eye(4)).max() < 1e-12 and np.linalg.det(gf) > 0
    return gf, None, {'realised_angle_deg': float(degrees)}


def plane_menus(geo):
    d = geo.face_neighbour()
    out = []
    for deg, exact in ((10, True), (36, True), (72, True), (36, False)):
        gf, gx, info = plane_rotation(geo, d, deg, exact)
        info.update({'requested_angle_deg': deg, 'fixed_plane': f'span(n_0, n_{d}) (face neighbour)',
                     'generator_angle_deg': round(rotation_angle(gf, geo.n0), 6)})
        name = f'plane-{deg}' + ('' if exact else '-float')
        label = (f'plane {deg}° (exact Cayley, {info["realised_angle_deg"]:.5f}°)' if exact
                 else f'plane {deg}° (float, exact angle)')
        out.append(Menu(geo, name, label, 'plane', [gf], [gx] if exact else None, info))
    return out


def all_menus(geo):
    a4 = Menu(geo, 'a4', 'A4 only (control)', 'control', [], [], {})
    return [a4] + realignment_menus(geo) + plane_menus(geo)


# ---------------------------------------------------------------------------------------------
# canonical representatives of K+-orbits

class Canon:
    """Maps a point y of S^3 to a K+-orbit representative: first into the Voronoi cell of n_0 by
    P_c^T for the nearest pole c, then by A4 to the image with the largest score against a fixed
    generic vector z near n_0. Near-ties (TIE_EPS) return every alternative, so that a lookup of
    all alternatives finds an existing representative of the same orbit. Frames are reduced
    modulo right multiplication by A4 in the same way, with a generic 4 x 4 score matrix."""

    def __init__(self, geo):
        self.geo = geo
        rng = np.random.default_rng(SEED)
        w = rng.normal(size=4)
        w -= (w @ geo.n0) * geo.n0
        z = geo.n0 + 0.37 * w / np.linalg.norm(w)
        self.z = z / np.linalg.norm(z)
        self.AZ = np.einsum('aij,i->aj', geo.A4, self.z)       # score of a: (A_a y) . z = y . AZ_a
        self.PT = np.swapaxes(geo.P, 1, 2)
        self.W = rng.normal(size=(4, 4))                         # frame score: trace(F a W)

    def near_poles(self, x, radius_deg):
        return np.where(self.geo.poles @ x > math.cos(math.radians(radius_deg)))[0]

    def primary(self, Y, cand):
        """Canonical point, pole and A4 index of each row of Y, and a near-tie flag."""
        D = Y @ self.geo.poles[cand].T
        if D.shape[1] > 1:
            top2 = -np.partition(-D, 1, axis=1)[:, :2]
            tie = top2[:, 0] - top2[:, 1] < TIE_EPS
        else:
            tie = np.zeros(len(Y), bool)
        c = cand[D.argmax(1)]
        Yp = np.einsum('bij,bj->bi', self.PT[c], Y)
        S = Yp @ self.AZ.T
        s2 = -np.partition(-S, 1, axis=1)[:, :2]
        tie |= s2[:, 0] - s2[:, 1] < TIE_EPS
        a = S.argmax(1)
        Yc = np.einsum('bij,bj->bi', self.geo.A4[a], Yp)
        return Yc, c, a, tie

    def alternatives(self, y):
        """Every (pole, A4 index, point) that is canonical for y up to near-ties."""
        D = self.geo.poles @ y
        out = []
        for c in np.where(D >= D.max() - TIE_EPS)[0]:
            yp = self.PT[c] @ y
            S = self.AZ @ yp
            for a in np.where(S >= S.max() - TIE_EPS)[0]:
                out.append((int(c), int(a), self.geo.A4[a] @ yp))
        return out

    def right_canon(self, F):
        """Canonical right-A4 representative F a of each frame, the index a, and a near-tie flag."""
        FA = np.einsum('bij,ajk->baik', F, self.geo.A4)
        S = np.einsum('baik,ki->ba', FA, self.W)
        s2 = -np.partition(-S, 1, axis=1)[:, :2]
        a = S.argmax(1)
        return FA[np.arange(len(F)), a], a, s2[:, 0] - s2[:, 1] < TIE_EPS

    def right_variants(self, f):
        """All twelve right-A4 variants of one frame, flattened (12, 16)."""
        return np.einsum('ij,ajk->aik', f, self.geo.A4).reshape(12, 16)


def hash_keys(y):
    """Dedup hash keys of a point: its cell, plus neighbour cells within POS_TOL of an edge."""
    t = np.asarray(y) / HASH_CELL
    k = np.floor(t)
    f = t - k
    opts = []
    for i in range(4):
        o = [int(k[i])]
        if f[i] < POS_TOL / HASH_CELL:
            o.append(int(k[i]) - 1)
        elif f[i] > 1 - POS_TOL / HASH_CELL:
            o.append(int(k[i]) + 1)
        opts.append(o)
    return list(itertools.product(*opts))


# ---------------------------------------------------------------------------------------------
# the closure search

class Store:
    """K+-orbit representatives of grips, grouped by canonical position.

    Representatives: float position, frame (canonical right-A4 form, all-grips closure only),
    depth, stabiliser order in K+, derivation. Positions: one canonical point per K+-orbit of
    positions, with its stabiliser order and the frames of the representatives stored there."""

    def __init__(self, frames):
        self.frames = frames
        self.X = np.zeros((1024, 4))
        self.F = np.zeros((1024, 4, 4)) if frames else None
        self.n = 0
        self.depth, self.stab, self.deriv, self.at = [], [], [], []
        self.ptable = {}
        self.pos, self.pos_stab, self.pos_depth, self.pos_reps = [], [], [], []
        self.pf_buf, self.pf_n = [], []

    def find_position(self, y, keys):
        for key in keys:
            for p in self.ptable.get(key, ()):
                q = self.pos[p]
                if (abs(q[0] - y[0]) < POS_TOL and abs(q[1] - y[1]) < POS_TOL
                        and abs(q[2] - y[2]) < POS_TOL and abs(q[3] - y[3]) < POS_TOL):
                    return p
        return -1

    def find_frame(self, p, variants):
        n = self.pf_n[p]
        if n == 0:
            return -1
        buf = self.pf_buf[p][:n]
        for f in variants:
            m = np.abs(buf - f).max(1)
            k = int(m.argmin())
            if m[k] < FRAME_TOL:
                return self.pos_reps[p][k]
        return -1

    def add_position(self, y, stab, depth):
        p = len(self.pos)
        self.pos.append(tuple(float(v) for v in y))
        self.pos_stab.append(stab)
        self.pos_depth.append(depth)
        self.pos_reps.append([])
        self.pf_buf.append(np.zeros((4, 16)) if self.frames else None)
        self.pf_n.append(0)
        key = tuple(int(v) for v in np.floor(np.asarray(y) / HASH_CELL))
        self.ptable.setdefault(key, []).append(p)
        return p

    def add(self, p, f, depth, stab, deriv):
        if self.n == len(self.X):
            self.X = np.concatenate([self.X, np.zeros_like(self.X)])
            if self.frames:
                self.F = np.concatenate([self.F, np.zeros_like(self.F)])
        i = self.n
        self.X[i] = self.pos[p]
        if self.frames:
            self.F[i] = f
            n = self.pf_n[p]
            if n == len(self.pf_buf[p]):
                self.pf_buf[p] = np.concatenate([self.pf_buf[p], np.zeros_like(self.pf_buf[p])])
            self.pf_buf[p][n] = f.ravel()
            self.pf_n[p] = n + 1
        self.pos_reps[p].append(i)
        self.n += 1
        self.depth.append(depth)
        self.stab.append(stab)
        self.deriv.append(deriv)
        self.at.append(p)
        return i


class Explorer:
    """Breadth-first closure of the grip set for one menu, interaction angle and closure kind."""

    def __init__(self, geo, canon, menu, twisting='lattice', threshold=DEFAULT_THRESHOLD,
                 max_depth=3, budget=200000, time_limit=None, exact_cap=2000, log=print):
        self.geo, self.canon, self.menu = geo, canon, menu
        self.twisting = twisting
        self.threshold = threshold
        self.cos_th = math.cos(math.radians(threshold))
        self.max_depth, self.budget, self.exact_cap = max_depth, budget, exact_cap
        self.time_limit = time_limit
        self.log = log
        self.frames = twisting == 'all'
        self.store = Store(self.frames)
        self.coincidences = []            # (candidate derivation, matched representative)
        self.coinc_count = 0
        self.rng = random.Random(SEED)
        self._tx, self._gx, self._posx, self._framex = {}, {}, {}, {}
        self.vor = 22.25                  # Voronoi cell radius of a pole (degrees), rounded up
        self.reach = threshold + self.vor + 0.5
        if twisting == 'lattice':
            # lattice twisting grips that can reach a canonical representative, their jumble
            # twists P_c m P_c^T, and the poles that can be nearest to one of their images
            self.twisting_poles = self.canon.near_poles(geo.n0, self.reach)
            self.T = {c: geo.P[c] @ menu.f[menu.jumble] @ geo.P[c].T for c in self.twisting_poles}
            self.cand = {c: self.canon.near_poles(geo.poles[c], self.reach) for c in self.twisting_poles}

    # -- exact reconstruction -----------------------------------------------------------------

    def g_x(self, c, a):
        """Exact canonicalising element A4_a P_c^T."""
        if (c, a) not in self._gx:
            self._gx[(c, a)] = matmul(self.geo.A4_x[a], transpose(self.geo.frame_x(c)))
        return self._gx[(c, a)]

    def t_lattice_x(self, c, j):
        if (c, j) not in self._tx:
            p = self.geo.frame_x(c)
            self._tx[(c, j)] = matmul(matmul(p, self.menu.x[j]), transpose(p))
        return self._tx[(c, j)]

    def lattice_image_x(self, parent, c, j, cc, a):
        return matvec(self.g_x(cc, a), matvec(self.t_lattice_x(c, j), self.pos_x(parent)))

    def pos_x(self, i):
        """Exact (unnormalised) position of representative i."""
        if i not in self._posx:
            if self.frames:
                self._posx[i] = matvec(self.frame_xr(i), self.geo.p0_x)
            elif i == 0:
                self._posx[i] = list(self.geo.p0_x)
            else:
                self._posx[i] = self.lattice_image_x(*self.store.deriv[i])
        return self._posx[i]

    def frame_xr(self, i):
        """Exact frame of representative i (all-grips closure), in its stored right-A4 form."""
        if i not in self._framex:
            if i == 0:
                self._framex[i] = identity(4)
            else:
                c, j, r, g, cc, a, ar = self.store.deriv[i]
                raw = self.image_frame_x(c, j, r, g, cc, a)
                self._framex[i] = matmul(raw, self.geo.A4_x[ar])
        return self._framex[i]

    def image_frame_x(self, c, j, r, g, cc, a):
        fc = self.frame_xr(c)
        t = matmul(matmul(fc, self.menu.x[j]), transpose(fc))
        e = matmul(self.geo.kx(g), self.frame_xr(r))
        return matmul(self.g_x(cc, a), matmul(t, e))

    def confirm_exact(self):
        """Exact Q(sqrt5) check of the recorded coincidences (all, or a uniform sample of
        exact_cap of them); returns (checked, confirmed)."""
        if not self.menu.exact or not self.coincidences:
            return 0, 0
        ok = 0
        for deriv, i in self.coincidences:
            if self.frames:
                c, j, r, g, cc, a = deriv
                fx = self.image_frame_x(c, j, r, g, cc, a)
                dmat = matmul(transpose(self.frame_xr(i)), fx)
                same = any(dmat == x for x in self.geo.A4_x)
            else:
                same = self.lattice_image_x(*deriv) == self.pos_x(i)
            ok += bool(same)
        return len(self.coincidences), ok

    def note_coincidence(self, deriv, i):
        self.coinc_count += 1
        if len(self.coincidences) < self.exact_cap:
            self.coincidences.append((deriv, i))
        else:
            k = self.rng.randrange(self.coinc_count)
            if k < self.exact_cap:
                self.coincidences[k] = (deriv, i)

    # -- insertion ----------------------------------------------------------------------------

    def insert(self, Y, Fm, derivs, depth, cand):
        """Canonicalise images Y (and frames Fm), deduplicate, store new representatives."""
        st = self.store
        Yc, cc, aa, tie = self.canon.primary(Y, cand)
        t = Yc / HASH_CELL
        k = np.floor(t)
        fr = t - k
        edge = ((fr < POS_TOL / HASH_CELL) | (fr > 1 - POS_TOL / HASH_CELL)).any(1)
        slow = tie | edge
        keys = k.astype(np.int64).tolist()
        ylist = Yc.tolist()
        if self.frames:
            G = self.geo.GA[aa, cc]
            Fc = np.einsum('bij,bjk->bik', G, Fm)
            Fcan, far, ftie = self.canon.right_canon(Fc)
        for b in range(len(Y)):
            if slow[b]:
                self.insert_slow(Y[b], Fm[b] if self.frames else None, derivs[b], depth,
                                 int(cc[b]), int(aa[b]))
                continue
            y = ylist[b]
            p = st.find_position(y, (tuple(keys[b]),))
            deriv = derivs[b] + (int(cc[b]), int(aa[b]))
            if not self.frames:
                if p >= 0:
                    self.note_coincidence(deriv, st.pos_reps[p][0])
                    continue
                p = st.add_position(y, 1, depth)
                st.add(p, None, depth, 1, deriv)
                continue
            if p >= 0:
                variants = self.canon.right_variants(Fc[b]) if ftie[b] else (Fcan[b].ravel(),)
                i = st.find_frame(p, variants)
                if i >= 0:
                    self.note_coincidence(deriv, i)
                    continue
            else:
                p = st.add_position(y, 1, depth)
            st.add(p, Fcan[b], depth, 1, deriv + (int(far[b]),))

    def insert_slow(self, y, fm, deriv, depth, c0, a0):
        """Near-tie or hash-edge case: look up every canonical alternative."""
        st = self.store
        a4, PT = self.geo.A4, self.canon.PT
        alts = self.canon.alternatives(y)
        if not any(c == c0 and a == a0 for c, a, _ in alts):
            alts.insert(0, (c0, a0, a4[a0] @ PT[c0] @ y))
        found_p = []
        for c2, a2, y2 in alts:
            p = st.find_position(y2, hash_keys(y2))
            if p < 0:
                continue
            found_p.append((p, c2, a2, y2))
            if not self.frames:
                self.note_coincidence(deriv + (c2, a2), st.pos_reps[p][0])
                return
            f2 = a4[a2] @ PT[c2] @ fm
            i = st.find_frame(p, self.canon.right_variants(f2))
            if i >= 0:
                self.note_coincidence(deriv + (c2, a2), i)
                return
        # new representative; it is stored at the existing canonical point of its position
        # orbit if there is one, otherwise at the primary alternative
        if found_p:
            p, cs, as_, ys = found_p[0]
        else:
            p, cs, as_, ys = -1, c0, a0, a4[a0] @ PT[c0] @ y
        same_pos = [(c2, a2) for c2, a2, y2 in alts if np.abs(y2 - ys).max() < POS_TOL]
        if p < 0:
            p = self.store.add_position(ys, len(same_pos), depth)
        if not self.frames:
            st.add(p, None, depth, len(same_pos), deriv + (cs, as_))
            return
        fs = a4[as_] @ PT[cs] @ fm
        fcan, far, _ = self.canon.right_canon(fs[None])
        stab = 0
        for c2, a2 in same_pos:
            f2 = a4[a2] @ PT[c2] @ fm
            d = (fs.T @ f2).ravel()
            if np.abs(self.geo.A4.reshape(12, 16) - d).max(1).min() < FRAME_TOL:
                stab += 1
        st.add(p, fcan[0], depth, max(stab, 1), deriv + (cs, as_, int(far[0])))

    # -- breadth-first search -----------------------------------------------------------------

    def run(self):
        geo = self.geo
        self.t0 = time.time()
        st = self.store
        p = st.add_position(geo.n0, 12, 0)
        st.add(p, np.eye(4) if self.frames else None, 0, 12, None)
        levels = [self.level_summary(0, True, 0, 0, 1.0)]
        status = 'depth limit reached'
        for d in range(1, self.max_depth + 1):
            before = self.coinc_count
            if self.twisting == 'lattice':
                images, fraction, complete, why = self.level_lattice(d)
            else:
                images, fraction, complete, why = self.level_all(d)
            levels.append(self.level_summary(d, complete, images, self.coinc_count - before, fraction))
            new = levels[-1]['new_representatives']
            self.log(f'  {self.menu.name} [{self.twisting}] depth {d}: +{new} representatives, '
                     f'{levels[-1]["cumulative_grips"]} grips, '
                     f'{"complete" if complete else f"truncated at {fraction:.4f} ({why})"} '
                     f'({time.time() - self.t0:.1f} s)')
            if complete and new == 0:
                status = 'closed'
                break
            if not complete:
                status = why
                break
        checked, confirmed = self.confirm_exact()
        return {'status': status, 'levels': levels,
                'exact_coincidence_check': {'menu_exact': self.menu.exact,
                                            'coincidences_total': self.coinc_count,
                                            'checked': checked, 'confirmed': confirmed},
                'search_seconds': round(time.time() - self.t0, 1)}

    def stop_reason(self):
        if self.store.n > self.budget:
            return 'size budget reached'
        if self.time_limit is not None and time.time() - self.t0 > self.time_limit:
            return 'time limit reached'
        return None

    def level_lattice(self, d):
        """The lattice twisting grips act on every representative of depth d - 1. A4 elements
        are skipped: P_c a P_c^T lies in K+, so its images stay in the same K+-orbit."""
        st = self.store
        frontier = np.array([i for i in range(st.n) if st.depth[i] == d - 1], dtype=np.int64)
        X = st.X[frontier]
        jumble = self.menu.jumble
        plan = []
        for c in self.twisting_poles:
            sel = frontier[X @ self.geo.poles[c] > self.cos_th]
            if len(sel):
                plan.append((c, sel))
        total = sum(len(sel) for _, sel in plan) * len(jumble)
        done = 0
        for c, sel in plan:
            for start in range(0, len(sel), 4096):
                chunk = sel[start:start + 4096]
                Y = np.einsum('kij,bj->bki', self.T[c], st.X[chunk]).reshape(-1, 4)
                derivs = [(int(p), int(c), int(j)) for p in chunk for j in jumble]
                self.insert(Y, None, derivs, d, self.cand[c])
                done += len(Y)
                why = self.stop_reason()
                if why and done < total:
                    return done, done / total, False, why
        return done, 1.0, True, None

    def window(self, reps):
        """All K+ images of the given representatives within `reach` of n_0: positions,
        representative and K+ index. The images A4_a P_c^T x with angle(x, n_c) < reach are
        exactly the images within reach of n_0."""
        geo = self.geo
        X = self.store.X[reps]
        rr, cc = np.where(X @ geo.poles.T > math.cos(math.radians(self.reach)))
        pos = np.einsum('apij,pj->pai', geo.GA[:, cc], X[rr]).reshape(-1, 4)
        rep = np.repeat(reps[rr], 12)
        gidx = geo.ga_idx[:, cc].T.reshape(-1)
        return pos, rep, gidx

    def level_all(self, d):
        """Every grip twists: each twisting representative c acts on all grips within the
        interaction angle of it, with at least one of the pair of depth d - 1. A4 twists of the
        lattice representative (frame I) are skipped for the same reason as in level_lattice."""
        st = self.store
        geo = self.geo
        depth = np.array(st.depth)
        old = np.where(depth <= d - 1)[0]

        def menu_of(c):
            return self.menu.jumble if c == 0 else list(range(self.menu.size))

        # moved grips come from blocks of representatives, so that the window stays small
        block = 3000
        blocks = [old[i:i + block] for i in range(0, len(old), block)]
        if len(old) <= 400:
            wins = [self.window(old)]
            total = 0
            for c in old:
                wpos, wrep, _ = wins[0]
                idx = np.arange(len(wpos)) if depth[c] == d - 1 else np.where(depth[wrep] == d - 1)[0]
                total += int(np.sum(wpos[idx] @ st.X[c] > self.cos_th)) * len(menu_of(c))
        else:
            # estimate: grips within the interaction angle, assuming uniform density on S^3
            th = math.radians(self.threshold)
            cap = (th - math.sin(th) * math.cos(th)) / math.pi
            sizes = 7200 // np.array(st.stab)
            g_old = sizes[depth <= d - 1].sum()
            g_new = sizes[depth == d - 1].sum()
            total = sum((g_old if depth[c] == d - 1 else g_new) * cap * len(menu_of(c)) for c in old)
            wins = None
        done = 0
        for bi, reps in enumerate(blocks):
            wpos, wrep, wg = wins[0] if wins else self.window(reps)
            wnew = np.where(depth[wrep] == d - 1)[0]
            for c in old:
                why = self.stop_reason()
                if why:
                    return done, min(done / max(total, 1), 0.999999), False, why
                idx = np.arange(len(wpos)) if depth[c] == d - 1 else wnew
                idx = idx[wpos[idx] @ st.X[c] > self.cos_th]
                if not len(idx):
                    continue
                js = menu_of(c)
                fc = st.F[c]
                T = np.einsum('ij,kjl,ml->kim', fc, self.menu.f[js], fc)    # F_c m F_c^T
                cand = self.canon.near_poles(st.X[c], self.reach)
                for start in range(0, len(idx), 1024):
                    sub = idx[start:start + 1024]
                    r, g = wrep[sub], wg[sub]
                    EF = np.einsum('bij,bjk->bik', geo.kplus[g], st.F[r])
                    Y = np.einsum('kij,bj->bki', T, wpos[sub]).reshape(-1, 4)
                    Fm = np.einsum('kij,bjl->bkil', T, EF).reshape(-1, 4, 4)
                    derivs = [(int(c), int(j), int(ri), int(gi)) for ri, gi in zip(r, g) for j in js]
                    self.insert(Y, Fm, derivs, d, cand)
                    done += len(Y)
                    why = self.stop_reason()
                    if why:
                        return done, min(done / max(total, 1), 0.999999), False, why
        return done, 1.0, True, None

    # -- summaries ----------------------------------------------------------------------------

    def level_summary(self, d, complete, images, coincidences, fraction):
        st = self.store
        depth = np.array(st.depth)
        sizes = 7200 // np.array(st.stab)
        pdepth = np.array(st.pos_depth)
        psizes = 7200 // np.array(st.pos_stab)
        return {'depth': d, 'complete': bool(complete),
                'new_representatives': int(np.sum(depth == d)),
                'new_grips': int(sizes[depth == d].sum()),
                'cumulative_representatives': int(np.sum(depth <= d)),
                'cumulative_grips': int(sizes[depth <= d].sum()),
                'cumulative_positions': int(psizes[pdepth <= d].sum()),
                'images_computed': int(images),
                'images_coinciding': int(coincidences),
                'fraction_of_level_processed': round(float(fraction), 6)}

    # -- separation ---------------------------------------------------------------------------

    def separation(self, upto_depth):
        """Smallest angle between two distinct grip positions of depth <= upto_depth, the
        approximate median nearest-neighbour angle, and an exact distinctness check of the
        closest pair. Every pair of positions has a K+-image with one point in the Voronoi cell
        V_0 of n_0, so it is enough to compare the A4-images of the canonical points (inside
        V_0) with every position image within the current bound of V_0."""
        st, geo = self.store, self.geo
        pids = [p for p in range(len(st.pos)) if st.pos_depth[p] <= upto_depth]
        P = np.array([st.pos[p] for p in pids])
        core = np.einsum('aij,pj->pai', geo.A4, P).reshape(-1, 4)            # (12 n, 4)
        core_src = [(p, -1, a) for p in pids for a in range(12)]
        vol = 2 * math.pi ** 2 / 600
        sigma = (vol / len(core)) ** (1 / 3)
        while True:
            d1, _ = grid_nn(core, core, sigma)
            if np.isfinite(d1).any() or sigma > 2:
                break
            sigma *= 2
        med = float(np.median(d1))
        s0 = float(d1.min()) if np.isfinite(d1).any() else 2.0
        # images in the cells around V_0 within s0 of V_0 (outer bound by the four face planes)
        faces = np.argsort(-(geo.poles @ geo.n0))[1:5]
        normals = geo.n0[None] - geo.poles[faces]
        lim = -s0 * np.linalg.norm(normals, axis=1)
        extra, extra_src = [], []
        for c in self.canon.near_poles(geo.n0, 45.0):
            if c == 0:
                continue
            Yc = core @ geo.P[c].T
            keep = np.where(((Yc @ normals.T) >= lim).all(1))[0]
            extra.append(Yc[keep])
            extra_src += [(core_src[k][0], int(c), core_src[k][2]) for k in keep]
        W = np.vstack([core] + extra)
        src = core_src + extra_src
        d2, j2 = grid_nn(core, W, max(s0, 1e-12))
        k = int(np.argmin(d2))
        smin = float(d2[k])
        out = {'min_separation_deg': round(chord_to_deg(smin), 9) if np.isfinite(smin) else None,
               'median_nearest_neighbour_deg_approx': round(chord_to_deg(med), 6) if np.isfinite(med) else None,
               'positions_compared': int(len(core))}
        if np.isfinite(smin) and self.menu.exact:
            out['closest_pair_exactly_distinct'] = self.exact_distinct(src[k], src[int(j2[k])])
        return out

    def exact_position_image(self, s):
        p, c, a = s
        v = matvec(self.geo.A4_x[a], self.pos_x(self.store.pos_reps[p][0]))
        return v if c < 0 else matvec(self.geo.frame_x(c), v)

    def exact_distinct(self, s1, s2):
        return self.exact_position_image(s1) != self.exact_position_image(s2)

    # -- viewer samples -----------------------------------------------------------------------

    def samples(self, per_depth=1500, window_deg=50.0):
        """Up to per_depth grip positions per depth within window_deg of n_0, by stereographic
        projection from -n_0, quantised to int16 (scale 1e4) and base64-encoded."""
        st, geo = self.store, self.geo
        rng = np.random.default_rng(SEED)
        out = []
        cosw = math.cos(math.radians(window_deg))
        for d in range(1, max(st.pos_depth) + 1):
            pids = np.array([p for p in range(len(st.pos)) if st.pos_depth[p] == d])
            if not len(pids):
                continue
            rng.shuffle(pids)
            pts, got = [], 0
            for start in range(0, len(pids), 256):
                P = np.array([st.pos[p] for p in pids[start:start + 256]])
                rr, cc = np.where(P @ geo.poles.T > cosw)
                img = np.einsum('apij,pj->pai', geo.GA[:, cc], P[rr]).reshape(-1, 4)
                pts.append(img)
                got += len(img)
                if got >= 4 * per_depth:
                    break
            pts = np.unique(np.round(np.vstack(pts), 9), axis=0)
            if len(pts) > per_depth:
                pts = pts[rng.choice(len(pts), per_depth, replace=False)]
            out.append({'depth': d, 'count': int(len(pts)), 'b64': encode_points(geo.stereo(pts))})
        return out


def encode_points(s):
    q = np.clip(np.round(s * 1e4), -32767, 32767).astype('<i2')
    return base64.b64encode(q.tobytes()).decode('ascii')


def grid_nn(Q, W, sigma):
    """For each row of Q, the smallest distance to a row of W that is at least POS_TOL away
    (inf if none lies in the 3^4 neighbouring grid cells of size sigma), and its index. Every
    pair closer than sigma is found; hash collisions only add candidate pairs."""
    kw = np.floor(W / sigma).astype(np.int64)
    kq = np.floor(Q / sigma).astype(np.int64)
    mult = np.array([1, 1000003, 998244353, 1000000007], dtype=np.int64)

    def pack(k):
        with np.errstate(over='ignore'):
            return (k * mult).sum(1)
    hw = pack(kw)
    order = np.argsort(hw, kind='stable')
    hs = hw[order]
    best = np.full(len(Q), np.inf)
    arg = np.full(len(Q), -1, dtype=np.int64)
    for off in itertools.product((-1, 0, 1), repeat=4):
        hq = pack(kq + np.array(off, dtype=np.int64))
        lo = np.searchsorted(hs, hq, 'left')
        hi = np.searchsorted(hs, hq, 'right')
        cnt = hi - lo
        for t in range(int(cnt.max()) if len(cnt) else 0):
            sel = np.where(cnt > t)[0]
            idx = order[lo[sel] + t]
            dist = np.linalg.norm(Q[sel] - W[idx], axis=1)
            dist[dist < POS_TOL] = np.inf
            better = dist < best[sel]
            best[sel[better]] = dist[better]
            arg[sel[better]] = idx[better]
    return best, arg


# ---------------------------------------------------------------------------------------------
# self-test: the K+-reduced search against a direct search without symmetry reduction

def brute_lattice(geo, menu, threshold, depth):
    """Cumulative position counts of the lattice closure by direct search (all 600 twisting
    grips, every menu element, no symmetry reduction); dedup by rounding to 1e-7."""
    cos_th = math.cos(math.radians(threshold))
    T = np.einsum('cij,mjk,clk->cmil', geo.P, menu.f, geo.P)                 # (600, m, 4, 4)
    keyset = set(map(tuple, np.round(geo.poles * 1e7).astype(np.int64).tolist()))
    frontier, counts = geo.poles, [600]
    for _ in range(depth):
        new = []
        near = frontier @ geo.poles.T > cos_th
        for c in range(600):
            sel = frontier[near[:, c]]
            if len(sel):
                new.append(np.einsum('mij,bj->bmi', T[c], sel).reshape(-1, 4))
        Y = np.vstack(new)
        K = np.round(Y * 1e7).astype(np.int64)
        _, first = np.unique(K, axis=0, return_index=True)
        fresh = [i for i in first if tuple(K[i]) not in keyset]
        keyset.update(tuple(K[i]) for i in fresh)
        frontier = Y[fresh]
        counts.append(counts[-1] + len(fresh))
    return counts


def brute_all_depth1(geo, canon, menu, threshold):
    """(grips, positions) of the all-grips closure at depth 1 by direct search; frames are
    compared modulo right A4."""
    cos_th = math.cos(math.radians(threshold))
    T = np.einsum('cij,mjk,clk->cmil', geo.P, menu.f, geo.P)
    near = geo.poles @ geo.poles.T > cos_th
    pos, frm = [geo.poles], [geo.P]
    for c in range(600):
        e = np.where(near[c])[0]
        pos.append(np.einsum('mij,bj->bmi', T[c], geo.poles[e]).reshape(-1, 4))
        frm.append(np.einsum('mij,bjk->bmik', T[c], geo.P[e]).reshape(-1, 4, 4))
    pos, frm = np.vstack(pos), np.vstack(frm)
    fcan, _, ftie = canon.right_canon(frm)
    assert not ftie.any()
    K = np.round(np.hstack([pos, fcan.reshape(-1, 16)]) * 1e6).astype(np.int64)
    return len(np.unique(K, axis=0)), len(np.unique(np.round(pos * 1e7).astype(np.int64), axis=0))


def selftest(geo, canon, menus, threshold):
    ok = True
    for name, depth in (('class-00', 2), ('class-05', 2), ('plane-10', 1)):
        m = menus[name]
        ex = Explorer(geo, canon, m, 'lattice', threshold, max_depth=depth, log=lambda s: None)
        red = [lv['cumulative_grips'] for lv in ex.run()['levels']]
        bru = brute_lattice(geo, m, threshold, depth)
        print(f'selftest lattice {name}: reduced {red}, direct {bru}', 'ok' if red == bru else 'MISMATCH')
        ok &= red == bru
    for name in ('class-00', 'plane-10'):
        m = menus[name]
        ex = Explorer(geo, canon, m, 'all', threshold, max_depth=1, log=lambda s: None)
        lv = ex.run()['levels'][1]
        red = (lv['cumulative_grips'], lv['cumulative_positions'])
        bru = brute_all_depth1(geo, canon, m, threshold)
        print(f'selftest all-grips {name} depth 1: reduced (grips, positions) {red}, direct {bru}',
              'ok' if red == bru else 'MISMATCH')
        ok &= red == bru
    return ok


# ---------------------------------------------------------------------------------------------
# runs and command line

def run_one(geo, canon, menu, twisting, threshold, depth, budget, time_limit, exact_cap,
            samples=True, log=print):
    ex = Explorer(geo, canon, menu, twisting, threshold, depth, budget, time_limit, exact_cap, log)
    res = ex.run()
    t = time.time()
    for lv in res['levels']:
        # complete levels only (a cut level is a partial set); large levels are skipped for time
        if (lv['depth'] >= 1 and lv['complete'] and 1 < lv['cumulative_representatives'] <= SEPARATION_MAX_REPS):
            lv['separation'] = ex.separation(lv['depth'])
    res['separation_seconds'] = round(time.time() - t, 1)
    grips = [lv['new_grips'] for lv in res['levels'] if lv['complete']]
    res['growth_ratios'] = [round(b / a, 4) for a, b in zip(grips[1:], grips[2:]) if a]
    res['verdict'] = ('finite: closed' if res['status'] == 'closed'
                      else 'growing: not closed within the budget')
    out = {'id': f'{menu.name}__{twisting}', 'menu': menu.name, 'twisting': twisting,
           'threshold_deg': threshold, 'depth_limit': depth, 'budget_representatives': budget,
           'time_limit_s': time_limit}
    out.update(res)
    pts = ex.samples() if samples else None
    return out, pts


def _worker(args):
    name, twisting, threshold, depth, budget, time_limit, exact_cap = args
    geo = Geometry()
    canon = Canon(geo)
    menus = {m.name: m for m in all_menus(geo)}
    lines = []
    res, pts = run_one(geo, canon, menus[name], twisting, threshold, depth, budget, time_limit,
                       exact_cap, log=lines.append)
    print('\n'.join(lines), flush=True)
    return res, pts


PRESET = {'lattice': {'depth': 8, 'budget': 150000, 'time_limit': 600},
          'all': {'depth': 3, 'budget': 60000, 'time_limit': 300}}


def preset(threshold, workers, exact_cap):
    from multiprocessing import Pool
    geo = Geometry()
    menus = all_menus(geo)
    jobs = []
    for twisting in ('lattice', 'all'):
        cfg = PRESET[twisting]
        for m in menus:
            jobs.append((m.name, twisting, threshold, cfg['depth'], cfg['budget'], cfg['time_limit'],
                         exact_cap))
    t0 = time.time()
    with Pool(workers) as pool:
        results = pool.map(_worker, jobs, chunksize=1)
    pdir = HERE / 'points'
    pdir.mkdir(exist_ok=True)
    runs = []
    for res, pts in results:
        runs.append(res)
        (pdir / f'{res["id"]}.json').write_text(json.dumps(
            {'id': res['id'], 'scale': 1e4, 'window_deg': 50.0, 'depths': pts}) + '\n')
    lat = geo.poles[geo.poles @ geo.n0 > math.cos(math.radians(120))]
    lat = lat[np.argsort(-(lat @ geo.n0))]
    ang = np.degrees(np.arccos(np.clip(lat @ geo.n0, -1, 1)))
    (pdir / 'lattice.json').write_text(json.dumps({
        'scale': 1e4, 'count': int(len(lat)), 'b64': encode_points(geo.stereo(lat)),
        'angle_deg': [round(float(a), 3) for a in ang]}) + '\n')
    doc = {
        'generated_by': 'python research/jumbling/explorer/explore.py --preset',
        'evidence_kind': ('synthetic geometry: float search with exact Q(sqrt5) confirmation of '
                          'sampled coincidences and of the closest pair; leads, not proofs'),
        'parameters': {'threshold_deg': threshold, 'preset': PRESET, 'pos_tol_chord': POS_TOL,
                       'frame_tol': FRAME_TOL, 'tie_eps': TIE_EPS, 'hash_cell': HASH_CELL,
                       'separation_max_representatives': SEPARATION_MAX_REPS,
                       'exact_cap_per_run': exact_cap, 'seed': SEED, 'workers': workers,
                       'wall_seconds': round(time.time() - t0, 1)},
        'menus': [m.describe() for m in menus],
        'runs': runs,
    }
    OUT_RESULTS.write_text(json.dumps(doc, indent=1) + '\n')
    print(f'wrote {OUT_RESULTS} and {pdir} ({time.time() - t0:.0f} s)')


def short(x):
    return f'{x:,}' if x < 100000 else f'{x:.3g}'.replace('e+0', 'e').replace('e+', 'e')


def table(path=OUT_RESULTS):
    """Markdown results tables (one per closure) from explorer-results.json."""
    doc = json.loads(Path(path).read_text())
    menus = {m['name']: m for m in doc['menus']}
    for twisting in ('lattice', 'all'):
        print(f'\n### Closure: {"600 lattice grips twist" if twisting == "lattice" else "every grip twists"}\n')
        print('| Menu | Jumble twists | Aligned poles | Complete depth | Cumulative grips per depth | '
              'Result | Min separation | Exact checks |')
        print('| --- | ---: | ---: | ---: | --- | --- | ---: | --- |')
        for run in doc['runs']:
            if run['twisting'] != twisting:
                continue
            m = menus[run['menu']]
            done = [lv for lv in run['levels'] if lv['complete']]
            curve = ' → '.join(short(lv['cumulative_grips']) for lv in done)
            cut = [lv for lv in run['levels'] if not lv['complete']]
            if cut:
                curve += f' → ≥{short(cut[0]["cumulative_grips"])}'
            with_sep = [lv for lv in done if lv.get('separation')]
            sep = with_sep[-1]['separation'] if with_sep else {}
            s = sep.get('min_separation_deg')
            sep_txt = '–' if s is None else (f'{s:.3g}°' if s >= 1e-3 else f'{s:.2e}°')
            if s is not None:
                sep_txt += f' (d{with_sep[-1]["depth"]}'
                sep_txt += ', exact)' if sep.get('closest_pair_exactly_distinct') is True else ')'
            ex = run['exact_coincidence_check']
            ex_txt = (f'{ex["confirmed"]:,}/{ex["checked"]:,}' if ex['checked']
                      else ('float menu' if not ex['menu_exact'] else '–'))
            result = 'closed' if run['status'] == 'closed' else 'growing'
            if run['growth_ratios']:
                result += f' (×{run["growth_ratios"][-1]:.3g})'
            print(f'| {m["label"]} | {m["jumble_elements"]} | {m.get("aligned_poles", "–")} | '
                  f'{done[-1]["depth"]} | {curve} | {result} | {sep_txt} | {ex_txt} |')


def main(argv=None):
    ap = argparse.ArgumentParser(description='Four-dimensional grip-orbit explorer (J4).')
    ap.add_argument('--menu', help='menu name (see --list-menus)')
    ap.add_argument('--twisting', choices=('lattice', 'all'), default='lattice',
                    help='which grips twist: the 600 lattice grips, or every grip in the closure')
    ap.add_argument('--depth', type=int, default=4, help='maximum BFS depth')
    ap.add_argument('--threshold', type=float, default=DEFAULT_THRESHOLD,
                    help='interaction angle in degrees (ball approximation)')
    ap.add_argument('--budget', type=int, default=150000,
                    help='maximum number of K+-orbit representatives')
    ap.add_argument('--time-limit', type=float, default=None, help='seconds per run')
    ap.add_argument('--exact-cap', type=int, default=2000,
                    help='coincidences checked exactly per run (uniform sample beyond this)')
    ap.add_argument('--out', help='write the run as JSON to this file')
    ap.add_argument('--list-menus', action='store_true')
    ap.add_argument('--preset', action='store_true',
                    help='run every menu with both closures and write the result files')
    ap.add_argument('--workers', type=int, default=4)
    ap.add_argument('--selftest', action='store_true',
                    help='compare the reduced search with a direct search on small cases')
    ap.add_argument('--table', action='store_true',
                    help='print the Markdown results tables from explorer-results.json')
    args = ap.parse_args(argv)
    if args.table:
        table()
        return 0
    if args.preset:
        preset(args.threshold, args.workers, args.exact_cap)
        return 0
    geo = Geometry()
    canon = Canon(geo)
    menus = {m.name: m for m in all_menus(geo)}
    if args.list_menus:
        for m in menus.values():
            print(f'{m.name:16s} {m.size:3d} elements ({len(m.jumble)} jumble), '
                  f'{"exact" if m.exact else "float"}: {m.label}')
        return 0
    if args.selftest:
        return 0 if selftest(geo, canon, menus, args.threshold) else 1
    if args.menu not in menus:
        ap.error('choose --menu from --list-menus, or use --preset or --selftest')
    res, _ = run_one(geo, canon, menus[args.menu], args.twisting, args.threshold, args.depth,
                     args.budget, args.time_limit, args.exact_cap, samples=False)
    text = json.dumps(res, indent=1)
    if args.out:
        Path(args.out).write_text(text + '\n')
    print(text)
    return 0


if __name__ == '__main__':
    sys.exit(main())

