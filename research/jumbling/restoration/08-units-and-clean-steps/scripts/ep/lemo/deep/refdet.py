"""Detail of one refinement instance: the S-term chambers and where W sends them.
usage: refdet.py <theory> <echain> <table> <target json> <q.jsonl[.gz]> --types 7,9 [--row N] [--n 2]"""
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

table, tfile, qf = sys.argv[3:6]
a = sys.argv[6:]
TYPES = [int(x) for x in a[a.index('--types') + 1].split(',')]
ROW = int(a[a.index('--row') + 1]) if '--row' in a else 0
NN = int(a[a.index('--n') + 1]) if '--n' in a else 2
C = GL.Ctx(table)
M, data = C.M, GL.data
kp = GL.BL.CL.ctx.kplus
KID = kp.index_of_matrix(to_tuple(identity(4)))
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
R = np.flatnonzero(np.isin(TYPE, TYPES))
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
Trare = T[np.isin(TYPE[T], TYPES)]
op = gzip.open if qf.endswith('.gz') else open
rows = [json.loads(x) for x in op(qf, 'rt') if x.strip()]
tmax = max(r['T_on'] for r in rows)
rows = [r for r in rows if r["T_dead"] == 0 and r["T_on"] == tmax]
r = rows[ROW]
Q = [(int(c), int(i)) for c, i in r['Q']]
Qi = CP.inverse_word(M, Q)
print('Q', Q)
ud, uq = C.contents(Trare)
print('U rare contents (dom,q,type)', list(zip(ud.tolist(), uq.tolist(), TYPE[uq].tolist())))
g, m, _ = C.ghost(Q, Trare, R)
L = g.q[:m].tolist()
print('L', L, 'types', TYPE[L].tolist())
al, dm, qq = g.alive[m:], g.dom[m:], g.q[m:]
Bq = R[~al | (dm != 0)]
print('Bad(Q) rare', Bq.tolist(), 'dead', R[~al].tolist())
for x in Bq.tolist():
    ix = int(np.searchsorted(R, x))
    print('   x', x, 't', int(TYPE[x]), '-> Q content', (int(dm[ix]), int(qq[ix])) if al[ix] else 'dead')
gi, _, _ = C.ghost(Qi, (), R)
Bp = R[~gi.alive | (gi.dom != 0)]
print('Bad(Q^-1) rare', Bp.tolist())
for x in Bp.tolist():
    ix = int(np.searchsorted(R, x))
    print('   w', x, 't', int(TYPE[x]), '-> Q^-1 content', (int(gi.dom[ix]), int(gi.q[ix])) if gi.alive[ix] else 'dead')
caps = np.flatnonzero(data.sig_matrix(np.array(L)).sum(axis=0) == len(L)).tolist()
done = 0
for f in caps:
    for i in range(len(M.items)):
        k = kp.index_of_matrix(GL.restore.element(M, f, i))
        if k < 0 or k == KID or done >= NN:
            continue
        j = int(CP.inverse_word(M, [(f, i)])[0][1])
        W = Q + [(f, i)] + Qi + [(f, j)]
        g2, m2, _ = C.ghost(W, Trare, R)
        if not (g2.alive[:m2].all() and (g2.dom[:m2] == 0).all()):
            continue
        done += 1
        print('== f', f, 'i', i, 'k', k, 'cap f holds Bad(Q) rare:', [int(GL.cap(f)[x]) for x in Bq.tolist()])
        print('   k~(L)', [int(FG.chamber_image(k, np.array([x]))[0]) for x in L], 'M', g2.q[:m2].tolist())
        BW = R[~g2.alive[m2:] | (g2.dom[m2:] != 0)]
        print('   Bad(W) rare', BW.tolist())
        for x in BW.tolist():
            ix = int(np.searchsorted(R, x))
            print('     x', x, 't', int(TYPE[x]), 'W content', (int(g2.dom[m2 + ix]), int(g2.q[m2 + ix])) if g2.alive[m2 + ix] else 'dead')
        # the U contents moved by k after Q^-1? ghost of U under Q k Q^-1
        g3, m3, _ = C.ghost(Q + [(f, i)] + Qi, Trare, ())
        print('   U after Q k Q^-1', list(zip(g3.dom[:m3].tolist(), g3.q[:m3].tolist())))
