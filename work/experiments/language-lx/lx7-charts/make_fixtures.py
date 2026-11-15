"""Reproducible rational lifts of asset samples plus declared synthetic extremes."""
import sys
sys.dont_write_bytecode = True
from fractions import Fraction as F
from hashlib import sha256
import json
from pathlib import Path
import struct

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def make():
    metadata_bytes = (ROOT / "assets/mesh.json").read_bytes()
    mesh = json.loads(metadata_bytes)
    mesh_bytes = (ROOT / "assets/mesh_vertices.f32").read_bytes()
    vertices = list(struct.iter_unpack("<4f", mesh_bytes))
    assert len(vertices) == mesh["base_vertices"] == 30480
    assert mesh["cells"] == 600 and mesh["slots"] == 259800
    state = 0x6007
    indices = set()
    while len(indices) < 24:
        state = (1664525 * state + 1013904223) & 0xffffffff
        indices.add(state % len(vertices))
    seeded = sorted(indices)
    # First index wins ties. These are additional geometric extremes of the asset.
    extrema = sorted({fn(range(len(vertices)), key=lambda i: vertices[i][axis])
                      for axis in range(4) for fn in (min, max)})
    samples = []
    for i in sorted(indices | set(extrema)):
        samples.append({"id": f"base-{i}", "origin": "asset-f32-lift-scaled",
                        "asset_index": i, "point": [str(F(v) / 8) for v in vertices[i]]})
    radius, d = F(1, 4096), F(59, 50)
    offsets = (F(0), radius / 2, -radius / 2, radius, -radius,
               radius + F(1, 2**26), -radius - F(1, 2**26), 2 * radius, -2 * radius)
    for pole_name, pole in (("perspective", d), ("north", F(1)), ("equator", F(0))):
        for i, offset in enumerate(offsets):
            samples.append({"id": f"{pole_name}-boundary-{i}", "origin": "synthetic",
                            "point": ["1/2", "-1/4", "1/8", str(pole + offset)]})
    for name, point in (
            ("zero", [F(0)] * 4),
            ("signed-range", [F(2**24), F(-2**24), F(1), F(1, 2)]),
            ("subnormal", [F(1, 2**149), F(-1, 2**149), F(1, 2**126), F(1, 2)]),
            ("overflow", [F(2**128 - 2**104), F(1), F(-1), F(1, 2)])):
        samples.append({"id": name, "origin": "synthetic", "point": [str(v) for v in point]})
    poses = [{"id": name, "matrix": [[str(sign if i == j else 0) for j in range(4)] for i in range(4)]}
             for name, sign in (("identity", 1), ("central-inversion", -1))]
    return {"format": "lx7-inputs-v1", "candidate": "lx7-r1", "decision": "open", "decider": "owner",
            "mesh_sha256": sha256(mesh_bytes).hexdigest(), "mesh_json_sha256": sha256(metadata_bytes).hexdigest(),
            "seed": "0x6007", "sampler": "LCG(1664525,1013904223) mod 2^32; index mod 30480; first 24 unique",
            "seeded_indices": seeded, "extreme_indices": extrema, "asset_scale": "1/8",
            "provenance": "Exact rational lift of stored binary32 mesh, not recovery of pre-rounded geometry. No shrink or cell frames.",
            "d": str(d), "exclusion_radius": str(radius), "vertices": samples, "poses": poses}
