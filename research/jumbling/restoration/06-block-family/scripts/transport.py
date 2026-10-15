"""K+ transport of a block word (menu I_a): every m in K+ that maps a source block onto a target block, and the
transported words m W m^-1.

The region of a piece is its home chamber moved by its pose. m maps the source block onto the target block when m
moves every source region onto a target region, bijectively. Candidates for m come from float centroids; every
reported m is then confirmed exactly: for every source piece p and its matched target piece p', the exact vertex set of
m X_p (home of p) equals the exact vertex set of X_p' (home of p') (Q(sqrt5) arithmetic). Letters are transported by
e3lib.Lib.conj_letter: (c, i) -> (m(c), i') with element(m(c), i') = K_m element(c, i) K_m^-1, an exact product and an
exact menu lookup.

usage: transport.py <research/jumbling> <e3lib dir> <classes json> <arrays npz> <source table> <source target json>
       <word json {P, Q}> <target table> <target target json> <out json>
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

J, LIBDIR, CLS, ARR, STAB, SBLK, WFILE, TTAB, TBLK, OUT = sys.argv[1:11]
sys.path.insert(0, str(Path(LIBDIR).resolve()))
sys.dont_write_bytecode = True
from e3lib import Lib  # noqa: E402

t0 = time.time()
L = Lib(J, CLS, ARR)
from exact import matvec  # noqa: E402
from sim.kplus import matrix_from_json  # noqa: E402

reg, kp = L.reg, L.kp
repVf = [np.array([[float(x) for x in v] for v in vs]) for vs in reg.rep_vertices]


def load(table, blk):
    tab = json.load(open(table))
    ps = sorted(int(p) for p in json.load(open(blk))['pieces'])
    mats = {}
    for p in ps:
        mats[p] = matrix_from_json(tab['poses'][tab['pose_id'][p]])
    return tab, ps, mats


def centroids(ps, mats):
    out = []
    for p in ps:
        V = repVf[L.orb[p]] @ L.Kf[L.tr[p]].T @ L.fm(mats[p]).T
        out.append(V.mean(0))
    return np.array(out)


def exact_region(p, X, pre=None):
    home = [matvec(kp.matrix(int(L.tr[p])), v) for v in reg.rep_vertices[int(L.orb[p])]]
    pts = [matvec(X, w) for w in home]
    if pre is not None:
        pts = [matvec(pre, w) for w in pts]
    return frozenset(tuple(repr(x) for x in w) for w in pts)


stab, sps, smat = load(STAB, SBLK)
ttab, tps, tmat = load(TTAB, TBLK)
assert len(sps) == len(tps), 'blocks differ in size'
Cs, Ct = centroids(sps, smat), centroids(tps, tmat)
tkey = {tuple(np.round(c, 5).tolist()): j for j, c in enumerate(Ct)}
found = []
for m in range(7200):
    img = Cs @ L.Kf[m].T
    match = [tkey.get(tuple(np.round(c, 5).tolist())) for c in img]
    if all(x is not None for x in match) and len(set(match)) == len(match):
        found.append((m, match))
confirmed = []
for m, match in found:
    Km = kp.matrix(m)
    ok = all(exact_region(p, smat[p], Km) == exact_region(tps[j], tmat[tps[j]]) for p, j in zip(sps, match))
    if ok:
        confirmed.append({'m': int(m), 'pairs': [[int(p), int(tps[j])] for p, j in zip(sps, match)]})
W = json.load(open(WFILE))
words = []
for row in confirmed:
    m = row['m']
    words.append({'m': m, 'P': [list(L.conj_letter(m, (int(c), int(i)))) for c, i in W['P']],
                  'Q': [list(L.conj_letter(m, (int(c), int(i)))) for c, i in W['Q']]})
res = {'source_table_digest': stab['digest'], 'target_table_digest': ttab['digest'], 'source_pieces': sps,
       'target_pieces': tps, 'float_candidates': len(found), 'confirmed': confirmed, 'words': words,
       'source_word': {'P': W['P'], 'Q': W['Q']}, 'seconds': round(time.time() - t0, 1)}
Path(OUT).write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps({'float_candidates': len(found), 'confirmed': [r['m'] for r in confirmed],
                  'words': [(w['m'], w['Q'][:3]) for w in words], 'sec': res['seconds']}))
