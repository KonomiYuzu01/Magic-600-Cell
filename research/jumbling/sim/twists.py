"""Twists of the jumbling puzzle (state contract section 1; amendments A1 and A4).

A twist is a grip c and an exact rotation g in SO(3)_c (g n_c = n_c, g^T g = I, det g = 1).
Every constructor here produces exact Q(sqrt5) matrices and checks these identities exactly.

Families:
- retained: the 1,200 generators and every element of A4_c (g in K+);
- plane: the witness construction g = P + C (I - P) + t J fixing n_c and a second pole n_d
  (state contract section 6, E2), with parameter s in Q(sqrt5);
- cayley (A1): R = (I - W)^-1 (I + W) = I + 2 (W + W^2) / (1 + |w|^2) in the exact frame
  u1 = (-n1, n0, -n3, n2), u2 = (-n2, n3, n0, -n1), u3 = (-n3, -n2, n1, n0) of n_c-perp (as
  quaternions, u1 = i n, u2 = j n, u3 = k n), with parameter w in Q(sqrt5)^3. R rotates by
  2 atan |w| about w; every rotation except a half-turn has such a parameter.
- half_turn (A1): H_u = 2(P_c + P_u) - I for a nonzero exact cap-frame axis u. It is its
  own inverse and includes the non-A4 negative control fixing n_0 and n_13.

A plane rotation with parameter s about the second pole d equals the Cayley rotation with
w = -s (u1 . n_d, u2 . n_d, u3 . n_d); `tests/test_jumbling_sim.py` checks this exactly.
"""
import hashlib
import json
import math
import operator
from fractions import Fraction

import numpy as np

from exact import ONE, ZERO, Q5, dot, identity, matmul, matvec, solve, transpose

from .kernel import q5_float
from .kplus import (generator_word_matrix, is_rotation, matrix_from_json, matrix_json, q5_from_json, q5_json,
                    to_tuple)


class TwistError(ValueError):
    pass


def cap_frame(n):
    """The exact frame (u1, u2, u3) of n-perp; each u_i has |u_i|^2 = |n|^2."""
    return [[-n[1], n[0], -n[3], n[2]], [-n[2], n[3], n[0], -n[1]], [-n[3], -n[2], n[1], n[0]]]


def _eps(*idx):
    if len(set(idx)) < 4:
        return 0
    inv = sum(1 for i in range(4) for j in range(i + 1, 4) if idx[i] > idx[j])
    return -1 if inv % 2 else 1


def plane_matrix(a, b, s):
    """g = P + C (I - P) + t J fixing a and b pointwise (the witness construction, any s)."""
    gram = [[dot(a, a), dot(a, b)], [dot(b, a), dot(b, b)]]
    m = gram[0][0] * gram[1][1] - gram[0][1] * gram[1][0]
    if m.is_zero():
        raise TwistError('the two poles are dependent')
    ginv = solve(gram, identity(2))
    cols = [a, b]
    p = [[sum((cols[i][r] * ginv[i][j] * cols[j][q] for i in range(2) for j in range(2)), ZERO)
          for q in range(4)] for r in range(4)]
    jm = [[sum((Q5(_eps(r, q, k, l)) * a[k] * b[l] for k in range(4) for l in range(4)), ZERO)
           for q in range(4)] for r in range(4)]
    den = ONE + m * s * s
    cc = (ONE - m * s * s) / den
    tt = (Q5(2) * s) / den
    ident = identity(4)
    return to_tuple([[p[r][q] + cc * (ident[r][q] - p[r][q]) + tt * jm[r][q] for q in range(4)]
                     for r in range(4)]), m


def cayley_matrix(n, omega, nn):
    """Exact Cayley rotation fixing n with parameter omega in the frame cap_frame(n)."""
    w1, w2, w3 = omega
    wm = [[ZERO, -w3, w2], [w3, ZERO, -w1], [-w2, w1, ZERO]]
    w2m = matmul(wm, wm)
    f = Q5(2) / (ONE + w1 * w1 + w2 * w2 + w3 * w3)
    r = [[(ONE if i == j else ZERO) + f * (wm[i][j] + w2m[i][j]) for j in range(3)] for i in range(3)]
    u = cap_frame(n)
    inv = nn.inv()
    return to_tuple([[(n[a] * n[b] + sum((r[i][j] * u[i][a] * u[j][b] for i in range(3) for j in range(3)), ZERO)) * inv
                      for b in range(4)] for a in range(4)])


def half_turn_matrix(n, axis):
    """H_u = 2(P_n + P_u) - I, with u in the exact cap-frame coordinates."""
    if len(axis) != 3:
        raise TwistError('half-turn axis needs three components')
    frame = cap_frame(n)
    u = [sum((axis[i] * frame[i][j] for i in range(3)), ZERO) for j in range(4)]
    uu, nn = dot(u, u), dot(n, n)
    if uu.is_zero():
        raise TwistError('half-turn axis must not be zero')
    if not dot(n, u).is_zero():
        raise AssertionError('cap frame is not perpendicular to the pole')
    return to_tuple([[Q5(2) * (n[i] * n[j] / nn + u[i] * u[j] / uu)
                      - (ONE if i == j else ZERO) for j in range(4)] for i in range(4)])


def _quat(w):
    w = np.atleast_2d(np.asarray(w, float))
    q = np.hstack([np.ones((len(w), 1)), w])
    return q / np.linalg.norm(q, axis=1, keepdims=True)


def rotation_distance_deg(w1, w2):
    """Angle in degrees of the rotation taking Cayley rotation w2 to w1 (float, display)."""
    p, q = _quat(w1), _quat(w2)
    s = np.sign(np.sum(p * q, axis=1, keepdims=True))
    s[s == 0] = 1
    d = np.linalg.norm(p - s * q, axis=1)
    return np.degrees(4 * np.arcsin(np.clip(d / 2, 0, 1)))


INPUT_TIE = 1e-12


def _input_quaternion(axis, degrees):
    a = np.asarray(axis, float)
    if a.shape != (3,) or not np.all(np.isfinite(a)) or not np.any(a):
        raise TwistError('axis must be three finite numbers, not all zero')
    # Scaling first also handles finite axes whose unscaled norm would overflow.
    a = a / np.max(np.abs(a))
    a /= np.linalg.norm(a)
    if not math.isfinite(float(degrees)):
        raise TwistError('angle must be finite')
    theta = math.radians(math.remainder(float(degrees), 360.0))
    target = np.r_[math.cos(theta / 2), math.sin(theta / 2) * a]
    return a, target / np.linalg.norm(target)


def _input_bounds(max_num, max_den):
    try:
        n, d = operator.index(max_num), operator.index(max_den)
    except TypeError as exc:
        raise TwistError('N and D must be integers') from exc
    if n < 0 or d < 1:
        raise TwistError('N must be nonnegative and D must be positive')
    return n, d


def _frobenius(nums, den, target):
    """Quaternion formula for ||R - R_target||_F, stable even at distance zero.

    nums has shape (m, 3), den shape (m,) or (m, k). A zero denominator is the half-turn
    branch. The fixed fourth-space direction adds zero to the Frobenius distance."""
    den = np.asarray(den, float)
    v = nums if den.ndim == 1 else np.broadcast_to(nums[:, None, :], den.shape + (3,))
    q = np.concatenate((den[..., None], v), axis=-1)
    q /= np.linalg.norm(q, axis=-1, keepdims=True)
    return math.sqrt(2) * np.linalg.norm(q - target, axis=-1) * np.linalg.norm(q + target, axis=-1)


def nearest_parameter(axis, degrees, max_den, max_num=16):
    """Global Frobenius minimum in A1's bounded space (N=max_num, D=max_den).

    Enumerate every numerator triple. For each, the squared quaternion dot product as a
    function of d is (c*d + B)^2/(d*d + C). Its maximum on integer 1..D is at an endpoint
    or next to d=c*C/B. No other denominator can improve it. A second pass finds the first
    denominator within 1e-12 of the global minimum, so tolerance ties are handled too.
    Half-turn axes need only denominator 1: scaling an axis leaves its rotation unchanged.
    See README.md for the pruning and tie argument."""
    n, dmax = _input_bounds(max_num, max_den)
    a, target = _input_quaternion(axis, degrees)
    grid = np.arange(-n, n + 1, dtype=np.int64)
    nums = np.stack(np.meshgrid(grid, grid, grid, indexing='ij'), axis=-1).reshape(-1, 3)
    b = nums @ target[1:]
    c = np.sum(nums.astype(float) ** 2, axis=1)
    stationary = np.divide(target[0] * c, b, out=np.ones(len(b)), where=b != 0)
    stationary = np.clip(stationary, 1, dmax)
    ds = np.stack((np.ones(len(b)), np.full(len(b), dmax),
                   np.floor(stationary), np.ceil(stationary)), axis=1)
    dist = _frobenius(nums, ds, target)
    arg = np.argmin(dist, axis=1)
    minima = dist[np.arange(len(nums)), arg]
    best = float(minima.min())
    half = abs(abs(math.remainder(float(degrees), 360.0)) - 180.0) <= 1.0
    nonzero = np.any(nums != 0, axis=1)
    half_dist = _frobenius(nums[nonzero], np.zeros(int(nonzero.sum())), target) if half and nonzero.any() else np.zeros(0)
    if half_dist.size:
        best = min(best, float(half_dist.min()))
    limit = best + INPUT_TIE
    tied = np.flatnonzero(minima <= limit)
    choices = []
    if tied.size:
        ns = nums[tied]
        lo = np.ones(len(tied), dtype=np.int64)
        hi = ds[tied, arg[tied]].astype(np.int64)
        # Before the first acceptable denominator the predicate is false, then true:
        # a positive stationary point is the only distance minimum; if the dot product
        # crosses zero instead, distance first increases, then decreases. Test d=1 first.
        first_ok = _frobenius(ns, lo, target) <= limit
        hi[first_ok] = 1
        while np.any(lo < hi):
            mid = (lo + hi) // 2
            ok = _frobenius(ns, mid, target) <= limit
            hi = np.where(ok, mid, hi)
            lo = np.where(ok, lo, mid + 1)
        for p, d in zip(ns.tolist(), hi.tolist()):
            choices.append((d, tuple(p), 'cayley'))
    if half_dist.size:
        hn = nums[nonzero]
        choices += [(1, tuple(p), 'half_turn') for p in hn[half_dist <= limit].tolist()]
    den, num, family = min(choices)  # final identical-key tie is deterministic (Cayley first)
    p = np.array(num, float)
    realised_axis = p / np.linalg.norm(p) if np.any(p) else a.copy()
    realised = 180.0 if family == 'half_turn' else math.degrees(2 * math.atan(float(np.linalg.norm(p)) / den))
    distance = float(_frobenius(p[None, :], np.array([0 if family == 'half_turn' else den]), target)[0])
    requested_axis = target[1:] * (1 if target[0] >= 0 else -1)
    rn = np.linalg.norm(requested_axis)
    cosine = float(realised_axis @ requested_axis / rn) if rn else 1.0
    if family == 'half_turn':
        cosine = abs(cosine)
    report = {'requested_angle_deg': float(degrees), 'requested_axis': a.tolist(),
              'N': n, 'D': dmax, 'max_num': n, 'max_den': dmax, 'family': family,
              'numerators': list(num), 'denominator': den, 'realised_axis': realised_axis.tolist(),
              'realised_angle_deg': realised, 'distance': distance, 'minimum_distance': best,
              'axis_error_deg': math.degrees(math.acos(max(-1.0, min(1.0, cosine)))),
              'rotation_error_deg': math.degrees(2 * math.asin(min(1.0, distance / (2 * math.sqrt(2)))))}
    return list(num), den, report


class Twist:
    """An exact twist (grip, g). Construction checks g exactly; kidx >= 0 marks g in A4_grip."""
    __slots__ = ('ctx', 'grip', 'matrix', 'family', 'params', 'kidx')

    def __init__(self, ctx, grip, matrix, family='matrix', params=None, kidx=None):
        if not 0 <= int(grip) < 600:
            raise TwistError('grip must be a pole index 0..599')
        m = to_tuple(matrix)
        if len(m) != 4 or any(len(r) != 4 for r in m) or not all(isinstance(x, Q5) for r in m for x in r):
            raise TwistError('a twist needs an exact 4 x 4 matrix over Q(sqrt5)')
        n = ctx.data.N[int(grip)]
        if not is_rotation(m):
            raise TwistError('matrix is not exactly a rotation (g^T g = I, det g = 1)')
        if matvec(m, n) != list(n):
            raise TwistError('matrix does not fix the pole of the grip')
        self.ctx = ctx
        self.grip = int(grip)
        self.matrix = m
        self.family = family
        self.params = dict(params or {})
        self.kidx = ctx.kplus.index_of_matrix(m) if kidx is None else int(kidx)

    @property
    def retained(self):
        return self.kidx >= 0

    @property
    def is_identity(self):
        return self.kidx == 0

    def angle_deg(self):
        """Rotation angle in n_c-perp (display): sin^2(theta/2) = (4 - trace g)/4 exactly."""
        tr = self.matrix[0][0] + self.matrix[1][1] + self.matrix[2][2] + self.matrix[3][3]
        x = q5_float((Q5(4) - tr) / 4)
        return math.degrees(2 * math.asin(math.sqrt(max(0.0, min(1.0, x)))))

    def inverse(self):
        p = dict(self.params)
        fam = self.family
        if fam == 'retained':
            p = {'word': [-x for x in reversed(self.params['word'])]}
        elif fam == 'plane':
            p['s'] = q5_json(-q5_from_json(self.params['s']))
        elif fam == 'cayley':
            p['omega'] = [q5_json(-q5_from_json(x)) for x in self.params['omega']]
            if 'requested' in p:
                p['requested'] = dict(p['requested'], requested_angle_deg=-p['requested']['requested_angle_deg'])
        elif fam == 'half_turn':
            pass  # this exact matrix is symmetric, hence its own inverse
        elif fam == 'menu':
            p = {'menu': self.params.get('menu'), 'label': '(%s)^-1' % self.params.get('label')}
        kinv = int(self.ctx.kplus.inv[self.kidx]) if self.kidx >= 0 else -1
        return Twist(self.ctx, self.grip, transpose(self.matrix), fam, p, kidx=kinv)

    def record(self):
        return {'grip': self.grip, 'family': self.family, 'params': self.params,
                'matrix': matrix_json(self.matrix), 'retained': self.retained,
                'angle_deg': round(self.angle_deg(), 9)}

    @classmethod
    def from_record(cls, ctx, rec):
        """Rebuild a twist from a journal record; the exact matrix is authoritative and the
        family parameters must reproduce it exactly."""
        grip = int(rec['grip'])
        m = matrix_from_json(rec['matrix'])
        tw = cls(ctx, grip, m, rec.get('family', 'matrix'), rec.get('params', {}))
        fam, p = tw.family, tw.params
        if fam == 'retained':
            if generator_word_matrix(ctx.kplus, p['word']) != m or any((abs(x) - 1) // 2 != grip for x in p['word']):
                raise TwistError('retained record: word does not reproduce the matrix')
        elif fam == 'plane':
            if plane_matrix(ctx.data.N[grip], ctx.data.N[int(p['second_pole'])], q5_from_json(p['s']))[0] != m:
                raise TwistError('plane record: parameter does not reproduce the matrix')
        elif fam == 'cayley':
            om = [q5_from_json(x) for x in p['omega']]
            if len(om) != 3 or cayley_matrix(ctx.data.N[grip], om, ctx.data.NN) != m:
                raise TwistError('cayley record: parameter does not reproduce the matrix')
        elif fam == 'half_turn':
            axis = [q5_from_json(x) for x in p['axis']]
            if half_turn_matrix(ctx.data.N[grip], axis) != m:
                raise TwistError('half-turn record: axis does not reproduce the matrix')
        if bool(rec.get('retained', tw.retained)) != tw.retained:
            raise TwistError('record retained flag disagrees with the matrix')
        return tw

    def __repr__(self):
        return f'Twist(grip={self.grip}, family={self.family}, retained={self.retained}, angle={self.angle_deg():.6f})'


class UnrepresentableTwist:
    """A requested rotation with no exact representation. The contract allows rigorous interval
    bounds or rejection; this simulator rejects it as uncertain, leaving the state unchanged."""
    __slots__ = ('grip', 'float_matrix', 'reason')

    def __init__(self, grip, float_matrix, reason='rotation has no exact Q(sqrt5) representation'):
        self.grip = int(grip)
        self.float_matrix = np.asarray(float_matrix, float)
        self.reason = reason


# ---------------------------------------------------------------------------------------------
# constructors

def primitive(ctx, mid):
    """Retained twist for a signed 1-based primitive id (generator k is id k + 1)."""
    k = abs(int(mid)) - 1
    if not 0 <= k < 1200 or mid == 0:
        raise TwistError('primitive id must be in +-1..+-1200')
    kp = ctx.kplus
    idx = int(kp.gen_idx[k])
    if mid < 0:
        idx = int(kp.inv[idx])
    return Twist(ctx, k // 2, kp.matrix(idx), 'retained', {'word': [int(mid)]}, kidx=idx)


def generator(ctx, k):
    return primitive(ctx, k + 1)


def a4_element(ctx, c, i):
    """Element i (0..11, 0 = identity) of A4_c as a retained twist with its generator word."""
    kp = ctx.kplus
    k = kp.a4(c)[i]
    return Twist(ctx, c, kp.matrix(k), 'retained', {'word': kp.a4_word(c, k)}, kidx=k)


def plane(ctx, c, d, s=None, degrees=None, max_den=1000):
    """Plane rotation fixing n_c and n_d (witness E2 construction). Give an exact s, or degrees,
    in which case s is chosen exactly as witness.witness_rotation does."""
    a, b = ctx.data.N[c], ctx.data.N[d]
    if s is None:
        if degrees is None:
            raise TwistError('give s or degrees')
        gram_m = float(dot(a, a) * dot(b, b) - dot(a, b) * dot(a, b))
        sf = Fraction(math.tan(math.radians(degrees) / 2) / math.sqrt(gram_m)).limit_denominator(max_den)
        s = Q5(sf.numerator, 0, sf.denominator)
    s = Q5.of(s)
    if s.is_zero():
        raise TwistError('the plane parameter is zero (angle too small for max_den)')
    m, _ = plane_matrix(a, b, s)
    return Twist(ctx, c, m, 'plane', {'second_pole': int(d), 's': q5_json(s)})


def cayley(ctx, c, omega):
    om = [Q5.of(x) for x in omega]
    if len(om) != 3:
        raise TwistError('omega needs three components')
    return Twist(ctx, c, cayley_matrix(ctx.data.N[c], om, ctx.data.NN), 'cayley',
                 {'omega': [q5_json(x) for x in om]})


def half_turn(ctx, c, axis):
    """Exact half-turn about an axis in cap_frame(n_c), with coordinates in Q(sqrt5)."""
    axis = [Q5.of(x) for x in axis]
    return Twist(ctx, c, half_turn_matrix(ctx.data.N[c], axis), 'half_turn',
                 {'axis': [q5_json(x) for x in axis]})


def cayley_axis_angle(ctx, c, axis, degrees, max_den=1000, frame='cap', max_num=16, menu=None):
    """A1's global minimum over bounded rational Cayley parameters and near-pi half-turns.

    frame='cap': axis in coordinates of cap_frame(n_c) (orthonormal after dividing by |n|);
    frame='world': a 4-vector, projected onto n_c-perp. Returns (twist, report) where the
    report gives the realised angle, the axis error and the rotation error (floats, display)."""
    if frame == 'world':
        v = np.asarray(axis, float)
        if v.shape != (4,) or not np.all(np.isfinite(v)):
            raise TwistError('world axis must be four finite numbers')
        if np.any(v):
            v = v / np.max(np.abs(v))
        n = ctx.data.NF[c]
        u = np.array([[float(x) for x in row] for row in cap_frame(ctx.data.N[c])])
        axis = (u @ v) / float(np.linalg.norm(n))
    elif frame != 'cap':
        raise TwistError("frame must be 'cap' or 'world'")
    num, q, report = nearest_parameter(axis, degrees, max_den, max_num)
    params = [Q5(x, 0, q) for x in num]
    tw = half_turn(ctx, c, params) if report['family'] == 'half_turn' else cayley(ctx, c, params)
    tw.params['requested'] = {k: report[k] for k in ('requested_angle_deg', 'requested_axis', 'N', 'D', 'distance')}
    if menu is not None and (report['distance'] > INPUT_TIE or not menu.contains(tw)):
        raise TwistError('requested rotation is not an exact menu element')
    report['exact_angle_check_deg'] = tw.angle_deg()
    return tw, report


# ---------------------------------------------------------------------------------------------
# twist menus (amendment A4)

class TwistMenu:
    """A twist menu: a set of exact rotations of the base cap 0, transported to cap c by its
    retained frame, Lambda_c = F_c Lambda_0 F_c^-1. The menu is K+-invariant (independent of the
    frame chosen for each cap) exactly when Lambda_0 is closed under conjugation by A4_0;
    `close=True` includes A4_0 and closes it under inverses and conjugation. With `close=False`
    all three requirements are checked and violations are refused. The A4 menu has 12 elements,
    including the identity, and gives the retained puzzle 600-cell-Full."""

    def __init__(self, ctx, name, base, close=True):
        self.ctx = ctx
        self.name = name
        n0 = ctx.data.N[0]
        items = []
        keys = {}
        for label, m in base:
            m = to_tuple(m)
            if not is_rotation(m) or matvec(m, n0) != list(n0):
                raise TwistError(f'menu element {label} is not an exact rotation fixing n_0')
            if m not in keys:
                keys[m] = label
                items.append((label, m))
        kp = ctx.kplus
        stab = [kp.matrix(s) for s in kp.stab0]
        if close:
            for j, s in enumerate(stab):
                if s not in keys:
                    keys[s] = f'a4[{j}]'
                    items.append((keys[s], s))
            i = 0
            while i < len(items):
                label, m = items[i]
                inv = to_tuple(transpose(m))
                if inv not in keys:
                    keys[inv] = f'({label})^-1'
                    items.append((keys[inv], inv))
                for j, s in enumerate(stab):
                    c = to_tuple(matmul(matmul(s, m), transpose(s)))
                    if c not in keys:
                        keys[c] = f'{label}^{j}'
                        items.append((keys[c], c))
                i += 1
        self.items = items
        self._keys = keys
        self.inverse_closed = all(to_tuple(transpose(m)) in keys for _, m in items)
        self.a4_invariant = all(to_tuple(matmul(matmul(s, m), transpose(s))) in keys for _, m in items for s in stab)
        self.contains_a4 = all(s in keys for s in stab)
        self.invariant = self.a4_invariant  # compatibility with the original J1 API
        if not (self.inverse_closed and self.a4_invariant and self.contains_a4):
            raise TwistError('menu must be inverse-closed, A4-conjugation-invariant and contain A4')
        canonical = sorted(json.dumps(matrix_json(m), separators=(',', ':')) for _, m in items)
        self.identity = hashlib.sha256(('[' + ','.join(canonical) + ']').encode()).hexdigest()

    def __len__(self):
        return len(self.items)

    def frame(self, c):
        return self.ctx.kplus.matrix(int(self.ctx.kplus.frame_idx[c]))

    def for_cap(self, c, transporter=None):
        """Transport by F_c, or a supplied K+ index taking n_0 to n_c."""
        if transporter is None:
            f = self.frame(c)
        else:
            kp = self.ctx.kplus
            transporter = int(transporter)
            if not 0 <= transporter < len(kp.perms) or int(kp.perms[transporter, 0]) != c:
                raise TwistError('menu transporter must take pole 0 to this cap')
            f = kp.matrix(transporter)
        out = []
        for label, m in self.items:
            g = to_tuple(matmul(matmul(f, m), transpose(f)))
            out.append(Twist(self.ctx, c, g, 'menu', {'menu': self.name, 'label': label}))
        return out

    def contains(self, twist):
        f = self.frame(twist.grip)
        return to_tuple(matmul(matmul(transpose(f), twist.matrix), f)) in self._keys

    def record(self):
        return {'name': self.name, 'invariant': self.invariant, 'inverse_closed': self.inverse_closed,
                'a4_invariant': self.a4_invariant, 'contains_a4': self.contains_a4, 'identity': self.identity,
                'base': [{'label': lab, 'matrix': matrix_json(m)} for lab, m in self.items]}

    @classmethod
    def a4(cls, ctx):
        kp = ctx.kplus
        return cls(ctx, 'A4', [(f'a4[{i}]', kp.matrix(s)) for i, s in enumerate(kp.stab0)], close=False)

    @classmethod
    def s4(cls, ctx):
        """The exact 24-element octahedral group, generated by A4 and the six Cayley quarters."""
        generators = [ctx.kplus.matrix(s) for s in ctx.kplus.stab0]
        for i in range(3):
            for sign in (-1, 1):
                w = [ZERO, ZERO, ZERO]
                w[i] = Q5(sign)
                generators.append(cayley_matrix(ctx.data.N[0], w, ctx.data.NN))
        elems = [ctx.kplus.matrix(0)]
        seen = set(elems)
        for m in elems:
            for g in generators:
                product = to_tuple(matmul(g, m))
                if product not in seen:
                    seen.add(product)
                    elems.append(product)
                    if len(elems) > 24:
                        raise TwistError('S4 quarters did not generate a 24-element group')
        if len(elems) != 24:
            raise TwistError('S4 must contain exactly 24 elements')
        return cls(ctx, 'S4', [(f's4[{i}]', m) for i, m in enumerate(elems)], close=False)

    @classmethod
    def cayley_set(cls, ctx, name, omegas, close=True):
        n0 = ctx.data.N[0]
        base = [(f'w{i}', cayley_matrix(n0, [Q5.of(x) for x in w], ctx.data.NN)) for i, w in enumerate(omegas)]
        return cls(ctx, name, base, close=close)

    @classmethod
    def from_record(cls, ctx, rec):
        menu = cls(ctx, rec['name'], [(b['label'], matrix_from_json(b['matrix'])) for b in rec['base']], close=False)
        for field in ('inverse_closed', 'a4_invariant', 'contains_a4', 'identity', 'invariant'):
            if field in rec and rec[field] != getattr(menu, field):
                raise TwistError(f'menu record {field} disagrees with exact matrices')
        return menu
