# Theory round 22, line (d), round 6, cloud line height4-descent-bbef1c8e, step 4: two-letter U steps at a state where
# H4-unlock (h4_unlock.py) is stuck. Reads a snapshot (h4_unlock.py --save-end; digest checked by --expect), recomputes
# H4-unlock's target tau and u, and enumerates pairs (first letter, second letter):
#   first letter (c, j): c clean at the state (no straddler), no height-4 piece inside c, j in 1..59; it must be
#     admissible and certain, leave no piece above height 4 and keep n4 and every height-4 pose;
#   second letter (h, i) at the intermediate state t, with the same conditions; the pair is a success when u after it
#     is strictly below u before the first letter.
# Order of first letters: class A (caps holding whole a straddler of some blocked holder of a straddler of tau, i.e.
# letters that can free such a holder) first, then class B (the remaining clean caps), each in (grip, item) order.
# Exact prune of second letters (argument in RESULT.md): only pieces inside a letter's cap move, so a second cap h can
# lower u only if it holds a straddler of tau whole at t; and when h is clean at both states and holds the same pieces
# in the same poses at t as at the state, the pair's u equals u(t) + (u after (h, i) alone) - u, which is not below u
# because the single-letter U family is exhausted and u(t) >= u. So the second letters tried are those at caps that
# hold a straddler of tau whole at t, are clean at t, hold no height-4 piece at t, and were blocked at the state or
# changed content (pieces or poses inside). Every second letter at such a cap is applied on a copy.
# Prints the context (tau, u, straddlers, their holders with clean flags, the blocked holders with their straddler
# counts, class A caps), one FIRST line per first letter (valid, u(t), freed blocked holders, second caps tried,
# successes), and the end (DONE or BUDGET with the index reached). --shard i --nshards n takes the first letters whose
# index in the enumeration order (class A, then class B) is i mod n, so shards can run concurrently. The chosen pair
# is read off all shards' FIRST lines: the first success whose first letter frees a blocked holder (class A, in
# (grip, item) order), else the first success among the remaining first letters in (grip, item) order, the second
# letter in (grip, item) order.
# Research only, headless Linux; exact geometry. It writes no file; theory/seams.py keeps its cache in
# work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Run from the repository root (one shard of three):
#   OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=<file> python3 -I -B -u \
#     research/jumbling/restoration/12-low-height/scripts/round22/cloud/height4-descent-bbef1c8e/two_letter_u.py \
#     --snapshot <dir>/h4-end-1.pkl --expect 6999b9214d6ea25486daf67e650773babff08f1a41742acfddd6eec2f88787ab \
#     --shard 0 --nshards 3 --cpu 12000
import sys, json, resource, time, argparse
import numpy as np
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
from sim.state import Classification
from sim.kernel import Evaluator, pullback_form

ap = argparse.ArgumentParser()
ap.add_argument('--snapshot', required=True); ap.add_argument('--expect')
ap.add_argument('--shard', type=int, default=0); ap.add_argument('--nshards', type=int, default=1)
ap.add_argument('--cpu', type=float, default=12000.0)
A = ap.parse_args()
resource.setrlimit(resource.RLIMIT_CPU, (int(A.cpu) + 1190, int(A.cpu) + 1200))
C = E.sim.get_context(); M = E.R.menu('I_a'); REG = C.regions
s, _ = load(A.snapshot, E)
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


def put(st, c, j):
    o = st.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
    return o.applied and not o.uncertain


def info(p):
    if p.kidx >= 0:
        return 0, -1, None
    if p.key not in cache:
        l, r = E.T.factors(p.matrix); assert E.T.dist(r) == 0
        cache[p.key] = (E.T.dist(l), E.T.branch(l), l)
    return cache[p.key]


def heights(st):
    return np.array([info(p)[0] for p in st._poses])[st.pose_id]


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
    """'in', 'out' or 'straddle' for each given off piece at the cut of grip c (h4_unlock.py's labels())."""
    out = {}
    if not pieces:
        return out
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
    """h4_unlock.py's target(): (tau, u, table)."""
    h = heights(st); four = np.flatnonzero(h == 4).tolist()
    table = []
    for c in range(600):
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
    tau = min(table, key=lambda ck: (-ck[1], ck[0]))[0]
    return tau, len(E.straddler_set(st, tau)), table


def caps(st):
    """For every cap: (clean, inside pieces) by the simulator's classification (inside only for clean caps)."""
    out = []
    for c in range(600):
        cl = st.classify(c, need_inside=True, stop_at_straddle=True); assert not cl.uncertain
        out.append((cl.straddle is None, np.sort(np.asarray(cl.inside)) if cl.straddle is None else None))
    return out


def ok(t, four, fourkeys):
    h = heights(t)
    return h.max() <= 4 and int((h == 4).sum()) == len(four) and keys(t, four) == fourkeys


h = heights(s); assert h.max() <= 4
four = np.flatnonzero(h == 4).tolist(); fourkeys = keys(s, four); fourset = set(four)
tau, u0, table = target(s)
strad = E.straddler_set(s, tau).tolist()
S0 = caps(s)
holders = [c for c in range(600) if any(v == 'in' for v in labels(s, strad, c).values())]
blocked = [c for c in holders if not S0[c][0]]
bstrad = {c: E.straddler_set(s, c).tolist() for c in blocked}
first_caps = [c for c in range(600) if S0[c][0] and not (set(S0[c][1].tolist()) & fourset)]
allb = sorted(set(p for v in bstrad.values() for p in v))
classA = [c for c in first_caps if any(v == 'in' for v in labels(s, allb, c).values())]
classB = [c for c in first_caps if c not in classA]
order = [(c, j) for c in classA for j in range(1, 60)] + [(c, j) for c in classB for j in range(1, 60)]
print('CONTEXT', json.dumps(dict(digest=s.digest(), tau=tau, u=u0, straddlers=strad, targets=table,
                                 holders=[[c, S0[c][0]] for c in holders],
                                 blocked_holders={str(c): len(v) for c, v in bstrad.items()},
                                 blocked_holder_straddlers=len(allb), clean_caps=sum(x[0] for x in S0),
                                 first_letter_caps=first_caps, classA=classA, first_letters=len(order),
                                 shard=A.shard, nshards=A.nshards, cpu=round(time.process_time() - T, 1))), flush=True)
done = 0
try:
    for k, (c, j) in enumerate(order):
        if k % A.nshards != A.shard:
            continue
        checktime()
        t = clone(s)
        rec = dict(index=k, cls='A' if c in classA else 'B', first=[c, j])
        if not put(t, c, j) or not ok(t, four, fourkeys):
            rec['valid'] = False
            print('FIRST', json.dumps(rec), flush=True); done += 1
            continue
        st = E.straddler_set(t, tau).tolist(); rec.update(valid=True, u_t=len(st))
        rec['freed'] = [b for b in blocked if not len(E.straddler_set(t, b))] if rec['cls'] == 'A' else None
        tried, wins = [], []
        for hcap in range(600):
            if not st:
                break
            if not any(v == 'in' for v in labels(t, st, hcap).values()):
                continue
            cl = t.classify(hcap, need_inside=True, stop_at_straddle=True); assert not cl.uncertain
            if cl.straddle is not None:
                continue
            ins = np.sort(np.asarray(cl.inside))
            if set(ins.tolist()) & fourset:
                continue
            was_clean, ins0 = S0[hcap]
            if was_clean and np.array_equal(ins0, ins) and np.array_equal(s.pose_id[ins], t.pose_id[ins]):
                continue                                       # unchanged clean cap: exact prune
            vec = []
            for i in range(1, 60):
                checktime()
                t2 = clone(t)
                if not put(t2, hcap, i) or not ok(t2, four, fourkeys):
                    vec.append(None); continue
                v = len(E.straddler_set(t2, tau)); vec.append(v)
                if v < u0:
                    wins.append([hcap, i, v])
            tried.append(dict(cap=hcap, u_after_items_1_to_59=vec))
        rec.update(second_caps=tried, successes=wins)
        print('FIRST', json.dumps(rec), flush=True); done += 1
    print('DONE', json.dumps(dict(first_letters_done=done, cpu=round(time.process_time() - T, 1))), flush=True)
except Budget:
    print('BUDGET', json.dumps(dict(first_letters_done=done, last_index=k, cpu=round(time.process_time() - T, 1))), flush=True)
