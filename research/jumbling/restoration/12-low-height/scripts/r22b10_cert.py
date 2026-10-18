# R22B-10 / E22-1b: build an exact, bounded vacating-word certificate (research only).
# Run from the repository root:
#   python3 -I -B research/jumbling/restoration/12-low-height/scripts/r22b10_cert.py \
#       (record|table) <state file> <target json> <out json[.gz]> [--seconds 3600] [--seed 2210]
# API: build(alphabet, target, seconds=3600, seed=2210, force_jordan=False,
#            state_digest=None). Supply the actual state's digest explicitly
# for a duck alphabet without buffer.Alphabet's state_digest attribute.
# The alphabet is sampled, never changed. Only the current hole inventory is used.
#
# Format r22b10/1: the packet's state_digest, T_digest, T_size, seed, complete,
# missing [{least, reason}], seconds, nodes, orbits, steps, V, V_length and bound.
# Node IDs are zero-based; every reference is earlier. Nodes have op=letter
# (grip,item), cat (args, chronological), inv (arg), or pow (arg,exp>=0).
# Each node also records its exact integer expanded length as "length".
# Orbit rows contain least,size,k,route,seed,anchors,P,Q,s,e,f. P entries are
# [child,parent,node], Q entries [parent,child,node]; roots have no entry.
# Jordan rows additionally contain jordan_word (SAFE pairs), p, M, h (node),
# r and U (node). Steps are {orbit: least chamber, cycle: [x,y,z], node}.
# Words are never expanded. SAFE-ALL orbits are invariant under every leaf,
# hence every node; occupancy on an orbit depends only on the restricted action
# of V on that orbit. Collateral effects on later orbits are retained this way.
# An incomplete record is a search outcome, never a nonexistence assertion.
import time

T0 = time.monotonic()
import argparse
import gzip
import hashlib
import json
import math
import random
import signal
import sys
from collections import OrderedDict, deque
from pathlib import Path
from types import SimpleNamespace

sys.dont_write_bytecode = True
if hasattr(sys, 'set_int_max_str_digits'):
    sys.set_int_max_str_digits(0)
HERE = Path(__file__).resolve().parent
np = None


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.monotonic() - T0, 3))), flush=True)


def modules():
    global np
    if np is None:
        import numpy as np
    sys.path[:0] = [str(HERE), str(HERE.parents[2] / 'theory')]
    import buffer
    return buffer


class Deadline(Exception):
    pass


def expired(*_):
    raise Deadline('wall-clock budget exhausted')


def check_time(deadline):
    if time.monotonic() >= deadline:
        raise Deadline('wall-clock budget exhausted')


def target_check(target, digest, count=None):
    T = target['T']
    if not isinstance(T, list) or any(type(x) is not int for x in T):
        raise ValueError('target chambers must be integers')
    if hashlib.sha256(json.dumps(sorted(T)).encode()).hexdigest() != target['T_digest']:
        raise ValueError('target digest differs from T')
    if not isinstance(digest, str) or digest != target['state_digest']:
        raise ValueError('state digest differs from target state')
    if len(set(T)) != len(T) or any(x < 0 or (count is not None and x >= count) for x in T):
        raise ValueError('target chambers are duplicated or out of range')


def cycle_data(p):
    labels = np.arange(len(p))
    q = p.copy()
    for _ in range(len(p).bit_length()):
        labels = np.minimum(labels, labels[q])
        q = q[q]
    return labels, np.bincount(labels, minlength=len(p))


def power(p, exponent):
    """Exact power by cycles; work does not scale with the exponent's value."""
    out = np.arange(len(p), dtype=np.int32)
    if not exponent:
        return out
    seen = np.zeros(len(p), bool)
    for start in range(len(p)):
        if seen[start]:
            continue
        xs, x = [], start
        while not seen[x]:
            seen[x] = True
            xs.append(x)
            x = int(p[x])
        shift = exponent % len(xs)
        out[xs] = xs[shift:] + xs[:shift]
    return out


class Program:
    def __init__(self):
        self.nodes, self.lengths, self.intern = [], [], {}
        self.empty = self.cat([])

    def add(self, key, data, length):
        if key not in self.intern:
            if len(self.nodes) >= 1000000:
                raise ValueError('straight-line program node memory budget reached')
            self.intern[key] = len(self.nodes)
            self.nodes.append(dict(data, length=length))
            self.lengths.append(length)
        return self.intern[key]

    def letter(self, G, n):
        g, i = G.pairs([int(n)])[0]
        return self.add(('letter', g, i), dict(op='letter', grip=g, item=i), 1)

    def cat(self, args):
        args = tuple(n for n in args if self.lengths[n])
        if len(args) == 1:
            return args[0]
        return self.add(('cat', args), dict(op='cat', args=list(args)), sum(self.lengths[n] for n in args))

    def inv(self, n):
        if not self.lengths[n]:
            return n
        if self.nodes[n]['op'] == 'inv':
            return self.nodes[n]['arg']
        return self.add(('inv', n), dict(op='inv', arg=n), self.lengths[n])

    def pow(self, n, e):
        if not e:
            return self.empty
        if e == 1:
            return n
        return self.add(('pow', n, e), dict(op='pow', arg=n, exp=e), self.lengths[n] * e)

    def word(self, G, word):
        return self.cat([self.letter(G, n) for n in word])


class Action:
    """Dense restricted permutations, with a bounded per-orbit node memo."""
    def __init__(self, G, D, program, deadline, cache_bytes=48 * 1024**2):
        self.G, self.D, self.program, self.deadline = G, D, program, deadline
        self.unit = np.arange(len(D), dtype=np.int32)
        self.local = np.full(G.NP, -1, np.int32)
        self.local[D] = self.unit
        self.caps = {}
        for g in G.by_grip:
            xs = self.local[G.capx[g]]
            pos = np.flatnonzero(xs >= 0)
            self.caps[g] = (xs[pos], pos)
        self.cache = OrderedDict()
        self.limit = max(1, cache_bytes // max(1, self.unit.nbytes))

    def letter(self, n):
        g, _, _, ys = self.G.letters[n]
        xs, pos = self.caps[g]
        p = self.unit.copy()
        p[xs] = self.local[ys[pos]]
        if (p < 0).any():
            raise ValueError('SAFE letter leaves its recomputed orbit')
        return p

    def save(self, n, p):
        self.cache[n] = p
        self.cache.move_to_end(n)
        while len(self.cache) > self.limit:
            self.cache.popitem(last=False)
        return p

    def eval(self, n):
        # Frames consume children one at a time, so cache eviction cannot drop
        # a permutation still needed by a parent; no Python recursion limit.
        frames = [[n, 0, None]]
        answer = None
        while frames:
            check_time(self.deadline)
            frame = frames[-1]
            ident, index, acc = frame
            node = self.program.nodes[ident]
            if index == 0 and ident in self.cache:
                answer = self.cache[ident]
                self.cache.move_to_end(ident)
                frames.pop()
            elif node['op'] == 'letter':
                answer = self.save(ident, self.letter(self.G.index[(node['grip'], node['item'])]))
                frames.pop()
            else:
                args = node['args'] if node['op'] == 'cat' else [node['arg']]
                if index:
                    if node['op'] == 'cat':
                        acc = answer[acc]
                    elif node['op'] == 'inv':
                        acc = self.unit.copy()
                        acc[answer] = self.unit
                    else:
                        acc = power(answer, node['exp'])
                    frame[2] = acc
                if index == len(args):
                    answer = self.save(ident, self.unit.copy() if acc is None else acc)
                    frames.pop()
                else:
                    frame[1] += 1
                    if acc is None and node['op'] == 'cat':
                        frame[2] = self.unit
                    frames.append([args[index], 0, None])
        return answer


class LetterGraph:
    def __init__(self, action):
        self.A = action
        src, dst, nums = [], [], []
        G = action.G
        for g in G.by_grip:
            check_time(action.deadline)
            xs, pos = action.caps[g]
            for n in G.by_grip[g]:
                ys = action.local[G.letters[n][3][pos]]
                moving = xs != ys
                src.append(xs[moving])
                dst.append(ys[moving])
                nums.append(np.full(int(moving.sum()), n, np.int32))
        src = np.concatenate(src) if src else np.zeros(0, np.int32)
        order = np.argsort(src, kind='stable')
        self.dst = (np.concatenate(dst) if dst else src)[order]
        self.nums = (np.concatenate(nums) if nums else src)[order]
        self.offset = np.searchsorted(src[order], np.arange(len(action.D) + 1))


class WordTree:
    """BFS closure under single letters and incrementally added word edges."""
    def __init__(self, graph, root, fixed):
        self.graph, self.A = graph, graph.A
        self.P, self.G = self.A.program, self.A.G
        self.root, self.fixed = root, fixed
        n = len(self.A.D)
        self.parent = np.full(n, -1, np.int32)
        self.via = np.full(n, -1, np.int64)
        self.paths = [None] * n
        self.cost = [0] * n
        self.parent[root], self.paths[root] = root, self.P.empty
        self.allowed = np.ones(len(self.G.letters), bool)
        for k in range(len(self.G.letters)):
            if k % 128 == 0:
                check_time(self.A.deadline)
            self.allowed[k] = np.array_equal(self.G.images([k], self.A.D[fixed]), self.A.D[fixed])
        self.words = []
        self.word_keys = {}
        self.word_limit = max(2, (64 * 1024**2) // max(1, self.A.unit.nbytes))
        self.queue = deque([root])
        self.spread()

    def install(self, xs, ys, word):
        # Choose the first parent per new vertex deterministically.
        keep = self.parent[ys] < 0
        xs, ys = xs[keep], ys[keep]
        if not len(ys):
            return
        _, first = np.unique(ys, return_index=True)
        for j in first.tolist():
            x, y = int(xs[j]), int(ys[j])
            edge = word(j)
            self.parent[y], self.via[y] = x, edge
            self.paths[y] = self.P.cat([self.paths[x], edge])
            self.cost[y] = self.cost[x] + self.P.lengths[edge]
            self.queue.append(y)

    def spread(self):
        graph = self.graph
        while self.queue:
            check_time(self.A.deadline)
            x = self.queue.popleft()
            lo, hi = graph.offset[x:x + 2]
            nums, ys = graph.nums[lo:hi], graph.dst[lo:hi]
            keep = self.allowed[nums]
            nums, ys = nums[keep], ys[keep]
            # Filter before install, so the callback uses the same indices.
            keep = self.parent[ys] < 0
            nums, ys = nums[keep], ys[keep]
            self.install(np.full(len(ys), x), ys, lambda j: self.P.letter(self.G, nums[j]))
            for node, p in self.words:
                y = int(p[x])
                if self.parent[y] < 0:
                    self.install(np.array([x]), np.array([y]), lambda j: node)

    def add_word(self, node):
        p = self.A.eval(node)
        if not np.array_equal(p[self.fixed], self.fixed):
            raise AssertionError('Schreier generator moves its anchors')
        q = np.argsort(p).astype(np.int32)
        for ident, perm in ((node, p), (self.P.inv(node), q)):
            if np.array_equal(perm, self.A.unit):
                continue
            key = hashlib.sha256(perm.tobytes()).digest()
            if any(np.array_equal(perm, self.words[i][1]) for i in self.word_keys.get(key, [])):
                continue
            if len(self.words) >= self.word_limit:
                raise ValueError('word-table generator memory budget reached before connectivity')
            self.word_keys.setdefault(key, []).append(len(self.words))
            self.words.append((ident, perm))
            xs = np.flatnonzero(self.parent >= 0)
            ys = perm[xs]
            self.install(xs, ys, lambda j: ident)
            self.spread()

    def complete(self):
        wanted = np.ones(len(self.parent), bool)
        wanted[self.fixed] = False
        return bool((self.parent[wanted] >= 0).all())

    def rows(self, forward):
        rows = []
        for child in range(len(self.parent)):
            if child in self.fixed or child == self.root:
                continue
            parent, edge = int(self.parent[child]), int(self.via[child])
            if forward:
                rows.append([int(self.A.D[parent]), int(self.A.D[child]), edge])
            else:
                rows.append([int(self.A.D[child]), int(self.A.D[parent]), self.P.inv(edge)])
        return rows


def commutator_seed(A, earlier, rng, deadline, trials=384):
    G, P, D = A.G, A.program, A.D
    grips = sorted(G.by_grip)
    inside = np.zeros((len(D), len(grips)), bool)
    for j, g in enumerate(grips):
        inside[A.caps[g][0], j] = True
    counts = np.zeros((len(grips), len(grips)), np.int32)
    for j, g in enumerate(grips):
        check_time(deadline)
        counts[j] = inside[A.caps[g][0]].sum(axis=0)
    u, v = np.where(np.triu(counts > 0, 1))
    order = list(range(len(u)))
    rng.shuffle(order)
    order.sort(key=lambda j: int(counts[u[j], v[j]]))
    if not order:
        return None
    # Fixed trial counts make successful, untruncated runs reproducible.
    for trial in range(trials):
        check_time(deadline)
        j = order[(trial // 8) % min(256, len(order))] if trial < 256 else rng.choice(order)
        x = int(rng.choice(G.by_grip[grips[int(u[j])]]))
        right = [int(rng.choice(G.by_grip[grips[int(v[j])]]))]
        if trial >= trials // 2:
            right.append(rng.randrange(len(G.letters)))
        word = [x] + right + [int(G.inv[x])] + [int(G.inv[n]) for n in right[::-1]]
        perm = G.images(word, D)
        p = A.local[perm]
        _, lens = cycle_data(p)
        thirds = np.flatnonzero((lens != 0) & (lens % 3 == 0))
        if len(thirds) != 1 or lens[thirds[0]] != 3:
            continue
        m = 1
        for length in np.unique(lens[lens > 1]).tolist():
            if length != 3:
                m = math.lcm(m, length)
        if len(earlier):
            ep = np.searchsorted(earlier, G.images(word, earlier))
            _, elens = cycle_data(ep)
            if (elens[elens != 0] % 3 == 0).any():
                continue
            for length in np.unique(elens[elens > 1]).tolist():
                m = math.lcm(m, length)
        node = P.pow(P.word(G, word), m)
        return dict(route='commutator', seed=node)
    return None


class JordanPool:
    """Share full-alphabet word evaluations between all target orbits."""
    def __init__(self, G, labels, seed, B):
        self.G, self.labels, self.B = G, labels, B
        self.found = {}
        self.candidates = self.words(seed)

    def words(self, seed):
        for stage, (length, trials) in enumerate(((120, 200), (400, 400), (800, 400), (1500, 400))):
            # Default 2210 reproduces the R22B-04 stream (1 + stage).
            rng = random.Random(seed - 2209 + stage)
            for _ in range(trials):
                yield [rng.randrange(len(self.G.letters)) for _ in range(length)]

    def get(self, lab, deadline):
        G, B = self.G, self.B
        if G.orbit_size[lab] < 8:
            raise ValueError('global Jordan route requires degree at least 8; bounded commutator search found no seed')
        while lab not in self.found:
            check_time(deadline)
            word = next(self.candidates, None)
            if word is None:
                raise ValueError('no global Jordan word in the fixed staged search')
            labels, lengths = B.cycles(G.perm(word))
            for o in self.labels:
                if o in self.found or G.orbit_size[o] < 8:
                    continue
                try:
                    cert = B._jordan_from_cycles(G, labels, lengths, o)
                except ValueError:
                    continue
                m = 1
                for r in np.flatnonzero(lengths).tolist():
                    if r != cert['cycle_least_point']:
                        m = math.lcm(m, int(lengths[r]))
                self.found[o] = (word, cert['prime'], m)
        word, prime, m = self.found[lab]
        B.check_jordan(G, G.pairs(word), lab, prime)  # global, including all other orbits
        return word, prime, m


def small_support(A, rng):
    G, P, d = A.G, A.program, len(A.D)
    best = None
    for n in range(len(G.letters)):
        if n % 64 == 0:
            check_time(A.deadline)
        p = A.letter(n)
        r = int((p != A.unit).sum())
        if 2 <= r and 2 * r - 1 <= d and (best is None or r < best[0]):
            best = r, n
    if best is not None:
        return P.letter(G, best[1])
    # This includes r^-1 b in the nine-point derangement alphabet.
    choices = [(x, y) for x in range(len(G.letters)) for y in range(len(G.letters))] if len(G.letters) <= 32 else []
    rng.shuffle(choices)
    for j in range(max(512, len(choices))):
        check_time(A.deadline)
        word = list(choices[j]) if j < len(choices) else [rng.randrange(len(G.letters)) for _ in range(2 + j % 2)]
        p = A.local[G.images(word, A.D)]
        r = int((p != A.unit).sum())
        if 2 <= r and 2 * r - 1 <= d:
            return P.word(G, word)
    raise ValueError('no small-support word found in single letters and bounded short products')


def jordan_seed(A, graph, lab, pool, rng):
    G, P = A.G, A.program
    word, prime, m = pool.get(lab, A.deadline)
    base = P.pow(P.word(G, word), m)
    ap = A.eval(base)
    support = np.flatnonzero(ap != A.unit)
    if len(support) != prime:
        raise AssertionError('isolated Jordan power is not a p-cycle')
    h = small_support(A, rng)
    R = np.flatnonzero(A.eval(h) != A.unit)
    # Multi-source single-letter paths from supp(A); roots have the empty word.
    paths = [None] * len(A.D)
    queue = deque(support.tolist())
    for x in queue:
        paths[x] = P.empty
    while queue:
        check_time(A.deadline)
        x = queue.popleft()
        lo, hi = graph.offset[x:x + 2]
        for y, n in zip(graph.dst[lo:hi].tolist(), graph.nums[lo:hi].tolist()):
            if paths[y] is None:
                paths[y] = P.cat([paths[x], P.letter(G, n)])
                queue.append(y)
    if any(x is None for x in paths):
        raise AssertionError('single-letter orbit graph is disconnected')
    conjugators = sorted(set(paths))
    cyc, x = [], int(support[0])
    while x not in cyc:
        cyc.append(x)
        x = int(ap[x])
    cyc = np.array(cyc, np.int32)
    position = np.full(len(A.D), -1, np.int32)
    position[cyc] = np.arange(prime)
    U, S = P.empty, R.copy()
    # For each conjugator, histogram all nonzero powers exactly. The overlap
    # is a fixed part outside its p-cycle plus a cyclic difference count.
    for iteration in range(256):
        check_time(A.deadline)
        if len(np.intersect1d(R, S)) == 1:
            hp = P.cat([P.inv(U), h, U])
            seed = P.cat([h, hp, P.inv(h), P.inv(hp)])
            return dict(route='jordan', seed=seed, jordan_word=G.pairs(word), p=prime,
                        M=m, h=h, r=len(R), U=U)
        sample = rng.sample(conjugators, min(24, len(conjugators)))
        # Ensure a support that covers one currently intersecting chamber is tried.
        common = np.intersect1d(R, S)
        if len(common):
            sample.append(paths[int(rng.choice(common))])
        best = None
        for u in sample:
            up = A.eval(u)
            ui = np.argsort(up)
            ss, rr = ui[S], ui[R]
            fixed = int(np.intersect1d(ss[position[ss] < 0], rr[position[rr] < 0]).size)
            si, ri = position[ss], position[rr]
            si, ri = si[si >= 0], ri[ri >= 0]
            differences = ((ri[:, None] - si[None, :]) % prime).ravel()
            scores = np.bincount(differences, minlength=prime) + fixed
            exact = np.flatnonzero(scores[1:] == 1) + 1
            choices = exact if len(exact) else np.flatnonzero(scores[1:] == scores[1:].min()) + 1
            j = int(rng.choice(choices))
            score = int(scores[j])
            rank = (0 if score == 1 else 1, score)
            if best is None or rank < best[0]:
                best = rank, u, j, up, ui
        _, u, j, up, ui = best
        old = ui[S]
        pos = position[old]
        new = old.copy()
        inside = pos >= 0
        new[inside] = cyc[(pos[inside] + j) % prime]
        S = np.sort(up[new])
        U = P.cat([U, P.inv(u), P.pow(base, j), u])
    raise ValueError('bounded exact Jordan support-separation search did not reach overlap one')


def tables(A, graph, anchors, rng):
    P, G = A.program, A.G
    a, b, c = anchors
    R = WordTree(graph, a, [])
    if not R.complete():
        raise ValueError('single-letter orbit tree did not complete')

    def schreier():
        x = rng.randrange(len(A.D))
        n = rng.randrange(len(G.letters))
        # A single leaf's image at x needs no dense permutation allocation.
        y = int(A.local[G.images([n], [int(A.D[x])])[0]])
        return P.cat([R.paths[x], P.letter(G, n), P.inv(R.paths[y])])

    B = WordTree(graph, b, [a])
    while not B.complete():
        check_time(A.deadline)
        B.add_word(schreier())
    Q = WordTree(graph, c, [a, b])
    while not Q.complete():
        check_time(A.deadline)
        h = schreier()
        hp = A.eval(h)
        x = rng.randrange(len(A.D) - 1)
        x += x >= a
        y = int(hp[x])
        j = P.cat([B.paths[x], h, P.inv(B.paths[y])])
        Q.add_word(j)
    return B, Q


def anchored_cycle(A, row, B, Q, x, y):
    P = A.program
    px = P.inv(B.paths[x])
    w = int(A.eval(px)[y])
    U = P.cat([px, P.inv(Q.paths[w])])
    return P.cat([U, row['seed'], P.inv(U)])


def three_cycle(A, row, B, Q, p, q, r):
    a = int(A.local[row['anchors'][0]])
    cycle = [p, q, r]
    if a in cycle:
        j = cycle.index(a)
        return anchored_cycle(A, row, B, Q, cycle[(j + 1) % 3], cycle[(j + 2) % 3])
    return A.program.cat([anchored_cycle(A, row, B, Q, p, q), anchored_cycle(A, row, B, Q, r, p)])


def vacate_orbit(A, row, B, Q, target_mask, V):
    P, D = A.program, A.D
    occupied = np.empty(len(D), bool)
    occupied[A.eval(V)] = A.G.occupied[D]
    inT = target_mask[D]
    steps = []
    while occupied[inT].any():
        check_time(A.deadline)
        t = int(np.flatnonzero(inT & occupied)[0])
        z = int(np.flatnonzero(~inT & ~occupied)[0])
        choices = np.flatnonzero(~inT & (A.unit != z))
        if not len(choices):
            choices = np.flatnonzero((A.unit != z) & (A.unit != t))
        r = int(choices[0])
        triple = [z, t, r] if occupied[r] else [z, r, t]
        node = three_cycle(A, row, B, Q, *triple)
        p = A.eval(node)
        want = A.unit.copy()
        want[triple] = triple[1:] + triple[:1]
        if not np.array_equal(p, want):
            raise AssertionError('completed step is not its stated three-cycle')
        new = np.empty_like(occupied)
        new[p] = occupied
        occupied = new
        steps.append(dict(orbit=int(D[0]), cycle=D[triple].tolist(), node=node))
    return steps


def compact(result, P):
    """Keep only nodes reachable from the public certificate, retaining order."""
    # Construct a replacement without mutating public IDs. A wall-clock alarm
    # during compaction leaves the original, valid (unpruned) DAG intact.
    replacement = dict(V=result['V'], orbits=[], steps=[dict(s) for s in result['steps']])
    for row in result['orbits']:
        replacement['orbits'].append(dict(row, P=[list(e) for e in row['P']], Q=[list(e) for e in row['Q']]))
    fields = [(replacement, 'V')]
    for row in replacement['orbits']:
        fields.append((row, 'seed'))
        for name in ('h', 'U'):
            if name in row:
                fields.append((row, name))
        fields.extend((entry, 2) for key in ('P', 'Q') for entry in row[key])
    fields.extend((step, 'node') for step in replacement['steps'])
    needed, todo = set(), [obj[key] for obj, key in fields]
    while todo:
        n = todo.pop()
        if n in needed:
            continue
        needed.add(n)
        node = P.nodes[n]
        todo.extend(node['args'] if node['op'] == 'cat' else [node['arg']] if 'arg' in node else [])
    mapping, nodes = {}, []
    for n in sorted(needed):
        mapping[n] = len(nodes)
        node = dict(P.nodes[n])
        if 'arg' in node:
            node['arg'] = mapping[node['arg']]
        if 'args' in node:
            node['args'] = [mapping[x] for x in node['args']]
        nodes.append(node)
    for obj, key in fields:
        obj[key] = mapping[obj[key]]
    replacement['nodes'] = nodes
    result.update(replacement)


def build(G, target, seconds=3600, seed=2210, force_jordan=False, deadline=None, result=None, state_digest=None):
    start = time.monotonic()
    deadline = start + seconds if deadline is None else deadline
    digest = getattr(G, 'state_digest', state_digest)
    if state_digest is not None and state_digest != digest:
        raise ValueError('state digest differs from alphabet state')
    target_check(target, digest, G.NP)
    Bmodule = modules()
    P = Program()
    result = {} if result is None else result
    result.update(format='r22b10/1', state_digest=digest, T_digest=target['T_digest'], T_size=len(target['T']),
                  seed=seed, complete=False,
                  missing=[dict(least=None, reason='orbit loading exceeded budget before orbits were known')],
                  seconds=0, nodes=P.nodes, orbits=[], steps=[],
                  V=P.empty, V_length=0, bound=0)
    orbit, sizes = G.orbits()
    T = np.array(target['T'], np.int64)
    mask = np.zeros(G.NP, bool)
    mask[T] = True
    labels = sorted(set(orbit[T].tolist()), key=lambda o: (-int(sizes[o]), int(np.flatnonzero(orbit == o)[0])))
    pending = {int(np.flatnonzero(orbit == o)[0]): 'orbit not reached within time bound' for o in labels if sizes[o] > 1}
    for x in T[sizes[orbit[T]] == 1].tolist():
        if G.occupied[x]:
            pending[x] = 'N1 fails: occupied singleton target'
    result['missing'] = [dict(least=p, reason=reason) for p, reason in pending.items()]
    pool = JordanPool(G, labels, seed, Bmodule)
    earlier = np.zeros(0, np.int64)
    rng = random.Random(seed)
    try:
        condition = Bmodule.condition(G, T)
        if not condition['necessary']:
            for o in labels:
                D = np.flatnonzero(orbit == o)
                if int((~G.occupied[D]).sum()) < int(mask[D].sum()):
                    pending[int(D[0])] = 'N1/N2 fails: fewer holes than target chambers'
            return result
        for j, lab in enumerate(labels):
            D = np.flatnonzero(orbit == lab)
            if len(D) == 1:
                continue
            least = int(D[0])
            share = max(0, deadline - time.monotonic()) / max(1, sum(sizes[o] > 1 for o in labels[j:]))
            orbit_end = min(deadline, time.monotonic() + share)
            phase = 'seed'
            try:
                check_time(orbit_end)
                if len(D) < 5:
                    raise ValueError('degree below five is outside the seed-and-table construction')
                A = Action(G, D, P, orbit_end)
                row = None
                if not force_jordan:
                    try:
                        row = commutator_seed(A, earlier, rng, min(orbit_end, time.monotonic() + min(8, share * 0.12)))
                    except Deadline:
                        pass
                graph = LetterGraph(A)
                if row is None:
                    row = jordan_seed(A, graph, lab, pool, rng)
                p = A.eval(row['seed'])
                moved = np.flatnonzero(p != A.unit)
                if len(moved) != 3:
                    raise AssertionError('seed is not a three-cycle')
                a = int(moved[0])
                b, c = int(p[a]), int(p[p[a]])
                if p[c] != a:
                    raise AssertionError('seed is not a three-cycle')
                if len(earlier):
                    previous = Action(G, earlier, P, orbit_end, cache_bytes=8 * 1024**2)
                    if not np.array_equal(previous.eval(row['seed']), previous.unit):
                        raise AssertionError('seed moves an earlier target orbit')
                phase = 'word tables'
                first, second = tables(A, graph, [a, b, c], rng)
                row.update(least=least, size=len(D), k=int(mask[D].sum()), anchors=D[[a, b, c]].tolist(),
                           P=first.rows(False), Q=second.rows(True), s=P.lengths[row['seed']],
                           e=max(first.cost), f=max(second.cost))
                phase = 'vacating steps'
                steps = vacate_orbit(A, row, first, second, mask, result['V'])
                V = P.cat([result['V']] + [s['node'] for s in steps])
                result['orbits'].append(row)
                result['steps'].extend(steps)
                result['V'], result['V_length'] = V, P.lengths[V]
                result['bound'] += 2 * row['k'] * (row['s'] + 2 * row['e'] + 2 * row['f'])
                del pending[least]
                log(least=least, route=row['route'], size=len(D), s=row['s'], e=row['e'], f=row['f'], steps=len(steps))
            except (Deadline, ValueError) as error:
                pending[least] = f'{phase}: {error}'
            finally:
                earlier = np.sort(np.concatenate([earlier, D]))
            # Drop the current orbit's dense caches before opening the next.
            A = graph = first = second = previous = None
        result['complete'] = not pending
        if result['V_length'] > result['bound']:
            raise AssertionError('constructed word exceeds the recomputed bound')
    finally:
        result['missing'] = [dict(least=p, reason=reason) for p, reason in pending.items()]
        compact(result, P)
        result['seconds'] = round(time.monotonic() - start, 6)
    return result


def write_result(path, result):
    stream = gzip.open(path, 'wt', encoding='utf-8', compresslevel=3) if path.endswith('.gz') else open(path, 'w', encoding='utf-8')
    with stream:
        json.dump(result, stream)
        stream.write('\n')


def main(argv=None):
    ap = argparse.ArgumentParser(description='Build an R22B-10 vacating certificate; incomplete is a bounded search result.')
    ap.add_argument('mode', choices=('record', 'table'))
    ap.add_argument('state')
    ap.add_argument('target')
    ap.add_argument('out')
    ap.add_argument('--seconds', type=float, default=3600)
    ap.add_argument('--seed', type=int, default=2210)
    args = ap.parse_args(argv)
    if not math.isfinite(args.seconds) or args.seconds <= 0:
        ap.error('--seconds must be positive and finite')
    result = dict(format='r22b10/1', state_digest=None, T_digest=None, T_size=0, seed=args.seed,
                  complete=False, missing=[dict(least=None, reason='loading exceeded budget before orbits were known')],
                  nodes=[dict(op='cat', args=[], length=0)], orbits=[], steps=[], V=0, V_length=0, bound=0)
    # Arm before NumPy, geometry imports, state loading and alphabet construction.
    # Reserve a small part of the total budget for a recoverable output record.
    end = T0 + args.seconds - min(10, args.seconds / 10)
    handler = signal.signal(signal.SIGALRM, expired)
    phase = 'target loading'
    try:
        check_time(end)
        signal.setitimer(signal.ITIMER_REAL, max(0.000001, end - time.monotonic()))
        target = json.loads(Path(args.target).read_text())
        target_check(target, target['state_digest'])
        result.update(T_digest=target['T_digest'], T_size=len(target['T']))
        modules()
        import screen
        phase = 'state loading'
        st, M, _ = screen.load(args.mode, args.state)
        result['state_digest'] = st.digest()
        target_check(target, st.digest(), len(st.pose_id))
        phase = 'alphabet loading'
        import buffer as B
        G = B.Alphabet(SimpleNamespace(st=st), M)
        phase = 'certificate construction'
        build(G, target, seed=args.seed, deadline=end, result=result, state_digest=st.digest())
    except Deadline:
        if result.get('missing') and result['missing'][0]['least'] is None:
            result['missing'][0]['reason'] = f'{phase}: wall-clock budget exhausted before orbits were known'
        log(timeout=True, phase=phase)
    except (ValueError, KeyError, OSError, AssertionError) as error:
        print(f'FAIL: {error}', flush=True)
        return 1
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, handler)
    result['seconds'] = round(time.monotonic() - T0, 6)
    # Serialization is part of the same budget. If a large DAG cannot be
    # written in time, replace it with a small incomplete record naming ALL
    # known target orbits, rather than leaving a truncated output file.
    handler = signal.signal(signal.SIGALRM, expired)
    finish = T0 + args.seconds - min(0.05, args.seconds / 100)
    try:
        try:
            check_time(finish)
            signal.setitimer(signal.ITIMER_REAL, max(0.000001, finish - time.monotonic()))
            write_result(args.out, result)
        except Deadline:
            signal.setitimer(signal.ITIMER_REAL, 0)
            known = {row['least']: 'serialization budget exhausted; constructed program omitted' for row in result['orbits']}
            known.update({row['least']: row['reason'] for row in result['missing']})
            if not known:
                known[None] = 'serialization budget exhausted before orbit evidence could be saved'
            result.update(complete=False, missing=[dict(least=p, reason=reason) for p, reason in known.items()],
                          nodes=[dict(op='cat', args=[], length=0)], orbits=[], steps=[], V=0, V_length=0, bound=0,
                          seconds=round(time.monotonic() - T0, 6))
            write_result(args.out, result)
    except OSError as error:
        print(f'FAIL: {error}', flush=True)
        return 1
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, handler)
    log(complete=result['complete'], missing=result['missing'], V_length=result['V_length'],
        bound=result['bound'], seconds=round(time.monotonic() - T0, 6))
    return 0


if __name__ == '__main__':
    sys.exit(main())
