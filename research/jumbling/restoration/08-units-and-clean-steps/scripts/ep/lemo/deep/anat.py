"""Rare-type anatomy of words that land a unit: D(W), Bad(W), S(W) = W(l(Bad \\ D)), B' = Bad(W^-1), W^-1(l(B')).
Prints one JSON line per word. Contents are (domain, chamber); domain 0 is the lattice frame.
usage: anat.py <theory> <echain> <table> <target json> <rows.jsonl> --types 7,9 [--n N] [--skip K]"""
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
import ghostlib as GL  # noqa: E402

table, tfile, rf = sys.argv[3:6]
a = sys.argv[6:]
TYPES = [int(x) for x in a[a.index('--types') + 1].split(',')]
NN = int(a[a.index('--n') + 1]) if '--n' in a else 5
SK = int(a[a.index('--skip') + 1]) if '--skip' in a else 1
C = GL.Ctx(table)
M = C.M
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
RARE = np.isin(TYPE, TYPES)
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
Tr = T[RARE[T]]
ud, uq = C.contents(Tr)


def anat(W):
    grips = sorted({c for c, _ in W})
    R = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in grips]) & RARE)
    g, m, _ = C.ghost(W, Tr, R)
    L = g.q[:m].tolist()
    al, dm, qq = g.alive[m:], g.dom[m:], g.q[m:]
    bad = ~al | (dm != 0)
    out = {'L': L, 'Ldom': g.dom[:m].tolist(), 'U': list(zip(ud.tolist(), uq.tolist())),
           'D': R[~al].tolist(),
           'S': [[int(x), int(dm[i]), int(qq[i])] for i, x in enumerate(R.tolist()) if bad[i] and al[i]]}
    Wi = CP.inverse_word(M, W)
    gi, _, _ = C.ghost(Wi, (), R)
    badi = ~gi.alive | (gi.dom != 0)
    out['Bp'] = [[int(x), int(gi.dom[i]), int(gi.q[i])] if gi.alive[i] else [int(x), None, None]
                 for i, x in enumerate(R.tolist()) if badi[i]]
    out['types'] = {str(x): int(TYPE[x]) for x in set(L) | {s[0] for s in out['S']} | {b[0] for b in out['Bp']}
                    | set(out['D'])}
    return out


rows = [json.loads(x) for x in open(rf) if x.strip()]
for r in rows[::SK][:NN]:
    W = [(int(c), int(i)) for c, i in (r['W'] if 'W' in r else r['Q'])]
    o = anat(W)
    o['len'] = len(W)
    o['W'] = W if len(W) <= 22 else len(W)
    print(json.dumps(o), flush=True)
