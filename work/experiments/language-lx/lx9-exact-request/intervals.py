"""Rational certificates: Machin pi, Taylor/Lagrange trig, outward dyadic bounds.

No floating arithmetic. An interval is a pair of Fractions. Precision parameters
bound analytic remainders, not machine arithmetic (which is exact).
"""
from fractions import Fraction as Q
from functools import lru_cache


def add(x, y):
    return x[0] + y[0], x[1] + y[1]


def scale(x, a):
    return (x[0] * a, x[1] * a) if a >= 0 else (x[1] * a, x[0] * a)


def mul(x, y):
    ends = [a * b for a in x for b in y]
    return min(ends), max(ends)


def outward(x, bits):
    unit = 1 << bits
    return Q(x[0] * unit // 1, unit), Q(-(-x[1] * unit // 1), unit)


def atan_inverse(n, bits):
    # Alternating decreasing series; the next term encloses the entire tail.
    s, k, power = Q(0), 0, Q(1, n)
    while True:
        s += (-1) ** k * power / (2 * k + 1)
        k += 1
        power /= n * n
        tail = (-1) ** k * power / (2 * k + 1)
        if abs(tail) < Q(1, 1 << bits):
            return min(s, s + tail), max(s, s + tail)


@lru_cache(None)
def pi(bits):
    # pi = 16 atan(1/5) - 4 atan(1/239).
    return outward(add(scale(atan_inverse(5, bits + 8), 16),
                        scale(atan_inverse(239, bits + 8), -4)), bits)


def trig_point(x, bits, sine):
    # Degree-m Maclaurin polynomial, |remainder| <= |x|^(m+1)/(m+1)!.
    # All derivatives of real sin/cos have magnitude <= 1.
    term, total, degree = Q(1), Q(0), 0
    while True:
        if degree % 2 == int(sine):
            total += (-1) ** (degree // 2) * term
        degree += 1
        term *= x / degree
        if abs(term) < Q(1, 1 << (bits + 4)) and degree > 2 * abs(x):
            return total - abs(term), total + abs(term)


def trig(x, bits):
    mid, radius = (x[0] + x[1]) / 2, (x[1] - x[0]) / 2
    # Both functions are 1-Lipschitz, so propagate argument uncertainty.
    return tuple(outward(add(trig_point(mid, bits + 4, sine), (-radius, radius)), bits)
                 for sine in (True, False))


def degree_trig(degrees, bits):
    d = (degrees + 180) % 360 - 180
    return trig(scale(pi(bits + 12), d / 180), bits)


def linear(coeff, sine, cosine):
    a, b, c = coeff
    return add((a, a), add(scale(cosine, b), scale(sine, c)))


def sign(x):
    return 1 if x[0] > 0 else -1 if x[1] < 0 else 0


def text(x):
    return [str(x[0]), str(x[1])]


def binary64(hex_bits):
    """Decode a finite IEEE binary64 bit string using integers only."""
    if len(hex_bits) != 16 or any(c not in '0123456789abcdef' for c in hex_bits):
        raise ValueError('expected 16 lowercase hexadecimal digits')
    u = int(hex_bits, 16)
    e, m = (u >> 52) & 2047, u & ((1 << 52) - 1)
    if e == 2047:
        raise ValueError('non-finite binary64')
    if e:
        m += 1 << 52
    shift = (e - 1023 - 52) if e else -1074
    value = Q(m << shift) if shift >= 0 else Q(m, 1 << -shift)
    return -value if u >> 63 else value
