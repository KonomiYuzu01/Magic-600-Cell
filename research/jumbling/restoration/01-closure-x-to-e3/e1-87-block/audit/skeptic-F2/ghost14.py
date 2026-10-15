"""Skeptic check of F2 with ghostlib: for each of the 14 screened candidates at E1, T on K, Bad(W), landing L,
|L & Bad(W)|, and Bad(W^-1) with L <= Bad(W^-1) (lower bound |Bad(W)| = |Bad(W^-1)| >= |L|)."""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(S / 'v26'))
import ghostlib as GL  # noqa: E402
from exact import transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

OUT = S / 'v26/audit/skeptic-F2'
C = GL.Ctx(str(S / 'v26/E2/c0/E1-table.json'))
print('digest', C.st.digest(), flush=True)
n_items = len(C.M.items)
_inv = {}


def inv(c, i):
    if (c, i) not in _inv:
        t = to_tuple(transpose(C.E(c, i)))
        _inv[(c, i)] = next(j for j in range(n_items) if to_tuple(C.E(c, j)) == t)
    return _inv[(c, i)]


def invw(w):
    return [(c, inv(c, i)) for c, i in reversed(w)]


T = C.block(87)
prot = np.setdiff1d(C.off, T)
d0, q0 = C.contents(prot)
holes = set(np.flatnonzero(~C.occ).tolist())
cands = json.loads((S / 'v26/E2/cands-87.json').read_text())
rows = [json.loads(l) for l in open(S / 'v26/E2/screen-87.jsonl')]
res = []
for n, cand in enumerate(cands):
    P = [tuple(x) for x in cand['P']]
    Q = [tuple(x) for x in cand['Q']]
    W = P + Q + invw(P)
    g, nP, U = C.ghost(W, np.concatenate([T, prot]))
    nT = len(T)
    lat = slice(nP, None)
    badW = np.sort(U[~g.alive[lat] | (g.dom[lat] != 0)])
    T_on = int((g.alive[:nT] & (g.dom[:nT] == 0)).sum())
    prot_exact = bool(g.alive[nT:nP].all() and np.array_equal(g.dom[nT:nP], d0) and np.array_equal(g.q[nT:nP], q0))
    L = g.q[:nT]
    Winv = invw(W)
    badWi, _, Ui = C.bad(Winv)
    Ls, Bs, Bis = set(L.tolist()), set(badW.tolist()), set(badWi.tolist())
    r = {'n': n, 'source': cand['source'], 'T_on': T_on, 'prot_exact': prot_exact, 'badW': len(Bs), 'L': len(Ls),
         'L_and_badW': len(Ls & Bs), 'badWinv': len(Bis), 'L_subset_badWinv': Ls <= Bis, 'L_eq_badWinv': Ls == Bis,
         'badW_eq_badWinv': Bs == Bis, 'U_same': bool(np.array_equal(np.sort(U), np.sort(Ui))),
         'L_holes_E1': len(Ls & holes), 'badW_holes_E1': len(Bs & holes),
         'matches_screen': (sorted(Ls) == sorted(rows[n]['landing_W']) and sorted(Bs) == sorted(rows[n]['badW_chambers'])),
         'screen_landing_in_badW': rows[n]['landing_in_badW']}
    res.append(r)
    print(json.dumps(r), flush=True)
(OUT / 'ghost14.json').write_text(json.dumps(res, indent=1) + '\n')
