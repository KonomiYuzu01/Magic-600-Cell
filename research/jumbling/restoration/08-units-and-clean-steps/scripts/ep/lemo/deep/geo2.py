"""Sides of U, of the copy (S(W) minus U) and of X' = B'(W) minus L against every cap that holds the landing of W
(types 7 and 9); one summary line per word and totals.
usage: geo2.py <theory> <echain> <table> <target json> <rows.jsonl> <n> <skip>"""
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
import ghostlib as GL  # noqa: E402

table, tfile, rf = sys.argv[3:6]
NN, SK = int(sys.argv[6]), int(sys.argv[7])
C = GL.Ctx(table)
M, data = C.M, GL.data
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
RARE = np.isin(TYPE, [7, 9])
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
Tr = T[RARE[T]]
ud, uq = C.contents(Tr)
Uc = set(zip(ud.tolist(), uq.tolist()))


def side(d, c, q):
    e = C.F.pole(d, c)
    if e >= 0:
        return int(data.in_sig(np.array([q]), np.array([e]))[0])
    return int(C.F.sides(d, c, [q])[0])


tot = Counter()
rows = [json.loads(x) for x in open(rf) if x.strip()][::SK][:NN]
for r in rows:
    W = [(int(c), int(i)) for c, i in r['W']]
    g, m, _ = C.ghost(W, T, ())
    L = g.q[:m]
    caps = np.flatnonzero(data.sig_matrix(L).sum(axis=0) == len(L)).tolist()
    R = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in sorted({c for c, _ in W})]) & RARE)
    g2, m2, _ = C.ghost(W, Tr, R)
    al, dm, qq = g2.alive[m2:], g2.dom[m2:], g2.q[m2:]
    S = [(int(dm[i]), int(qq[i])) for i in range(len(R)) if al[i] and dm[i] != 0]
    Lr = set(g2.q[:m2].tolist())
    gi, _, _ = C.ghost(CP.inverse_word(M, W), (), R)
    Xp = [int(x) for i, x in enumerate(R.tolist()) if (not gi.alive[i] or gi.dom[i] != 0) and x not in Lr]
    Sig = [s for s in S if s not in Uc]
    pat = len(S) == 6 and len(Sig) == 3 and len(Xp) == 3
    tot['words'] += 1
    if not pat:
        tot['other_anatomy'] += 1
        continue
    tot['U_plus_copy'] += 1
    u_out = all(side(d, f, q) == 0 for f in caps for d, q in Uc)
    x_out = all(not GL.cap(f)[x] for f in caps for x in Xp)
    s_in = all(side(d, f, q) == 1 for f in caps for d, q in Sig)
    tot['U_outside_all'] += u_out
    tot['Xp_outside_all'] += x_out
    tot['copy_inside_all'] += s_in
    tot['all_three'] += u_out and x_out and s_in
print(json.dumps(dict(tot)))
