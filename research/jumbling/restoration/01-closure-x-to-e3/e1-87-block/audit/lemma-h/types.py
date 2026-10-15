"""Audit lemma-h: type counts (K+ orbits of chambers) of holes and off pieces at a table; Bad(W2) occupancy and types.

Usage: python3 -I types.py <theory dir> <table> [<testB.json>]
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
from sim.kplus import matrix_from_json  # noqa: E402

ctx = sim.get_context()
kp, reg = ctx.kplus, ctx.regions
t = json.loads(Path(sys.argv[2]).read_text())
kidx = np.array([kp.index_of_matrix(matrix_from_json(m)) for m in t['poses']], np.int64)
K = kidx[np.asarray(t['pose_id'], np.int64)]
lat = np.flatnonzero(K >= 0)
off = np.flatnonzero(K < 0)
ch = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
assert len(np.unique(ch)) == len(ch)
occ = np.zeros(len(K), bool)
occ[ch] = True
holes = np.flatnonzero(~occ)
typ = np.asarray(reg.orbit_of)
# lattice piece type equals the type of its chamber
same = bool((typ[ch] == typ[lat]).all())
ht = np.bincount(typ[holes], minlength=36)
ot = np.bincount(typ[off], minlength=36)
out = {'off': int(len(off)), 'holes': int(len(holes)), 'lattice_piece_type_equals_chamber_type': same,
       'holes_by_type_equal_off_by_type': bool(np.array_equal(ht, ot)),
       'types_with_holes': {int(k): int(ht[k]) for k in np.flatnonzero(ht)}}
if len(sys.argv) > 3:
    r = json.loads(Path(sys.argv[3]).read_text())
    bw = np.array(r['badW2'], np.int64)
    out['badW2_occupied_at_table'] = int(occ[bw].sum())
    out['badW2_by_type'] = {int(k): int(v) for k, v in enumerate(np.bincount(typ[bw], minlength=36)) if v}
    good_holes = np.array(r['holes_minus_badU'], np.int64)
    out['good_holes_types'] = typ[good_holes].tolist()
    out['Bp_types'] = typ[np.array([16245, 16249, 32088])].tolist()
    T = np.setdiff1d(off, [16245, 16249, 32088])
    out['T_by_type'] = {int(k): int(v) for k, v in enumerate(np.bincount(typ[T], minlength=36)) if v}
print(json.dumps(out))
