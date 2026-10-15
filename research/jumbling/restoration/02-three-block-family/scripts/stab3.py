"""K+ acts on the grips, and the K part of every grip's menu is the grip's full stabilizer in K+.

Checks, exactly: (1) the poles k n_0 (k in K+) are all 600 grips, so |Stab_K+(c)| = 7200 / 600 = 12 for every grip c;
(2) every grip's menu has exactly 12 elements in K+ (the identity included), all fixing the pole. Then the K part of
menu(c) is Stab_K+(c), and k Stab_K+(c) k^-1 = Stab_K+(k c) gives K+-equivariance of the SAFE-ALL letters' elements.

Usage: python -I stab3.py <theory dir> <v26 dir> <pose table> <out json>
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import layers3 as L3  # noqa: E402
from exact import matvec  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402


def main():
    a = sys.argv[1:]
    table, out = a[2], Path(a[3])
    C = L3.GL.Ctx(table)
    data, kp = L3.data, L3.kp
    orbit = sorted({data.POLE.get(tuple(matvec(kp.matrix(k), data.N[0])), -1) for k in range(7200)})
    counts, fixes = [], True
    for c in range(600):
        ks = [i for i in range(len(C.M.items)) if L3.kidx(C.E(c, i)) >= 0]
        counts.append(len(ks))
        for i in ks:
            if tuple(matvec(C.E(c, i), data.N[c])) != tuple(data.N[c]):
                fixes = False
    res = {'grip_orbit_of_0': len(orbit), 'orbit_is_all_grips': orbit == list(range(600)),
           'menu_kplus_counts': sorted(set(counts)), 'all_12': all(x == 12 for x in counts),
           'kplus_elements_fix_pole': fixes, 'menu_size': len(C.M.items)}
    out.write_text(json.dumps(res, indent=1) + '\n')
    L3.log(**res)


if __name__ == '__main__':
    main()
