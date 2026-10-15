"""Audit 'words': replay the five parts from E1-table.json and check each part's digests by simulation.

Usage: python3 -I replay_chain.py <out.json>       (replay)
       python3 -I replay_chain.py --digest <table>  (digest of a table only)
"""
import json
import sys
import time
from pathlib import Path

SP = Path('<work>')
C0 = SP / 'v26' / 'E2' / 'c0'
TH = SP / 'wt-e3' / 'research' / 'jumbling' / 'theory'
sys.dont_write_bytecode = True
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import numpy as np  # noqa: E402
import blocks as BL  # noqa: E402

if sys.argv[1] == '--digest':
    R, M = BL.O.start(sys.argv[2])
    off = int((R.st._kpose()[R.st.pose_id] < 0).sum())
    print(json.dumps({'table': sys.argv[2], 'digest': R.st.digest(), 'off': off}))
    sys.exit(0)

t0 = time.time()
R, M = BL.O.start(str(C0 / 'E1-table.json'))
st = R.st
rows = []
for k in ['V', 'P', 'Q', 'Pinv', 'Vinv']:
    doc = json.loads((C0 / 'parts' / f'{k}.json').read_text())
    din = st.digest()
    err = None
    n = 0
    for c, i in doc['word']:
        try:
            R.apply(c, i)
        except RuntimeError as e:
            err = f'letter {n}: {e}'
            break
        n += 1
    dout = st.digest()
    off = int((st._kpose()[st.pose_id] < 0).sum())
    row = dict(part=k, applied=n, of=len(doc['word']), error=err, input_ok=din == doc['input_digest'],
               output_ok=dout == doc['output_digest'], digest_in=din, digest_out=dout, off=off,
               sec=round(time.time() - t0, 1))
    rows.append(row)
    print(json.dumps(row), flush=True)
    if err:
        break
Path(sys.argv[1]).write_text(json.dumps(rows, indent=1))
