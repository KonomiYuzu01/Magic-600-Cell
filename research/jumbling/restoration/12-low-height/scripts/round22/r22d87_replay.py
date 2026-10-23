# Theory round 22, line (d), round 6: the integrator's letter-by-letter replay of the literal words of the joint attack
# on caps 311 and 290 (R22D-87), from the 3000-twist stop of seed 7302 (015d839e...), read from a snapshot written by
# r22d31_snapshots.py. The words are those of analysis A (A-R22J-01 to A-R22J-04), of analysis B (B-R22J-01, B-R22J-02
# and B-R22J-04) and of the cross-examination (R22JX-02), each from its start state: the stop, 1ff0061f... (R22D-75's
# seven letters), 9e6b991b... (and (136, 2), (136, 6); R22D-86), fe273a8c... and efab42da... (analysis B's two words).
# It prints GEOMETRY (R^2, the pairs of caps of R22D-75's seven letters that meet inside the ball, and those of the
# seven caps that meet cap 311, by the test of A-R22J-03), PRODUCTS (the item equal to the product of two letters at
# one grip, 0 for the identity), and for each word START (its source, the start digest, the target tau, u = the
# number of straddlers of tau by eb6.straddler_set, how many of the start's height-6 pieces tau holds whole, the
# counts, off and N), one LETTER line per letter (admissible and certain, the largest height, whether n6 and the exact
# pose of every height-6 piece of the start are kept, how many of those keep their pose, how many are now at height 4
# and whether they are the ones tau held, how many pieces are at height 6 that were not, u, the straddlers of tau the
# letter removed and added and the branches of the added ones, the counts, off, N and the digest) and END (the end
# digest and u). At the ends of analysis B's two words it also prints BLOCKERS (the straddlers of tau), HOLDERS (each
# cap that holds one of them whole, with its own straddler count and the number of height-6 pieces it holds whole) and
# ITEMS (u after each of the 59 items of each holder that is clean and holds no height-6 piece, each on its own copy).
# It stops with an assertion at a start digest that differs or at the first letter that is not admissible and certain.
# The integrator's script; output in runs/round22/r22d87_replay.txt. About ten CPU minutes after the snapshot loads.
# Research only, headless Linux; exact geometry. It writes no file; theory/seams.py keeps its cache in
# work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d87_replay.py <dir>/s7302-stop.pkl
import sys, json, resource
from collections import Counter
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU, (2390, 2400))
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
from sim.state import Classification
from sim.kernel import Evaluator, pullback_form
from exact import dot, matmul, Q5
from sim.kplus import to_tuple

C = E.sim.get_context(); M = E.R.menu('I_a'); REG = C.regions; D = C.data
s0, _ = load(sys.argv[1], E)
STOP = '015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f'
S7 = '1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81'
S9 = '9e6b991bf057d3f2c28c3158aed3878b55720c970601019eb819e53bd0601e94'
SU = 'fe273a8c88d8a8fe7f030f95e6130fc76a57da6d708dea110a1cb31125276122'
SW = 'efab42dab21df190c53346f34b78f87a138e34be9e0314c92d78f5e04b666f43'
assert s0.digest() == STOP
SEVEN = [(64, 15), (185, 14), (357, 12), (471, 12), (263, 14), (407, 13), (523, 12)]
NINE = SEVEN + [(136, 2), (136, 6)]
BU = [(185, 1), (263, 1), (263, 3)]
BW = [(185, 1), (185, 7), (185, 16), (185, 3), (263, 1), (263, 3), (263, 8), (364, 1), (407, 10), (263, 2)]
A1 = [(407, 10), (263, 11), (185, 21), (364, 1), (263, 2)]
A2 = [(523, 45), (407, 47), (263, 14), (471, 45), (357, 45), (185, 14)] + A1
# (source, prefix from the stop, digest after it, word, tau, print the holders at the end)
WORDS = [('A-R22J-01', [], STOP, A1 + [(311, 14)], 311, False),
         ('A-R22J-02', SEVEN, S7, A2 + [(311, 14)], 311, False),
         ('A-R22J-03', [], STOP, SEVEN, 311, False),
         ('A-R22J-03', [], STOP, [(185, 14), (263, 14)], 311, False),
         ('A-R22J-03', [], STOP, [(263, 14), (185, 14)], 311, False),
         ('A-R22J-03', [], STOP, [(407, 13)], 311, False),
         ('B-R22J-02', [], STOP, SEVEN[:5] + [(523, 12), (407, 13)], 311, False),
         ('B-R22J-02', [], STOP, [(64, 15), (263, 14), (357, 12), (471, 12), (185, 14), (407, 13), (523, 12)], 311, False),
         ('B-R22J-02', [], STOP, [(64, 15), (407, 13), (357, 12), (471, 12), (263, 14), (185, 14), (523, 12)], 311, False),
         ('A-R22J-04', NINE, S9, [(277, 1), (301, 4)], 290, False),
         ('A-R22J-04', NINE, S9, [(277, 3), (277, 5), (301, 4)], 290, False),
         ('A-R22J-04', NINE, S9, [(136, 2), (301, 1)], 290, False),
         ('B-R22J-04', SEVEN, S7, [(136, 10)], 290, False),
         ('B-R22J-01', SEVEN, S7, BU, 311, True),
         ('B-R22J-01', [], STOP, BW, 311, True),
         ('R22JX-02', SEVEN + BU, SU, [(364, 3), (263, 6)], 311, False),
         ('R22JX-02', BW, SW, [(185, 9), (364, 1)], 311, False)]
# (grip, first item, second item): the letter (grip, second) played after (grip, first)
PRODUCTS = [(277, 3, 5), (136, 2, 6), (523, 12, 45), (407, 13, 47), (263, 14, 14), (471, 12, 45), (357, 12, 45),
            (185, 14, 14)]
cache = {}


def clone(st):
    t = E.sim.State(C, filtered=False, menu=M)
    t.pose_id = st.pose_id.copy(); t._poses = list(st._poses); t._lookup = dict(st._lookup)
    return t


def put(st, c, j):
    o = st.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
    return o.applied and not o.uncertain


def info(p):
    """(left height, branch) of a pose; (0, -1) at the lattice."""
    if p.kidx >= 0:
        return 0, -1
    if p.key not in cache:
        l, r = E.T.factors(p.matrix); assert E.T.dist(r) == 0
        cache[p.key] = (E.T.dist(l), E.T.branch(l))
    return cache[p.key]


def heights(st):
    return np.array([info(p)[0] for p in st._poses])[st.pose_id]


def branch(st, p):
    return info(st._poses[int(st.pose_id[p])])[1]


def keys(st, ps):
    return [st._poses[int(st.pose_id[p])].key for p in ps]


def inside(st, pieces, c):
    """The given off pieces that lie whole inside cap c, by the simulator's own group and anchor tests."""
    out = []
    cl = Classification(int(c))
    for pid, members in st._groups(np.asarray(sorted(pieces), np.int64)):
        ev = Evaluator(pullback_form(st._pose_matrix(pid), D.N[c]))
        for anchor, sub in st._anchors(pid, members):
            r = st._eval(ev, REG.cap_vform(anchor), lambda a=anchor: REG.cap_frows(a), cl, None)
            assert r[0] != 'uncertain'
            if r[0] != 'straddle':
                if r[0] == 'in':
                    out += sub.tolist()
                continue
            for p in sub.tolist():
                q = st._eval(ev, REG.vform(p), lambda x=p: REG.frows(x), cl, None)
                assert q[0] != 'uncertain'
                if q[0] == 'in':
                    out.append(p)
    return out


def tail(st, h):
    return dict(n2_n4_n6=[int((h == k).sum()) for k in (2, 4, 6)], off=int(st.off_lattice_count()),
                N=E.R.Restorer(st, M).misalignment(), digest=st.digest())


def show(tag, **kw):
    print(tag, json.dumps(kw), flush=True)


R2 = max((dot(v, v) for vs in REG.rep_vertices for v in vs), key=float)
assert all((dot(v, v) - R2).sign() <= 0 for vs in REG.rep_vertices for v in vs)
assert (R2 - Q5(96, -32)).sign() == 0                       # 96 - 32 sqrt 5


def meet(c, d):
    """Caps c and d meet inside the ball (A-R22J-03): (NN + n_c . n_d) R^2 >= 2 KAPPA^2, equality counted as meeting."""
    return ((D.NN + dot(D.N[c], D.N[d])) * R2 - 2 * D.KAPPA * D.KAPPA).sign() >= 0


caps7 = [c for c, _ in SEVEN]
show('GEOMETRY', R2=[R2.a, R2.b, R2.d], meeting_pairs=[[c, d] for i, c in enumerate(caps7) for d in caps7[i + 1:] if meet(c, d)],
     meeting_311=[c for c in caps7 if meet(c, 311)])
rows = []
for c, a, b in PRODUCTS:
    g = to_tuple(matmul(E.R.element(M, c, b), E.R.element(M, c, a)))
    js = [j for j in range(60) if E.R.element(M, c, j) == g]
    rows.append(dict(grip=c, first=a, second=b, item=js, identity=g == E.R.I4))
show('PRODUCTS', rows=rows)
for n, (source, prefix, start, word, tau, ends) in enumerate(WORDS, 1):
    s = clone(s0)
    for c, j in prefix:
        assert put(s, c, j)
    assert s.digest() == start, (source, s.digest())
    h = heights(s); six = np.flatnonzero(h == 6).tolist(); k6 = keys(s, six); sixset = set(six)
    held = set(inside(s, six, tau)); S = set(E.straddler_set(s, tau).tolist())
    show('START', word=n, source=source, prefix=prefix, letters=word, tau=tau, u=len(S), tau_holds_h6=len(held), **tail(s, h))
    for i, (c, j) in enumerate(word, 1):
        ok = put(s, c, j); h = heights(s)
        kk = keys(s, six)
        same = sum(1 for a, b in zip(kk, k6) if a == b)
        low = [p for p in six if h[p] == 4]
        T = set(E.straddler_set(s, tau).tolist())
        rec = dict(word=n, i=i, letter=[c, j], admissible_certain=ok, max_height=int(h.max()),
                   h6_kept=int((h == 6).sum()) == len(six) and kk == k6, h6_same_pose=same, start_h6_now_at_4=len(low))
        if low:
            rec['now_at_4_are_tau_h6'] = set(low) == held
        rec.update(new_h6=sum(1 for p in np.flatnonzero(h == 6).tolist() if p not in sixset), u=len(T),
                   removed=len(S - T), added=len(T - S),
                   added_branches={str(k): v for k, v in sorted(Counter(branch(s, p) for p in T - S).items())})
        rec.update(tail(s, h))
        show('LETTER', **rec)
        assert ok
        S = T
    show('END', word=n, source=source, u=len(S), digest=s.digest())
    if not ends:
        continue
    show('BLOCKERS', word=n, tau=tau, pieces=sorted(S))
    hold = []
    for c in range(600):
        k = inside(s, S, c)
        if k:
            hold.append(dict(cap=c, held=len(k), u=len(E.straddler_set(s, c)), h6_inside=len(inside(s, six, c))))
    show('HOLDERS', word=n, tau=tau, rows=hold)
    for r in hold:
        if r['u'] or r['h6_inside']:
            continue
        vec = []
        for j in range(1, 60):
            t = clone(s)
            vec.append(len(E.straddler_set(t, tau)) if put(t, r['cap'], j) else None)
        low = min(v for v in vec if v is not None)
        show('ITEMS', word=n, cap=r['cap'], tau=tau, u_after_items_1_to_59=vec, lowest=low,
             at_items=[j for j, v in enumerate(vec, 1) if v == low])
