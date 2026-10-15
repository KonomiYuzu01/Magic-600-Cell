"""Recheck rows of a .full.jsonl file at another state: |Bad(W)| per type against the holes per type there (Lemma D),
and whether W still lands the unit. usage: rechk.py <theory> <echain> <table> <target json> <out jsonl> <rows...>"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa: E402

table, tfile, out = sys.argv[3:6]
C = GL.Ctx(table)
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
NT = int(TYPE.max()) + 1
HOLES = np.bincount(TYPE[~C.occ], minlength=NT)
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
fo = open(out, 'w')
for fn in sys.argv[6:]:
    for line in open(fn):
        if not line.strip():
            continue
        r = json.loads(line)
        W = [(int(c), int(i)) for c, i in r['W']]
        g, m, _ = C.ghost(W, T, np.zeros(0, np.int64))
        lands = bool(g.alive[:m].all() and (g.dom[:m] == 0).all())
        bad, _, _ = C.bad(W)
        bt = np.bincount(TYPE[bad], minlength=NT)
        d = np.maximum(bt - HOLES, 0)
        row = {'src': Path(fn).name, 'kind': r.get('kind'), 'len': len(W), 'lands': lands, 'bad': int(len(bad)),
               'occ': int(C.occ[bad].sum()), 'deficit': int(d.sum()),
               'deficit_types': {str(t): [int(bt[t]), int(HOLES[t])] for t in np.flatnonzero(d)},
               'bad_before': r['bad'], 'occ_before': r['occ'], 'W': r['W']}
        fo.write(json.dumps(row) + '\n')
        fo.flush()
        print(json.dumps({k: v for k, v in row.items() if k != 'W'}), flush=True)
