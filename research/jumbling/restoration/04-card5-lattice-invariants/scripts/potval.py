"""End-to-end validation of an orbit-5 potential on real configurations.
Builds pi (class -> K+ index) with potsolve's machinery for one association, then along random admissible I_a walks
from solved (and on given pose tables) computes V(X): piece -> pi(region) and checks
  (a) V(X) is a bijection onto the orbit slots, equal to the actual slot on lattice pieces;
  (b) every piece's region lies in a class of the refined over-approximation;
  (c) at every applied twist (d, h): the pieces with V in cap d are exactly the orbit pieces inside cap d, and
      V(X') = tau_d(h) o V(X) on them, V unchanged elsewhere.
usage: potval.py <research/jumbling> <kplus orbit> <assoc index> <steps> <seed> [pose tables ...]"""
import sys, json, random, time
from collections import Counter
from pathlib import Path
J, OI, AI, STEPS, SEED = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5])
TABLES = sys.argv[6:]
src = open(Path(__file__).resolve().parent / 'potsolve.py').read()
cut = src.index('reasons = Counter(); sols = []')
g = {'__name__': 'potval', '__file__': str(Path(__file__).resolve().parent / 'potsolve.py')}
sys.argv = ['potsolve.py', J, OI, '/dev/null']
exec(compile(src[:cut], 'potsolve.py', 'exec'), g)
from itertools import product
import numpy as np
R, B, kp, reg, T, Q5 = g['R'], g['B'], g['kp'], g['reg'], g['T'], g['Q5']
solve, opts, caps_order, a_mapping, perms, piece_of, sig, inv = (g[k] for k in (
    'solve', 'opts', 'caps_order', 'a_mapping', 'perms', 'piece_of', 'sig', 'inv'))
combos = list(product(*opts))
assoc = dict(zip(caps_order, combos[AI % len(combos)]))
ok, why, t = solve(assoc)
assert ok, why
print('assoc', assoc, 'classes', len(t), flush=True)
def e_of(dg, j):
    d0 = int(perms[inv[j]][dg])
    return int(perms[j][assoc[d0]])
import sim, wj
import restore as RS
from sim.kplus import matrix_from_json
from exact import matmul
ctx = R.ctx
fixture, M = wj.load(ctx, 'I_a')
o = int(OI)
pieces = piece_of.astype(np.int64)
kof = {int(p): k for k, p in enumerate(pieces.tolist())}
slot_set = set(pieces.tolist())
basis = list(kp.basis)
def kidx_float(mf):
    imgs = []
    for b in basis:
        y = mf @ R.NFu[b]
        imgs.append(R.pole_key[tuple(np.round(y / np.linalg.norm(y), 7) + 0.0)])
    k = kp.index_of_images(np.array([imgs]))[0]
    assert k >= 0
    return int(k)
Kf = {}
def kmat_f(k):
    if k not in Kf:
        Kf[k] = np.array([[float(x) for x in row] for row in kp.matrix(k)])
    return Kf[k]
def region_slot(Mf, p):
    """V value of piece p with pose matrix (float, exact-derived) Mf: returns (slot, class key)."""
    k = kof[p]
    Pf = Mf @ kmat_f(k)
    # left factor of P = [l, r]: l from the float factorization via the quaternion matrix identity
    l = left_factor(Pf)
    i, key = R.canon(l)
    if key not in t:
        return None, key
    lc = R.qf(B.cls[key][0])
    mf = Pf @ R.Lmat(lc).T
    m = kidx_float(mf)
    return int(pieces[kp.compose(m, t[key])]), key
def left_factor(P):
    # P x = l x r^-1; with x = 1: P e0 = l r^-1, x = i: P e1 = l i r^-1 ... recover l up to sign via
    # l = (sum_b P(e_b) conj(e_b) ...) standard: for rotation x -> l x rbar, sum_b P(e_b) * conj(e_b) = 4 l rbar... use
    # A = sum_b P(e_b) e_b^* gives 4 * (l . rbar-part); simpler: solve with the 4x4 'left-right' decomposition
    E = np.eye(4)
    Q = np.zeros(4)
    best = None
    # l is proportional to sum_b P(e_b) * conj(e_b) * ... use known formula: for x -> l x r^{-1}, the matrix
    # sum over b of Lmat(P e_b) Rmat(conj e_b)?  Use eigen-approach: l satisfies P Rmat(r) = Lmat(l); pick r from basis
    for u in R.UNf:
        # candidate r = u (right factors are icosian units up to sign): l = P(x) * r  at x = 1 -> l = P e0 * r
        lc = qmulf(P @ E[0], u)
        if np.allclose(R.Lmat(lc) @ R.Rmat(qconjf(u)), P, atol=1e-9):
            return lc
    raise AssertionError('right factor not an icosian unit')
def qmulf(p, q):
    a0, a1, a2, a3 = p; b0, b1, b2, b3 = q
    return np.array([a0*b0 - a1*b1 - a2*b2 - a3*b3, a0*b1 + a1*b0 + a2*b3 - a3*b2,
                     a0*b2 - a1*b3 + a2*b0 + a3*b1, a0*b3 + a1*b2 - a2*b1 + a3*b0])
def qconjf(q):
    return q * np.array([1, -1, -1, -1])
def V_of(st):
    V = {}
    miss = Counter()
    kpose = st._kpose()
    pid = st.pose_id
    for p in pieces.tolist():
        k = int(kpose[pid[p]])
        if k >= 0:
            V[p] = int(pieces[kp.compose(k, kof[p])])
            continue
        Mf = np.array([[float(x) for x in row] for row in st.pose(p)])
        s, key = region_slot(Mf, p)
        if s is None:
            miss[key] += 1
        V[p] = s
    return V, miss
def tau(dg, h_f):
    """tau_d(h) on the orbit slots of cap dg: dict slot -> slot."""
    out = {}
    for s in pieces.tolist():
        if dg not in sig[s]:
            continue
        j = kof[s]
        e = e_of(dg, j)
        y = h_f @ R.NFu[e]
        e2 = R.pole_key[tuple(np.round(y / np.linalg.norm(y), 7) + 0.0)]
        a = a_mapping(dg, e, e2)
        assert a is not None
        out[s] = int(pieces[kp.compose(a, j)])
    assert sorted(out.values()) == sorted(out), 'tau not a permutation'
    return out
def check_V(V, st, label):
    vals = list(V.values())
    assert None not in vals, (label, 'class outside the refined over-approximation')
    assert len(set(vals)) == len(vals) and set(vals) == slot_set, (label, 'V not a bijection')
rng = random.Random(SEED)
st = sim.State(ctx, menu=M)
V, miss = V_of(st)
check_V(V, st, 'solved')
stats = Counter(); t1 = time.time()
lat = st.lattice_flags()
for step in range(STEPS):
    off = ~st.lattice_flags()
    offo = [p for p in pieces.tolist() if off[p]]
    if offo and rng.random() < 0.7:
        p = rng.choice(offo)
        Mf = np.array([[float(x) for x in row] for row in st.pose(p)])
        cand = [c for c in range(600) if True]
        c = rng.randrange(600)
        # prefer grips near an off orbit piece: a grip of the piece's slot V(p)
        s = V[p]
        c = rng.choice(sorted(sig[s] | set(cand[:0]))) if rng.random() < 0.8 else c
    else:
        c = rng.randrange(600)
    i = rng.randrange(60)
    tw = sim.Twist(ctx, c, RS.element(M, c, i), 'menu', {'menu': M.name})
    cl = st.classify(c, need_inside=True, stop_at_straddle=True)
    if cl.straddle is not None or cl.uncertain:
        stats['blocked'] += 1
        continue
    inside = set(int(x) for x in cl.inside.tolist()) & slot_set
    out = st.apply(tw, journal=False)
    assert out.applied
    stats['applied'] += 1
    hf = np.array([[float(x) for x in row] for row in tw.matrix])
    tz = tau(c, hf)
    V2, miss = V_of(st)
    check_V(V2, st, f'step {step}')
    vin = {p for p, s in V.items() if c in sig[s]}
    assert vin == inside, ('pieces with V in cap differ from pieces inside', len(vin), len(inside))
    for p in pieces.tolist():
        want = tz[V[p]] if p in inside else V[p]
        assert V2[p] == want, ('tau mismatch', step, p)
    V = V2
    stats['max_off'] = max(stats['max_off'], len(offo))
    if stats['applied'] % 25 == 0:
        print(step, dict(stats), 'off orbit pieces', int((~st.lattice_flags())[pieces].sum()),
              'sec', round(time.time() - t1, 1), flush=True)
print('walk done', dict(stats), 'off orbit pieces', int((~st.lattice_flags())[pieces].sum()))
for tb in TABLES:
    doc = json.load(open(tb))
    st2 = sim.State(ctx, menu=M)
    st2._poses = []
    st2._lookup = {}
    from sim.state import Pose
    for m in doc['poses']:
        mm = matrix_from_json(m)
        st2._intern_matrix(mm)
    st2.pose_id = np.asarray(doc['pose_id'], np.int32)
    V, miss = V_of(st2)
    print(Path(tb).name, 'missing classes', sum(miss.values()))
    check_V(V, st2, tb)
    print(Path(tb).name, 'V bijective; off orbit pieces', int((~st2.lattice_flags())[pieces].sum()))
