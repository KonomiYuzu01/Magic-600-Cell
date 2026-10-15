"""Validate the frame ghost against the simulator: a word at a pose table, ghost of T/Prot/union-of-caps chambers
versus the real replay (collateral, T on lattice, Prot exact)."""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import blocks as BL  # noqa: E402
import conjclose as CC  # noqa: E402
import fastghost as FG  # noqa: E402
import rim  # noqa: E402

O, restore, data = BL.O, BL.restore, BL.CL.ctx.data
NP = len(data.sigbits)
R, M = O.start(sys.argv[2])
word = [tuple(x) for x in json.loads(sys.argv[3])]
size = int(sys.argv[4])
st = R.st
bl = BL.Blocks(R, M)
b = next(j for j, p in enumerate(bl.blocks) if len(p) == size)
T = np.asarray(bl.blocks[b], np.int64)
off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
prot = np.setdiff1d(off, T)
D = rim.Domains(st)
F = FG.Frames()
dmap = {j: F.domain_of(rep) for j, rep in enumerate(D.reps)}


def contents(pieces):
    dom, q = [], []
    for p in pieces.tolist():
        d2, k = dmap[int(D.dom[p])]
        dom.append(d2)
        q.append(int(FG.chamber_image(k, np.array([D.cham[p]]))[0]))
    return np.array(dom, np.int64), np.array(q, np.int64)


T_dom, T_q = contents(T)
P_dom, P_q = contents(prot)
lat = np.flatnonzero(D.dom == 0)
occ = np.zeros(NP, bool)
occ[D.cham[lat]] = True
grips = sorted({c for c, _ in word})
U = np.flatnonzero(np.logical_or.reduce([data.in_sig(np.arange(NP), np.full(NP, c, np.int64)) for c in grips]))
g = FG.Ghost(F, np.concatenate([T_dom, P_dom, np.zeros(len(U), np.int64)]), np.concatenate([T_q, P_q, U]))
for c, i in word:
    g.apply(c, restore.element(M, c, i), (c, i))
nT, nP = len(T), len(prot)
lA = g.alive[nT + nP:]
bad = U[~lA | (g.dom[nT + nP:] != 0)]
ghost = {'T_on': int((g.alive[:nT] & (g.dom[:nT] == 0)).sum()), 'T_dead': int((~g.alive[:nT]).sum()),
         'prot_moved': int((g.dom[nT:nT + nP] != P_dom).sum() + (g.q[nT:nT + nP] != P_q).sum()),
         'prot_dead': int((~g.alive[nT:nT + nP]).sum()),
         'bad': int(len(bad)), 'bad_unclean': int((~lA).sum()), 'bad_occupied': int(occ[bad].sum())}
codes = CC.Codes()
c_x = codes.of(st)
real = {}
try:
    for n, (c, i) in enumerate(word):
        R.apply(c, i)
    c_y = codes.of(st)
    inT = np.zeros(len(c_x), bool)
    inT[T] = True
    real = {'T_on': int((inT & (c_y >= 0)).sum()), 'collateral': int(((c_x >= 0) & (c_y < 0)).sum()),
            'prot_changed': int(((c_x < 0) & ~inT & (c_y != c_x)).sum())}
except RuntimeError as exc:
    real = {'inadmissible_at': n, 'why': str(exc)[:200]}
print(json.dumps({'ghost': ghost, 'real': real}))
