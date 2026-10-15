"""Math audit: the report's candidate 13 at E1 (park-E1-87-9.jsonl:169, E2/w87-c13.json).

Recomputes, by the chamber and frame ghosts only: Bad(W) and Bad(W^-1) over all chambers for W = P Q P^-1, T's landing
set L, and tests L against Bad(W) (the report: disjoint) and against Bad(W^-1) (the counting lemma: L inside it, with
|Bad(W^-1)| = |Bad(W)|), plus per-type counts.
Usage: python3 -I cand13.py <theory dir> <v26 dir> <out json>
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
ID = to_tuple(identity(4))


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


ctx = GL.Ctx(str(V26 / 'E2' / 'c0' / 'E1-table.json'))
reg = GL.BL.CL.ctx.regions
orb = np.asarray(reg.orbit_of)


def inv_letter(c, i):
    m = ctx.E(c, i)
    js = [j for j in range(len(ctx.M.items)) if to_tuple(matmul(m, ctx.E(c, j))) == ID]
    return (c, js[0])


def mirror(w):
    return [inv_letter(c, i) for c, i in reversed(w)]


cand = json.loads((V26 / 'E2' / 'w87-c13.json').read_text())
row = json.loads((V26 / 'park-E1-87-9.jsonl').read_text().splitlines()[169])
P = [(int(c), int(i)) for c, i in cand['P']]
Q = [(int(c), int(i)) for c, i in cand['Q']]
out = {'digest_E1': ctx.st.digest(), 'source': cand.get('source'), 'row169_P_equals_cand_P': [list(x) for x in P] == row['P'],
       'row169_badW_recorded': row['badW']}
W = P + Q + mirror(P)
Winv = mirror(W)
allc = np.arange(len(ctx.st.pose_id))
bW, _, _ = ctx.bad(W, allc)
bWi, _, _ = ctx.bad(Winv, allc)
log(badW=len(bW), badWinv=len(bWi))
T = ctx.block(87)
Bp = ctx.block(3)
g, n, _ = ctx.ghost(W, np.concatenate([T, Bp]), np.zeros(0, np.int64))
aliveT = g.alive[:len(T)]
onT = aliveT & (g.dom[:len(T)] == 0)
L = np.sort(g.q[:len(T)][onT])
d0, q0 = ctx.contents(Bp)
out.update({'badW': int(len(bW)), 'badWinv': int(len(bWi)),
            'T_on_K': int(onT.sum()), 'landing_distinct': int(len(set(L.tolist()))),
            'Bprime_exact': bool(g.alive[len(T):].all() and np.array_equal(g.dom[len(T):], d0) and np.array_equal(g.q[len(T):], q0)),
            'landing_meets_badW': int(np.isin(L, bW).sum()),
            'landing_subset_badWinv': bool(np.isin(L, bWi).all()),
            'landing_equals_badWinv': sorted(L.tolist()) == sorted(bWi.tolist()),
            'badW_subset_holes_E1': bool((~ctx.occ[bW]).all()),
            'badW_occupied_E1': int(ctx.occ[bW].sum()),
            'types_T': {str(k): v for k, v in sorted(Counter(orb[T].tolist()).items())},
            'types_badW': {str(k): v for k, v in sorted(Counter(orb[bW].tolist()).items())},
            'types_badWinv': {str(k): v for k, v in sorted(Counter(orb[bWi].tolist()).items())},
            'types_landing': {str(k): v for k, v in sorted(Counter(orb[L].tolist()).items())}})
out['seconds'] = round(time.time() - T0, 1)
OUT.write_text(json.dumps(out, indent=1) + '\n')
log(**{k: v for k, v in out.items() if not isinstance(v, (list, dict))})
