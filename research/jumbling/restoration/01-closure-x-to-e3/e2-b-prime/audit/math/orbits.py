"""Audit (math): number of orbits and singleton orbits of the SAFE-ALL group at a table (own union-find).

Usage: python3 -I orbits.py <theory dir> <v26 dir> <table>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import blocks as BL  # noqa: E402
import ghostlib as GL  # noqa: E402
import restore  # noqa: E402
from exact import identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

CL, O = BL.CL, BL.O
reg, kp = CL.ctx.regions, CL.ctx.kplus
R, M = O.start(sys.argv[3])
st = R.st
NP = len(st.pose_id)
K = st._kpose()[st.pose_id]
kid = kp.index_of_matrix(to_tuple(identity(4)))
parent = np.arange(NP)


def find_all(p):
    while True:
        q = p[p]
        if np.array_equal(q, p):
            return p
        p = q


safe = 0
nlet = 0
for c in range(600):
    cl = st.classify(c, need_inside=True, stop_at_straddle=False)
    inside = np.asarray(cl.inside, np.int64)
    if cl.straddle is not None or (len(inside) and (K[inside] < 0).any()):
        continue
    safe += 1
    xs = np.flatnonzero(GL.cap(c))
    for i in range(len(M.items)):
        k = kp.index_of_matrix(restore.element(M, c, i))
        if k < 0 or k == kid:
            continue
        nlet += 1
        ys = reg.piece_of[reg.orbit_of[xs], kp.compose_vec(np.full(len(xs), k), reg.transport[xs])]
        # union xs[j] with ys[j]
        while True:
            parent = find_all(parent)
            a, b = parent[xs], parent[ys]
            m = a != b
            if not m.any():
                break
            lo, hi = np.minimum(a[m], b[m]), np.maximum(a[m], b[m])
            np.minimum.at(parent, hi, lo)
parent = find_all(parent)
labs, counts = np.unique(parent, return_counts=True)
print(json.dumps({'digest': st.digest(), 'safe_grips': safe, 'letters': nlet, 'orbits': int(len(labs)),
                  'singletons': int((counts == 1).sum())}))
