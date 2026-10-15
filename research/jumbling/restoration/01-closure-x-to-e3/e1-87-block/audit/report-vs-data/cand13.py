"""Recheck screen-87.jsonl candidate 13 (landing vs Bad(W)) and K+ membership of c2's Y letter, with ghostlib."""
import json, sys
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(S / 'v26'))
import ghostlib as GL  # noqa
from sim.kplus import to_tuple  # noqa
from exact import transpose  # noqa
kp = GL.BL.CL.ctx.kplus
C = GL.Ctx(str(S / 'v26/E2/c0/E1-table.json'))
print('digest', C.st.digest())
inK = lambda g: kp.index_of_matrix(to_tuple(g)) >= 0
for c, i in [(190, 8), (190, 4), (190, 3), (190, 9)]:
    print('letter', (c, i), 'in K+', inK(C.E(c, i)))
print('(190,4) is inverse of (190,8):', to_tuple(C.E(190, 4)) == to_tuple(transpose(C.E(190, 8))))
rows = [json.loads(l) for l in open(S / 'v26/E2/screen-87.jsonl')]
n_items = len(C.M.items)
def inv(c, i):
    t = to_tuple(transpose(C.E(c, i)))
    return next(j for j in range(n_items) if to_tuple(C.E(c, j)) == t)
T = C.block(87)
prot = np.setdiff1d(C.off, T)
for n in (0, 2, 13):
    r = rows[n]
    P = [tuple(x) for x in r['P']]; Q = [tuple(x) for x in r['Q']]
    Pinv = [(c, inv(c, i)) for c, i in reversed(P)]
    g, nP, U = C.ghost(P + Q + Pinv, np.concatenate([T, prot]))
    nT = len(T)
    lat = slice(nP, None)
    badW = np.sort(U[~g.alive[lat] | (g.dom[lat] != 0)])
    land = g.q[:nT]
    T_on = int((g.alive[:nT] & (g.dom[:nT] == 0)).sum())
    holes = set(np.flatnonzero(~C.occ).tolist())
    print('cand', n, 'T_on', T_on, '|Bad(W)|', len(badW), '|landing|', len(set(land.tolist())),
          '|landing ∩ Bad(W)|', int(np.isin(land, badW).sum()), 'landing holes at E1', sum(1 for x in land.tolist() if x in holes),
          'Bad(W) holes at E1', sum(1 for x in badW.tolist() if x in holes),
          'recorded landing_in_badW', r['landing_in_badW'])
