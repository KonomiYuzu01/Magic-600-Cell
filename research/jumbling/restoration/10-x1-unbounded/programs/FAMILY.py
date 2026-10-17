# Members of the unbounded family of the Theorem of REPORT.md, built and replayed exactly. Run from the repository root, with S = research/jumbling/restoration/10-x1-unbounded:
#   python3 -I -B $S/programs/FAMILY.py <N> [certificate, default $S/witnesses/CERT.json]
# Member t (t = 0, 1, ...) is the word: the first n0 + centre letters of the witness, then W_{i_0} ... W_{i_{t-1}},
# where u_0 = 1, i_s is the word of the first box (in the certificate's order) whose closed box contains the
# vector part of u_s, and u_{s+1} = lambda_{i_s} u_s (Hamilton product; lambda_i from P_i = L(lambda_i), exact).
# The program builds member N in exact Q(sqrt5) arithmetic (checking |u_s - 1| <= eta at every s), writes it to
# $S/witnesses/F<N>.json, and replays it with the repository simulator (every letter applied and not uncertain),
# checking at every member boundary s that K's pose is exactly L(u_s) Q* and that K's left height is h* + 2 m
# (m = letters after the centre). Members t < N are prefixes of member N.
import sys, json, time
from fractions import Fraction as Fr
from pathlib import Path
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import sim, restore as R, trees as T
from exact import matmul, Q5
from sim.kplus import to_tuple

N = int(sys.argv[1])
cpath = sys.argv[2] if len(sys.argv) > 2 else 'research/jumbling/restoration/10-x1-unbounded/witnesses/CERT.json'
t0 = time.time()
cert = json.loads(Path(cpath).read_text())
wit = json.loads(Path(cert['witness']).read_text()); WORD = [tuple(x) for x in wit['word']]
K = tuple(cert['claims']['pieces']); n0 = cert['n0']; CI = cert['centre']
pe, qe = cert['eta']; ETA = Fr(pe, qe)
C = sim.get_context(); M = R.menu('I_a'); s = sim.State(C, filtered=False, menu=M)
KP = C.kplus; I4 = KP.matrix(0)
EL = {}
def E(g, j):
    if (g, j) not in EL: EL[(g, j)] = R.element(M, g, j)
    return EL[(g, j)]
def Lm(q):
    a, b, c, d = q
    return [[a, -b, -c, -d], [b, a, -d, c], [c, d, a, -b], [d, -c, b, a]]
def qmul(x, y):
    a0, a1, a2, a3 = x; b0, b1, b2, b3 = y
    return (a0 * b0 - a1 * b1 - a2 * b2 - a3 * b3, a0 * b1 + a1 * b0 + a2 * b3 - a3 * b2,
            a0 * b2 - a1 * b3 + a2 * b0 + a3 * b1, a0 * b3 + a1 * b2 - a2 * b1 + a3 * b0)
WORDS = [[tuple(x) for x in w] for w in cert['words']]
LAM = []
for w in WORDS:
    P = to_tuple(I4)
    for c, j in w: P = to_tuple(matmul(E(c, j), P))
    lam = tuple(P[r][0] for r in range(4))
    assert all((P[r][k] - Lm(lam)[r][k]).sign() == 0 for r in range(4) for k in range(4))
    LAM.append(lam)
BOXES = cert['boxes']
def box_of(u):
    for dep, i, j, k, wi in BOXES:
        h = 2 * ETA / 2 ** dep
        lo = [-ETA + i * h, -ETA + j * h, -ETA + k * h]
        if all((u[t + 1] - lo[t]).sign() >= 0 and (lo[t] + h - u[t + 1]).sign() >= 0 for t in range(3)):
            return wi
    raise AssertionError('no box contains u')
u = (Q5(1), Q5(0), Q5(0), Q5(0)); US = [u]; CHOICE = []
for t in range(N):
    assert (u[0] - (1 - ETA ** 2 / 2)).sign() >= 0, '|u - 1| > eta'
    wi = box_of(u); CHOICE.append(wi)
    u = qmul(LAM[wi], u); US.append(u)
assert (u[0] - (1 - ETA ** 2 / 2)).sign() >= 0
word = WORD[:n0 + CI] + [x for wi in CHOICE for x in WORDS[wi]]
bounds = [n0 + CI]
for wi in CHOICE: bounds.append(bounds[-1] + len(WORDS[wi]))
hstar = 30 + 2 * CI
print('member %d: words %s, %d letters; built in %.0fs' % (N, CHOICE, len(word), time.time() - t0), flush=True)
out = {'from': 'solved', 'word': [list(x) for x in word],
       'claims': {'pieces': list(K), 'final_left_height_of_pieces': hstar + 2 * (len(word) - n0 - CI),
                  'member': N, 'certificate_words': CHOICE, 'member_boundaries': bounds,
                  'heights_at_boundaries': [hstar + 2 * (b - n0 - CI) for b in bounds]}}
Path('research/jumbling/restoration/10-x1-unbounded/witnesses/F%d.json' % N).write_text(json.dumps(out))
# replay
Qstar = None; nb = 0
for n, (c, j) in enumerate(word, start=1):
    o = s.apply(sim.Twist(C, c, E(c, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, ('letter not admissible', n, c, j)
    if n in bounds:
        Q = s.pose(K[0]); assert all(s.pose(p) == Q for p in K)
        if Qstar is None: Qstar = Q
        assert Q == to_tuple(matmul(Lm(US[nb]), Qstar)), ('pose differs from L(u) Q*', nb)
        h = T.dist(T.factors(Q)[0])
        assert h == hstar + 2 * (n - n0 - CI), ('height', nb, h)
        print('  member %d ends at letter %d: K at L(u_%d) Q*, height %d; %.0fs' % (nb, n, nb, h, time.time() - t0), flush=True)
        nb += 1
assert nb == N + 1
print('PASS: members 0..%d admissible (exact replay of %d letters); K at L(u_t) Q* at every boundary; heights %d .. %d'
      % (N, len(word), hstar, hstar + 2 * (len(word) - n0 - CI)))
