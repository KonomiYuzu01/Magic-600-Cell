"""Math audit, E1: Bad(W2), Bad(W2^-1), T's landing set, types of T / B' / holes, word-shape checks.

Tests the counting lemma (landing set of restored pieces lies in Bad(U^-1), |Bad(U^-1)| = |Bad(U)|) on W2,
and the type-level tightness argument. Read-only on inputs; writes one JSON to the given output path.
Usage: python3 -I e1check.py <theory dir> <v26 dir> <out json>
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
OUT = Path(sys.argv[3])
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(V26))
import ghostlib as GL  # noqa: E402
from exact import matmul, identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
C0 = V26 / 'E2' / 'c0'


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def word(name):
    return [(int(c), int(i)) for c, i in json.loads((C0 / 'parts' / f'{name}.json').read_text())['word']]


ctx = GL.Ctx(str(C0 / 'E1-table.json'))
M = ctx.M
kp = GL.BL.CL.ctx.kplus
reg = GL.BL.CL.ctx.regions
ID = to_tuple(identity(4))


def inv_index(c, i):
    m = ctx.E(c, i)
    for j in range(len(M.items)):
        if to_tuple(matmul(m, ctx.E(c, j))) == ID:
            return j
    raise ValueError((c, i))


def mirror_inverse(w):
    return [(c, inv_index(c, i)) for c, i in reversed(w)]


out = {'digest': ctx.st.digest()}
P, Q, Pinv, V, Vinv = word('P'), word('Q'), word('Pinv'), word('V'), word('Vinv')
out['Pinv_is_mirror_inverse_of_P'] = (Pinv == mirror_inverse(P))
out['Vinv_is_mirror_inverse_of_V'] = (Vinv == mirror_inverse(V))
out['V_letters_all_in_Kplus'] = all(kp.index_of_matrix(ctx.E(c, i)) >= 0 for c, i in set(V))
out['V_grips'] = sorted({c for c, _ in V})
log(shape=True)

W = P + Q + Pinv
Winv = mirror_inverse(W)
out['Winv_equals_P_Qinv_Pinv'] = (Winv == P + mirror_inverse(Q) + Pinv)
off = ctx.off
blocks = sorted(ctx.bl.blocks, key=len)
Bp = np.asarray(blocks[0], np.int64)
T = np.asarray(blocks[-1], np.int64)
out['blocks'] = [len(b) for b in blocks]
out['Bprime'] = Bp.tolist()

# Bad sets (word only) over the union of the word's caps
badW, gW, chW = ctx.bad(W)
badWinv, gWi, chWi = ctx.bad(Winv)
out['badW_size'] = int(len(badW))
out['badWinv_size'] = int(len(badWinv))
pre = json.loads((C0 / 'pre.json').read_text())
out['badW_equals_pre'] = sorted(badW.tolist()) == sorted(pre['ghost']['badW'])
log(badW=len(badW), badWinv=len(badWinv))

# T's landing set and B' under W (frame ghost of the off pieces only)
g, n, _ = ctx.ghost(W, np.concatenate([T, Bp]), np.zeros(0, np.int64))
alive_T = g.alive[:len(T)]
onK_T = alive_T & (g.dom[:len(T)] == 0)
L = np.sort(g.q[:len(T)][onK_T])
out['T_alive_all'] = bool(alive_T.all())
out['T_on_K'] = int(onK_T.sum())
out['Bprime_alive_all'] = bool(g.alive[len(T):].all())
d0, q0 = ctx.contents(Bp)
out['Bprime_same_domain_and_chamber'] = bool(np.array_equal(g.dom[len(T):], d0) and np.array_equal(g.q[len(T):], q0))
out['landing_size'] = int(len(L))
out['landing_distinct'] = int(len(set(L.tolist())))
out['landing_equals_badW'] = sorted(L.tolist()) == sorted(badW.tolist())
out['landing_subset_badWinv'] = bool(np.isin(L, badWinv).all())
out['landing_equals_badWinv'] = sorted(L.tolist()) == sorted(badWinv.tolist())
out['badW_equals_badWinv'] = sorted(badW.tolist()) == sorted(badWinv.tolist())

# types (K+ orbits of chambers / home chambers)
orb = np.asarray(reg.orbit_of)
holes = np.flatnonzero(~ctx.occ)
out['holes'] = int(len(holes))
tT = Counter(orb[T].tolist())
tB = Counter(orb[Bp].tolist())
tH = Counter(orb[holes].tolist())
tBad = Counter(orb[badW].tolist())
out['types_T'] = {str(k): v for k, v in sorted(tT.items())}
out['types_Bprime'] = {str(k): v for k, v in sorted(tB.items())}
out['types_holes_E1'] = {str(k): v for k, v in sorted(tH.items())}
out['types_badW'] = {str(k): v for k, v in sorted(tBad.items())}
out['types_shared_T_Bprime'] = sorted(set(tT) & set(tB))
out['holes_per_type_equals_off_per_type'] = (tH == (tT + tB))
out['badW_per_type_equals_T_per_type'] = (tBad == tT)
out['orbit_sizes_of_types'] = {str(k): int((orb == k).sum()) for k in sorted(set(tT) | set(tB))}
out['badW_occupied_E1'] = int(ctx.occ[badW].sum())
out['seconds'] = round(time.time() - T0, 1)
OUT.write_text(json.dumps(out, indent=1) + '\n')
log(done=True)
