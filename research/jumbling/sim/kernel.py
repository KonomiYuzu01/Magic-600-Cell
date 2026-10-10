"""Exact sign kernel over Q(sqrt 5) in integer form, and the optional filtered sign test (A2).

Every admissibility decision reduces to the sign of h_e(x) = n_e . x / |n|^2 - alpha at an
exact point x. All poles have the same |n|^2 = 12 + 4 sqrt5, so the sign of h_e(x) is the sign
of y . v - kappa with kappa = alpha |n|^2 = (1452 + 484 sqrt5) / 125, where v is a vertex in
the piece's home frame and y = G^T n_e is the cut normal pulled back by the pose G.

Integer form. A vector of Q(sqrt5) numbers is stored as (A, B, D) with integer tuples A, B and an
integer D > 0, meaning (A_i + B_i sqrt5) / D. Signs only need the numerators, because every
denominator is positive. Nothing in the exact path uses floating point.

Filtered sign test (amendment A2, OFF by default). `filtered_signs` evaluates y . v - kappa in
binary64 and decides the sign only where |S^| exceeds a proven forward error bound; every other
entry is decided by the exact kernel. Derivation, with u = 2^-53 and the standard model
fl(a op b) = (a op b)(1 + d), |d| <= u, for normal-range results:

1. Conversion. x = (a + b sqrt5)/d is converted as fl(fl(fl(a) + fl(fl(b) s)) / fl(d)) with
   s = fl(sqrt5). The sqrt5 summand carries six rounding factors (fl(b), s, the product, the
   sum, the division and fl(d) in the denominator), the other summand four, so
   |x^ - x| <= ((1+u)^5/(1-u) - 1) m(x) <= 6.01u m(x), where m(x) = (|a| + sqrt5 |b|)/d is the
   magnitude of x (m(x) >= |x|; it is much larger than |x| when a + b sqrt5 cancels).
2. Products. |w^ x^ - w x| <= |w^ - w| |x^| + |w| |x^ - x| <= 12.03u m(w) m(x).
3. Inner product. The float value of sum_{i<=4} w^_i x^_i - kappa^ (any summation order, with or
   without fused multiply-add) differs from its exact value by at most
   gamma_5 (sum |w^_i x^_i| + |kappa^|), gamma_5 = 5u/(1-5u) (Higham, Accuracy and Stability of
   Numerical Algorithms, 2nd ed., eq. 3.5), and |w^_i x^_i| <= (1 + 6.01u)^2 m(w_i) m(x_i).
4. Total. |S^ - S| <= (12.03u + gamma_5 (1 + 6.01u)^2) (sum m(w_i) m(x_i) + m(kappa))
   <= 17.1u (sum m(w_i) m(x_i) + m(kappa)).
5. Computed magnitudes. m^ is a float evaluation of nonnegative terms, so m <= (1 + 6.02u) m^,
   and the computed bound sum B^ = fl(sum m^(w_i) m^(x_i) + m^(kappa)) satisfies
   sum m(w_i) m(x_i) + m(kappa) <= (1 + 17.1u) B^. Hence |S^ - S| <= 17.2u B^.
6. Range. Nonzero magnitudes are kept in [2^-500, 2^500], so no product or sum overflows and every
   magnitude product is a normal number. Where a value itself underflows, each of the at most
   ten operations adds an absolute error below 2^-1074, covered by the margin 2^-1000.

The test decides sign(S) = sign(S^) whenever |S^| > 2^-48 B^ + 2^-1000; the threshold is at least
31.9u B^, which exceeds the error bound. Inputs outside the window, or not convertible to finite
floats, are never filtered: their signs come from the exact kernel.
"""
from fractions import Fraction
from math import gcd, isfinite

import numpy as np

from exact import ZERO

KAPPA_FORM = (1452, 484, 125)  # alpha |n|^2 = (121/125)(12 + 4 sqrt5) = (1452 + 484 sqrt5)/125
S5 = 5 ** 0.5
FILTER_REL = 2.0 ** -48
FILTER_ABS = 2.0 ** -1000
WINDOW_LO = 2.0 ** -500
WINDOW_HI = 2.0 ** 500
ARITHMETIC_VERSION = 'Q5-integer-sign-1 / binary64'
ERROR_BOUND_VERSION = 'Q5-forward-error-1 (2^-48 magnitude + 2^-1000)'


def lcm(a, b):
    return a // gcd(a, b) * b


def q5_float(x):
    """Overflow-safe, cancellation-free display value; None only for a non-finite result.

    Keep ratios rational until the final conversion, so large coefficients/denominators do
    not overflow on their own. Opposite signs use the conjugate, whose denominator cannot
    cancel. This approximation is never used by the exact kernel or the A2 filter.
    """
    a, b, d = x.a, x.b, x.d
    root = Fraction(S5)
    try:
        if a == 0 or b == 0 or (a > 0) == (b > 0):
            value = float((Fraction(a) + b * root) / d)
        else:
            value = float(Fraction(a * a - 5 * b * b) / (d * (a - b * root)))
    except OverflowError:
        return None
    return value if isfinite(value) else None


def sign5(x, y):
    """Sign of x + y sqrt5 for Python integers x and y."""
    if y == 0:
        return (x > 0) - (x < 0)
    if x == 0:
        return (y > 0) - (y < 0)
    if (x > 0) == (y > 0):
        return 1 if x > 0 else -1
    t = x * x - 5 * y * y
    s = (t > 0) - (t < 0)
    return s if x > 0 else -s


def sign5_vec(x, y):
    """Vectorised sign5 for int64 arrays whose squares fit in int64 (callers check the range)."""
    x = np.asarray(x, dtype=np.int64)
    y = np.asarray(y, dtype=np.int64)
    sx, sy = np.sign(x), np.sign(y)
    st = np.sign(x * x - 5 * y * y)
    return np.where(sy == 0, sx, np.where(sx == 0, sy, np.where(sx == sy, sx, st * sx))).astype(np.int64)


def int_form(vec):
    """(A, B, D) with vec_i = (A_i + B_i sqrt5)/D for a sequence of Q5 numbers."""
    d = 1
    for x in vec:
        d = lcm(d, x.d)
    return (tuple(x.a * (d // x.d) for x in vec), tuple(x.b * (d // x.d) for x in vec), d)


def rows_form(points):
    """Integer form of a list of exact points with one common positive denominator.

    Returns (E, rows) with rows[j] = (x0, x1, x2, x3, z0, z1, z2, z3) meaning
    point_j = (x + z sqrt5) / E."""
    e = 1
    for p in points:
        for x in p:
            e = lcm(e, x.d)
    rows = []
    for p in points:
        xs = tuple(x.a * (e // x.d) for x in p)
        zs = tuple(x.b * (e // x.d) for x in p)
        rows.append(xs + zs)
    return e, rows


def pullback_form(g, n):
    """Integer form of y = G^T n for an exact 4x4 matrix G (rows of Q5) and an exact vector n."""
    return int_form([sum((g[i][j] * n[i] for i in range(4)), ZERO) for j in range(4)])


class Evaluator:
    """Signs of y . v - kappa for one pulled-back normal y over vertex row sets."""
    __slots__ = ('a0', 'a1', 'a2', 'a3', 'b0', 'b1', 'b2', 'b3', 'dy', 'yform', '_float')

    def __init__(self, yform):
        (a0, a1, a2, a3), (b0, b1, b2, b3), dy = yform
        k3 = KAPPA_FORM[2]
        self.a0, self.a1, self.a2, self.a3 = a0 * k3, a1 * k3, a2 * k3, a3 * k3
        self.b0, self.b1, self.b2, self.b3 = b0 * k3, b1 * k3, b2 * k3, b3 * k3
        self.dy = dy
        self.yform = yform
        self._float = None

    def sign_row(self, e, row):
        x0, x1, x2, x3, z0, z1, z2, z3 = row
        p = (self.a0 * x0 + self.a1 * x1 + self.a2 * x2 + self.a3 * x3
             + 5 * (self.b0 * z0 + self.b1 * z1 + self.b2 * z2 + self.b3 * z3))
        q = (self.a0 * z0 + self.a1 * z1 + self.a2 * z2 + self.a3 * z3
             + self.b0 * x0 + self.b1 * x1 + self.b2 * x2 + self.b3 * x3)
        de = self.dy * e
        return sign5(p - de * KAPPA_FORM[0], q - de * KAPPA_FORM[1])

    def classify(self, vform, budget=None):
        """Classify one vertex set: ('in', n), ('out', n) or ('straddle', n, j_below, j_above).

        n is the number of exact evaluations used. With a budget, returns ('uncertain', n) when
        the budget runs out before a certificate is complete."""
        e, rows = vform
        neg = pos = None
        n = 0
        for j, row in enumerate(rows):
            if budget is not None and n >= budget:
                return ('uncertain', n)
            s = self.sign_row(e, row)
            n += 1
            if s < 0:
                if neg is None:
                    neg = j
                if pos is not None:
                    return ('straddle', n, neg, pos)
            elif s > 0:
                if pos is None:
                    pos = j
                if neg is not None:
                    return ('straddle', n, neg, pos)
        if neg is None:
            return ('in', n)
        if pos is None:
            return ('out', n)
        raise AssertionError('unreachable')

    # -- A2 filter ------------------------------------------------------------------------
    def float_form(self):
        """(y^, m(y)) as float arrays, or None when the inputs leave the filter window."""
        if self._float is None:
            try:
                a, b, d = self.yform
                fd = float(d)
                yf = np.array([(float(x) + float(z) * S5) / fd for x, z in zip(a, b)])
                my = np.array([(abs(float(x)) + S5 * abs(float(z))) / fd for x, z in zip(a, b)])
            except OverflowError:
                self._float = False
                return None
            nz = my[my > 0]
            if (not np.all(np.isfinite(yf)) or not np.all(np.isfinite(my))
                    or (nz.size and (nz.min() < WINDOW_LO or nz.max() > WINDOW_HI))):
                self._float = False
            else:
                self._float = (yf, my)
        return self._float or None


KAPPA_FLOAT = (KAPPA_FORM[0] + KAPPA_FORM[1] * S5) / KAPPA_FORM[2]
KAPPA_MAG = (abs(KAPPA_FORM[0]) + S5 * abs(KAPPA_FORM[1])) / KAPPA_FORM[2]


def float_rows(vform):
    """(x^, m(x)) float arrays of shape (n, 4) for an integer-form vertex set, or None when the
    integers do not convert to finite floats (the filter is then not used)."""
    e, rows = vform
    try:
        arr = np.array(rows, dtype=object)
        x = np.array(arr[:, :4].tolist(), dtype=np.float64)
        z = np.array(arr[:, 4:].tolist(), dtype=np.float64)
        fe = float(e)
    except OverflowError:
        return None
    return (x + z * S5) / fe, (np.abs(x) + S5 * np.abs(z)) / fe


def _in_window(m):
    nz = m[m > 0]
    return bool(np.all(np.isfinite(m)) and (nz.size == 0 or (nz.min() >= WINDOW_LO and nz.max() <= WINDOW_HI)))


def filtered_signs(ev, vform, frows, budget=None, record=None):
    """Signs of y . v - kappa for every vertex: float where the bound decides, exact elsewhere.

    With a list in `record`, append each vertex's enclosure and accepted sign or exact
    fallback. State.classify supplies the pose, identities and coverage of those inputs.
    Returns (signs list, number decided by the filter, number decided exactly), or None when
    more than `budget` exact evaluations would be needed (the caller treats that as uncertain)."""
    ff = ev.float_form()
    e, rows = vform
    if ff is None or frows is None or not _in_window(frows[1]):
        if budget is not None and len(rows) > budget:
            return None
        out = [ev.sign_row(e, r) for r in rows]
        if record is not None:
            for j, sign in enumerate(out):
                record.append({'vertex': j, 'method': 'exact_fallback', 'enclosure': None,
                               'accepted_sign': sign, 'fallback_reason': 'input_outside_filter_window',
                               'arithmetic_version': ARITHMETIC_VERSION,
                               'error_bound_version': ERROR_BOUND_VERSION})
        return out, 0, len(rows)
    yf, my = ff
    xf, mx = frows
    with np.errstate(over='ignore', invalid='ignore'):
        s = xf @ yf - KAPPA_FLOAT
        bound = (mx @ my + KAPPA_MAG) * FILTER_REL + FILTER_ABS
        lo = np.nextafter(s - bound, -np.inf)
        hi = np.nextafter(s + bound, np.inf)
    finite = np.isfinite(s) & np.isfinite(bound) & np.isfinite(lo) & np.isfinite(hi)
    decided = finite & ((lo > 0) | (hi < 0))
    exact_n = int((~decided).sum())
    if budget is not None and exact_n > budget:
        return None
    out = [(1 if s[j] > 0 else -1) if decided[j] else ev.sign_row(e, row) for j, row in enumerate(rows)]
    if record is not None:
        for j, sign in enumerate(out):
            rec = {'vertex': j, 'method': 'filtered' if decided[j] else 'exact_fallback',
                   'enclosure': [float(lo[j]), float(hi[j])] if finite[j] else None,
                   'accepted_sign': sign, 'arithmetic_version': ARITHMETIC_VERSION,
                   'error_bound_version': ERROR_BOUND_VERSION}
            if not decided[j]:
                rec['fallback_reason'] = 'enclosure_contains_zero' if finite[j] else 'non_finite_evaluation'
            record.append(rec)
    return out, len(rows) - exact_n, exact_n


def classify_signs(signs):
    """Class of a vertex set from its signs: ('in', n), ('out', n) or ('straddle', n, j-, j+)."""
    neg = next((j for j, s in enumerate(signs) if s < 0), None)
    pos = next((j for j, s in enumerate(signs) if s > 0), None)
    if neg is None:
        return ('in', len(signs))
    if pos is None:
        return ('out', len(signs))
    return ('straddle', len(signs), neg, pos)
