"""Which lattice chambers does each off unit overlap? (exact; Lemma O certificate input, menu I_a)

For every off piece p of the given blocks, with region R_d(q) (domain representative R_d, chamber q of the domain's
frame; rim.Domains), a positive weighted average of the vertices of q is an interior point of q (q is a full-dimensional
convex polytope), so its image y is an interior point of the region. y is computed exactly (Q(sqrt5), exact.Q5). If
N_c . y < |n|^2 for all 600 facets (y in the open body) and N_c . y - kappa is not zero for any of the 600 cuts, the
signs of the cuts give y's signature, and y lies in the interior of the chamber x with that signature (a chamber is
the intersection of the body with the open half-spaces of its signature, closed). So the region of p meets the
interior of x. Points outside the body or on a cut are skipped; every block reports how many samples fell outside the
body (outside), on a cut (on_cut), in a chamber (in) or on no signature of a chamber (no_chamber). Several weight vectors (--samples, fixed seed) give a set of such
chambers per piece: a lower bound of the set H(U) of chambers that the unit's region meets.
Also reported: whether the whole region lies in the closure of one chamber (no cut straddles it).

Output per block: pieces, the chambers found per piece, types of pieces and of the chambers found, whether every chamber
found is a hole at the state (it must be: a lattice piece cannot share interior points with an off piece).

usage: hloc.py <theory dir> <E-chain scripts dir> <pose table> <blocks json {blocks: [[pieces]...]}> <out json>
       [--samples N]
"""
import json
import sys
from collections import Counter
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa: E402
from exact import Q5, dot, matvec  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402

table, bfile, out = sys.argv[3:6]
a = sys.argv[6:]
NS = int(a[a.index('--samples') + 1]) if '--samples' in a else 24
RNG = np.random.default_rng(20261014)
C = GL.Ctx(table)
data, reg = GL.data, GL.restore.ctx.regions
TYPE = np.asarray(reg.orbit_of, np.int64)
NP = GL.NP
sigkey = {bytes(data.sigbits[x]): x for x in range(NP)}
assert len(sigkey) == NP, 'signatures are not unique'
holes = ~C.occ
blocks = json.loads(Path(bfile).read_text())['blocks']
_ev = {}


def ev(d, c):
    if (d, c) not in _ev:
        _ev[(d, c)] = Evaluator(pullback_form(C.D.reps[d], data.N[c]))
    return _ev[(d, c)]


def point_chamber(d, q, w):
    V = reg.vertices(q)
    tot = sum(int(x) for x in w)
    pt = [sum((V[j][i] * Q5(int(w[j])) for j in range(len(V))), Q5(0)) / Q5(tot) for i in range(4)]
    y = matvec(C.D.reps[d], pt)
    bits = np.zeros(8 * data.sigbits.shape[1], bool)
    for c in range(600):
        v = dot(data.N[c], y)
        if (v - data.NN).sign() >= 0:
            return 'outside', None
        s = (v - data.KAPPA).sign()
        if s == 0:
            return 'on_cut', None
        bits[c] = s > 0
    x = sigkey.get(bytes(np.packbits(bits)))
    return ('in', x) if x is not None else ('no_chamber', None)


def locate(p):
    d, q = int(C.D.dom[p]), int(C.D.cham[p])
    e, rows = reg.vform(q)
    found, why = set(), Counter()
    for j in range(NS):
        w = [1] * len(rows) if j == 0 else RNG.integers(1, 1000, len(rows)).tolist()
        r, x = point_chamber(d, q, w)
        why[r] += 1
        if x is not None:
            found.add(int(x))
    whole = all(ev(d, c).classify((e, rows))[0] != 'straddle' for c in range(600))
    return sorted(found), whole, why


res = []
for b in blocks:
    b = [int(p) for p in b]
    xs, wh, why = [], [], Counter()
    for p in b:
        x, w, y = locate(p)
        xs.append(x)
        wh.append(w)
        why.update(y)
    ok = sorted({x for xx in xs for x in xx})
    row = {'first': min(b), 'size': len(b), 'pieces_located': sum(1 for xx in xs if xx), 'chambers': len(ok),
           'all_holes': bool(all(holes[x] for x in ok)), 'region_in_one_chamber': sum(wh), 'samples_by_outcome': dict(sorted(why.items())),
           'piece_types': dict(sorted(Counter(int(TYPE[p]) for p in b).items())),
           'chamber_types': dict(sorted(Counter(int(TYPE[x]) for x in ok).items())),
           'x': xs, 'pieces': b, 'H0': ok}
    res.append(row)
    print(json.dumps({k: v for k, v in row.items() if k not in ('x', 'pieces', 'H0')}), flush=True)
allx = [x for r in res for x in r['H0']]
doc = {'table': Path(table).name, 'off': int(len(C.off)), 'holes': int(holes.sum()),
       'chambers_total': len(allx), 'distinct_total': len(set(allx)), 'samples': NS,
       'hole_types_all': dict(sorted(Counter(int(TYPE[x]) for x in np.flatnonzero(holes)).items())), 'blocks': res}
Path(out).write_text(json.dumps(doc) + '\n')
print(json.dumps({k: v for k, v in doc.items() if k != 'blocks'}))
