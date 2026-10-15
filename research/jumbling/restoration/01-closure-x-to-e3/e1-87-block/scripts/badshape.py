"""Shape of Bad(Q) for B' Q candidates: caps holding most of Bad(Q), chambers outside the best cap intersection,
and the locked chambers of B'. Reads only the state's pose table.
Usage: python -I badshape.py <theory dir> <pose table> <Q list json> <count> <out json>"""
import json, sys
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import ghostlib as GL
C = GL.Ctx(sys.argv[2])
Qs = json.loads(Path(sys.argv[3]).read_text())[:int(sys.argv[4])]
locked = [84069, 84070, 84071]
out = []
for Q in Qs:
    Q = [tuple(x) for x in Q]
    bad, g, ch = C.bad(Q)
    sig = GL.data.sig_matrix(bad)
    cnt = sig.sum(axis=0)
    top = np.argsort(-cnt)[:8]
    full = np.flatnonzero(cnt == len(bad)).tolist()
    # best triple: three caps with the largest common count
    caps = top[:6].tolist()
    best = None
    for i in range(len(caps)):
        for j in range(i + 1, len(caps)):
            for k in range(j + 1, len(caps)):
                n = int((sig[:, caps[i]] & sig[:, caps[j]] & sig[:, caps[k]]).sum())
                if best is None or n > best[0]:
                    best = (n, [caps[i], caps[j], caps[k]])
    inter = sig[:, best[1][0]] & sig[:, best[1][1]] & sig[:, best[1][2]]
    outside = bad[~inter]
    dead = int((~g.alive).sum())
    out.append({'Q': [list(x) for x in Q], 'bad': int(len(bad)), 'caps_containing_all': full,
                'top_caps': [[int(c), int(cnt[c])] for c in top], 'best_triple': best[1], 'in_triple': best[0],
                'outside_triple': outside.tolist()[:40], 'n_outside': int(len(outside)),
                'locked_in_bad': [int(x) for x in locked if x in set(bad.tolist())],
                'straddle_dead': dead, 'ends_off': int(len(bad) - dead)})
    print(json.dumps({k: v for k, v in out[-1].items() if k not in ('Q', 'outside_triple')}), flush=True)
Path(sys.argv[5]).write_text(json.dumps(out, indent=1) + '\n')
