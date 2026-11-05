"""Small rational cyclotomic fields, embedded by zeta_m = exp(2*pi*i/m).

Polynomial coefficients are ascending. Exact zero is a zero remainder modulo
Phi_m; signs of real elements use rational enclosures of that chosen embedding.
"""
from fractions import Fraction as Q
from functools import lru_cache
from math import gcd, lcm


def trim(a):
    a = list(a)
    while len(a) > 1 and not a[-1]:
        a.pop()
    return a


def remainder(a, b):
    a = trim(a)
    while len(a) >= len(b) and any(a):
        k, c = len(a) - len(b), a[-1] / Q(b[-1])
        for j, v in enumerate(b):
            a[j + k] -= c * v
        a = trim(a)
    return a


@lru_cache(None)
def phi_polynomial(n):
    p = [-1] + [0] * (n - 1) + [1]
    for d in range(1, n):
        if n % d:
            continue
        b = phi_polynomial(d)
        quotient = [Q(0)] * (len(p) - len(b) + 1)
        while len(p) >= len(b):
            k, c = len(p) - len(b), Q(p[-1], b[-1])
            quotient[k] = c
            for j, v in enumerate(b):
                p[j + k] -= c * v
            p = trim(p)
        assert not any(p)
        p = trim(quotient)
    assert all(Q(x).denominator == 1 for x in p)
    return tuple(int(x) for x in p)


class Field:
    def __init__(self, order):
        self.order = order
        self.mod = phi_polynomial(order)
        self.degree = len(self.mod) - 1
        self.one = self.make([1])
        self.zero = self.make([0])

    def make(self, coefficients):
        r = remainder(list(map(Q, coefficients)), self.mod)
        return tuple(r + [Q(0)] * (self.degree - len(r)))

    @lru_cache(None)
    def power(self, k):
        k %= self.order
        return self.make([0] * k + [1])

    def add(self, x, y):
        return tuple(a + b for a, b in zip(x, y))

    def scale(self, x, c):
        return tuple(a * c for a in x)

    def sub(self, x, y):
        return self.add(x, self.scale(y, -1))

    def mul(self, x, y):
        p = [Q(0)] * (2 * self.degree - 1)
        for i, a in enumerate(x):
            if a:
                for j, b in enumerate(y):
                    if b:
                        p[i + j] += a * b
        return self.make(p)

    def trig(self, power):
        assert self.order % 4 == 0
        z, inv = self.power(power), self.power(-power)
        cosine = self.scale(self.add(z, inv), Q(1, 2))
        sine = self.scale(self.mul(self.power(-self.order // 4), self.sub(z, inv)), Q(1, 2))
        return sine, cosine


@lru_cache(None)
def angle_field(turns):
    order = lcm(turns.denominator, 4)
    field = Field(order)
    sine, cosine = field.trig(turns.numerator * (order // turns.denominator))
    return field, sine, cosine


def exact_linear(coeff, turns):
    field, sine, cosine = angle_field(turns)
    a, b, c = coeff
    return field.add(field.scale(field.one, a),
                     field.add(field.scale(cosine, b), field.scale(sine, c)))


@lru_cache(None)
def exceptional_orders():
    # n divides 360*2^k: n=2^a*3^b*5^c, b<=2, c<=1.
    # phi(2^a)<=16 forces a<=5. No unbounded search cutoff is assumed.
    orders = set()
    for a in range(6):
        for b in range(3):
            for c in range(2):
                n = 2 ** a * 3 ** b * 5 ** c
                totient = sum(gcd(j, n) == 1 for j in range(1, n + 1))
                if totient <= 16:
                    orders.add(n)
    return sorted(orders)


def exceptional_angles():
    return sorted({Q(j, n) for n in exceptional_orders() for j in range(n) if gcd(j, n) == 1})


def radical_zero(turns, a, b, c0, c1, q):
    """F=A+B*cos(t)+(C0+C1*sqrt(q))*sin(t); coefficients are Q5 pairs.

    Work in a subfield of Q(zeta_1440). sqrt(q) need not belong to it:
    X+Y*sqrt(q)=0 iff X^2=q*Y^2 and the nonzero real signs oppose.
    """
    import intervals as iv

    assert turns.denominator in exceptional_orders() and q > 0
    m = lcm(turns.denominator, 20)
    f = Field(m)
    s, c = f.trig(turns.numerator * (m // turns.denominator))
    _, cos72 = f.trig(m // 5)
    sqrt5 = f.add(f.one, f.scale(cos72, 4))

    def embed(pair):
        return f.add(f.scale(f.one, Q(pair[0])), f.scale(sqrt5, Q(pair[1])))

    x = f.add(embed(a), f.add(f.mul(embed(b), c), f.mul(embed(c0), s)))
    y = f.mul(embed(c1), s)
    if not any(y):
        return not any(x)
    if f.mul(x, x) != f.scale(f.mul(y, y), q):
        return False

    def real_sign(p):
        for bits in (16, 32, 64, 128, 256, 512):
            interval = (Q(0), Q(0))
            for j, v in enumerate(p):
                if v:
                    interval = iv.add(interval, iv.scale(iv.degree_trig(Q(360 * j, m), bits)[1], v))
            answer = iv.sign(interval)
            if answer:
                return answer
        raise ArithmeticError('radical sign guard unresolved')

    return real_sign(x) == -real_sign(y)
