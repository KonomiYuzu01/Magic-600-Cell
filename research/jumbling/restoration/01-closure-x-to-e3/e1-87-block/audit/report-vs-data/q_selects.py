"""Which parked pieces (C) does Q2 select? Start at the state after V2, apply P2, then step through Q2 and P2^-1
recording, per letter, which pieces of C, T and B' change exact pose. Read-only on inputs."""
import json, sys, time
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa
from sim.kplus import to_tuple  # noqa
O = BL.O
C0 = S / 'v26/E2/c0'
t0 = time.time()
R, M = O.start(str(C0 / 'after-V-table.json'))
st = R.st
print('start digest', st.digest(), 'sec', round(time.time() - t0, 1), flush=True)
pre = json.loads((C0 / 'pre.json').read_text())
T = np.array(pre['target_block']); Bp = np.array(pre['protected_off'])
def word(name):
    return [tuple(x) for x in json.loads((C0 / 'parts' / f'{name}.json').read_text())['word']]
P, Q, Pinv = word('P'), word('Q'), word('Pinv')
for c, i in P:
    R.apply(c, i)
print('after P digest', st.digest(), flush=True)
off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
C = np.setdiff1d(off, np.concatenate([T, Bp]))
print('off after P', len(off), 'C', len(C))
watch = np.concatenate([C, T, Bp])
def poses():
    return [to_tuple(st.pose(int(p))) for p in watch]
nC, nT = len(C), len(T)
unionC = set()
for stage, w in (('Q', Q), ('Pinv', Pinv)):
    stage_union = set()
    for j, (c, i) in enumerate(w):
        before = poses()
        R.apply(c, i)
        after = poses()
        moved = [k for k in range(len(watch)) if before[k] != after[k]]
        mC = {int(watch[k]) for k in moved if k < nC}
        mT = sum(1 for k in moved if nC <= k < nC + nT)
        mB = sum(1 for k in moved if k >= nC + nT)
        stage_union |= mC
        print(stage, j, (c, i), 'C selected', len(mC), 'T selected', mT, "B' selected", mB, flush=True)
    print(stage, 'union of C selected', len(stage_union))
    if stage == 'Q':
        unionC = stage_union
print('end digest', st.digest(), 'sec', round(time.time() - t0, 1))
