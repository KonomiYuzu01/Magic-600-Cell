"""For cap 0: the grips d != 0 whose caps share a home of each orbit with cap 0, grouped by angle.
usage: share45.py <research/jumbling> <orbit ...>"""
import sys
from pathlib import Path
from collections import defaultdict
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
ctx = sim.get_context(); d = ctx.data
NF = np.array([[float(v) for v in row] for row in d.N])
ang = np.degrees(np.arccos(np.clip(NF @ NF[0] / float(d.NN), -1, 1)))
members = [int(p) for p in d.cap_members(0)]
for o in [int(x) for x in sys.argv[2:]]:
    homes = [p for p in members if int(d.orbit_id[p]) == o]
    by = defaultdict(set)
    for p in homes:
        for c in d.signature_list(p):
            c = int(c)
            if c != 0:
                by[round(float(ang[c]), 2)].add(c)
    print('orbit', o, 'homes in cap 0', len(homes), 'sharing grips by angle', {a: len(s) for a, s in sorted(by.items())}, flush=True)
