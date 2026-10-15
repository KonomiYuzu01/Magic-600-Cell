"""Along the reverse W-J history from the fixture (Z_40, Z_39, ...): at each Z_j, the 12 letters of the coset that
contains the exact inverse of t_j, scored by sibling support (hpcore.Siblings) and by hole alignment, and the rank of
the exact inverse under each.

Hole alignment of a letter at cap c: restored pieces land in chambers R_X inside cap c; weight of a chamber z inside
cap c = sum over caps c' != c containing z of the holes of c' outside cap c. Score = -(sum of weights over R_X): restored
pieces should land where the outside is lattice too, leaving holes next to outside holes.
Also reported: the old rule (lattice-free overlapping caps).

usage: lettertest2.py <J> <SC> <fixture table> <from j> <to j>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'gf'))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'hp'))
import glib  # noqa: E402

J, SC, TABLE, A, B = sys.argv[1:6]
glib.setup(J + '/theory', SC)
sys.path.insert(0, J + '/fixtures')
import hpcore as H  # noqa: E402
import wj  # noqa: E402

W = glib.World(TABLE, '-')
G = H.Geo(W)
fixture, _ = wj.load(W.GL.BL.CL.ctx, 'I_a')
recs = [(int(r['grip']), int(r['params']['label'].split('[')[1].rstrip(']'))) for r in fixture['journal']['records'][:40]]
st = H.HState.from_world(W, G)


def score_letters(st, c, lets):
    row = st.row(c)
    ins = row['inside']
    occ = st.occ()
    capc = G.capm[c]
    ov = [c2 for c2 in G.overlap(c).tolist() if c2 != c]
    capov = G.capm[ov]
    out_cham = capov & ~capc
    h_out = (out_cham & ~occ).sum(axis=1)  # holes of c' outside cap c
    w = (capov * h_out[:, None]).sum(axis=0)  # per chamber
    lat = np.flatnonzero(st.dom == 0)
    lat_in = lat[capc[st.q[lat]]]
    occ0 = occ.copy()
    occ0[st.q[lat_in]] = False
    res = []
    sib = H.Siblings(G, st, ins)
    lat = np.flatnonzero(st.dom == 0)
    allc = np.concatenate([ins, lat[G.capm[c][st.q[lat]]]])
    for X in lets:
        da, qa = glib.moved_contents(W, st.dom[allc], st.q[allc], W.E(c, X))
        d2, q2 = da[:len(ins)], qa[:len(ins)]
        Rq = q2[d2 == 0]
        o = occ0.copy()
        o[Rq] = True
        free = int(((capov & o).sum(axis=1) == 0).sum())
        res.append({'X': X, 'align': -int(w[Rq].sum()), 'free': free, 'restored': int(len(Rq)),
                    'moved': int((d2 != 0).sum()), 'support': sib.support(da, qa)})
    return row, res


for j in range(int(A), int(B) - 1, -1):
    c, item = recs[j - 1]
    xinv = W.inv(c, item)
    row = st.row(c)
    dset = [d for d in row['frames'] if xinv in G.letters(c, d)]
    out = {'j': j, 'cap': c, 'inverse': xinv, 'strad': row['strad'], 'occ': row['occ'], 'frames': row['frames']}
    if row['strad'] or not dset:
        out['note'] = 'no restoring frame' if not dset else 'straddled'
    else:
        _, res = score_letters(st, c, G.letters(c, dset[0]))
        al = sorted(res, key=lambda r: -r['align'])
        su = sorted(res, key=lambda r: (-r['support'], -r['align']))
        out.update({'support_rank': [r['X'] for r in su].index(xinv) + 1,
                    'support_ties': sum(1 for r in res if r['support'] == su[0]['support']),
                    'support_inv_minus_best_other': next(r['support'] for r in res if r['X'] == xinv) - max(
                        r['support'] for r in res if r['X'] != xinv)})
        out.update({'moved': res[0]['moved'], 'restored': res[0]['restored'],
                    'align_rank': [r['X'] for r in al].index(xinv) + 1,
                    'align_ties': sum(1 for r in res if r['align'] == al[0]['align']),
                    'align_inv_minus_best_other': next(r['align'] for r in res if r['X'] == xinv) - max(
                        r['align'] for r in res if r['X'] != xinv),
                    'free_max': max(r['free'] for r in res), 'free_inv': next(r['free'] for r in res if r['X'] == xinv),
                    'free_ties': sum(1 for r in res if r['free'] == max(x['free'] for x in res))})
    print(json.dumps(out), flush=True)
    st = st.apply([(c, xinv)])
    if st is None:
        print(json.dumps({'j': j, 'error': 'inverse not admissible'}))
        break
