"""Probe: load E2 and print basic facts (audit 'shrink'; read-only)."""
import sys, json, time, hashlib
sys.dont_write_bytecode = True
from pathlib import Path
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent))
import numpy as np
t0 = time.time()
import blocks as BL
O = BL.O
CL = BL.CL
restore = BL.restore
from sim.kplus import to_tuple, matrix_json
E2 = S / 'v26/B4/E2-table.json'
doc = json.loads(E2.read_text())
R, M = O.start(str(E2))
st = R.st
print('load', round(time.time()-t0,1), 'digest', st.digest(), doc['digest'])
kp, reg, data = CL.ctx.kplus, CL.ctx.regions, CL.ctx.data
K = st._kpose()[st.pose_id]
off = np.flatnonzero(K < 0)
print('off', off.tolist(), 'NP', len(st.pose_id))
print('poses of off', {int(p): matrix_json(st.pose(int(p))) == matrix_json(st.pose(int(off[0]))) for p in off})
lat = np.flatnonzero(K >= 0)
ch = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
occ = np.zeros(len(st.pose_id), bool); occ[ch] = True
print('unique chambers', len(np.unique(ch)) == len(ch), 'holes', np.flatnonzero(~occ).tolist())
print('menu items', len(M.items), [M.items[i][0] for i in range(min(5, len(M.items)))])
I4 = restore.I4
ident = [i for i in range(len(M.items)) if restore.element(M, 237, i) == I4]
print('identity items at 237', ident)
kitems = [i for i in range(len(M.items)) if kp.index_of_matrix(restore.element(M, 237, i)) >= 0]
print('K+ items at 237', kitems)
cap237 = np.flatnonzero(data.sig_matrix(np.arange(len(st.pose_id)))[:, 237])
print('cap237 size', len(cap237), 'holes in cap237', np.isin(np.flatnonzero(~occ), cap237).tolist())
cl = st.classify(237, need_inside=True, stop_at_straddle=False)
ins = np.asarray(cl.inside)
print('237 inside', len(ins), 'off inside', np.isin(off, ins).tolist(), 'straddle', cl.straddle is not None)
print('lattice inside 237', int((K[ins] >= 0).sum()))
print('time', round(time.time()-t0,1))
