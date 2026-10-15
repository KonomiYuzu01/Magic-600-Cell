"""Probe: load E2 with the simulator, time the reference classifier at E2 and after R = (237,13)."""
import sys, time, json
sys.dont_write_bytecode = True
from pathlib import Path
SP = Path('<work>')
TH = SP / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent))
import numpy as np
T0 = time.time()
import blocks as BL
O = BL.O
print('import', round(time.time() - T0, 1), flush=True)
R, M = O.start(str(SP / 'v26/B4/E2-table.json'))
st = R.st
print('start', round(time.time() - T0, 1), st.digest(), flush=True)
K = st._kpose()[st.pose_id]
print('off', np.flatnonzero(K < 0).tolist(), 'nposes', len(st._poses), flush=True)
t = time.time(); cl = st.classify_per_piece(237, need_inside=True, stop_at_straddle=False)
print('ref classify 237 at E2', round(time.time() - t, 2), 'straddle', cl.straddle is not None, 'unc', cl.uncertain, 'inside', len(cl.inside), flush=True)
t = time.time(); cl2 = st.classify(237, need_inside=True, stop_at_straddle=True)
print('fast classify 237 at E2', round(time.time() - t, 2), 'straddle', cl2.straddle is not None, 'inside', len(cl2.inside), flush=True)
t = time.time(); R.apply(237, 13); print('apply', round(time.time() - t, 2), 'moved', st.journal[-1]['moved'], flush=True)
K = st._kpose()[st.pose_id]
print('off after R', int((K < 0).sum()), flush=True)
t = time.time(); cl = st.classify_per_piece(91, need_inside=True, stop_at_straddle=False)
print('ref classify 91 after R', round(time.time() - t, 2), 'straddle', cl.straddle is not None, 'unc', cl.uncertain, 'inside', len(cl.inside), flush=True)
t = time.time(); cl = st.classify_per_piece(237, need_inside=True, stop_at_straddle=False)
print('ref classify 237 after R', round(time.time() - t, 2), 'straddle', cl.straddle is not None, 'unc', cl.uncertain, 'inside', len(cl.inside), flush=True)
print('total', round(time.time() - T0, 1))
