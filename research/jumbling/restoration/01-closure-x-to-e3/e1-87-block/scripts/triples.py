"""For restoring double commutators (dcomm2 rows with Bad = 198): the caps holding Bad(Q), and the involution and
lens-shape tests that window 36568's Q0 satisfies (B, A involutions; c and the pole X^-1 n_e among the caps)."""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import ghostlib as GL  # noqa: E402
from exact import matvec, transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

data = GL.data
C = GL.Ctx(sys.argv[2])
rows = [json.loads(x) for x in open(sys.argv[3])]
rows = [r for r in rows if r['T_on'] == r.get('T_on') and r['bad'] <= 198 and r['prot_moved'] == 0 and r['prot_dead'] == 0
        and r['T_dead'] == 0]
seen, out = set(), []
inv_ok = {}


def invol(c, i):
    if (c, i) not in inv_ok:
        g = C.E(c, i)
        inv_ok[(c, i)] = to_tuple(g) == to_tuple(transpose(g))
    return inv_ok[(c, i)]


for r in rows:
    key = (r['e'], r['B'], r['c'], r['X'], r['a'], r['A'])
    if key in seen:
        continue
    seen.add(key)
    Q = [tuple(x) for x in r['Q']]
    badQ, _, _ = C.bad(Q)
    caps = np.flatnonzero(data.sig_matrix(badQ).sum(axis=0) == len(badQ)).tolist()
    pe = data.POLE.get(tuple(matvec(transpose(C.E(r['c'], r['X'])), data.N[r['e']])), -1)
    out.append(dict(r, Q=r['Q'], caps=caps, invB=invol(r['e'], r['B']), invA=invol(r['a'], r['A']),
                    c_in=r['c'] in caps, pe=pe, pe_in=pe in caps, e_in=r['e'] in caps))
Path(sys.argv[4]).write_text('\n'.join(json.dumps(x) for x in out) + '\n')
from collections import Counter  # noqa: E402
print(len(out), Counter((x['invB'], x['invA'], x['c_in'], x['pe_in'], x['e_in']) for x in out).most_common())
