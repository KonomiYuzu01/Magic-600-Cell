"""Math audit, E2 and X: holes vs B' types; the single letter R = (237, 13) and its type-S conjugate at E2;
the counting lemma on segment 1's word at X.

Usage: python3 -I e2check.py <theory dir> <v26 dir> <segment-1 package dir> <out json>
"""
import json
import sys
import time
from collections import Counter
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
V26 = Path(sys.argv[2]).resolve()
PKG = Path(sys.argv[3]).resolve()
OUT = Path(sys.argv[4])
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(V26))
import buffer as B  # noqa: E402
import ghostlib as GL  # noqa: E402
from exact import matmul, identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
C0 = V26 / 'E2' / 'c0'
ID = to_tuple(identity(4))
reg = GL.BL.CL.ctx.regions
kp = GL.BL.CL.ctx.kplus
orb = np.asarray(reg.orbit_of)


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def mirror_inverse(ctx, w):
    out = []
    for c, i in reversed(w):
        m = ctx.E(c, i)
        js = [j for j in range(len(ctx.M.items)) if to_tuple(matmul(m, ctx.E(c, j))) == ID]
        out.append((c, js[0]))
    return out


out = {}
# ---------------------------------------------------------------- segment 1 at X: counting lemma
cx = GL.Ctx(str(PKG / 'input' / 'X-table.json'))
w17 = json.loads((PKG / 'D17' / 'w17-cand1.json').read_text())
P1 = [(int(c), int(i)) for c, i in w17['P']]
Q1 = [(int(c), int(i)) for c, i in w17['Q']]
W1 = P1 + Q1 + mirror_inverse(cx, P1)
W1inv = mirror_inverse(cx, W1)
bW1, _, _ = cx.bad(W1)
bW1i, _, _ = cx.bad(W1inv)
T17 = cx.block(17)
g, n, _ = cx.ghost(W1, T17, np.zeros(0, np.int64))
on = g.alive[:n] & (g.dom[:n] == 0)
L17 = g.q[:n][on]
out['X'] = {'digest': cx.st.digest(), 'W1_bad': int(len(bW1)), 'W1inv_bad': int(len(bW1i)),
            'T17_on_K': int(on.sum()), 'L17_subset_badW1inv': bool(np.isin(L17, bW1i).all()),
            'L17_subset_badW1': bool(np.isin(L17, bW1).all()),
            'badW1_equals_badW1inv': sorted(bW1.tolist()) == sorted(bW1i.tolist())}
log(X=out['X'])
del cx

# ---------------------------------------------------------------- E2
ce = GL.Ctx(str(C0 / 'E2-table.json'))
st = ce.st
holes = np.flatnonzero(~ce.occ)
Bp = np.sort(ce.off)
out['E2'] = {'digest': st.digest(), 'off': Bp.tolist(), 'holes': holes.tolist(),
             'types_holes': {str(k): v for k, v in sorted(Counter(orb[holes].tolist()).items())},
             'types_Bprime': {str(k): v for k, v in sorted(Counter(orb[Bp].tolist()).items())}}
G0 = B.Alphabet(ce.R, ce.M)
G0.orbits()
out['E2']['hole_orbit_sizes'] = sorted(int(G0.orbit_size[G0.orbit[h]]) for h in holes)
out['E2']['safe_grips'] = len(G0.safe)
R1 = [(237, 13)]
R1inv = mirror_inverse(ce, R1)
bR, _, chR = ce.bad(R1)
bRi, _, _ = ce.bad(R1inv)
cap237 = GL.cap(237)
out['R'] = {'letter': R1, 'inverse': R1inv, 'in_Kplus': kp.index_of_matrix(ce.E(237, 13)) >= 0,
            'bad_R': int(len(bR)), 'bad_Rinv': int(len(bRi)), 'cap237_chambers': int(cap237.sum()),
            'bad_R_equals_cap237': sorted(bR.tolist()) == np.flatnonzero(cap237).tolist(),
            'E2_holes_in_cap237': [int(h) for h in holes if cap237[h]]}
log(R=out['R'])
# apply R at E2 (fresh restorer of the same table)
Rr = ce.R
K0 = st._kpose()[st.pose_id]
try:
    Rr.apply(237, 13)
    out['R']['admissible_at_E2'] = True
except RuntimeError as exc:
    out['R']['admissible_at_E2'] = False
    out['R']['error'] = str(exc)[:300]
if out['R']['admissible_at_E2']:
    K1 = st._kpose()[st.pose_id]
    out['R']['Bprime_on_K_after'] = bool((K1[Bp] >= 0).all())
    out['R']['off_after'] = int((K1 < 0).sum())
    out['R']['lattice_of_E2_taken_off'] = int(((K0 >= 0) & (K1 < 0)).sum())
    G = B.Alphabet(Rr, ce.M)
    G.orbits()
    hy = np.asarray(G.holes)
    out['Y'] = {'digest': st.digest(), 'holes': int(len(hy)), 'safe_grips': len(G.safe), 'letters': len(G.letters),
                'orbits': int(len(G.orbit_size))}
    cond = B.condition(G, bRi)
    out['Y']['target_Bad_Rinv'] = int(len(bRi))
    out['Y']['N1'] = cond['fixed_vacant']
    out['Y']['N2'] = cond['necessary']
    out['Y']['fixed_occupied'] = cond['fixed_occupied'][:20]
    out['Y']['n_fixed_occupied'] = len(cond['fixed_occupied'])
    out['Y']['deficits'] = cond['necessary_deficits'][:20]
    occ_y = np.ones(G.NP, bool)
    occ_y[hy] = False
    occb = np.flatnonzero(occ_y[bRi] if False else occ_y[np.asarray(bRi)])
    out['Y']['occupied_in_target'] = [int(np.asarray(bRi)[j]) for j in occb]
    out['Y']['occupied_in_target_orbit_sizes'] = [int(G.orbit_size[G.orbit[int(np.asarray(bRi)[j])]]) for j in occb]
    log(Y=out['Y'])
out['seconds'] = round(time.time() - T0, 1)
OUT.write_text(json.dumps(out, indent=1) + '\n')
log(done=True)
