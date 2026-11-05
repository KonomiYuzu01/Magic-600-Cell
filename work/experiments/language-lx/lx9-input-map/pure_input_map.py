"""Pure Python A1 port for the frozen bounded experiment, including exact ties.

The target uses left-to-right fused multiply-add for the short dot products.
This is controlled against the installed NumPy build, not assumed portable
across NumPy/BLAS versions. The chooser exhausts all bounded candidates instead
of using the reference's denominator pruning and 1e-12 float screening.
"""
import math
from functools import lru_cache
from itertools import product


def norm(values, fused=True):
    total = 0.0
    for x in values:
        total = math.fma(x, x, total) if fused else total + x * x
    return math.sqrt(total)


def input_quaternion(axis, degrees, fused=True):
    if len(axis) != 3 or not all(map(math.isfinite, axis)) or not any(axis):
        raise ValueError("axis must be three finite numbers, not all zero")
    if not math.isfinite(degrees):
        raise ValueError("angle must be finite")
    scale = max(map(abs, axis))
    a = [x / scale for x in axis]
    length = norm(a, fused)
    a = [x / length for x in a]
    theta = math.radians(math.remainder(degrees, 360.0))
    target = [math.cos(theta / 2), *(math.sin(theta / 2) * x for x in a)]
    length = norm(target, fused)
    return a, [x / length for x in target]


@lru_cache(None)
def candidates(n, dmax, half):
    # Sorted exactly as the reference's (denominator, numerator tuple, family).
    choices = [(d, p, "cayley") for d in range(1, dmax + 1)
               for p in product(range(-n, n + 1), repeat=3)]
    if half:
        choices += [(1, p, "half_turn") for p in product(range(-n, n + 1), repeat=3) if any(p)]
    return [(choice, ((0 if choice[2] == "half_turn" else choice[0]), *choice[1]))
            for choice in sorted(choices)]


def choose(target, degrees, n, dmax):
    ratios = [x.as_integer_ratio() for x in target]
    common = max(d for _, d in ratios)
    t = [a * (common // b) for a, b in ratios]
    half = abs(abs(math.remainder(degrees, 360.0)) - 180.0) <= 1.0
    best, best_num, best_den = None, -1, 1
    for choice, q in candidates(n, dmax, half):
        dot = sum(x * y for x, y in zip(t, q))
        num = dot * dot
        den = sum(x * x for x in q)
        if num * best_den > best_num * den:
            best, best_num, best_den = choice, num, den
    return best


def nearest_parameter(axis, degrees, max_den, max_num=1, fused=True):
    _, target = input_quaternion(axis, degrees, fused)
    return target, choose(target, degrees, max_num, max_den)
