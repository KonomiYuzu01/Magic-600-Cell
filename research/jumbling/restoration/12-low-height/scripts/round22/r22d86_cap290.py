# Theory round 22, line (d), round 6: the integrator's replay of R22D-86. From the state after R22D-75's seven letters
# (1ff0061f...), with cap 290 as the target: the two letters (136, 2), (136, 6), each admissible and certain, keeping
# n6 and every height-6 pose, leaving no piece above height 6 and lowering cap 290's straddler count u. Then, at the
# end state, the caps that hold one of the remaining straddlers of cap 290 whole, which of them are clean, and u after
# each of the 59 items of every clean one, each on its own copy of the state. A letter whose cap holds no straddler of
# cap 290 whole moves none of them, so it cannot lower u; a cap that is not clean has no admissible letter.
# The integrator's script; output in runs/round22/r22d86_cap290.txt. Under ten CPU minutes.
# Research only, headless Linux; exact geometry. It writes no file; theory/seams.py keeps its cache in
# work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d86_cap290.py <dir>/s7302-stop.pkl
import sys, json, resource
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU, (1190, 1200))
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
from sim.state import Classification
from sim.kernel import Evaluator, pullback_form

C = E.sim.get_context(); M = E.R.menu('I_a'); s, _ = load(sys.argv[1], E)
REG = C.regions
cache = {}


def clone(st):
    t = E.sim.State(C, filtered=False, menu=M)
    t.pose_id = st.pose_id.copy(); t._poses = list(st._poses); t._lookup = dict(st._lookup)
    return t


def put(st, c, j):
    o = st.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
    return o.applied and not o.uncertain


def heights(st):
    for p in st._poses:
        if p.key not in cache:
            if p.kidx >= 0:
                cache[p.key] = 0
            else:
                l, r = E.T.factors(p.matrix); assert E.T.dist(r) == 0; cache[p.key] = E.T.dist(l)
    return np.array([cache[p.key] for p in st._poses])[st.pose_id]


def keys(st, ps):
    return [st._poses[int(st.pose_id[p])].key for p in ps]


def inside(st, pieces, c):
    """The given off pieces that lie whole inside cap c, by the simulator's own group and anchor tests."""
    out = []
    cl = Classification(int(c))
    for pid, members in st._groups(np.asarray(sorted(pieces), np.int64)):
        ev = Evaluator(pullback_form(st._pose_matrix(pid), C.data.N[c]))
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


def report(tag, **kw):
    h = heights(s); assert h.max() <= 6
    print(tag, json.dumps(dict(kw, u290=len(E.straddler_set(s, 290)), n2_n4_n6=[int((h == k).sum()) for k in (2, 4, 6)],
                               off=int((h > 0).sum()), N=E.R.Restorer(s, M).misalignment(), digest=s.digest())), flush=True)


for c, j in [(64, 15), (185, 14), (357, 12), (471, 12), (263, 14), (407, 13), (523, 12)]:
    assert put(s, c, j)
assert s.digest() == '1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81'
h = heights(s); six = np.flatnonzero(h == 6); sixkeys = keys(s, six)
report('START')
for c, j in [(136, 2), (136, 6)]:
    u0 = len(E.straddler_set(s, 290)); assert put(s, c, j)
    h = heights(s)
    assert keys(s, six) == sixkeys and int((h == 6).sum()) == len(six) and len(E.straddler_set(s, 290)) < u0
    report('STEP', letter=[c, j])
strad = E.straddler_set(s, 290).tolist()
holders = [c for c in range(600) if inside(s, strad, c)]
clean = [c for c in holders if not len(E.straddler_set(s, c))]
print('HOLDERS', json.dumps(dict(straddlers=len(strad), holders=holders, clean=clean)), flush=True)
for c in clean:
    vec = []
    for j in range(1, 60):
        t = clone(s)
        vec.append(len(E.straddler_set(t, 290)) if put(t, c, j) else None)
    print('ITEMS', json.dumps(dict(cap=c, u290_after_items_1_to_59=vec, lowest=min(v for v in vec if v is not None))), flush=True)
