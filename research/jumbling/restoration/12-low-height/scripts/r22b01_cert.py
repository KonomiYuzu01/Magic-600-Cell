# R22B-01: bounded three-cycle seed search, two letter trees, and an exact vacating word.
# From the repository root: python3 -I -B r22b01_cert.py <census record> <target json> <out json> [--seconds 900]
import time

T0 = time.monotonic()
import argparse
import hashlib
import json
import math
import random
import signal
import sys
from collections import deque
from pathlib import Path
from types import SimpleNamespace

sys.dont_write_bytecode = True
np = None                         # Loaded after the command's wall-clock alarm is armed.


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.monotonic() - T0, 2))), flush=True)


class Deadline(Exception):
    pass


def expired(*_):
    raise Deadline()


def check_time(deadline):
    if time.monotonic() >= deadline:
        raise Deadline()


def inverse(G, word):
    return [int(G.inv[n]) for n in reversed(word)]


def extend_reduced(G, dest, word):
    for n in word:
        if dest and int(G.inv[n]) == dest[-1]:
            dest.pop()
        else:
            dest.append(int(n))


def cycle_data(p):
    """Least-point labels and cycle lengths for a local exact permutation."""
    labels = np.arange(len(p))
    q = p.copy()
    for _ in range(len(p).bit_length()):
        labels = np.minimum(labels, labels[q])
        q = q[q]
    return labels, np.bincount(labels, minlength=len(p))


def commutator_action(G, left, right, caps, membership):
    """Restrict [left, right] using its entire possible moved support.

    If I is the intersection of the two supports, the commutator's support
    lies in I union left^-1(I) union right^-1(I). Caps are support supersets;
    their unions are invariant, so the same bound applies to them. Including
    fixed points is harmless. No moved point in membership is omitted.
    """
    xs = G.capx[G.letters[left][0]]
    keep = np.zeros(len(xs), bool)
    for c in caps:
        keep |= G.home[xs, c]
    I = xs[keep & membership[xs]]
    if not len(I):
        return I, np.zeros(0, np.int64)
    points = np.unique(np.concatenate((I, G.images([int(G.inv[left])], I),
                                      G.images(inverse(G, right), I))))
    word = [left] + right + [int(G.inv[left])] + inverse(G, right)
    images = G.images(word, points)
    p = np.searchsorted(points, images)
    if (p >= len(points)).any() or not np.array_equal(points[p], images):
        raise AssertionError('the commutator left its support bound')
    return points, p


def seed_power(points, p, want_three):
    """The power killing all cycles except one 3-cycle, or no such power."""
    labels, lengths = cycle_data(p)
    thirds = np.flatnonzero((lengths != 0) & (lengths % 3 == 0))
    if want_three:
        if len(thirds) != 1 or lengths[thirds[0]] != 3:
            return None
        triple = points[labels == thirds[0]].tolist()
    else:
        if len(thirds):
            return None
        triple = []
    m = 1
    for length in np.unique(lengths[lengths > 1]).tolist():
        if length != 3:
            m = math.lcm(m, length)
    return m, triple


def overlap_counts(G, D, safe, deadline):
    """Exact cap intersection sizes; do not build a dense letter action."""
    counts = np.empty((len(safe), len(safe)), np.int32)
    for j, c in enumerate(safe):
        check_time(deadline)
        xs = D[G.home[D, c]]
        counts[j] = G.home[np.ix_(xs, safe)].sum(axis=0)
    return counts


def find_seed(G, D, earlier, counts, prior_counts, safe, deadline, rng):
    """Try two-letter and then three-letter commutators within this share."""
    current = np.zeros(G.NP, bool)
    current[D] = True
    before = np.zeros(G.NP, bool)
    before[earlier] = True
    u, v = np.where(np.triu(counts > 0, 1))
    if not len(u):
        return None, 0
    # Far cap overlaps are good sources of single 3-cycles. Random candidates
    # interleaved with this prefix also cover larger, non-separated overlaps.
    tie = list(range(len(u)))
    rng.shuffle(tie)
    tie.sort(key=lambda j: (int(counts[u[j], v[j]]), int(prior_counts[u[j], v[j]])))
    prefix = tie[:256]
    best, trials = None, 0
    start = time.monotonic()
    double_end = start + 0.45 * max(0, deadline - start)
    for arity, end in ((2, double_end), (3, deadline)):
        trial = 0
        while time.monotonic() < end:
            if trial < 12 * len(prefix):
                j = prefix[trial // 12]
            else:
                j = rng.randrange(len(u))
            c, d = int(safe[u[j]]), int(safe[v[j]])
            x = rng.choice(G.by_grip[c])
            right = [rng.choice(G.by_grip[d])]
            caps = [d]
            if arity == 3:
                # Mix both local perturbations and arbitrary SAFE letters.
                if trial % 2:
                    I = D[G.home[D, c] & G.home[D, d]]
                    r = int(rng.choice(I))
                    choices = safe[G.home[r, safe]]
                    e = int(rng.choice(choices))
                else:
                    e = int(rng.choice(safe))
                right.append(rng.choice(G.by_grip[e]))
                caps.append(e)
            trial += 1
            trials += 1
            pts, p = commutator_action(G, x, right, caps, current)
            candidate = seed_power(pts, p, True)
            if candidate is None:
                continue
            m, triple = candidate
            if len(earlier):
                ep, eq = commutator_action(G, x, right, caps, before)
                prior = seed_power(ep, eq, False)
                if prior is None:
                    continue
                m = math.lcm(m, prior[0])
            base = []
            extend_reduced(G, base, [x] + right + [int(G.inv[x])] + inverse(G, right))
            length = len(base) * m
            # Materialising more would consume the word/output memory budget.
            # This is a bounded witness search, never a nonexistence claim.
            if not length or length > 20000 or (best is not None and length >= len(best[0])):
                continue
            word = base * m
            a = min(triple)
            b = G.image(word, a)
            c = G.image(word, b)
            anchors = [a, b, c]
            verify_seed(G, D, earlier, word, anchors)
            best = word, anchors
            log(seed_candidate=int(D[0]), s=len(word), arity=arity, trials=trials)
            if length == 4:       # The shortest possible two-letter commutator.
                return best, trials
    return best, trials


def verify_seed(G, D, earlier, word, anchors):
    a, b, c = anchors
    if len({a, b, c}) != 3 or not set(anchors) <= set(D.tolist()):
        raise AssertionError('seed anchors are not distinct points of the orbit')
    points = np.concatenate((D, earlier))
    want = points.copy()
    for x, y in ((a, b), (b, c), (c, a)):
        want[np.searchsorted(D, x)] = y
    if not np.array_equal(G.images(word, points), want):
        raise AssertionError('seed moves another chamber of its orbit or an earlier orbit')


def orbit_edges(G, D, deadline):
    """Sparse letter edges on one orbit, shared by its two BFS searches."""
    local = np.full(G.NP, -1, np.int32)
    local[D] = np.arange(len(D))
    src, dst, letters = [], [], []
    for c in G.by_grip:
        check_time(deadline)
        xs = D[G.home[D, c]]
        if not len(xs):
            continue
        pos = np.searchsorted(G.capx[c], xs)
        for n in G.by_grip[c]:
            ys = G.letters[n][3][pos]
            moved = xs != ys
            src.append(local[xs[moved]])
            dst.append(local[ys[moved]])
            letters.append(np.full(int(moved.sum()), n, np.int32))
    if not src:
        empty = np.zeros(0, np.int32)
        return empty, empty, empty
    return np.concatenate(src), np.concatenate(dst), np.concatenate(letters)


def letter_tree(G, D, edges, root, fixed, forward, deadline):
    """BFS from root; P stores inverse edges, Q stores forward edges."""
    allowed = np.ones(len(G.letters), bool)
    for n in range(len(G.letters)):
        if n % 128 == 0:
            check_time(deadline)
        allowed[n] = np.array_equal(G.images([n], fixed), fixed)
    src, dst, letters = edges
    keep = allowed[letters]
    src, dst, letters = src[keep], dst[keep], letters[keep]
    order = np.argsort(src, kind='stable')
    src, dst, letters = src[order], dst[order], letters[order]
    offsets = np.searchsorted(src, np.arange(len(D) + 1))
    root_local = int(np.searchsorted(D, root))
    parent = np.full(len(D), -1, np.int32)
    via = np.full(len(D), -1, np.int32)
    depth = np.full(len(D), -1, np.int32)
    parent[root_local], depth[root_local] = root_local, 0
    queue = deque([root_local])
    while queue:
        x = queue.popleft()
        check_time(deadline)
        lo, hi = offsets[x:x + 2]
        # All neighbours of x get the same BFS distance. Unique destinations
        # avoid revisiting the many parallel cap-letter edges in Python.
        js = np.flatnonzero(parent[dst[lo:hi]] < 0) + lo
        if not len(js):
            continue
        _, first = np.unique(dst[js], return_index=True)
        js = js[first]
        ys = dst[js]
        parent[ys], via[ys], depth[ys] = x, letters[js], depth[x] + 1
        queue.extend(ys.tolist())
    members = ~np.isin(D, fixed)
    if (parent[members] < 0).any():
        return None, int((parent[members] < 0).sum())
    rows = []
    for x in np.flatnonzero(members).tolist():
        if x == root_local:
            continue
        n = int(via[x]) if not forward else int(G.inv[via[x]])
        grip, item = G.pairs([n])[0]
        rows.append([int(D[x]), grip, item, int(D[parent[x]])])
    return (rows, int(depth[members].max())), 0


def tree_word(table, point, root, forward):
    word = []
    while point != root:
        n, point = table[point]
        word.append(n)
    return word if forward else word[::-1]


def decode_tree(G, rows):
    return {x: (G.index[(g, i)], y) for x, g, i, y in rows}


def anchored_cycle(G, anchors, seed, P, Q, x, y):
    a, b, c = anchors
    px = tree_word(P, x, b, True)
    w = G.image(px, y)
    qw = tree_word(Q, w, c, False)
    U = px + inverse(G, qw)
    return U + seed + inverse(G, U)


def three_cycle(G, row, seed, P, Q, p, q, r):
    a = row['anchors'][0]
    if a in (p, q, r):
        cycle = [p, q, r]
        j = cycle.index(a)
        x, y = cycle[(j + 1) % 3], cycle[(j + 2) % 3]
        return anchored_cycle(G, row['anchors'], seed, P, Q, x, y)
    return (anchored_cycle(G, row['anchors'], seed, P, Q, p, q) +
            anchored_cycle(G, row['anchors'], seed, P, Q, r, p))


def vacate_orbit(G, D, Tmask, row, occupied, V, deadline):
    seed = G.numbers(row['seed'])
    P, Q = decode_tree(G, row['P']), decode_tree(G, row['Q'])
    targets = D[Tmask[D]]
    steps = 0
    while occupied[targets].any():
        check_time(deadline)
        t = int(targets[np.flatnonzero(occupied[targets])[0]])
        choices = D[~Tmask[D] & ~occupied[D]]
        if not len(choices):
            raise AssertionError('N2 did not supply a hole outside T')
        z = int(choices[0])
        third = D[~Tmask[D] & (D != z)]
        if not len(third):
            third = D[(D != z) & (D != t)]
        r = int(third[0])
        triple = (z, t, r) if occupied[r] else (z, r, t)
        step = three_cycle(G, row, seed, P, Q, *triple)
        p = G.perm(step)
        expected = occupied[D].copy()
        expected[np.searchsorted(D, z)] = True
        expected[np.searchsorted(D, t)] = False
        new = np.empty_like(occupied)
        new[p] = occupied          # Entire permutation, including later orbits.
        if not np.array_equal(new[D], expected):
            raise AssertionError('three-cycle did not move exactly the selected hole to t')
        occupied[:] = new
        extend_reduced(G, V, step)
        steps += 1
    return steps


def build(G, target, result, deadline):
    orbit, sizes = G.orbits()
    T = np.array(target['T'], np.int64)
    if ((T < 0) | (T >= G.NP)).any() or len(np.unique(T)) != len(T):
        raise ValueError('target chamber IDs are out of range or duplicated')
    Tmask = np.zeros(G.NP, bool)
    Tmask[T] = True
    labels = sorted(set(orbit[T].tolist()), key=lambda o: (-int(sizes[o]), o))
    pending = {int(np.flatnonzero(orbit == o)[0]): 'seed search not reached within time bound' for o in labels}
    result['missing'] = [{'least': least, 'reason': reason} for least, reason in pending.items()]
    safe = np.array(sorted(G.by_grip), np.int64)
    prior_counts = np.zeros((len(safe), len(safe)), np.int32)
    earlier = np.zeros(0, np.int64)
    occupied = G.occupied.copy()
    V = []
    rng = random.Random(2201)
    for j, lab in enumerate(labels):
        check_time(deadline)
        D = np.flatnonzero(orbit == lab)
        least, k = int(D[0]), int(Tmask[D].sum())
        pending[least] = 'time bound reached during seed search'
        result['missing'] = [{'least': p, 'reason': reason} for p, reason in pending.items()]
        if len(D) < 5 or int((~occupied[D]).sum()) < k:
            pending[least] = 'N1/N2 or the degree >= 5 hypothesis fails'
        else:
            counts = overlap_counts(G, D, safe, deadline)
            # Reserve part of each remaining share for trees, composition and
            # serialisation. Loading and all these operations share one alarm.
            share = max(0, (deadline - time.monotonic() - 5) / (len(labels) - j))
            search_end = min(deadline - 5, time.monotonic() + 0.72 * share)
            found, trials = find_seed(G, D, earlier, counts, prior_counts, safe, search_end, rng)
            if found is None:
                pending[least] = 'no seed found in bounded two-/three-letter commutator search (expanded seed limit 20000 letters)'
                log(least=least, d=len(D), k=k, seed=False, trials=trials)
            else:
                seed, anchors = found
                pending[least] = 'time bound reached before both letter tables and the orbit word were complete'
                result['missing'] = [{'least': p, 'reason': reason} for p, reason in pending.items()]
                edges = orbit_edges(G, D, deadline)
                first, pmissing = letter_tree(G, D, edges, anchors[1], anchors[:1], True, deadline)
                second, qmissing = letter_tree(G, D, edges, anchors[2], anchors[:2], False, deadline)
                del edges
                if pmissing or qmissing:
                    pending[least] = f'disconnected letter table: P misses {pmissing}, Q misses {qmissing}'
                else:
                    P, e = first
                    Q, f = second
                    row = {'least': least, 'size': len(D), 'k': k, 'anchors': anchors,
                           'seed': G.pairs(seed), 'P': P, 'Q': Q}
                    steps = vacate_orbit(G, D, Tmask, row, occupied, V, deadline)
                    result['orbits'].append(row)
                    result['bound'] += 2 * k * (len(seed) + 2 * e + 2 * f)
                    result['V'] = G.pairs(V)
                    result['V_length'] = len(V)
                    earlier = np.concatenate((earlier, D))
                    prior_counts += counts
                    del pending[least]
                    log(least=least, d=len(D), k=k, s=len(seed), e=e, f=f, steps=steps,
                        V_length=len(V), bound=result['bound'])
        result['missing'] = [{'least': least, 'reason': reason} for least, reason in pending.items()]
    if not pending:
        if occupied[T].any() or len(V) > result['bound']:
            raise AssertionError('vacating word failed the target or length bound')
        result['complete'] = True
        result.pop('missing', None)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('record')
    ap.add_argument('target')
    ap.add_argument('out')
    ap.add_argument('--seconds', type=float, default=900)
    args = ap.parse_args(argv)
    if not math.isfinite(args.seconds) or args.seconds <= 0:
        ap.error('--seconds must be positive and finite')
    result = {'state_digest': None, 'T_digest': None, 'complete': False,
              'missing': [{'least': None, 'reason': 'time bound reached before target orbits were known'}],
              'orbits': [], 'V': [], 'V_length': 0, 'bound': 0}
    # Leave time for the incomplete record even if loading consumes the budget.
    deadline = T0 + args.seconds - min(2, args.seconds / 10)
    previous = signal.signal(signal.SIGALRM, expired)
    signal.setitimer(signal.ITIMER_REAL, max(0.000001, deadline - time.monotonic()))
    phase = 'target loading'
    try:
        target = json.loads(Path(args.target).read_text())
        if hashlib.sha256(json.dumps(sorted(target['T'])).encode()).hexdigest() != target['T_digest']:
            raise ValueError('target digest differs from T')
        result['T_digest'] = target['T_digest']
        global np
        import numpy as np
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        phase = 'state loading'
        import screen
        import buffer as B
        st, M, _ = screen.load('record', args.record)
        result['state_digest'] = st.digest()
        if result['state_digest'] != target['state_digest']:
            raise ValueError('state digest differs from the target state')
        log(loaded=True, state_digest=result['state_digest'])
        phase = 'alphabet loading'
        G = B.Alphabet(SimpleNamespace(st=st), M)
        phase = 'certificate construction'
        build(G, target, result, deadline)
    except Deadline:
        log(timeout=True, phase=phase)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)
    result['seconds'] = round(time.monotonic() - T0, 3)
    Path(args.out).write_text(json.dumps(result) + '\n')
    log(complete=result['complete'], V_length=result['V_length'], bound=result['bound'],
        missing=result.get('missing', []), seconds=result['seconds'])
    return 0


if __name__ == '__main__':
    sys.exit(main())
