"""Compare a start and an end pose table: lattice membership and exact poses counted apart.

The table digests are recomputed here with the standard library (canonical exact matrices, identity skipped, sha256
over sorted pairs of matrix JSON and little-endian int32 member lists) and compared with each table's digest field.
K+ membership and chambers use the model (sim.kplus, regions). Reported: off sets, restored, collateral, start
lattice pieces still on K, of these how many kept their exact pose and how many kept their chamber, the block's end
poses and chambers, and the holes at both ends.

Usage: python -I compare3.py <theory dir> <v26 dir> <start table> <end table> <out json>
"""
import hashlib
import json
import struct
import sys
from math import gcd
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import layers3 as L3  # noqa: E402
from sim.kplus import matrix_from_json  # noqa: E402

IDENTITY = tuple((1, 0, 1) if i == j else (0, 0, 1) for i in range(4) for j in range(4))


def norm_entry(e):
    a, b, d = e
    if d < 0:
        a, b, d = -a, -b, -d
    g = gcd(gcd(a, b), d)
    return (a // g, b // g, d // g) if g > 1 else (a, b, d)


def canon(m):
    return tuple(norm_entry(e) for r in m for e in r)


def table_digest(doc):
    keys = [canon(m) for m in doc['poses']]
    groups = {}
    for piece, i in enumerate(doc['pose_id']):
        groups.setdefault(keys[i], []).append(piece)
    items = []
    for k, members in groups.items():
        if k == IDENTITY:
            continue
        mj = json.dumps([[list(k[4 * i + j]) for j in range(4)] for i in range(4)])
        items.append((mj, struct.pack(f'<{len(members)}i', *sorted(members))))
    items.sort()
    h = hashlib.sha256()
    for mj, b in items:
        h.update(mj.encode())
        h.update(b)
    return h.hexdigest()


def classify(doc):
    kp, reg = L3.kp, L3.reg
    ks = np.array([kp.index_of_matrix(matrix_from_json(m)) for m in doc['poses']], np.int64)
    pid = np.asarray(doc['pose_id'], np.int64)
    k = ks[pid]
    canon_id = {}
    cid = np.array([canon_id.setdefault(canon(m), len(canon_id)) for m in doc['poses']], np.int64)[pid]
    on = k >= 0
    ch = np.full(len(pid), -1, np.int64)
    lat = np.flatnonzero(on)
    ch[lat] = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(k[lat], reg.transport[lat])]
    return on, k, ch, cid, canon_id


def main():
    a = sys.argv[1:]
    s_doc = json.loads(Path(a[2]).read_text())
    e_doc = json.loads(Path(a[3]).read_text())
    out = Path(a[4])
    res = {}
    for name, doc in (('start', s_doc), ('end', e_doc)):
        dg = table_digest(doc)
        res[name] = {'pieces': len(doc['pose_id']), 'digest_recomputed': dg, 'digest_field': doc.get('digest'),
                     'digest_matches': dg == doc.get('digest')}
    on0, k0, ch0, c0, ids0 = classify(s_doc)
    on1, k1, ch1, c1, ids1 = classify(e_doc)
    # exact pose equality across tables: compare canonical matrices, not table-local ids
    inv0 = {v: k for k, v in ids0.items()}
    inv1 = {v: k for k, v in ids1.items()}
    same_pose = np.array([inv0[int(x)] == inv1[int(y)] for x, y in zip(c0, c1)], bool)
    lat0 = on0
    holes = {}
    for name, on, ch in (('start', on0, ch0), ('end', on1, ch1)):
        occ = np.zeros(len(on), bool)
        occ[ch[on]] = True
        holes[name] = np.flatnonzero(~occ).tolist()
    off0, off1 = np.flatnonzero(~on0), np.flatnonzero(~on1)
    res['counts'] = {'off_start': int(len(off0)), 'off_end': int(len(off1)),
                     'restored': int((~on0 & on1).sum()), 'collateral': int((on0 & ~on1).sum()),
                     'start_lattice': int(lat0.sum()), 'start_lattice_on_K_at_end': int((lat0 & on1).sum()),
                     'start_lattice_same_exact_pose': int((lat0 & same_pose).sum()),
                     'start_lattice_pose_changed': int((lat0 & ~same_pose).sum()),
                     'start_lattice_same_chamber': int((lat0 & on1 & (ch0 == ch1)).sum()),
                     'start_lattice_chamber_changed': int((lat0 & on1 & (ch0 != ch1)).sum())}
    res['block'] = {'pieces': off0.tolist(), 'end_on_K': bool(on1[off0].all()),
                    'end_kplus': k1[off0].tolist(), 'end_chambers': ch1[off0].tolist()}
    res['holes'] = {'start': holes['start'], 'end': holes['end']}
    out.write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps(res))


if __name__ == '__main__':
    main()
