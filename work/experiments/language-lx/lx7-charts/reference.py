"""Independent rational map expectations and the staged float32 CPU reference."""
import sys
sys.dont_write_bytecode = True
from fractions import Fraction as F
import json
import math
from pathlib import Path
import struct
from generate_hlsl import STAGES, CHARTS
from subset import OK, SINGULAR, NONFINITE, admit, evaluate, f32, bits

HERE = Path(__file__).resolve().parent
EPSILON = F(1, 4096)


def json_bytes(value):
    return (json.dumps(value, indent=2, ensure_ascii=True, allow_nan=False) + "\n").encode("ascii")


def cases(fixtures):
    for stage_id, stage in enumerate(STAGES):
        for pose in fixtures["poses"]:
            matrix = [[F(v) for v in row] for row in pose["matrix"]]
            for vertex in fixtures["vertices"]:
                yield {"id": f'{stage}/{pose["id"]}/{vertex["id"]}', "stage": stage,
                       "stage_id": stage_id, "vertex": tuple(F(v) for v in vertex["point"]),
                       "matrix": matrix, "d": F(fixtures["d"])}


def exact(case):
    p = tuple(sum((a * b for a, b in zip(row, case["vertex"], strict=True)), F(0))
              for row in case["matrix"])
    stage, d = case["stage"], case["d"]
    if stage in ("builtin_perspective", "perspective"):
        den, numerator = d - p[3], tuple(d * v for v in p[:3])
    elif stage == "stereographic":
        den, numerator = 1 - p[3], p[:3]
    elif stage == "gnomonic":
        den, numerator = p[3], p[:3]
    else:
        raise ValueError("unknown stage")
    if abs(den) <= EPSILON:
        return SINGULAR, None
    return OK, tuple(v / den for v in numerator)


def expectations(fixtures):
    result = []
    for case in cases(fixtures):
        coverage, point = exact(case)
        result.append({"id": case["id"], "coverage": coverage,
                       "point": None if point is None else [str(v) for v in point]})
    return {"format": "lx7-exact-expectations-v1", "cases": result}


def posed_f32(case):
    vertex = [f32(float(v)) for v in case["vertex"]]
    point = []
    for row in case["matrix"]:
        products = [f32(f32(float(q)) * v) for q, v in zip(row, vertex, strict=True)]
        # Matches HLSL row(): (((q0*v0 + q1*v1) + q2*v2) + q3*v3).
        point.append(f32(f32(f32(products[0] + products[1]) + products[2]) + products[3]))
    return tuple(point)


def builtin(point, d):
    if not all(math.isfinite(v) for v in (*point, d)):
        return NONFINITE, None
    den = f32(d - point[3])
    if not math.isfinite(den):
        return NONFINITE, None
    if abs(den) <= float(EPSILON):
        return SINGULAR, None
    numerator = tuple(f32(d * v) for v in point[:3])
    if not all(math.isfinite(v) for v in numerator):
        return NONFINITE, None
    projected = tuple(f32(v / den) for v in numerator)
    if not all(math.isfinite(v) for v in projected):
        return NONFINITE, None
    return OK, projected


def programs():
    return {name: admit((HERE / "charts" / (name + ".chart")).read_text(encoding="utf-8")) for name in CHARTS}


def cpu(case, checked):
    p, d = posed_f32(case), f32(float(case["d"]))
    return builtin(p, d) if case["stage_id"] == 0 else evaluate(checked[case["stage"]], p, d)


def record(result):
    coverage, point = result
    return (coverage, 0, 0, 0) if point is None else (coverage, *(bits(v) for v in point))


def input_bytes(fixtures):
    rows = []
    for case in cases(fixtures):
        values = (*case["vertex"], *(q for row in case["matrix"] for q in row), case["d"])
        rows.append(struct.pack("<21f3I", *(f32(float(v)) for v in values), case["stage_id"], 0, 0))
    return b"".join(rows)


def ordered(value):
    b = bits(value)
    return 0x80000000 - (b & 0x7fffffff) if b & 0x80000000 else 0x80000000 + b


def distribution(values):
    if not values:
        return {"n": 0, "min": None, "p50": None, "p95": None, "p99": None, "max": None}
    values = sorted(values)
    return {"n": len(values), "min": float(values[0]),
            **{name: float(values[max(0, math.ceil(len(values) * pct / 100) - 1)])
               for name, pct in (("p50", 50), ("p95", 95), ("p99", 99))}, "max": float(values[-1])}


def summarize(fixtures, records):
    """Raw coordinate errors, never a chart-admission or precision verdict."""
    checked = programs()
    all_cases = list(cases(fixtures))
    if len(records) != len(all_cases):
        raise ValueError("result count mismatch")
    groups = {name: {"cases": 0, "coverage": {"point": 0, "singular": 0, "nonfinite": 0},
                     "exact_coverage_disagreements": [], "cpu_coverage_disagreements": [],
                     "exact_abs": [], "exact_rel_nonzero": [], "cpu_abs": [], "cpu_ulp": []}
              for name in STAGES}
    for case, raw in zip(all_cases, records, strict=True):
        coverage, *coordinates = raw
        if coverage not in (OK, SINGULAR, NONFINITE):
            raise ValueError("invalid coverage code")
        if coverage != OK and any(coordinates):
            raise ValueError("missing coverage must have empty coordinate payload")
        point = struct.unpack("<3f", struct.pack("<3I", *coordinates)) if coverage == OK else None
        if point is not None and not all(math.isfinite(v) for v in point):
            raise ValueError("nonfinite coordinate marked as point")
        ecode, ep = exact(case)
        ccode, cp = cpu(case, checked)
        group = groups[case["stage"]]
        group["cases"] += 1
        group["coverage"][("point", "singular", "nonfinite")[coverage]] += 1
        if coverage != ecode:
            group["exact_coverage_disagreements"].append(case["id"])
        if coverage != ccode:
            group["cpu_coverage_disagreements"].append(case["id"])
        if coverage == ecode == OK:
            for observed, expected in zip(point, ep, strict=True):
                delta = abs(F(observed) - expected)
                group["exact_abs"].append(delta)
                if expected:
                    group["exact_rel_nonzero"].append(delta / abs(expected))
        if coverage == ccode == OK:
            for observed, expected in zip(point, cp, strict=True):
                group["cpu_abs"].append(abs(F(observed) - F(expected)))
                group["cpu_ulp"].append(abs(ordered(observed) - ordered(expected)))
    for group in groups.values():
        for field in ("exact_abs", "exact_rel_nonzero", "cpu_abs", "cpu_ulp"):
            group[field] = distribution(group[field])
    return {"format": "lx7-errors-v1", "precision_decision": "open", "decider": "owner",
            "quantiles": "nearest rank, per coordinate; relative omits exact zero; ULP collapses signed zero",
            "units": "absolute: projected coordinate units; relative: dimensionless; ULP: binary32 steps to CPU",
            "charts": groups}
