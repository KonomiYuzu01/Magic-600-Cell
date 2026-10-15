"""Exact replay of a block word W = P Q P^-1 at a state, without vacating (for states where Bad(W) is already vacant).

Loads the pose table, replays P, Q, P^-1 on the simulator (every twist checked admissible by the simulator), and
compares the end with the start as tclose.py does: target block restored, collateral (lattice pieces of the start
that end off), protected off pieces whose exact pose changed, lattice pieces of the start kept on the lattice and
kept at the same exact pose. Writes result.json, word-full.json and the end pose table.

usage: wreplay.py <theory dir> <E-chain scripts dir> <pose table> <target json {pieces}> <W json {P, Q}> <out dir>
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
import conjclose as CC  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

t0 = time.time()
table, tfile, wfile, out = sys.argv[3], sys.argv[4], sys.argv[5], Path(sys.argv[6])
out.mkdir(parents=True, exist_ok=True)
R, M = GL.O.start(table)
st = R.st
Wd = json.loads(Path(wfile).read_text())
P = [(int(c), int(i)) for c, i in Wd['P']]
Q = [(int(c), int(i)) for c, i in Wd['Q']]
Pinv = CP.inverse_word(M, P)
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
codes = CC.Codes()
c_s = codes.of(st)
d_s = st.digest()
assert (c_s[T] < 0).all(), 'target pieces must be off the lattice'
poses_s = {p: to_tuple(st.pose(int(p))) for p in np.flatnonzero(c_s < 0).tolist()}
rows = []
for name, part in (('P', P), ('Q', Q), ('Pinv', Pinv)):
    d_in = st.digest()
    for c, i in part:
        R.apply(c, i)
    rows.append({'part': name, 'twists': len(part), 'input_digest': d_in, 'output_digest': st.digest()})
c_e = codes.of(st)
inT = np.zeros(len(c_s), bool)
inT[T] = True
off_s, off_e = c_s < 0, c_e < 0
lat_s = ~off_s
end = {'digest': st.digest(), 'off': int(off_e.sum()), 'T_restored': int((inT & ~off_e).sum()),
       'collateral': int((lat_s & off_e).sum()),
       'protected_still_off': int((off_s & ~inT & off_e).sum()),
       'protected_exact_pose_changed': sum(1 for p, m in poses_s.items() if not inT[p]
                                           and (not off_e[p] or to_tuple(st.pose(int(p))) != m)),
       'lattice_of_S_on_K_at_end': int((lat_s & ~off_e).sum()), 'lattice_of_S': int(lat_s.sum()),
       'lattice_of_S_same_exact_pose': int((lat_s & (c_e == c_s)).sum())}
full = [list(x) for part in (P, Q, Pinv) for x in part]
(out / 'word-full.json').write_text(json.dumps({'word': full, 'input_digest': d_s, 'output_digest': end['digest']}) + '\n')
res = {'input_digest': d_s, 'V_source': 'none (Bad(W) vacant at the start)', 'parts': rows, 'end': end,
       'length': len(full), 'seconds': round(time.time() - t0, 1)}
(out / 'result.json').write_text(json.dumps(res, indent=1) + '\n')
B.export_pose_table(st, str(out / 'end-table.json'))
print(json.dumps(res['end']), flush=True)
