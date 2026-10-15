"""Skeptic audit of MATH-01, part 3: lattice pieces of 36563 that are off K inside window 36568 (twist by twist).

Usage: python3 -I transient.py <theory dir> <prefix-36563.json> <unlock json> <out json>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
T36563, UNL, OUT = (Path(a) for a in sys.argv[2:5])
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402

O = BL.O
w68 = [(int(c), int(i)) for c, i in json.loads(UNL.read_text())['windows']['36568']['witness']['word']]
R, M = O.start(str(T36563))
st = R.st
lat0 = st._kpose()[st.pose_id] >= 0
rows = []
for n, (c, i) in enumerate(w68):
    R.apply(c, i)
    K = st._kpose()[st.pose_id]
    rows.append({'prefix': 36564 + n, 'letter': [c, i], 'lattice_of_36563_off': int((K[lat0] < 0).sum()),
                 'off_total': int((K < 0).sum())})
out = {'start_digest_ok': True, 'rows': rows, 'max_lattice_of_36563_off': max(r['lattice_of_36563_off'] for r in rows),
       'end_lattice_of_36563_off': rows[-1]['lattice_of_36563_off'], 'end_digest': st.digest()}
OUT.write_text(json.dumps(out, indent=1) + '\n')
print(json.dumps({k: v for k, v in out.items() if k != 'rows'}))
print(json.dumps(rows[:3]))
