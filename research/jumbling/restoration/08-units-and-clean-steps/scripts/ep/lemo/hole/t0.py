"""Types with no off piece (no hole) at a state, with their chamber counts, and off pieces per type."""
import json, sys
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve(); sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa
C = GL.Ctx(sys.argv[3])
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
NT = int(TYPE.max()) + 1
holes = np.bincount(TYPE[~C.occ], minlength=NT)
offt = np.bincount(TYPE[C.off], minlength=NT)
size = np.bincount(TYPE, minlength=NT)
assert (holes == offt).all()
print(json.dumps({'types': NT, 'zero_hole_types': [int(t) for t in np.flatnonzero(holes == 0)],
                  'size': {int(t): int(size[t]) for t in range(NT)}, 'holes': {int(t): int(holes[t]) for t in range(NT)}}))
