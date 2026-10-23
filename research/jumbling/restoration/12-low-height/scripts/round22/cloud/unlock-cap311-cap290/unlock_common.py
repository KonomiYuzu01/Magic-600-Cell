# Theory round 22, line (d), round 6, cloud line unlock-cap311-cap290: shared helpers for cap311_unlock.py and
# prep_two_letters.py. clone, put, heights, keys and inside are copied from the draft r22d86_cap290.py (put returns
# the admissible-and-certain flag instead of asserting); all_labels is copied from the draft r22d82_unlock_graph.py
# (unchanged except for an optional list of caps). New here: strad_among (straddle test of given pieces at
# one cap, the same simulator calls as E.straddler_set restricted to those pieces), snapshot(st) (heights 2/4/6, off,
# N, digest) and Labels, a full in/out/straddle table of every piece at every cap that is updated, after a letter at a
# clean cap, only for the pieces that letter moved (a clean cap moves exactly the pieces it holds whole; every other
# piece keeps its pose and therefore its labels).
# Research only, headless Linux; exact geometry. A module: it writes nothing and is not run by itself.
import sys, json, time, resource
import numpy as np
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
from sim.state import Classification
from sim.kernel import Evaluator, pullback_form

C = E.sim.get_context(); M = E.R.menu('I_a')
REG = C.regions
SEVEN = [(64, 15), (185, 14), (357, 12), (471, 12), (263, 14), (407, 13), (523, 12)]
STOP = '015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f'
AFTER75 = '1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81'
S290 = '9e6b991bf057d3f2c28c3158aed3878b55720c970601019eb819e53bd0601e94'
cache = {}


def cpu():
    r = resource.getrusage(resource.RUSAGE_SELF)
    return round(r.ru_utime + r.ru_stime, 1)


def clone(st):
    t = E.sim.State(C, filtered=False, menu=M)
    t.pose_id = st.pose_id.copy(); t._poses = list(st._poses); t._lookup = dict(st._lookup)
    return t


def put(st, c, j):
    o = st.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
    return bool(o.applied and not o.uncertain)


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


def strad_among(st, pieces, e):
    """The given pieces whose pose straddles the cut of grip e (E.straddler_set restricted to these pieces)."""
    pieces = np.asarray(sorted(pieces), np.int64)
    off = pieces[st._kpose()[st.pose_id[pieces]] < 0] if len(pieces) else pieces
    cl = Classification(int(e)); out = []
    if not len(off):
        return np.array([], np.int64)
    for pid, members in st._groups(off):
        ev = Evaluator(pullback_form(st._pose_matrix(pid), C.data.N[e]))
        for anchor, sub in st._anchors(pid, members):
            r = st._eval(ev, REG.cap_vform(anchor), lambda a=anchor: REG.cap_frows(a), cl, None)
            assert r[0] != 'uncertain'
            if r[0] != 'straddle':
                continue
            for p in sub.tolist():
                r = st._eval(ev, REG.vform(p), lambda q=p: REG.frows(q), cl, None)
                assert r[0] != 'uncertain'
                if r[0] == 'straddle':
                    out.append(p)
    return np.array(sorted(out), np.int64)


def all_labels(st, pieces=None, caps=None):
    """0 out, 1 in, 2 straddle, for the given pieces (all pieces by default) at the given caps (all 600 by default;
    column i of the result is caps[i])."""
    n = len(st.pose_id); pick = np.arange(n) if pieces is None else np.asarray(pieces, np.int64)
    caps = list(range(600)) if caps is None else list(caps)
    out = np.zeros((len(pick), len(caps)), np.uint8); pos = np.full(n, -1, np.int32); pos[pick] = np.arange(len(pick))
    if not len(pick):
        return out
    k = st._kpose()[st.pose_id]; lp = pick[k[pick] >= 0]; op = pick[k[pick] < 0]
    groups = [(pid, st._anchors(pid, mem)) for pid, mem in st._groups(op)] if len(op) else []
    for i, c in enumerate(caps):
        if len(lp):
            out[pos[lp], i] = C.data.in_sig(lp, C.kplus.perms[C.kplus.inv[k[lp]], c])
        for pid, anchors in groups:
            ev = Evaluator(pullback_form(st._pose_matrix(pid), C.data.N[c]))
            for a, sub in anchors:
                lab = ev.classify(C.regions.cap_vform(a))[0]; assert lab != 'uncertain'
                if lab != 'straddle':
                    if lab == 'in':
                        out[pos[sub], i] = 1
                    continue
                for p in sub.tolist():
                    lab = ev.classify(C.regions.vform(p))[0]; assert lab != 'uncertain'; out[pos[p], i] = {'in': 1, 'out': 0, 'straddle': 2}[lab]
    return out


def snapshot(st, h=None):
    h = heights(st) if h is None else h
    return dict(n2_n4_n6=[int((h == k).sum()) for k in (2, 4, 6)], off=int((h > 0).sum()), max_height=int(h.max()),
                N=int(E.R.Restorer(st, M).misalignment()), digest=st.digest())


class Labels:
    """Full label table of a state, with per-cap straddler counts."""

    def __init__(self, st, L=None):
        self.L = all_labels(st) if L is None else L
        self.cnt = (self.L == 2).sum(axis=0)

    def moved(self, c):
        assert self.cnt[c] == 0, 'letters are taken only at clean caps'
        return np.flatnonzero(self.L[:, c] == 1)

    def after(self, t, c):
        """Labels after a letter at clean cap c has been applied to t: (moved pieces, their new rows, new counts)."""
        mv = self.moved(c); rows = all_labels(t, mv)
        cnt = self.cnt - (self.L[mv] == 2).sum(axis=0) + (rows == 2).sum(axis=0)
        return mv, rows, cnt

    def commit(self, mv, rows):
        L = self.L.copy(); L[mv] = rows
        return Labels(None, L)


def blockers_from(L, mv, rows, tau):
    """Straddlers of tau, from a table L and replacement rows for the pieces mv."""
    col = L[:, tau].copy(); col[mv] = rows[:, tau]
    return np.flatnonzero(col == 2)


def emit(tag, **kw):
    print(tag, json.dumps(kw, default=int), flush=True)
