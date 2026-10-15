"""report-vs-data audit: exact-pose change sets along X -> E1 -> E2 -> E3, block overlaps, chamber changes.

Usage: python3 -I chain.py <theory dir> <X table> <E1 table> <E2 table> <E3 table> <out json>
"""
import hashlib
import json
import sys
import time
from math import gcd
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402
import rim  # noqa: E402

T0 = time.time()
O = BL.O


def norm(e):
    a, b, d = e
    g = gcd(gcd(abs(a), abs(b)), abs(d)) or 1
    a, b, d = a // g, b // g, d // g
    if d < 0:
        a, b, d = -a, -b, -d
    return (a, b, d)


def poses_of(path):
    doc = json.loads(Path(path).read_text())
    mats = [tuple(norm(e) for row in m for e in row) for m in doc['poses']]
    # map each distinct matrix to a global key
    return doc, mats


def main():
    a = sys.argv[1:]
    names = ['X', 'E1', 'E2', 'E3']
    tables = dict(zip(names, a[1:5]))
    keys = {}
    gid = {}
    off = {}
    cham = {}
    digest = {}
    for n in names:
        doc, mats = poses_of(tables[n])
        ids = np.array([gid.setdefault(m, len(gid)) for m in mats], np.int64)
        keys[n] = ids[np.asarray(doc['pose_id'], np.int64)]
        R, M = O.start(tables[n])
        st = R.st
        digest[n] = st.digest()
        K = st._kpose()[st.pose_id]
        off[n] = K < 0
        D = rim.Domains(st)
        c = D.cham.copy()
        c[D.dom != 0] = -1
        cham[n] = c
    out = {'digests': digest, 'off_counts': {n: int(off[n].sum()) for n in names}}

    def changed(x, y):
        return np.flatnonzero(keys[x] != keys[y])
    ch = {f'{x}_to_{y}': changed(x, y) for x, y in [('X', 'E1'), ('E1', 'E2'), ('E2', 'E3'), ('E1', 'E3'), ('X', 'E3'),
                                                    ('X', 'E2')]}
    out['changed_counts'] = {k: int(len(v)) for k, v in ch.items()}
    b17 = np.flatnonzero(off['X'] & ~off['E1'])
    b87 = np.flatnonzero(off['E1'] & ~off['E2'])
    bp = np.flatnonzero(off['E2'])
    out['block17'] = int(len(b17))
    out['block87'] = int(len(b87))
    out['bprime'] = bp.tolist()
    out['X_off_minus_blocks'] = int(off['X'].sum() - len(b17) - len(b87) - len(bp))
    out['X_to_E1_equals_17block'] = bool(np.array_equal(ch['X_to_E1'], b17))
    out['E1_to_E2_equals_87block'] = bool(np.array_equal(ch['E1_to_E2'], b87))
    e2e3 = ch['E2_to_E3']
    lat_changed = np.setdiff1d(e2e3, bp)
    out['E2_to_E3_includes_bprime'] = bool(np.isin(bp, e2e3).all())
    out['E2_lattice_changed'] = int(len(lat_changed))
    out['E2_lattice_changed_all_lattice_at_E2'] = bool((~off['E2'][lat_changed]).all())
    out['E2_lattice_changed_chamber'] = int((cham['E2'][lat_changed] != cham['E3'][lat_changed]).sum())
    out['E3_all_on_K'] = bool((~off['E3']).all())
    out['overlap_87_with_E2E3_lattice_changed'] = int(np.isin(b87, lat_changed).sum())
    out['overlap_17_with_E2E3_lattice_changed'] = int(np.isin(b17, lat_changed).sum())
    # predicted sizes from the block-sum argument
    out['E1_to_E3_predicted'] = int(len(b87) + len(bp) + len(lat_changed) - out['overlap_87_with_E2E3_lattice_changed'])
    out['X_to_E3_predicted'] = int(len(b17) + len(b87) + len(bp) + len(lat_changed)
                                   - out['overlap_87_with_E2E3_lattice_changed']
                                   - out['overlap_17_with_E2E3_lattice_changed'])
    # pieces back at their earlier pose (changed in an earlier segment and returned)
    out['E1_to_E3_set_equals_union_minus_returned'] = None
    u = np.union1d(np.union1d(b87, bp), lat_changed)
    out['E1_to_E3_equals_union_87_bp_latchanged'] = bool(np.array_equal(ch['E1_to_E3'], u))
    u2 = np.union1d(u, b17)
    out['X_to_E3_equals_union_17_87_bp_latchanged'] = bool(np.array_equal(ch['X_to_E3'], u2))
    lat_ids_cc = json.loads(Path(a[5]).read_text())['E2_lattice_pieces_with_changed_exact_pose_ids'] if len(a) > 6 else None
    if lat_ids_cc is not None:
        out['chain_check_ids_equal'] = sorted(lat_ids_cc) == lat_changed.tolist()
    out['sec'] = round(time.time() - T0, 1)
    Path(a[-1]).write_text(json.dumps(out, indent=1) + '\n')
    print(json.dumps(out))


if __name__ == '__main__':
    main()
