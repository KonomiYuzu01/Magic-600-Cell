"""Apply a word at a pose table, each twist checked; on the first blocked twist report the grip, the status and the
straddling piece (its class: lattice of the start, off at the start, block size), and the counts so far."""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402
import sim  # noqa: E402

O, restore, CL = BL.O, BL.restore, BL.CL
R, M = O.start(sys.argv[2])
st = R.st
word = [(int(c), int(i)) for c, i in json.loads(sys.argv[3])]
bl = BL.Blocks(R, M)
blk = {int(p): len(b) for b in bl.blocks for p in b}
K0 = st._kpose()[st.pose_id].copy()
res = {'input_digest': st.digest(), 'length': len(word)}
for j, (c, i) in enumerate(word):
    out = st.apply(sim.Twist(CL.ctx, c, restore.element(M, c, i), 'menu', {'menu': M.name}))
    if not out.applied:
        cl = st.classify(c, need_inside=True, stop_at_straddle=True)
        p = None
        if cl.straddle is not None:
            p = int(cl.straddle['piece']) if isinstance(cl.straddle, dict) else int(cl.straddle)
        res['blocked'] = {'index': j, 'twist': [c, i], 'status': str(out.status), 'straddling_piece': p,
                          'piece_off_at_start': bool(K0[p] < 0) if p is not None else None,
                          'piece_block_size_at_start': blk.get(p)}
        break
K1 = st._kpose()[st.pose_id]
res['applied'] = j + (0 if 'blocked' in res else 1)
res['off_before'], res['off_now'] = int((K0 < 0).sum()), int((K1 < 0).sum())
res['restored_so_far'] = int(((K0 < 0) & (K1 >= 0)).sum())
res['newly_off_so_far'] = int(((K0 >= 0) & (K1 < 0)).sum())
print(json.dumps(res))
