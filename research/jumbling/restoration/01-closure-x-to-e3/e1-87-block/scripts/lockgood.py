"""For every distinct restoring Q (dcomm2 rows) of B': Bad(Q), and whether B''s locked chambers are bad (their content
straddles or ends off). Reads only the state's pose table.
Usage: python -I lockgood.py <theory dir> <pose table> <dcomm2 jsonl> <out jsonl>"""
import json, sys, time
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import ghostlib as GL
C = GL.Ctx(sys.argv[2])
locked = np.array(C.bl.locked[0], np.int64)
seen = set(); T0 = time.time(); n = 0
with open(sys.argv[4], 'w') as fh:
    for line in open(sys.argv[3]):
        r = json.loads(line)
        k = json.dumps(r['Q'])
        if k in seen:
            continue
        seen.add(k)
        Q = [tuple(x) for x in r['Q']]
        bad, g, ch = C.bad(Q, np.union1d(GL.np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c, _ in Q])), locked))
        lk = np.isin(locked, bad)
        idx = {int(z): i for i, z in enumerate(ch.tolist())}
        st = [('dead' if not g.alive[idx[int(z)]] else ('off' if g.dom[idx[int(z)]] != 0 else 'on')) for z in locked]
        fh.write(json.dumps({'Q': r['Q'], 'bad': int(len(bad)), 'locked_bad': int(lk.sum()), 'locked_status': st,
                             'c': r['c'], 'e': r['e'], 'a': r['a']}) + '\n')
        n += 1
print(json.dumps({'distinct': n, 'locked': locked.tolist(), 'sec': round(time.time() - T0)}))
