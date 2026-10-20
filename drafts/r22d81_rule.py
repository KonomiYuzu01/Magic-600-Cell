# Theory round 22, line (d), round 6: the integrator's implementation of the unlock rule of R22D-81. It reads one state
# (a snapshot written by r22d31_snapshots.py or by this script's --save, or a census record rebuilt from its history
# and all descent twists, every twist admissible and certain), optionally plays a prefix of letters (each admissible and
# certain; --expect checks the digest after it), and applies the rule in its stated order until no piece is at
# height 6. Target: among the caps that hold a height-6 piece whole and hold no height-6 piece of another branch whole,
# the one with the most height-6 pieces one item lowers to height 4 (ties by cap index); u is its number of straddlers.
# With no such cap the target is absent and u = 0.
# Families in order, letters in (grip, item) order, each admissible, certain and with no piece above height 6: H6
# lowers n6; U keeps n6 and every height-6 pose and lowers u; H4 keeps n6 and every height-6 pose, does not raise u and
# lowers n4; H2 keeps n6, n4 and every height-6 pose, does not raise u and lowers n2. Every step is one letter (length
# 1). H6, H4 and H2 use R22D-75's exact count prediction, checked against the simulator after the letter; U and the
# guards of H4 and H2 apply the letter to a copy of the state. Prints the target table at the start, the u value of
# every item of each cap the U family tries, each step, and the end: handoff (no piece at height 6, so every height at
# most 4), stuck, or a budget stop past --cpu CPU seconds (default 1,500) after the start (with any letter already
# played but not yet printed). Precondition (R22D-75's): every height at most 6; a state with a piece above height 6
# prints NOT_APPLICABLE. A run that prints none of HANDOFF, STUCK, BUDGET and NOT_APPLICABLE has no outcome. --no-target
# is for a differential test only: it keeps the target absent, so the rule plays H6, H4 and H2.
# The integrator's script; outputs in runs/round22/: r22d81_rule.txt for the 3000-twist stop of seed 7302 after
# R22D-75's seven letters (it chooses the five letters of R22D-80), r22d81_from_stop.txt for the stop itself, and
# r22d81_no_target.txt for --no-target from the stop after (64, 15). Containment of single pieces in a cap uses the
# simulator's own group and anchor tests (as eb6.straddler_set does), restricted to those pieces.
# Research only, headless Linux; exact geometry. It writes no file unless --save names one; theory/seams.py keeps its
# cache in work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Run from the repository root, for R22D-80 (about 7 CPU minutes after the snapshot):
#   OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d81_rule.py \
#     --snapshot <dir>/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12]]' \
#     --expect 1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81
# Other states: --record <census record json> [--save <snapshot pkl>] [--prefix <json letters>] [--cpu <seconds>].
import sys, json, resource, time, argparse, pickle
from pathlib import Path
from collections import Counter
import numpy as np
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
from sim.state import Classification
from sim.kernel import Evaluator, pullback_form

ap = argparse.ArgumentParser()
src = ap.add_mutually_exclusive_group(required=True)
src.add_argument('--snapshot'); src.add_argument('--record')
ap.add_argument('--prefix', default='[]'); ap.add_argument('--expect'); ap.add_argument('--save')
ap.add_argument('--cpu', type=float, default=1500.0)
ap.add_argument('--no-target', action='store_true')
A = ap.parse_args()
resource.setrlimit(resource.RLIMIT_CPU, (int(A.cpu) + 5390, int(A.cpu) + 5400))   # room for a 3000-twist replay
C = E.sim.get_context(); M = E.R.menu('I_a')
if A.snapshot:
    s, _ = load(A.snapshot, E)
else:
    rec = json.loads(Path(A.record).read_text())
    s = E.sim.State(C, filtered=False, menu=M)
    for c, j in rec['history'] + rec['moves']:
        o = s.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
        assert o.applied and not o.uncertain
    assert s.off_lattice_count() == rec['off_now']
    if A.save:
        pickle.dump({'record': A.record, 'history': len(rec['history']), 'moves': len(rec['moves']), 'digest': s.digest(),
                     'off': int(s.off_lattice_count()), 'pose_id': s.pose_id.copy(),
                     'poses': [(p.kidx, p.matrix, p.key) for p in s._poses]}, open(A.save, 'wb'), protocol=5)
print('LOADED', json.dumps({'digest': s.digest(), 'off': int(s.off_lattice_count()), 'cpu': round(time.process_time(), 1)}), flush=True)
REG = C.regions


def put(st, c, j):
    o = st.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
    return o.applied and not o.uncertain


for c, j in json.loads(A.prefix):
    assert put(s, c, j)
if A.expect:
    assert s.digest() == A.expect, s.digest()
T = time.process_time()
soft = int(T + A.cpu) + 600                                    # the budget test stops first; this limit is a backstop
resource.setrlimit(resource.RLIMIT_CPU, (soft, max(soft + 10, resource.getrlimit(resource.RLIMIT_CPU)[1])))
br = [E.T.branch(tuple(n)) for n in C.data.N]
cache, gcache = {}, {}


class Budget(Exception):
    pass


def checktime():
    if time.process_time() - T > A.cpu:
        raise Budget


def info(p):
    if p.kidx >= 0:
        return 0, -1, None
    if p.key not in cache:
        l, r = E.T.factors(p.matrix); assert E.T.dist(r) == 0
        cache[p.key] = (E.T.dist(l), E.T.branch(l), l)
    return cache[p.key]


def heights(st):
    return np.array([info(p)[0] for p in st._poses])[st.pose_id]


def counts(st):
    h = heights(st); assert h.max() <= 6
    return [int((h == k).sum()) for k in (6, 4, 2)]


def keys(st, ps):
    return [st._poses[int(st.pose_id[p])].key for p in ps]


def clone(st):
    t = E.sim.State(C, filtered=False, menu=M)
    t.pose_id = st.pose_id.copy(); t._poses = list(st._poses); t._lookup = dict(st._lookup)
    return t


def item(c, j):
    if (c, j) not in gcache:
        g = E.R.element(M, c, j)
        gcache[c, j] = (g, None if C.kplus.index_of_matrix(g) >= 0 else E.T.factors(g)[0])
    return gcache[c, j]


def labels(st, pieces, c):
    """'in', 'out' or 'straddle' for each given off piece at the cut of grip c, by the simulator's own tests."""
    out = {}
    cl = Classification(int(c))
    for pid, members in st._groups(np.asarray(sorted(pieces), np.int64)):
        ev = Evaluator(pullback_form(st._pose_matrix(pid), C.data.N[c]))
        for anchor, sub in st._anchors(pid, members):
            r = st._eval(ev, REG.cap_vform(anchor), lambda a=anchor: REG.cap_frows(a), cl, None)
            assert r[0] != 'uncertain'
            if r[0] != 'straddle':
                for p in sub.tolist():
                    out[p] = r[0]
                continue
            for p in sub.tolist():
                q = st._eval(ev, REG.vform(p), lambda x=p: REG.frows(x), cl, None)
                assert q[0] != 'uncertain'
                out[p] = q[0]
    return out


def target(st):
    """(tau, u, table): table lists (cap, k) for every cap whose k is positive."""
    h = heights(st); six = np.flatnonzero(h == 6).tolist()
    if not six:
        return None, 0, []
    table = []
    for c in range(600):
        checktime()
        lab = labels(st, six, c)
        ins = [p for p in six if lab[p] == 'in']
        if not ins:
            continue
        inf = [info(st._poses[int(st.pose_id[p])]) for p in ins]
        if any(b != br[c] for _, b, _ in inf):
            continue
        k = 0
        for j in range(1, 60):
            g, l = item(c, j)
            if l is None:
                continue
            k = max(k, sum(E.T.dist(E.T.qmul(l, q)) == 4 for _, _, q in inf))
        if k > 0:
            table.append((c, k))
    if not table:
        return None, 0, []
    tau = min(table, key=lambda ck: (-ck[1], ck[0]))[0]
    return tau, len(E.straddler_set(st, tau)), table


def rows(st, memo, ct):
    """R22D-75's exact candidates per clean cap: (item, predicted counts, tier), tier 0/1/2 = n6/n4/n2 lowered first."""
    inf = [info(p) for p in st._poses]; phi = counts(st)
    def go(c):
        if c in memo:
            return memo[c]
        cl = st.classify(c, need_inside=True, stop_at_straddle=True); assert not cl.uncertain
        if cl.straddle is not None:
            memo[c] = None; ct['blocked_caps'] += 1; return None
        ids, ns = np.unique(st.pose_id[cl.inside], return_counts=True); P = br[c]
        own = {2: 0, 4: 0, 6: 0}; foreign = {2: 0, 4: 0, 6: 0}; lat = 0; ops = []
        for i, n in zip(ids.tolist(), ns.tolist()):
            hh, b, l = inf[i]
            if hh == 0:
                lat += n
            elif b == P:
                own[hh] += n; ops.append((hh, l, n))
            else:
                foreign[hh] += n
        ans = []
        if foreign[6] or foreign[4] > own[6] or (foreign[4] == own[6] and (foreign[2] > own[4] or (foreign[2] == own[4] and lat >= own[2]))):
            ct['cap_bound_prunes'] += 1                        # R22D-75's bound: no item of this cap lowers the counts
        else:
            for j in range(1, 60):
                checktime()
                g, l = item(c, j)
                if l is None:
                    continue
                r = {2: 0, 4: 0, 6: 0}
                for hh, q, n in ops:
                    new = E.T.dist(E.T.qmul(l, q)); assert new in (hh, hh - 2)
                    if new == hh - 2:
                        r[hh] += n
                delta = [foreign[4] - r[6], foreign[2] + r[6] - foreign[4] - r[4], lat + r[4] - foreign[2] - r[2]]
                out = [v + d for v, d in zip(phi, delta)]
                if out < phi:
                    ans.append((j, out, next(k for k in range(3) if out[k] != phi[k])))
        memo[c] = (cl.inside, ans)
        return memo[c]
    return go


def report(tag, **kw):
    v = counts(s)
    print(tag, json.dumps(dict(kw, digest=s.digest(), n2_n4_n6=[v[2], v[1], v[0]], off=sum(v),
                               N=E.R.Restorer(s, M).misalignment(), cpu=round(time.process_time() - T, 1))), flush=True)


def guarded(st, c, j, six, sixkeys, tau, u, need):
    """Apply (c, j) to a copy: admissible and certain, no height above 6, n6 and every height-6 pose kept, then
    u strictly lower (need 'lower') or not higher (need 'keep'). Returns (ok, new u)."""
    t = clone(st)
    if not put(t, c, j):
        return False, None
    h = heights(t)
    if h.max() > 6 or int((h == 6).sum()) != len(six) or keys(t, six) != sixkeys:
        return False, None
    v = len(E.straddler_set(t, tau)) if tau is not None else 0
    return (v < u) if need == 'lower' else (v <= u), v


steps = 0; pending = None                                      # a played letter whose STEP line is not yet printed
if heights(s).max() > 6:
    print('NOT_APPLICABLE', json.dumps({'digest': s.digest(), 'max_height': int(heights(s).max())}), flush=True)
    sys.exit(0)
try:
    tau, u, table = (None, 0, []) if A.no_target else target(s)
    report('START', prefix=json.loads(A.prefix), tau=tau, u=u, targets=table)
    while counts(s)[0] > 0:
        memo = {}; ct = Counter(); go = rows(s, memo, ct)
        six = np.flatnonzero(heights(s) == 6).tolist(); sixkeys = keys(s, six)
        chosen = None
        for c in range(600):                                   # H6
            r = go(c)
            if r is not None:
                hit = next(((j, out) for j, out, k in r[1] if k == 0), None)
                if hit:
                    chosen = ('H6', c, hit[0], hit[1]); break
        if chosen is None and tau is not None:                 # U
            strad = E.straddler_set(s, tau).tolist()
            for c in range(600):
                checktime()
                r = go(c)
                if r is None:
                    continue
                lab = labels(s, set(strad) | set(six), c)
                if not any(lab[p] == 'in' for p in strad) or any(lab[p] == 'in' for p in six):
                    continue
                vec = []
                for j in range(1, 60):
                    checktime()
                    ok, v = guarded(s, c, j, six, sixkeys, tau, u, 'lower')
                    vec.append(v)
                    if ok and chosen is None:
                        chosen = ('U', c, j, None)
                print('U_ITEMS', json.dumps({'cap': c, 'tau': tau, 'u': u, 'u_after_items_1_to_59': vec}), flush=True)
                if chosen is not None:
                    break
        for fam, tier in (('H4', 1), ('H2', 2)):               # H4, then H2
            if chosen is not None:
                break
            for c in range(600):
                r = go(c)
                if r is None:
                    continue
                for j, out, k in r[1]:
                    if k != tier:
                        continue
                    if any(p in set(r[0].tolist()) for p in six):
                        continue
                    ok, v = guarded(s, c, j, six, sixkeys, tau, u, 'keep')
                    if ok:
                        chosen = (fam, c, j, out); break
                if chosen is not None:
                    break
        if chosen is None:
            report('STUCK', steps=steps, tau=tau, u=u, checks=dict(ct)); break
        fam, c, j, pred = chosen
        before = s.digest(); assert put(s, c, j)
        pending = dict(step=steps + 1, family=fam, letter=[c, j], length=1, input_digest=before)
        if pred is not None:
            assert counts(s) == pred
        steps += 1
        if fam == 'H6':
            tau, u, table = (None, 0, []) if A.no_target else target(s)
        else:
            u = len(E.straddler_set(s, tau)) if tau is not None else 0
        report('STEP', step=steps, family=fam, letter=[c, j], length=1, input_digest=before, tau=tau, u=u,
               targets=table if fam == 'H6' else None, checks=dict(ct))
        pending = None
    else:
        report('HANDOFF', steps=steps)
except Budget:
    report('BUDGET', steps=steps, limit_cpu=A.cpu, **({'played_unreported': pending} if pending else {}))
