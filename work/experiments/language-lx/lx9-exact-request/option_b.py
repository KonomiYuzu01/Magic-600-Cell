"""Option B prototype, not an accepted wire or input-map contract.

C and H carry canonical bounded dyadics; M carries a bound menu letter. Direct
menu requests require membership. Drags use exact trace scores, lowest index.
"""
from fractions import Fraction as Q
from functools import reduce
from hashlib import sha256
from itertools import product
from math import gcd
import json
import re

from intervals import binary64
from q5 import Q5, determinant3, dot, identity, multiply, transpose

MAX_NUM_BITS = 1024
MAX_EXPONENT = 1074
MAX_BYTES = 2048
TOKEN = re.compile(r'(0|-?[1-9][0-9]*):(0|[1-9][0-9]*)\Z')
HEX = re.compile(r'[0-9a-f]{64}\Z')


def dyadic_token(value):
    if not isinstance(value, (int, Q)) or isinstance(value, bool):
        raise ValueError('exact rational required')
    value = Q(value)
    d, n = value.denominator, value.numerator
    if d & (d - 1):
        raise ValueError('not dyadic')
    k = d.bit_length() - 1
    if abs(n).bit_length() > MAX_NUM_BITS or k > MAX_EXPONENT:
        raise ValueError('dyadic size bound')
    return f'{n}:{k}'


def encode(kind, values):
    if kind not in ('C', 'H') or len(values) != 3:
        raise ValueError('Cayley or half-turn triple required')
    tokens = [dyadic_token(v) for v in values]
    values = list(map(Q, values))
    if kind == 'H':
        d = max(v.denominator for v in values)
        p = [int(v * d) for v in values]
        common = reduce(gcd, p)
        if not common:
            raise ValueError('half-turn axis is zero')
        if next(v for v in p if v) < 0:
            common = -common
        tokens = [dyadic_token(Q(v // common)) for v in p]
    return ('LX9B1|' + kind + '|' + '|'.join(tokens) + '\n').encode('ascii')


def from_binary64(kind, components):
    # Signed zero is deliberately canonicalized to 0:0 at this ingress only.
    return encode(kind, [binary64(x) for x in components])


def menu_letter(menu, index):
    if type(index) is not int or not 0 <= index < len(menu.matrices):
        raise ValueError('menu index')
    return f'LX9B1|M|{menu.set_identity}|{menu.order_identity}|{index}\n'.encode('ascii')


def decode(data, menu=None):
    if not isinstance(data, bytes) or len(data) > MAX_BYTES:
        raise ValueError('request byte bound')
    try:
        text = data.decode('ascii')
    except UnicodeDecodeError as exc:
        raise ValueError('ASCII request required') from exc
    if not text.endswith('\n') or text.count('\n') != 1:
        raise ValueError('single LF required')
    fields = text[:-1].split('|')
    if len(fields) != 5 or fields[0] != 'LX9B1':
        raise ValueError('request format')
    kind = fields[1]
    if kind == 'M':
        a, b, index = fields[2:]
        if not HEX.fullmatch(a) or not HEX.fullmatch(b) or not re.fullmatch(r'0|[1-9][0-9]*', index):
            raise ValueError('menu binding syntax')
        if menu is None or (a, b) != (menu.set_identity, menu.order_identity):
            raise ValueError('menu identity or order differs')
        j = int(index)
        if j >= len(menu.matrices):
            raise ValueError('menu index')
        return kind, j
    if kind not in ('C', 'H'):
        raise ValueError('request branch')
    values = []
    for token in fields[2:]:
        if not TOKEN.fullmatch(token):
            raise ValueError('non-canonical dyadic token')
        n, k = map(int, token.split(':'))
        if abs(n).bit_length() > MAX_NUM_BITS or k > MAX_EXPONENT:
            raise ValueError('dyadic size bound')
        if k and (n == 0 or n % 2 == 0):
            raise ValueError('non-reduced dyadic')
        values.append(Q(n, 1 << k))
    if encode(kind, values) != data:
        raise ValueError('non-canonical half-turn axis')
    return kind, tuple(values)


def rotation(kind, values):
    w = tuple(Q5(v) for v in values)
    norm = dot(w, w)
    if kind == 'H':
        if not norm:
            raise ValueError('zero half-turn axis')
        return tuple(tuple(2 * w[i] * w[j] / norm - int(i == j) for j in range(3)) for i in range(3))
    if kind != 'C':
        raise ValueError('rotation branch')
    x, y, z = w
    skew = ((Q5(), -z, y), (z, Q5(), -x), (-y, x, Q5()))
    squared = multiply(skew, skew)
    return tuple(tuple(int(i == j) + 2 * (skew[i][j] + squared[i][j]) / (1 + norm)
                       for j in range(3)) for i in range(3))


class Menu:
    def __init__(self, record):
        entries = record['base']
        assert len(entries) == 60
        keys = [json.dumps(e['matrix'], separators=(',', ':')) for e in entries]
        digest = lambda k: sha256(('[' + ','.join(k) + ']').encode('ascii')).hexdigest()
        self.set_identity = digest(sorted(keys))
        self.order_identity = digest(keys)
        assert self.set_identity == record['identity']
        raw = [tuple(tuple(Q5.read(v) for v in row) for row in e['matrix']) for e in entries]
        # The group average is the projector onto its common fixed line.
        pole = tuple(sum((m[i][0] for m in raw), Q5()) for i in range(4))
        assert any(pole)
        n0, n1, n2, n3 = pole
        frame = ((-n1, n0, -n3, n2), (-n2, n3, n0, -n1), (-n3, -n2, n1, n0))
        norm = dot(pole, pole)
        assert multiply(frame, transpose(frame)) == tuple(tuple(norm * int(i == j) for j in range(3)) for i in range(3))
        self.matrices = []
        self.pole, self.frame = pole, frame
        for m in raw:
            assert multiply(m, transpose(m)) == identity(4)
            assert tuple(dot(row, pole) for row in m) == pole
            cap = tuple(tuple(dot(frame[i], tuple(dot(row, frame[j]) for row in m)) / norm
                              for j in range(3)) for i in range(3))
            assert multiply(cap, transpose(cap)) == identity(3) and determinant3(cap) == 1
            lifted = tuple(tuple((pole[a] * pole[b] + sum(
                (cap[i][j] * frame[i][a] * frame[j][b] for i in range(3) for j in range(3)), Q5())) / norm
                for b in range(4)) for a in range(4))
            assert lifted == m
            self.matrices.append(cap)
        assert len(set(self.matrices)) == 60

    def bound_request(self, data, drag=False):
        """Prototype output: an index is meaningful only under both identities."""
        index, ties, distance = self.request(data, drag)
        return {'menu_identity': self.set_identity, 'menu_order': self.order_identity,
                'index': index, 'ties': ties, 'distance_squared_Q5': distance.record(),
                'request_sha256': sha256(data).hexdigest(), 'operation': 'drag' if drag else 'direct'}

    def choose(self, target):
        scores = [sum((a * b for r, s in zip(target, m) for a, b in zip(r, s)), Q5())
                  for m in self.matrices]
        best = 0
        for j in range(1, 60):
            if (scores[j] - scores[best]).sign() > 0:
                best = j
        ties = [j for j, s in enumerate(scores) if s == scores[best]]
        return best, ties, 6 - 2 * scores[best]

    def request(self, data, drag=False):
        kind, value = decode(data, self)
        if kind == 'M':
            if drag:
                raise ValueError('drag requires C or H')
            return value, [value], Q5()
        target = rotation(kind, value)
        best, ties, distance = self.choose(target)
        if not drag and distance != 0:
            raise ValueError('direct request is not a menu element')
        return best, ties, distance


def free(data, quantise=False, n=1, dmax=16):
    kind, values = decode(data)
    target = rotation(kind, values)
    if not quantise:
        return target, None
    if type(n) is not int or type(dmax) is not int or n < 0 or dmax < 1:
        raise ValueError('grid bounds')
    # Within one degree of pi iff cos(theta)<=-cos(pi/180). The squared
    # quaternion scalar is rational. An independent trig certificate tests it.
    from intervals import degree_trig
    scalar_squared = Q(0) if kind == 'H' else 1 / (1 + sum(v * v for v in values))
    if scalar_squared == 0:
        half = True
    else:
        for bits in (32, 64, 128, 256):
            _, cosine = degree_trig(Q(1), bits)
            limit = ((1 - cosine[1]) / 2, (1 - cosine[0]) / 2)
            if scalar_squared < limit[0] or scalar_squared > limit[1]:
                half = scalar_squared < limit[0]
                break
        else:
            raise ArithmeticError('half-turn eligibility unresolved')
    choices = [(d, *p, 'C') for d in range(1, dmax + 1)
               for p in product(range(-n, n + 1), repeat=3)]
    if half:
        choices += [(1, *p, 'H') for p in product(range(-n, n + 1), repeat=3) if any(p)]
    best, best_score = None, None
    for row in sorted(choices):
        d, *p, family = row
        candidate = rotation(family, [Q(v, d) for v in p])
        score = sum((a * b for r, s in zip(target, candidate) for a, b in zip(r, s)), Q5())
        if best is None or (score - best_score).sign() > 0:
            best, best_score, realised = row, score, candidate
    return realised, best
