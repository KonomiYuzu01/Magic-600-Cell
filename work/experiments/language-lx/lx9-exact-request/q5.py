"""Exact Q(sqrt(5)) and small matrices; experiment-local, no floating point."""
from fractions import Fraction as Q
from math import lcm


class Q5:
    __slots__ = ('a', 'b')

    def __init__(self, a=0, b=0):
        self.a, self.b = Q(a), Q(b)

    @staticmethod
    def of(x):
        return x if isinstance(x, Q5) else Q5(x)

    @classmethod
    def read(cls, triple):
        a, b, d = triple
        return cls(Q(a, d), Q(b, d))

    def record(self):
        d = lcm(self.a.denominator, self.b.denominator)
        return [int(self.a * d), int(self.b * d), d]

    def __add__(self, other):
        o = self.of(other)
        return Q5(self.a + o.a, self.b + o.b)

    __radd__ = __add__

    def __neg__(self):
        return Q5(-self.a, -self.b)

    def __sub__(self, other):
        return self + -self.of(other)

    def __rsub__(self, other):
        return self.of(other) + -self

    def __mul__(self, other):
        o = self.of(other)
        return Q5(self.a * o.a + 5 * self.b * o.b, self.a * o.b + self.b * o.a)

    __rmul__ = __mul__

    def __truediv__(self, other):
        o = self.of(other)
        norm = o.a * o.a - 5 * o.b * o.b
        return self * Q5(o.a / norm, -o.b / norm)

    def __bool__(self):
        return bool(self.a or self.b)

    def __eq__(self, other):
        o = self.of(other)
        return self.a == o.a and self.b == o.b

    def __hash__(self):
        return hash((self.a, self.b))

    def sign(self):
        a, b = self.a, self.b
        if not b:
            return (a > 0) - (a < 0)
        if not a or (a > 0) == (b > 0):
            return (b > 0) - (b < 0)
        norm = a * a - 5 * b * b
        return ((norm > 0) - (norm < 0)) * ((a > 0) - (a < 0))


def dot(a, b):
    return sum((x * y for x, y in zip(a, b)), Q5())


def transpose(m):
    return tuple(zip(*m))


def multiply(a, b):
    return tuple(tuple(dot(r, c) for c in transpose(b)) for r in a)


def identity(size):
    return tuple(tuple(Q5(int(i == j)) for j in range(size)) for i in range(size))


def determinant3(m):
    return (m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]))


def matrix_record(m):
    return [[x.record() for x in row] for row in m]
