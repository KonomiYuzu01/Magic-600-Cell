"""Finite T3P/T3P+ certificate tests and exhaustive, family-specific experiments.

Run from the repository root. All decisions use exact labels, integer counts or
trees.py arithmetic. Temporary one-letter images are formal piecewise maps (so
they can also test a blocked T3P+ helper), never assertions of admissibility.
They are undone by the inverse map before any base-state label is used again.
Actual words use State.apply, which updates every moved row at all 600 grips.
"""
import argparse
from contextlib import contextmanager
from functools import lru_cache
import heapq
import itertools
import json
import os
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_name] = '1'
sys.path.insert(0, str(Path(__file__).resolve().parent))
import numpy as np  # noqa: E402
import blockmap as BM  # noqa: E402
import jamcheck as J  # noqa: E402
from exact import transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402
from locality import external_path, graph, write_json  # noqa: E402

C, M, R, T, CLS = BM.C, BM.M, BM.R, BM.T, BM.CLS
JUMBLES = tuple(range(12, 60))


def grip(c):
    if type(c) is not int or not 0 <= c < 600:
        raise ValueError('grip must be an integer in 0..599')
    return c


def item(i):
    if type(i) is not int or not 0 <= i < 60:
        raise ValueError('item must be an integer in 0..59')
    return i


def parse_word(s):
    if not s:
        return []
    out = []
    for letter in s.split(','):
        c, i = (int(v) for v in letter.split(':'))
        out.append((grip(c), item(i)))
    return out


def parse_items(s):
    return sorted({item(int(i)) for i in s.split(',')}) if s else list(range(60))


@lru_cache(maxsize=8192)
def matrix(c, i):
    return R.element(M, c, i)


@lru_cache(maxsize=8192)
def left_letter(c, i):
    left, right = T.factors(matrix(c, i))
    assert T.dist(right) == 0
    return left


class TreeOracle:
    """Caches are keyed by immutable exact poses, never compacted pose IDs."""

    def __init__(self):
        self.lefts = {}
        self.lower = {}

    def lowers(self, st, pid, c, items, height):
        pose = st._poses[int(pid)]
        key = pose.key
        if key not in self.lefts:
            l, r = T.factors(st._pose_matrix(int(pid)))
            assert T.dist(r) == 0
            self.lefts[key] = l
        l = self.lefts[key]
        out = []
        for i in items:
            cache_key = (key, c, i)
            if cache_key not in self.lower:
                self.lower[cache_key] = T.dist(T.qmul(left_letter(c, i), l)) == height - 2
            out.append(self.lower[cache_key])
        return np.asarray(out, dtype=bool)

    def counts(self, st, h, b, pieces, c, items=JUMBLES):
        """D_c(i), using the actual per-piece inside set supplied by the caller."""
        pieces = np.asarray(pieces, dtype=np.int64)
        ps = pieces[(h[pieces] >= 4) & (b[pieces] == CLS[c])]
        result = np.zeros(len(items), np.int64)
        if len(ps):
            pids, first, counts = np.unique(st.pose_id[ps], return_index=True, return_counts=True)
            for pid, k, n in zip(pids.tolist(), first.tolist(), counts.tolist()):
                result += n * self.lowers(st, pid, c, items, int(h[ps[k]]))
        return result

    def local_counts(self, st, pieces, heights, branches, labels, c, items):
        ps = np.flatnonzero((labels == 1) & (heights >= 4) & (branches == CLS[c]))
        result = np.zeros(len(items), np.int64)
        if len(ps):
            pids, first, counts = np.unique(st.pose_id[pieces[ps]], return_index=True, return_counts=True)
            for pid, k, n in zip(pids.tolist(), first.tolist(), counts.tolist()):
                result += n * self.lowers(st, pid, c, items, int(heights[ps[k]]))
        return result


def updated_rows(st, pieces):
    """The blockmap exact evaluator, restricted to moved pieces, at every grip."""
    k = st._kpose()[st.pose_id]
    lp = pieces[k[pieces] >= 0]
    op = pieces[k[pieces] < 0]
    groups = [(pid, st._anchors(pid, mem)) for pid, mem in st._groups(op)]
    return BM._label_caps(st, list(range(600)), lp, k, groups)[pieces]


class State:
    def __init__(self, st, lab, h=None, b=None):
        self.st, self.lab = st, lab
        self.h, self.b = BM.tree_info(st) if h is None else (h, b)
        self.oracle = TreeOracle()

    def inside(self, c):
        return np.flatnonzero(self.lab[:, c] == 1)

    def budget(self, c, items=JUMBLES):
        ps = self.inside(c)
        f = int(((self.h[ps] >= 2) & (self.b[ps] != CLS[c])).sum())
        return f, self.oracle.counts(self.st, self.h, self.b, ps, c, items)

    def apply(self, c, i, full_heights=False, record=True):
        moved = self.inside(c)
        expected_legal = not (self.lab[:, c] == 2).any()
        old = (moved, self.h[moved].copy(), self.b[moved].copy(), self.lab[moved].copy())
        classified = self.st.classify(c, need_inside=True, stop_at_straddle=True)
        assert not classified.uncertain
        if classified.straddle is None:
            assert np.array_equal(classified.inside, moved), 'label table disagrees with exact moved set'
        tw = BM.sim.Twist(C, c, matrix(c, i), 'menu', {'menu': M.name})
        outcome = self.st.apply(tw, journal=False)
        assert not outcome.uncertain
        assert outcome.applied == expected_legal, 'label table disagrees with exact legality'
        row = dict(letter=[c, i], admissible=bool(outcome.applied))
        if not outcome.applied:
            return row, None
        assert outcome.moved == len(moved), 'applied count disagrees with exact moved set'
        if i:
            self.lab[moved] = updated_rows(self.st, moved)
        if full_heights:
            # Same independent full recomputation used by replay_word.py.
            self.h, self.b = BM.tree_info(self.st)
        elif i:
            self.h[moved], self.b[moved] = J.tree_rows(self.st, moved)
        row['moved'] = int(outcome.moved)
        if record:
            row.update(phi=J.phi(self.h), n4=int((self.h == 4).sum()), digest=self.st.digest())
        return row, old

    def undo(self, c, i, token):
        moved, h, b, lab = token
        assert J.twist(self.st, c, to_tuple(transpose(matrix(c, i))))
        self.h[moved], self.b[moved], self.lab[moved] = h, b, lab


def load_input(table, word='', labels=None):
    timings = {}
    start = time.process_time()
    st = BM.load_table(table)
    timings['load'] = dict(cpu_s=round(time.process_time() - start, 6))
    start = time.process_time()
    for c, i in parse_word(word):
        if not J.twist(st, c, matrix(c, i)):
            raise ValueError(f'--word: inadmissible letter {c}:{i}')
    timings['word'] = dict(cpu_s=round(time.process_time() - start, 6))
    start = time.process_time()
    h, b = BM.tree_info(st)
    timings['trees'] = dict(cpu_s=round(time.process_time() - start, 6))
    start = time.process_time()
    cache = external_path(labels) if labels else None
    lab = None
    if cache and cache.exists():
        with np.load(cache, allow_pickle=False) as data:
            if str(data['digest']) == st.digest():
                lab = data['labels']
                if lab.shape != (len(h), 600) or lab.dtype != np.uint8 or (lab > 2).any():
                    raise ValueError('invalid label cache')
    if lab is None:
        lab = BM.all_labels(st, workers=1)
        if cache:
            cache.parent.mkdir(parents=True, exist_ok=True)
            with cache.open('wb') as stream:
                np.savez_compressed(stream, labels=lab, digest=np.array(st.digest()))
    timings['labels'] = dict(cpu_s=round(time.process_time() - start, 6))
    return State(st, lab, h, b), timings


@contextmanager
def formal_image(st, c, i, pieces):
    """Apply just to I_c(Z), even at a blocked c; inverse uses the SAME set.

    No base label table is read until the inverse has restored Z. Callers check
    the canonical digest after their evaluation batch, including exceptional exits.
    """
    tw = BM.sim.Twist(C, c, matrix(c, i), 'menu', {'menu': M.name})
    if i:
        st._commit(tw, pieces, journal=False)
    try:
        yield
    finally:
        if i:
            st._commit(tw.inverse(), pieces, journal=False)


def image_data(state, c, i, pieces, x, hs, peers):
    before_phi = J.phi(state.h[pieces])
    with formal_image(state.st, c, i, pieces):
        h, b = J.tree_rows(state.st, pieces)
        labs = {d: J.labels_at(state.st, pieces, d) for d in sorted({x, *peers})}
        lx = labs[x]
        ds = state.oracle.local_counts(state.st, pieces, h, b, lx, x, hs)
        return dict(item=i, s=int((lx == 2).sum()),
                    f=int(((lx == 1) & (h >= 2) & (b != CLS[x])).sum()),
                    d=ds.tolist(), dphi=J.phi(h) - before_phi, labels=labs)


def topological(grips, edges):
    grips = sorted(grips)
    degree = dict.fromkeys(grips, 0)
    following = {c: [] for c in grips}
    for c, d in sorted(set(edges)):
        degree[d] += 1
        following[c].append(d)
    ready = [c for c in grips if degree[c] == 0]
    heapq.heapify(ready)
    order = []
    while ready:
        c = heapq.heappop(ready)
        order.append(c)
        for d in following[c]:
            degree[d] -= 1
            if degree[d] == 0:
                heapq.heappush(ready, d)
    return order if len(order) == len(grips) else None


def unit_members(state, piece):
    if type(piece) is not int or not 0 <= piece < len(state.h):
        raise ValueError('unit_piece must be a piece ID in the table')
    return np.flatnonzero(state.st.pose_id == state.st.pose_id[piece])


def check_round(state, spec):
    start = time.process_time()
    x, h, p = grip(spec['x']), item(spec['h']), spec['unit_piece']
    members = unit_members(state, p)
    raw = [(grip(c), item(i)) for c, i in spec['helpers']]
    helpers = sorted((c, i) for c, i in raw if i)
    errors = []
    # Duplicated grips make the indexed theorem data ill-defined. Name P0 and
    # stop before collapsing the duplicate into a dictionary.
    if len({c for c, _ in helpers}) != len(helpers):
        return dict(passed=False, failures=['P0: helper grips are not distinct'],
                    helpers=[list(a) for a in helpers])
    phi0, digest0 = J.phi(state.h), state.st.digest()
    high_own = bool((state.h[members] >= 4).all() and (state.b[members] == CLS[x]).all()
                    and (state.lab[members, x] == 1).all())
    lowers = high_own and h >= 12 and bool(state.oracle.lowers(
        state.st, state.st.pose_id[p], x, (h,), int(state.h[p]))[0])
    if not lowers:
        errors.append('unit not lowered: target must own the high unit and h must lower it')
    caps = [c for c, _ in helpers]
    inside = {c: state.inside(c) for c in caps}
    union = np.zeros(len(state.h), bool)
    for ps in inside.values():
        union[ps] = True
    intersections = [dict(grips=[c, d], size=int(np.intersect1d(inside[c], inside[d]).size))
                     for c, d in itertools.combinations(caps, 2)]
    g_intersections = {c: int(np.intersect1d(members, inside[c]).size) for c in caps}
    p0 = (x not in caps and not any(r['size'] for r in intersections)
          and not any(g_intersections.values()))
    if any(r['size'] for r in intersections):
        errors.append('P0: overlapping inside sets')
    images, helper_rows, edges = {}, [], set()
    p2 = True
    try:
        for c, i in helpers:
            ps = inside[c]
            f, d = state.budget(c, (i,))
            expected = 0 if i < 12 else 2 * (f - int(d[0]))
            im = image_data(state, c, i, ps, x, (h,), caps)
            images[c] = im
            tame = expected <= 0 and expected == im['dphi']
            p2 &= tame
            helper_rows.append(dict(grip=c, item=i, inside_size=len(ps),
                                    unit_intersection=g_intersections[c],
                                    clean=not bool((state.lab[:, c] == 2).any()),
                                    F_at_helper=f, D_at_helper=int(d[0]),
                                    delta_phi=im['dphi'], delta_phi_formula=expected,
                                    tame=tame, s=im['s'], f=im['f'], d=im['d'][0]))
        for c in caps:
            for d in caps:
                if c != d and (images[d]['labels'][c] != 0).any():
                    edges.add((c, d))
        for row in helper_rows:
            c = row['grip']
            straddlers = np.flatnonzero(state.lab[:, c] == 2)
            row['straddlers'] = len(straddlers)
            row['uncovered_straddlers'] = int((~union[straddlers]).sum())
            row['straddlers_not_sent_out'] = 0
            row['evacuators'] = []
            for d in caps:
                shared = np.isin(inside[d], straddlers)
                if shared.any():
                    edges.add((d, c))
                    bad = int((images[d]['labels'][c][shared] != 0).sum())
                    row['straddlers_not_sent_out'] += bad
                    row['evacuators'].append(dict(grip=d, pieces=int(shared.sum()), not_out=bad))
            p0 &= not (row['uncovered_straddlers'] or row['straddlers_not_sent_out'])
    finally:
        assert state.st.digest() == digest0, 'helper evaluation batch did not undo'
    order = topological(caps, edges)
    s0 = int(((state.lab[:, x] == 2) & ~union).sum())
    f0 = int(((state.lab[:, x] == 1) & ~union & (state.h >= 2) & (state.b != CLS[x])).sum())
    remaining = np.flatnonzero((state.lab[:, x] == 1) & ~union)
    d0 = int(state.oracle.counts(state.st, state.h, state.b, remaining, x, (h,))[0])
    s = s0 + sum(r['s'] for r in helper_rows)
    f = f0 + sum(r['f'] for r in helper_rows)
    d = d0 + sum(r['d'] for r in helper_rows)
    delta_helpers = sum(r['delta_phi'] for r in helper_rows)
    predicted = phi0 + delta_helpers + 2 * (f - d)
    hypotheses = dict(P0=bool(p0), P1=order is not None, P2=bool(p2), P3=s == 0, P4=f < d)
    for name, passed in hypotheses.items():
        if not passed and not any(s.startswith(name + ':') for s in errors):
            errors.append(name + ': hypothesis failed')
    result = dict(x=x, target_class=int(CLS[x]), unit_piece=p, unit_size=len(members),
                  unit_height=int(state.h[p]), unit_branch=int(state.b[p]), h=h, digest_before=digest0,
                  phi_before=phi0, dropped_identities=[list(a) for a in sorted(raw) if a[1] == 0],
                  hypotheses=hypotheses, unit_lowering=bool(lowers), helpers=helper_rows,
                  intersections=intersections, edges=[list(e) for e in sorted(edges)], order=order,
                  s0=s0, f0=f0, d0=d0, S=s, F=f, D=d,
                  helper_delta_phi=delta_helpers, predicted_phi=predicted,
                  predicted_delta_phi=predicted - phi0, replay=[])
    if not errors:
        items = dict(helpers)
        for c in order:
            old_phi = J.phi(state.h)
            row, token = state.apply(c, items[c], full_heights=True)
            row['moved_set_equal'] = token is not None and np.array_equal(token[0], inside[c])
            row['delta_phi_equal'] = token is not None and row['phi'] - old_phi == images[c]['dphi']
            result['replay'].append(row)
            if not row['admissible'] or not row['moved_set_equal'] or not row['delta_phi_equal']:
                errors.append('replay: helper admissibility, moved set or delta mismatch')
                break
        result['helper_phi_equal'] = J.phi(state.h) == phi0 + delta_helpers
        if not result['helper_phi_equal']:
            errors.append('replay: helper Phi mismatch')
        held = bool((state.lab[members, x] == 1).all())
        result['unit_held_before_lowering'] = held
        if not held:
            errors.append('replay: target does not hold every unit member')
        if not errors:
            expected_inside = state.inside(x)
            before_h = state.h[members].copy()
            row, token = state.apply(x, h, full_heights=True)
            row['moved_set_equal'] = token is not None and np.array_equal(token[0], expected_inside)
            row['every_member_lowered'] = bool((state.h[members] == before_h - 2).all())
            result['replay'].append(row)
            if not row['admissible'] or not row['moved_set_equal']:
                errors.append('replay: lowering inadmissible or moved set mismatch')
            if not row['every_member_lowered']:
                errors.append('unit not lowered: replay')
            if J.phi(state.h) != predicted:
                errors.append('replay: predicted Phi mismatch')
    result.update(passed=not errors, failures=errors, cpu_s=round(time.process_time() - start, 6))
    return result


def check(state, certificate):
    start = time.process_time()
    failures = []
    if certificate.get('format') != 't3p-certificate/1':
        failures.append('format: expected t3p-certificate/1')
    if certificate.get('table_digest') != state.st.digest():
        failures.append('table_digest: certificate is for a different state')
    if not isinstance(certificate.get('rounds'), list) or not certificate.get('rounds'):
        failures.append('rounds: expected a non-empty list')
    rounds = []
    if not failures:
        for spec in certificate['rounds']:
            try:
                result = check_round(state, spec)
            except (ValueError, KeyError, TypeError) as exc:
                result = dict(passed=False, failures=['certificate: ' + str(exc)])
            rounds.append(result)
            if not result['passed']:
                failures.extend(result['failures'])
                break
    return dict(format='t3p-check/1', passed=not failures, failures=failures, rounds=rounds,
                digest_after=state.st.digest(), cpu_s=round(time.process_time() - start, 6))


def bits(pieces):
    mask = np.zeros(len(BM.C.data.orbit_id), np.uint8)
    mask[pieces] = 1
    return int.from_bytes(np.packbits(mask, bitorder='little').tobytes(), 'little')


def maximal_covers(candidates, sets, required):
    """Bron--Kerbosch on disjointness, including the empty maximal cover.

    Every disjoint subset extends to a maximal one. Identity options on the
    extension recover the original subset, so maximal covers lose no round.
    """
    neighbours = {c: {d for d in candidates if c != d and not (sets[c] & sets[d])}
                  for c in candidates}

    def visit(chosen, available, excluded, covered):
        if not available and not excluded:
            if required & ~covered == 0:
                yield tuple(sorted(chosen))
            return
        pivot = min(available | excluded,
                    key=lambda c: (-len(available & neighbours[c]), c))
        for c in sorted(available - neighbours[pivot]):
            yield from visit(chosen + [c], available & neighbours[c],
                             excluded & neighbours[c], covered | sets[c])
            available.remove(c)
            excluded.add(c)

    yield from visit([], set(candidates), set(), 0)


def pose_counts(state, pieces):
    ids, counts = np.unique(state.st.pose_id[pieces], return_counts=True)
    return [dict(pose=int(pid), count=int(n)) for pid, n in zip(ids, counts)]


def high_units(state):
    for pid in np.unique(state.st.pose_id[state.h >= 4]).tolist():
        members = np.flatnonzero(state.st.pose_id == pid)
        own = np.flatnonzero((state.lab[members] == 1).all(axis=0) & (CLS == state.b[members[0]]))
        yield pid, members, own.tolist()


def single_descents(state):
    rows = []
    for c in np.flatnonzero(~(state.lab == 2).any(axis=0)).tolist():
        f, ds = state.budget(c)
        for i, d in zip(JUMBLES, ds.tolist()):
            if f < d:
                rows.append(dict(grip=c, item=i, F=f, D=d, delta_phi=2 * (f - d)))
    return rows


def helper_options(state, x, c, hs, peers):
    ps, digest0 = state.inside(c), state.st.digest()
    f, ds = state.budget(c)
    options = []
    try:
        for i in range(60):
            expected = 0 if i < 12 else 2 * (f - int(ds[i - 12]))
            if expected > 0:
                continue
            im = image_data(state, c, i, ps, x, hs, peers)
            assert im['dphi'] == expected, 'one-letter image violates Lemma Phi'
            if im['s']:
                continue
            before = [d for d in peers if (im['labels'][d] != 0).any()]
            options.append(dict(item=i, f=im['f'], d=im['d'], dphi=im['dphi'], before=before))
    finally:
        assert state.st.digest() == digest0, 'item evaluation batch did not undo'
    return options


def assignments(cover, options, column):
    """Exact optimisation, with a separate budget coordinate.

    With no possible edges, dynamic programming retains the smallest helper
    delta at EVERY F-D value; minimising total delta alone could violate P4.
    With possible edges, enumerate the complete product, drop identities and
    check the actual assignment's digraph. No search or time cutoff is used.
    """
    active_possible = {c for c in cover if any(o['item'] for o in options[c])}
    has_edges = any(o['item'] and active_possible.intersection(o['before'])
                    for c in cover for o in options[c])
    if has_edges:
        for chosen in itertools.product(*(options[c] for c in cover)):
            active = {c for c, o in zip(cover, chosen) if o['item']}
            edges = {(d, c) for c, o in zip(cover, chosen) if c in active
                     for d in o['before'] if d in active}
            order = topological(active, edges)
            if order is not None:
                yield (sum(o['f'] - o['d'][column] for o in chosen),
                       sum(o['dphi'] for o in chosen), chosen, order)
        return
    frontier = {0: (0, (), ())}
    for c in cover:
        nxt = {}
        for budget, (delta, item_tuple, chosen) in frontier.items():
            for o in options[c]:
                nb = budget + o['f'] - o['d'][column]
                candidate = (delta + o['dphi'], item_tuple + (o['item'],), chosen + (o,))
                if nb not in nxt or candidate[:2] < nxt[nb][:2]:
                    nxt[nb] = candidate
        frontier = nxt
    for budget, (delta, _, chosen) in sorted(frontier.items()):
        yield budget, delta, chosen, [c for c, o in zip(cover, chosen) if o['item']]


def helper_item_key(helpers):
    """Tie-break by the grip-indexed item vector; omitted helpers are identity.

    A fixed-length vector makes the tie-break consistent across different
    maximal covers and with the independent-cover dynamic programme.
    """
    items = dict(helpers)
    return tuple(items.get(c, 0) for c in range(600))


FAMILY = ('All high pose units and their whole own holders (restricted to --targets when given); '
          'clean helpers distinct from the target, interacting by exact L1, avoiding every unit member, '
          'holding an obstruction piece or a branch-Q piece of height >=4; every maximal pairwise '
          'disjoint inside-set cover of S_x; all 60 items including dropped identities, subject to '
          'per-letter tameness and zero image straddlers; every tree-computed lowering item; '
          'assignment-specific acyclic precedence. This does not exhaust overlapping helpers, '
          'blocked helpers (T3P+), repeated helper grips, or multi-round policies.')


def scan(state, targets=None):
    start = time.process_time()
    digest0, phi0 = state.st.digest(), J.phi(state.h)
    clean = ~(state.lab == 2).any(axis=0)
    clean_caps = np.flatnonzero(clean).tolist()
    _, _, partners = graph()
    inside = {c: state.inside(c) for c in clean_caps}
    sets = {c: bits(ps) for c, ps in inside.items()}
    option_cache, rows = {}, []
    best_overall = None
    singles = single_descents(state)
    for pid, members, holders in high_units(state):
        for x in holders:
            if targets is not None and x not in targets:
                continue
            base = dict(pose=pid, unit_piece=int(members[0]), unit_size=len(members), x=x)
            sx = np.flatnonzero(state.lab[:, x] == 2)
            missing = sx[~(state.lab[np.ix_(sx, clean_caps)] == 1).any(axis=1)]
            if len(missing):
                rows.append(dict(base, status='straddler-without-clean-holder',
                                 count=len(missing), pose_groups=pose_counts(state, missing)))
                continue
            obstruction = ((state.lab[:, x] == 2)
                           | ((state.lab[:, x] == 1) & (state.h >= 2) & (state.b != CLS[x])))
            useful = obstruction | ((state.h >= 4) & (state.b == CLS[x]))
            candidates = [c for c in partners[x] if clean[c]
                          and not (state.lab[members, c] == 1).any() and useful[inside[c]].any()]
            overlap = [dict(grips=[c, d], size=(sets[c] & sets[d]).bit_count())
                       for c, d in itertools.combinations(candidates, 2) if sets[c] & sets[d]]
            hs = [i for i, yes in zip(JUMBLES, state.oracle.lowers(
                state.st, pid, x, JUMBLES, int(state.h[members[0]]))) if yes]
            assert len(hs) == 12, 'high own unit must have twelve tree-computed lowering items'
            options = {}
            cover_count = feasible_covers = feasible_assignments = 0
            best_round = best_budget = None
            for cover in maximal_covers(candidates, sets, bits(sx)):
                cover_count += 1
                for c in cover:
                    if c not in options:
                        peers = tuple(d for d in candidates if d != c and d in partners[c]
                                      and not sets[c] & sets[d])
                        key = (x, c, tuple(hs), peers)
                        if key not in option_cache:
                            option_cache[key] = helper_options(state, x, c, hs, peers)
                        options[c] = option_cache[key]
                if any(not options[c] for c in cover):
                    continue
                union = np.zeros(len(state.h), bool)
                for c in cover:
                    union[inside[c]] = True
                s0 = int(((state.lab[:, x] == 2) & ~union).sum())
                if s0:
                    continue
                remaining = np.flatnonzero((state.lab[:, x] == 1) & ~union)
                f0 = int(((state.h[remaining] >= 2) & (state.b[remaining] != CLS[x])).sum())
                d0 = state.oracle.counts(state.st, state.h, state.b, remaining, x, hs)
                cover_feasible = False
                for j, h in enumerate(hs):
                    for budget, delta, chosen, order in assignments(cover, options, j):
                        feasible_assignments += 1
                        cover_feasible = True
                        value = f0 - int(d0[j]) + budget
                        helpers = [(c, o['item']) for c, o in zip(cover, chosen) if o['item']]
                        predicted = delta + 2 * value
                        witness = dict(x=x, unit_piece=int(members[0]), h=h,
                                       helpers=[list(a) for a in helpers], order=order,
                                       F=f0 + sum(o['f'] for o in chosen),
                                       D=int(d0[j]) + sum(o['d'][j] for o in chosen),
                                       F_minus_D=value, helper_delta_phi=delta,
                                       predicted_delta_phi=predicted, predicted_phi=phi0 + predicted)
                        budget_key = (value, predicted, h, helper_item_key(helpers))
                        if best_budget is None or budget_key < best_budget[0]:
                            best_budget = (budget_key, witness)
                        round_key = (predicted, h, helper_item_key(helpers))
                        if value < 0 and (best_round is None or round_key < best_round[0]):
                            best_round = (round_key, witness)
                feasible_covers += cover_feasible
            detail = dict(candidates=candidates, overlaps=overlap, covers=cover_count,
                          feasible_covers=feasible_covers, feasible_frontier_records=feasible_assignments,
                          lowering_items=hs)
            if best_round:
                witness = best_round[1]
                rows.append(dict(base, **detail, status='round', best=witness))
                key = (*best_round[0], x, int(members[0]))
                if best_overall is None or key < best_overall[0]:
                    best_overall = (key, witness)
            elif not cover_count:
                rows.append(dict(base, **detail, status='no-disjoint-cover'))
            elif best_budget is None:
                rows.append(dict(base, **detail, status='no-feasible-items'))
            else:
                rows.append(dict(base, **detail, status='budget', best=best_budget[1]))
    assert state.st.digest() == digest0, 'scan did not restore its state'
    certificate = None
    if best_overall:
        w = best_overall[1]
        certificate = dict(format='t3p-certificate/1', table_digest=digest0,
                           rounds=[{k: w[k] for k in ('x', 'unit_piece', 'h', 'helpers')}])
    return dict(format='t3p-scan/1', digest=digest0, phi=phi0, family_exhausted=FAMILY,
                tie_break='predicted delta, h, item vector at grips 0..599 (omitted = identity)',
                targets=sorted(targets) if targets is not None else None, exhausted=True,
                rows=rows, single_letter_descents=singles, best_certificate=certificate,
                cpu_s=round(time.process_time() - start, 6))


def pairs(state, x, y, z, items_y=None, items_z=None):
    start = time.process_time()
    if y == z:
        raise ValueError('pairs requires two distinct helper grips')
    choices = {y: sorted(set(range(60) if items_y is None else items_y)),
               z: sorted(set(range(60) if items_z is None else items_z))}
    for c in (x, y, z):
        grip(c)
    for values in choices.values():
        for i in values:
            item(i)
    digest0 = state.st.digest()
    rows, best = [], None
    counts = dict(words=0, admissible=0, tame=0, clean=0)
    for a, b in ((y, z), (z, y)):
        first_legal = not bool((state.lab[:, a] == 2).any())
        f1, d1 = state.budget(a)
        for i in choices[a]:
            tame1 = i < 12 or f1 <= int(d1[i - 12])
            token1 = None
            try:
                if first_legal:
                    _, token1 = state.apply(a, i, record=False)
                second_legal = first_legal and not bool((state.lab[:, b] == 2).any())
                # These are the intermediate contents, even when the caps overlap.
                f2, d2 = state.budget(b) if first_legal else (None, None)
                prefix_digest = state.st.digest()
                try:
                    for j in choices[b]:
                        counts['words'] += 1
                        tame2 = bool(second_legal and (j < 12 or f2 <= int(d2[j - 12])))
                        admissible = bool(first_legal and second_legal)
                        tame = bool(admissible and tame1 and tame2)
                        row = dict(word=[[a, i], [b, j]], admissible=admissible, tame=tame,
                                   first_tame=bool(tame1), second_tame=tame2,
                                   first_F=f1, first_D=0 if i < 12 else int(d1[i - 12]),
                                   second_F=f2, second_D=(0 if j < 12 else int(d2[j - 12]))
                                   if first_legal else None)
                        counts['admissible'] += admissible
                        counts['tame'] += tame
                        if tame:
                            _, token2 = state.apply(b, j, record=False)
                            try:
                                s = int((state.lab[:, x] == 2).sum())
                                f, ds = state.budget(x)
                                d = int(ds.max())
                                lower = [k for k, v in zip(JUMBLES, ds.tolist()) if v == d]
                                row.update(S=s, F=f, max_D=d, F_minus_D=f - d,
                                           maximizing_items=lower, phi=J.phi(state.h))
                                counts['clean'] += s == 0
                                key = (f - d, ((a, i), (b, j)))
                                if s == 0 and (best is None or key < best[0]):
                                    best = (key, dict(row))
                            finally:
                                state.undo(b, j, token2)
                        rows.append(row)
                finally:
                    assert state.st.digest() == prefix_digest, 'second-letter batch did not undo'
            finally:
                if token1 is not None:
                    state.undo(a, i, token1)
                assert state.st.digest() == digest0, 'pair batch did not undo'
    return dict(format='t3p-pairs/1', digest=digest0, x=x, y=y, z=z,
                items_y=choices[y], items_z=choices[z], counts=counts,
                metric_scope='S, F and D and clean count cover admissible per-letter-tame words; '
                             'admissible count includes non-tame words. Identity is a menu letter '
                             'and still requires a clean grip, as in the exact simulator.',
                rows=rows, best=best[1] if best else None,
                cpu_s=round(time.process_time() - start, 6))


def plus(state, limit=None):
    start = time.process_time()
    if limit is not None and limit < 1:
        raise ValueError('--limit must be positive')
    digest0 = state.st.digest()
    clean = ~(state.lab == 2).any(axis=0)
    clean_caps = np.flatnonzero(clean).tolist()
    inside = {c: state.inside(c) for c in range(600)}
    budgets = {}
    found, tested_pairs, tested_items = [], 0, 0
    exhausted = True
    for y in np.flatnonzero(~clean).tolist():
        sy = np.flatnonzero(state.lab[:, y] == 2)
        for z in clean_caps:
            if not (state.lab[sy, z] == 1).all() or np.intersect1d(inside[y], inside[z]).size:
                continue
            tested_pairs += 1
            if z not in budgets:
                budgets[z] = state.budget(z)
            f, ds = budgets[z]
            try:
                for i in range(1, 60):  # identity cannot send a non-empty S_y out
                    delta = 0 if i < 12 else 2 * (f - int(ds[i - 12]))
                    if delta > 0:
                        continue
                    tested_items += 1
                    with formal_image(state.st, z, i, inside[z]):
                        labels = J.labels_at(state.st, inside[z], y)
                    if labels.any():
                        continue
                    before = inside[y]
                    phi0 = J.phi(state.h)
                    replay, token = state.apply(z, i, full_heights=True)
                    try:
                        direct = J.labels_at(state.st, np.arange(len(state.h)), y)
                        passed = bool(replay['admissible'] and not (direct == 2).any()
                                      and np.array_equal(np.flatnonzero(direct == 1), before)
                                      and np.array_equal(direct, state.lab[:, y])
                                      and J.phi(state.h) - phi0 == delta)
                        found.append(dict(y=y, z=z, item=i, straddlers=len(sy),
                                          inside_y=len(before), inside_z=len(inside[z]),
                                          delta_phi=delta, replay=replay, passed=passed))
                    finally:
                        if token is not None:
                            state.undo(z, i, token)
                    if not passed or (limit is not None and len(found) >= limit):
                        exhausted = False
                        break
            finally:
                assert state.st.digest() == digest0, 'plus evaluation batch did not undo'
            if not exhausted:
                break
        if not exhausted:
            break
    return dict(format='t3p-plus/1', digest=digest0, instances=found,
                passed=all(r['passed'] for r in found), exhausted=exhausted,
                tested_pairs=tested_pairs, tested_items=tested_items, limit=limit,
                cpu_s=round(time.process_time() - start, 6))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest='command', required=True)
    for command in ('check', 'scan', 'pairs', 'plus'):
        parser = sub.add_parser(command)
        parser.add_argument('table')
        if command == 'check':
            parser.add_argument('certificate')
        elif command == 'pairs':
            for name in ('x', 'y', 'z'):
                parser.add_argument(name, type=int)
        parser.add_argument('out')
        parser.add_argument('--labels')
        if command != 'check':
            parser.add_argument('--word', default='')
        if command == 'scan':
            parser.add_argument('--targets')
        if command == 'pairs':
            parser.add_argument('--items-y')
            parser.add_argument('--items-z')
        if command == 'plus':
            parser.add_argument('--limit', type=int)
    args = ap.parse_args(argv)
    external_path(args.out)
    state, setup = load_input(args.table, getattr(args, 'word', ''), args.labels)
    if args.command == 'check':
        result = check(state, json.loads(Path(args.certificate).read_text()))
    elif args.command == 'scan':
        targets = {grip(int(c)) for c in args.targets.split(',')} if args.targets else None
        result = scan(state, targets)
        if result['best_certificate'] is not None:
            sidecar = Path(args.out).with_suffix('.certificate.json')
            if sidecar.resolve() == Path(args.out).resolve():
                sidecar = Path(str(args.out) + '.certificate.json')
            write_json(sidecar, result['best_certificate'])
    elif args.command == 'pairs':
        result = pairs(state, args.x, args.y, args.z, parse_items(args.items_y), parse_items(args.items_z))
    else:
        result = plus(state, args.limit)
    result['setup'] = setup
    result['prefix_word'] = [list(a) for a in parse_word(getattr(args, 'word', ''))]
    write_json(args.out, result)
    return 0 if result.get('passed', True) else 1


if __name__ == '__main__':
    sys.exit(main())
