"""Reachable-class bound for single-jumble bases at a representative grip of every grip class (class 0's grip 0 was
base_reach.py), and for two-jumble bases mixing a class-0 grip with a grip of another class.
Bound: ghost classes of the level-1 vacated holes over all clean level-1 letters (see base_reach.py).
usage: base_reach3.py <research/jumbling> <classes.json> <arrays.npz> <out json> <n mixed bases> <seed> [prior json]"""
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
reps = {}
for c in range(600):
    reps.setdefault(L.cls[c], c)
print('grip classes and representatives', reps, flush=True)
reach = set(json.load(open(PRIOR))['union']) if PRIOR else set()
rows = []


def run(U0, kind):
    global reach
    B = HV.base_state(U0)
    if B is None:
        print('inadmissible', U0, flush=True)
        return
    node = {'H': B['H'], 'lab': np.arange(len(B['H'])), 'P': B['P'], 'off': set(), 'level': 0, 'word': list(U0)}
    vcls, ng = set(), set()
    for letter, ka, kai, Rh, Rl, Vh, Vl, Vimg in HV.expand(B, node, set()):
        ng.add(letter[0])
        vcls |= {int(x) for x in B['gcls'][Vl].tolist() if x >= 0}
    new = vcls - reach
    reach |= vcls
    gc = Counter(REGNAMES[int(L.reg_code[x])] for x in set(B['gcls'][B['alive'] & (B['gcls'] >= 0)].tolist()))
    rows.append({'U0': U0, 'kind': kind, 'holes': len(B['H']), 'alive': int(B['alive'].sum()),
                 'ghost_classes_by_registry': dict(gc), 'clean_grips_with_holes': len(ng), 'vacated_classes': len(vcls),
                 'new': len(new), 'new_by_registry': dict(Counter(REGNAMES[int(L.reg_code[x])] for x in new))})
    print(rows[-1], 'union', len(reach), '%.0fs' % (time.time() - t0), flush=True)


for P, c in sorted(reps.items()):
    if c == 0:
        continue
    for j in L.JI:
        run([(c, j)], f'single@class{P}')
others = [c for c in range(600) if L.cls[c] != L.cls[0]]
for n in range(int(NB)):
    c = rng.choice(sorted(others, key=lambda x: -float(NF[0] @ NF[x]))[:40])
    U0 = [(0, rng.choice(L.JI)), (c, rng.choice(L.JI))]
    if rng.random() < 0.5:
        U0 = U0[::-1]
    run(U0, 'mixed')
print('union by registry', dict(Counter(REGNAMES[int(L.reg_code[x])] for x in reach)), 'total', len(reach))
json.dump({'rows': rows, 'union': sorted(reach)}, open(OUT, 'w'))
