"""List single letters (g, i) at E2 that put B' on the lattice: g a grip with B' inside an admissible cut, i a menu item
with E(g, i) m in K+ (m: B''s common exact pose). Reads only the pose table; every test is exact.

Usage: python -I singleP.py <theory dir> <pose table> <out json>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402
from exact import matmul  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

CL, O, restore = BL.CL, BL.O, BL.restore
kp = CL.ctx.kplus


def main():
    a = sys.argv[1:]
    R, M = O.start(a[1])
    st = R.st
    bl = BL.Blocks(R, M)
    out = []
    for b, pcs in enumerate(bl.blocks):
        poses = {to_tuple(st.pose(int(p))) for p in pcs}
        m = st.pose(int(pcs[0]))
        for g in bl.I[b].tolist():
            if R.inside(g) is None:
                continue
            for i in range(len(M.items)):
                e = restore.element(M, g, i)
                if kp.index_of_matrix(to_tuple(e)) >= 0:
                    continue
                if kp.index_of_matrix(to_tuple(matmul(e, m))) >= 0:
                    out.append({'block': b, 'size': len(pcs), 'grip': g, 'item': i, 'P': [[g, i]],
                                'one_pose': len(poses) == 1})
    Path(a[2]).write_text(json.dumps(out) + '\n')
    print(len(out), [(r['grip'], r['item']) for r in out])


if __name__ == '__main__':
    main()
