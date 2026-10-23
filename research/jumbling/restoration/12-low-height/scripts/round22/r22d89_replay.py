# Theory round 22, line (d), round 6: the integrator's letter-by-letter replay of a literal word from a snapshot written
# by r22d31_snapshots.py (R22D-89). Arguments: the snapshot, a prefix and a word (JSON lists of [grip, item]) and a
# target cap tau. It plays the prefix (each letter admissible and certain), takes the pieces at height 6 after it as H6,
# and prints START (digest, u = the number of straddlers of tau by eb6.straddler_set, and n6), then one LETTER line per
# letter of the word: admissible and certain, the largest left height, whether n6 and the exact pose of every piece of
# H6 are kept, how many pieces of H6 are now at height 4, u, (n2, n4, n6), off, N and the digest. It stops with an
# assertion at the first letter that is not admissible and certain. It writes no file; theory/seams.py keeps its cache
# in work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file. Research only, headless Linux; exact geometry.
# Outputs: runs/round22/r22d89_cap290_route.txt and r22d89_cap311_stop_route.txt, the cap 290 route from 9e6b991b and
# the cap 311 route from the stop of the cloud line unlock-cap311-cap290. Run from the repository root (each a few CPU
# minutes after the snapshot loads):
#   OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d89_replay.py \
#     <dir>/s7302-stop.pkl '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12],[136,2],[136,6]]' \
#     '[[301,2],[136,7],[301,4],[136,1],[136,4],[136,19],[301,3],[136,1],[301,3],[136,5],[290,15]]' 290
#   OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d89_replay.py \
#     <dir>/s7302-stop.pkl '[]' '[[185,1],[185,7],[185,16],[185,3],[263,1],[263,3],[263,8],[364,1],[263,2],[407,1],[407,3],[407,1],[407,2],
#     [407,4],[364,1],[407,2],[407,4],[407,10],[407,15],[364,2],[263,2],[311,14]]' 311
import sys, json
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import numpy as np
import eb6 as E
from r22d31_snapshots import load
snap, prefix, word, tau = sys.argv[1], json.loads(sys.argv[2]), json.loads(sys.argv[3]), int(sys.argv[4])
C = E.sim.get_context(); M = E.R.menu('I_a')
s, _ = load(snap, E)
cache = {}
def h_of(p):
    if p.kidx >= 0: return 0
    if p.key not in cache:
        l, r = E.T.factors(p.matrix); assert E.T.dist(r) == 0; cache[p.key] = E.T.dist(l)
    return cache[p.key]
def heights(st): return np.array([h_of(p) for p in st._poses])[st.pose_id]
def keys(st, ps): return [st._poses[int(st.pose_id[p])].key for p in ps]
def put(st, c, j):
    o = st.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
    return o.applied and not o.uncertain
for c, j in prefix: assert put(s, c, j)
h = heights(s); six = np.flatnonzero(h == 6).tolist(); k0 = keys(s, six)
print('START', json.dumps({'digest': s.digest(), 'u': len(E.straddler_set(s, tau)), 'n6': len(six)}), flush=True)
for i, (c, j) in enumerate(word, 1):
    ok = put(s, c, j); h = heights(s)
    n = [int((h == v).sum()) for v in (2, 4, 6)]
    kept = int((h == 6).sum()) == len(six) and keys(s, six) == k0
    low = sum(1 for p, kk in zip(six, keys(s, six)) if h[p] == 4)
    print('LETTER', json.dumps({'i': i, 'letter': [c, j], 'admissible_certain': ok, 'max_height': int(h.max()),
          'h6_kept': kept, 'start_h6_now_at_4': low, 'u': len(E.straddler_set(s, tau)), 'n2_n4_n6': n,
          'off': int(s.off_lattice_count()), 'N': E.R.Restorer(s, M).misalignment(), 'digest': s.digest()}), flush=True)
    assert ok
