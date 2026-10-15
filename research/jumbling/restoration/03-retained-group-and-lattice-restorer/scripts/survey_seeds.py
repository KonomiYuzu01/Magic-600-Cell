import json, sys, time
from math import lcm
import numpy as np
sys.path.insert(0, sys.argv[1])
from rlib import Retained, sparse, perm_order_on
R = Retained(sys.argv[2])
out = []
for o in range(35):
    t = R.trees[o]; at = R.atlas[o]
    assert t['seed'] == at['word']
    t0 = time.time()
    P = R.word_perm(t['seed'])
    s, d = sparse(P)
    orbs = sorted(set(int(x) for x in R.so[s]))
    tgt = s[R.so[s] == o]; col = s[R.so[s] != o]
    tp = sorted(set(int(x) for x in R.sp[tgt]))
    ot = perm_order_on(P, tgt); oc = perm_order_on(P, col) if len(col) else 1
    per = {}
    for q in orbs:
        if q == o: continue
        ss = s[R.so[s] == q]
        per[q] = dict(stickers=int(len(ss)), pieces=int(len(set(R.sp[ss]))), order=perm_order_on(P, ss))
    rec = dict(orbit=o, seed_len=len(t['seed']), support=int(len(s)), target_pieces=len(tp), target_order=ot,
               collateral_order=oc, collateral=per, secs=round(time.time()-t0, 2))
    out.append(rec)
    print(o, len(t['seed']), len(s), len(tp), ot, oc, {q: (v['pieces'], v['order']) for q, v in per.items()}, flush=True)
json.dump(out, open(sys.argv[3], 'w'), indent=1)
