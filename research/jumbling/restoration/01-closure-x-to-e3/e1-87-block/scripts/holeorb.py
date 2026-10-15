"""Orbit sizes (SAFE-ALL group) of the holes at a state, and the hole chambers' cap sets against the off blocks."""
import json, sys
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent))
import blocks as BL
import buffer as B
R, M = BL.O.start(sys.argv[2])
G = B.Alphabet(R, M); G.orbits()
bl = BL.Blocks(R, M)
h = np.asarray(G.holes)
sz = G.orbit_size[G.orbit[h]]
out = {'holes': len(h), 'fixed_holes': int((sz == 1).sum()), 'hole_orbit_sizes': sorted(map(int, sz)),
       'blocks': [{'size': len(p), 'I': bl.I[b].tolist(), 'S': bl.S[b].tolist(), 'locked': bl.locked[b],
                   'vacant_locked': bl.vacant_locked[b]} for b, p in enumerate(bl.blocks)]}
print(json.dumps(out))
