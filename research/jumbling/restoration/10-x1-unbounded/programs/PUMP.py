# Exact check of the two-grip isolated pump (claims P1-P5 of REPORT.md). Run from the repository root, with S = research/jumbling/restoration/10-x1-unbounded:
#   python3 -I -B $S/programs/PUMP.py [witness, default $S/witnesses/H1230.json] [n0, default 156]
# Uses only the repository simulator and its exact Evaluator; no floating point decides anything.
#  P1  The first n0 letters are admissible (applied and not uncertain); call the state X.
#  P2  At X: grips 11 and 80 hold every piece of K whole (exact), cuts 11 and 80 are clean (no straddler,
#      exact classify), 80 is not in NB(11), branches differ, and no piece other than K lies whole in both caps.
#  P3  Good_x (x = 11, 80, y the other grip) = items g at x such that every piece of D_x (pieces other than K
#      whole in cap x at X), at its pose turned by g, is 'out' of cap y; decided by the exact Evaluator for
#      every piece and every item.
#  P4  Every later letter of the witness is admissible, alternates between 11 and 80, is a jumble, keeps the
#      running product g_x of the letters at x since X in Good_x, keeps K whole in both caps, and raises K's
#      left height by exactly 2.
#  P5  At checkpoints, every piece outside K has the pose predicted by the isolation lemma: pieces of D_x at
#      E(x, g_x) times their pose at X, all other pieces at their pose at X.
import sys, json, time, hashlib
from pathlib import Path
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import numpy as np
import sim, restore as R, trees as T
from exact import matmul, matvec
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form

wpath = sys.argv[1] if len(sys.argv) > 1 else 'research/jumbling/restoration/10-x1-unbounded/witnesses/H1230.json'
n0 = int(sys.argv[2]) if len(sys.argv) > 2 else 156
t0 = time.time()
wit = json.loads(Path(wpath).read_text()); WORD = [tuple(x) for x in wit['word']]
K = tuple(wit['claims']['pieces'])
C = sim.get_context(); D = C.data; RG = C.regions; KP = C.kplus
M = R.menu('I_a'); s = sim.State(C, filtered=False, menu=M)
I4 = KP.matrix(0)
JUMBLE = set(R.jumble_items(M))
EL = {}
def E(g, j):
    if (g, j) not in EL: EL[(g, j)] = R.element(M, g, j)
    return EL[(g, j)]
NB = lambda a, b: (D.NN * (D.NN + sum((D.N[a][i] * D.N[b][i] for i in range(4)), D.NN * 0)) - 2 * D.KAPPA * D.KAPPA).sign() > 0
G = {}
def apply(c, j):
    o = s.apply(sim.Twist(C, c, E(c, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, ('letter not admissible', c, j)
    G[c] = to_tuple(matmul(E(c, j), G.get(c, I4)))
def kpose():
    Q = s.pose(K[0]); assert all(s.pose(p) == Q for p in K); return Q
def height(Q): return T.dist(T.factors(Q)[0])
VK = sorted({tuple(v) for p in K for v in RG.vertices(p)}, key=repr)
def k_in(Q, c):
    vf = rows_form({tuple(matvec(Q, v)) for v in VK})
    return Evaluator(pullback_form(I4, D.N[c])).classify(vf)[0] == 'in'

# P1
for c, j in WORD[:n0]: apply(c, j)
Q0 = kpose(); h0 = height(Q0)
print('P1: %d letters admissible; K at height %d; %.0fs' % (n0, h0, time.time() - t0))
# P2
c1, c2 = 11, 80
assert k_in(Q0, c1) and k_in(Q0, c2), 'K not whole in both caps'
cl1 = s.classify(c1, need_inside=True, stop_at_straddle=False); cl2 = s.classify(c2, need_inside=True, stop_at_straddle=False)
assert cl1.straddle is None and not cl1.uncertain and cl2.straddle is None and not cl2.uncertain, 'a cut is not clean'
assert not NB(c1, c2), '80 in NB(11)'
assert T.branch(tuple(D.N[c1])) != T.branch(tuple(D.N[c2]))
Dx = {c1: sorted(set(cl1.inside.tolist()) - set(K)), c2: sorted(set(cl2.inside.tolist()) - set(K))}
assert set(K) <= set(cl1.inside.tolist()) and set(K) <= set(cl2.inside.tolist())
assert not (set(Dx[c1]) & set(Dx[c2])), 'another piece lies whole in both caps'
print('P2: K whole in caps 11 and 80, both cuts clean, not NB neighbours, branches %d and %d; |D_11| = %d, |D_80| = %d; no other piece in both caps'
      % (T.branch(tuple(D.N[c1])), T.branch(tuple(D.N[c2])), len(Dx[c1]), len(Dx[c2])))
# poses at X
P0 = {p: s._pose_matrix(int(s.pose_id[p])) for x in (c1, c2) for p in Dx[x]}
key0 = s.pose_id.copy(); poses0 = list(s._poses)
# P3
def good_set(x, y):
    by = {}
    for p in Dx[x]: by.setdefault(int(s.pose_id[p]), []).append(p)
    good = []
    for j in range(60):
        ok = True
        for pid, ps in by.items():
            ev = Evaluator(pullback_form(to_tuple(matmul(E(x, j), s._pose_matrix(pid))), D.N[y]))
            for p in ps:
                if ev.classify(RG.vform(p))[0] != 'out': ok = False; break
            if not ok: break
        if ok: good.append(j)
    return good
Good = {c1: good_set(c1, c2), c2: good_set(c2, c1)}
print('P3: Good_11 = %s (%d items, %d jumbles)' % (Good[c1], len(Good[c1]), sum(j in JUMBLE for j in Good[c1])))
print('    Good_80 = %s (%d items, %d jumbles); %.0fs' % (Good[c2], len(Good[c2]), sum(j in JUMBLE for j in Good[c2]), time.time() - t0))
IDX = {x: {to_tuple(E(x, j)): j for j in range(60)} for x in (c1, c2)}
gl = {c1: to_tuple(I4), c2: to_tuple(I4)}
# P4 and P5
def check_kind(tag):
    bad = 0
    for x in (c1, c2):
        for p in Dx[x]:
            if s.pose(p) != to_tuple(matmul(gl[x], P0[p])): bad += 1
    others = np.ones(len(s.pose_id), bool); others[list(K)] = False
    for x in (c1, c2): others[Dx[x]] = False
    idx = np.flatnonzero(others)
    cur = {pid: s._poses[pid].key for pid in set(s.pose_id[idx].tolist())}
    old = {pid: poses0[pid].key for pid in set(key0[idx].tolist())}
    mism = sum(1 for p in idx.tolist() if cur[int(s.pose_id[p])] != old[int(key0[p])])
    assert bad == 0 and mism == 0, ('kind check failed', tag, bad, mism)
    return len(idx)
nother = check_kind('X')
print('P5 at X: kind holds trivially; %d other pieces' % nother)
last = WORD[n0 - 1][0]; h = h0; Q = Q0
nout = lambda: sum(1 for g in G if KP.index_of_matrix(G[g]) < 0)
for n, (c, j) in enumerate(WORD[n0:], start=n0 + 1):
    assert c in (c1, c2) and c != last, ('grips do not alternate', n)
    assert j in JUMBLE, ('not a jumble', n)
    gnew = to_tuple(matmul(E(c, j), gl[c]))
    assert IDX[c][gnew] in Good[c], ('running product leaves Good', n)
    assert k_in(Q, c), ('K not whole in the cap of the moving grip', n)
    apply(c, j); gl[c] = gnew; last = c
    Q = kpose(); h2 = height(Q)
    assert h2 == h + 2, ('height did not rise by 2', n, h, h2)
    assert k_in(Q, c1) and k_in(Q, c2), ('K left the lens', n)
    h = h2
    if (n - n0) % 100 == 0 or n == len(WORD):
        check_kind(n)
        print('P4/P5 after letter %d: K at height %d, labels (%d, %d), %d out grips; kind holds; %.0fs'
              % (n, h, IDX[c1][gl[c1]], IDX[c2][gl[c2]], nout(), time.time() - t0), flush=True)
print('PASS: %d pump letters, each a jumble at alternating grips 11/80 raising K by 2 (height %d -> %d); running products in Good; '
      'kind S(h) verified at every 100th letter; digest %s' % (len(WORD) - n0, h0, h, s.digest()))
