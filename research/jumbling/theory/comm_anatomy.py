"""Anatomy of one commutator at a configuration: which pieces change lattice status, by region class.
Usage: python research/jumbling/theory/comm_anatomy.py <menu> <t> <moves.json> d a c q
Run from the repository root. Evidence kind: source and exact synthetic geometry; timings are cloud
timings of research code, not performance evidence."""
import sys, json
import numpy as np
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import seams
from sim.kplus import to_tuple
menu, t, mv, d, a, c, q = sys.argv[1], int(sys.argv[2]), sys.argv[3], *map(int, sys.argv[4:8])
st, M = seams._fixture_state(menu, t); R = seams.SeamRestorer(st, M)
for x, i in json.load(open(mv))['moves']:
    R.apply(x, i)
kp = seams.ctx.kplus
inK = lambda m: kp.index_of_matrix(to_tuple(m)) >= 0
print('retained a, q:', inK(R.g(d, a)), inK(R.g(c, q)))
D0 = R.domains(); on0 = D0.dom == 0
Sd0, Sc0 = set(np.asarray(R.inside(d)).tolist()), set(np.asarray(R.inside(c)).tolist())
word = [(d, a), (c, q), (d, R.inv_item(d, a)), (c, R.inv_item(c, q))]
hist = {}
for w in word:
    Sd, Sc = set(np.asarray(R.inside(d)).tolist()), set(np.asarray(R.inside(c)).tolist())
    mover = Sd if w[0] == d else Sc
    for p in mover:
        hist.setdefault(p, []).append(len(hist.get(p, [])) and 0)
    R.apply(*w)
    for p in mover:
        hist[p][-1] = word.index(w) + 1
D1 = R.domains(); on1 = D1.dom == 0
pat = {}
for p, h in hist.items():
    k = tuple(h)
    s = pat.setdefault(k, [0, 0, 0])
    s[0] += 1
    if on0[p] and not on1[p]: s[1] += 1
    if not on0[p] and on1[p]: s[2] += 1
print('N', R.misalignment(), 'off', int((~on1).sum()), 'was', int((~on0).sum()))
print('moved by (which of the 4 twists): count, lifted, returned')
for k, s in sorted(pat.items(), key=lambda x: -x[1][0]):
    print(k, s)
