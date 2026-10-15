"""Which chambers make the S-term of the refinement identity, and does W keep its own landing chambers bad.
usage: refwho.py <theory> <echain> <table> <target json> <q.jsonl[.gz]> --types 7,9 [--max-q N]"""
import gzip
import json
import sys
from collections import Counter
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
MAXQ = int(a[a.index('--max-q') + 1]) if '--max-q' in a else 20
C = GL.Ctx(table)
M, data = C.M, GL.data
kp = GL.BL.CL.ctx.kplus
KID = kp.index_of_matrix(to_tuple(identity(4)))
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
R = np.flatnonzero(np.isin(TYPE, TYPES))
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
rareT = np.isin(TYPE[np.arange(len(TYPE))], TYPES)
op = gzip.open if qf.endswith('.gz') else open
rows = [json.loads(x) for x in op(qf, 'rt') if x.strip()]
tmax = max(r['T_on'] for r in rows)
rows = [r for r in rows if r['T_dead'] == 0 and r['T_on'] == tmax][:MAXQ]
stat = Counter()
for r in rows:
    Q = [(int(c), int(i)) for c, i in r['Q']]
    Qi = CP.inverse_word(M, Q)
    g, m, _ = C.ghost(Q, T, R)
    L = g.q[:m].copy()
    Lr = set(int(x) for x in L if np.isin(TYPE[x], TYPES))
    al, dm, qq = g.alive[m:], g.dom[m:], g.q[m:]
    Bq = set(R[~al | (dm != 0)].tolist())
    stat['L_rare_in_BadQ'] += int(Lr <= Bq)
    content = {int(x): (int(d), int(q)) for x, d, q, a_ in zip(R, dm, qq, al) if a_ and d != 0}
    Soff = set(content.values())
    caps = np.flatnonzero(data.sig_matrix(L).sum(axis=0) == len(L)).tolist()
    for f in caps:
        for i in range(len(M.items)):
            k = kp.index_of_matrix(GL.restore.element(M, f, i))
            if k < 0 or k == KID:
                continue
            j = int(CP.inverse_word(M, [(f, i)])[0][1])
            W = Q + [(f, i)] + Qi + [(f, j)]
            g2, m2, _ = C.ghost(W, T, R)
            if not (g2.alive[:m2].all() and (g2.dom[:m2] == 0).all()):
                continue
            Mland = set(int(x) for x in g2.q[:m2] if np.isin(TYPE[x], TYPES))
            BadW = set(R[~g2.alive[m2:] | (g2.dom[m2:] != 0)].tolist())
            xs = sorted(content)
            gk = FG.Ghost(C.F, np.array([content[x][0] for x in xs]), np.array([content[x][1] for x in xs]))
            gk.apply(f, C.E(f, i), (f, i))
            kS = set(zip(gk.dom[gk.alive].tolist(), gk.q[gk.alive].tolist()))
            Sx = set(x for x in xs if content[x] not in kS)
            stat['landing'] += 1
            stat['Sx_eq_L'] += int(Sx == Lr)
            stat['Sx_sub_L'] += int(Sx <= Lr)
            stat['M_in_BadW'] += int(Mland <= BadW)
            stat['M_eq_L'] += int(Mland == Lr)
            stat['excess3'] += int(len(BadW) == 2 * len(Lr))
print(json.dumps(dict(stat)))
