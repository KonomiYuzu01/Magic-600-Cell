"""Ghost premises of block words W = P Q P^-1 at a state (menu I_a), read from the pose table and the words only.

For each word of a transport.py file (or a single {P, Q} file): the exact frame ghost of W on the target block T, on
every other off piece (protected) and on every chamber of the union of W's caps. Reports T on the lattice at the end,
the protected pieces that die (would straddle a cut) or end at another content, Bad(W) and how many of its chambers
hold lattice pieces at the state, and the grips of W whose caps hold protected pieces at the state.

usage: tghost.py <theory dir> <E-chain scripts dir> <pose table> <target json {pieces}> <words json> <out json>
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402

t0 = time.time()
table, tfile, wfile, out = sys.argv[3:7]
C = GL.Ctx(table)
M = C.M
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
prot = np.setdiff1d(C.off, T)
d0, q0 = C.contents(prot)
wd = json.loads(Path(wfile).read_text())
words = wd['words'] if 'words' in wd else [wd]
rows = []
for w in words:
    P = [(int(c), int(i)) for c, i in w['P']]
    Q = [(int(c), int(i)) for c, i in w['Q']]
    W = P + Q + CP.inverse_word(M, P)
    g, n, U = C.ghost(W, np.concatenate([T, prot]))
    nT = len(T)
    lat = slice(n, None)
    badW = np.sort(U[~g.alive[lat] | (g.dom[lat] != 0)])
    pa = g.alive[nT:n]
    moved = pa & ((g.dom[nT:n] != d0) | (g.q[nT:n] != q0))
    row = {'m': w.get('m'), 'T_size': nT, 'T_on': int((g.alive[:nT] & (g.dom[:nT] == 0)).sum()),
           'T_dead': int((~g.alive[:nT]).sum()), 'prot': int(len(prot)), 'prot_dead': int((~pa).sum()),
           'prot_moved': int(moved.sum()), 'prot_dead_pieces': prot[~pa][:40].tolist(),
           'badW': int(len(badW)), 'badW_occupied': int(C.occ[badW].sum()), 'holes': int((~C.occ).sum()),
           'T_landing_in_badW': int(np.isin(g.q[:nT], badW).sum()),
           'grips': sorted({c for c, _ in W})}
    rows.append(row)
    print(json.dumps({k: v for k, v in row.items() if k not in ('prot_dead_pieces', 'grips')}), flush=True)
Path(out).write_text(json.dumps({'table': table, 'target': T.tolist(), 'rows': rows,
                                 'seconds': round(time.time() - t0, 1)}, indent=1) + '\n')
