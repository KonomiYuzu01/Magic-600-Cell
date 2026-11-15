"""Primary A1 small-grid certificates, scored as A+B*cos(t)+C*sin(t)."""
from fractions import Fraction as Q
from itertools import product

import intervals as iv
from cyclotomic import exact_linear, exceptional_orders


def candidates(n=1, dmax=16, half=False):
    rows = [(d, x, y, z, 'C') for d in range(1, dmax + 1)
            for x, y, z in product(range(-n, n + 1), repeat=3)]
    if half:
        rows += [(1, x, y, z, 'H') for x, y, z in product(range(-n, n + 1), repeat=3)
                 if x or y or z]
    return sorted(rows)


def coeff(row, axis=(1, 0, 0)):
    # This fixture path uses a signed unit coordinate axis (q=1).
    den, *p, family = row
    d = den if family == 'C' else 0
    norm = d * d + sum(x * x for x in p)
    projection = sum(x * a for x, a in zip(p, axis))
    return (Q(d * d + projection * projection, 2 * norm),
            Q(d * d - projection * projection, 2 * norm), Q(d * projection, norm))


def subtract(x, y):
    return tuple(a - b for a, b in zip(x, y))


def choose(degrees, axis=(1, 0, 0), radian=None):
    assert degrees.denominator & (degrees.denominator - 1) == 0
    assert sum(x*x for x in axis) == 1 and all(x in (-1, 0, 1) for x in axis)
    reduced = (degrees + 180) % 360 - 180
    rows = candidates(half=abs(abs(reduced) - 180) <= 1)
    cs = {row: coeff(row, axis) for row in rows}
    cache, precision = {}, 0

    def compare(x, y):
        nonlocal precision
        delta = subtract(cs[x], cs[y])
        if not any(delta):
            return 0
        turns = degrees / 360
        if (radian is None and turns.denominator in exceptional_orders()
                and not any(exact_linear(delta, turns))):
            return 0
        for bits in (16, 32, 64, 128, 256, 512):
            precision = max(precision, bits)
            if bits not in cache:
                cache[bits] = (iv.degree_trig(degrees, bits) if radian is None
                               else iv.trig((radian, radian), bits))
            answer = iv.sign(iv.linear(delta, *cache[bits]))
            if answer:
                return answer
        raise ArithmeticError('fixture refinement cap reached; no certified result')

    best = rows[0]
    for row in rows[1:]:
        if compare(row, best) > 0:
            best = row
    ties = [row for row in rows if compare(row, best) == 0]
    assert best == min(ties)
    return best, ties, precision


def case_certificate(degrees, radian, bits=96):
    result = {}
    for name, angle in (('exact_degrees', iv.scale(iv.pi(bits + 20), degrees / 180)),
                        ('binary64_radians', (radian, radian))):
        sine, cosine = iv.trig(angle, bits + 12)
        delta = iv.linear((Q(0), Q(-3, 10), Q(1, 10)), sine, cosine)
        assert iv.sign(delta) == 1
        result[name] = {'radians': iv.text(iv.outward(angle, bits)),
                        'sin': iv.text(iv.outward(sine, bits)),
                        'cos': iv.text(iv.outward(cosine, bits)),
                        'score_1_minus_2': iv.text(iv.outward(delta, bits)), 'sign': 1}
    result['pi'] = iv.text(iv.outward(iv.pi(bits + 20), bits))
    result['interval_bits'] = bits
    return result
