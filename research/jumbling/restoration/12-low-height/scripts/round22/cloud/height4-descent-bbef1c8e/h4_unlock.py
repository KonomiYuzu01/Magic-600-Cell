# Theory round 22, line (d), round 6, cloud line height4-descent-bbef1c8e, step 3: the height-4 unlock rule H4-unlock,
# R22D-81's unlock rule one level down. A copy of the integrator's draft r22d81_rule.py (sha256 in RESULT.md), changed
# only where the level shift needs it. It reads one state (a snapshot written by r22d31_snapshots.py or by this
# script's --save-end, or a census record), optionally plays a prefix of letters (each admissible and certain;
# --expect checks the digest after it), requires that no piece is above height 4, and applies the rule in its stated
# order until no piece is at height 4.
# Target: among the caps that hold a height-4 piece whole and no height-4 piece of another branch, k(c) = the most
# height-4 pieces one item lowers to height 2 (exact quaternion products); tau = the cap minimising (-k, c); u = the
# number of straddlers of tau. Families in order, letters in (grip, item) order, each admissible, certain and with no
# piece above height 4: L4 lowers n4; U keeps n4 and every height-4 pose and strictly lowers u; L2 keeps n4 and every
# height-4 pose, does not raise u and lowers n2. R22D-75's exact count prediction with n6 = 0 is computed for every
# played letter and checked against the simulator after it; U and the guards of L2 apply the letter to a copy of the
# state. Potential n4*B^2 + u*B + n2 with B = 177,121 (termination argument in RESULT.md).
# Prints the target table at the start and after every L4 letter, the u value of every item of each cap the U family
# tries, each step, and the end: HANDOFF (n4 = 0: only heights 0 and 2 left), STUCK (all three families exhausted;
# the target table is printed again) or BUDGET (a stop past --cpu CPU seconds after the start). --save-end <pkl>
# saves the end state as a snapshot (load() format, with the letters played) for a later run with --snapshot.
# Changes against r22d81_rule.py: heights 6/4 -> 4/2 throughout (target, families, guards, handoff); the families
# renamed L4, U, L2 (H6 dropped: no piece is above height 4); the count prediction kept per cap for every item and
# checked after every letter, U included (a retained item, in K+, is predicted to keep every count); the target table printed at STUCK; checktime() inside target(); --save-end;
# the letters played since 015d839e kept in the saved snapshot and printed at the end. Containment tests are the draft's (the simulator's own group
# and anchor tests, restricted to the given pieces).
# Research only, headless Linux; exact geometry. It writes no file unless --save or --save-end names one;
# theory/seams.py keeps its cache in work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Run from the repository root, from the end of step 2 (bbef1c8e... when step 2 plays no letter):
#   OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=<file> python3 -I -B -u \
#     research/jumbling/restoration/12-low-height/scripts/round22/cloud/height4-descent-bbef1c8e/h4_unlock.py \
#     --snapshot <dir>/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12],[124,2],[274,13],[46,1],[46,3],[104,14]]' \
#     --expect bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299 --cpu 14400 --save-end <dir>/h4-end-1.pkl
# Continue: --snapshot <dir>/h4-end-1.pkl --expect <its digest> --cpu <seconds> --save-end <dir>/h4-end-2.pkl
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
ap.add_argument('--cpu', type=float, default=1500.0); ap.add_argument('--save-end')
A = ap.parse_args()
resource.setrlimit(resource.RLIMIT_CPU, (int(A.cpu) + 5390, int(A.cpu) + 5400))   # room for a 3000-twist replay
C = E.sim.get_context(); M = E.R.menu('I_a')
if A.snapshot:
    s, meta = load(A.snapshot, E)
    played = list(pickle.load(open(A.snapshot, 'rb')).get('letters', []))   # letters since the stop 015d839e
else:
    rec = json.loads(Path(A.record).read_text())
    s = E.sim.State(C, filtered=False, menu=M)
    for c, j in rec['history'] + rec['moves']:
        o = s.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
        assert o.applied and not o.uncertain
    assert s.off_lattice_count() == rec['off_now']
    played = []
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
    assert put(s, c, j); played.append([c, j])
if A.expect:
    assert s.digest() == A.expect, s.digest()
T = time.process_time()
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
    h = heights(st); assert h.max() <= 4
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
    h = heights(st); four = np.flatnonzero(h == 4).tolist()
    if not four:
        return None, 0, []
    table = []
    for c in range(600):
        checktime()
        lab = labels(st, four, c)
        ins = [p for p in four if lab[p] == 'in']
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
            k = max(k, sum(E.T.dist(E.T.qmul(l, q)) == 2 for _, _, q in inf))
        if k > 0:
            table.append((c, k))
    if not table:
        return None, 0, []
    tau = min(table, key=lambda ck: (-ck[1], ck[0]))[0]
    return tau, len(E.straddler_set(st, tau)), table


def rows(st, memo, ct):
    """R22D-75's exact candidates per clean cap: (item, predicted counts, tier), tier 0/1/2 = n6/n4/n2 lowered first;
    memo[c] = (inside, candidates, predicted counts of every item), None for a blocked cap."""
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
        ans = []; pred = {}
        for j in range(1, 60):                                 # the prediction of every item, for the after-letter check
            g, l = item(c, j)
            if l is None:                                      # a retained item (in K+) keeps every height
                pred[j] = list(phi); continue
            r = {2: 0, 4: 0, 6: 0}
            for hh, q, n in ops:
                new = E.T.dist(E.T.qmul(l, q)); assert new in (hh, hh - 2)
                if new == hh - 2:
                    r[hh] += n
            pred[j] = [v + d for v, d in zip(phi, [foreign[4] - r[6], foreign[2] + r[6] - foreign[4] - r[4], lat + r[4] - foreign[2] - r[2]])]
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
        memo[c] = (cl.inside, ans, pred)
        return memo[c]
    return go


def report(tag, **kw):
    v = counts(s)
    print(tag, json.dumps(dict(kw, digest=s.digest(), n2_n4_n6=[v[2], v[1], v[0]], off=sum(v),
                               N=E.R.Restorer(s, M).misalignment(), cpu=round(time.process_time() - T, 1))), flush=True)


def guarded(st, c, j, four, fourkeys, tau, u, need):
    """Apply (c, j) to a copy: admissible and certain, no height above 4, n4 and every height-4 pose kept, then
    u strictly lower (need 'lower') or not higher (need 'keep'). Returns (ok, new u)."""
    t = clone(st)
    if not put(t, c, j):
        return False, None
    h = heights(t)
    if h.max() > 4 or int((h == 4).sum()) != len(four) or keys(t, four) != fourkeys:
        return False, None
    v = len(E.straddler_set(t, tau)) if tau is not None else 0
    return (v < u) if need == 'lower' else (v <= u), v


def save_end():
    if A.save_end:
        pickle.dump({'record': 'h4_unlock end', 'history': 0, 'moves': len(played), 'letters': played, 'digest': s.digest(),
                     'off': int(s.off_lattice_count()), 'pose_id': s.pose_id.copy(),
                     'poses': [(p.kidx, p.matrix, p.key) for p in s._poses]}, open(A.save_end, 'wb'), protocol=5)
        load(A.save_end, E)


steps = 0; fams = Counter(); tau = u = None; table = []
try:
    tau, u, table = target(s)
    report('START', tau=tau, u=u, targets=table)
    while counts(s)[1] > 0:
        memo = {}; ct = Counter(); go = rows(s, memo, ct)
        four = np.flatnonzero(heights(s) == 4).tolist(); fourkeys = keys(s, four); fourset = set(four)
        chosen = None
        for c in range(600):                                   # L4
            r = go(c)
            if r is not None:
                hit = next(((j, out) for j, out, k in r[1] if k == 1), None)
                if hit:
                    chosen = ('L4', c, hit[0]); break
        if chosen is None and tau is not None:                 # U
            strad = E.straddler_set(s, tau).tolist()
            for c in range(600):
                checktime()
                r = go(c)
                if r is None:
                    continue
                lab = labels(s, set(strad) | fourset, c)
                if not any(lab[p] == 'in' for p in strad) or any(lab[p] == 'in' for p in four):
                    continue
                vec = []
                for j in range(1, 60):
                    ok, v = guarded(s, c, j, four, fourkeys, tau, u, 'lower')
                    vec.append(v)
                    if ok and chosen is None:
                        chosen = ('U', c, j)
                print('U_ITEMS', json.dumps({'cap': c, 'tau': tau, 'u': u, 'u_after_items_1_to_59': vec}), flush=True)
                if chosen is not None:
                    break
        if chosen is None:                                     # L2
            for c in range(600):
                r = go(c)
                if r is None:
                    continue
                for j, out, k in r[1]:
                    if k != 2:
                        continue
                    if any(p in fourset for p in r[0].tolist()):
                        continue
                    ok, v = guarded(s, c, j, four, fourkeys, tau, u, 'keep')
                    if ok:
                        chosen = ('L2', c, j); break
                if chosen is not None:
                    break
        if chosen is None:
            report('STUCK', steps=steps, tau=tau, u=u, targets=table, families=dict(fams), checks=dict(ct)); break
        fam, c, j = chosen
        pred = go(c)[2][j]
        before = s.digest(); assert put(s, c, j); played.append([c, j])
        assert counts(s) == pred, (counts(s), pred)
        steps += 1; fams[fam] += 1
        if fam == 'L4':
            tau, u, table = target(s)
        else:
            u = len(E.straddler_set(s, tau))
        report('STEP', step=steps, family=fam, letter=[c, j], input_digest=before, tau=tau, u=u,
               targets=table if fam == 'L4' else None, checks=dict(ct))
    else:
        report('HANDOFF', steps=steps, families=dict(fams))
except Budget:
    report('BUDGET', steps=steps, tau=tau, u=u, families=dict(fams), limit_cpu=A.cpu)
save_end()
print('LETTERS', json.dumps({'letters_since_015d839e': played}), flush=True)
