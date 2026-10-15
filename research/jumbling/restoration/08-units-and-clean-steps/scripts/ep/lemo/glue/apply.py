"""Apply a path of K+ letters, given as [grip, K+ index], to a pose table on the exact simulator; write the new table,
the path as menu letters [grip, item], and where the holes of the given types are afterwards.

usage: apply.py <theory> <echain> <table> <path json [[c, k], ...]> <out table> <out json> --types 7,9
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import buffer as B  # noqa: E402
import ghostlib as GL  # noqa: E402

table, pfile, otab, ojs = sys.argv[3:7]
a = sys.argv[7:]
TYPES = [int(x) for x in a[a.index('--types') + 1].split(',')]
C = GL.Ctx(table)
R, M, st = C.R, C.M, C.st
kp = GL.BL.CL.ctx.kplus
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
path = json.loads(Path(pfile).read_text())
word = []
for c, k in path:
    its = [i for i in range(len(M.items)) if kp.index_of_matrix(GL.restore.element(M, int(c), i)) == int(k)]
    assert len(its) == 1, (c, k, its)
    word.append([int(c), its[0]])
d_in = st.digest()
off_in = sorted(map(int, C.off))
for c, i in word:
    R.apply(c, i)
B.export_pose_table(st, otab)
C2 = GL.Ctx(otab)
holes = np.flatnonzero(~C2.occ & np.isin(TYPE, TYPES))
doc = {'input_digest': d_in, 'output_digest': st.digest(), 'path': path, 'word': word,
       'off_set_unchanged': sorted(map(int, C2.off)) == off_in, 'rare_holes': holes.tolist()}
Path(ojs).write_text(json.dumps(doc) + '\n')
print(json.dumps(doc))
