# H20: an admissible word of 150 letters from the solved state that takes the seven pieces K to left height 20.
# Run from the repository root, with S = research/jumbling/restoration/10-x1-unbounded:  python3 -I -B $S/programs/H20.py
# Uses only the repository simulator. The first 141 letters are those of the word of E17-4, read from
# SENIOR-REVIEW.md and checked by the same SHA-256 digest as programs/T20-03.py. Every letter must be applied
# by the exact simulator with o.applied and not o.uncertain. Heights are T.dist of the left factor.
import sys, ast, hashlib, json
from pathlib import Path
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import sim, restore as R, trees as T
from exact import matmul
from sim.kplus import to_tuple
from collections import Counter

src = Path('research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md').read_text()
code = src.split('E17-4, checked letter by letter: the word of 147 letters from the solved state.')[1].split('```python\n')[1].split('```')[0]
assert hashlib.sha256(code.encode()).hexdigest() == '897b98b4306a832f2f07e4cf570316bbe15c8cdf41dbd9bd3c408f80238ad911'
W = next(ast.literal_eval(n.value) for n in ast.parse(code).body if isinstance(n, ast.Assign)
         and any(isinstance(t, ast.Name) and t.id == 'W' for t in n.targets))
assert len(W) == 147
TAIL = [(592, 22), (593, 30), (582, 30), (576, 51), (525, 21), (410, 15), (393, 20), (410, 14), (361, 12)]
WORD = [tuple(x) for x in W[:141]] + TAIL
wit = Path('research/jumbling/restoration/10-x1-unbounded/witnesses/H20.json')
if wit.exists():
    assert [tuple(x) for x in json.loads(wit.read_text())['word']] == WORD, 'witness file differs'

C = sim.get_context(); M = R.menu('I_a'); s = sim.State(C, filtered=False, menu=M)
KP = C.kplus; I = KP.matrix(0)
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
G = {}

def kh():
    Q = s.pose(K[0]); assert all(s.pose(p) == Q for p in K), 'K split'
    l = T.factors(Q)[0]
    return T.dist(l), T.branch(l)

for n, (c, j) in enumerate(WORD):
    A = R.element(M, c, j)
    o = s.apply(sim.Twist(C, c, A, 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, ('letter not admissible', n, c, j)
    G[c] = to_tuple(matmul(A, G.get(c, I)))
    if n >= 140:
        out = sorted(g for g in G if KP.index_of_matrix(G[g]) < 0)
        print('letter %3d (%3d, %2d): K height %2d branch %d; out grips %d' % (n + 1, c, j, *kh(), len(out)))

h, b = kh()
assert h == 20
hist = Counter()
for i, P in enumerate(s._poses):
    pass
hp = {}
for pid in set(int(x) for x in s.pose_id):
    hp[pid] = T.dist(T.factors(s._pose_matrix(pid))[0])
for x in s.pose_id:
    hist[hp[int(x)]] += 1
print('final heights (height: pieces):', dict(sorted(hist.items())))
top = max(hist)
print('pieces at the top height', top, ':', sorted(int(p) for p in range(len(s.pose_id)) if hp[int(s.pose_id[p])] == top))
print('out grips:', sorted(g for g in G if KP.index_of_matrix(G[g]) < 0))
print('digest', s.digest())
print('PASS: %d letters admissible (exact); K at left height %d in branch %d' % (len(WORD), h, b))
