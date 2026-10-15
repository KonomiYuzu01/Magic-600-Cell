"""Glued holes (menu I_a): for every hole h of the given types at a state and every off block U, check
(1) every grip whose cap holds h has U inside its cap or straddling its cut;
(2) every grip whose cap holds all of U, unstraddled, holds h;
(3) no grip has pieces of U on both sides of its cut without a straddling piece.
If (1) to (3) hold, h is glued to U (PROOFS-U Proposition GH): every word of admissible K+ letters that returns U to
its exact poses returns the hole to h. Cut statuses are exact (sim.kernel Evaluator); cap membership of chambers is
the library's exact signature test.

usage: glued.py <theory> <echain> <table> <out json> --types 7,9
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa: E402
from exact import matmul  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

table, out = sys.argv[3:5]
a = sys.argv[5:]
TYPES = [int(x) for x in a[a.index('--types') + 1].split(',')]
CTX = GL.BL.CL.ctx
data, reg = CTX.data, CTX.regions
C = GL.Ctx(table)
st = C.st
TYPE = np.asarray(reg.orbit_of, np.int64)
NG = len(data.N)
caps = np.array([GL.cap(c) for c in range(NG)])  # NG x NP


def status(pieces):
    """grip -> 0 outside, 1 inside, 2 straddle, 3 both sides without a straddling piece."""
    groups = {}
    for p in pieces:
        g = st.pose(int(p))
        groups.setdefault(to_tuple(g), (g, []))[1].append(int(p))
    res = []
    for e in range(NG):
        seen = set()
        for g, pcs in groups.values():
            ev = Evaluator(pullback_form(g, data.N[e]))
            for p in pcs:
                r = ev.classify(reg.vform(int(p)))[0]
                seen.add(r)
                if r == 'straddle':
                    break
            if 'straddle' in seen:
                break
        res.append(2 if 'straddle' in seen else (3 if len(seen) > 1 else (1 if 'in' in seen else 0)))
    return np.array(res)


blocks = [np.asarray(sorted(b), np.int64) for b in C.bl.blocks if len(b)]
stat = {int(b.min()): status(b.tolist()) for b in blocks}
holes = np.flatnonzero(~C.occ & np.isin(TYPE, TYPES))
rows = []
for h in holes.tolist():
    Hh = np.flatnonzero(caps[:, h])
    glued_to = []
    for u, s in stat.items():
        c1 = bool(np.isin(s[Hh], [1, 2]).all())
        movers = np.flatnonzero(s == 1)
        c2 = bool(caps[movers, h].all())
        c3 = bool((s != 3).all())
        if c1 and c2 and c3:
            glued_to.append(u)
    rows.append({'hole': h, 'type': int(TYPE[h]), 'caps_holding': len(Hh), 'glued_to': glued_to,
                 'by_block': {str(u): {'caps_holding_h_with_status': {str(k): int((s[Hh] == k).sum()) for k in range(4)},
                                       'movers': np.flatnonzero(s == 1).tolist(),
                                       'movers_holding_h': int(caps[np.flatnonzero(s == 1), h].sum())}
                              for u, s in stat.items()}})
doc = {'off': int(len(C.off)), 'blocks': {str(int(b.min())): int(len(b)) for b in blocks}, 'types': TYPES,
       'holes': len(rows), 'all_glued': all(len(r['glued_to']) == 1 for r in rows),
       'status3': {str(u): int((s == 3).sum()) for u, s in stat.items()}, 'rows': rows}
Path(out).write_text(json.dumps(doc) + '\n')
print(json.dumps({k: v for k, v in doc.items() if k != 'rows'}))
for r in rows:
    print(json.dumps({k: r[k] for k in ('hole', 'type', 'caps_holding', 'glued_to')}))
