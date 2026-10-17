# Integrator's independent check of the family of REPORT.md section 1 and of the property Z(t), written from the
# statements, not from programs/FAMILY.py. It builds member N (own Hamilton product and box lookup), replays it with the
# repository simulator, and checks:
#   - every item at the two grips is orthogonal and fixes its grip normal (used by the isolation lemma);
#   - every letter after X is applied, not uncertain, and moves exactly D_x and K (|D_x| + 7 pieces);
#   - every 100 letters and at every member boundary, the pose of every piece: D_c at its X-pose turned by the
#     running product at c, D_d by the one at d, every other piece outside K at its X-pose;
#   - at sigma, K at (product of the pump letters) times its X-pose; at boundary t, K at L(u_t) Q*, with left height
#     h_X + 2 (letters after X) in the branch of d, and |u_t - 1| <= eta.
# Run from the repository root, with S = research/jumbling/restoration/10-x1-unbounded:
#   python3 -I -B $S/check/family_check.py $S/witnesses/CERT.json $S/witnesses/H6030.json <N> [$S/witnesses/F<N>.json]
import sys, json, time
from fractions import Fraction as Fr
import numpy as np
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import sim, restore as R, trees as T
from exact import matmul, transpose
from sim.kplus import to_tuple

t0 = time.time()
cert = json.load(open(sys.argv[1]))
WORD = [tuple(x) for x in json.load(open(sys.argv[2]))['word']]
NMEM = int(sys.argv[3])
THEIRS = json.load(open(sys.argv[4])) if len(sys.argv) > 4 else None
K = tuple(cert['claims']['pieces'])
n0, CI = cert['n0'], cert['centre']
c, d = cert['grips']
ETA = Fr(*cert['eta'])
C = sim.get_context(); D = C.data; KP = C.kplus
M = R.menu('I_a')
I4 = to_tuple(KP.matrix(0))
EL = {(x, j): to_tuple(R.element(M, x, j)) for x in (c, d) for j in range(60)}
ITEM = {x: {EL[(x, j)]: j for j in range(60)} for x in (c, d)}

def dot(u, v):
    return u[0] * v[0] + u[1] * v[1] + u[2] * v[2] + u[3] * v[3]
def mv(Q, v):
    return tuple(dot(Q[i], v) for i in range(4))

for x in (c, d):
    n = tuple(D.N[x])
    for j in range(60):
        A = EL[(x, j)]
        assert to_tuple(matmul(A, transpose(A))) == I4, ('item not orthogonal', x, j)
        assert mv(transpose(A), n) == n, ('item does not fix its grip normal', x, j)
print('items: all 120 at grips %d and %d are orthogonal and fix their grip normals' % (c, d))

# ---- the family rule, with an own Hamilton product and box lookup
def Lmat(q):
    a, b, e, f = q
    return ((a, -b, -e, -f), (b, a, -f, e), (e, f, a, -b), (f, -e, b, a))
def hamilton(p, q):
    p0, p1, p2, p3 = p; q0, q1, q2, q3 = q
    return (p0 * q0 - p1 * q1 - p2 * q2 - p3 * q3,
            p0 * q1 + q0 * p1 + (p2 * q3 - p3 * q2),
            p0 * q2 + q0 * p2 + (p3 * q1 - p1 * q3),
            p0 * q3 + q0 * p3 + (p1 * q2 - p2 * q1))
WORDS = [[tuple(x) for x in w] for w in cert['words']]
LAM = []
for w in WORDS:
    P = I4
    for x, j in w:
        P = to_tuple(matmul(EL[(x, j)], P))
    lam = tuple(P[r][0] for r in range(4))
    assert P == to_tuple(Lmat(lam))
    LAM.append(lam)
def first_box(u):
    for dep, i, j, k, wi in cert['boxes']:
        h = 2 * ETA / 2 ** dep
        if all((u[t + 1] - (-ETA + idx * h)).sign() >= 0 and ((-ETA + (idx + 1) * h) - u[t + 1]).sign() >= 0
               for t, idx in enumerate((i, j, k))):
            return wi
    raise AssertionError('no listed box contains the vector part of u')
one = LAM[0][0] - LAM[0][0] + 1
U = [(one, one - 1, one - 1, one - 1)]
CHOICE = []
for t in range(NMEM):
    wi = first_box(U[-1])
    CHOICE.append(wi)
    U.append(hamilton(LAM[wi], U[-1]))
for u in U:
    assert dot(u, u) == 1 and (u[0] - (1 - ETA * ETA / 2)).sign() >= 0, '|u - 1| > eta'
word = WORD[:n0 + CI] + [l for wi in CHOICE for l in WORDS[wi]]
bounds = [n0 + CI]
for wi in CHOICE:
    bounds.append(bounds[-1] + len(WORDS[wi]))
print('member %d: words %s; %d letters; |u_t - 1| <= eta for t = 0..%d; %.0fs' % (NMEM, CHOICE, len(word), NMEM, time.time() - t0), flush=True)
if THEIRS is not None:
    assert [tuple(l) for l in THEIRS['word']] == word, 'differs from the run\'s word'
    assert THEIRS['claims']['certificate_words'] == CHOICE
    print('the run\'s F%d.json holds the same word and the same word choices' % NMEM)

# ---- replay
s = sim.State(C, filtered=False, menu=M)
def step(x, j):
    o = s.apply(sim.Twist(C, x, R.element(M, x, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, ('not admissible', x, j)
    return o.moved
for x, j in word[:n0]:
    step(x, j)
QX = s.pose(K[0]); assert all(s.pose(p) == QX for p in K)
lX, _ = T.factors(QX); hX = T.dist(lX)
cl = {x: s.classify(x, need_inside=True, stop_at_straddle=False) for x in (c, d)}
for x in (c, d):
    assert cl[x].straddle is None and not cl[x].uncertain
    assert set(K) <= set(cl[x].inside.tolist())
DS = {x: sorted(set(cl[x].inside.tolist()) - set(K)) for x in (c, d)}
assert not (set(DS[c]) & set(DS[d]))
NPC = len(s.pose_id)
pidX = s.pose_id.copy()
matX = {int(p): s._pose_matrix(int(p)) for p in np.unique(pidX)}
CLS = np.zeros(NPC, np.int8)
CLS[DS[c]] = 1; CLS[DS[d]] = 2; CLS[list(K)] = 3
GROUPS = []
for k in (0, 1, 2):
    sel = np.nonzero(CLS == k)[0]
    for p in np.unique(pidX[sel]):
        GROUPS.append((k, int(p), sel[pidX[sel] == p]))
NONK = CLS != 3
print('X: K at height %d, branch %d; |D_%d| = %d, |D_%d| = %d; %d pieces in %d groups; %.0fs'
      % (hX, T.branch(lX), c, len(DS[c]), d, len(DS[d]), NPC, len(GROUPS), time.time() - t0), flush=True)

def key_of(m):
    k = KP.index_of_matrix(m)
    return ('K', int(k)) if k >= 0 else ('M', m)
def check_debris(G, where):
    pred = np.full(NPC, -1, np.int64)
    for k, p, members in GROUPS:
        m = matX[p] if k == 0 else to_tuple(matmul(G[c] if k == 1 else G[d], matX[p]))
        cur = s._lookup.get(key_of(m))
        assert cur is not None, ('a predicted pose is held by no piece', where)
        pred[members] = cur
    bad = int(np.count_nonzero(pred[NONK] != s.pose_id[NONK]))
    assert bad == 0, ('pieces off their predicted pose', where, bad)

G = {c: I4, d: I4}
check_debris(G, n0)
PK = I4
Qstar = None
nchecks = 1
for n in range(n0 + 1, len(word) + 1):
    x, j = word[n - 1]
    moved = step(x, j)
    assert moved == len(DS[x]) + len(K), ('moved count', n, moved)
    G[x] = to_tuple(matmul(EL[(x, j)], G[x]))
    PK = to_tuple(matmul(EL[(x, j)], PK))
    if n % 100 == 0 or n in bounds:
        check_debris(G, n); nchecks += 1
    if n in bounds:
        b = bounds.index(n)
        Q = s.pose(K[0]); assert all(s.pose(p) == Q for p in K)
        if Qstar is None:
            assert Q == to_tuple(matmul(PK, QX)), 'K at sigma is not the pump product times its X-pose'
            Qstar = Q
            labels = (ITEM[c][G[c]], ITEM[d][G[d]])
            print('sigma (letter %d): every letter after X moved exactly D_x and K; debris as predicted; labels %s; '
                  'K at P_pump Q_X' % (n, labels), flush=True)
        else:
            assert (ITEM[c][G[c]], ITEM[d][G[d]]) == labels, ('labels do not return', b)
        assert Q == to_tuple(matmul(Lmat(U[b]), Qstar)), ('K not at L(u_t) Q*', b)
        lq, _ = T.factors(Q)
        h = T.dist(lq)
        assert h == hX + 2 * (n - n0) and T.branch(lq) == T.branch(tuple(D.N[d])), ('height or branch', b, h)
        print('  boundary %d (letter %d): debris as predicted, K at L(u_%d) Q*, height %d; %.0fs'
              % (b, n, b, h, time.time() - t0), flush=True)
print('PASS (independent family check): member %d, %d letters, %d debris checks of all %d pieces, heights %d .. %d'
      % (NMEM, len(word), nchecks, NPC, hX + 2 * CI, hX + 2 * (len(word) - n0)))
