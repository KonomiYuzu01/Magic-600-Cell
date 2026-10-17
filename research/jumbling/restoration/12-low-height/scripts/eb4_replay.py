# E-B4, independent replay: each step word A C A^-1 of eb4_close.py is replayed from the Z pose table on a fresh
# simulator, every twist checked admissible and the digests checked at both ends, and the strict' contract (R21F-01)
# is checked from exact poses: the unit lands, no lattice piece of Z leaves the lattice, and every other off piece of
# Z ends on the lattice or at its exact pose. The units are recomputed at Z as in eb4.py and eb4_close.py (one unit per
# frame part of each rigid block, numbered in block order), so the step files hold only the words.
# Usage (from the repository root):
#   python3 -I -B eb4_replay.py <Z pose table json> <step dir> <out json>
# <step dir> holds one file u<k>.json ({"word", "input_digest", "output_digest"}) per unit k.
import json
import sys
import time
from pathlib import Path

import numpy as np

REPO = Path.cwd()
J = REPO / 'research/jumbling'
sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures')]
import blocks as BL  # noqa: E402
import buffer as B  # noqa: E402
import rim  # noqa: E402

O = B.O
T0 = time.time()


def pose(s, p):
    return B.matrix_json(s._pose_matrix(int(s.pose_id[p])))


def main():
    table, sdir, outp = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
    R, M = O.start(table)
    st = R.st
    z = st.digest()
    bl = BL.Blocks(R, M)
    dom = np.asarray(rim.Domains(st).dom)
    units = []
    for pieces in bl.blocks:
        pieces = np.asarray(pieces)
        for d in np.unique(dom[pieces]).tolist():
            units.append((set(pieces[dom[pieces] == d].tolist()), set(pieces[dom[pieces] != d].tolist())))
    off0 = B.off_pieces(st).tolist()
    pose0 = {p: pose(st, p) for p in off0}
    rows = []
    for f in sorted(sdir.glob('u*.json'), key=lambda x: int(x.stem[1:])):
        k = int(f.stem[1:])
        doc = json.loads(f.read_text())
        assert doc['input_digest'] == z, f'{f.name}: not a step at this state'
        T, F = units[k]
        R2, _ = O.start(table)
        s2 = R2.st
        for c, i in doc['word']:
            R2.apply(int(c), int(i))                # raises when a twist is not admissible
        assert s2.digest() == doc['output_digest'], f'{f.name}: output digest differs'
        off1 = set(B.off_pieces(s2).tolist())
        others = [p for p in off0 if p not in T]
        kept = sum(1 for p in others if p in off1 and pose(s2, p) == pose0[p])
        landed = sum(1 for p in others if p not in off1)
        row = {'unit': k, 'letters': len(doc['word']), 'unit_size': len(T), 'free': len(F),
               'unit_on_lattice': sum(1 for p in T if p not in off1),
               'collateral': len(off1 - set(off0)), 'others': len(others), 'others_exact': kept,
               'others_landed': landed, 'off_after': len(off1)}
        row['strict_prime'] = (row['unit_on_lattice'] == len(T) and row['collateral'] == 0
                               and kept + landed == len(others))
        rows.append(row)
        print(json.dumps(row), file=sys.stderr, flush=True)
    res = {'digest': z, 'off': len(off0), 'steps': len(rows), 'strict_prime': sum(r['strict_prime'] for r in rows),
           'rows': rows, 'seconds': round(time.time() - T0, 1)}
    outp.write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: v for k, v in res.items() if k != 'rows'}))
    assert res['strict_prime'] == len(rows), 'a step breaks the strict-prime contract'


if __name__ == '__main__':
    main()
