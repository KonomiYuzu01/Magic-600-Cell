"""All restoring Q of B' at the current state: Bad(Q) shape against the parking template (caps holding all of Bad(Q)),
chambers whose content straddles (dead) and their status (hole, fixed by SAFE-ALL), the locked chambers, and |Bad(Q^-1)|
for the type-S use of Q as P. Reads only the state's pose table.
Usage: python -I qstruct.py <theory dir> <pose table> <dcomm2 jsonl> <out jsonl>"""
import json, sys, time
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import ghostlib as GL
import buffer as B
import conjprep as CP
C = GL.Ctx(sys.argv[2])
G = B.Alphabet(C.R, C.M); G.orbits()
fixed = G.orbit_size[G.orbit] == 1
locked = np.array(C.bl.locked[0], np.int64)
summary = {'safe_grips': len(G.safe), 'holes': np.flatnonzero(~C.occ).tolist(),
           'hole_orbit_sizes': [int(G.orbit_size[G.orbit[h]]) for h in np.flatnonzero(~C.occ)],
           'locked': locked.tolist(), 'locked_fixed': [bool(fixed[x]) for x in locked],
           'locked_occupied': [bool(C.occ[x]) for x in locked]}
print(json.dumps(summary), flush=True)
seen = set(); T0 = time.time(); n = 0
with open(sys.argv[4], 'w') as fh:
    for line in open(sys.argv[3]):
        r = json.loads(line)
        k = json.dumps(r['Q'])
        if k in seen:
            continue
        seen.add(k)
        Q = [tuple(x) for x in r['Q']]
        bad, g, ch = C.bad(Q)
        sig = GL.data.sig_matrix(bad)
        allcaps = np.flatnonzero(sig.sum(axis=0) == len(bad)).tolist()
        dead = ch[~g.alive]
        badi, _, _ = C.bad(CP.inverse_word(C.M, Q))
        fh.write(json.dumps({'Q': r['Q'], 'c': r['c'], 'e': r['e'], 'a': r['a'], 'bad': int(len(bad)),
                             'caps_holding_all_bad': allcaps, 'dead': dead.tolist(),
                             'dead_holes': int((~C.occ[dead]).sum()), 'dead_fixed_occupied': int((fixed[dead] & C.occ[dead]).sum()),
                             'bad_fixed_occupied': int((fixed[bad] & C.occ[bad]).sum()),
                             'locked_in_bad': int(np.isin(locked, bad).sum()), 'bad_inverse': int(len(badi))}) + '\n')
        n += 1
print(json.dumps({'distinct': n, 'sec': round(time.time() - T0)}))
