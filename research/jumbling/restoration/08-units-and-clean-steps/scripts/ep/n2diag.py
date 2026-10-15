"""Which SAFE-ALL orbits fail N2 for given words at a state (diagnostic).

usage: n2diag.py <theory> <echain> <table> <words jsonl> <out json>
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

table, wfile, out = sys.argv[3:6]
C = GL.Ctx(table)
G = B.Alphabet(C.R, C.M)
G.orbits()
GO = np.asarray(G.orbit, np.int64)
NO = len(G.orbit_size)
holes = np.zeros(GL.NP, bool)
holes[G.holes] = True
HO = np.bincount(GO[np.flatnonzero(holes)], minlength=NO)
TYPE = np.asarray(GL.data.orbit_id, np.int64) + 1
res = {'safe_grips': len(G.safe), 'orbits': int(NO), 'singletons': int((G.orbit_size == 1).sum()), 'words': []}
for line in Path(wfile).read_text().splitlines():
    W = [(int(c), int(i)) for c, i in json.loads(line)['W']]
    g, m, U = C.ghost(W, np.zeros(0, np.int64))
    bad = U[~g.alive[m:] | (g.dom[m:] != 0)]
    need = np.bincount(GO[bad], minlength=NO)
    short = np.flatnonzero(need > HO)
    rows = []
    for o in short.tolist():
        ch = np.flatnonzero(GO == o)
        rows.append({'orbit': o, 'size': int(G.orbit_size[o]), 'bad': int(need[o]), 'holes': int(HO[o]),
                     'types': sorted(set(TYPE[ch].tolist())), 'bad_occupied': int((~holes[bad[GO[bad] == o]]).sum())})
    res['words'].append({'W': [list(x) for x in W], 'bad': int(len(bad)), 'short': rows})
Path(out).write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps(res)[:3000])
