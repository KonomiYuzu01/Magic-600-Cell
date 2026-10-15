"""Reachable-class bound for class-puzzle bases: two or three jumbles at grips of grip 0's class (they never block each
other), optionally conjugated: U0 = (0,j1)(c,j2) or (0,j1)(c,j2)(0,j1^-1) or (c,j2)(0,j1)(c,j2^-1), c a same-class grip
at 36 or 60 degrees. Bound: classes of the level-1 vacated holes' ghosts.
usage: base_reach2.py <research/jumbling> <classes.json> <arrays.npz> <out json> <n bases> <seed> [prior json]"""
import json, sys, time, random
from collections import Counter
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
H = Path(__file__).resolve().parent
sys.path.insert(0, str(H))
J, CL, AR, OUT, NB, SEED = sys.argv[1:7]
PRIOR = sys.argv[7] if len(sys.argv) > 7 else None
sys.argv = ['harvest.py', J, CL, AR, '/dev/null', SEED, '0']
import harvest as HV  # noqa: E402
L = HV.L
from e3lib import REGNAMES
rng = random.Random(int(SEED))
t0 = time.time()
NF = HV.NF
P0 = L.cls[0]
ang = lambda c: float(np.degrees(np.arccos(np.clip(NF[0] @ NF[c] / (NF[0] @ NF[0]), -1, 1))))
same = [c for c in range(1, 600) if L.cls[c] == P0]
near = sorted(same, key=ang)
print('same-class grips by angle', sorted(Counter(round(ang(c), 1) for c in same).items())[:6], flush=True)
reach = set(json.load(open(PRIOR))['union']) if PRIOR else set()
rows = []
for n in range(int(NB)):
    c = rng.choice([x for x in near if ang(x) < 61])
    j1, j2 = rng.choice(L.JI), rng.choice(L.JI)
    kind = rng.choice(['pair', 'conj1', 'conj2'])
    if kind == 'pair':
        U0 = [(0, j1), (c, j2)]
    elif kind == 'conj1':
        U0 = [(0, j1), (c, j2), L.inv_letter((0, j1))]
    else:
        U0 = [(c, j2), (0, j1), L.inv_letter((c, j2))]
    B = HV.base_state(U0)
    if B is None:
        print('inadmissible', U0, flush=True)
        continue
    node = {'H': B['H'], 'lab': np.arange(len(B['H'])), 'P': B['P'], 'off': set(), 'level': 0, 'word': list(U0)}
    vcls, ng = set(), set()
    for letter, ka, kai, Rh, Rl, Vh, Vl, Vimg in HV.expand(B, node, set()):
        ng.add(letter[0])
        vcls |= {int(x) for x in B['gcls'][Vl].tolist() if x >= 0}
    new = vcls - reach
    reach |= vcls
    gc = Counter(REGNAMES[int(L.reg_code[x])] for x in set(B['gcls'][B['alive'] & (B['gcls'] >= 0)].tolist()))
    rows.append({'U0': U0, 'kind': kind, 'angle': round(ang(c), 1), 'holes': len(B['H']), 'alive': int(B['alive'].sum()),
                 'ghost_classes_by_registry': dict(gc), 'clean_grips_with_holes': len(ng), 'vacated_classes': len(vcls),
                 'new': len(new), 'new_by_registry': dict(Counter(REGNAMES[int(L.reg_code[x])] for x in new))})
    print(rows[-1], 'union', len(reach), '%.0fs' % (time.time() - t0), flush=True)
print('union by registry', dict(Counter(REGNAMES[int(L.reg_code[x])] for x in reach)))
json.dump({'rows': rows, 'union': sorted(reach)}, open(OUT, 'w'))
