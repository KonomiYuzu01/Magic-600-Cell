"""Is each off piece of a table wholly outside the body (some facet plane n_f . x = |n|^2 with every posed vertex on
the far side, exactly)? Also: does any off piece overlap a hole (float vertex-hull screen against the hole chambers'
cut constraints is not attempted; only the separation by a facet or by a cut is reported).
Usage: python3 -I outside_check.py <theory dir> <table> <out json>"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import sim  # noqa: E402
from exact import Q5, ZERO  # noqa: E402

ctx = sim.get_context()
data, reg = ctx.data, ctx.regions
S5 = 5 ** 0.5
NFl = np.array([[float(x) for x in row] for row in data.N])
doc = json.loads(Path(sys.argv[2]).read_text())
poses, pid = doc['poses'], doc['pose_id']
NN = data.NN
out = {'table': sys.argv[2], 'pieces': {}}
for j, entry in enumerate(poses):
    pass
# off pieces by float pole test
def fm(e):
    return np.array([[(a + b * S5) / d for a, b, d in row] for row in e])
def onk(F):
    D = (NFl @ F.T) @ NFl.T
    return bool((np.abs(float(NN) - D.max(axis=1)) < 1e-9).all())
used = sorted(set(pid))
offpose = {j for j in used if not onk(fm(poses[j]))}
off = [p for p in range(len(pid)) if pid[p] in offpose]
for p in off:
    m = [[Q5(a, b, d) for a, b, d in row] for row in poses[pid[p]]]
    vs = [[sum((m[r][k] * v[k] for k in range(4)), ZERO) for r in range(4)] for v in reg.vertices(p)]
    vf = np.array([[float(x) for x in v] for v in vs])
    dots = vf @ NFl.T                              # (nv, 600)
    cand = np.flatnonzero((dots >= float(NN) - 1e-9).all(axis=0)).tolist()
    exact_sep = []
    for f in cand:
        n = data.N[f]
        if all((sum((n[k] * v[k] for k in range(4)), ZERO) - NN).sign() >= 0 for v in vs):
            exact_sep.append(f)
    out['pieces'][str(p)] = {'vertices': len(vs), 'separating_facets_exact': exact_sep,
                             'max_facet_excess_min_over_vertices': float((dots - float(NN)).min(axis=0).max())}
print(json.dumps(out))
Path(sys.argv[3]).write_text(json.dumps(out, indent=1) + '\n')
