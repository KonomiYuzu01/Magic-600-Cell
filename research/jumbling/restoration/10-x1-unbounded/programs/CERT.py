# Exact check of the ball certificate of the Theorem of REPORT.md (sections 1 and 2). Run from the repository root, with S = research/jumbling/restoration/10-x1-unbounded:
#   python3 -I -B $S/programs/CERT.py [certificate, default $S/witnesses/CERT.json]
# Exact arithmetic only (repository simulator, its exact Evaluator, Q(sqrt5) numbers, Python fractions);
# no floating point decides anything.
#  A1  The first n0 letters of the witness are admissible (simulator); state X. At X (as in PUMP.py P2):
#      K whole in caps c1 = 11 and c2 = 80, both cuts clean, c2 not in NB(c1), grip normals at tree distance 1
#      in branches 3 and 2, K in branch 3, no other piece whole in both caps. Good sets computed by the exact
#      Evaluator for every piece of D_x and every item; they contain the identity item.
#  A1b The same Good sets, and property (i) of the isolation lemma, with the simulator's own classify (which
#      uses signature certificates for pieces at lattice poses): for every item g at x, with D_x turned by g and
#      every other piece at its X-pose, g is good iff cut y has no straddler and no piece of D_x lies in cap y;
#      for good g, cut x has no straddler and D_x lies in cap x.
#  A2  The next `centre` letters of the witness satisfy the hypotheses of the step (alternation, jumble,
#      running product in Good, K whole in both caps after every letter, by exact vertex margins > 0); the
#      state reached is the centre: K at pose Q*, labels (g_c*, g_d*), next grip x*, as claimed in the
#      certificate. The same letters are replayed with the simulator, which reaches K at exactly Q*.
#  B   For every certificate word W_i, started at the centre: alternation from x*, jumbles, running products in
#      Good, and at every state along W_i (the start included), for every vertex v of K and both caps n, with
#      exact margin m = n.(Q v) - kappa:
#          m - (m + kappa) eta^2 / 2 > eta * sqrt(|n|^2 |v|^2 - (m + kappa)^2)          (*)
#      checked exactly in squared form. At the end the labels equal (g_c*, g_d*), the length is even, and the
#      product P_i of the word's matrices (P_i = Q_end Q*^T) equals the left multiplication L(lambda_i).
#      By (*), W_i satisfies the step hypotheses from every pose L(u) Q* with |u - 1| <= eta (REPORT.md section 2.5).
#  C   The dyadic boxes cover the u_vec-ball of {u in S^3 : |u - 1| <= eta}; on each box b the assigned word
#      satisfies lambda_0 s_b - max_b(lambda_vec . u_vec) >= 1 - eta^2/2, with s_b a rational lower bound of
#      sqrt(1 - min(r2max(b), R^2)), R^2 = eta^2 - eta^4/4; hence |lambda u - 1| <= eta on b (the letters are
#      orthogonal and |lambda_i| = 1, both checked exactly).
#  D   Every certificate word is replayed with the simulator from the centre (every letter applied and not
#      uncertain), and K ends at exactly L(lambda_i) Q*.
import sys, json, time
from math import isqrt
from fractions import Fraction as Fr
from pathlib import Path
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import sim, restore as R, trees as T
from exact import matmul, transpose, Q5
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form

cpath = sys.argv[1] if len(sys.argv) > 1 else 'research/jumbling/restoration/10-x1-unbounded/witnesses/CERT.json'
t0 = time.time()
cert = json.loads(Path(cpath).read_text())
wit = json.loads(Path(cert['witness']).read_text()); WORD = [tuple(x) for x in wit['word']]
K = tuple(cert['claims']['pieces']); n0 = cert['n0']; CI = cert['centre']; c1, c2 = cert['grips']
pe, qe = cert['eta']; ETA = Fr(pe, qe)
C = sim.get_context(); D = C.data; RG = C.regions; KP = C.kplus
M = R.menu('I_a'); s = sim.State(C, filtered=False, menu=M)
I4 = KP.matrix(0)
JUMBLE = set(R.jumble_items(M))
EL = {}
def E(g, j):
    if (g, j) not in EL: EL[(g, j)] = R.element(M, g, j)
    return EL[(g, j)]
NB = lambda a, b: (D.NN * (D.NN + sum((D.N[a][i] * D.N[b][i] for i in range(4)), D.NN * 0)) - 2 * D.KAPPA * D.KAPPA).sign() > 0
def apply(c, j):
    o = s.apply(sim.Twist(C, c, E(c, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, ('letter not admissible', c, j)
def kpose():
    Q = s.pose(K[0]); assert all(s.pose(p) == Q for p in K); return Q
def height(Q): return T.dist(T.factors(Q)[0])
VK = sorted({tuple(v) for p in K for v in RG.vertices(p)}, key=repr)
# the simulator classifies piece p by the rows of RG.vform(p); these are the points RG.vertices(p)
for p in K:
    e, rows = RG.vform(p)
    pts = {tuple((q.a, q.b, q.d) for q in (Q5(r[i], r[4 + i], e) for i in range(4))) for r in rows}
    assert pts == {tuple((q.a, q.b, q.d) for q in v) for v in RG.vertices(p)}, ('vform differs from vertices', p)
def k_in(Q, c):
    vf = rows_form({tuple((sum((Q[i][k] * v[k] for k in range(4)), Q5()) for i in range(4))) for v in VK})
    return Evaluator(pullback_form(I4, D.N[c])).classify(vf)[0] == 'in'
KAPPA = D.KAPPA
NN = {x: sum((D.N[x][i] * D.N[x][i] for i in range(4)), Q5()) for x in (c1, c2)}
VV = [sum((v[i] * v[i] for i in range(4)), Q5()) for v in VK]
def margins(Q, x):
    """exact n_x.(Q v) - kappa for every vertex v of K"""
    y = [sum((Q[i][j] * D.N[x][i] for i in range(4)), Q5()) for j in range(4)]       # Q^T n
    return [sum((y[j] * v[j] for j in range(4)), Q5()) - KAPPA for v in VK]
P2, Q2 = pe * pe, qe * qe
def star(m, x, k):
    """(*) for margin m of vertex k in cap x, squared form; True iff it holds"""
    G0 = m + KAPPA
    A = 2 * Q2 * m - P2 * G0
    if A.sign() <= 0: return False
    return (A * A - 4 * P2 * Q2 * (NN[x] * VV[k] - G0 * G0)).sign() > 0

# A1
for c, j in WORD[:n0]: apply(c, j)
QX = kpose(); hX = height(QX)
assert k_in(QX, c1) and k_in(QX, c2), 'K not whole in both caps at X'
cl1 = s.classify(c1, need_inside=True, stop_at_straddle=False); cl2 = s.classify(c2, need_inside=True, stop_at_straddle=False)
assert cl1.straddle is None and not cl1.uncertain and cl2.straddle is None and not cl2.uncertain, 'a cut is not clean'
assert not NB(c1, c2), 'c2 in NB(c1)'
assert T.dist(tuple(D.N[c1])) == 1 and T.dist(tuple(D.N[c2])) == 1
assert T.branch(tuple(D.N[c1])) == 3 and T.branch(tuple(D.N[c2])) == 2
assert T.branch(T.factors(QX)[0]) == 3, 'K not in branch 3 at X'
Dx = {c1: sorted(set(cl1.inside.tolist()) - set(K)), c2: sorted(set(cl2.inside.tolist()) - set(K))}
assert set(K) <= set(cl1.inside.tolist()) and set(K) <= set(cl2.inside.tolist())
assert not (set(Dx[c1]) & set(Dx[c2])), 'another piece lies whole in both caps'
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
Good = {c1: set(good_set(c1, c2)), c2: set(good_set(c2, c1))}
IDX = {x: {to_tuple(E(x, j)): j for j in range(60)} for x in (c1, c2)}
assert all(IDX[x][to_tuple(I4)] in Good[x] for x in (c1, c2)), 'identity not in Good'
other = {c1: c2, c2: c1}
def clone(st):
    u = sim.State(C, filtered=False, menu=M)
    u._poses = list(st._poses); u._lookup = dict(st._lookup); u.pose_id = st.pose_id.copy()
    return u
# A1b
sx = clone(s); base = sx.pose_id.copy(); nsig = 0
for x in (c1, c2):
    y = other[x]; good_sim = set()
    for j in range(60):
        sx.pose_id = base.copy(); cache = {}
        for p in Dx[x]:
            pid = int(base[p])
            if pid not in cache: cache[pid] = sx._intern_matrix(to_tuple(matmul(E(x, j), sx._pose_matrix(pid))))
            sx.pose_id[p] = cache[pid]
        nsig += int(sx.lattice_flags()[Dx[x]].sum())
        cy = sx.classify(y, need_inside=True, stop_at_straddle=False)
        assert not cy.uncertain
        if cy.straddle is None and not (set(cy.inside.tolist()) & set(Dx[x])):
            good_sim.add(j)
            cx = sx.classify(x, need_inside=True, stop_at_straddle=False)
            assert cx.straddle is None and not cx.uncertain and set(Dx[x]) <= set(cx.inside.tolist()), ('(i) fails', x, j)
    assert good_sim == Good[x], ('simulator Good differs', x)
print('A1: %d letters admissible; X: K at height %d whole in caps %d and %d, cuts clean, |D| = %d, %d; |Good| = %d, %d; '
      'K has %d vertices (= the simulator rows of its 7 pieces)'
      % (n0, hX, c1, c2, len(Dx[c1]), len(Dx[c2]), len(Good[c1]), len(Good[c2]), len(VK)))
print('    Good_%d = %s' % (c1, sorted(Good[c1])))
print('    Good_%d = %s' % (c2, sorted(Good[c2])))
print('A1b: Good sets and (i) agree with the simulator classify (%d piece-cases at lattice poses); %.0fs' % (nsig, time.time() - t0), flush=True)

def run(Q, gl, nxt, word, eta_check):
    """apply word from (Q, gl, next grip nxt) checking the step hypotheses with exact margins; with
    eta_check, (*) at every state (start included). Returns (Q, gl, nxt, number of margin checks)."""
    gl = dict(gl); nchk = 0
    def check_state(Q):
        nonlocal nchk
        for x in (c1, c2):
            ms = margins(Q, x)
            for k, m in enumerate(ms):
                assert m.sign() > 0, 'K not whole in a cap'
                if eta_check: assert star(m, x, k), ('(*) fails', x, k)
                nchk += 1
    check_state(Q)
    for n, (c, j) in enumerate(word):
        assert c == nxt, ('grips do not alternate', n)
        assert j in JUMBLE, ('not a jumble', n)
        gnew = to_tuple(matmul(E(c, j), gl[c]))
        assert IDX[c][gnew] in Good[c], ('running product leaves Good', n)
        Q = to_tuple(matmul(E(c, j), Q)); gl[c] = gnew; nxt = other[c]
        check_state(Q)
    return Q, gl, nxt, nchk

# A2
gl0 = {c1: to_tuple(I4), c2: to_tuple(I4)}
nxt0 = other[WORD[n0 - 1][0]]
Qs, gls, nxts, nchk = run(QX, gl0, nxt0, WORD[n0:n0 + CI], False)
hs = height(Qs)
assert hs == hX + 2 * CI
assert to_tuple(matmul(Qs, transpose(Qs))) == to_tuple(I4)
cc = cert['claims']
assert [IDX[c1][gls[c1]], IDX[c2][gls[c2]]] == cc['centre_labels'] and nxts == cc['centre_next_grip'] and hs == cc['centre_height']
for c, j in WORD[n0:n0 + CI]: apply(c, j)
assert kpose() == Qs, 'simulator pose at the centre differs from Q*'
print('A2: %d pump letters satisfy the step hypotheses (%d exact margins > 0); centre: K at height %d, labels (%d, %d), next grip %d; '
      'the simulator replays them and reaches K at Q*; %.0fs'
      % (CI, nchk, hs, IDX[c1][gls[c1]], IDX[c2][gls[c2]], nxts, time.time() - t0), flush=True)

# B
assert all(to_tuple(matmul(E(x, j), transpose(E(x, j)))) == to_tuple(I4) for x in (c1, c2) for j in range(60)), 'a letter is not orthogonal'
LAM = []
for i, w in enumerate(cert['words']):
    w = [tuple(x) for x in w]
    assert len(w) % 2 == 0
    Qe, gle, nxte, nchk = run(Qs, gls, nxts, w, True)
    assert gle == gls and nxte == nxts, 'labels do not return'
    P = matmul(Qe, transpose(Qs))
    lam = [P[i_][0] for i_ in range(4)]
    a, b, c_, d_ = lam
    Lm = [[a, -b, -c_, -d_], [b, a, -d_, c_], [c_, d_, a, -b], [d_, -c_, b, a]]
    assert all((P[r][k] - Lm[r][k]).sign() == 0 for r in range(4) for k in range(4)), 'P is not a left multiplication'
    assert (a * a + b * b + c_ * c_ + d_ * d_ - 1).sign() == 0, '|lambda| != 1'
    he = height(Qe)
    assert he == hs + 2 * len(w)
    LAM.append(lam)
    print('B: word %d (%d letters): step hypotheses and (*) hold at every state (%d margin checks); labels return; '
          'P = L(lambda); K at height %d at the end; %.0fs' % (i, len(w), nchk, he, time.time() - t0), flush=True)

# C
R2 = ETA ** 2 - ETA ** 4 / 4
TGT = 1 - ETA ** 2 / 2
SC = 10 ** 40
def sqrt_lo(r):
    return Fr(isqrt(r.numerator * SC * SC // r.denominator), SC)
leaves = {tuple(b[:4]): b[4] for b in cert['boxes']}
assert len(leaves) == len(cert['boxes']), 'duplicate boxes'
maxdep = max(k[0] for k in leaves)
for lam in LAM: assert lam[0].sign() > 0
stats = {'leaf': 0, 'out': 0, 'split': 0}
def cover(dep, i, j, k):
    h = 2 * ETA / 2 ** dep
    lo = [-ETA + i * h, -ETA + j * h, -ETA + k * h]; hi = [x + h for x in lo]
    r2max = sum(max(a * a, b * b) for a, b in zip(lo, hi))
    r2min = sum(Fr(0) if a <= 0 <= b else min(a * a, b * b) for a, b in zip(lo, hi))
    if r2min > R2: stats['out'] += 1; return
    key = (dep, i, j, k)
    if key in leaves:
        lam = LAM[leaves[key]]
        sb = sqrt_lo(1 - min(r2max, R2))
        val = lam[0] * sb
        for t in range(3):
            lt = lam[t + 1]
            val = val - (lt * hi[t] if lt.sign() >= 0 else lt * lo[t])
        assert (val - TGT).sign() >= 0, ('box inequality fails', key)
        stats['leaf'] += 1; return
    assert dep < maxdep, ('box not covered', key)
    stats['split'] += 1
    for a in (0, 1):
        for b in (0, 1):
            for e in (0, 1):
                cover(dep + 1, 2 * i + a, 2 * j + b, 2 * k + e)
cover(0, 0, 0, 0)
assert stats['leaf'] == len(leaves)
print('C: the u_vec-ball of {|u - 1| <= %d/%d} is covered by %d boxes (%d split boxes, %d boxes outside the ball), '
      'each with a word mapping it into the ball; %.0fs' % (pe, qe, stats['leaf'], stats['split'], stats['out'], time.time() - t0), flush=True)

# D
for i, w in enumerate(cert['words']):
    sw = clone(s)
    for c, j in w:
        o = sw.apply(sim.Twist(C, c, E(c, j), 'menu', {'menu': M.name}), journal=False)
        assert o.applied and not o.uncertain, ('letter not admissible in the simulator', i, c, j)
    a, b, c_, d_ = LAM[i]
    Lm = [[a, -b, -c_, -d_], [b, a, -d_, c_], [c_, d_, a, -b], [d_, -c_, b, a]]
    Qw = sw.pose(K[0]); assert all(sw.pose(p) == Qw for p in K)
    assert Qw == to_tuple(matmul(Lm, Qs)), ('simulator end pose differs from L(lambda) Q*', i)
print('D: every certificate word replays with the simulator from the centre and ends with K at L(lambda_i) Q*; %.0fs' % (time.time() - t0))
print('PASS: ball certificate holds: centre after %d + %d letters (K at height %d), eta = %d/%d, %d words (%s letters), %d boxes'
      % (n0, CI, hs, pe, qe, len(LAM), [len(w) for w in cert['words']], len(leaves)))
