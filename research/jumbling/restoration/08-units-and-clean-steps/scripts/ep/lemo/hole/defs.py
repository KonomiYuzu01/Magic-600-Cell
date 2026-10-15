"""Per-type deficit at the table's state (Lemma D) of sampled words: |Bad(W)|, occupied Bad, deficit, worst types.
usage: defs.py <theory> <echain> <table> <rows.jsonl> [--every N] [--n N]"""
import json, sys, time
from collections import Counter
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve(); sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa
a = sys.argv[5:]
EV = int(a[a.index('--every') + 1]) if '--every' in a else 1
NN = int(a[a.index('--n') + 1]) if '--n' in a else 10**9
C = GL.Ctx(sys.argv[3])
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
NT = int(TYPE.max()) + 1
holes = np.bincount(TYPE[~C.occ], minlength=NT)
T0 = time.time()
rows = [json.loads(x) for x in open(sys.argv[4]) if x.strip()][::EV][:NN]
hist = Counter()
for r in rows:
    W = [(int(c), int(i)) for c, i in r['W']]
    bad, g, ch = C.bad(W)
    bt = np.bincount(TYPE[bad], minlength=NT)
    d = np.maximum(bt - holes, 0)
    hist[int(d.sum())] += 1
    print(json.dumps({'len': len(W), 'bad': int(len(bad)), 'occ': int(C.occ[bad].sum()), 'deficit': int(d.sum()),
                      'worst': sorted([[int(t), int(bt[t]), int(holes[t])] for t in np.flatnonzero(d)],
                                      key=lambda x: x[2] - x[1])[:4], 'sec': round(time.time() - T0)}), flush=True)
print(json.dumps({'n': len(rows), 'deficit_hist': sorted(hist.items())}))
