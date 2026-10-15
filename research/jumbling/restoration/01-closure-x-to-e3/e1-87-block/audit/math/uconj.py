"""Math audit: (U2) on the instance and the counting lemma for the whole word U = V2 W2 V2^-1 at E1.

Computes Bad(U) and Bad(U^-1) by the chamber ghost over all chambers, V2's chamber permutation sigma, and checks
Bad(U) = sigma^-1(Bad(W2)), Bad(U) inside the holes of E1, |Bad(U)| = |Bad(U^-1)|, and T's final landing
chambers (read from E2-table.json) inside Bad(U^-1).
Usage: python3 -I uconj.py <theory dir> <v26 dir> <out json>
"""
import json
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
import ghostlib as GL  # noqa: E402

T0 = time.time()
C0 = V26 / 'E2' / 'c0'


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def word(name):
    return [(int(c), int(i)) for c, i in json.loads((C0 / 'parts' / f'{name}.json').read_text())['word']]


ctx = GL.Ctx(str(C0 / 'E1-table.json'))
kp = GL.BL.CL.ctx.kplus
reg = GL.BL.CL.ctx.regions
data = GL.BL.CL.ctx.data
NP = len(ctx.st.pose_id)
ORB, TRN, POF = np.asarray(reg.orbit_of), np.asarray(reg.transport), reg.piece_of
V, P, Q, Pinv, Vinv = word('V'), word('P'), word('Q'), word('Pinv'), word('Vinv')
W = P + Q + Pinv
U = V + W + Vinv
Uinv = V + [x for x in reversed([])]  # placeholder, replaced below
# U^-1 = V2 W2^-1 V2^-1; W2^-1 is the mirror inverse of W2 (Pinv's inverse is P, etc.)
from exact import matmul, identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402
ID = to_tuple(identity(4))


def inv_letter(c, i):
    m = ctx.E(c, i)
    for j in range(len(ctx.M.items)):
        if to_tuple(matmul(m, ctx.E(c, j))) == ID:
            return (c, j)
    raise ValueError


Winv = [inv_letter(c, i) for c, i in reversed(W)]
Uinv = V + Winv + Vinv
allc = np.arange(NP)
badU, gU, chU = ctx.bad(U, allc)
log(badU=len(badU))
badUi, gUi, _ = ctx.bad(Uinv, allc)
log(badUinv=len(badUi))
badW, _, _ = ctx.bad(W)
# sigma = chamber permutation of V2 (K+ letters): track every chamber
pts = allc.copy()
for c, i in V:
    k = kp.index_of_matrix(ctx.E(c, i))
    m = GL.cap(c)[pts]
    pts[m] = POF[ORB[pts[m]], kp.compose_vec(np.full(int(m.sum()), k, np.int64), TRN[pts[m]])]
sigma = pts
pre_img = np.flatnonzero(np.isin(sigma, badW))
holes = np.flatnonzero(~ctx.occ)
# T's final landing at E2
c2 = GL.Ctx(str(C0 / 'E2-table.json'))
T = np.asarray(sorted(ctx.block(87).tolist()), np.int64)
K2 = c2.st._kpose()[c2.st.pose_id]
LT = POF[ORB[T], kp.compose_vec(K2[T], TRN[T])]
out = {'badU': int(len(badU)), 'badUinv': int(len(badUi)),
       'badU_equals_sigma_preimage_of_badW': sorted(badU.tolist()) == sorted(pre_img.tolist()),
       'badU_subset_holes_E1': bool(np.isin(badU, holes).all()),
       'T_on_K_at_E2': bool((K2[T] >= 0).all()),
       'landing_U_subset_badUinv': bool(np.isin(LT, badUi).all()),
       'landing_U_equals_badUinv': sorted(LT.tolist()) == sorted(badUi.tolist()),
       'holes_E1_minus_badU': sorted(set(holes.tolist()) - set(badU.tolist())),
       'seconds': round(time.time() - T0, 1)}
OUT.write_text(json.dumps(out, indent=1) + '\n')
log(**{k: v for k, v in out.items() if not isinstance(v, list)})
