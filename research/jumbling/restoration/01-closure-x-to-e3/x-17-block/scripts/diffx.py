"""Differences between a reference state and X: exact poses per piece, chamber of every lattice piece, and holes.

Usage: python -I diffx.py <theory dir> <reference pose table> <X pose table> <out json>
Exact matrices are compared as JSON entries [a, b, d] = (a + b sqrt5)/d; chambers and holes come from rim.Domains.
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import orient as O  # noqa: E402
import rim  # noqa: E402


def read(table):
    doc = json.loads(Path(table).read_text())
    R, _ = O.start(table)
    st = R.st
    D = rim.Domains(st)
    NP = len(st.pose_id)
    lat = np.asarray(D.dom) == 0
    cham = np.full(NP, -1, np.int64)
    cham[lat] = np.asarray(D.cham)[lat]
    occ = np.zeros(NP, bool)
    occ[cham[lat]] = True
    poses = [json.dumps(m) for m in doc['poses']]
    pose = np.array([poses[i] for i in doc['pose_id']], dtype=object)
    return {'digest': st.digest(), 'lat': lat, 'cham': cham, 'holes': np.flatnonzero(~occ), 'pose': pose}


a = sys.argv[1:]
A, X = read(a[1]), read(a[2])
both_lat = A['lat'] & X['lat']
res = {'reference_digest': A['digest'], 'X_digest': X['digest'],
       'pieces_with_different_exact_pose': int((A['pose'] != X['pose']).sum()),
       'off_in_both': int((~A['lat'] & ~X['lat']).sum()), 'off_only_in_reference': int((~A['lat'] & X['lat']).sum()),
       'off_only_in_X': int((A['lat'] & ~X['lat']).sum()),
       'off_with_different_exact_pose': int(((~A['lat'] & ~X['lat']) & (A['pose'] != X['pose'])).sum()),
       'lattice_in_both': int(both_lat.sum()),
       'lattice_in_a_different_chamber': int((both_lat & (A['cham'] != X['cham'])).sum()),
       'lattice_with_different_exact_pose': int((both_lat & (A['pose'] != X['pose'])).sum()),
       'holes_reference': int(len(A['holes'])), 'holes_in_X': int(len(X['holes'])),
       'holes_common': int(len(np.intersect1d(A['holes'], X['holes']))),
       'holes_X_list': X['holes'].tolist(), 'holes_reference_list': A['holes'].tolist()}
Path(a[3]).write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps({k: v for k, v in res.items() if not isinstance(v, list)}, indent=1))
