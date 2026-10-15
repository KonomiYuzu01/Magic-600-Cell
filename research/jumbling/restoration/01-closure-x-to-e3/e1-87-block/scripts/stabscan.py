"""Screen type-S P candidates for the simplest Q': is there a SAFE-ALL cap (after P) holding a chamber of L (B''s
chambers) and no bad hole (hole after P inside Bad(P^-1)), and do L and the good holes share components of H0 (letters
whose caps avoid every bad hole)? Each P is undone before the next.
Usage: python -I stabscan.py <theory dir> <pose table> <typeS jsonl> <out jsonl>"""
import json, sys, time
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import buffer as B
import conjprep as CP
import ghostlib as GL
from blocks import CL
reg, kp = CL.ctx.regions, CL.ctx.kplus
T0 = time.time()
C = GL.Ctx(sys.argv[2])
R, M, st = C.R, C.M, C.st
d0 = st.digest()
off0 = C.off.copy()
rows = [json.loads(l) for l in open(sys.argv[3])]
with open(sys.argv[4], 'w') as fh:
    for r in rows:
        if not r.get('N2'):
            continue
        P = [tuple(x) for x in r['P']]
        badPinv, _, _ = C.bad(CP.inverse_word(M, P))
        for c, i in P:
            R.apply(c, i)
        G = B.Alphabet(R, M); G.orbits()
        holes = np.asarray(G.holes)
        inbad = np.zeros(G.NP, bool); inbad[badPinv] = True
        BH = holes[inbad[holes]]; GH = holes[~inbad[holes]]
        Kp = st._kpose()[st.pose_id]
        L = reg.piece_of[reg.orbit_of[off0], kp.compose_vec(Kp[off0], reg.transport[off0])]
        isBH = np.zeros(G.NP, bool); isBH[BH] = True
        h0 = [c for c in G.safe if not isBH[G.capx[c]].any()]
        lab = np.arange(G.NP)
        while True:
            old = lab.copy()
            for c, _, _, ys in G.letters:
                if c in h0:
                    xs = G.capx[c]
                    m = np.minimum(lab[xs], lab[ys])
                    np.minimum.at(lab, xs, m); np.minimum.at(lab, ys, m)
            lab = lab[lab]
            if np.array_equal(lab, old):
                break
        out = {'n': r['n'], 'parked': r['parked'], 'L': L.tolist(), 'GH': GH.tolist(),
               'L_caps_without_BH': [int(sum(1 for c in h0 if G.home[z, c])) for z in L.tolist()],
               'L_h0': [int(lab[z]) for z in L.tolist()], 'GH_h0': [int(lab[z]) for z in GH.tolist()],
               'match': sorted(int(lab[z]) for z in L.tolist()) == sorted(int(lab[z]) for z in GH.tolist()),
               'sec': round(time.time() - T0, 1)}
        fh.write(json.dumps(out) + '\n'); fh.flush()
        print(json.dumps(out), file=sys.stderr, flush=True)
        for _ in P:
            R.undo()
        assert st.digest() == d0
