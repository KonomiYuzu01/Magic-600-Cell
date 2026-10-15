"""For grip 0: every grip d sharing an inner-orbit home with cap 0, the shared homes by orbit, the class of d,
and the angle between n_0 and n_d."""
import json, sys
from pathlib import Path
from collections import Counter, defaultdict
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
ctx = sim.get_context()
d = ctx.data
cls = {int(k): v for k, v in json.load(open(sys.argv[2])).items()}
INNER = [int(x) for x in sys.argv[3].split(',')]
S2 = [int(x) for x in sys.argv[4].split(',')]
NF = np.array([[float(v) for v in row] for row in d.N])
members = [int(p) for p in d.cap_members(0)]
share = defaultdict(Counter)
allshare = defaultdict(Counter)
for p in members:
    o = int(d.orbit_id[p])
    for c in d.signature_list(p):
        if c != 0:
            allshare[c][o] += 1
            if o in INNER:
                share[c][o] += 1
for c in sorted(share, key=lambda c: -NF[c] @ NF[0]):
    ang = np.degrees(np.arccos(NF[c] @ NF[0] / float(d.NN)))
    print(c, 'class', cls[c], 'angle %.2f' % ang, 'S2fail' if c in S2 else '', 'inner shared', dict(sorted(share[c].items())),
          'all shared total', sum(allshare[c].values()))
