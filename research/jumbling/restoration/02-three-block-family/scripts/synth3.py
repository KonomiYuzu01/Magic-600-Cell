"""Synthetic frozen-branch table (reachability UNPROVED): move the lattice piece in chamber z0 into the hole chamber y
of the same K+ type, with the least K+ pose that puts it there; z0 becomes a hole. Everything else is unchanged.
The output is a pose table that no word is known to reach; it only exercises the constructor's failure branch.

Usage: python -I synth3.py <theory dir> <v26 dir> <source table> <z0> <y> <out table>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import compare3 as C3  # noqa: E402
import layers3 as L3  # noqa: E402
from sim.kplus import matrix_from_json  # noqa: E402


def main():
    a = sys.argv[1:]
    src, z0, y, out = a[2], int(a[3]), int(a[4]), Path(a[5])
    doc = json.loads(Path(src).read_text())
    kp, reg = L3.kp, L3.reg
    on, k, ch, _, _ = C3.classify(doc)
    occ = np.full(len(on), -1, np.int64)
    occ[ch[on]] = np.flatnonzero(on)
    p = int(occ[z0])
    if p < 0 or occ[y] >= 0:
        raise SystemExit('z0 must be occupied and y a hole')
    if reg.orbit_of[z0] != reg.orbit_of[y]:
        raise SystemExit('z0 and y have different K+ types')
    ks = \
        [j for j in range(7200) if int(L3.FG.chamber_image(j, np.array([p]))[0]) == y]
    m = L3.matrix_json(kp.matrix(ks[0]))
    poses = doc['poses']
    keys = [C3.canon(x) for x in poses]
    ck = C3.canon(m)
    if ck in keys:
        doc['pose_id'][p] = keys.index(ck)
    else:
        poses.append(m)
        doc['pose_id'][p] = len(poses) - 1
    doc['digest'] = C3.table_digest(doc)
    doc['journal'] = []
    doc['synthetic'] = {'reachability': 'UNPROVED', 'source_digest': json.loads(Path(src).read_text())['digest'],
                        'moved_piece': p, 'from_chamber': z0, 'to_chamber': y, 'kplus_choices': ks}
    out.write_text(json.dumps(doc) + '\n')
    print(json.dumps(doc['synthetic'] | {'digest': doc['digest']}))


if __name__ == '__main__':
    main()
