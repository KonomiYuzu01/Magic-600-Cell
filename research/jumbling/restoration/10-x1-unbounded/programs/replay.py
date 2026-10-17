# Generic exact witness checker. Run from the repository root, with S = research/jumbling/restoration/10-x1-unbounded:  python3 -I -B $S/programs/replay.py <witness.json> [from]
# Uses only the repository simulator. Replays the word from the solved state; every letter must be applied
# with o.applied and not o.uncertain. Tracks the product of the letters at every grip (out = not in K+).
# Reports the left height and branch of the pieces of claims["pieces"] after every letter from position
# <from> on (default: last 12), checks that they share one pose, and checks claims["final_left_height_of_pieces"]
# and, if present, claims["final_digest"]. Prints PASS or raises AssertionError.
import sys, json
from pathlib import Path
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import sim, restore as R, trees as T
from exact import matmul
from sim.kplus import to_tuple
from collections import Counter

wit = json.loads(Path(sys.argv[1]).read_text())
assert wit['from'] == 'solved'
WORD = [tuple(x) for x in wit['word']]
cl = wit['claims']
K = tuple(cl['pieces'])
start = int(sys.argv[2]) if len(sys.argv) > 2 else max(0, len(WORD) - 12)

C = sim.get_context(); M = R.menu('I_a'); s = sim.State(C, filtered=False, menu=M)
KP = C.kplus; I = KP.matrix(0)
G = {}

def kh():
    Q = s.pose(K[0]); assert all(s.pose(p) == Q for p in K), 'pieces do not share one pose'
    l = T.factors(Q)[0]
    return T.dist(l), T.branch(l)

hs = []
for n, (c, j) in enumerate(WORD):
    A = R.element(M, c, j)
    o = s.apply(sim.Twist(C, c, A, 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, ('letter not admissible', n + 1, c, j)
    G[c] = to_tuple(matmul(A, G.get(c, I)))
    if n + 1 >= start:
        h, b = kh(); hs.append(h)
        nout = sum(1 for g in G if KP.index_of_matrix(G[g]) < 0)
        print('letter %3d (%3d, %2d): height %2d branch %d; out grips %d' % (n + 1, c, j, h, b, nout))

h, b = kh()
hp = {pid: T.dist(T.factors(s._pose_matrix(pid))[0]) for pid in set(int(x) for x in s.pose_id)}
hist = Counter(hp[int(x)] for x in s.pose_id)
print('final heights (height: pieces):', dict(sorted(hist.items())))
top = max(hist)
print('pieces at the top height', top, ':', sorted(p for p in range(len(s.pose_id)) if hp[int(s.pose_id[p])] == top))
print('out grips:', sorted(g for g in G if KP.index_of_matrix(G[g]) < 0))
dg = s.digest(); print('digest', dg)
assert h == cl['final_left_height_of_pieces'], (h, cl['final_left_height_of_pieces'])
if 'final_digest' in cl: assert dg == cl['final_digest']
print('PASS: %d letters admissible (exact); pieces %s at left height %d in branch %d'
      % (len(WORD), 'K' if len(K) == 7 else K, h, b))
