# Theory round 22, line (d), round 5 (R22D-75): the integrator's independent check of the seven letters that the
# height-first rule plays from the 3000-twist stop of seed 7302 (state 015d839e...): replay them, compare the exact
# poses of the 185 height-6 pieces of the start with their poses at the end, and list cap 274's straddlers at the end.
# The integrator's script; output in runs/round22/r22d75_check.txt. Under a CPU minute.
# Build the snapshot first (about 30 CPU minutes): python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d31_snapshots.py <dir>
# It writes no file; theory/seams.py keeps its cache in work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d75_check.py <dir>/s7302-stop.pkl
import sys
import numpy as np
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
from collections import Counter
C = E.sim.get_context(); M = E.R.menu('I_a')
s, _ = load(sys.argv[1], E)
assert s.digest().startswith('015d839e')
def hts(st):
    info = {i: E.T.pose_tree(st._pose_matrix(i)) for i in np.unique(st.pose_id).tolist()}
    return np.array([info[int(i)][0] for i in st.pose_id])
h0 = hts(s)
six = np.flatnonzero(h0 == 6)
pose0 = {int(p): s._pose_matrix(int(s.pose_id[p])) for p in six}
word = [(64, 15), (185, 14), (357, 12), (471, 12), (263, 14), (407, 13), (523, 12)]
for c, j in word:
    o = s.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain
h1 = hts(s)
same = [int(p) for p in six if s._pose_matrix(int(s.pose_id[p])) == pose0[int(p)]]
moved = sorted(set(six.tolist()) - set(same))
print('END', s.digest(), 'heights', dict(Counter(h1.tolist())))
print('HEIGHT6_START', len(six), 'SAME_POSE', len(same), 'MOVED', len(moved), 'MOVED_HEIGHTS', dict(Counter(h1[moved].tolist())))
print('MOVED_PIECES', moved)
print('CAP274_STRADDLERS', E.straddler_set(s, 274).tolist())
