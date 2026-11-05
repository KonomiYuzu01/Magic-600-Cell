"""Second computation: different pi identity, alternating trig, quaternion scores.

No imports from certify.py or intervals.py. Cyclotomic polynomial arithmetic is
shared, but scores are constructed by squaring half-angle quaternion products,
not by the primary full-angle coefficient formula.
"""
from fractions import Fraction as Q
from functools import lru_cache
from itertools import product
from math import gcd, lcm

from cyclotomic import Field


def rounded(bounds, bits):
    a, b = bounds
    scale = 2 ** bits
    lo = a.numerator * scale // a.denominator
    hi = -((-b.numerator * scale) // b.denominator)
    return Q(lo, scale), Q(hi, scale)


def times(x, y):
    values = (x[0] * y[0], x[0] * y[1], x[1] * y[0], x[1] * y[1])
    return min(values), max(values)


def plus(x, y):
    return x[0] + y[0], x[1] + y[1]


def factor(x, v):
    return times(x, (v, v))


@lru_cache(None)
def other_pi(bits):
    ends = []
    for n in (2, 3):
        terms, j = [], 0
        while True:
            term = Q((-1) ** j, (2 * j + 1) * n ** (2 * j + 1))
            if abs(term) < Q(1, 2 ** (bits + 8)):
                partial = sum(terms, Q(0))
                ends.append((min(partial, partial + term), max(partial, partial + term)))
                break
            terms.append(term)
            j += 1
    # atan(1/2)+atan(1/3)=pi/4 by the addition formula and quadrant.
    return rounded(factor(plus(*ends), 4), bits)


def other_trig(bounds, bits):
    centre = sum(bounds) / 2
    radius = (bounds[1] - bounds[0]) / 2
    x, negative = abs(centre), centre < 0
    answers = []
    for parity in (1, 0):
        term, total, k = (x if parity else Q(1)), Q(0), parity
        while True:
            total += term
            term = -term * x * x / ((k + 1) * (k + 2))
            k += 2
            if abs(term) < Q(1, 2 ** (bits + 8)) and k > x:
                lo, hi = min(total, total + term), max(total, total + term)
                break
        if parity and negative:
            lo, hi = -hi, -lo
        answers.append(rounded((lo - radius, hi + radius), bits))
    return tuple(answers)


@lru_cache(None)
def angle_trig(turns, bits):
    turns = (turns + Q(1, 2)) % 1 - Q(1, 2)
    return other_trig(factor(other_pi(bits + 16), 2 * turns), bits)


def square(x):
    lo, hi = x
    return (Q(0) if lo <= 0 <= hi else min(lo * lo, hi * hi), max(lo * lo, hi * hi))


def case_certificate(degrees, radian, bits=96):
    result = {}
    for name, angle in (('exact_degrees', factor(other_pi(bits + 24), degrees / 180)),
                        ('binary64_radians', (radian, radian))):
        s, c = other_trig(factor(angle, Q(1, 2)), bits + 16)
        sine = factor(times(s, c), 2)
        cosine = plus(square(c), factor(square(s), -1))
        score1 = factor(square(plus(c, s)), Q(1, 2))
        score2 = factor(square(plus(factor(c, 2), s)), Q(1, 5))
        delta = plus(score1, factor(score2, -1))
        assert delta[0] > 0
        show = lambda x: list(map(str, rounded(x, bits)))
        result[name] = {'radians': show(angle), 'sin': show(sine), 'cos': show(cosine),
                        'score_1_minus_2': show(delta), 'sign': 1}
    result['pi'] = list(map(str, rounded(other_pi(bits + 24), bits)))
    result['interval_bits'] = bits
    return result


def rows(half=False):
    # Independent loop order; final order is A1's denominator/numerator/family.
    choices = []
    for p in product((-1, 0, 1), repeat=3):
        for d in range(16, 0, -1):
            choices.append((d, *p, 'C'))
        if half and p != (0, 0, 0):
            choices.append((1, *p, 'H'))
    return sorted(choices)


def verify_case_global(degrees, radian, axis, winner):
    """Certify winner against all 432 tuples using half-angle interval squares."""
    largest = 0
    for mode in ('exact_degrees', 'binary64_radians'):
        for bits in (16, 32, 64, 128, 256):
            angle = (factor(other_pi(bits + 16), degrees / 360) if mode == 'exact_degrees'
                     else (radian / 2, radian / 2))
            s, c = other_trig(angle, bits + 8)

            def score(row):
                d, *p, _ = row
                dot = plus(factor(c, d), factor(s, sum(x * a for x, a in zip(p, axis))))
                return factor(square(dot), Q(1, d * d + sum(x * x for x in p)))

            best = score(winner)
            if all(best[0] > score(row)[1] for row in rows() if row != winner):
                largest = max(largest, bits)
                break
        else:
            raise ArithmeticError('independent global certificate failed')
    return largest


def brute_exact(turns):
    """All candidates scored in the half-angle cyclotomic field; no pruning."""
    half_turns = turns / 2
    m = lcm(half_turns.denominator, 4)
    f = Field(m)
    s, c = f.trig(half_turns.numerator * (m // half_turns.denominator))
    choices = rows(half=turns == Q(1, 2))
    scores = []
    for d, x, y, z, family in choices:
        if family == 'H':
            d = 0
        scalar = f.add(f.scale(c, d), f.scale(s, x))
        scores.append(f.scale(f.mul(scalar, scalar), Q(1, d * d + x * x + y * y + z * z)))
    precision = 0

    def compare(a, b):
        nonlocal precision
        p = f.sub(a, b)
        if not any(p):
            return 0
        for bits in (16, 32, 64, 128, 256, 512):
            precision = max(precision, bits)
            enclosure = (Q(0), Q(0))
            for j, v in enumerate(p):
                if v:
                    enclosure = plus(enclosure, factor(angle_trig(Q(j, m), bits)[1], v))
            if enclosure[0] > 0:
                return 1
            if enclosure[1] < 0:
                return -1
        raise ArithmeticError('independent algebraic sign unresolved')

    index = 0
    for j in range(1, len(choices)):
        if compare(scores[j], scores[index]) > 0:
            index = j
    ties = [row for row, score in zip(choices, scores) if score == scores[index]]
    assert choices[index] == min(ties)
    return choices[index], ties, precision


def other_exceptional_angles():
    # Independent finite enumeration: the bound derived in PROOF.md is n<=1440.
    out, orders = [], []
    for n in range(1, 1441):
        if 1440 % n == 0 and sum(gcd(k, n) == 1 for k in range(1, n + 1)) <= 16:
            orders.append(n)
            out += [Q(k, n) for k in range(n) if gcd(k, n) == 1]
    return orders, sorted(out)
