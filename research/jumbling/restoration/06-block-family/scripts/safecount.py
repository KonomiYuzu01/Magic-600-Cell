"""Count SAFE-ALL grips (unstraddled caps with no off piece inside) and the hole count of a pose table.

usage: safecount.py <theory dir> <E-chain scripts dir> <pose table> ...
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import buffer as B  # noqa: E402
import ghostlib as GL  # noqa: E402

for table in sys.argv[3:]:
    t0 = time.time()
    R, M = GL.O.start(table)
    st = R.st
    lat = st.lattice_flags()
    safe, straddled, offin = 0, 0, 0
    for c in range(600):
        cl = st.classify(c, need_inside=True, stop_at_straddle=False)
        oi = int((~lat[np.asarray(cl.inside, np.int64)]).sum()) if cl.inside is not None else 0
        if cl.straddle is None and not oi:
            safe += 1
        elif cl.straddle is not None:
            straddled += 1
        else:
            offin += 1
    print(json.dumps({'table': Path(table).name, 'off': int((~lat).sum()), 'holes': int(len(B.hole_set(st))),
                      'safe_grips': safe, 'straddled_grips': straddled, 'unstraddled_with_off_inside': offin,
                      'sec': round(time.time() - t0, 1)}), flush=True)
