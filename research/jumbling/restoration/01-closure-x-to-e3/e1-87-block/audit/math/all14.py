"""Math audit: the 14 screened candidates at E1 (E2/cands-87.json).

For each W = P Q P^-1: Bad(W), Bad(W^-1) over all chambers; T's landing set L by the frame ghost; L = Bad(W^-1)?
L = Bad(W)?; per K+ type: |Bad(W) & type| against |T & type| and the E1 holes of that type (tightness by type).
Usage: python3 -I all14.py <theory dir> <v26 dir> <out json>
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
ctx = GL.Ctx(str(V26 / 'E2' / 'c0' / 'E1-table.json'))
orb = np.asarray(GL.BL.CL.ctx.regions.orbit_of)
allc = np.arange(len(ctx.st.pose_id))
holes = np.flatnonzero(~ctx.occ)
T = ctx.block(87)
tT = Counter(orb[T].tolist())
tH = Counter(orb[holes].tolist())
_inv = {}


def inv_letter(c, i):
    if (c, i) not in _inv:
        m = ctx.E(c, i)
        _inv[(c, i)] = (c, next(j for j in range(len(ctx.M.items)) if to_tuple(matmul(m, ctx.E(c, j))) == ID))
    return _inv[(c, i)]


def mirror(w):
    return [inv_letter(c, i) for c, i in reversed(w)]


rows = []
for n, cand in enumerate(json.loads((V26 / 'E2' / 'cands-87.json').read_text())):
    P = [(int(c), int(i)) for c, i in cand['P']]
    Q = [(int(c), int(i)) for c, i in cand['Q']]
    W = P + Q + mirror(P)
    bW, _, _ = ctx.bad(W, allc)
    bWi, _, _ = ctx.bad(mirror(W), allc)
    g, k, _ = ctx.ghost(W, T, np.zeros(0, np.int64))
    on = g.alive[:k] & (g.dom[:k] == 0)
    L = g.q[:k][on]
    tB = Counter(orb[bW].tolist())
    rows.append({'n': n, 'source': cand.get('source'), 'badW': int(len(bW)), 'badWinv': int(len(bWi)), 'T_on': int(on.sum()),
                 'L_eq_badWinv': sorted(L.tolist()) == sorted(bWi.tolist()),
                 'L_eq_badW': sorted(L.tolist()) == sorted(bW.tolist()),
                 'L_meets_badW': int(np.isin(L, bW).sum()),
                 'badW_types_eq_T_types': tB == tT,
                 'tight_on_T_types': all(tB.get(t, 0) == tH.get(t, 0) for t in tT)})
    print(json.dumps(dict(rows[-1], sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)
out = {'rows': rows, 'all_L_eq_badWinv': all(r['L_eq_badWinv'] for r in rows),
       'n_L_eq_badW': sum(r['L_eq_badW'] for r in rows), 'all_tight_on_T_types': all(r['tight_on_T_types'] for r in rows),
       'seconds': round(time.time() - T0, 1)}
OUT.write_text(json.dumps(out, indent=1) + '\n')
print(json.dumps({k: v for k, v in out.items() if k != 'rows'}), file=sys.stderr)
