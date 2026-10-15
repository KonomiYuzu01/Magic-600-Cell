"""Audit 'certificate', negative control detail: after W applied directly at E2, does each occupant of Bad(W) occupy
exactly the region of a B' piece at E2? Region of piece p at pose m is m(home_p); equal regions g(home_b) and
m(home_p) hold iff k = g^-1 m is in K+ and maps chamber p to chamber b.

Usage: python3 -I swap.py <theory dir> <out json>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
S = Path('<work>')
B4 = S / 'v26/B4'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(S / 'v26'))
from blocks import CL, O  # noqa: E402
from exact import matmul, transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

BPRIME = [16245, 16249, 32088]
OCC = {23354: 69058, 23362: 69059, 38667: 121428}
reg, kp = CL.ctx.regions, CL.ctx.kplus
R, M = O.start(str(B4 / 'E2-table.json'))
st = R.st
g = st.pose(BPRIME[0])
parts = {n: [tuple(int(v) for v in x) for x in json.loads((B4 / 'c3/parts' / f'{n}.json').read_text())['word']]
         for n in ('P', 'Q', 'Pinv')}
W = parts['P'] + parts['Q'] + parts['Pinv']
for c, i in W:
    R.apply(c, i)
res = {}
for z, p in OCC.items():
    m = st.pose(p)
    k = kp.index_of_matrix(to_tuple(matmul(transpose(g), m)))
    img = int(reg.piece_of[reg.orbit_of[p], kp.compose_vec(np.array([k]), np.array([reg.transport[p]]))[0]]) if k >= 0 else None
    res[str(p)] = {'from_chamber': z, 'g_inv_m_in_Kplus': k >= 0, 'region_equals_Bprime_piece_region': img}
print(json.dumps(res))
Path(sys.argv[2]).write_text(json.dumps(res, indent=1) + '\n')
