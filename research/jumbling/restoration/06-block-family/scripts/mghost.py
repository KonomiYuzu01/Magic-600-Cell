"""Ghost premises of transported block words for several target blocks of one state (tghost.py, many targets).

Loads the state once without the block computation (ghostlib.Ctx minus BL.Blocks, which is slow on very large off
sets), then for every (target json, transport json) pair computes, for each transported word, the exact frame ghost of
the target block T, of every other off piece (protected) and of the chambers of the word's caps, exactly as
tghost.py: T on the lattice at the end, protected pieces killed or moved, Bad(W) and its occupancy. It also lists
the grips of W that some off piece straddles in the state (those make W inadmissible at its first use).

usage: mghost.py <theory dir> <E-chain scripts dir> <pose table> <out json> <target json> <transport json> ...
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
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
import rim  # noqa: E402


class Lite(GL.Ctx):
    def __init__(self, table):  # noqa: D401 - same fields as GL.Ctx except bl
        self.R, self.M = GL.O.start(table)
        self.st = self.R.st
        self.D = rim.Domains(self.st)
        self.F = FG.Frames()
        self.dmap = {j: self.F.domain_of(rep) for j, rep in enumerate(self.D.reps)}
        lat = np.flatnonzero(self.D.dom == 0)
        self.occ = np.zeros(GL.NP, bool)
        self.occ[self.D.cham[lat]] = True
        self.off = np.flatnonzero(self.st._kpose()[self.st.pose_id] < 0)
        self._el = {}


t0 = time.time()
table, out = sys.argv[3], sys.argv[4]
pairs = list(zip(sys.argv[5::2], sys.argv[6::2]))
C = Lite(table)
M = C.M
print(json.dumps({'loaded': True, 'off': int(len(C.off)), 'sec': round(time.time() - t0)}), flush=True)
res = []
for tfile, wfile in pairs:
    T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
    prot = np.setdiff1d(C.off, T)
    d0, q0 = C.contents(prot)
    wd = json.loads(Path(wfile).read_text())
    for w in (wd['words'] if 'words' in wd else [wd]):
        P = [(int(c), int(i)) for c, i in w['P']]
        Q = [(int(c), int(i)) for c, i in w['Q']]
        W = P + Q + CP.inverse_word(M, P)
        grips = sorted({c for c, _ in W})
        blocked = [c for c in grips if C.R.inside(c) is None]
        g, n, U = C.ghost(W, np.concatenate([T, prot]))
        nT = len(T)
        lat = slice(n, None)
        badW = U[~g.alive[lat] | (g.dom[lat] != 0)]
        pa = g.alive[nT:n]
        moved = pa & ((g.dom[nT:n] != d0) | (g.q[nT:n] != q0))
        row = {'target': tfile.split('/')[-1], 'm': w.get('m'), 'T_size': nT,
               'T_on': int((g.alive[:nT] & (g.dom[:nT] == 0)).sum()), 'T_dead': int((~g.alive[:nT]).sum()),
               'prot': int(len(prot)), 'prot_dead': int((~pa).sum()), 'prot_moved': int(moved.sum()),
               'badW': int(len(badW)), 'badW_occupied': int(C.occ[badW].sum()), 'grips': grips,
               'grips_blocked_in_state': blocked}
        res.append(row)
        print(json.dumps(row), flush=True)
Path(out).write_text(json.dumps({'table': table, 'rows': res, 'seconds': round(time.time() - t0, 1)}, indent=1) + '\n')
