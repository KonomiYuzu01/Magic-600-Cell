"""Audit lemma-h: is an off piece outside the 600-cell body? Facet hyperplanes n_f . x = |n|^2 (checked on chambers).

Usage: python3 -I outside.py <theory dir> <table> <json list of pieces>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import sim  # noqa: E402
from exact import ZERO, matvec  # noqa: E402
from sim.kplus import matrix_from_json  # noqa: E402

ctx = sim.get_context()
kp, reg, data = ctx.kplus, ctx.regions, ctx.data
NN = data.NN
t = json.loads(Path(sys.argv[2]).read_text())
pieces = json.loads(sys.argv[3])


def fvals(verts):
    """Exact n_f . v - NN for all facets f and vertices v (list of lists)."""
    return [[sum((data.N[f][k] * v[k] for k in range(4)), ZERO) - NN for v in verts] for f in range(600)]


# convention check on 300 random chambers: every vertex satisfies n_f . v <= NN, and some vertex of a surface
# chamber reaches equality
rng = np.random.default_rng(3)
viol, touch = 0, 0
for z in rng.choice(len(data.sigbits), 300, replace=False).tolist():
    vals = fvals(reg.vertices(z))
    viol += sum(1 for row in vals for x in row if x.sign() > 0)
    touch += int(any(x.sign() == 0 for row in vals for x in row))
out = {'chamber_check_vertices_outside_body': viol, 'chambers_touching_a_facet_of_300': touch, 'pieces': []}
for p in pieces:
    m = matrix_from_json(t['poses'][t['pose_id'][p]])
    verts = [matvec(m, v) for v in reg.vertices(p)]
    vals = fvals(verts)
    sep = [f for f in range(600) if all(x.sign() >= 0 for x in vals[f]) and any(x.sign() > 0 for x in vals[f])]
    outside_vertices = sum(1 for j in range(len(verts)) if any(vals[f][j].sign() > 0 for f in range(600)))
    out['pieces'].append({'piece': p, 'vertices': len(verts), 'vertices_outside_body': outside_vertices,
                          'facets_with_piece_wholly_outside': sep})
print(json.dumps(out))
