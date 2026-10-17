# Integrator's independent check of an X1-run witness, written from the repository API only (not from
# programs/replay.py). Run from the repository root, with S = research/jumbling/restoration/10-x1-unbounded:
#   python3 -I -B $S/check/replay_witness.py <witness.json>
import sys, json, hashlib, ast
from pathlib import Path
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import sim, restore as R, trees as T
from exact import matmul
from sim.kplus import to_tuple

wit = json.loads(Path(sys.argv[1]).read_text())
assert wit['from'] == 'solved'
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
assert tuple(wit['claims']['pieces']) == K
C = sim.get_context(); M = R.menu('I_a'); s = sim.State(C, filtered=False, menu=M)
KP = C.kplus; I = KP.matrix(0); G = {}
# every letter is taken as written: two JSON integers, grip and item in range; nothing is converted
W = []
for n, x in enumerate(wit['word'], 1):
    assert type(x) is list and len(x) == 2 and all(type(v) is int for v in x), ('letter is not two integers', n, x)
    assert 0 <= x[0] < 600 and 0 <= x[1] < len(M.items), ('letter out of range', n, x)
    W.append(tuple(x))

# prefix shared with E17-4 (read from the repository and digest-checked as in T20-03)
src = Path('research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md').read_text()
code = src.split('E17-4, checked letter by letter: the word of 147 letters from the solved state.')[1].split('```python\n')[1].split('```')[0]
assert hashlib.sha256(code.encode()).hexdigest() == '897b98b4306a832f2f07e4cf570316bbe15c8cdf41dbd9bd3c408f80238ad911'
E = next(ast.literal_eval(n.value) for n in ast.parse(code).body if isinstance(n, ast.Assign)
         and any(isinstance(t, ast.Name) and t.id == 'W' for t in n.targets))
pre = next((i for i in range(min(len(E), len(W))) if tuple(E[i]) != W[i]), min(len(E), len(W)))
print('letters:', len(W), '(each two integers, grip and item in range); common prefix with E17-4:', pre)

def kstate():
    Q = s.pose(K[0])
    assert all(s.pose(p) == Q for p in K), 'K split'
    dl, dr, bl, br = T.pose_tree(Q)
    return dl, dr, bl

trace = []
for n, (c, j) in enumerate(W):
    assert 0 <= c < 600
    A = R.element(M, c, j)
    o = s.apply(sim.Twist(C, c, A, 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, ('not admissible', n + 1, c, j)
    G[c] = to_tuple(matmul(A, G.get(c, I)))
    dl, dr, bl = kstate()
    assert dr == 0, ('right height not 0', n + 1)
    trace.append(dl)
    if n + 1 > pre - 2:
        out = sum(1 for g in G if KP.index_of_matrix(G[g]) < 0)
        print('%3d (%3d,%2d) K left height %2d branch %d, out grips %d' % (n + 1, c, j, dl, bl, out))
print('max height along the word:', max(trace), 'final:', trace[-1])
print('digest', s.digest())
cl = wit['claims']
assert trace[-1] == cl['final_left_height_of_pieces']
if 'final_digest' in cl:
    assert s.digest() == cl['final_digest']; print('digest matches the claim')
print('VERIFIED', Path(sys.argv[1]).name)
