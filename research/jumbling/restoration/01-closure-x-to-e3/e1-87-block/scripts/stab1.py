"""After a type-S P: commutators [x, y] and conjugates of SAFE-ALL letters that move a chamber of L (B''s chambers after
P) and fix every bad hole (holes after P inside Bad(P^-1)); then the orbits of H1 = <H0 letters, these words> and
whether each L chamber joins the component of a good hole. H0: letters whose caps avoid every bad hole.
Usage: python -I stab1.py <theory dir> <pose table> <P json word> <out json>"""
import json, sys, time
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import buffer as B
import conjprep as CP
import ghostlib as GL
T0 = time.time()
C = GL.Ctx(sys.argv[2])
R, M, st = C.R, C.M, C.st
off0 = C.off.copy()
P = [tuple(x) for x in json.loads(sys.argv[3])]
badPinv, _, _ = C.bad(CP.inverse_word(M, P))
for c, i in P:
    R.apply(c, i)
G = B.Alphabet(R, M); G.orbits()
holes = np.asarray(G.holes)
inbad = np.zeros(G.NP, bool); inbad[badPinv] = True
BH = holes[inbad[holes]]; GH = holes[~inbad[holes]]
from blocks import CL
reg, kp = CL.ctx.regions, CL.ctx.kplus
Kp = st._kpose()[st.pose_id]
L = reg.piece_of[reg.orbit_of[off0], kp.compose_vec(Kp[off0], reg.transport[off0])]
isBH = np.zeros(G.NP, bool); isBH[BH] = True
touch = [c for c in G.safe if isBH[G.capx[c]].any()]
xl = [n for n, (c, _, _, _) in enumerate(G.letters) if c in touch and G.home[L, c].any()]
ident = np.arange(G.NP)
good = []
tested = 0
for x in xl:
    cx = G.letters[x][0]
    capx = G.home[:, cx]
    for y in range(len(G.letters)):
        cy = G.letters[y][0]
        if cy == cx or not (capx & G.home[:, cy]).any():
            continue
        tested += 1
        w = [x, y, int(G.inv[x]), int(G.inv[y])]
        p = G.perm(w)
        mv = np.flatnonzero(p != ident)
        if len(mv) and not isBH[mv].any() and np.isin(L, mv).any():
            good.append({'word': G.pairs(w), 'moved': int(len(mv)), 'L_moved': [int(z) for z in L if p[z] != z],
                         'L_images': [int(p[z]) for z in L]})
res = {'after_P_digest': st.digest(), 'touch_grips': touch, 'x_letters': len(xl), 'pairs_tested': tested,
       'commutators_fixing_BH_moving_L': len(good), 'examples': good[:20], 'L': L.tolist(), 'GH': GH.tolist(),
       'sec': round(time.time() - T0, 1)}
Path(sys.argv[4]).write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps({k: v for k, v in res.items() if k != 'examples'}))
