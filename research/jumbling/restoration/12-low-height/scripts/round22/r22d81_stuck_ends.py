# Theory round 22, line (d), round 6: the two stuck ends of R22D-81 at the state after R22D-75's seven letters
# (1ff0061f...). For every cap that holds a height-6 piece whole and no height-6 piece of another branch whole, it
# prints the number held whole, k (the most of them one non-retained item lowers to height 4), the height-6 pieces
# that straddle the cap (first stuck end: U, H4 and H2 keep every height-6 pose, so such a straddler never moves), the
# other-branch height-4 pieces held whole, F4 (second stuck end: at a clean target an item lowers n6 only when it
# lowers more than F4 pieces), and the cap's straddler count u. Rows of caps with a foreign height-6 piece held whole
# are printed with k = 0. Then the caps of each kind with k > 0.
# The integrator's script (first written by a reviewer of R22D-81); output in runs/round22/r22d81_stuck_ends.txt.
# Research only, headless Linux; exact geometry. It writes no file; theory/seams.py keeps its cache in
# work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Run from the repository root (about one CPU minute after the snapshot):
#   OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d81_stuck_ends.py <dir>/s7302-stop.pkl
import sys, json, resource, time
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU, (600, 610))
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
from sim.state import Classification
from sim.kernel import Evaluator, pullback_form
C = E.sim.get_context(); M = E.R.menu('I_a')
s, _ = load(sys.argv[1], E)
for c, j in [[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12]]:
    o = s.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False); assert o.applied and not o.uncertain
assert s.digest() == '1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81'
REG = C.regions
br = [E.T.branch(tuple(n)) for n in C.data.N]
cache = {}
def info(p):
    if p.kidx >= 0: return 0, -1, None
    if p.key not in cache:
        l, r = E.T.factors(p.matrix); cache[p.key] = (E.T.dist(l), E.T.branch(l), l)
    return cache[p.key]
def labels(st, pieces, c):
    out = {}; cl = Classification(int(c))
    for pid, members in st._groups(np.asarray(sorted(pieces), np.int64)):
        ev = Evaluator(pullback_form(st._pose_matrix(pid), C.data.N[c]))
        for anchor, sub in st._anchors(pid, members):
            r = st._eval(ev, REG.cap_vform(anchor), lambda a=anchor: REG.cap_frows(a), cl, None); assert r[0] != 'uncertain'
            if r[0] != 'straddle':
                for p in sub.tolist(): out[p] = r[0]
                continue
            for p in sub.tolist():
                q = st._eval(ev, REG.vform(p), lambda x=p: REG.frows(x), cl, None); assert q[0] != 'uncertain'; out[p] = q[0]
    return out
pinfo = [info(p) for p in s._poses]
h = np.array([pi[0] for pi in pinfo])[s.pose_id]
six = np.flatnonzero(h == 6).tolist(); four = np.flatnonzero(h == 4).tolist()
rows = []
for c in range(600):
    lab = labels(s, six, c)
    ins = [p for p in six if lab[p] == 'in']
    if not ins: continue
    inf = [pinfo[int(s.pose_id[p])] for p in ins]
    foreign6 = any(b != br[c] for _, b, _ in inf)
    st6 = [p for p in six if lab[p] == 'straddle']
    k = 0
    if not foreign6:
        for j in range(1, 60):
            g = E.R.element(M, c, j)
            if C.kplus.index_of_matrix(g) >= 0: continue
            l = E.T.factors(g)[0]
            k = max(k, sum(E.T.dist(E.T.qmul(l, q)) == 4 for _, _, q in inf))
    lab4 = labels(s, four, c)
    F4 = sum(1 for p in four if lab4[p] == 'in' and pinfo[int(s.pose_id[p])][1] != br[c])
    nstrad = len(E.straddler_set(s, c))
    rows.append(dict(cap=c, held6=len(ins), foreign6=foreign6, k=k, strad6=len(st6), F4=F4, u=nstrad))
print('ROWS', json.dumps(rows))
print('STRADDLED_BY_HEIGHT6', json.dumps([r for r in rows if not r['foreign6'] and r['k'] > 0 and r['strad6'] > 0]))
print('F4_AT_LEAST_K', json.dumps([r for r in rows if not r['foreign6'] and r['k'] > 0 and r['F4'] >= r['k']]))
print('CPU', round(time.process_time(), 1))
