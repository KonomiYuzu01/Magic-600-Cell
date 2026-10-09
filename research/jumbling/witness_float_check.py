"""Independent floating-point cross-check of the witness admissibility sets.

Run from the repository root after witness.py:  python research/jumbling/witness_float_check.py
It recomputes which grips are blocked after (c, g) and after (c, g), (d, T_d) with float
vertices and a 1e-9 band, and compares the sets with witness-results.json. It is a
consistency check only; the certified results are the exact ones.
"""
import json
import sys

import numpy as np
sys.path.insert(0,'research/jumbling')
import witness as W
from exact import matvec, transpose
r=json.load(open('research/jumbling/witness-results.json'))
c,d=0,13
NF=W.NF; nn=(NF*NF).sum(1)
g=np.array([[float(W.Q5(*x)) for x in row] for row in r['E2']['g']])
td=next(a for a in W.a4(d) if sum(float(a[i][i]) for i in range(4))==1.0 and W.pole_perm(a)[c]!=c)
T=np.array([[float(x) for x in row] for row in td])
print('108 in span(c,d)?', np.linalg.matrix_rank(np.vstack([NF[0],NF[13],NF[108]]),tol=1e-9)==2)
caps=[p for p in range(len(W.MO)-1) if 0 in W.signature(p)]
import multiprocessing as mp
cand=[e for e in range(600) if e!=0]
R=W.cap_region(0)
near=[e for e in range(600) if NF[e]@NF[0] > 0.6*(NF[0]@NF[0])]
out=W.implied(R); candp=[e for e in range(600) if e not in out and e!=0]
with mp.Pool(4) as pool:
    built=pool.map(W.build_region,[(p,candp,near) for p in caps],chunksize=32)
V={p:np.array([[float(x) for x in v] for v in vs]) for p,vs,_,_ in built}
def blocked(poses):
    bl=set()
    for e in range(600):
        for p,P in poses.items():
            hv=(V[p]@P.T)@NF[e]/nn[e]-121/125
            if hv.min()<-1e-9 and hv.max()>1e-9: bl.add(e); break
    return bl
poses={p:g for p in caps}
b1=blocked(poses)
poses2={p:(T@g if (V[p]@g.T@NF[d]/nn[d]-121/125).min()>=-1e-12 else g) for p in caps}
b2=blocked(poses2)
print('float blocked after g', len(b1), sorted(b1)==r['E2']['admissible_after_g']['blocked'])
print('float blocked after g,t', len(b2), sorted(b2)==r['E3']['blocked_after_g_then_t'], 'moved by t', sum(1 for p in caps if poses2[p] is not g))
