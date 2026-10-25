"""P*: declared finite macro families, exact candidate replay, and resumable runs.

Search records describe the actual enumeration/optimisation, not a claim that
all legal macros have been exhausted. R is exhaustive through two retained
letters; its longer prefixes use a beam. All decisions are integer/exact.
"""
import argparse
from contextlib import contextmanager
from functools import lru_cache
import heapq
import itertools
import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import time

sys.dont_write_bytecode = True
for _var in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_var] = '1'
sys.path.insert(0, str(Path(__file__).resolve().parent))
import macro as A  # noqa: E402

P, BM, J, T, C, CLS, np = A.P, A.BM, A.J, A.T, A.C, A.CLS, A.np
ORDER = ('T', 'R', 'M', 'E', 'L')


class Limit(Exception):
    pass


def tick(deadline=None):
    if deadline is not None and time.process_time() >= deadline:
        raise Limit('cpu-limit')


def candidate_key(row):
    return row['terminal_phi'], row['excursion'], row['length'], row['word']


def raw_candidate(events, phi, excursion, unit=None):
    events = [list(p) for p in events]
    return dict(word=events, terminal_phi=int(phi), excursion=int(excursion),
                length=len(events), target=events[-1][0], unit=unit)


def keep_best(store, key, row):
    if key not in store or candidate_key(row) < candidate_key(store[key]):
        store[key] = row


def replay_candidates(state, rows, e, deadline=None):
    out = []
    seen = set()
    for row in sorted(rows, key=candidate_key):
        key = tuple(map(tuple, row['word']))
        if key in seen:
            continue
        seen.add(key)
        tick(deadline)
        with A.evaluated(state) as tokens:
            claims = A.execute(state, row['word'], e, row['unit'], tokens=tokens)
            A.require(claims['phi'][-1] == row['terminal_phi'], 'search phi')
            A.require(claims['excursion'] == row['excursion'], 'search excursion')
            row = dict(row, claims=claims)
            out.append(row)
    return out


class Seconds:
    """The twenty exact second vertices, and their exact K+ action.

    Each representative is an inverse jumble left factor. Equality of vertices
    u v0 and v v0 is dist(v^-1 u)==0. All D tests are tree tests. Transport of
    these vertices is a sufficient statistic of the exact retained product on
    the original pose; heights/branches alone are never used to infer D.
    """
    def __init__(self):
        self.vertices = []
        for q in range(5):
            c = int(np.flatnonzero(CLS == q)[0])
            vs = []
            for i in P.JUMBLES:
                v = T.qconj(P.left_letter(c, i))
                if not any(T.dist(T.qmul(T.qconj(u), v)) == 0 for u in vs):
                    vs.append(v)
            A.require(len(vs) == 4, 'second vertices')
            self.vertices.extend(vs)

    @lru_cache(maxsize=600)
    def codes(self, c):
        q = int(CLS[c])
        return np.array([next(j for j in range(4*q, 4*q+4)
                              if T.dist(T.qmul(P.left_letter(c, i), self.vertices[j])) == 0)
                         for i in P.JUMBLES], np.int8)

    @lru_cache(maxsize=7200)
    def action(self, k):
        lf, _ = T.factors(BM.KP.matrix(k))
        sigma = A.pole_action(k)[2]
        result = np.empty(21, np.int8)
        result[-1] = -1
        for j, v in enumerate(self.vertices):
            q = int(sigma[j // 4])
            image = T.qmul(lf, v)
            result[j] = next(r for r in range(4*q, 4*q+4)
                             if T.dist(T.qmul(T.qconj(self.vertices[r]), image)) == 0)
        return result

    @lru_cache(maxsize=20000)
    def of_pose(self, matrix, height, branch):
        lf, rf = T.factors(matrix)
        A.require(T.dist(rf) == 0, 'right tree')
        return next(j for j in range(4*branch, 4*branch+4)
                    if T.dist(T.qmul(T.qconj(self.vertices[j]), lf)) == height - 2)

    def rows(self, st, pieces, hs, bs):
        result = np.full(len(pieces), -1, np.int8)
        for pid in np.unique(st.pose_id[pieces[hs >= 4]]).tolist():
            ids = np.flatnonzero((st.pose_id[pieces] == pid) & (hs >= 4))
            j = ids[0]
            result[ids] = self.of_pose(st._pose_matrix(pid), int(hs[j]), int(bs[j]))
        return result


@lru_cache(maxsize=1)
def seconds():
    return Seconds()


class RetainedSearch:
    """Weighted exact equivalence rows for a retained-only search.

    Lattice pieces can be omitted: they remain lattice, hence contribute to
    neither straddlers, foreign load, high supply nor the R generator. Off
    pieces are grouped only when their complete 600 labels and branch agree;
    high pieces additionally need the same exact pose. Original poses remain
    in `source`; `rot` records their accumulated exact K+ product.
    """
    def __init__(self, state):
        self.source = state
        ps = np.flatnonzero(state.h > 0)
        keys = np.empty((len(ps), 604), np.uint8)
        keys[:, :600] = state.lab[ps]
        tags = np.where(state.h[ps] >= 4, state.st.pose_id[ps] + 10, state.b[ps]).astype('<i4')
        keys[:, 600:] = tags.view(np.uint8).reshape(-1, 4)
        _, ix, weights = np.unique(keys.view('V604').ravel(), return_index=True, return_counts=True)
        self.pieces, self.w = ps[ix], weights.astype(np.int64)
        self.lab = state.lab[self.pieces].copy()
        self.h, self.b = state.h[self.pieces].copy(), state.b[self.pieces].copy()
        self.sec = seconds().rows(state.st, self.pieces, self.h, self.b)
        self.rot = np.zeros(len(ix), np.int32)
        self.ns = (self.lab == 2).T @ self.w
        self.partners = P.graph()[2]

    def generator(self, x):
        lx = self.lab[:, x]
        obstruction = (lx == 2) | ((lx == 1) & (self.b != CLS[x]))
        useful = obstruction | ((self.h >= 4) & (self.b == CLS[x]))
        clean = self.ns == 0
        hold_useful = (self.lab[useful] == 1).any(0)
        caps = {c for c in self.partners[x] if clean[c] and hold_useful[c]}
        if clean[x]:
            caps.add(x)
        meets = (self.lab[obstruction] == 1).any(0)
        blocked = [y for y in [x, *self.partners[x]] if not clean[y] and meets[y]]
        if blocked:
            straddlers = (self.lab[:, blocked] == 2).any(1)
            caps.update(np.flatnonzero(clean & (self.lab[straddlers] == 1).any(0)).tolist())
        return sorted(caps)

    def metric(self, x):
        inside = self.lab[:, x] == 1
        f = int(self.w[inside & (self.b != CLS[x])].sum())
        high = inside & (self.sec >= 0)
        dc = np.zeros(20, np.int64)
        np.add.at(dc, self.sec[high], self.w[high])
        return int(self.ns[x]), f, dc[seconds().codes(x)]

    def leaf(self, x, c, i, base, moved):
        s, f, ds = base
        k, pre, _, _ = A.retained(c, i)
        old, new = self.lab[moved, x], self.lab[moved, pre[x]]
        weights, bs = self.w[moved], self.b[moved]
        s += int(weights[new == 2].sum() - weights[old == 2].sum())
        f += int(weights[(new == 1) & (bs != CLS[pre[x]])].sum()
                 - weights[(old == 1) & (bs != CLS[x])].sum())
        high = self.sec[moved] >= 0
        if high.any():
            old_sec = self.sec[moved[high]]
            new_sec = seconds().action(k)[old_sec]
            dc = np.zeros(20, np.int64)
            np.add.at(dc, old_sec[old[high] == 1], -weights[high][old[high] == 1])
            np.add.at(dc, new_sec[new[high] == 1], weights[high][new[high] == 1])
            ds = ds + dc[seconds().codes(x)]
        return s, f, ds

    @contextmanager
    def applied(self, c, i):
        A.require(self.ns[c] == 0, 'search admissibility')
        moved = np.flatnonzero(self.lab[:, c] == 1)
        old = self.lab[moved].copy(), self.b[moved].copy(), self.sec[moved].copy(), self.rot[moved].copy(), self.ns.copy()
        k, pre, _, sigma = A.retained(c, i)
        self.lab[moved] = old[0][:, pre]
        self.b[moved] = sigma[old[1]]
        high = self.h[moved] >= 4
        if high.any():
            self.sec[moved[high]] = seconds().action(k)[old[2][high]]
            self.rot[moved[high]] = BM.KP.compose_vec(np.full(int(high.sum()), k), old[3][high])
        self.ns += ((self.lab[moved] == 2).astype(np.int64) - (old[0] == 2)) .T @ self.w[moved]
        try:
            yield
        finally:
            self.lab[moved], self.b[moved], self.sec[moved], self.rot[moved], self.ns = *old[:4], old[4]

    @contextmanager
    def path(self, events):
        from contextlib import ExitStack
        with ExitStack() as stack:
            for c, i in events:
                stack.enter_context(self.applied(c, i))
            yield


def family_r(state, depth, beam, deadline=None):
    started = time.process_time()
    rs = RetainedSearch(state)
    phi0 = J.phi(state.h)
    targets = sorted({x for _, _, holders in P.high_units(state) for x in holders})
    counts = dict(targets=len(targets), nodes=0, retained_letters=0, lowering_tests=0, descending_words=0)
    best = {}
    per_target = []
    for x in targets:
        tick(deadline)
        by_depth = [0] * (depth + 1)

        def visit_leaf(prefix, metric):
            s, f, ds = metric
            if s:
                return
            counts['lowering_tests'] += 48
            for i, d in zip(P.JUMBLES, ds.tolist()):
                if f < d:
                    counts['descending_words'] += 1
                    keep_best(best, x, raw_candidate([*prefix, (x, i)], phi0 + 2*(f-d), 0))

        def children(prefix, frontier):
            tick(deadline)
            counts['nodes'] += 1
            base = rs.metric(x)
            for c in rs.generator(x):
                moved = np.flatnonzero(rs.lab[:, c] == 1)
                for i in range(1, 12):
                    counts['retained_letters'] += 1
                    events = (*prefix, (c, i))
                    metric = rs.leaf(x, c, i, base, moved)
                    by_depth[len(events)] += 1
                    visit_leaf(events, metric)
                    if frontier is not None:
                        s, f, ds = metric
                        frontier.append(((s, f-int(ds.max())), events))

        visit_leaf((), rs.metric(x))
        roots = []
        if depth >= 1:
            children((), roots if depth >= 2 else None)
        frontier = []
        if depth >= 2:
            # All depth-one nodes, without beam truncation.
            for _, prefix in roots:
                with rs.path(prefix):
                    children(prefix, frontier if depth >= 3 else None)
        truncated = False
        for d in range(3, depth + 1):
            truncated |= len(frontier) > beam
            selected = heapq.nsmallest(beam, frontier)
            frontier = []
            for _, prefix in selected:
                with rs.path(prefix):
                    children(prefix, frontier if d < depth else None)
        per_target.append(dict(target=x, nodes_at_depth=by_depth, beam_truncated=truncated))
    rows = replay_candidates(state, best.values(), 0, deadline)
    return dict(candidates=rows, counts=counts, exhausted=depth <= 2,
                exhaustive_through=min(2, depth), depth=depth, beam=beam,
                search='R generator from the packet; beam per target ranked by (S,F-max D,word)',
                reporting='minimum choice key per target; all tested descending words counted',
                targets=per_target, cpu_s=round(time.process_time()-started, 6))


@lru_cache(maxsize=600)
def item_cosets(c):
    """All 60 items as a * representative, a retained; representatives exact."""
    matrices = {P.matrix(c, i): i for i in range(60)}
    remaining = set(range(60))
    groups = []
    while remaining:
        rep = min(remaining)
        group = []
        for a in range(12):
            product = A.to_tuple(A.matmul(P.matrix(c, a), P.matrix(c, rep)))
            i = matrices[product]
            group.append((i, a))
        A.require(len({i for i, _ in group}) == 12, 'item coset')
        remaining.difference_update(i for i, _ in group)
        groups.append((rep, sorted(group)))
    return groups


def images(state, c, caps, allowed, deadline=None):
    """One exact image per left A4 coset, then exact retained column reading.

    Returns per-item s/f/D contributions at each requested cap and the helper
    delta. The second vertices are read from exact image poses and transported
    with the retained left factor. No full-state label update is claimed here.
    """
    caps, allowed = sorted(set(caps)), set(allowed)
    moved = state.inside(c)
    digest = state.st.digest()
    output = {}
    try:
        for rep, group in item_cosets(c):
            group = [(i, a) for i, a in group if i in allowed]
            if not group:
                continue
            tick(deadline)
            columns = sorted({int(A.retained(c, a)[1][x]) for _, a in group for x in caps})
            with P.formal_image(state.st, c, rep, moved):
                if rep:
                    hs, bs = J.tree_rows(state.st, moved)
                    k = state.st._kpose()[state.st.pose_id]
                    lp, op = moved[k[moved] >= 0], moved[k[moved] < 0]
                    groups = [(pid, state.st._anchors(pid, mem)) for pid, mem in state.st._groups(op)]
                    labs = BM._label_caps(state.st, columns, lp, k, groups)[moved]
                else:
                    hs, bs, labs = state.h[moved], state.b[moved], state.lab[np.ix_(moved, columns)]
                sec = seconds().rows(state.st, moved, hs, bs)
                delta = J.phi(hs) - J.phi(state.h[moved])
                col = {x: j for j, x in enumerate(columns)}
                for i, a in group:
                    k, pre, _, sigma = A.retained(c, a)
                    new_sec = seconds().action(k)[sec] if (sec >= 0).any() else sec
                    by_cap = {}
                    for x in caps:
                        lx = labs[:, col[int(pre[x])]]
                        high = (lx == 1) & (new_sec >= 0)
                        dc = np.bincount(new_sec[high], minlength=20).astype(np.int64)
                        by_cap[x] = dict(s=int((lx == 2).sum()),
                                         f=int(((lx == 1) & (hs >= 2) & (bs != CLS[pre[x]])).sum()),
                                         d=dc[seconds().codes(x)], not_out=bool((lx != 0).any()))
                    output[i] = dict(item=i, dphi=delta, caps=by_cap)
    finally:
        A.require(state.st.digest() == digest, 'undo', 'coset image batch')
    return output


@contextmanager
def retained_image(state, c, i):
    """Interior search node; exact poses plus calculus (replay checks later)."""
    moved = state.inside(c)
    A.require(not (state.lab[:, c] == 2).any(), 'search admissibility')
    token = moved, state.h[moved].copy(), state.b[moved].copy(), state.lab[moved].copy()
    digest = state.st.digest()
    with P.formal_image(state.st, c, i, moved):
        _, pre, _, sigma = A.retained(c, i)
        state.lab[moved] = token[3][:, pre]
        state.b[moved] = sigma[token[2]]
        try:
            yield
        finally:
            state.h[moved], state.b[moved], state.lab[moved] = token[1:]
    A.require(state.st.digest() == digest, 'undo', 'retained interior')


def family_m(state, deadline=None):
    started, phi0 = time.process_time(), J.phi(state.h)
    clean = ~(state.lab == 2).any(0)
    targets = sorted({x for _, _, holders in P.high_units(state) for x in holders if clean[x]})
    best = {}
    counts = dict(targets=len(targets), reaims=0, partners=0, clean_partners=0,
                  helper_items=0, tame_items=0, lowering_tests=0, descending_words=0)
    for x in targets:
        for a in range(12):
            tick(deadline)
            counts['reaims'] += 1
            with retained_image(state, x, a):
                clean = ~(state.lab == 2).any(0)
                for y in P.graph()[2][x]:
                    counts['partners'] += 1
                    if not clean[y]:
                        continue
                    counts['clean_partners'] += 1
                    f, ds = state.budget(y)
                    allowed = list(range(12)) + [i for i, d in zip(P.JUMBLES, ds) if f <= d]
                    counts['helper_items'] += 60
                    counts['tame_items'] += len(allowed)
                    ims = images(state, y, [x], allowed, deadline)
                    moved = state.inside(y)
                    keep = np.ones(len(state.h), bool)
                    keep[moved] = False
                    lx = state.lab[:, x]
                    s0 = int(((lx == 2) & keep).sum())
                    remaining = np.flatnonzero((lx == 1) & keep)
                    f0 = int(((state.h[remaining] >= 2) & (state.b[remaining] != CLS[x])).sum())
                    d0 = state.oracle.counts(state.st, state.h, state.b, remaining, x)
                    for g in allowed:
                        im, part = ims[g], ims[g]['caps'][x]
                        if s0 + part['s']:
                            continue
                        counts['lowering_tests'] += 48
                        ff, dd = f0 + part['f'], d0 + part['d']
                        for h, d in zip(P.JUMBLES, dd.tolist()):
                            # A lowering is the final negative jumble; helper tame.
                            if ff < d:
                                counts['descending_words'] += 1
                                keep_best(best, x, raw_candidate([(x, a), (y, g), (x, h)],
                                          phi0 + im['dphi'] + 2*(ff-d), 0))
    rows = replay_candidates(state, best.values(), 0, deadline)
    return dict(candidates=rows, exhausted=True, counts=counts,
                reporting='minimum choice key per target; identities retained as declared letters',
                cpu_s=round(time.process_time()-started, 6))


def family_t(state, deadline=None):
    started = time.process_time()
    tick(deadline)
    scan = P.scan(state)
    # scan exhausts T, but its tie-break is (delta,h,item vector), whereas
    # P* requires (Phi,excursion,length,chronological word). If rounds exist,
    # optimise the same covers with the latter key (zero raising budget and
    # P4 retained). An exhaustive empty scan needs no second enumeration.
    optimised = (family_e(state, 0, deadline, require_lowering=True)
                 if any(r['status'] == 'round' for r in scan['rows']) else None)
    rows = optimised['candidates'] if optimised is not None else []
    return dict(candidates=rows, exhausted=True, counts=dict(unit_holders=len(scan['rows']),
                covers=sum(r.get('covers', 0) for r in scan['rows']),
                frontier_records=sum(r.get('feasible_frontier_records', 0) for r in scan['rows'])),
                search=P.FAMILY, scan=scan,
                choice_optimisation=optimised['counts'] if optimised else None,
                cpu_s=round(time.process_time()-started, 6))


def e_assignments(cover, options, column, e0, counts, deadline):
    active_possible = {c for c in cover if any(o['item'] for o in options[c])}
    edges_possible = any(o['item'] and active_possible.intersection(o['before'])
                         for c in cover for o in options[c])
    if edges_possible:
        counts['product_covers'] += 1
        for chosen in itertools.product(*(options[c] for c in cover)):
            tick(deadline)
            counts['products'] += 1
            if sum(max(0, o['dphi']) for o in chosen) > e0:
                continue
            active = {c for c, o in zip(cover, chosen) if o['item']}
            edges = {(d, c) for c, o in zip(cover, chosen) if c in active
                     for d in o['before'] if d in active}
            order = P.topological(active, edges)
            if order is not None:
                yield chosen, order
        return
    counts['dp_covers'] += 1
    # Two budget coordinates: F-D and sum of positive helper deltas.
    # Within a coordinate keep the least total delta. Tied deltas retain
    # every peak, with the shortest/lexicographically least word for that
    # peak: a later peak can erase an earlier excursion advantage.
    frontier = {(0, 0): (0, {0: ((), ())})}
    for c in cover:
        nxt = {}
        for (budget, raising), (delta, variants) in frontier.items():
            for o in options[c]:
                tick(deadline)
                counts['dp_transitions'] += 1
                nr = raising + max(0, o['dphi'])
                if nr > e0:
                    continue
                key = (budget + o['f'] - o['d'][column], nr)
                nd = delta + o['dphi']
                if key not in nxt or nd < nxt[key][0]:
                    nxt[key] = (nd, {})
                if nd != nxt[key][0]:
                    continue
                dest = nxt[key][1]
                for peak, (events, chosen) in variants.items():
                    ne = events + (((c, o['item']),) if o['item'] else ())
                    npk = max(peak, nd)
                    if npk not in dest or (len(ne), ne) < (len(dest[npk][0]), dest[npk][0]):
                        dest[npk] = (ne, chosen + (o,))
        frontier = nxt
    for _, variants in frontier.values():
        for events, chosen in variants.values():
            yield chosen, [c for c, _ in events]


def family_e(state, e0, deadline=None, require_lowering=False):
    started, phi0 = time.process_time(), J.phi(state.h)
    clean = ~(state.lab == 2).any(0)
    caps = np.flatnonzero(clean).tolist()
    inside = {c: state.inside(c) for c in caps}
    sets = {c: P.bits(ps) for c, ps in inside.items()}
    partners = P.graph()[2]
    counts = dict(unit_holders=0, covers=0, helper_items=0, admitted_items=0,
                  assignments=0, descending_words=0, product_covers=0, products=0,
                  dp_covers=0, dp_transitions=0)
    cache, best, records = {}, {}, []
    for pid, members, holders in P.high_units(state):
        for x in holders:
            tick(deadline)
            counts['unit_holders'] += 1
            sx = np.flatnonzero(state.lab[:, x] == 2)
            record = dict(target=x, unit=int(members[0]), covers=0, feasible_assignments=0)
            missing = sx[~(state.lab[np.ix_(sx, caps)] == 1).any(1)]
            if len(missing):
                records.append(dict(record, status='straddler-without-clean-holder', count=len(missing)))
                continue
            obstruction = ((state.lab[:, x] == 2) | ((state.lab[:, x] == 1)
                            & (state.h >= 2) & (state.b != CLS[x])))
            useful = obstruction | ((state.h >= 4) & (state.b == CLS[x]))
            candidates = [c for c in partners[x] if clean[c]
                          and not (state.lab[members, c] == 1).any() and useful[inside[c]].any()]
            hs = [i for i, yes in zip(P.JUMBLES, state.oracle.lowers(
                state.st, pid, x, P.JUMBLES, int(state.h[members[0]]))) if yes]
            options = {}
            for cover in P.maximal_covers(candidates, sets, P.bits(sx)):
                counts['covers'] += 1
                record['covers'] += 1
                for c in cover:
                    if c in options:
                        continue
                    peers = tuple(d for d in candidates if c != d and d in partners[c] and not sets[c] & sets[d])
                    key = (x, c, tuple(hs), peers)
                    if key not in cache:
                        ff, dd = state.budget(c)
                        allowed = list(range(12)) + [i for i, d in zip(P.JUMBLES, dd) if 2*(ff-int(d)) <= e0]
                        counts['helper_items'] += 60
                        ims = images(state, c, [x, *peers], allowed, deadline)
                        opts = []
                        for i in sorted(ims):
                            im, at = ims[i], ims[i]['caps'][x]
                            if at['s']:
                                continue
                            expected = 0 if i < 12 else 2*(ff-int(dd[i-12]))
                            A.require(im['dphi'] == expected, 'helper delta')
                            opts.append(dict(item=i, f=at['f'], d=[int(at['d'][h-12]) for h in hs],
                                             dphi=im['dphi'], before=[d for d in peers if im['caps'][d]['not_out']]))
                        cache[key] = opts
                        counts['admitted_items'] += len(opts)
                    options[c] = cache[key]
                if any(not options[c] for c in cover):
                    continue
                union = np.zeros(len(state.h), bool)
                for c in cover:
                    union[inside[c]] = True
                if ((state.lab[:, x] == 2) & ~union).any():
                    continue
                remaining = np.flatnonzero((state.lab[:, x] == 1) & ~union)
                f0 = int(((state.h[remaining] >= 2) & (state.b[remaining] != CLS[x])).sum())
                d0 = state.oracle.counts(state.st, state.h, state.b, remaining, x, hs)
                for j, h in enumerate(hs):
                    for chosen, order in e_assignments(cover, options, j, e0, counts, deadline):
                        counts['assignments'] += 1
                        record['feasible_assignments'] += 1
                        fd = f0-int(d0[j])+sum(o['f']-o['d'][j] for o in chosen)
                        delta = sum(o['dphi'] for o in chosen) + 2*fd
                        if delta >= 0 or (require_lowering and fd >= 0):
                            continue
                        counts['descending_words'] += 1
                        lookup = dict(zip(cover, chosen))
                        events = [(c, lookup[c]['item']) for c in order] + [(x, h)]
                        running, peak = 0, 0
                        for c in order:
                            running += lookup[c]['dphi']
                            peak = max(peak, running)
                        peak = max(peak, delta)
                        keep_best(best, (x, int(members[0])), raw_candidate(events, phi0+delta, peak, int(members[0])))
            records.append(dict(record, status='round' if (x, int(members[0])) in best else 'no-round'))
    rows = replay_candidates(state, best.values(), e0, deadline)
    return dict(candidates=rows, exhausted=True, e0=e0, counts=counts, targets=records,
                search='T3P covers and P0/P1, raising-budget DP or complete acyclic products; net descent',
                reporting='minimum choice key per unit/holder; DP frontier records, not every dominated word',
                cpu_s=round(time.process_time()-started, 6))


def clean_pairs(state):
    clean = ~(state.lab == 2).any(0)
    pairs = set()
    for p in np.flatnonzero(state.h >= 4):
        held = np.flatnonzero(clean & (state.lab[p] == 1)).tolist()
        pairs.update(itertools.combinations(held, 2))
    return sorted(pairs)


def lens_replay(state, events, intersection, a, x, e0, deadline):
    """Verify the full lens, including restored collateral, before reporting it."""
    tick(deadline)
    phi0 = J.phi(state.h)
    old_ids, old_poses = state.st.pose_id.copy(), list(state.st._poses)
    originals = {int(pid): state.st._pose_matrix(int(pid)) for pid in np.unique(old_ids[intersection])}
    with A.evaluated(state) as tokens:
        profile = [phi0]
        for c, i in events:
            row, token = A.apply_checked(state, c, i)
            tokens.append((c, i, token))
            profile.append(row['phi'])
        outside = np.ones(len(state.h), bool)
        outside[intersection] = False
        remap = np.array([state.st._lookup.get(p.key, -1) for p in old_poses], np.int64)
        A.require(np.array_equal(state.st.pose_id[outside], remap[old_ids[outside]]),
                  'lens collateral', 'a non-intersection pose did not return')
        for pid, matrix in originals.items():
            ids = np.unique(state.st.pose_id[intersection[old_ids[intersection] == pid]])
            A.require(len(ids) == 1 and state.st._pose_matrix(int(ids[0]))
                      == A.to_tuple(A.matmul(P.matrix(x, a), matrix)), 'lens intersection')
        exc = max(profile) - phi0
        if profile[-1] >= phi0 or exc > e0:
            return None
        return raw_candidate(events, profile[-1], exc)


def lens_pair(state, x, z, e0=20, deadline=None):
    """Both forms and both orientations. Counts distinguish formal predicates.

    A P1 cell with a retained final b^-1 is replayable but cannot be encoded
    by the packet's mandatory terminal-jumble schema. Such witnesses are
    explicitly recorded, never silently called an exhaustive negative.
    """
    digest = state.st.digest()
    counts = {form: dict(orientations=2, a_items=0, b_items=0, item_pairs=0,
                         formal_pass=0, descending=0, replayed=0,
                         terminal_retained=0) for form in ('P1', 'P2')}
    best, unencodable = {}, []
    ix, iz = state.inside(x), state.inside(z)
    intersection = np.intersect1d(ix, iz)
    try:
        for xx, yy, xx_in, yy_in in ((x, z, ix, iz), (z, x, iz, ix)):
            dx, dy = np.setdiff1d(xx_in, intersection), np.setdiff1d(yy_in, intersection)
            # P1: lowering items of the high pieces in the intersection.
            dc = state.oracle.counts(state.st, state.h, state.b, intersection, xx)
            aa = [a for a, d in zip(P.JUMBLES, dc) if d > 0]
            ct = counts['P1']
            ct['a_items'] += len(aa)
            ct['item_pairs'] += len(aa)*60
            for a in aa:
                tick(deadline)
                with P.formal_image(state.st, xx, a, xx_in):
                    if (J.labels_at(state.st, dx, yy) != 0).any():
                        continue
                    if (J.labels_at(state.st, intersection, yy) != 1).any():
                        continue
                    for b in range(60):
                        tick(deadline)
                        ct['b_items'] += 1
                        with P.formal_image(state.st, yy, b, yy_in):
                            if (J.labels_at(state.st, dy, xx) != 0).any():
                                continue
                            if (J.labels_at(state.st, intersection, xx) != 0).any():
                                continue
                        ct['formal_pass'] += 1
                        # Defer actual replay until all formal poses are undone.
                        best.setdefault(('formal', 'P1', xx), []).append((a, b))
            # P2's predicates separate: enumerate both lists, then their product.
            ct = counts['P2']
            ct['a_items'] += 60
            ct['b_items'] += 60
            ct['item_pairs'] += 3600
            good_b, good_a = [], []
            for b in range(60):
                tick(deadline)
                with P.formal_image(state.st, yy, b, yy_in):
                    if not (J.labels_at(state.st, yy_in, xx) != 0).any():
                        good_b.append(b)
            for a in range(60):
                tick(deadline)
                with P.formal_image(state.st, xx, A.inverse_item(xx, a), dx):
                    if not (J.labels_at(state.st, dx, yy) != 0).any():
                        good_a.append(a)
            ct['formal_pass'] += len(good_a)*len(good_b)
            best[('formal', 'P2', xx)] = list(itertools.product(good_a, good_b))
        A.require(state.st.digest() == digest, 'undo', 'lens predicates')
        reported = {}
        for (tag, form, xx), pairs in sorted(best.items()):
            yy = z if xx == x else x
            for a, b in pairs:
                tick(deadline)
                # The formal theorem changes ONLY K by a; reject non-descents
                # before the more expensive full exact chronological replay.
                with P.formal_image(state.st, xx, a, intersection):
                    hh, _ = J.tree_rows(state.st, intersection)
                if J.phi(hh) >= J.phi(state.h[intersection]):
                    continue
                counts[form]['descending'] += 1
                if form == 'P1':
                    events = [(xx, a), (yy, b), (xx, A.inverse_item(xx, a)), (yy, A.inverse_item(yy, b))]
                else:
                    events = [(yy, b), (xx, A.inverse_item(xx, a)), (yy, A.inverse_item(yy, b)), (xx, a)]
                counts[form]['replayed'] += 1
                row = lens_replay(state, events, intersection, a, xx, e0, deadline)
                if row is None:
                    continue
                if events[-1][1] < 12:
                    counts[form]['terminal_retained'] += 1
                    unencodable.append(dict(row, form=form))
                else:
                    row['lens_form'] = form
                    keep_best(reported, form, row)
        return counts, list(reported.values()), sorted(unencodable, key=candidate_key)
    finally:
        A.require(state.st.digest() == digest, 'undo', 'lens pair batch')


def census(state, e0=20, deadline=None):
    started = time.process_time()
    rows, candidates, unencodable = [], [], []
    for x, z in clean_pairs(state):
        tick(deadline)
        ix, iz = state.inside(x), state.inside(z)
        intersection = np.intersect1d(ix, iz)
        groups = []
        for pid in np.unique(state.st.pose_id[intersection]).tolist():
            ps = intersection[state.st.pose_id[intersection] == pid]
            whole = np.flatnonzero(state.st.pose_id == pid)
            groups.append(dict(unit_piece=int(whole[0]), intersection_piece=int(ps[0]),
                               intersection_size=len(ps), whole_size=len(whole),
                               height=int(state.h[ps[0]]), branch=int(state.b[ps[0]])))
        groups.sort(key=lambda row: row['unit_piece'])
        uniform = (len(groups) == 1 and groups[0]['height'] >= 4
                   and groups[0]['intersection_size'] == groups[0]['whole_size'])
        tests, found, unsupported = lens_pair(state, x, z, e0, deadline)
        rows.append(dict(grips=[x, z], inside_sizes=[len(ix), len(iz)],
                         intersection_size=len(intersection), pose_groups=groups,
                         exactly_one_uniform_high_unit=uniform, classes_differ=bool(CLS[x] != CLS[z]),
                         lens_tests=tests))
        candidates.extend(found)
        unencodable.extend(unsupported)
    return dict(format='pstar-census/1', digest=state.st.digest(), phi=J.phi(state.h),
                pairs=rows, pair_count=len(rows), clean_grips=int((~(state.lab == 2).any(0)).sum()),
                candidates=candidates, unencodable=unencodable,
                exhausted=True, cpu_s=round(time.process_time()-started, 6))


def family_l(state, e0, deadline=None):
    result = census(state, e0, deadline)
    result['candidates'] = replay_candidates(state, result['candidates'], e0, deadline)
    result['exhausted'] = not result['unencodable']
    if result['unencodable']:
        result['limitation'] = 'P1 retained terminal event cannot satisfy macro-certificate/1 terminal-jumble rule'
    return result


def options(families='T,R,M,E,L', e0=20, depth=3, beam=64, choice='best'):
    requested = families.split(',') if isinstance(families, str) else families
    A.require(requested and len(requested) == len(set(requested)) and set(requested) <= set(ORDER), 'families')
    A.allowance(e0)
    A.require(type(depth) is int and depth >= 0, 'depth')
    A.require(type(beam) is int and beam >= 1, 'beam')
    A.require(choice in ('best', 'priority'), 'choice')
    return dict(families=[f for f in ORDER if f in requested], e0=e0, depth=depth, beam=beam, choice=choice)


def step(state, opts, deadline=None):
    start, digest = time.process_time(), state.st.digest()
    families = {}
    for name in opts['families']:
        tick(deadline)
        if name == 'T':
            record = family_t(state, deadline)
        elif name == 'R':
            record = family_r(state, opts['depth'], opts['beam'], deadline)
        elif name == 'M':
            record = family_m(state, deadline)
        elif name == 'E':
            record = family_e(state, opts['e0'], deadline)
        else:
            record = family_l(state, opts['e0'], deadline)
        A.require(state.st.digest() == digest, 'undo', f'family {name}')
        families[name] = record
    all_candidates = [dict(c, family=f) for f in opts['families'] for c in families[f]['candidates']]
    chosen = None
    if all_candidates:
        if opts['choice'] == 'priority':
            first = next(f for f in ORDER if f in families and families[f]['candidates'])
            all_candidates = [c for c in all_candidates if c['family'] == first]
        chosen = min(all_candidates, key=candidate_key)
    tick(deadline)
    return dict(format='pstar-step/1', digest=digest, phi=J.phi(state.h), options=opts,
                families=families, choice=chosen, cpu_s=round(time.process_time()-start, 6))


def attach_certificate(result, table_digest, prefix, state, e0):
    certificate = A.empty_certificate(table_digest, prefix, state, e0)
    row = result['choice']
    if row is not None:
        spec = dict(events=row['word'], claims=row['claims'])
        if row['unit'] is not None:
            spec['unit_piece'] = row['unit']
        certificate['macros'].append(spec)
    result['certificate'] = certificate
    return certificate


def atomic_json(path, obj):
    path = P.external_path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp = tempfile.mkstemp(prefix=path.name+'.', dir=path.parent)
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(obj, stream, sort_keys=True, indent=1, allow_nan=False)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temp, path)
    finally:
        if os.path.exists(temp):
            os.unlink(temp)


def checkpoint(out, run, state):
    """run.json is the commit marker and includes the authoritative certificate.

    A crash between replacements can leave newer sidecars; resume replays the
    run.json word, checks its digest, and ignores a mismatching label sidecar.
    """
    out = P.external_path(out)
    out.mkdir(parents=True, exist_ok=True)
    fd, temp = tempfile.mkstemp(prefix='labels.', suffix='.npz', dir=out)
    try:
        with os.fdopen(fd, 'wb') as stream:
            np.savez_compressed(stream, labels=state.lab, digest=np.array(state.st.digest()))
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temp, out/'labels.npz')
    finally:
        if os.path.exists(temp):
            os.unlink(temp)
    atomic_json(out/'certificate.json', run['certificate'])
    atomic_json(out/'run.json', run)


@contextmanager
def cpu_guard(deadline):
    if deadline is None:
        yield
        return
    tick(deadline)
    old = signal.getsignal(signal.SIGPROF)
    interrupted = False
    def handler(signum, frame):
        nonlocal interrupted
        interrupted = True
        raise Limit('cpu-limit')
    signal.signal(signal.SIGPROF, handler)
    signal.setitimer(signal.ITIMER_PROF, max(0.001, deadline-time.process_time()))
    try:
        yield
    except BaseException:
        # An interrupt can land between application and registration of an
        # undo token. A cleanup assertion must not mask the resource limit:
        # the caller restores its complete pre-step snapshot in this case.
        if interrupted:
            raise Limit('cpu-limit') from None
        raise
    finally:
        signal.setitimer(signal.ITIMER_PROF, 0)
        signal.signal(signal.SIGPROF, old)


def run_procedure(table, out, prefix, opts, labels=None, first=None,
                  max_steps=1000, cpu_limit=None, resume=False):
    started = time.process_time()
    out = P.external_path(out)
    A.require(type(max_steps) is int and max_steps >= 0, 'max-steps')
    A.require(cpu_limit is None or cpu_limit > 0, 'cpu-limit')
    source_digest = BM.load_table(table).digest()
    prior_cpu = 0
    if resume:
        A.require(first is None, 'resume', '--first applies only to a new run')
        run = json.loads((out/'run.json').read_text())
        A.require(run['format'] == 'pstar-run/1' and run['table_digest'] == source_digest, 'resume source')
        A.require(run['prefix'] == [list(p) for p in prefix] and run['options'] == opts, 'resume options')
        certificate = run['certificate']
        A.require(certificate['table_digest'] == source_digest
                  and certificate['prefix'] == run['prefix'], 'resume certificate source')
        A.require(certificate['excursion'] == opts['e0'], 'resume certificate allowance')
        A.require(run['macro_words'] == [spec['events'] for spec in certificate['macros']],
                  'resume macro words')
        A.require(run['trajectory'] == [certificate['start']] + [
            dict(phi=spec['claims']['phi'][-1], digest=spec['claims']['digests'][-1])
            for spec in certificate['macros']], 'resume trajectory')
        replay = A.independent_replay(table, certificate)
        A.require(replay['digest'] == run['digest'] and replay['phi'] == run['phi'], 'resume digest')
        cumulative = list(prefix) + [tuple(p) for m in certificate['macros'] for p in m['events']]
        state, _, setup = A.load(table, cumulative, out/'labels.npz')
        A.require(state.st.digest() == run['digest'], 'resume digest')
        prior_cpu = run['cpu_s']
        run.setdefault('resumptions', []).append(dict(start_digest=run['digest'], start_phi=run['phi'],
                                                     committed_macros=len(certificate['macros'])))
    else:
        A.require(not (out/'run.json').exists(), 'run exists', 'use --resume or a fresh output directory')
        state, _, setup = A.load(table, prefix, labels)
        certificate = A.empty_certificate(source_digest, prefix, state, opts['e0'])
        run = dict(format='pstar-run/1', table_digest=source_digest, prefix=[list(p) for p in prefix],
                   options=opts, certificate=certificate, macro_words=[], trajectory=[certificate['start']],
                   steps=[], cpu_s=0, status='unknown', reason='initialised',
                   phi=J.phi(state.h), digest=state.st.digest())
    run['setup'] = setup
    deadline = None if cpu_limit is None else started + cpu_limit-prior_cpu

    def save():
        run['cpu_s'] = round(prior_cpu + time.process_time()-started, 6)
        checkpoint(out, run, state)

    def record_commit(row, claims, record):
        # Called only after the complete checked application has returned,
        # outside the CPU interrupt region. No incomplete macro is published.
        spec = dict(events=row['word'], claims=claims)
        if row['unit'] is not None:
            spec['unit_piece'] = row['unit']
        certificate['macros'].append(spec)
        record['choice'] = dict(row, claims=claims)
        record['after'] = dict(phi=J.phi(state.h), digest=state.st.digest())
        run['steps'].append(record)
        run['macro_words'].append(row['word'])
        run['trajectory'].append(record['after'])
        run.update(record['after'])
        run.update(status='unknown', reason='checkpoint')
        save()

    def snapshot_state():
        return (state.lab.copy(), state.h.copy(), state.b.copy(), state.st.pose_id.copy(),
                list(state.st._poses), dict(state.st._lookup)) if deadline is not None else None

    def restore(snapshot):
        if snapshot is not None:
            state.lab, state.h, state.b, state.st.pose_id, state.st._poses, state.st._lookup = snapshot
            state.st._anchor_cache = {}

    save()
    if first is not None:
        snapshot, before = snapshot_state(), state.st.digest()
        try:
            with cpu_guard(deadline):
                claims = A.execute(state, [list(p) for p in first], opts['e0'])
                tick(deadline)
        except Limit:
            restore(snapshot)
            run.update(status='unknown', reason='cpu-limit')
            save()
            return run
        row = raw_candidate(first, claims['phi'][-1], claims['excursion'])
        row['claims'] = claims
        record_commit(row, claims, dict(kind='first', digest=before, families={}))
    while True:
        if J.phi(state.h) == 0:
            run.update(status='depth-one', reason='phi-zero')
            break
        if len(certificate['macros']) >= max_steps:
            run.update(status='unknown', reason='max-steps')
            break
        # A one-shot CPU interrupt may arrive inside an imported operation.
        # Preserve the exact checkpoint so its unfinished search is discarded.
        snapshot = snapshot_state()
        try:
            with cpu_guard(deadline):
                result = step(state, opts, deadline)
                row = result['choice']
                if row is not None:
                    # Recompute at the CURRENT state before committing; compare
                    # every claim, including the retained exact row evaluator.
                    claims = A.execute(state, row['word'], opts['e0'], row['unit'], row['claims'])
                tick(deadline)
        except Limit:
            restore(snapshot)
            run.update(status='unknown', reason='cpu-limit')
            break
        if result['choice'] is None:
            run['steps'].append(result)
            run.update(status='stopped', reason='no-family-candidate')
            break
        record_commit(row, claims, result)
    save()
    return run


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest='command', required=True)
    for command in ('step', 'run', 'census'):
        p = sub.add_parser(command)
        p.add_argument('table')
        p.add_argument('out')
        p.add_argument('--prefix', default='')
        p.add_argument('--labels')
        p.add_argument('--families', default='T,R,M,E,L')
        p.add_argument('--e0', type=int, default=20)
        p.add_argument('--depth', type=int, default=3)
        p.add_argument('--beam', type=int, default=64)
        p.add_argument('--choice', choices=('best', 'priority'), default='best')
        if command == 'run':
            p.add_argument('--first')
            p.add_argument('--max-steps', type=int, default=1000)
            p.add_argument('--cpu-limit', type=float)
            p.add_argument('--resume', action='store_true')
    a = ap.parse_args(argv)
    P.external_path(a.out)
    if a.labels:
        P.external_path(a.labels)
    opts = options(a.families, a.e0, a.depth, a.beam, a.choice)
    prefix = P.parse_word(a.prefix)
    try:
        if a.command == 'run':
            result = run_procedure(a.table, a.out, prefix, opts, a.labels,
                                   P.parse_word(a.first) if a.first is not None else None,
                                   a.max_steps, a.cpu_limit, a.resume)
        else:
            state, digest, setup = A.load(a.table, prefix, a.labels)
            if a.command == 'step':
                result = step(state, opts)
                cert = attach_certificate(result, digest, prefix, state, a.e0)
                atomic_json(str(a.out)+'.certificate.json', cert)
            else:
                result = census(state, a.e0)
            result.update(prefix=[list(p) for p in prefix], setup=setup)
            atomic_json(a.out, result)
        return 0
    except (A.Failure, ValueError, KeyError, TypeError, AssertionError) as exc:
        error = exc.record() if isinstance(exc, A.Failure) else dict(test='pstar', detail=str(exc))
        dest = Path(a.out)/'error.json' if a.command == 'run' else Path(a.out)
        atomic_json(dest, dict(passed=False, failures=[error]))
        print(json.dumps(error, sort_keys=True), file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
