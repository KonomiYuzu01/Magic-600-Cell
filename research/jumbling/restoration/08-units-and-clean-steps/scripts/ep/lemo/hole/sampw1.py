"""Sample of depth-one refinements W1 = [W0, k] of landing roots, with |Bad(W1)| and its part in given types from the
ghost library, written as rows that ep/ghostsim.py recounts with the simulator's own classifier.

usage: sampw1.py <theory> <echain> <table> <target json {pieces}> <out jsonl> <rows.jsonl[.gz]>... --types 29,30,32
       [--every N] [--per-root K]
"""
import gzip
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import conjprep as CP  # noqa: E402
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
from exact import identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

table, tfile, out = sys.argv[3:6]
a = sys.argv[6:]
cut = min([a.index(x) for x in a if x.startswith('--')] + [len(a)])
files = a[:cut]
TYPES = [int(x) for x in a[a.index('--types') + 1].split(',')]
EV = int(a[a.index('--every') + 1]) if '--every' in a else 100
PR = int(a[a.index('--per-root') + 1]) if '--per-root' in a else 2
C = GL.Ctx(table)
M, data = C.M, GL.data
kp = GL.BL.CL.ctx.kplus
KID = kp.index_of_matrix(to_tuple(identity(4)))
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
roots = []
for fn in files:
    op = gzip.open if fn.endswith('.gz') else open
    for x in op(fn, 'rt'):
        if x.strip():
            r = json.loads(x)
            roots.append(r['W'] if 'W' in r else r['Q'])
n = 0
with open(out, 'w') as fh:
    for W0l in roots[::EV]:
        W0 = [(int(c), int(i)) for c, i in W0l]
        g, m, _ = C.ghost(W0, T, np.zeros(0, np.int64))
        if not (g.alive[:m].all() and (g.dom[:m] == 0).all()):
            continue
        L = g.q[:m].copy()
        Lset = set(L.tolist())
        Wi = CP.inverse_word(M, W0)
        cand = []
        for f in np.flatnonzero(data.sig_matrix(L).sum(axis=0) == len(L)).tolist():
            for i in range(len(M.items)):
                k = kp.index_of_matrix(GL.restore.element(M, f, i))
                if k >= 0 and k != KID and not Lset & set(FG.chamber_image(k, L).tolist()):
                    cand.append((f, i))
        picked = 0
        for f, i in cand[::max(1, len(cand) // (2 * PR))]:
            j = int(CP.inverse_word(M, [(f, i)])[0][1])
            W1 = W0 + [(f, i)] + Wi + [(f, j)]
            g1, m1, _ = C.ghost(W1, T, np.zeros(0, np.int64))
            if not (g1.alive[:m1].all() and (g1.dom[:m1] == 0).all()):
                continue
            bad, _, _ = C.bad(W1)
            fh.write(json.dumps({'Q': [list(x) for x in W1], 'badQ': int(len(bad)), 'e': None, 'B': None,
                                 'c': None, 'X': None, 'a': None, 'A': None,
                                 'lib_types': {str(t): int((TYPE[bad] == t).sum()) for t in TYPES}}) + '\n')
            picked += 1
            n += 1
            if picked >= PR:
                break
print(json.dumps({'roots': len(roots), 'every': EV, 'rows': n}))
