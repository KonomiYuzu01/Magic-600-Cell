# Theory round 22, line (d), round 6, cloud line unlock-cap311-cap290, step 3: two-letter preparations (a, b) for a
# target cap tau at a state where the single-letter U family is exhausted (cap 290 at 9e6b991b..., u290 = 38, rebuilt
# from the stop with R22D-75's seven letters and (136, 2), (136, 6); or cap 311 at the stall of cap311_unlock.py).
# A pair is accepted when a is admissible and certain, keeps H6 (n6 unchanged and every height-6 piece at its exact
# pose) and leaves no height above 6 (u may stay or rise), and b is admissible and certain, keeps H6, leaves no height
# above 6 and leaves u strictly below u0, the value before a.
# First letters, in this exact order (only clean caps have admissible letters; each first letter is (cap, item),
# item 1..59):
#   class (i) caps: the clean caps that hold at least one remaining blocker whole, or hold whole every straddler of a
#     blocked cap that holds a remaining blocker whole; in cap order, items 1..59. An item of such a cap is class (i)
#     when, after it, some (blocker, clean cap holding it whole) pair exists that did not exist before it, or a blocked
#     holder has become clean; its seconds are tested at once. Its other items are deferred to class (ii).
#   class (ii): every other clean cap's items and the deferred items, in (cap, item) order, until the CPU budget of the
#     shard (argument, seconds) is spent. Exact cap-level bound used first: a piece that a does not move keeps its pose,
#     so b treats it as b alone would at the start state; hence for b at a cap clean at the start,
#     u(a, b) >= |Y_b| - |in(a) & Y_b|, Y_b being the straddlers of tau after b alone and in(a) the pieces cap a holds
#     whole (the same for all 59 items of a). A class (ii) letter creates no new (blocker, clean holder) pair and makes
#     no blocked holder clean, so its seconds lie at caps that are clean holders at the start; when the bound is >= u0
#     for every such b, all 59 items of cap a are recorded as pruned with the bound, without trials.
# For each first letter a that passes, the seconds b are every item of every clean cap after a that holds a blocker
# after a whole; a cap is skipped (and counted) when it holds whole at most u' - u0 of the u' blockers after a, since
# u after b is at least the number of blockers b does not move. u after a letter at a clean cap is counted as
# (blockers it does not move) + (moved pieces that straddle tau after it), which equals len(E.straddler_set(...));
# every accepted pair is re-checked with E.straddler_set. Labels: the full in/out/straddle table of the start state,
# and for the pieces moved since then, label columns recomputed at the current state when a cap is queried. Optional three-letter words (a, a', b) with a and a' class (i)
# (--three): after each passing class (i) a, the class (i) caps of the state after a, then seconds as above.
# Work is split over shards by first-letter cap: shard k of n takes the caps whose position in the class order is k
# mod n. Research only, headless Linux; exact geometry. It writes <out.json> (every first letter tried with its
# outcome, the number of pairs tried and every accepted pair).
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=<file>
#   python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cloud/unlock-cap311-cap290/prep_two_letters.py
#   <dir>/s7302-stop.pkl <start: stop|after75> <letters c:j,c:j,... or -> <expected digest> <tau> <k/n> <budget s>
#   <out.json> [--three]
import sys, json
sys.path.insert(0, 'research/jumbling/restoration/12-low-height/scripts/round22/cloud/unlock-cap311-cap290')
import numpy as np
from unlock_common import (E, load, SEVEN, STOP, AFTER75, clone, put, heights, keys, strad_among, all_labels,
                           snapshot, Labels, emit, cpu)


class Ctx:
    """A state with its labels: the full table L of the base state, and, for the pieces D moved since the base, label
    columns recomputed on demand at the current state (every other piece keeps its pose and labels)."""

    def __init__(self, s, L, D, tau, six, sixkeys):
        self.s, self.L, self.D, self.tau, self.six, self.sixkeys = s, L, D, tau, six, sixkeys
        self._col = {}
        self.B = np.flatnonzero(self.col(tau) == 2); self.u = len(self.B)
        self.R = self.rows(self.B)
        self.holders = [int(c) for c in np.flatnonzero((self.R == 1).any(axis=0))]
        self.clean_holders = [c for c in self.holders if self.cnt(c) == 0]
        self.blocked_holders = [c for c in self.holders if self.cnt(c) > 0]
        self.pairs = {(int(p), c) for c in self.clean_holders for p in self.B[self.R[:, c] == 1]}

    def col(self, c):
        if c not in self._col:
            v = self.L[:, c].copy()
            if len(self.D):
                v[self.D] = all_labels(self.s, self.D, [c])[:, 0]
            self._col[c] = v
        return self._col[c]

    def cnt(self, c):
        return int((self.col(c) == 2).sum())

    def rows(self, P):
        R = self.L[P].copy()
        if len(self.D) and len(P):
            m = np.isin(P, self.D)
            if m.any():
                R[m] = all_labels(self.s, P[m])
        return R

    def moved(self, c):
        assert self.cnt(c) == 0, 'letters are taken only at clean caps'
        return np.flatnonzero(self.col(c) == 1)

    def class_i_caps(self, clean=None):
        out = set(self.clean_holders)
        for d in self.blocked_holders:
            S = np.flatnonzero(self.col(d) == 2)
            out.update(int(c) for c in np.flatnonzero((self.rows(S) == 1).all(axis=0))
                       if (clean is not None and c in clean) or (clean is None and self.cnt(int(c)) == 0))
        return sorted(out)

    def trial(self, c, j, below=None):
        """Apply (c, j) to a copy: None if not admissible and certain, else (state, keeps H6, max height, moved, u).
        With below given, keeps H6 and max height are computed only when u < below (else None, None)."""
        t = clone(self.s)
        if not put(t, c, j):
            return None
        mv = self.moved(c)
        rest = len(set(self.B.tolist()) - set(mv.tolist()))
        u = rest + len(strad_among(t, mv, self.tau))
        if below is not None and u >= below:
            return t, None, None, mv, u
        h = heights(t)
        keep = int((h == 6).sum()) == len(self.six) and keys(t, self.six) == self.sixkeys
        return t, keep, int(h.max()), mv, u

    def after(self, t, c, mv):
        return Ctx(t, self.L, np.union1d(self.D, mv), self.tau, self.six, self.sixkeys)


def seconds(x, u0, stats):
    """Every b at x (the state after the first letters) that keeps H6, max height <= 6 and u < u0; also the lowest u
    after any admissible b (whatever its H6 status), in stats['last_min_u']."""
    found = []; low = None
    for c in x.clean_holders:
        whole = int((x.R[:, c] == 1).sum())
        if whole <= x.u - u0:
            stats['b_caps_skipped'] += 1; continue
        for j in range(1, 60):
            stats['pairs_tried'] += 1
            r = x.trial(c, j, below=u0)
            if r is None:
                continue
            t, keep, mh, mv, u = r
            low = u if low is None else min(low, u)
            if keep and mh <= 6 and u < u0:
                assert u == len(E.straddler_set(t, x.tau))
                assert np.isin(np.flatnonzero(E.stable_pose(x.s) != E.stable_pose(t)), mv).all()
                found.append(dict(letter=[c, j], u=u, **snapshot(t)))
    stats['last_min_u'] = low
    return found


def first(x, c, j, u0, stats, record, cls, three):
    r = x.trial(c, j)
    if r is None:
        return 'not admissible'
    t, keep, mh, mv, u = r
    if not keep or mh > 6:
        return 'breaks H6' if not keep else 'height above 6'
    y = x.after(t, c, mv); assert y.u == u
    new_pairs = sorted(y.pairs - x.pairs)
    newly_clean = sorted(d for d in x.blocked_holders if y.cnt(d) == 0)
    is_i = bool(new_pairs or newly_clean)
    if cls == 'i' and not is_i:
        return 'defer'
    stats['first_letters_tested'] += 1
    got = seconds(y, u0, stats)
    row = dict(a=[c, j], cls=cls, u_after_a=u, min_u_after_b=stats.pop('last_min_u'), new_pairs=len(new_pairs), newly_clean=newly_clean,
               clean_holders_after_a=y.clean_holders, accepted=got, cpu=cpu())
    emit('FIRST', **row)
    record['firsts'].append(row)
    for g in got:
        emit('PAIR', a=[c, j], b=g['letter'], **{k: v for k, v in g.items() if k != 'letter'})
        record['pairs'].append(dict(a=[c, j], b=g['letter'], **{k: v for k, v in g.items() if k != 'letter'}))
    if three and is_i:
        for c2 in y.class_i_caps():
            for j2 in range(1, 60):
                r2 = y.trial(c2, j2)
                if r2 is None:
                    continue
                t2, keep2, mh2, mv2, u2 = r2
                if not keep2 or mh2 > 6:
                    continue
                z = y.after(t2, c2, mv2)
                if not (sorted(z.pairs - y.pairs) or [d for d in y.blocked_holders if z.cnt(d) == 0]):
                    continue
                stats['triples_first_two'] += 1
                for g in seconds(z, u0, stats):
                    w = dict(a=[c, j], a2=[c2, j2], b=g['letter'], **{k: v for k, v in g.items() if k != 'letter'})
                    emit('TRIPLE', **w); record['triples'].append(w)
    return 'tested'


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]; three = '--three' in sys.argv
    pkl, start, letters, expect, tau, shard, budget, out = args
    tau = int(tau); k, n = map(int, shard.split('/')); budget = float(budget)
    s, _ = load(pkl, E)
    if start == 'after75':
        for c, j in SEVEN:
            assert put(s, c, j)
        assert s.digest() == AFTER75
    else:
        assert start == 'stop' and s.digest() == STOP
    word = [] if letters == '-' else [tuple(map(int, x.split(':'))) for x in letters.split(',')]
    for c, j in word:
        assert put(s, c, j)
    assert s.digest() == expect, s.digest()
    h = heights(s); six = np.flatnonzero(h == 6)
    lab = Labels(s); clean = [int(c) for c in np.flatnonzero(lab.cnt == 0)]
    x = Ctx(s, lab.L, np.array([], np.int64), tau, six, keys(s, six)); u0 = x.u
    assert u0 == len(E.straddler_set(s, tau)) == lab.cnt[tau]
    ci = x.class_i_caps(set(clean))
    rest = [c for c in clean if c not in ci]
    emit('START', tau=tau, u=u0, shard=shard, clean_caps=len(clean), holders=x.holders, clean_holders=x.clean_holders,
         blocked_holders={c: x.cnt(c) for c in x.blocked_holders}, class_i_caps=ci, cpu=cpu(), **snapshot(s, h))
    stats = dict(first_letters_tried=0, last_min_u=None, first_letters_tested=0, pairs_tried=0, b_caps_skipped=0, triples_first_two=0)
    record = dict(start=start, prefix=word, digest=expect, tau=tau, u0=u0, shard=shard, budget=budget, three=three,
                  class_i_caps=ci, firsts=[], pairs=[], triples=[], outcomes={})
    deferred = []
    for c in ci[k::n]:
        for j in range(1, 60):
            stats['first_letters_tried'] += 1
            o = first(x, c, j, u0, stats, record, 'i', three)
            record['outcomes'][f'{c}:{j}'] = o
            if o == 'defer':
                deferred.append((c, j))
    emit('CLASS_I_DONE', stats=stats, cpu=cpu())
    record['class_i_cpu'] = cpu()
    Y = {}
    for cb in x.clean_holders:
        for jb in range(1, 60):
            t = clone(s)
            if put(t, cb, jb):
                Y[(cb, jb)] = set(E.straddler_set(t, tau).tolist())
    bounds = {}
    for c in rest[k::n]:
        ina = set(np.flatnonzero(lab.L[:, c] == 1).tolist())
        bounds[c] = min((len(v) - len(v & ina) for v in Y.values()), default=None)
    pruned = [c for c in rest[k::n] if bounds[c] is None or bounds[c] >= u0]
    for c in pruned:
        for j in range(1, 60):
            record['outcomes'][f'{c}:{j}'] = f'ii pruned (bound {bounds[c]})'
    stats['first_letters_pruned'] = 59 * len(pruned)
    emit('BOUND', b_letters=len(Y), bounds=bounds, pruned_caps=pruned, cpu=cpu())
    record['class_ii_bounds'] = bounds; record['class_ii_pruned_caps'] = pruned
    order = sorted(deferred + [(c, j) for c in rest[k::n] if c not in pruned for j in range(1, 60)])
    t0 = cpu(); done = 0
    for c, j in order:
        if cpu() - t0 > budget:
            break
        stats['first_letters_tried'] += 1
        o = first(x, c, j, u0, stats, record, 'ii', False)
        record['outcomes'][f'{c}:{j}'] = 'ii ' + o
        done += 1
    record['class_ii'] = dict(listed=len(order), done=done, last=list(order[done - 1]) if done else None,
                              complete=done == len(order))
    record['stats'] = stats; record['cpu'] = cpu()
    json.dump(record, open(out, 'w'), indent=1, default=int)
    emit('END', stats=stats, class_ii=record['class_ii'], pairs=len(record['pairs']), triples=len(record['triples']), cpu=cpu())


main()
