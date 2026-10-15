"""For bases U0 (single jumbles at grip 0, or commutator bases), the classes that nested conjugates can still reach:
a single refills a hole whose label lies in the vacated set of the first transition, so the reachable classes are
contained in the ghost classes of the level-1 vacated holes (over all clean level-1 letters).
usage: base_reach.py <research/jumbling> <classes.json> <arrays.npz> <out json> <n random commutator bases> <seed>"""
import json, sys, time, random
from collections import Counter
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
sys.argv_saved = list(sys.argv)
H = Path(__file__).resolve().parent
sys.path.insert(0, str(H))
# reuse harvest's helpers by importing it as a module with a dummy argv
J, CL, AR, OUT, NB, SEED = sys.argv[1:7]
sys.argv = ['harvest.py', J, CL, AR, '/dev/null', SEED, '0']
import harvest as HV  # noqa: E402  (budget 0: the module's search loop does nothing)
L = HV.L
from e3lib import REGNAMES
rng = random.Random(int(SEED))
t0 = time.time()
reach_all, rows = Counter(), []
bases = [[(0, j)] for j in L.JI]
for _ in range(int(NB)):
    bases.append(HV.make_base('comm'))
for U0 in bases:
    B = HV.base_state(U0)
    if B is None:
        continue
    node = {'H': B['H'], 'lab': np.arange(len(B['H'])), 'P': B['P'], 'off': set(), 'level': 0, 'word': list(U0)}
    vcls, rcls, ng = set(), set(), set()
    for letter, ka, kai, Rh, Rl, Vh, Vl, Vimg in HV.expand(B, node, set()):
        ng.add(letter[0])
        vcls |= {int(c) for c in B['gcls'][Vl].tolist() if c >= 0}
        rcls |= {int(c) for c in B['gcls'][Rl].tolist() if c >= 0}
    reach_all.update(vcls)
    rows.append({'U0': U0, 'holes': len(B['H']), 'clean_grips_with_holes': len(ng), 'vacated_classes': len(vcls),
                 'by_registry': dict(Counter(REGNAMES[int(L.reg_code[c])] for c in vcls))})
    print(rows[-1], 'union', len(reach_all), '%.0fs' % (time.time() - t0), flush=True)
print('union by registry', dict(Counter(REGNAMES[int(L.reg_code[c])] for c in reach_all)))
json.dump({'rows': rows, 'union': sorted(reach_all)}, open(OUT, 'w'))
