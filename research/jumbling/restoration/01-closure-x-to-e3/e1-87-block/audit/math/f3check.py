"""Math audit: T after P2 Q2 lies on K and outside Bad(P2^-1) (the report's premise F3), from E1-table.json and parts.
Usage: python3 -I f3check.py <theory dir> <v26 dir> <out json>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
V26 = Path(sys.argv[2]).resolve()
OUT = Path(sys.argv[3])
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(V26))
import ghostlib as GL  # noqa: E402

C0 = V26 / 'E2' / 'c0'


def word(name):
    return [(int(c), int(i)) for c, i in json.loads((C0 / 'parts' / f'{name}.json').read_text())['word']]


ctx = GL.Ctx(str(C0 / 'E1-table.json'))
P, Q, Pinv = word('P'), word('Q'), word('Pinv')
T = ctx.block(87)
g, n, _ = ctx.ghost(P + Q, T, np.zeros(0, np.int64))
on = g.alive[:n] & (g.dom[:n] == 0)
qT = g.q[:n][on]
bPi, _, _ = ctx.bad(Pinv, np.arange(len(ctx.st.pose_id)))
out = {'T_on_K_after_PQ': int(on.sum()), 'badPinv': int(len(bPi)), 'T_after_PQ_in_badPinv': int(np.isin(qT, bPi).sum())}
OUT.write_text(json.dumps(out, indent=1) + '\n')
print(json.dumps(out), file=sys.stderr)
