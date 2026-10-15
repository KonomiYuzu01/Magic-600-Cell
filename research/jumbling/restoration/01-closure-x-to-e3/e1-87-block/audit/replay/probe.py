"""Probe: load time of E1, digest time, cost of one twist and of the per-prefix bookkeeping."""
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
C0 = S / 'v26/E2/c0'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import json  # noqa: E402

import numpy as np  # noqa: E402

t = time.time()
import blocks as BL  # noqa: E402
O = BL.O
print('import', round(time.time() - t, 1), flush=True)
t = time.time()
R, M = O.start(str(C0 / 'E1-table.json'))
st = R.st
print('start', round(time.time() - t, 1), flush=True)
t = time.time()
d = st.digest()
print('digest', d, round(time.time() - t, 3), flush=True)
K = st._kpose()[st.pose_id]
print('off', int((K < 0).sum()), 'poses', len(st._poses), flush=True)
V = json.loads((C0 / 'parts/V.json').read_text())['word']
t = time.time()
for c, i in V[:20]:
    R.apply(c, i)
print('20 twists', round(time.time() - t, 2), flush=True)
t = time.time()
for _ in range(5):
    d = st.digest()
print('digest x5', round(time.time() - t, 3), 'poses', len(st._poses), flush=True)
t = time.time()
for _ in range(5):
    K = st._kpose()[st.pose_id]
print('kpose x5', round(time.time() - t, 3), flush=True)
t = time.time()
keys = {}
for _ in range(5):
    ids = np.array([keys.setdefault(p.key, len(keys)) for p in st._poses], np.int64)[st.pose_id]
print('keyids x5', round(time.time() - t, 3), len(keys), flush=True)
print('journal', len(st.journal), st.journal[-1].get('moved'), sorted(st.journal[-1].keys()))
