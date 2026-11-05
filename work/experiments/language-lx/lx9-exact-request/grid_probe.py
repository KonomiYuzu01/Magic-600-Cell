"""Default-grid synthetic bisectors: full Rust search vs exact Python pruning.

The independent complete audit groups the z coordinate, which is absent from
these target dots. It also reproduces the full scorer's largest operand bits.
"""
from fractions import Fraction as Q
from itertools import product
from math import gcd, isqrt
import os
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
N, D = 16, 1000


def bisector_bracket(u, v, bits=32):
    # w=tan((atan(u)+atan(v))/2), root of
    # (u+v)w^2 + 2(1-u*v)w - (u+v)=0, between u and v.
    a, b = u + v, 2 * (1 - u * v)
    polynomial = lambda x: a * x * x + b * x - a
    lo, hi = int(u * (1 << bits)), -int(-v * (1 << bits) // 1)
    while hi - lo > 1:
        mid = (lo + hi) // 2
        if polynomial(Q(mid, 1 << bits)) < 0:
            lo = mid
        else:
            hi = mid
    assert polynomial(Q(lo, 1 << bits)) < 0 < polynomial(Q(hi, 1 << bits))
    # A second construction: sqrt((1+u^2)(1+v^2)) by integer-square-root
    # bounds, then w=(u+v)/(sqrt(...) + 1-u*v).
    radicand = (1 + u*u) * (1 + v*v)
    scale = 1 << (bits + 64)
    floor = isqrt(radicand.numerator * scale * scale // radicand.denominator)
    rlo, rhi = Q(floor, scale), Q(floor + 1, scale)
    assert rlo*rlo <= radicand <= rhi*rhi
    low, high = a / (rhi + 1-u*v), a / (rlo + 1-u*v)
    assert Q(lo, 1 << bits) < low <= high < Q(hi, 1 << bits)
    return lo, hi, [str(low), str(high)], [str(a), str(b), str(-a)]


def requests():
    # Axial Cayley numerators/denominators give the entire global competition:
    # orthogonal numerator components strictly increase the denominator only.
    fractions = sorted({Q(p, d) for p in range(-N, N + 1) for d in range(1, D + 1)})
    half_slopes = sorted({Q(y, x) for x in range(1, N + 1) for y in range(-N, N + 1)})
    cases, certificates = [], []
    for name, kind, u, v in [('small_cayley', 'C', Q(1, 1000), Q(1, 999)),
                              ('near_quarter', 'C', Q(15, 16), Q(16, 17)),
                              ('half_axes', 'H', Q(1, 16), Q(1, 15))]:
        assert 0 < u < v < 1
        domain = fractions if kind == 'C' else half_slopes
        denominator_limit = D if kind == 'C' else N
        independent_count = 1 + 2 * sum(gcd(p, d) == 1 for p in range(1, N + 1)
                                        for d in range(1, denominator_limit + 1))
        assert len(domain) == independent_count
        index = domain.index(u)
        assert domain[index + 1] == v, 'not globally adjacent in the complete domain'
        # Integer cross products independently exclude every intervening slope.
        assert not any(u.numerator*d < p*u.denominator and p*v.denominator < v.numerator*d
                       for p in range(1, N + 1) for d in range(1, denominator_limit + 1))
        lo, hi, interval, polynomial = bisector_bracket(u, v)
        certificates.append({'name': name, 'branch': kind, 'left': str(u), 'right': str(v),
                             'coordinate_bits': 32, 'dyadic_lower': str(Q(lo, 1 << 32)),
                             'dyadic_upper': str(Q(hi, 1 << 32)), 'root_interval': interval,
                             'polynomial_descending': polynomial,
                             'complete_distinct_slopes': len(domain)})
        for suffix, m in [('below', lo), ('above', hi)]:
            t = (1 << 32, m, 0, 0) if kind == 'C' else (0, 1 << 32, m, 0)
            common = gcd(*t)
            cases.append((name + '_' + suffix, tuple(x // common for x in t), kind == 'H'))
    return cases, certificates


def pruned(t, half):
    winner, best_num, best_den, count = None, 0, 1, 0
    for p in product(range(-N, N + 1), repeat=3):
        projection = sum(a * v for a, v in zip(t[1:], p))
        norm = sum(v*v for v in p)
        if projection:
            quotient, rem = divmod(t[0] * norm, projection)
            ds = {1, D, max(1, min(D, quotient)), max(1, min(D, quotient + bool(rem)))}
        else:
            ds = {1, D}
        choices = [(d, *p, 'C') for d in ds]
        if half and norm:
            choices.append((1, *p, 'H'))
        for key in choices:
            d = key[0] if key[-1] == 'C' else 0
            num, den = (t[0] * d + projection) ** 2, d*d + norm
            left, right = num * best_den, best_num * den
            if winner is None or left > right or (left == right and key < winner):
                winner, best_num, best_den = key, num, den
            count += 1
    return winner, count


def complete_audit(t, half):
    """Frobenius residual numerator = |t|^2 |q|^2 - (t.q)^2.

    For each (d,x,y), numerator of the dot score is independent of z. z=-16,
    0,16 preserve its final maximum, lexicographic ties and peak comparison
    operands: before z=0 norms decrease, afterwards they increase. For t0=0,
    d>=1 only worsens scores, so d=1,1000 also suffice for the same audit.
    """
    assert t[3] == 0
    target_norm = sum(v*v for v in t)
    best, residual, best_num, best_den, peak = None, 0, 0, 1, 0
    for family in (('C', 'H') if half else ('C',)):
        ds = (1,) if family == 'H' else ((1, D) if t[0] == 0 else range(1, D + 1))
        for d in ds:
            scalar = d if family == 'C' else 0
            for x in range(-N, N + 1):
                for y in range(-N, N + 1):
                    numerator = (scalar*t[0] + x*t[1] + y*t[2]) ** 2
                    for z in (-N, 0, N):
                        norm = scalar*scalar + x*x + y*y + z*z
                        if not norm:
                            continue
                        key = (d, x, y, z, family)
                        peak = max(peak, numerator, norm, numerator * best_den, best_num * norm)
                        complement = target_norm * norm - numerator
                        left, right = complement * best_den, residual * norm
                        if best is None or left < right or (left == right and key < best):
                            best, residual, best_num, best_den = key, complement, numerator, norm
    return best, peak.bit_length()


def run(build):
    build.mkdir(exist_ok=True)
    temp = build / 'tmp'
    temp.mkdir(exist_ok=True)
    env = dict(os.environ, TMP=str(temp), TEMP=str(temp))
    exe = build / ('grid.exe' if os.name == 'nt' else 'grid')
    command = ['rustc', '--edition=2024', '-C', 'opt-level=2', '-C', 'debuginfo=0', '-C', 'panic=abort',
               str(HERE / 'grid.rs'), '-o', str(exe)]
    subprocess.run(command, env=env, check=True, capture_output=True, timeout=90)
    cases, certificates = requests()
    frozen = json.loads((HERE / 'INPUTS.json').read_bytes())['default_grid_requests']
    assert frozen == [{'name': name, 'target': list(t), 'half': half} for name, t, half in cases]
    request_file = build / 'grid-inputs.txt'
    request_file.write_bytes(''.join(name + ' ' + ' '.join(map(str, t)) + f' {int(half)}\n'
                                   for name, t, half in cases).encode('ascii'))
    output = subprocess.run([str(exe), str(request_file)], env=env, check=True,
                            capture_output=True, timeout=300).stdout.decode('ascii').splitlines()
    assert len(output) == len(cases)
    rows = []
    for line, (name, t, half) in zip(output, cases):
        label, key, count, bits = line.split('\t')
        assert label == name
        best, pruned_count = pruned(t, half)
        audit, audit_bits = complete_audit(t, half)
        assert ','.join(map(str, best)) == key and best == audit, ('default-grid winner differs', name)
        assert int(count) == D * (2*N + 1)**3 + (((2*N + 1)**3 - 1) if half else 0)
        assert int(bits) == audit_bits, ('default-grid operand audit differs', name, bits, audit_bits)
        rows.append({'name': name, 'target_integer_quaternion': list(t), 'half_turns_eligible': half,
                     'complete_candidates': int(count), 'pruned_candidates': pruned_count,
                     'winner': list(best), 'pruning_kept_global_winner': True,
                     'largest_score_operand_bits': int(bits)})
    from fixtures import json_bytes
    return {'default_grid.json': json_bytes(rows), 'default_bisectors.json': json_bytes(certificates)}
