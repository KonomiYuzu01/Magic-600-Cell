"""Rare-type holes at a state and their relation to the off contents of the unit."""
import json, sys
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve(); sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa
C = GL.Ctx(sys.argv[3])
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
for t in (7, 9):
    h = np.flatnonzero((~C.occ) & (TYPE == t))
    offt = [int(p) for p in C.off if TYPE[p] == t]
    print(json.dumps({'type': t, 'holes': h.tolist(), 'off_pieces': offt}))
for f in sys.argv[4:]:
    T = json.loads(Path(f).read_text())['pieces']
    Tr = [p for p in T if TYPE[p] in (7, 9)]
    d, q = C.contents(Tr)
    print(json.dumps({'file': Path(f).name, 'rare_pieces': Tr, 'contents': list(zip(d.tolist(), q.tolist()))}))
