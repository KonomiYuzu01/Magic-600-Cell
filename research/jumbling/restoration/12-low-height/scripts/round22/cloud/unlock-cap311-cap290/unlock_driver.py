# Theory round 22, line (d), round 6, cloud line unlock-cap311-cap290, steps 3 and 4 iterated: drives the straddler
# count u of a target cap tau to 0 with single letters (family U) and, where they are exhausted, two-letter
# preparations (family prep), then plays the best H6 item of tau. Each round, at the current state:
#   U: the clean caps holding a remaining blocker whole, in cap order, items 1..59; the first letter that keeps H6
#      (n6 unchanged, every height-6 piece at its exact pose), leaves no height above 6 and lowers u is played (as in
#      cap311_unlock.py);
#   prep, when U is exhausted: first letters a in the order of prep_two_letters.py (class (i) caps in cap order, items
#      1..59, an item counting only if it creates a new (blocker, clean holder) pair or makes a blocked holder clean;
#      then class (ii): every other clean cap's items and the class (i) caps' other items in (cap, item) order, with
#      prep_two_letters.py's exact cap-level bound pruning whole caps); a must keep H6 and leave no height above 6;
#      for each a, seconds b over the clean caps holding a blocker whole after a, in cap order, items 1..59 (a cap
#      holding at most u' - u0 blockers whole is skipped). The first pair whose b keeps H6, leaves no height above 6
#      and gives u < u0 (u0 = u before a) is played, a then b.
#   Stop: u = 0 (then every item of tau on a copy; the one lowering n6 most, ties by item, with no height above 6, is
#   played, family H6), or neither U nor prep finds a letter (a stall of exactly the enumerated sets), or the CPU
#   budget (argument, seconds) is spent before a round ends.
# Every letter played is checked with E.straddler_set and the simulator's admissible-and-certain outcome. Labels: the
# full in/out/straddle table of the round's state, with columns recomputed on demand for pieces moved since (as in
# prep_two_letters.py). Research only, headless Linux; exact geometry. It writes <out.json> after every round.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=<file>
#   python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cloud/unlock-cap311-cap290/unlock_driver.py
#   <dir>/s7302-stop.pkl <start: stop|after75> <letters c:j,c:j,... or -> <expected digest> <tau> <budget s> <out.json>
import sys, json
sys.path.insert(0, 'research/jumbling/restoration/12-low-height/scripts/round22/cloud/unlock-cap311-cap290')
import numpy as np
from unlock_common import (E, load, SEVEN, STOP, AFTER75, clone, put, heights, keys, strad_among, all_labels,
                           snapshot, Labels, emit, cpu)


class Ctx:
    """As in prep_two_letters.py: a state, the base table L and the pieces D moved since the base."""

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

    def class_i_caps(self, clean):
        out = set(self.clean_holders)
        for d in self.blocked_holders:
            S = np.flatnonzero(self.col(d) == 2)
            out.update(int(c) for c in np.flatnonzero((self.rows(S) == 1).all(axis=0)) if c in clean)
        return sorted(out)

    def trial(self, c, j, below=None):
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

    def after(self, t, mv):
        return Ctx(t, self.L, np.union1d(self.D, mv), self.tau, self.six, self.sixkeys)


def first_b(x, u0, count):
    for c in x.clean_holders:
        if int((x.R[:, c] == 1).sum()) <= x.u - u0:
            continue
        for j in range(1, 60):
            count['b'] += 1
            r = x.trial(c, j, below=u0)
            if r is not None and r[1] and r[2] <= 6 and r[4] < u0:
                return (c, j), r
    return None, None


def round_(s, tau, six, sixkeys, log):
    """One round at s: returns (letters played with their states) or [] at a stall."""
    lab = Labels(s); clean = set(int(c) for c in np.flatnonzero(lab.cnt == 0))
    x = Ctx(s, lab.L, np.array([], np.int64), tau, six, sixkeys); u0 = x.u
    assert u0 == len(E.straddler_set(s, tau))
    count = dict(u=0, a=0, b=0)
    # U
    for c in x.clean_holders:
        for j in range(1, 60):
            count['u'] += 1
            r = x.trial(c, j, below=u0)
            if r is not None and r[1] and r[2] <= 6 and r[4] < u0:
                log.update(u_tried=count['u'])
                return [('U', (c, j), r)]
    log.update(u_tried=count['u'], u_caps=x.clean_holders)
    # prep, class (i)
    ci = x.class_i_caps(clean); deferred = []
    log['class_i_caps'] = ci
    for c in ci:
        for j in range(1, 60):
            count['a'] += 1
            r = x.trial(c, j)
            if r is None or not r[1] or r[2] > 6:
                continue
            y = x.after(r[0], r[3])
            if not (y.pairs - x.pairs or [d for d in x.blocked_holders if y.cnt(d) == 0]):
                deferred.append((c, j)); continue
            bl, rb = first_b(y, u0, count)
            if bl:
                log.update(counts=count, cls='i')
                return [('prep', (c, j), r), ('prep', bl, rb)]
    # prep, class (ii) with the cap-level bound
    Y = []
    for cb in x.clean_holders:
        for jb in range(1, 60):
            t = clone(s)
            if put(t, cb, jb):
                Y.append(set(E.straddler_set(t, tau).tolist()))
    rest = sorted(clean - set(ci)); pruned = []
    for c in rest:
        ina = set(np.flatnonzero(lab.L[:, c] == 1).tolist())
        if min((len(v) - len(v & ina) for v in Y), default=u0) >= u0:
            pruned.append(c)
    log['class_ii_pruned_caps'] = pruned
    for c, j in sorted(deferred + [(c, j) for c in rest if c not in pruned for j in range(1, 60)]):
        count['a'] += 1
        r = x.trial(c, j)
        if r is None or not r[1] or r[2] > 6:
            continue
        y = x.after(r[0], r[3])
        bl, rb = first_b(y, u0, count)
        if bl:
            log.update(counts=count, cls='ii')
            return [('prep', (c, j), r), ('prep', bl, rb)]
    log.update(counts=count, stall=True, holders=x.holders, clean_holders=x.clean_holders,
               blocked_holders={c: x.cnt(c) for c in x.blocked_holders})
    return []


def main():
    pkl, start, letters, expect, tau, budget, out = sys.argv[1:8]
    tau = int(tau); budget = float(budget)
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
    h0 = heights(s); six = np.flatnonzero(h0 == 6); sixkeys = keys(s, six)
    record = dict(start=start, prefix=word, digest=expect, tau=tau, letters=[], rounds=[])
    emit('START', tau=tau, u=len(E.straddler_set(s, tau)), cpu=cpu(), **snapshot(s, h0))
    done = False
    while True:
        u = len(E.straddler_set(s, tau))
        if u == 0:
            done = True; break
        if cpu() > budget:
            record['budget_spent'] = True; break
        log = dict(u_before=u)
        got = round_(s, tau, six, sixkeys, log)
        log['cpu'] = cpu(); record['rounds'].append(log)
        if not got:
            emit('STALL', **log); break
        for fam, (c, j), r in got:
            before = E.stable_pose(s); assert put(s, c, j) and s.digest() == r[0].digest()
            assert np.isin(np.flatnonzero(before != E.stable_pose(s)), r[3]).all()
            uu = len(E.straddler_set(s, tau)); snap = snapshot(s)
            assert snap['max_height'] <= 6 and keys(s, six) == sixkeys and snap['n2_n4_n6'][2] == len(six)
            row = dict(letter=[c, j], family=fam, u=uu, **snap)
            record['letters'].append(row); emit('STEP', cpu=cpu(), **row)
        assert uu < u
        json.dump(record, open(out, 'w'), indent=1, default=int)
    if done:
        hb = heights(s); n6 = int((hb == 6).sum()); rows = []
        for j in range(1, 60):
            t = clone(s)
            if not put(t, tau, j):
                rows.append(dict(item=j, applied=False)); continue
            h = heights(t)
            rows.append(dict(item=j, applied=True, n6_drop=n6 - int((h == 6).sum()),
                             lattice_taken_off=int(((hb == 0) & (h > 0)).sum()), **snapshot(t, h)))
            emit('H6_ITEM', cap=tau, **rows[-1])
        record['h6_items'] = rows
        ok = [r for r in rows if r['applied'] and r['max_height'] <= 6 and r['n6_drop'] > 0]
        if ok:
            r = max(ok, key=lambda r: (r['n6_drop'], -r['item']))
            assert put(s, tau, r['item']) and s.digest() == r['digest']
            row = dict(letter=[tau, r['item']], family='H6', u=0, n6_drop=r['n6_drop'],
                       lattice_taken_off=r['lattice_taken_off'], **snapshot(s))
            record['letters'].append(row); emit('H6_PLAYED', cpu=cpu(), **row)
        else:
            emit('H6_NONE', cap=tau)
        h = heights(s)
        record['start_height6_now'] = {int(k): int(v) for k, v in zip(*np.unique(h[six], return_counts=True))}
        record['start_lattice_off_now'] = int(((h0 == 0) & (h > 0)).sum())
    record['end'] = snapshot(s); record['done'] = done; record['cpu'] = cpu()
    json.dump(record, open(out, 'w'), indent=1, default=int)
    emit('END', done=done, cpu=cpu(), word=[l['letter'] for l in record['letters']], **record['end'])


main()
