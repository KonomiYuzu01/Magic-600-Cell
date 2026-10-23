# Theory round 22, line (d), round 6: the integrator's replay of R22D-85. The two letters (124, 2), (274, 13), from
# the 3000-twist stop of seed 7302 (state 015d839e...) and from the state after R22D-75's seven letters (1ff0061f...).
# For each start: the first letter must be admissible and certain, keep n6 and every height-6 pose and leave cap 274
# with no straddler; the second must be admissible and certain and leave no piece above height 6. Prints after each
# letter the counts, the off count, N and the digest, then where the start's height-6 pieces are and how many pieces
# that were at the lattice at the start are off at the end.
# The integrator's script; output in runs/round22/r22d85_two_letters.txt. Under three CPU minutes.
# Research only, headless Linux; exact geometry. It writes no file; theory/seams.py keeps its cache in
# work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d85_two_letters.py <dir>/s7302-stop.pkl
import sys, json, resource
from collections import Counter
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU, (1190, 1200))
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load

C = E.sim.get_context(); M = E.R.menu('I_a'); s0, _ = load(sys.argv[1], E)
SEVEN = [(64, 15), (185, 14), (357, 12), (471, 12), (263, 14), (407, 13), (523, 12)]
cache = {}


def clone(st):
    t = E.sim.State(C, filtered=False, menu=M)
    t.pose_id = st.pose_id.copy(); t._poses = list(st._poses); t._lookup = dict(st._lookup)
    return t


def put(st, c, j):
    o = st.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain


def heights(st):
    for p in st._poses:
        if p.key not in cache:
            if p.kidx >= 0:
                cache[p.key] = 0
            else:
                l, r = E.T.factors(p.matrix); assert E.T.dist(r) == 0; cache[p.key] = E.T.dist(l)
    return np.array([cache[p.key] for p in st._poses])[st.pose_id]


def keys(st, ps):
    return [st._poses[int(st.pose_id[p])].key for p in ps]


def report(st, tag, **kw):
    h = heights(st); assert h.max() <= 6
    print(tag, json.dumps(dict(kw, n2_n4_n6=[int((h == k).sum()) for k in (2, 4, 6)], off=int((h > 0).sum()),
                               N=E.R.Restorer(st, M).misalignment(), digest=st.digest())), flush=True)


for name, prefix, start in [('stop', [], '015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f'),
                            ('after R22D-75', SEVEN, '1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81')]:
    s = clone(s0)
    for c, j in prefix:
        put(s, c, j)
    assert s.digest() == start
    h0 = heights(s); six = np.flatnonzero(h0 == 6); sixkeys = keys(s, six)
    report(s, 'START', start=name, u274=len(E.straddler_set(s, 274)))
    put(s, 124, 2)
    h1 = heights(s)
    u = len(E.straddler_set(s, 274))
    assert keys(s, six) == sixkeys and int((h1 == 6).sum()) == len(six) and u == 0
    report(s, 'STEP', start=name, letter=[124, 2], u274=u, height6_poses_kept=True)
    put(s, 274, 13)
    report(s, 'STEP', start=name, letter=[274, 13])
    h2 = heights(s)
    print('END', json.dumps(dict(start=name, start_height6=len(six), their_heights=dict(sorted(Counter(h2[six].tolist()).items())),
                                 lattice_pieces_off=int(((h0 == 0) & (h2 > 0)).sum()))), flush=True)
