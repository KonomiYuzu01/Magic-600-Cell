"""Orbit composition of cap 0 intersect cap d for every grip d sharing a slot with cap 0, grouped by angle.
usage: shared36.py <research/jumbling>"""
import sys
from collections import Counter, defaultdict
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
ctx = sim.get_context(); d = ctx.data
NF = np.array([[float(v) for v in row] for row in d.N])
ang = np.degrees(np.arccos(np.clip(NF @ NF[0] / float(d.NN), -1, 1)))
share = defaultdict(Counter)
for p in d.cap_members(0):
    p = int(p)
    for c in d.signature_list(p):
        c = int(c)
        if c != 0:
            share[c][int(d.orbit_id[p])] += 1
by = defaultdict(Counter)
for c, cnt in share.items():
    by[round(float(ang[c]), 2)][tuple(sorted(cnt.items()))] += 1
for a in sorted(by):
    for comp, n in by[a].items():
        print(a, 'grips', n, 'shared slots by orbit', dict(comp))
