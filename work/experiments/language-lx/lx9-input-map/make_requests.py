"""Freeze the request rule before step 1; extend step 2 only by that rule."""
import sys
sys.dont_write_bytecode = True

import argparse
from fractions import Fraction as F
from itertools import product
import json
import math
import random

from common import HERE, bits, digest, json_bytes, read_requests, request_bytes

SEED = 0x4C5839
AXES = ((1., 0., 0.), (0., 1., 0.), (0., 0., 1.), (1., 2., 3.),
        (.3, -.2, .9), (1., 1., 1.), (-1., 2., -3.), (1., -1., .25))


def step1_requests():
    rng = random.Random(SEED)
    angles = [i / 10 for i in range(3600)] + [rng.random() * 360 for _ in range(10000)]
    return [(f"s{i:05d}", AXES[i % len(AXES)], angle, 1, 4)
            for i, angle in enumerate(angles)]


def atan_interval(x):
    """Alternating rational series: successive partial sums enclose atan(x)."""
    assert 0 < x < 1
    term, total, k = x, F(0), 0
    while True:
        total += term / (2 * k + 1)
        term *= -x * x
        k += 1
        next_term = term / (2 * k + 1)
        if abs(next_term) < F(1, 2**180):
            return min(total, total + next_term), max(total, total + next_term)


def enclosing_floats(lo, hi):
    """Certify adjacent floats around an exact interval, without libm inversion."""
    low = float((lo + hi) / 2)
    if lo == hi == F(low):
        return low, math.nextafter(low, math.inf)
    while F(low) >= lo:
        low = math.nextafter(low, -math.inf)
    high = math.nextafter(low, math.inf)
    while F(high) <= lo:
        low, high = high, math.nextafter(high, math.inf)
    assert F(low) < lo <= hi < F(high)
    assert F(high) - F(low) <= F(math.ulp(low))
    assert F(high) - F(low) <= F(math.ulp(high))
    return low, high


def q2_sign(a, b):
    """Sign of a+b*sqrt(2), using only rational comparisons."""
    if not b:
        return (a > 0) - (a < 0)
    if not a:
        return (b > 0) - (b < 0)
    if (a > 0) == (b > 0):
        return 1 if a > 0 else -1
    delta = a * a - 2 * b * b
    return ((delta > 0) - (delta < 0)) * (1 if a > 0 else -1)


def bisector_requests():
    rows = []
    certificates = {"half_turn": {}, "cayley": []}
    scale = 2**200
    floor = math.isqrt(2 * scale * scale)
    tlo, thi = enclosing_floats(F(floor, scale) - 1, F(floor + 1, scale) - 1)
    assert F(tlo)**2 + 2 * F(tlo) - 1 < 0 < F(thi)**2 + 2 * F(thi) - 1
    certificates["half_turn"] = {
        "polynomial": [1, 2, -1], "root": "positive",
        "axis": ["1", "sqrt(2)-1", "z"], "angle_degrees": 180,
        "z_rule": "(2*i-127)/1024, i=0..127",
        "enclosing_bits": [bits(tlo), bits(thi)],
        "pair": [[1, 0, 0], [1, 1, 0]],
        "global_maximum_checked": "all 26 half-turn triples for each z; Cayley norms are larger",
    }
    for i in range(128):
        z = F(2 * i - 127, 1024)
        assert abs(z) < F(1, 8)
        ties = []
        for p in product(range(-1, 2), repeat=3):
            if not any(p):
                continue
            a, b = p[0] - p[1] + p[2] * z, F(p[1])
            gap = q2_sign(sum(x*x for x in p) - a*a - 2*b*b, -2*a*b)
            assert gap >= 0, "bisector pair is not globally nearest"
            if gap == 0:
                ties.append(p)
        assert ties == [(-1, -1, 0), (-1, 0, 0), (1, 0, 0), (1, 1, 0)]
        for transform in range(4):
            for value in (tlo, thi):
                axes = ((1., value, float(z)), (value, 1., float(z)),
                        (-1., value, float(z)), (float(z), value, 1.))
                rows.append((f"b{len(rows):04d}", axes[transform], 180., 1, 4))

    a, b = atan_interval(F(1, 5)), atan_interval(F(1, 239))
    pi_lo, pi_hi = 16*a[0] - 4*b[1], 16*a[1] - 4*b[0]
    angles = [(F(0), F(0))]
    quats = [(1, 0)]
    for d in range(16, 0, -1):
        angles.append((pi_lo / 4, pi_hi / 4) if d == 1 else atan_interval(F(1, d)))
        quats.append((d, 1))
    for j in range(16):
        lo = (angles[j][0] + angles[j+1][0]) * 180 / pi_hi
        hi = (angles[j][1] + angles[j+1][1]) * 180 / pi_lo
        # atan(1/3)+atan(1/2)=atan(1)=pi/4 exactly; interval dependency
        # otherwise straddles the exactly representable 45-degree boundary.
        if quats[j:j+2] == [(3, 1), (2, 1)]:
            assert (F(1, 3) + F(1, 2)) / (1 - F(1, 3)*F(1, 2)) == 1
            lo = hi = F(45)
        low, high = enclosing_floats(lo, hi)
        qa, qb = quats[j:j+2]
        na, nb = sum(x*x for x in qa), sum(x*x for x in qb)
        polynomial = [qa[1]**2*nb - qb[1]**2*na,
                      2*qa[0]*qa[1]*nb - 2*qb[0]*qb[1]*na,
                      qa[0]**2*nb - qb[0]**2*na]
        certificates["cayley"].append({
            "pair_quaternions_2d": [qa, qb], "tan_half_angle_polynomial": polynomial,
            "degree_interval": [str(lo), str(hi)], "enclosing_bits": [bits(low), bits(high)],
        })
        for coordinate in range(3):
            for sign in (-1., 1.):
                axis = tuple(sign if k == coordinate else 0. for k in range(3))
                for angle in (low, high):
                    rows.append((f"b{len(rows):04d}", axis, angle, 1, 16))
    assert len(rows) == 1216
    assert len({(r[1], r[2], r[3], r[4]) for r in rows}) == len(rows)
    return rows, certificates


def initial_files():
    near, certificates = bisector_requests()
    return {"step1.tsv": request_bytes(step1_requests()),
            "bisectors.tsv": request_bytes(near),
            "bisectors.json": json_bytes(certificates)}


def freeze_initial():
    assert not (HERE / "MANIFEST.json").exists(), "already frozen"
    files = initial_files()
    for name, data in files.items():
        (HERE / name).write_bytes(data)
    manifest = {"schema": "lx9-inputs-v1", "seed": SEED, "decision": "open",
                "decider": "owner", "step1_count": 13600, "near_bisector_count": 1216,
                "step2_included_step1_ids": None, "files": {}}
    for name in (*files, "CARD.md", "make_requests.py"):
        data = (HERE / name).read_bytes()
        manifest["files"][name] = {"sha256": digest(data), "bytes": len(data)}
    (HERE / "MANIFEST.json").write_bytes(json_bytes(manifest))


def freeze_step2(ids):
    manifest = json.loads((HERE / "MANIFEST.json").read_bytes())
    assert manifest["step2_included_step1_ids"] is None, "step 2 already frozen"
    requests = {r[0]: r for r in read_requests(HERE / "step1.tsv")}
    ids = sorted(set(ids))
    rows = read_requests(HERE / "bisectors.tsv") + [requests[k] for k in ids]
    data = request_bytes(rows)
    (HERE / "step2.tsv").write_bytes(data)
    manifest["step2_included_step1_ids"] = ids
    manifest["step2_count"] = len(rows)
    manifest["files"]["step2.tsv"] = {"sha256": digest(data), "bytes": len(data)}
    (HERE / "MANIFEST.json").write_bytes(json_bytes(manifest))


def verify_construction(manifest):
    for name, data in initial_files().items():
        assert (HERE / name).read_bytes() == data, name + " construction differs"
    if manifest["step2_included_step1_ids"] is not None:
        all_rows = {r[0]: r for r in read_requests(HERE / "step1.tsv")}
        expected = read_requests(HERE / "bisectors.tsv") + [
            all_rows[k] for k in manifest["step2_included_step1_ids"]]
        assert (HERE / "step2.tsv").read_bytes() == request_bytes(expected)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--step2-ids", type=str, help="JSON array, relative to this experiment folder")
    args = parser.parse_args()
    if args.step2_ids:
        path = (HERE / args.step2_ids).resolve()
        assert path.parent == HERE
        freeze_step2(json.loads(path.read_bytes()))
    else:
        freeze_initial()
    print("Request manifest frozen; no experiment decision made.")
