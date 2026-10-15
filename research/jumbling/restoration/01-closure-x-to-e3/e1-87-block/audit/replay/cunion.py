"""Union of the parked pieces C that Q2 selects (moves), from my after-V export."""
import json
import sys
from pathlib import Path
sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
C0 = S / 'v26/E2/c0'
OUT = S / 'v26/audit/replay'
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent))
import numpy as np
import blocks as BL
O = BL.O
R1, _ = O.start(str(C0 / 'E1-table.json'))
off1 = R1.st._kpose()[R1.st.pose_id] < 0
R, M = O.start(str(OUT / 'state-after-V.json'))
st = R.st
for c, i in json.loads((C0 / 'parts/P.json').read_text())['word']:
    R.apply(c, i)
C = set(np.flatnonzero((st._kpose()[st.pose_id] < 0) & ~off1).tolist())
prev = [st._poses[j].key for j in st.pose_id.tolist()]
sel = []
for c, i in json.loads((C0 / 'parts/Q.json').read_text())['word']:
    R.apply(c, i)
    now = [st._poses[j].key for j in st.pose_id.tolist()]
    sel.append({p for p in C if now[p] != prev[p]})
    prev = now
u = set().union(*sel)
nz = [s for s in sel if s]
out = {'C': len(C), 'union_selected': len(u), 'per_letter': [len(s) for s in sel],
       'same_197_each_time': all(s == nz[0] for s in nz)}
print(json.dumps(out))
(OUT / 'cunion.json').write_text(json.dumps(out) + '\n')
