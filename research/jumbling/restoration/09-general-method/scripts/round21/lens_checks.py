"""Exact checks of round 21 (SENIOR-REVIEW.md section 26) at the states X and sigma of stage 10, from the solved state.
X is the state after the first 156 letters of the stage 10 witness H6030, sigma after the first 2,085; K is the island
of seven pieces that the family raises. Every letter is applied by the exact simulator and must be admissible and
certain. The checks:
- R21-06: from X, (11, 15), (80, 31) lowers K from 30 to 28 to 26, K stays whole in caps 11 and 80, both cuts stay clean;
- E21-4 and X21A-08: at sigma, the landing (11, 15) puts exactly the debris D_11 of cap 11 (its 2,505 pieces other
  than K) on the lattice, takes no lattice piece off and raises K from 3,888 (branch 2) to 3,890 (branch 3), so it is
  not tame; the landing (80, 14) puts exactly D_80 (2,702 pieces) on the lattice, takes none off and keeps K at 3,888;
  undoing either gives sigma back;
- R21F-08: (11, 57), (80, 14) puts exactly D_11 and D_80 on the lattice and takes no lattice piece off: the off count
  falls from 52,883 to 47,676 (the state Z');
- X21A-11: at Z', the two jumbles (11, 40), (0, 15) leave both cuts clean, and the common content of caps 11 and 0 is
  exactly K, at height 3,892.
Prints one line per check and exits 0 when all pass. Run from the repository root:
    python3 -I -B research/jumbling/restoration/09-general-method/scripts/round21/lens_checks.py
Research only: exact synthetic geometry; no Windows, DirectX, input or performance claim.
"""
import json
import sys
import time

import numpy as np

sys.path[:0] = ['research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import sim
import restore as R
import trees as T

t0 = time.time()
W = [tuple(x) for x in json.load(open('research/jumbling/restoration/10-x1-unbounded/witnesses/H6030.json'))['word']]
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
DIGEST = {156: '052d47cd58a668e18b7dc121d9ef642a408969db2c03f30707a12707bf99e3db',
          2085: 'f067f0150922efe3f33e25903ac4275342e7a51cdacb2cd5f6be685aa76d3226'}
C = sim.get_context()
M = R.menu('I_a')


def apply(s, c, j, journal=False):
    o = s.apply(sim.Twist(C, c, R.element(M, c, j), 'menu', {'menu': M.name}), journal=journal)
    assert o.applied and not o.uncertain, ('letter not admissible', c, j)


def build(n):
    s = sim.State(C, filtered=False, menu=M)
    for c, j in W[:n]:
        apply(s, c, j)
    assert s.digest() == DIGEST[n], ('digest', n, s.digest())
    return s


def k_height(s):
    """Left height and branch of K; its seven pieces share one pose."""
    Q = s.pose(K[0])
    assert all(s.pose(p) == Q for p in K)
    left = T.factors(Q)[0]
    return T.dist(left), T.branch(left)


def cap(s, c):
    cl = s.classify(c, need_inside=True, stop_at_straddle=False)
    assert not cl.uncertain
    return cl.straddle is None, set(cl.inside.tolist())


def lattice(s):
    """Boolean array over the pieces: True where the piece's pose is a lattice pose."""
    return s.lattice_flags().copy()


def moved(before, after):
    """(pieces put on the lattice, pieces taken off it) between two lattice arrays."""
    return set(np.flatnonzero(after & ~before).tolist()), set(np.flatnonzero(before & ~after).tolist())


def check(name, ok, detail):
    print(json.dumps({'check': name, 'pass': bool(ok), **detail}), flush=True)
    return bool(ok)


results = []
s = build(156)
for c, j in ((11, 15), (80, 31)):
    apply(s, c, j)
    h = k_height(s)[0]
    (cl11, in11), (cl80, in80) = cap(s, 11), cap(s, 80)
    results.append(check('R21-06 %s' % ((c, j),), h == (28 if c == 11 else 26) and set(K) <= in11 & in80 and cl11 and cl80,
                         {'height': h, 'K_in_both_caps': set(K) <= in11 & in80, 'cuts_clean': cl11 and cl80,
                          'off': int(s.off_lattice_count())}))

s = build(2085)
off0 = int(s.off_lattice_count())
lat0 = lattice(s)
(cl11, in11), (cl80, in80) = cap(s, 11), cap(s, 80)
D = {11: in11 - set(K), 80: in80 - set(K)}
results.append(check('sigma', off0 == 52883 and k_height(s) == (3888, 2) and cl11 and cl80 and set(K) <= in11 & in80
                     and len(D[11]) == 2505 and len(D[80]) == 2702 and not lat0[sorted(in11 | in80)].any(),
                     {'off': off0, 'K': k_height(s), 'D_11': len(D[11]), 'D_80': len(D[80]),
                      'caps_hold_no_lattice_piece': not lat0[sorted(in11 | in80)].any()}))
for c, j, want, off_want in ((11, 15, (3890, 3), 50378), (80, 14, (3888, 2), 50181)):
    apply(s, c, j, journal=True)
    landed, taken_off = moved(lat0, lattice(s))
    off1 = int(s.off_lattice_count())
    results.append(check('E21-4 landing %s' % ((c, j),),
                         k_height(s) == want and landed == D[c] and not taken_off and off1 == off_want,
                         {'K': k_height(s), 'off': off1, 'landed': len(landed), 'landed_is_debris': landed == D[c],
                          'taken_off': len(taken_off)}))
    s.undo()
    results.append(check('E21-4 undo %s' % ((c, j),), s.digest() == DIGEST[2085], {'digest_is_sigma': s.digest() == DIGEST[2085]}))
for c, j in ((11, 57), (80, 14)):
    apply(s, c, j)
landed, taken_off = moved(lat0, lattice(s))
results.append(check("R21F-08 Z'", int(s.off_lattice_count()) == 47676 and k_height(s) == (3890, 3)
                     and landed == D[11] | D[80] and not taken_off,
                     {'off': int(s.off_lattice_count()), 'K': k_height(s), 'landed': len(landed),
                      'landed_is_both_debris': landed == D[11] | D[80], 'taken_off': len(taken_off)}))
for c, j in ((11, 40), (0, 15)):
    apply(s, c, j)
(cl11, in11), (cl0, in0) = cap(s, 11), cap(s, 0)
results.append(check('X21A-11 lens creation', cl11 and cl0 and in11 & in0 == set(K) and k_height(s)[0] == 3892,
                     {'cuts_clean': cl11 and cl0, 'common_content_is_K': in11 & in0 == set(K), 'K': k_height(s),
                      'off': int(s.off_lattice_count())}))
print(json.dumps({'all_pass': all(results), 'seconds': round(time.time() - t0)}))
sys.exit(0 if all(results) else 1)
