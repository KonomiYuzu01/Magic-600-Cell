"""Independent B scorer: projective quaternions, squared inner products.

Shares Q5 scalar arithmetic only. It does not call the primary decoder,
rotation constructor, membership test, trace scorer or bounded-grid chooser.
"""
from fractions import Fraction as Q
from hashlib import sha256
from itertools import product
from math import gcd, isqrt
import json

from q5 import Q5, dot
from oracle import angle_trig


def decode_float_bits(text):
    if len(text) != 16 or any(c not in '0123456789abcdef' for c in text):
        raise ValueError('binary64 encoding')
    bits = bin(int(text, 16))[2:].zfill(64)
    exponent = int(bits[1:12], 2)
    if exponent == 2047:
        raise ValueError('non-finite binary64')
    value = Q(0)
    fraction = ('1' if exponent else '0') + bits[12:]
    power = max(1, exponent) - 1023
    for j, bit in enumerate(fraction):
        if bit == '1':
            k = power - j
            value += Q(2 ** k) if k >= 0 else Q(1, 2 ** -k)
    return -value if bits[0] == '1' else value


def parse(data, identities=None):
    if type(data) is not bytes or len(data) > 2048:
        raise ValueError('invalid bytes')
    try:
        s = data.decode('ascii')
    except UnicodeDecodeError as exc:
        raise ValueError('invalid ASCII') from exc
    if s[-1:] != '\n' or '\n' in s[:-1]:
        raise ValueError('invalid line')
    f = s[:-1].split('|')
    if len(f) != 5 or f[0] != 'LX9B1':
        raise ValueError('invalid record')
    if f[1] == 'M':
        if identities is None or tuple(f[2:4]) != identities:
            raise ValueError('unbound menu')
        j = int(f[4])
        if str(j) != f[4] or not 0 <= j < 60:
            raise ValueError('invalid index')
        return 'M', j
    if f[1] not in ('C', 'H'):
        raise ValueError('invalid kind')
    values = []
    for s in f[2:]:
        pair = s.split(':')
        if len(pair) != 2:
            raise ValueError('invalid scalar')
        a, k = map(int, pair)
        if s != str(a) + ':' + str(k) or k < 0 or k > 1074 or abs(a).bit_length() > 1024:
            raise ValueError('noncanonical or overbound scalar')
        if k and a % 2 == 0:
            raise ValueError('nonreduced scalar')
        values.append(Q(a, 2 ** k))
    if f[1] == 'H':
        if any(v.denominator != 1 for v in values) or not any(values):
            raise ValueError('noncanonical half turn')
        integers = [v.numerator for v in values]
        if gcd(*integers) != 1 or next(v for v in integers if v) < 0:
            raise ValueError('noncanonical half turn')
    return f[1], (Q5(int(f[1] == 'C')), *(Q5(v) for v in values))


def quat_matrix(q):
    a, b, c, d = q
    norm = dot(q, q)
    return tuple(tuple(x / norm for x in row) for row in (
        (a*a+b*b-c*c-d*d, 2*(b*c-a*d), 2*(b*d+a*c)),
        (2*(b*c+a*d), a*a-b*b+c*c-d*d, 2*(c*d-a*b)),
        (2*(b*d-a*c), 2*(c*d+a*b), a*a-b*b-c*c+d*d)))


def score(p, q):
    inner = dot(p, q)
    return inner * inner / (dot(p, p) * dot(q, q))


def surd_sign(value):
    # Independent of Q5.sign's norm comparison: compare an integer with the
    # exact floor square root. 5*q^2 is nonsquare for every nonzero integer q.
    a, b = value.a, value.b
    sa, sb = (a > 0) - (a < 0), (b > 0) - (b < 0)
    if not sa:
        return sb
    if not sb or sa == sb:
        return sa
    p = abs(a.numerator) * b.denominator
    q = abs(b.numerator) * a.denominator
    return sa if p > isqrt(5 * q * q) else sb


def menu_data(record):
    # Encode hashes as one JSON array instead of joining pre-serialized keys.
    raw = [e['matrix'] for e in record['base']]
    serial = lambda x: json.dumps(x, separators=(',', ':')).encode('ascii')
    identities = (sha256(serial(sorted(raw, key=lambda m: serial(m)))).hexdigest(),
                  sha256(serial(raw)).hexdigest())
    raw = [[[Q5.read(v) for v in row] for row in m] for m in raw]
    # Use a different column of the invariant projector to recover the same line.
    columns = [[sum((m[r][j] for m in raw), Q5()) for r in range(4)] for j in range(4)]
    pole = next(p for p in reversed(columns) if any(p))
    a, b, c, d = pole
    basis = [(-b, a, -d, c), (-c, d, a, -b), (-d, -c, b, a)]
    norm = dot(pole, pole)
    quats = []
    for m in raw:
        r = [[sum((basis[i][u] * m[u][v] * basis[j][v]
                   for u in range(4) for v in range(4)), Q5()) / norm
              for j in range(3)] for i in range(3)]
        scalar = 1 + r[0][0] + r[1][1] + r[2][2]
        if scalar:
            q = (scalar, r[2][1]-r[1][2], r[0][2]-r[2][0], r[1][0]-r[0][1])
        else:
            j = next(j for j in range(3) if r[j][j] + 1)
            q = (Q5(), *(r[i][j] + int(i == j) for i in range(3)))
        assert quat_matrix(q) == tuple(map(tuple, r))
        quats.append(q)
    return identities, quats


def select(target, quats):
    best, ties = None, []
    for j, q in enumerate(quats):
        value = score(target, q)
        direction = 1 if best is None else surd_sign(value - best)
        if direction > 0:
            best, ties = value, [j]
        elif direction == 0:
            ties.append(j)
    return ties[0], ties, 8 * (1 - best)


def request(data, identities, quats, drag=False):
    kind, value = parse(data, identities)
    if kind == 'M':
        if drag:
            raise ValueError('menu letter is not a drag')
        return value, [value], Q5()
    best, ties, distance = select(value, quats)
    if distance and not drag:
        raise ValueError('not a direct menu member')
    return best, ties, distance


def free(data, quantise=False):
    kind, q = parse(data)
    if kind == 'M':
        raise ValueError('free menu letter')
    if not quantise:
        return quat_matrix(q), None
    a = q[0] * q[0] / dot(q, q)
    assert a.b == 0
    if not a:
        half = True
    else:
        for bits in (32, 64, 128, 256):
            _, cos = angle_trig(Q(1, 360), bits)
            upper, lower = (1 - cos[0]) / 2, (1 - cos[1]) / 2
            if a.a < lower or a.a > upper:
                half = a.a < lower
                break
        else:
            raise ArithmeticError('oracle eligibility unresolved')
    choices = []
    for x, y, z in product((-1, 0, 1), repeat=3):
        choices += [(d, x, y, z, 'C') for d in range(1, 17)]
        if half and (x or y or z):
            choices.append((1, x, y, z, 'H'))
    best, selected, winner = None, None, None
    for row in sorted(choices):
        d, x, y, z, family = row
        target = tuple(map(Q5, (d if family == 'C' else 0, x, y, z)))
        value = score(q, target)
        if best is None or surd_sign(value - best) > 0:
            best, selected, winner = value, row, target
    return quat_matrix(winner), selected
