"""For each off-lattice domain F at a configuration: the caps that hold F whole, and the smallest
intersection of 1, 2, 3 or 4 holders that contains F (extra = pieces in the intersection outside F).
Usage: python research/jumbling/theory/isolate.py <menu> <t> <moves.json> [--prefix k]
Run from the repository root. Evidence kind: source and exact synthetic geometry; timings are cloud
timings of research code, not performance evidence."""
import sys, json, itertools, argparse
import numpy as np
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import seams
ap = argparse.ArgumentParser(); ap.add_argument('menu'); ap.add_argument('t', type=int); ap.add_argument('moves')
ap.add_argument('--prefix', type=int); a_ = ap.parse_args()
moves = json.load(open(a_.moves))['moves']
if a_.prefix is not None:
    moves = moves[:a_.prefix]
st, M = seams._fixture_state(a_.menu, a_.t); R = seams.SeamRestorer(st, M)
for c, i in moves:
    R.apply(c, i)
D = R.domains()
sets = {c: frozenset(np.asarray(R.inside(c)).tolist()) for c in range(600) if R.inside(c) is not None}
blocked = 600 - len(sets)
print(json.dumps({'N': R.misalignment(), 'off': int((D.dom != 0).sum()), 'admissible_caps': len(sets)}))
for d in range(1, len(D.members)):
    F = frozenset(np.asarray(D.members[d]).tolist())
    H = [c for c, s in sets.items() if F <= s]
    row = {'domain': d, 'size': len(F), 'holders': len(H)}
    for k in (1, 2, 3, 4):
        best = None
        for comb in itertools.combinations(H, k):
            I = frozenset.intersection(*(sets[c] for c in comb))
            e = len(I) - len(F)
            if best is None or e < best[0]:
                best = (e, comb)
            if e == 0:
                break
        row[f'k{k}'] = None if best is None else [best[0], list(best[1])]
        if best and best[0] == 0:
            break
    # also: is F the intersection of ALL holders?
    if H:
        I = frozenset.intersection(*(sets[c] for c in H)); row['all_holders_extra'] = len(I) - len(F)
    print(json.dumps(row), flush=True)
