"""Audit (math): at Y_k = E2 . P(k), every SAFE-ALL grip whose cap holds at least one chamber of L_k: how many of
L_k it holds and how many bad holes (holes in T_k = Bad(P(k)^-1)) it holds.

Usage: python3 -I anycaps.py <theory dir> <v26 dir> <E2 table> <out json> <levels...>
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
import blocks as BL  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402

CL = BL.CL
reg, kp = CL.ctx.regions, CL.ctx.kplus
BPRIME = np.array([16245, 16249, 32088], np.int64)
XS = [(91, 6), (154, 7), (133, 1), (183, 6)]


def main():
    a = sys.argv[1:]
    table, outp, todo = a[2], Path(a[3]), [int(x) for x in a[4:]]
    C = GL.Ctx(table)
    R, M, st = C.R, C.M, C.st
    NP = len(st.pose_id)
    d0 = st.digest()
    Pl = [[(237, 13)]]
    for k in range(4):
        Pl.append(Pl[k] + [XS[k]] + CP.inverse_word(M, Pl[k]))
    out = {}
    for k in todo:
        P = Pl[k]
        badm, alive = CP.bad_chambers(M, CP.inverse_word(M, P), NP)
        for c, i in P:
            R.apply(c, i)
        K = st._kpose()[st.pose_id]
        on = np.flatnonzero(K >= 0)
        ch = reg.piece_of[reg.orbit_of[on], kp.compose_vec(K[on], reg.transport[on])]
        occ = np.zeros(NP, bool)
        occ[ch] = True
        L = reg.piece_of[reg.orbit_of[BPRIME], kp.compose_vec(K[BPRIME], reg.transport[BPRIME])]
        BH = badm & ~occ
        rows = []
        for c in range(600):
            cap = GL.cap(c)
            if not cap[L].any():
                continue
            cl = st.classify(c, need_inside=True, stop_at_straddle=False)
            inside = np.asarray(cl.inside, np.int64)
            safe = cl.straddle is None and not (len(inside) and (K[inside] < 0).any())
            rows.append({'grip': c, 'safe': bool(safe), 'L_in_cap': int(cap[L].sum()), 'bh_in_cap': int(BH[cap].sum())})
        safe_rows = [r for r in rows if r['safe']]
        out[str(k)] = {'rows': rows,
                       'min_bh_any': min(r['bh_in_cap'] for r in safe_rows),
                       'min_bh_all': min((r['bh_in_cap'] for r in safe_rows if r['L_in_cap'] == 3), default=None)}
        for _ in P:
            R.undo()
        assert st.digest() == d0
        print(k, json.dumps(out[str(k)]), flush=True)
    outp.write_text(json.dumps(out) + '\n')


if __name__ == '__main__':
    main()
