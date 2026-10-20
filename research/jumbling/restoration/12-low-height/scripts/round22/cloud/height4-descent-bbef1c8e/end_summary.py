# Theory round 22, line (d), round 6, cloud line height4-descent-bbef1c8e: replay of the whole line. From a snapshot of
# the 3000-twist stop of seed 7302 (015d839e...), plays the given letters in order (each admissible and certain), and
# after each prints its digest, counts at heights 2/4/6, off count and N, checking the listed digests (bbef1c8e at the
# start of this line and every phase end). At the end: how many pieces that were on the lattice at bbef1c8e are off
# (lattice pieces taken off on the way), how many height-4 pieces of bbef1c8e are at heights 0, 2 and 4, and the same
# against 015d839e.
# Research only, headless Linux; exact geometry. It writes no file; theory/seams.py keeps its cache in
# work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Run from the repository root:
#   OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=<file> python3 -I -B -u \
#     research/jumbling/restoration/12-low-height/scripts/round22/cloud/height4-descent-bbef1c8e/end_summary.py \
#     <dir>/s7302-stop.pkl '<letters since 015d839e as json>' '<json {"index": "digest", ...} of checked digests>'
import sys, json
from collections import Counter
import numpy as np
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load

C = E.sim.get_context(); M = E.R.menu('I_a'); s, _ = load(sys.argv[1], E)
letters = json.loads(sys.argv[2]); check = {int(k): v for k, v in json.loads(sys.argv[3]).items()}
cache = {}


def heights():
    for p in s._poses:
        if p.key not in cache:
            if p.kidx >= 0:
                cache[p.key] = 0
            else:
                l, r = E.T.factors(p.matrix); assert E.T.dist(r) == 0; cache[p.key] = E.T.dist(l)
    return np.array([cache[p.key] for p in s._poses])[s.pose_id]


h0 = heights(); hb = None
for k, (c, j) in enumerate(letters, 1):
    o = s.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain
    h = heights(); d = s.digest()
    if k in check:
        assert d == check[k], (k, d)
    if d.startswith('bbef1c8e'):
        hb = h.copy()
    print('LETTER', json.dumps(dict(n=k, letter=[c, j], n2_n4_n6=[int((h == v).sum()) for v in (2, 4, 6)], off=int((h > 0).sum()),
                                    N=E.R.Restorer(s, M).misalignment(), digest=d)), flush=True)
h = heights()
print('END', json.dumps(dict(digest=s.digest(),
                             lattice_at_bbef1c8e_now_off=int(((hb == 0) & (h > 0)).sum()),
                             off_at_bbef1c8e_now_lattice=int(((hb > 0) & (h == 0)).sum()),
                             height4_of_bbef1c8e_now=dict(Counter(h[hb == 4].tolist())),
                             lattice_at_015d839e_now_off=int(((h0 == 0) & (h > 0)).sum()),
                             height6_of_015d839e_now=dict(Counter(h[h0 == 6].tolist())))), flush=True)
