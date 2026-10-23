# Theory round 22, line (d), round 6: the deciding experiment of the joint attack on the unlocking of caps 311 and 290.
# It reads one state (a snapshot written by r22d31_snapshots.py or by r22d81_rule.py --save), optionally plays a prefix
# of letters (each admissible and certain; --expect checks the digest after it), fixes the target cap tau (--target),
# and lowers u, the number of off pieces that straddle the cut of tau, with two families in order until u = 0:
#   U   one letter that lowers u;
#   U2  two letters (c, g) then (d, h), d != c, that end strictly below u; the middle state may have any u.
# Every letter is admissible and certain, and every prefix keeps every height at most 6, the number of height-6 pieces
# and every height-6 pose. Letters are tried in (grip, item) order and pairs in (c, g, d, h) order; the first that
# qualifies is played. A step is one letter (U) or two (U2); every step prints its length.
# Pruning, each exact: a first letter at a cap that holds a height-6 piece is skipped (it would move it); a U letter's
# cap holds a straddler of tau whole; a U2 second cap d meets c inside the ball (A-R22J-08: otherwise the two letters
# commute, their changes of u add and one of them alone is a U letter), every piece that straddles d before the pair
# moves with the first letter, and d holds more straddlers of tau in the middle state than the first letter added
# (delta), counted first from an upper bound and then exactly. --no-meet-prune turns the meet test off for validation.
# Candidates are evaluated on a copy that moves the inside pieces with the simulator's own composition and labels the
# moved pieces with the simulator's own tests; heights are predicted from left factors (as in R22D-75). Every played
# letter is replayed with the simulator's apply, and its heights and u are checked against the prediction. --selfcheck
# also compares the copy's u and guard with a full apply for every item of the first cap the U family tries.
# Ends: UNLOCKED (u = 0; it also prints, for each item of tau, how many height-6 pieces held by tau it lowers to height
# 4), STUCK (neither family has a step; it prints the smallest end u of any pair it evaluated at that last state),
# BUDGET (past --cpu CPU seconds after the load and the prefix, default 3,000) or NOT_APPLICABLE (a piece above height
# 6 at the start). The integrator's script; outputs in runs/round22/: r22d81_pairs_cap311_tail.txt for the command
# below, r22d81_pairs_cap311.txt for cap 311 after R22D-75's seven letters, and r22d81_pairs_cap290.txt for cap 290
# after them and (136, 2), (136, 6). The first and the third were made with the revision before the fix of SU2-01,
# which changes no step. Research only, headless Linux; exact geometry. It writes no file; theory/seams.py keeps its
# cache in work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Run from the repository root, for the stall of cap 311 (about ten CPU minutes after the load and the prefix):
#   OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d81_pairs.py \
#     --snapshot <dir>/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12],[185,1],[263,1],[263,3]]' \
#     --target 311 --selfcheck
import sys, json, resource, time, argparse
from collections import Counter
import numpy as np
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
from sim.state import Classification
from sim.kernel import Evaluator, pullback_form
from exact import dot, Q5

ap = argparse.ArgumentParser()
ap.add_argument('--snapshot', required=True)
ap.add_argument('--prefix', default='[]'); ap.add_argument('--expect')
ap.add_argument('--target', type=int, required=True)
ap.add_argument('--cpu', type=float, default=3000.0)
ap.add_argument('--no-meet-prune', action='store_true')
ap.add_argument('--selfcheck', action='store_true')
A = ap.parse_args()
C = E.sim.get_context(); M = E.R.menu('I_a'); REG = C.regions; D = C.data
s, _ = load(A.snapshot, E)
twists = {}


def twist(c, j):
    if (c, j) not in twists:
        twists[c, j] = E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name})
    return twists[c, j]


def put(st, c, j):
    o = st.apply(twist(c, j), journal=False)
    return o.applied and not o.uncertain


for c, j in json.loads(A.prefix):
    assert put(s, c, j)
if A.expect:
    assert s.digest() == A.expect, s.digest()
T = time.process_time()
soft = int(T + A.cpu) + 600                                    # the budget test stops first; this limit is a backstop
resource.setrlimit(resource.RLIMIT_CPU, (soft, soft + 10))
tau = A.target
R2 = max(dot(v, v) for vs in REG.rep_vertices for v in vs)
assert (R2 - Q5(96, -32)).sign() == 0                       # 96 - 32 sqrt 5
cache, gcache, meets = {}, {}, {}


class Budget(Exception):
    pass


def checktime():
    if time.process_time() - T > A.cpu:
        raise Budget


def meet(c, d):
    """Caps c and d meet inside the ball (A-R22J-03): (NN + n_c . n_d) R^2 >= 2 KAPPA^2."""
    if (c, d) not in meets:
        meets[c, d] = meets[d, c] = ((D.NN + dot(D.N[c], D.N[d])) * R2 - 2 * D.KAPPA * D.KAPPA).sign() >= 0
    return meets[c, d]


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


def item(c, j):
    if (c, j) not in gcache:
        g = E.R.element(M, c, j)
        gcache[c, j] = None if C.kplus.index_of_matrix(g) >= 0 else E.T.factors(g)[0]
    return gcache[c, j]


def clone(st):
    t = E.sim.State(C, filtered=False, menu=M)
    t.pose_id = st.pose_id.copy(); t._poses = list(st._poses); t._lookup = dict(st._lookup)
    return t


def labels(st, pieces, c):
    """'in', 'out' or 'straddle' for each given off piece at the cut of grip c, by the simulator's own tests."""
    out = {}
    cl = Classification(int(c))
    if not len(pieces):
        return out
    for pid, members in st._groups(np.asarray(sorted(pieces), np.int64)):
        ev = Evaluator(pullback_form(st._pose_matrix(pid), D.N[c]))
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


def move(st, c, j, inside):
    """A copy of st with letter (c, j) applied to the certified inside pieces (no admissibility test here), and the
    predicted largest new height of a moved piece. The copy is not compacted; its poses are those apply would give."""
    t = clone(st); tw = twist(c, j); l = item(c, j); top = 0
    if inside.size:
        uniq, inv = np.unique(t.pose_id[inside], return_inverse=True)
        new = []
        for q in uniq.tolist():
            h, b, lq = info(st._poses[q])
            if l is not None:
                h = E.T.dist(l) if lq is None else E.T.dist(E.T.qmul(l, lq))
            top = max(top, h)
            new.append(t._compose(tw, q))
        t.pose_id[inside] = np.array(new, np.int32)[inv]
    return t, top


def stradded(st, pieces):
    """The off pieces among the given ones that straddle the cut of tau in st."""
    off = [p for p in pieces if st._poses[int(st.pose_id[p])].kidx < 0]
    lab = labels(st, off, tau)
    return {p for p in off if lab[p] == 'straddle'}


def report(tag, **kw):
    v = counts(s)
    print(tag, json.dumps(dict(kw, tau=tau, digest=s.digest(), n2_n4_n6=[v[2], v[1], v[0]], off=sum(v),
                               N=E.R.Restorer(s, M).misalignment(), cpu=round(time.process_time() - T, 1))), flush=True)


def play(letters, umid_pred, u_pred, six, sixkeys):
    """Play the letters on s with the simulator's apply, checking the guard at every prefix and the predicted u."""
    seen = []
    for (c, j), want in zip(letters, (umid_pred, u_pred)):
        assert put(s, c, j)
        h = heights(s)
        assert h.max() <= 6 and int((h == 6).sum()) == len(six) and [s._poses[int(s.pose_id[p])].key for p in six] == sixkeys
        seen.append(len(E.straddler_set(s, tau)))
    assert seen[-1] == u_pred and (len(letters) == 1 or seen[0] == umid_pred), (seen, umid_pred, u_pred)
    return seen


steps = 0; selfchecked = not A.selfcheck
if heights(s).max() > 6:
    print('NOT_APPLICABLE', json.dumps({'digest': s.digest(), 'max_height': int(heights(s).max())}), flush=True)
    sys.exit(0)
try:
    S0 = set(E.straddler_set(s, tau).tolist()); u = len(S0)
    report('START', prefix=json.loads(A.prefix), u=u, meet_prune=not A.no_meet_prune)
    while u > 0:
        ct = Counter(); h = heights(s); best_pair = None              # the smallest end u of a pair evaluated at this state
        six = np.flatnonzero(h == 6).tolist(); sixkeys = [s._poses[int(s.pose_id[p])].key for p in six]; sixset = set(six)
        cls = {}
        for c in range(600):
            checktime()
            cls[c] = s.classify(c, need_inside=True, stop_at_straddle=True); assert not cls[c].uncertain
        clean = [c for c in range(600) if cls[c].straddle is None]
        first = [c for c in clean if not sixset.intersection(cls[c].inside.tolist())]
        hold = {d: {p for p, x in labels(s, S0, d).items() if x == 'in'} for d in range(600)}
        full = {}
        chosen = None
        for c in first:                                        # U
            Ic = cls[c].inside; Icset = set(Ic.tolist())
            if not (S0 & Icset):
                continue
            vec = []
            for j in range(1, 60):
                checktime()
                t, top = move(s, c, j, Ic)
                v = None
                if top <= 4:
                    v = len((S0 - Icset) | stradded(t, Ic.tolist()))
                vec.append(v); ct['u_letters'] += 1
                if v is not None and v < u and chosen is None:
                    chosen = ('U', [[c, j]], None, v)
            if not selfchecked:
                full_vec = []
                for j in range(1, 60):
                    t = clone(s); ok = put(t, c, j); hh = heights(t)
                    good = ok and hh.max() <= 6 and int((hh == 6).sum()) == len(six) and [t._poses[int(t.pose_id[p])].key for p in six] == sixkeys
                    full_vec.append(len(E.straddler_set(t, tau)) if good else None)
                assert full_vec == vec, (c, full_vec, vec)
                print('SELFCHECK', json.dumps({'cap': c, 'items': 59, 'agree': True}), flush=True)
                selfchecked = True
            print('U_ITEMS', json.dumps({'cap': c, 'u': u, 'u_after_items_1_to_59': vec}), flush=True)
            if chosen is not None:
                break
        if chosen is None:                                     # U2
            for c in first:
                Ic = cls[c].inside; Icset = set(Ic.tolist())
                cands = [d for d in range(600) if d != c and (A.no_meet_prune or meet(c, d))]
                for j in range(1, 60):
                    checktime()
                    ta, top = move(s, c, j, Ic)
                    if top > 4:
                        ct['first_height_guard'] += 1; continue
                    ct['first_letters'] += 1
                    newA = stradded(ta, Ic.tolist()) if meet(c, tau) else set(); rest = S0 - Icset; Sa = rest | newA; delta = len(Sa) - u
                    for d in cands:
                        if cls[d].straddle is not None:        # d is clean in the middle state only if its blockers move
                            if cls[d].straddle['piece'] not in Icset:
                                continue
                            if d not in full:
                                full[d] = set(E.straddler_set(s, d).tolist())
                            if not full[d] <= Icset:
                                continue
                        if len(rest & hold[d]) + len(newA) <= delta:
                            ct['bound_prunes'] += 1; continue
                        labA = labels(ta, newA, d)
                        if any(x == 'straddle' for x in labA.values()):
                            continue
                        if len(rest & hold[d]) + sum(x == 'in' for x in labA.values()) <= delta:
                            ct['bound_prunes'] += 1; continue
                        cd = ta.classify(d, need_inside=True, stop_at_straddle=True); assert not cd.uncertain
                        ct['second_caps_classified'] += 1
                        if cd.straddle is not None:
                            continue
                        Id = cd.inside; Idset = set(Id.tolist())
                        if sixset & Idset or len(Sa & Idset) <= delta:
                            continue
                        for k in range(1, 60):
                            tb, top2 = move(ta, d, k, Id); ct['pairs'] += 1
                            if top2 > 4:
                                continue
                            v = len((Sa - Idset) | stradded(tb, Id.tolist()))
                            if best_pair is None or v < best_pair[0]:
                                best_pair = (v, [[c, j], [d, k]], len(Sa))
                            if v < u:
                                chosen = ('U2', [[c, j], [d, k]], len(Sa), v); break
                        if chosen is not None:
                            break
                    if chosen is not None:
                        break
                if chosen is not None:
                    break
        if chosen is None:
            report('STUCK', steps=steps, u=u, checks=dict(ct),
                   best_pair=None if best_pair is None else {'u_end': best_pair[0], 'letters': best_pair[1], 'u_mid': best_pair[2]})
            break
        fam, letters, umid, v = chosen
        before = s.digest()
        seen = play(letters, umid, v, six, sixkeys)
        steps += 1; S0 = set(E.straddler_set(s, tau).tolist()); u = len(S0); assert u == v
        report('STEP', step=steps, family=fam, letters=letters, length=len(letters), u_mid=umid, u=u, input_digest=before,
               checks=dict(ct))
    else:
        cl = s.classify(tau, need_inside=True, stop_at_straddle=True); assert cl.straddle is None and not cl.uncertain
        held = [p for p in cl.inside.tolist() if info(s._poses[int(s.pose_id[p])])[0] == 6]
        ls = [info(s._poses[int(s.pose_id[p])])[2] for p in held]
        lowered = [0 if item(tau, j) is None else sum(E.T.dist(E.T.qmul(item(tau, j), q)) == 4 for q in ls) for j in range(1, 60)]
        report('UNLOCKED', steps=steps, held6=len(held), lowered_to_4_by_items_1_to_59=lowered)
except Budget:
    report('BUDGET', steps=steps, limit_cpu=A.cpu)
