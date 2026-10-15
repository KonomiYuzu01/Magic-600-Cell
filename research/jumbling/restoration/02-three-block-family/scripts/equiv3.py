"""Menu equivariance under K+ for the template's letters: for every k in K+ and every letter (c, i) of W and W^-1,
the grip k(c) (pole k n_c) must carry the element k E(c, i) k^-1 in its menu. Read-only, exact.

Usage: python -I equiv3.py <theory dir> <v26 dir> <pose table> <template json> <out json>
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import layers3 as L3  # noqa: E402
from exact import matmul, matvec, transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

GL, CP = L3.GL, L3.CP


def main():
    a = sys.argv[1:]
    table, tfile, out = a[2], a[3], Path(a[4])
    sigma = json.loads(Path(tfile).read_text())
    C = GL.Ctx(table)
    M, data, kp = C.M, L3.data, L3.kp
    words = L3.level_words(M, tuple(sigma['R']), [tuple(x) for x in sigma['x']])
    W = words[-1] + [tuple(sigma['F'])] + CP.inverse_word(M, words[-1])
    letters = sorted(set(W) | set(CP.inverse_word(M, W)))
    menu = {}

    def menu_of(c):
        if c not in menu:
            menu[c] = {to_tuple(C.E(c, j)): j for j in range(len(M.items))}
        return menu[c]

    bad, checked = [], 0
    for k in range(7200):
        K = kp.matrix(k)
        Kt = transpose(K)
        for c, i in letters:
            c2 = data.POLE.get(tuple(matvec(K, data.N[c])), -1)
            if c2 < 0 or c2 >= 600:
                bad.append({'k': k, 'letter': [c, i], 'reason': 'pole not a grip'})
                continue
            E2 = to_tuple(matmul(matmul(K, C.E(c, i)), Kt))
            if E2 not in menu_of(c2):
                bad.append({'k': k, 'letter': [c, i], 'reason': 'conjugate not in menu', 'grip': c2})
            checked += 1
        if k % 1200 == 1199:
            L3.log(k=k + 1, checked=checked, failures=len(bad))
    res = {'letters': [list(x) for x in letters], 'menu_size': len(M.items), 'kplus': 7200, 'checked': checked,
           'failures': len(bad), 'first_failures': bad[:20]}
    out.write_text(json.dumps(res, indent=1) + '\n')
    L3.log(done=str(out), failures=len(bad), checked=checked)


if __name__ == '__main__':
    main()
