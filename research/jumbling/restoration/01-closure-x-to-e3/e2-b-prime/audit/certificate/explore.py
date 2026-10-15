"""Exploration: E2 holes, SAFE-ALL orbits of holes and grips whose caps hold them (audit 'certificate')."""
import json, sys, time
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(S / 'v26'))
T0 = time.time()
import blocks as BL
import buffer as B
O = BL.O
R, M = O.start(str(S / 'v26/B4/E2-table.json'))
st = R.st
print('load', round(time.time() - T0, 1), 'digest', st.digest(), flush=True)
off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
print('off', off.tolist())
holes = B.hole_set(st)
print('holes', holes.tolist())
G = B.Alphabet(R, M)
G.orbits()
print('alphabet', round(time.time() - T0, 1), len(G.safe), len(G.letters), len(G.orbit_size), int((G.orbit_size == 1).sum()))
for h in holes.tolist() + [23354, 23362, 38667]:
    grips = np.flatnonzero(G.home[h])
    print(h, 'orbit', int(G.orbit[h]), 'size', int(G.orbit_size[G.orbit[h]]), 'ncaps', len(grips),
          'safe caps', [int(c) for c in grips if c in set(G.safe)][:20], 'blocked caps', [int(c) for c in grips if c not in set(G.safe)])
