"""Exact cap sets of the off pieces of a pose table (menu I_a): for every off piece, the grips whose closed cap
wholly contains its posed region ('in') and those that straddle it, by the exact evaluator on every grip the float
test does not place out with margin. Groups pieces with identical 'in' sets (such pieces move together under every
admissible twist that moves one of them).
usage: capsets.py <research/jumbling> <classes.json> <arrays.npz> <pose table> <out json>"""
import json, sys, time
from collections import defaultdict
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from e3lib import Lib  # noqa: E402
t0 = time.time()
L = Lib(sys.argv[1], sys.argv[2], sys.argv[3])
from sim.kplus import matrix_from_json  # noqa: E402
from sim.state import Pose  # noqa: E402
doc = json.loads(Path(sys.argv[4]).read_text())
st = L.sim.State(L.ctx, menu=L.M)
st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
ids = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
st.pose_id = ids[np.asarray(doc['pose_id'], np.int64)]
st._compact()
st.journal, st._anchor_cache = [], {}
assert st.digest() == doc['digest']
NF = np.array(L.d.NF, float)
KAP = float(L.d.KAPPA)
repV = [np.array([[float(v) for v in vert] for vert in vs]) for vs in L.reg.rep_vertices]
off = np.flatnonzero(st._kpose()[st.pose_id] < 0).tolist()
rows = {}
for p in off:
    V = (repV[L.orb[p]] @ L.Kf[L.tr[p]].T) @ L.fm(st.pose(p)).T
    vals = V @ NF.T - KAP
    maybe = np.flatnonzero(~(vals < -1e-6).all(0))      # not certainly out
    ins, strad = [], []
    for c in maybe.tolist():
        s = L.status(st, p, c)
        if s == 'in':
            ins.append(c)
        elif s != 'out':
            strad.append(c)
    rows[p] = {'piece': p, 'in': ins, 'straddle': strad, 'checked_exactly': len(maybe)}
groups = defaultdict(list)
for p, r in rows.items():
    groups[tuple(r['in'])].append(p)
out = {'table_digest': doc['digest'], 'off': len(off), 'pieces': list(rows.values()),
       'groups_with_identical_in_sets': [{'in': list(k), 'pieces': v} for k, v in groups.items() if len(v) > 1],
       'seconds': round(time.time() - t0, 1)}
json.dump(out, open(sys.argv[5], 'w'), indent=1)
print(json.dumps({k: out[k] for k in ('off', 'groups_with_identical_in_sets', 'seconds')})[:2000])
for r in list(rows.values())[:10]:
    print(r)
