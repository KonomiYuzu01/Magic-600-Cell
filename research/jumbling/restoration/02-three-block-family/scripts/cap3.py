"""Two model premises used by the K+ conjugation (Corollary C) and the disjointness argument (Theorem Q), exactly:
(1) every element of every grip's menu fixes the grip's pole, so every cap (a half-space at the common cut) is
    invariant under its menu; (2) K+ maps caps to caps: for grip 0 and every k in K+, the chamber image k(cap(0)) is
    cap(k 0). With K+ transitive on the grips (stab3.py), (2) for one grip gives it for all: cap(k j 0) = k j cap(0).

Usage: python -I cap3.py <theory dir> <v26 dir> <pose table> <out json>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import layers3 as L3  # noqa: E402
from exact import matvec  # noqa: E402


def main():
    a = sys.argv[1:]
    table, out = a[2], Path(a[3])
    C = L3.GL.Ctx(table)
    data, kp = L3.data, L3.kp
    bad_fix = []
    for c in range(600):
        n = tuple(data.N[c])
        for i in range(len(C.M.items)):
            if tuple(matvec(C.E(c, i), data.N[c])) != n:
                bad_fix.append([c, i])
    cap0 = np.flatnonzero(L3.GL.cap(0))
    bad_cap = []
    for k in range(7200):
        c2 = data.POLE.get(tuple(matvec(kp.matrix(k), data.N[0])), -1)
        img = np.sort(L3.FG.chamber_image(k, cap0))
        if c2 < 0 or not np.array_equal(img, np.flatnonzero(L3.GL.cap(c2))):
            bad_cap.append(k)
    res = {'menu_elements_checked': 600 * len(C.M.items), 'menu_elements_not_fixing_pole': len(bad_fix),
           'cap_size': int(len(cap0)), 'kplus_checked': 7200, 'cap_images_not_caps': len(bad_cap),
           'first_failures': {'fix': bad_fix[:5], 'cap': bad_cap[:5]}}
    out.write_text(json.dumps(res, indent=1) + '\n')
    L3.log(**{x: res[x] for x in res if x != 'first_failures'})


if __name__ == '__main__':
    main()
