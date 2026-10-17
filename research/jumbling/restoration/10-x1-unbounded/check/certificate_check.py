# Integrator's independent check of the X1 certificate (REPORT.md sections 1 and 2), written from the
# mathematical statements, not from programs/CERT.py. Run from the repository root, with S = research/jumbling/restoration/10-x1-unbounded:
#   python3 -I -B $S/check/certificate_check.py $S/witnesses/CERT.json $S/witnesses/H6030.json
# Exact arithmetic only (the repository's Q(sqrt5) numbers and Python fractions). The simulator is used for
# the letters' admissibility from the solved state, for cap membership (classify) and for the poses at X.
import sys, json, time
from fractions import Fraction as Fr
from math import isqrt
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import sim, restore as R, trees as T
from exact import Q5, matmul, transpose
from sim.kplus import to_tuple

t0 = time.time()
cert = json.load(open(sys.argv[1]))
WORD = [tuple(x) for x in json.load(open(sys.argv[2]))['word']]
K = tuple(cert['claims']['pieces'])
n0, CI = cert['n0'], cert['centre']
c, d = cert['grips']
ETA = Fr(*cert['eta'])
OTHER = {c: d, d: c}
C = sim.get_context(); D = C.data; RG = C.regions; KP = C.kplus
M = R.menu('I_a')
I4 = to_tuple(KP.matrix(0))
EL = {(x, j): to_tuple(R.element(M, x, j)) for x in (c, d) for j in range(60)}
ITEM = {x: {EL[(x, j)]: j for j in range(60)} for x in (c, d)}
assert all(len(ITEM[x]) == 60 for x in (c, d))
JUMBLE = {x: {j for j in range(60) if KP.index_of_matrix(EL[(x, j)]) < 0} for x in (c, d)}
print('items: %d jumbles at %d, %d at %d (own test: not in K+)' % (len(JUMBLE[c]), c, len(JUMBLE[d]), d))

def dot(u, v):
    return u[0] * v[0] + u[1] * v[1] + u[2] * v[2] + u[3] * v[3]
def mv(Q, v):
    return tuple(dot(Q[i], v) for i in range(4))
KAP = D.KAPPA
N = {x: tuple(D.N[x]) for x in (c, d)}
NN = {x: dot(N[x], N[x]) for x in (c, d)}

# ---- the tree lemma's premises for every item at both grips
def qconj(q): return (q[0], -q[1], -q[2], -q[3])
for x in (c, d):
    n = N[x]
    assert T.dist(n) == 1
    for j in range(60):
        a = T.factors(EL[(x, j)])[0]
        assert T.dist(T.qmul(T.qmul(qconj(n), a), n)) == 0, ('left factor does not fix w_x', x, j)
        if j in JUMBLE[x]:
            assert T.dist(a) == 2, ('jumble does not move v0 to distance 2', x, j)
        else:
            assert T.dist(a) == 0, ('retained item moves v0', x, j)
print('tree lemma premises: every left factor at %d and %d fixes w_x; jumbles move v0 (distance 2); '
      'branches of the grips: %d, %d' % (c, d, T.branch(N[c]), T.branch(N[d])))
assert T.branch(N[c]) != T.branch(N[d])

# ---- X: the first n0 letters, with the simulator
s = sim.State(C, filtered=False, menu=M)
def sim_apply(st, x, j):
    o = st.apply(sim.Twist(C, x, R.element(M, x, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, ('not admissible', x, j)
for x, j in WORD[:n0]:
    sim_apply(s, x, j)
QX = s.pose(K[0]); assert all(s.pose(p) == QX for p in K)
VK = sorted({tuple(v) for p in K for v in RG.vertices(p)}, key=repr)
VV = [dot(v, v) for v in VK]
def margins(Q, x):
    w = mv(transpose(Q), N[x])           # Q^T n
    return [dot(w, v) - KAP for v in VK]
assert all(m.sign() > 0 for x in (c, d) for m in margins(QX, x)), 'K not strictly inside both caps at X'
lX, _ = T.factors(QX)
print('X: K at left height %d, branch %d; %d vertices; all margins > 0 in caps %d and %d'
      % (T.dist(lX), T.branch(lX), len(VK), c, d))
cl = {x: s.classify(x, need_inside=True, stop_at_straddle=False) for x in (c, d)}
for x in (c, d):
    assert cl[x].straddle is None and not cl[x].uncertain, ('cut not clean at X', x)
Dset = {x: sorted(set(cl[x].inside.tolist()) - set(K)) for x in (c, d)}
assert set(K) <= set(cl[c].inside.tolist()) and set(K) <= set(cl[d].inside.tolist())
assert not (set(Dset[c]) & set(Dset[d])), 'another piece lies whole in both caps'
print('X: cuts %d and %d clean; |D_%d| = %d, |D_%d| = %d; no other piece whole in both caps'
      % (c, d, c, len(Dset[c]), d, len(Dset[d])))

# ---- Good sets by own vertex margins (pieces are convex: a piece is 'out' of cap y iff every vertex has margin <= 0)
POSEX = {p: s.pose(p) for x in (c, d) for p in Dset[x]}
def good_set(x):
    y = OTHER[x]
    groups = {}
    for p in Dset[x]:
        groups.setdefault(POSEX[p], []).append(p)
    good = []
    for j in range(60):
        A = EL[(x, j)]
        ok = True
        for P, ps in groups.items():
            w = mv(transpose(to_tuple(matmul(A, P))), N[y])
            for p in ps:
                if any((dot(w, v) - KAP).sign() > 0 for v in RG.vertices(p)):
                    ok = False; break
            if not ok: break
        if ok: good.append(j)
    return good
GOOD = {x: set(good_set(x)) for x in (c, d)}
print('Good_%d (%d) = %s' % (c, len(GOOD[c]), sorted(GOOD[c])))
print('Good_%d (%d) = %s; %.0fs' % (d, len(GOOD[d]), sorted(GOOD[d]), time.time() - t0), flush=True)
assert ITEM[c][I4] in GOOD[c] and ITEM[d][I4] in GOOD[d]

# ---- the step hypotheses along a word, from (Q, labels, next grip); optional (*) at every state
E2 = ETA * ETA
def star_ok(m, x, k):
    G0 = m + KAP
    L = m - G0 * (E2 / 2)
    if L.sign() <= 0:
        return False
    return (L * L - (NN[x] * VV[k] - G0 * G0) * E2).sign() > 0
def walk(Q, lab, nxt, word, star):
    lab = dict(lab); n_star = 0
    def state_ok(Q):
        nonlocal n_star
        for x in (c, d):
            for k, m in enumerate(margins(Q, x)):
                assert m.sign() > 0, 'K not strictly inside a cap'
                if star:
                    assert star_ok(m, x, k), ('(*) fails', x, k)
                    n_star += 1
    state_ok(Q)
    for i, (x, j) in enumerate(word):
        assert x == nxt, ('grips do not alternate', i)
        assert j in JUMBLE[x], ('not a jumble', i)
        g = to_tuple(matmul(EL[(x, j)], lab[x]))
        assert ITEM[x][g] in GOOD[x], ('running product leaves Good', i)
        lab[x] = g
        Q = to_tuple(matmul(EL[(x, j)], Q))
        nxt = OTHER[x]
        state_ok(Q)
    return Q, lab, nxt, n_star

# ---- the pump to the centre sigma
assert T.branch(lX) == T.branch(N[c])
Qs, labs, nxs, _ = walk(QX, {c: I4, d: I4}, d, WORD[n0:n0 + CI], False)
ls_, _ = T.factors(Qs)
hs = T.dist(ls_)
print('sigma: %d pump letters keep the step hypotheses; K at height %d (= %d + 2*%d), branch %d; labels (%d, %d); next grip %d; %.0fs'
      % (CI, hs, T.dist(lX), CI, T.branch(ls_), ITEM[c][labs[c]], ITEM[d][labs[d]], nxs, time.time() - t0), flush=True)
assert hs == T.dist(lX) + 2 * CI
assert [ITEM[c][labs[c]], ITEM[d][labs[d]]] == cert['claims']['centre_labels'] and nxs == cert['claims']['centre_next_grip']

# ---- the certificate words from sigma
def Lmat(q):
    a, b, cc, dd = q
    return ((a, -b, -cc, -dd), (b, a, -dd, cc), (cc, dd, a, -b), (dd, -cc, b, a))
LAM = []
for i, w in enumerate(cert['words']):
    w = [tuple(x) for x in w]
    Qe, labe, nxe, nst = walk(Qs, labs, nxs, w, True)
    assert labe == labs and nxe == nxs, ('labels or phase do not return', i)
    P = I4
    for x, j in w:
        P = to_tuple(matmul(EL[(x, j)], P))
    lam = tuple(P[r][0] for r in range(4))
    assert P == to_tuple(Lmat(lam)), ('product is not a left multiplication', i)
    assert dot(lam, lam) == 1 and lam[0].sign() > 0
    assert Qe == to_tuple(matmul(P, Qs))
    LAM.append(lam)
    print('word %d: %d letters, step hypotheses and (*) at every state (%d checks), labels return, P = L(lambda), '
          'lambda_0 > 0, |lambda| = 1; %.0fs' % (i, len(w), nst, time.time() - t0), flush=True)

# ---- covering of the ball {u_vec : |u - 1| <= eta} and the box inequality
R2 = E2 - E2 * E2 / 4
TGT = 1 - E2 / 2
boxes = cert['boxes']
dmax = max(b[0] for b in boxes)
for dep, i, j, k, wi in boxes:
    assert 0 <= dep <= dmax and all(0 <= t < 2 ** dep for t in (i, j, k)) and 0 <= wi < len(LAM)
assert len({tuple(b[:4]) for b in boxes}) == len(boxes)
def box_bounds(dep, i, j, k):
    h = 2 * ETA / 2 ** dep
    return [(-ETA + t * h, -ETA + (t + 1) * h) for t in (i, j, k)]
ncell = 0
n = 2 ** dmax
for a in range(n):
    for b in range(n):
        for e in range(n):
            bb = box_bounds(dmax, a, b, e)
            r2min = sum(Fr(0) if lo <= 0 <= hi else min(lo * lo, hi * hi) for lo, hi in bb)
            if r2min > R2:
                continue
            ncell += 1
            assert any(a >> (dmax - dep) == i and b >> (dmax - dep) == j and e >> (dmax - dep) == k
                       for dep, i, j, k, wi in boxes), ('cell not covered', a, b, e)
def sqrt_floor(r, bits=256):
    # largest multiple of 2^-bits whose square is <= r
    s = Fr(isqrt((r.numerator << (2 * bits)) // r.denominator), 1 << bits)
    assert s * s <= r
    return s
for dep, i, j, k, wi in boxes:
    bb = box_bounds(dep, i, j, k)
    r2max = sum(max(lo * lo, hi * hi) for lo, hi in bb)
    sb = sqrt_floor(1 - min(r2max, R2))
    lam = LAM[wi]
    corners = [(x0, x1, x2) for x0 in bb[0] for x1 in bb[1] for x2 in bb[2]]
    vals = [lam[1] * p[0] + lam[2] * p[1] + lam[3] * p[2] for p in corners]
    mx = vals[0]
    for v in vals[1:]:
        if (v - mx).sign() > 0: mx = v
    assert (lam[0] * sb - mx - TGT).sign() >= 0, ('box inequality fails', dep, i, j, k)
print('cover: %d cells of depth %d meet the ball and each lies in a listed box; all %d boxes satisfy the inequality; %.0fs'
      % (ncell, dmax, len(boxes), time.time() - t0))
print('PASS (independent check): tree premises, X, Good sets, pump to sigma, %d words with (*), cover of eta = %s'
      % (len(LAM), ETA))
