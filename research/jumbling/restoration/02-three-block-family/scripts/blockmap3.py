"""For a chamber z0 and each template level: per grip holding pi_{P_k}(z0), how many block chambers (L_k) it holds and
how many parked contents it moves or cuts. Word-only.

Usage: python -I blockmap3.py <theory dir> <v26 dir> <pose table> <template json> <z0> <out json>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import layers3 as L3  # noqa: E402

CP, GL = L3.CP, L3.GL


def main():
    a = sys.argv[1:]
    table, tfile, z0, out = a[2], a[3], int(a[4]), Path(a[5])
    sigma = json.loads(Path(tfile).read_text())
    C = GL.Ctx(table)
    H = L3.HOME()
    Bp = np.asarray(C.off, np.int64)
    dom0, q0 = C.contents(Bp)
    words = L3.level_words(C.M, tuple(sigma['R']), [tuple(x) for x in sigma['x']])
    allg = sorted({sigma['R'][0]} | {x[0] for x in sigma['x']} | ({sigma['F'][0]} if sigma.get('F') else set()))
    U = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in allg]))
    rows = []
    for k, Pk in enumerate(words):
        gP = L3.Chambers(C, Pk, U)
        blk = L3.block_run(C, Pk, dom0, q0, [C.st.pose(int(p)) for p in Bp])
        v = int(gP.pi[z0])
        pd, pq = gP.parked()
        row = {'k': k, 'v': v, 'parked': int(len(pq)), 'L': blk['chambers'], 'grips': []}
        if v >= 0:
            for c in np.flatnonzero(H[v]).tolist():
                sd = L3.sides(C, pd, pq, c)
                row['grips'].append({'grip': c, 'L_in_cap': int(H[blk['chambers'], c].sum()),
                                     'parked_in': int((sd == 1).sum()), 'parked_cut': int((sd < 0).sum()),
                                     'cap_size': int(H[:, c].sum())})
        rows.append(row)
        L3.log(k=k, v=v, parked=row['parked'], grips=[(g['grip'], g['L_in_cap'], g['parked_in'], g['parked_cut'])
                                                       for g in row['grips']])
    out.write_text(json.dumps(rows, indent=1) + '\n')


if __name__ == '__main__':
    main()
