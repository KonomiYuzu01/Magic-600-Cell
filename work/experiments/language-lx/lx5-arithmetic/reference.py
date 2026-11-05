"""Integer-only LX-5 oracle. Experiment code; no engine or language decision."""
import hashlib
import math
from pathlib import Path
import struct
import sys

sys.dont_write_bytecode = True
sys.set_int_max_str_digits(0)


def sign(a):
    return (a > 0) - (a < 0)


def rational(a, d):
    if not d:
        raise ValueError("zero denominator")
    g = math.gcd(a, d) * sign(d)
    return a // g, d // g


def q5(a, b, d):
    if not d:
        raise ValueError("zero denominator")
    g = math.gcd(a, b, d) * sign(d)
    return a // g, b // g, d // g


def qadd(x, y):
    a, b, d = x
    c, e, f = y
    return q5(a * f + c * d, b * f + e * d, d * f)


def qmul(x, y):
    a, b, d = x
    c, e, f = y
    return q5(a * c + 5 * b * e, a * e + b * c, d * f)


def qsign(x):
    a, b, d = q5(*x)
    if not a:
        return sign(b)
    if not b or sign(a) == sign(b):
        return sign(a)
    return sign(a) * sign(a * a - 5 * b * b)


def load(path):
    values, cases = {}, []
    for line in Path(path).read_text(encoding="ascii").splitlines():
        fields = line.split("\t")
        if fields[0] == "value":
            _, name, decimal = fields
            assert name not in values
            values[name] = int(decimal)
        else:
            tag, key, size, op, measured, *args = fields
            assert tag == "case"
            cases.append(dict(id=key, size=size, op=op, measured=measured == "m", args=args))
    assert len({c["id"] for c in cases}) == len(cases)
    return values, cases


def word_summary(path):
    digest = hashlib.sha256()
    counts = [0] * 65536
    length = total = 0
    with Path(path).open("rb") as stream:
        while block := stream.read(65536):
            if len(block) % 2:
                raise ValueError("odd ia16 byte count")
            digest.update(block)
            length += len(block) // 2
            for (letter,) in struct.iter_unpack("<H", block):
                counts[letter] += 1
                total += letter
    histogram = ",".join(f"{i}:{n}" for i, n in enumerate(counts) if n)
    return f"{length}|{total}|{digest.hexdigest()}|{histogram}"


def evaluate(case, values, word_dir):
    op = case["op"]
    if op == "word":
        return word_summary(Path(word_dir) / case["args"][0])
    x = [values[k] for k in case["args"]]
    a = x[0]
    b = x[1] if len(x) > 1 else 0
    if op == "add": result = a + b
    elif op == "sub": result = a - b
    elif op == "mul": result = a * b
    elif op == "div": result = a // b
    elif op == "rem": result = a % b
    elif op == "gcd": result = math.gcd(a, b)
    elif op == "cmp": result = sign(a - b)
    elif op == "shl": result = a << b
    elif op == "shr": result = a >> b
    elif op == "bits": result = abs(a).bit_length()
    elif op == "dec": result = int(str(a))
    elif op == "hex":
        h = ("-" if a < 0 else "") + format(abs(a), "x")
        assert int(h, 16) == a
        return h + "|" + str(int(h, 16))
    elif op == "sum": result = sum(x)
    elif op.startswith("rat_"):
        a, d, b, e = x
        if op == "rat_add": result = rational(a * e + b * d, d * e)
        elif op == "rat_mul": result = rational(a * b, d * e)
        else:
            a, d = rational(a, d)
            b, e = rational(b, e)
            result = sign(a * e - b * d)
    elif op.startswith("q_"):
        left = q5(*x[:3])
        if op == "q_add": result = qadd(left, q5(*x[3:]))
        elif op == "q_mul": result = qmul(left, q5(*x[3:]))
        elif op == "q_conj": result = (left[0], -left[1], left[2])
        elif op == "q_norm": result = rational(left[0] ** 2 - 5 * left[1] ** 2, left[2] ** 2)
        elif op == "q_sign": result = qsign(left)
        else: raise ValueError(op)
    else:
        raise ValueError(op)
    return ",".join(map(str, result)) if isinstance(result, tuple) else str(result)


def self_test():
    """Independent small identities and Fraction cross-checks, not fixture echo."""
    from fractions import Fraction
    import random
    rng = random.Random(0x4C5835)
    assert (-7 // 3, -7 % 3, 7 // -3, 7 % -3) == (-3, 2, -3, -2)
    assert rational(0, -8) == (0, 1) and q5(0, 0, -8) == (0, 0, 1)
    for _ in range(200):
        a, b, c = (rng.randrange(-10000, 10001) for _ in range(3))
        d, e, f = (rng.randrange(1, 1000) for _ in range(3))
        r = Fraction(a, d) + Fraction(b, e)
        assert rational(a * e + b * d, d * e) == (r.numerator, r.denominator)
        r = Fraction(a, d) * Fraction(b, e)
        assert rational(a * b, d * e) == (r.numerator, r.denominator)
        x, y = q5(a, b, d), q5(c, e, f)
        product = qmul(x, y)
        pa = Fraction(x[0], x[2]) * Fraction(y[0], y[2]) + 5 * Fraction(x[1], x[2]) * Fraction(y[1], y[2])
        pb = Fraction(x[0], x[2]) * Fraction(y[1], y[2]) + Fraction(x[1], x[2]) * Fraction(y[0], y[2])
        assert (Fraction(product[0], product[2]), Fraction(product[1], product[2])) == (pa, pb)
        assert qsign(x) == -qsign((-x[0], -x[1], x[2]))
        assert qsign(qmul(x, x)) == (0 if x[:2] == (0, 0) else 1)
    # Pell margins approach zero, but the sign follows norm = (-1)^n exactly.
    a, b = 1, 0
    for n in range(1, 150):
        a, b = 2 * a + 5 * b, a + 2 * b
        assert a * a - 5 * b * b == (-1) ** n
        assert qsign((a, -b, 7)) == (-1) ** n
    big = 10 ** 6774 + 123456789
    assert len(str(big)) == 6775 and int(str(big)) == big

