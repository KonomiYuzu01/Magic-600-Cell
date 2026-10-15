"""Lock-lemma premises for the off block at a pose table: I_B (caps holding every member), S_B (cuts some member
straddles), the chambers x with I_B within caps(x) within I_B + S_B, and blocks.Blocks' own locked list. Read-only.

Usage: python -I lock3.py <theory dir> <v26 dir> <pose table> <out json>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import layers3 as L3  # noqa: E402

GL = L3.GL


def main():
    a = sys.argv[1:]
    table, out = a[2], Path(a[3])
    C = GL.Ctx(table)
    H = L3.HOME()
    Bp = np.asarray(C.off, np.int64)
    dom0, q0 = C.contents(Bp)
    I, S = [], []
    for c in range(600):
        sd = L3.sides(C, dom0, q0, c)
        if (sd < 0).any():
            S.append(c)
        elif (sd == 1).all():
            I.append(c)
    Im = np.zeros(600, bool)
    Im[I] = True
    Sm = np.zeros(600, bool)
    Sm[S] = True
    hasI = H[:, Im].all(axis=1)
    within = ~(H[:, ~(Im | Sm)].any(axis=1))
    locked = np.flatnonzero(hasI & within)
    typ = np.asarray(L3.reg.orbit_of)
    res = {'input_digest': C.st.digest(), 'block': Bp.tolist(), 'I_B': I, 'S_B': S,
           'locked': locked.tolist(), 'locked_types': typ[locked].tolist(),
           'locked_occupied': [bool(C.occ[x]) for x in locked],
           'blocks_module_locked': [sorted(map(int, x)) for x in C.bl.locked],
           'caps_84071': np.flatnonzero(H[84071]).tolist()}
    out.write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: v for k, v in res.items() if k != 'input_digest'}))


if __name__ == '__main__':
    main()
