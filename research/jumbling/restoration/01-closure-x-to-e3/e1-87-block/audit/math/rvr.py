"""Math audit: Bad of the type-S conjugate R V R^-1 at E2, R = (237, 13), against Bad(R).

Computes over all chambers, by the chamber ghost: Bad(R R^-1) (V empty), and for random SAFE-ALL words V' at
Y = E2 R: Bad(R V' R^-1), compared with the formula sigma_V'^-1(cap 237) minus cap 237. Also holes(Y) minus cap 237
against holes(E2), which by the formula is Bad(R V R^-1) for any SAFE-ALL V at Y that vacates Bad(R^-1).
Usage: python3 -I rvr.py <theory dir> <v26 dir> <out json>
"""
import json
import random
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
V26 = Path(sys.argv[2]).resolve()
OUT = Path(sys.argv[3])
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(V26))
import buffer as B  # noqa: E402
import ghostlib as GL  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


ce = GL.Ctx(str(V26 / 'E2' / 'c0' / 'E2-table.json'))
kp = GL.BL.CL.ctx.kplus
reg = GL.BL.CL.ctx.regions
ORB, TRN, POF = np.asarray(reg.orbit_of), np.asarray(reg.transport), reg.piece_of
NP = len(ce.st.pose_id)
allc = np.arange(NP)
R, Rinv = [(237, 13)], [(237, 47)]
cap237 = np.flatnonzero(GL.cap(237))
holesE2 = np.flatnonzero(~ce.occ)
out = {'digest_E2': ce.st.digest(), 'cap237': int(len(cap237))}
b0, _, _ = ce.bad(R + Rinv, allc)
bR, _, _ = ce.bad(R, allc)
out['bad_R'] = int(len(bR))
out['bad_R_Rinv_empty_V'] = int(len(b0))
ce.R.apply(237, 13)
out['digest_Y'] = ce.st.digest()
G = B.Alphabet(ce.R, ce.M)
holesY = np.asarray(G.holes, np.int64)
out['holes_Y'] = int(len(holesY))
out['holesY_minus_cap237_equals_holesE2'] = sorted(set(holesY.tolist()) - set(cap237.tolist())) == sorted(holesE2.tolist())
log(**out)


def sigma(word):
    pts = allc.copy()
    for c, i in word:
        k = kp.index_of_matrix(ce.E(c, i))
        assert k >= 0
        m = GL.cap(c)[pts]
        pts[m] = POF[ORB[pts[m]], kp.compose_vec(np.full(int(m.sum()), k, np.int64), TRN[pts[m]])]
    return pts


runs = []
for seed, length in ((1, 10), (2, 40), (3, 120)):
    rng = random.Random(seed)
    Vp = [tuple(int(x) for x in G.letters[rng.randrange(len(G.letters))][:2]) for _ in range(length)]
    s = sigma(Vp)
    formula = np.setdiff1d(np.flatnonzero(np.isin(s, cap237)), cap237)
    b, _, _ = ce.bad(R + Vp + Rinv, allc)
    runs.append({'seed': seed, 'len': length, 'bad_RVR': int(len(b)), 'formula': int(len(formula)),
                 'equal_formula': sorted(b.tolist()) == sorted(formula.tolist()),
                 'bad_meets_cap237': int(np.isin(b, cap237).sum())})
    log(run=runs[-1])
out['random_V'] = runs
out['seconds'] = round(time.time() - T0, 1)
OUT.write_text(json.dumps(out, indent=1) + '\n')
log(done=True)
