"""Audit 'certificate': Bad sets by a second, independent code path (simulator ghost of every home chamber, as in
conjprep.bad_chambers: all 177,120 pieces at the identity, straddlers killed per twist), for W, P4^-1 and the whole word.

Usage: python3 -I bad2.py <theory dir> <which: W|Pinv|U> <out json>
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
S = Path('<work>')
B4 = S / 'v26/B4'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(S / 'v26'))
import conjprep as CP  # noqa: E402
import restore  # noqa: E402

T0 = time.time()
which, out = sys.argv[2], Path(sys.argv[3])
parts = {n: [tuple(int(v) for v in x) for x in json.loads((B4 / 'c3/parts' / f'{n}.json').read_text())['word']]
         for n in ('V', 'P', 'Q', 'Pinv', 'Vinv')}
word = {'W': parts['P'] + parts['Q'] + parts['Pinv'], 'Pinv': parts['Pinv'],
        'U': parts['V'] + parts['P'] + parts['Q'] + parts['Pinv'] + parts['Vinv']}[which]
M = restore.menu('I_a')
NP = 177120
bad, alive = CP.bad_chambers(M, word, NP)
res = {'which': which, 'length': len(word), 'bad': np.flatnonzero(bad).tolist(), 'bad_count': int(bad.sum()),
       'unclean': int((~alive).sum()), 'unclean_chambers': np.flatnonzero(~alive).tolist()[:50],
       'seconds': round(time.time() - T0, 1)}
out.write_text(json.dumps(res) + '\n')
print(json.dumps({k: v for k, v in res.items() if k != 'unclean_chambers'})[:600])
