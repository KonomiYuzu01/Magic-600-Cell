"""Vacancy transport read from the current exact jumbling state.

SAFE-ALL letters rotate lattice chambers in a cap that contains no off-lattice
piece and whose cut is not straddled. Their words preserve every off pose and
every lattice flag at every prefix. Orbit inventories give a necessary clearing
condition; independently checked Jordan words and a constructive peeling
certificate give two ways to establish transport within those orbits.

All public, saved words are chronological ``(grip, menu_index)`` pairs. Internal
words use letter numbers. A certificate records geometry, not a journal or the
locations of lattice pieces, and can be reused after any SAFE-ALL word.

Example (from the repository root)::

    python research/jumbling/theory/buffer.py --words <word files...> \
        --cls 0 --cell 369 507 --jordan --certificate --out <file>
    python research/jumbling/theory/buffer.py --table <pose table> \
        --cls 0 --cell 369 507 --certificate-in <file> --demo 300 1 --out <file>

Evidence: exact synthetic geometry and simulator replay, not native host or
performance evidence. Only Python's standard library and NumPy are used.
"""
import argparse
import hashlib
import json
import random
import sys
import tempfile
import time
from pathlib import Path

import numpy as np

# Imports must not leave bytecode in directories outside this packet's files.
sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import classes as CL  # noqa: E402
import orient as O  # noqa: E402
import restore  # noqa: E402
import sim  # noqa: E402
from exact import identity  # noqa: E402
from sim.kplus import matrix_json, to_tuple  # noqa: E402

T0 = time.time()
CERTIFICATE_FORMAT = 'buffer-vacancy-1'


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def off_pieces(st):
    return np.flatnonzero(st._kpose()[st.pose_id] < 0)


def off_fingerprint(st):
    """SHA-256 of sorted off piece IDs and their exact ``matrix_json`` poses."""
    matrices = {}
    records = []
    for p in off_pieces(st).tolist():
        pid = int(st.pose_id[p])
        if pid not in matrices:
            matrices[pid] = matrix_json(st._pose_matrix(pid))
        records.append([p, matrices[pid]])
    return hashlib.sha256(json.dumps(records, separators=(',', ':')).encode('ascii')).hexdigest()


def hole_set(st):
    """Current vacant home chambers, with collision and cardinality checks."""
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    K = st._kpose()[st.pose_id]
    lattice = np.flatnonzero(K >= 0)
    ch = reg.piece_of[reg.orbit_of[lattice], kp.compose_vec(K[lattice], reg.transport[lattice])]
    if len(np.unique(ch)) != len(ch):
        raise AssertionError('two lattice pieces occupy the same chamber')
    occupied = np.zeros(len(st.pose_id), bool)
    occupied[ch] = True
    holes = np.flatnonzero(~occupied)
    if len(holes) != len(st.pose_id) - len(lattice):
        raise AssertionError('vacancy count differs from the off-lattice count')
    return holes


def cell_chambers(home, members, P, positive):
    """Chambers with exactly the given positive cuts in the full-space class cell."""
    if not 0 <= P < len(members):
        raise ValueError(f'unknown class {P}')
    positive = set(positive)
    if not positive <= set(members[P]):
        raise ValueError('a positive cut is outside the selected class')
    sig = home[:, members[P]]
    want = np.isin(members[P], sorted(positive))
    return np.flatnonzero((sig == want[None, :]).all(axis=1))


def cycles(p):
    """Cycle labels (least chamber) and lengths indexed by those labels."""
    lab = np.arange(len(p))
    q = p.copy()
    for _ in range(len(p).bit_length()):
        lab = np.minimum(lab, lab[q])
        q = q[q]
    return lab, np.bincount(lab, minlength=len(p))


def primes_upto(n):
    sieve = np.ones(n + 1, bool)
    sieve[:2] = False
    for q in range(2, int(n ** 0.5) + 1):
        if sieve[q]:
            sieve[q * q::q] = False
    return set(np.flatnonzero(sieve).tolist())


class Alphabet:
    """SAFE-ALL cap permutations, inverse letters, and chamber group orbits.

    The state is sampled at construction. ``holes`` is that state's inventory;
    the alphabet and its orbits depend only on the exact off-lattice geometry.
    ``perm``/``images``/``inverse`` take internal letter numbers; ``permutation``
    takes saved pairs. No operation here applies twists to the simulator.
    """

    def __init__(self, R, M):
        self.R, self.M, self.st = R, M, R.st
        self.NP = len(R.st.pose_id)
        self.state_digest = R.st.digest()
        self.fingerprint = off_fingerprint(R.st)
        self.home = CL.data.sig_matrix(np.arange(self.NP))
        self.type = np.asarray(CL.ctx.regions.orbit_of)
        self.lat = R.st.lattice_flags()
        self.holes = hole_set(R.st)
        self.occupied = np.ones(self.NP, bool)
        self.occupied[self.holes] = False
        self.safe, self.blocked = [], {}
        for c in range(600):
            cl = R.st.classify(c, need_inside=True, stop_at_straddle=False)
            if cl.uncertain:
                raise AssertionError(f'uncertain classification at grip {c}')
            off_inside = int((~self.lat[np.asarray(cl.inside, np.int64)]).sum())
            if cl.straddle is None and not off_inside:
                self.safe.append(c)
            else:
                self.blocked[c] = {'straddled': cl.straddle is not None, 'off_inside': off_inside}
        reg, kp = CL.ctx.regions, CL.ctx.kplus
        kid = kp.index_of_matrix(to_tuple(identity(4)))
        self.capx, self.letters, self.by_grip = {}, [], {}
        for c in self.safe:
            xs = np.flatnonzero(self.home[:, c])
            self.capx[c] = xs
            for i in range(len(M.items)):
                k = kp.index_of_matrix(restore.element(M, c, i))
                if k < 0 or k == kid:
                    continue
                ys = reg.piece_of[reg.orbit_of[xs], kp.compose_vec(np.full(len(xs), k), reg.transport[xs])]
                if not np.array_equal(np.sort(ys), xs):
                    raise AssertionError(f'letter ({c}, {i}) does not permute its cap')
                self.by_grip.setdefault(c, []).append(len(self.letters))
                self.letters.append((c, i, k, ys))
        self.index = {(l[0], l[1]): n for n, l in enumerate(self.letters)}
        kindex = {(l[0], l[2]): n for n, l in enumerate(self.letters)}
        try:
            self.inv = np.array([kindex[(l[0], int(kp.inv[l[2]]))] for l in self.letters], np.int64)
        except KeyError as e:
            raise ValueError('the retained menu is not closed under inverses') from e
        for n, (c, _, _, ys) in enumerate(self.letters):
            xs = self.capx[c]
            back = self.letters[int(self.inv[n])][3][np.searchsorted(xs, ys)]
            if not np.array_equal(back, xs):
                raise AssertionError('inverse letter failed the cap permutation check')
        rng = random.Random(1)
        probe = [rng.randrange(len(self.letters)) for _ in range(25)] if self.letters else []
        if not np.array_equal(self.perm(probe), self.slow_perm(probe)):
            raise AssertionError('fast composition differs from chronological slow composition')
        log(safe_all_grips=len(self.safe), letters=len(self.letters), off_lattice=len(self.holes))

    def pairs(self, word):
        return [[self.letters[n][0], self.letters[n][1]] for n in word]

    def numbers(self, word):
        out = []
        for pair in word:
            if not isinstance(pair, (list, tuple)) or len(pair) != 2 or any(type(x) is not int for x in pair):
                raise ValueError('a word must contain integer (grip, menu_index) pairs')
            if tuple(pair) not in self.index:
                raise ValueError(f'{pair} is not a SAFE-ALL letter at this geometry')
            out.append(self.index[tuple(pair)])
        return out

    def inverse(self, word):
        return [int(self.inv[n]) for n in reversed(word)]

    def perm(self, word):
        """Source-to-destination permutation; the first letter acts first.

        Right composition in reverse order only writes each cap. It avoids
        constructing and indexing a full identity permutation for every letter.
        """
        q = np.arange(self.NP)
        for n in reversed(word):
            c, _, _, ys = self.letters[n]
            q[self.capx[c]] = q[ys]
        return q

    def slow_perm(self, word):
        p = np.arange(self.NP)
        for n in word:
            c, _, _, ys = self.letters[n]
            lp = np.arange(self.NP)
            lp[self.capx[c]] = ys
            p = lp[p]
        return p

    def permutation(self, word):
        return self.perm(self.numbers(word))

    def images(self, word, points):
        points = np.array(points, np.int64)
        for n in word:
            c, _, _, ys = self.letters[n]
            m = self.home[points, c]
            points[m] = ys[np.searchsorted(self.capx[c], points[m])]
        return points

    def image(self, word, point):
        return int(self.images(word, [point])[0])

    def orbits(self):
        """Connected components of all letter edges, numbered by least chamber."""
        if hasattr(self, 'orbit'):
            return self.orbit, self.orbit_size
        lab = np.arange(self.NP)
        while True:
            old = lab.copy()
            for c, _, _, ys in self.letters:
                xs = self.capx[c]
                m = np.minimum(lab[xs], lab[ys])
                np.minimum.at(lab, xs, m)
                np.minimum.at(lab, ys, m)
            while True:
                nl = lab[lab]
                if np.array_equal(nl, lab):
                    break
                lab = nl
            if np.array_equal(lab, old):
                break
        self.orbit_least, self.orbit, self.orbit_size = np.unique(lab, return_inverse=True, return_counts=True)
        self.orbit = self.orbit.ravel()
        log(orbits=len(self.orbit_size), singletons=int((self.orbit_size == 1).sum()))
        return self.orbit, self.orbit_size


def _jordan_from_cycles(G, lab, lengths, orbit, prime=None):
    n = int(G.orbit_size[orbit])
    if n < 8:
        raise ValueError('orbits of size below 8 cannot have this Jordan certificate')
    reps = np.flatnonzero(lengths)
    primes = primes_upto(n)
    for r in reps[G.orbit[reps] == orbit].tolist():
        p = int(lengths[r])
        if p not in primes or (prime is not None and p != prime) or not (n / 2 < p <= n - 3):
            continue
        if int((lengths[reps] % p == 0).sum()) != 1:
            continue
        if not (G.orbit[lab == r] == orbit).all():
            continue
        return {'orbit': int(orbit), 'orbit_size': n, 'prime': p, 'cycle_least_point': r, 'verified': True}
    raise ValueError(f'word has no Jordan certificate for orbit {orbit}')


def check_jordan(alphabet, word, orbit, prime=None):
    """Recompute a saved pair word on *all* chambers and check every condition.

    Since p > |D|/2, a cycle in D divisible by p necessarily has length p.
    Other orbits' cycles are included in the uniqueness test.
    """
    alphabet.orbits()
    if not 0 <= orbit < len(alphabet.orbit_size):
        raise ValueError('unknown orbit')
    lab, lengths = cycles(alphabet.permutation(word))
    return _jordan_from_cycles(alphabet, lab, lengths, orbit, prime)


def jordan(alphabet, target, seed=1):
    """Seeded random search in increasing word lengths; missing orbits are explicit."""
    G = alphabet
    G.orbits()
    want = {int(o) for o in G.orbit[target] if G.orbit_size[o] >= 8}
    small = sorted({int(o) for o in G.orbit[target] if G.orbit_size[o] < 8})
    found = {}
    for stage, (length, tries) in enumerate(((120, 200), (400, 400), (800, 400), (1500, 400))):
        rng = random.Random(seed + stage)
        for trial in range(tries):
            left = want - found.keys()
            if not left:
                break
            word = [rng.randrange(len(G.letters)) for _ in range(length)]
            lab, lengths = cycles(G.perm(word))
            for o in sorted(left):
                try:
                    cert = _jordan_from_cycles(G, lab, lengths, o)
                except ValueError:
                    continue
                found[o] = dict(cert, word=G.pairs(word), length=length)
            if trial % 20 == 19:
                log(jordan_length=length, tries=trial + 1, certified=len(found), of=len(want))
        if len(found) == len(want):
            break
    return {'certificates': {str(o): c for o, c in sorted(found.items())},
            'uncertifiable_below_8': small, 'missing': sorted(want - found.keys())}


class Transport:
    """Letter routing graph and short peeling candidates at fixed geometry."""

    def __init__(self, G, target):
        self.G, self.NP, self.T = G, G.NP, np.asarray(target, np.int64)
        G.orbits()
        self.orbit, self.size = G.orbit, G.orbit_size
        self.inT = np.zeros(G.NP, bool)
        self.inT[self.T] = True
        self.capsof = np.array([l[0] for l in G.letters], np.int64)
        self.safe = np.array(G.safe, np.int64)
        self.avoidT = np.array([not self.inT[G.capx[l[0]]].any() for l in G.letters], bool)
        src, dst, lid = [], [], []
        for n, (c, _, _, ys) in enumerate(G.letters):
            src.append(G.capx[c])
            dst.append(ys)
            lid.append(np.full(len(ys), n, np.int32))
        self.src = np.concatenate(src) if src else np.array([], np.int64)
        self.dst = np.concatenate(dst) if dst else np.array([], np.int64)
        self.lid = np.concatenate(lid) if lid else np.array([], np.int32)
        keep = self.src != self.dst
        self.src, self.dst, self.lid = self.src[keep], self.dst[keep], self.lid[keep]
        order = np.argsort(self.orbit[self.src], kind='stable')
        self.src, self.dst, self.lid = self.src[order], self.dst[order], self.lid[order]
        self.edges = np.searchsorted(self.orbit[self.src], np.arange(len(self.size) + 1))
        log(routing_edges=len(self.src), letters_avoiding_target=int(self.avoidT.sum()))

    def support(self, p):
        return np.flatnonzero(p != np.arange(self.NP))

    def letters_near(self, x, protected):
        """Caps at x, or at protected points sharing a SAFE cap with x."""
        G, safe = self.G, self.safe
        cx = safe[G.home[x, safe]]
        share = protected[G.home[np.ix_(protected, cx)].any(axis=1)] if len(protected) else protected
        cs = set(cx.tolist())
        if len(share):
            cs.update(safe[G.home[np.ix_(share, safe)].any(axis=0)].tolist())
        return [n for c in sorted(cs) for n in G.by_grip[c]]

    def peel(self, maxg=2):
        """Build the order backwards from letters and short commutators.

        A candidate's target support must have just one point not already chosen
        late, and the preimage of that point must lie outside the entire target.
        The returned chronological order therefore fixes earlier targets.
        """
        G, T, L = self.G, self.T, self.G.letters
        touchT = np.flatnonzero(~self.avoidT).tolist()
        hcaps = np.array(sorted({L[n][0] for n in np.flatnonzero(self.avoidT)}), np.int64)
        words, masks = [], []

        def add(word, indices):
            mask = 0
            for j in indices:
                mask |= 1 << int(j)
            if mask:
                words.append(word)
                masks.append(mask)

        for n in touchT:
            c, _, _, ys = L[n]
            inside = np.flatnonzero(G.home[T, c])
            img = ys[np.searchsorted(G.capx[c], T[inside])]
            add([n], inside[img != T[inside]])
        frontier = [[]]
        for _ in range(maxg):
            nxt = []
            for g0 in frontier:
                for n in touchT:
                    if g0 and n == G.inv[g0[-1]]:
                        continue
                    g = g0 + [n]
                    nxt.append(g)
                    P = G.images(G.inverse(g), T)
                    inside = G.home[np.ix_(P, hcaps)]
                    for ci in np.flatnonzero(inside.any(axis=0)).tolist():
                        c = int(hcaps[ci])
                        js = np.flatnonzero(inside[:, ci])
                        xs = P[js]
                        for hn in G.by_grip[c]:
                            img = L[hn][3][np.searchsorted(G.capx[c], xs)]
                            moved = js[img != xs]
                            if len(moved):
                                add([int(G.inv[hn])] + G.inverse(g) + [hn] + g, moved)
            frontier = nxt
        log(peeling_candidates=len(words))
        order = np.argsort([len(w) for w in words], kind='stable').tolist()
        late, chosen, bad = 0, [], set()
        while len(chosen) < len(T):
            for e in order:
                new = masks[e] & ~late
                if not new or new & (new - 1) or (e, new) in bad:
                    continue
                t = int(T[new.bit_length() - 1])
                p = G.perm(words[e])
                y = int(np.flatnonzero(p == t)[0])
                if self.inT[y]:
                    bad.add((e, new))
                    continue
                late |= new
                chosen.append((t, words[e], y))
                break
            else:
                raise ValueError(f'peeling stopped at {len(chosen)} of {len(T)} target chambers')
        chosen.reverse()
        protected = []
        for t, w, y in chosen:
            p = G.perm(w)
            if p[t] == t or p[y] != t or not np.array_equal(p[protected], protected):
                raise AssertionError('peeling word does not satisfy its pointwise constraints')
            protected.append(t)
        return chosen


def weighted_tree(tr, root, allowed, macros, extra=()):
    """Shortest chronological routes over letters, macros, and conjugate edges.

    Letters have weight 1, macros and conjugate edges their word length. Every
    macro and extra edge is offered in both directions. A predecessor edge is
    a nonnegative letter number, or -(2k+1)/-(2k+2) for forward/backward word k;
    words are ordered as macros followed by extra edges. Extra edges describe
    only their two endpoints, even if their words move other chambers too.
    """
    G = tr.G
    o = int(tr.orbit[root])
    a, b = tr.edges[o:o + 2]
    src, dst, lid = tr.src[a:b], tr.dst[a:b], tr.lid[a:b]
    keep = allowed[lid]
    src, dst, lid = src[keep], dst[keep], lid[keep]
    sources, targets = [src], [dst]
    ids, weights = [lid.astype(np.int64)], [np.ones(len(src), np.int64)]
    for k, (w, p, sup) in enumerate(macros):
        sup = sup[tr.orbit[sup] == o]
        if not len(sup):
            continue
        sources.extend([sup, p[sup]])
        targets.extend([p[sup], sup])
        ids.extend([np.full(len(sup), -(2 * k + 1)), np.full(len(sup), -(2 * k + 2))])
        weights.extend([np.full(len(sup), len(w)), np.full(len(sup), len(w))])
    for k, edge in enumerate(extra, len(macros)):
        z, to, w = edge['from'], edge['to'], edge['word']
        sources.append(np.array([z, to]))
        targets.append(np.array([to, z]))
        ids.append(np.array([-(2 * k + 1), -(2 * k + 2)]))
        weights.append(np.full(2, len(w)))
    src, dst = np.concatenate(sources), np.concatenate(targets)
    eid, wt = np.concatenate(ids), np.concatenate(weights)
    order = np.argsort(src, kind='stable')
    src, dst, eid, wt = src[order], dst[order], eid[order], wt[order]
    start = np.searchsorted(src, np.arange(tr.NP + 1))
    dist = np.full(tr.NP, -1, np.int64)
    pn, pe = np.full(tr.NP, -1, np.int64), np.zeros(tr.NP, np.int64)
    dist[root] = 0
    buckets, final = {0: [root]}, np.zeros(tr.NP, bool)
    while buckets:
        d = min(buckets)
        xs = np.unique(buckets.pop(d))
        xs = xs[(dist[xs] == d) & ~final[xs]]
        if not len(xs):
            continue
        final[xs] = True
        lo, hi = start[xs], start[xs + 1]
        idx = np.concatenate([np.arange(x, y) for x, y in zip(lo.tolist(), hi.tolist())])
        if not len(idx):
            continue
        nd, tgt = d + wt[idx], dst[idx]
        better = (dist[tgt] < 0) | (nd < dist[tgt])
        idx, nd, tgt = idx[better], nd[better], tgt[better]
        order = np.lexsort((nd, tgt))
        tgt, nd, idx = tgt[order], nd[order], idx[order]
        u, first = np.unique(tgt, return_index=True)
        dist[u], pn[u], pe[u] = nd[first], src[idx[first]], eid[idx[first]]
        for val in np.unique(nd[first]).tolist():
            buckets.setdefault(int(val), []).extend(u[nd[first] == val].tolist())
    return dist, pn, pe


def tree_path(tr, tree, macros, x, extra=()):
    """Reconstruct a root -> x word, expanding every weighted edge."""
    dist, pn, pe = tree
    x = int(x)
    if dist[x] < 0:
        raise ValueError('point is not reached by the route tree')
    edge_words = [m[0] for m in macros] + [e['word'] for e in extra]
    parts = []
    while dist[x] > 0:
        e = int(pe[x])
        if e >= 0:
            parts.append([e])
        else:
            k, back = divmod(-e - 1, 2)
            w = edge_words[k]
            parts.append(tr.G.inverse(w) if back else list(w))
        parent = int(pn[x])
        if parent < 0 or dist[parent] >= dist[x]:
            raise AssertionError('invalid weighted predecessor')
        x = parent
    return [n for part in reversed(parts) for n in part]


def _best_conjugate(tr, z, protected, dist, conjugator_length):
    """Shortest found edge z -> a reached point, using ``letters_near``."""
    G, best = tr.G, None

    def consider(g, points):
        nonlocal best
        gz, gp = int(points[0]), points[1:]
        caps = tr.safe[G.home[gz, tr.safe]]
        if len(gp):
            caps = caps[~G.home[np.ix_(gp, caps)].any(axis=0)]
        for c in caps.tolist():
            pos = np.searchsorted(G.capx[c], gz)
            letters = G.by_grip[c]
            images = np.array([G.letters[n][3][pos] for n in letters])
            ends = G.images(G.inverse(g), images)
            ok = (images != gz) & (ends != z) & (dist[ends] >= 0)
            if not ok.any():
                continue
            candidates = np.flatnonzero(ok)
            k = int(candidates[np.argmin(dist[ends[candidates]])])
            cost = 2 * len(g) + 1 + int(dist[ends[k]])
            if best is None or cost < best[0]:
                best = (cost, g + [letters[k]] + G.inverse(g), int(ends[k]))

    points = np.concatenate(([z], protected))
    for g1 in tr.letters_near(z, protected):
        p1 = G.images([g1], points)
        if conjugator_length == 1:
            consider([g1], p1)
        else:
            for g2 in tr.letters_near(int(p1[0]), p1[1:]):
                if g2 == G.inv[g1]:
                    continue  # The identity conjugator gives an existing free letter edge.
                consider([g1, g2], G.images([g2], p1))
        if best is not None and best[0] == 2 * conjugator_length + 1:
            break
    return best


def close_routes(tr, t, y, protected, allowed, macros):
    """Close route gaps with verified one- and two-letter conjugates.

    After each batch of edges, recompute the full shortest paths, including
    ordinary letters out of newly reached points. Restart the one-letter pass
    after every gain. Stop only when neither conjugator size gains an edge.
    """
    G, extra = tr.G, []
    protected = np.asarray(protected, np.int64)
    points = np.flatnonzero(tr.orbit == tr.orbit[t])
    points = points[~np.isin(points, protected)]
    tree = weighted_tree(tr, y, allowed, macros)
    while True:
        unreached = points[(tree[0][points] < 0) & (points != t)]
        if not len(unreached):
            break
        added = []
        for length in (1, 2):
            for z in unreached.tolist():
                best = _best_conjugate(tr, z, protected, tree[0], length)
                if best is None:
                    continue
                _, word, to = best
                p = G.perm(word)
                if p[z] != to or not np.array_equal(p[protected], protected):
                    raise AssertionError('conjugate edge failed its full-permutation check')
                added.append({'from': z, 'to': to, 'word': word, 'conjugator_length': length})
            if added:
                break
        if not added:
            break
        extra.extend(added)
        tree = weighted_tree(tr, y, allowed, macros, extra)
    return tree, extra


def _route_metrics(G, t, protected, word, tree):
    o = int(G.orbit[t])
    pts = np.flatnonzero(G.orbit == o)
    inP = np.isin(pts, protected)
    cand = pts[~inP]
    unreached = cand[(tree[0][cand] < 0) & (cand != t)].tolist()
    ecc = int(tree[0][cand].max())
    return {'E_length': len(word), 'ecc': ecc, 'orbit_minus_P': len(cand),
            'reached': int((tree[0][cand] >= 0).sum()), 'unreached': unreached, 'u_j': len(unreached),
            'holes_needed': int(inP.sum()) + 1 + len(unreached), 'bound': len(word) + ecc}


def _totals(G, target, rows):
    targets = np.bincount(G.orbit[target], minlength=len(G.orbit_size))
    thresholds = {}
    for row in rows:
        o = str(row['orbit'])
        thresholds[o] = max(thresholds.get(o, 0), row['holes_needed'])
    return {'thresholds': thresholds, 'family_bound': sum(r['bound'] for r in rows),
            'all_reached': all(not r['unreached'] for r in rows),
            'thresholds_equal_targets': all(v == targets[int(o)] for o, v in thresholds.items())}


def certificate(G, P, positive):
    """Build a serializable peeling/routing certificate from current geometry."""
    members, _ = CL.classes()
    T = cell_chambers(G.home, members, P, positive)
    G.orbits()
    fixed = T[G.orbit_size[G.orbit[T]] == 1]
    movable = T[G.orbit_size[G.orbit[T]] > 1]
    tr = Transport(G, movable)
    chosen = tr.peel()
    macros = []
    for _, w, _ in chosen:
        p = G.perm(w)
        macros.append((w, p, tr.support(p)))
    free, protected, rows = np.ones(len(G.letters), bool), [], []
    for j, (t, w, y) in enumerate(chosen):
        tree, extra = close_routes(tr, t, y, protected, free, macros[j:])
        metrics = _route_metrics(G, t, protected, w, tree)
        rows.append(dict(t=t, orbit=int(G.orbit[t]), y=y, word=G.pairs(w), **metrics,
                         conjugate_edges=[dict(e, word=G.pairs(e['word'])) for e in extra]))
        if extra or metrics['unreached']:
            log(certificate_row=j, target=t, conjugate_edges=len(extra), **metrics)
        protected.append(t)
        free &= ~G.home[t, tr.capsof]
    doc = {'format': CERTIFICATE_FORMAT, 'off_geometry_fingerprint': G.fingerprint,
           'source_digest': G.state_digest, 'menu': G.M.name,
           'target': {'class': int(P), 'cell': sorted(set(positive)), 'chambers': T.tolist()},
           'fixed': fixed.tolist(), 'peeled': len(chosen), 'of': len(movable), 'rows': rows,
           **_totals(G, T, rows)}
    log(peeled=doc['peeled'], family_bound=doc['family_bound'], all_reached=doc['all_reached'])
    return doc


def _certificate_geometry(G, doc, P=None, positive=None):
    if doc.get('format') != CERTIFICATE_FORMAT:
        raise ValueError('unknown buffer certificate format')
    if doc.get('off_geometry_fingerprint') != G.fingerprint:
        raise ValueError('certificate off-geometry fingerprint differs from the current state')
    if off_fingerprint(G.st) != G.fingerprint:
        raise ValueError('alphabet off-geometry fingerprint differs from the current state')
    if doc.get('menu') != G.M.name:
        raise ValueError('certificate menu differs from the current menu')
    target = doc['target']
    if P is not None and target['class'] != P:
        raise ValueError('certificate target class differs from the requested target')
    if positive is not None and target['cell'] != sorted(set(positive)):
        raise ValueError('certificate target cell differs from the requested target')
    members, _ = CL.classes()
    T = cell_chambers(G.home, members, target['class'], target['cell'])
    if target['chambers'] != T.tolist():
        raise ValueError('certificate target chambers differ from the exact target cell')
    G.orbits()
    fixed = T[G.orbit_size[G.orbit[T]] == 1].tolist()
    if doc['fixed'] != fixed:
        raise ValueError('certificate fixed chambers differ from the current group orbits')
    return T


def _decoded_routes(G, doc):
    movable = [r['t'] for r in doc['rows']]
    tr = Transport(G, movable)
    macros, extras = [], []
    for row in doc['rows']:
        w = G.numbers(row['word'])
        p = G.perm(w)
        macros.append((w, p, tr.support(p)))
        extras.append([dict(e, word=G.numbers(e['word'])) for e in row['conjugate_edges']])
    return tr, macros, extras


def validate_certificate(G, doc, P=None, positive=None):
    """Check geometry, every stored word/edge, routes, and all claimed bounds.

    Saved distance summaries are not trusted: routes are recomputed. This also
    makes a reused certificate an independently checked finite witness.
    """
    T = _certificate_geometry(G, doc, P, positive)
    movable = T[G.orbit_size[G.orbit[T]] > 1].tolist()
    peeled = [r['t'] for r in doc['rows']]
    if sorted(peeled) != movable or doc['peeled'] != len(movable) or doc['of'] != len(movable):
        raise ValueError('certificate does not peel every movable target exactly once')
    tr, macros, extras = _decoded_routes(G, doc)
    protected, free = [], np.ones(len(G.letters), bool)
    for j, row in enumerate(doc['rows']):
        t, y, o = row['t'], row['y'], row['orbit']
        w, p, _ = macros[j]
        if not 0 <= y < G.NP or y in T or o != G.orbit[t] or p[t] == t or p[y] != t:
            raise ValueError(f'invalid peeling row {j}')
        if not np.array_equal(p[protected], protected):
            raise ValueError(f'peeling row {j} moves an earlier target')
        for edge in extras[j]:
            z, to, ew = edge['from'], edge['to'], edge['word']
            length = edge['conjugator_length']
            if length not in (1, 2) or len(ew) != 2 * length + 1 or ew[-length:] != G.inverse(ew[:length]):
                raise ValueError(f'invalid conjugate word at row {j}')
            if not (0 <= z < G.NP and 0 <= to < G.NP) or z == to or z in protected or to in protected:
                raise ValueError(f'invalid conjugate endpoints at row {j}')
            if G.orbit[z] != o or G.orbit[to] != o:
                raise ValueError(f'conjugate edge leaves its orbit at row {j}')
            ep = G.perm(ew)
            if ep[z] != to or not np.array_equal(ep[protected], protected):
                raise ValueError(f'conjugate full-permutation check failed at row {j}')
            gp = G.images(ew[:length], protected)
            c = G.letters[ew[length]][0]
            if not G.home[G.image(ew[:length], z), c] or G.home[gp, c].any():
                raise ValueError(f'conjugate middle cap violates its constraints at row {j}')
        tree = weighted_tree(tr, y, free, macros[j:], extras[j])
        metrics = _route_metrics(G, t, protected, w, tree)
        if any(row.get(k) != v for k, v in metrics.items()):
            raise ValueError(f'certificate route summary differs from recomputation at row {j}')
        protected.append(t)
        free &= ~G.home[t, tr.capsof]
    if any(doc.get(k) != v for k, v in _totals(G, T, doc['rows']).items()):
        raise ValueError('certificate totals differ from recomputation')
    return doc


def load_certificate(path, G, P=None, positive=None):
    """Read either a standalone certificate or this command's JSON output."""
    doc = json.loads(Path(path).read_text())
    if 'certificate' in doc:
        doc = doc['certificate']
    return validate_certificate(G, doc, P, positive)


def _holes_mask(G, holes=None):
    holes = G.holes if holes is None else np.asarray(holes)
    if holes.dtype == bool:
        if holes.shape != (G.NP,):
            raise ValueError('a hole mask must have one entry per chamber')
        return holes.copy()
    if holes.ndim == 1 and not holes.size:
        return np.zeros(G.NP, bool)
    if holes.ndim != 1 or not np.issubdtype(holes.dtype, np.integer):
        raise ValueError('holes must be a one-dimensional array of chamber IDs')
    if ((holes < 0) | (holes >= G.NP)).any() or len(np.unique(holes)) != len(holes):
        raise ValueError('hole chamber IDs are out of range or duplicated')
    mask = np.zeros(G.NP, bool)
    mask[holes] = True
    return mask


def condition(G, target, cert=None, holes=None):
    """Fixed vacancies and orbit inventories against targets and thresholds."""
    G.orbits()
    holes = _holes_mask(G, holes)
    target = np.asarray(target, np.int64)
    if cert is not None and not np.array_equal(_certificate_geometry(G, cert), target):
        raise ValueError('condition target differs from the certificate target')
    targets = np.bincount(G.orbit[target], minlength=len(G.orbit_size))
    counts = np.bincount(G.orbit[np.flatnonzero(holes)], minlength=len(G.orbit_size))
    fixed = target[G.orbit_size[G.orbit[target]] == 1]
    fixed_occupied = fixed[~holes[fixed]].tolist()
    rows, necessary, sufficient = [], [], []
    for o in np.flatnonzero(targets).tolist():
        need, have = int(targets[o]), int(counts[o])
        threshold = cert['thresholds'].get(str(o), 1) if cert is not None else None
        rows.append({'orbit': o, 'least_chamber': int(G.orbit_least[o]), 'size': int(G.orbit_size[o]),
                     'target': need, 'holes': have, 'threshold': threshold})
        if have < need:
            necessary.append({'orbit': o, 'have': have, 'need': need, 'deficit': need - have})
        if threshold is not None and have < threshold:
            sufficient.append({'orbit': o, 'have': have, 'need': threshold, 'deficit': threshold - have})
    return {'fixed_vacant': not fixed_occupied, 'fixed_occupied': fixed_occupied,
            'necessary': not fixed_occupied and not necessary, 'necessary_deficits': necessary,
            'sufficient': (not fixed_occupied and not sufficient) if cert is not None else None,
            'sufficient_deficits': sufficient, 'by_orbit': rows}


def prepare(G, cert, holes=None):
    """Construct a clearing word using only holes and a geometry certificate.

    Choose the nearest reached hole outside earlier targets and the current
    target. Reverse its root route, then apply E_j, tracking the full predicted
    hole permutation. No journal, history, or simulator mutation is used.
    """
    T = _certificate_geometry(G, cert)
    holes = _holes_mask(G, holes)
    cond = condition(G, T, cert, holes)
    if not cond['sufficient']:
        raise ValueError(f'certificate condition fails: {json.dumps(cond)}')
    tr, macros, extras = _decoded_routes(G, cert)
    protected, free, word, steps = [], np.ones(len(G.letters), bool), [], []
    for j, row in enumerate(cert['rows']):
        t, y = row['t'], row['y']
        if not holes[t]:
            tree = weighted_tree(tr, y, free, macros[j:], extras[j])
            candidates = holes & (tree[0] >= 0)
            candidates[protected], candidates[t] = False, False
            points = np.flatnonzero(candidates)
            if not len(points):
                raise ValueError(f'no reached hole is available for target {t}')
            z = int(points[np.argmin(tree[0][points])])
            path = tree_path(tr, tree, macros[j:], z, extras[j])
            route = G.inverse(path)
            if len(path) != tree[0][z] or G.image(route, z) != y:
                raise AssertionError('reconstructed route does not match its shortest path')
            step = route + macros[j][0]
            p = G.perm(step)
            predicted = np.zeros(G.NP, bool)
            predicted[p[holes]] = True
            holes = predicted
            word.extend(step)
            steps.append({'row': j, 'target': t, 'hole': z, 'route_length': len(route), 'length': len(step)})
        if not holes[t] or not holes[protected].all():
            raise AssertionError('greedy step did not preserve the cleared target prefix')
        protected.append(t)
        free &= ~G.home[t, tr.capsof]
    if not holes[T].all() or len(word) > cert['family_bound']:
        raise AssertionError('preparation failed its target or family bound')
    return {'word': G.pairs(word), 'predicted_holes': np.flatnonzero(holes).tolist(),
            'length': len(word), 'filled': len(steps), 'steps': steps, 'condition': cond}


def export_pose_table(st, path):
    """Export only the current poses in ``orient.set_pose_table`` format."""
    doc = {'poses': [matrix_json(st._pose_matrix(i)) for i in range(len(st._poses))],
           'pose_id': st.pose_id.tolist(), 'digest': st.digest(), 'journal': []}
    Path(path).write_text(json.dumps(doc) + '\n')
    return doc


def _fresh_restorer(table):
    """A constructor that reads the exported table, without replaying a history."""
    M = restore.menu('I_a')
    st = sim.State(CL.ctx, menu=M)
    O.set_pose_table(st, json.loads(Path(table).read_text()))
    if st.journal:
        raise AssertionError('pose table constructor retained a journal')
    return restore.Restorer(st, M), M


def _off_keys(st, off):
    return [st._poses[int(q)].key for q in st.pose_id[off]]


def _check_off_prefix(st, off, keys, prefix):
    if not np.array_equal(off_pieces(st), off):
        raise AssertionError(f'lattice flags changed at preparation prefix {prefix}')
    if _off_keys(st, off) != keys:
        raise AssertionError(f'an off pose changed at preparation prefix {prefix}')


def _depth_one(R, members, cls, cache):
    D = R.domains()
    ds = np.unique(np.asarray(D.dom)[off_pieces(R.st)])
    for d in ds.tolist():
        rep = to_tuple(D.reps[d])
        if rep not in cache:
            cache[rep] = len(CL.hub(rep, members, cls)) == 1
        if not cache[rep]:
            return False
    return True


def demo(G, cert, length, seed, suffix=None, certificate_in=None):
    """Generate, reconstruct from a current-only table, prepare, and replay.

    The generator has reachability evidence; the fresh constructor sees only
    its exported pose table and the saved geometry certificate. Every
    preparation prefix is replayed through ``Restorer.apply`` and checked.
    If supplied, every suffix prefix is checked for depth one.
    """
    if length < 0:
        raise ValueError('demo length must be nonnegative')
    if length and not G.letters:
        raise ValueError('the SAFE-ALL alphabet is empty')
    target = cert['target']
    with tempfile.TemporaryDirectory(prefix='buffer-demo-') as directory:
        tmp = Path(directory)
        initial = tmp / 'initial.json'
        export_pose_table(G.st, initial)
        generator, _ = _fresh_restorer(initial)
        off0 = off_pieces(generator.st)
        keys0 = _off_keys(generator.st, off0)
        rng = random.Random(seed)
        gen = G.pairs([rng.randrange(len(G.letters)) for _ in range(length)])
        for n, (c, i) in enumerate(gen, 1):
            generator.apply(c, i)
            _check_off_prefix(generator.st, off0, keys0, n)
        table = tmp / 'source.json'
        source_table = export_pose_table(generator.st, table)
        source_digest = source_table['digest']
        log(demo_stage='generator', generator_length=len(gen), source_digest=source_digest)
        # No generator restorer, journal, or word enters the constructor.
        del generator
        R, M = _fresh_restorer(table)
        A = Alphabet(R, M)
        A.orbits()
        if certificate_in is None:
            certificate_in = tmp / 'certificate.json'
            Path(certificate_in).write_text(json.dumps(cert) + '\n')
        reused = load_certificate(certificate_in, A, target['class'], target['cell'])
        T = np.asarray(reused['target']['chambers'], np.int64)
        prepared = prepare(A, reused)
        off = off_pieces(R.st)
        keys = _off_keys(R.st, off)
        matrices = {p: matrix_json(R.st._pose_matrix(int(R.st.pose_id[p]))) for p in off.tolist()}
        for n, (c, i) in enumerate(prepared['word'], 1):
            R.apply(c, i)
            _check_off_prefix(R.st, off, keys, n)
        actual = hole_set(R.st)
        if actual.tolist() != prepared['predicted_holes'] or not np.isin(T, actual).all():
            raise AssertionError('predicted and actual holes differ, or the target is not vacant')
        after_digest = R.st.digest()
        log(demo_stage='preparation', length=prepared['length'], filled=prepared['filled'], digest=after_digest)
        suffix_word, depth_checks = [], 0
        if suffix is not None:
            suffix_doc = json.loads(Path(suffix).read_text())
            suffix_word = suffix_doc['word'] if isinstance(suffix_doc, dict) else suffix_doc
            members, cls = CL.classes()
            cache = {}
            if not _depth_one(R, members, cls, cache):
                raise AssertionError('suffix source is not depth one')
            for n, (c, i) in enumerate(suffix_word, 1):
                R.apply(int(c), int(i))
                if not _depth_one(R, members, cls, cache):
                    raise AssertionError(f'suffix is not depth one at prefix {n}')
                depth_checks += 1
                if n % 100 == 0:
                    log(demo_stage='suffix', prefix=n, of=len(suffix_word))
        end_off = off_pieces(R.st)
        restored = sorted(set(off.tolist()) - set(end_off.tolist()))
        taken_off = sorted(set(end_off.tolist()) - set(off.tolist()))
        if taken_off:
            raise AssertionError('suffix took a source lattice piece off the lattice')
        same = all(matrix_json(R.st._pose_matrix(int(R.st.pose_id[p]))) == matrices[p] for p in end_off.tolist())
        return {'initial_digest': G.state_digest, 'generator_length': length, 'generator_seed': seed,
                'generator_word': gen, 'source_digest': source_digest, 'source_pose_table': source_table,
                'certificate_reused': True, 'fingerprint_matches': off_fingerprint(A.st) == cert['off_geometry_fingerprint']
                if suffix is None else A.fingerprint == cert['off_geometry_fingerprint'],
                'constructor_journal_empty': True, 'condition': prepared['condition'],
                'target_vacant_at_source': int((~A.occupied[T]).sum()),
                'preparation_word': prepared['word'], 'preparation_length': prepared['length'],
                'filled': prepared['filled'], 'family_bound': cert['family_bound'],
                'preparation_prefix_checks': prepared['length'], 'off_prefixes_unchanged': True,
                'predicted_holes': prepared['predicted_holes'], 'predicted_equals_actual': True,
                'target_vacant_after_preparation': True, 'after_preparation_digest': after_digest,
                'suffix_word': suffix_word, 'suffix_length': len(suffix_word), 'depth_one_prefix_checks': depth_checks,
                'end_digest': R.st.digest(), 'restored_pieces': restored, 'restored': len(restored),
                'pieces_taken_off': taken_off, 'off_after': len(end_off), 'remaining_off_poses_equal_source': same}


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--table', help='current-only pose table (default: E2 end)')
    ap.add_argument('--words', nargs='+', default=[], help='word files replayed first, in the order given')
    ap.add_argument('--cls', type=int, default=0)
    ap.add_argument('--cell', type=int, nargs='*', default=[369, 507], help='positive cuts of the target class cell')
    ap.add_argument('--jordan', action='store_true')
    ap.add_argument('--seed', type=int, default=1, help='seed for staged Jordan search')
    ap.add_argument('--certificate', action='store_true')
    ap.add_argument('--certificate-in', help='reuse a saved certificate after geometry and witness checks')
    ap.add_argument('--demo', type=int, nargs=2, metavar=('LENGTH', 'SEED'))
    ap.add_argument('--suffix', help='word file replayed after demo preparation, checking every depth-one prefix')
    ap.add_argument('--out', help='write full JSON output (otherwise write it to stdout)')
    a = ap.parse_args(argv)
    if a.suffix and a.demo is None:
        ap.error('--suffix requires --demo')
    R, M = O.start(a.table, a.words)
    G = Alphabet(R, M)
    members, _ = CL.classes()
    T = cell_chambers(G.home, members, a.cls, a.cell)
    G.orbits()
    cert = None
    if a.certificate_in:
        cert = load_certificate(a.certificate_in, G, a.cls, a.cell)
    elif a.certificate or a.demo is not None:
        cert = certificate(G, a.cls, a.cell)
    fixed = T[G.orbit_size[G.orbit[T]] == 1].tolist()
    cond = condition(G, T, cert)
    out = {'state_digest': G.state_digest, 'off_lattice': len(G.holes), 'class': a.cls, 'cell': sorted(set(a.cell)),
           'off_geometry_fingerprint': G.fingerprint, 'safe_all_grips': len(G.safe), 'alphabet_size': len(G.letters),
           'target_chambers': len(T), 'target_vacant': int((~G.occupied[T]).sum()),
           'orbits': len(G.orbit_size), 'orbits_meeting_target': len(cond['by_orbit']),
           'fixed': fixed, 'orbit_table': cond['by_orbit'], 'condition': cond}
    if a.jordan:
        out['jordan'] = jordan(G, T, a.seed)
    if cert is not None:
        out['certificate'] = cert
    if a.demo is not None:
        out['demo'] = demo(G, cert, *a.demo, suffix=a.suffix, certificate_in=a.certificate_in)
    out['seconds'] = round(time.time() - T0)
    text = json.dumps(out, indent=1) + '\n'
    if a.out:
        Path(a.out).write_text(text)
        print(json.dumps({k: v for k, v in out.items() if k not in ('certificate', 'jordan', 'demo', 'orbit_table')}))
    else:
        print(text, end='')
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (ValueError, AssertionError, KeyError) as error:
        print(f'buffer: {error}', file=sys.stderr)
        sys.exit(1)
