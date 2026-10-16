"""Round 11 retained cascade (SENIOR-REVIEW.md, T10-01/04 and T11-01..05).

All products are chronological, and sparse nets are source -> destination.  The
only model inputs are the committed trees, atlas, primitives and realigned seeds.
Setup words have no stored net; completed gadgets retain their full collateral
until application.  Their SLP records remain after the transient nets are freed.
"""
import numpy as np

from kit import OrbitKit, SLP
from rlib import NS, invword

BOUND = 20866844
EMPTY = (np.zeros(0, np.int64), np.zeros(0, np.int64))


def mul(*xs):
    """Compose fibre permutations in chronological order."""
    p = tuple(range(len(xs[0])))
    for x in xs:
        p = tuple(x[i] for i in p)
    return p


def inv(x):
    return tuple(x.index(i) for i in range(len(x)))


def _routes(R, kit):
    """Exact BFS transports; cache words, discard the large primitive tables.

    Reverse searches record a shortest word *to* each tariff's root.  A letter
    fixes the named position, without imposing any condition on its fibre.
    """
    cache = getattr(R, '_schedule11_routes', None)
    if cache is None:
        cache = R._schedule11_routes = {}
    if kit.o in cache:
        return cache[kit.o]
    pcs = R.orbit_pieces(kit.o)
    n = len(pcs)
    ix = np.full(R.np, -1, dtype=np.int32)
    ix[pcs] = np.arange(n)
    fs = R.psorted[R.poff[pcs]]
    ai, bi, ci = (int(ix[p]) for p in (kit.A, kit.B, kit.C))
    pm = {}
    for m in range(1, 1201):
        s, d = R.move(m)
        j = np.minimum(np.searchsorted(s, fs), len(s) - 1)
        hit = s[j] == fs
        img = fs.copy()
        img[hit] = d[j[hit]]
        p = ix[R.sp[img]]
        assert np.all(p >= 0)
        pm[m], pm[-m] = p, np.argsort(p).astype(np.int32)
    fix_a = [m for m, p in pm.items() if p[ai] == ai]

    def bfs(root, gens, excluded=()):
        dist = np.full(n, -1, dtype=np.int32)
        nxt = np.zeros(n, dtype=np.int32)
        dist[root] = 0
        front = np.array([root])
        depth = 0
        while len(front):
            depth += 1
            new = []
            for m in gens:
                y = pm[-m][front]
                y = y[dist[y] < 0]
                dist[y], nxt[y] = depth, m
                new.append(y)
            front = np.unique(np.concatenate(new))
        assert set(np.flatnonzero(dist < 0)) == set(excluded)
        words = {root: ()}
        for p in np.argsort(dist):
            if dist[p] <= 0:
                continue
            m = int(nxt[p])
            words[int(p)] = (m,) + words[int(pm[m][p])]
        return {int(pcs[p]): w for p, w in words.items()}, int(max(dist))

    out = {}
    out['a'], out['a_max'] = bfs(ai, pm)
    out['e'], out['e_max'] = bfs(bi, fix_a, (ai,))
    if kit.group_name != '1':
        out['b'], out['b_max'] = bfs(ci, pm)
        out['f'], out['f_max'] = bfs(bi, [m for m, p in pm.items() if p[ci] == ci], (ci,))
        # Preserve the tie ordering of the T11-04 reproduction, hence its tau.
        dist = np.full(n, -1, dtype=np.int32)
        nxt = np.zeros(n, dtype=np.int32)
        dist[bi] = 0
        front, depth = np.array([bi]), 0
        while dist[ci] < 0:
            depth += 1
            new = []
            for m in fix_a:
                y = pm[m][front]
                y = y[dist[y] < 0]
                dist[y], nxt[y] = depth, -m
                new.append(y)
            front = np.concatenate(new)
            assert len(front)
        p, w = ci, []
        while p != bi:
            m = int(nxt[p])
            w.append(m)
            p = int(pm[m][p])
        z = bi
        for m in w:
            z = int(pm[m][z])
        out['swap'] = w + invword(kit.path(kit.node_at[int(pcs[z])]))
    cache[kit.o] = out
    return out


def _loop_cycles(cur, A):
    """The committed position loop, planned without touching any stickers."""
    cur = cur.copy()
    unsolved = {p for p, h in cur.items() if p != h}
    out = []
    while unsolved:
        h = cur[A]
        if h == A:
            X = next(iter(unsolved))
            Y = cur[X]
        else:
            X, Y = h, cur[h]
            if Y == A:
                Y = next((p for p in unsolved if p not in (A, X)), None)
                if Y is None:
                    raise ValueError('odd position permutation')
        assert len({A, X, Y}) == 3
        out.append((A, X, Y))
        cur[A], cur[X], cur[Y] = cur[Y], cur[A], cur[X]
        for p in (A, X, Y):
            if cur[p] == p:
                unsolved.discard(p)
            else:
                unsolved.add(p)
        assert len(out) <= 3 * len(cur) // 4
    return out


def _fewest_cycles(cur):
    """T11-03: split odd cycles and pair the residual transpositions."""
    seen, transpositions, out = set(), [], []
    odd_count = 0
    for start in cur:
        if start in seen:
            continue
        cyc, p = [], start
        while p not in seen:
            seen.add(p)
            cyc.append(p)
            p = cur[p]
        assert p == start
        odd_count += len(cyc) % 2
        for j in range(1, len(cyc) - 1, 2):
            out.append((cyc[0], cyc[j], cyc[j + 1]))
        if len(cyc) % 2 == 0:
            transpositions.append((cyc[0], cyc[-1]))
    assert len(transpositions) % 2 == 0, 'odd position permutation'
    for j in range(0, len(transpositions), 2):
        a, b = transpositions[j]
        c, d = transpositions[j + 1]
        out.extend(((a, c, d), (a, c, b)))
    assert len(out) == (len(cur) - odd_count) // 2
    return out


class _Orbit11(OrbitKit):
    def __init__(self, schedule, o, c, allowed, rec):
        super().__init__(schedule.R, schedule.slp, o, c, allowed, compact_paths=True)
        self.schedule = schedule
        self.group_name = rec['H']
        self.s = self.slp.length[c]
        self.ident = tuple(range(self.k))
        self._words = {}
        self._prepared = False

    @property
    def routes(self):
        return _routes(self.R, self)

    def word(self, w):
        """A conjugating word, deliberately without a dense/sparse net."""
        key = tuple(w)
        if key not in self._words:
            self._words[key] = self.slp.word(w, net=EMPTY)
        return self._words[key]

    def prod(self, fs):
        return self.slp.prod(fs, scratch=self.schedule.scratch)

    def fibre(self, w, X, Y):
        img = self.R.map_through(self.lab[X], w)
        pp = [self.slot_pos[int(x)] for x in img]
        assert all(p == Y for p, _ in pp), (self.o, X, Y)
        return tuple(j for _, j in pp)

    def edge(self, node, X):
        pp = [self.slot_pos[int(x)] for x in SLP.apply_sparse(self.lab[X], self.slp.net[node])]
        Y = pp[0][0]
        assert all(p == Y for p, _ in pp)
        return Y, tuple(j for _, j in pp)

    def check(self, node, expected):
        self.check_support(self.slp.net[node], set(expected))
        for X, edge in expected.items():
            assert self.edge(node, X) == edge, (self.o, X, self.edge(node, X), edge)
        return node

    def needed(self, L, X, Y):
        pp = [self.slot_pos[int(x)] for x in L[self.lab[X]]]
        assert all(p == Y for p, _ in pp), (self.o, X, Y)
        return tuple(j for _, j in pp)

    def setup(self, X, Y, Z):
        """g = w1 w2 u^-1 takes (X,Y,Z) to (A,B,C)."""
        assert len({X, Y, Z}) == 3
        w1 = [] if X == self.A else list(self.routes['a'][X])
        y, z = self.R.sp[self.R.map_through([self.lab[Y][0], self.lab[Z][0]], w1)]
        w2 = list(self.routes['e'][int(y)])
        z = int(self.R.sp[self.R.map_through([self.lab[int(z)][0]], w2)[0]])
        return w1 + w2 + invword(self.path(self.node_at[z]))

    def prepare(self):
        if self._prepared:
            return
        self.H, self.returns = [self.ident], {self.ident: []}
        if self.group_name == '1':
            self._prepared = True
            return
        for node, pos in enumerate(self.pos):
            if pos != self.C:
                continue
            w = self.path(node)
            x = self.fibre(w, self.C, self.C)
            assert self.fibre(w, self.A, self.A) == self.fibre(w, self.B, self.B) == self.ident
            for v, word in ((x, w), (inv(x), invword(w))):
                if v not in self.returns or len(word) < len(self.returns[v]):
                    self.returns[v] = word
        self.H = sorted(self.returns)
        expected_size = {'C2': 2, 'C5': 5, 'D5': 10, 'A5': 60}[self.group_name]
        assert len(self.H) == expected_size
        assert all(mul(x, y) in self.returns for x in self.H for y in self.H)
        for v, w in self.returns.items():
            assert self.fibre(w, self.C, self.C) == v
        self.h = max(map(len, self.returns.values()))
        self.swap = self.routes['swap']
        assert self.fibre(self.swap, self.A, self.A) == self.ident
        b_to_c = self.fibre(self.swap, self.B, self.C)
        self.fibre(self.swap, self.C, self.B)
        self.at_B = {}
        for z, w in self.returns.items():
            v = mul(b_to_c, z, inv(b_to_c))
            bw = self.swap + w + invword(self.swap) if z != self.ident else []
            assert self.fibre(bw, self.B, self.B) == v
            assert self.fibre(bw, self.A, self.A) == self.fibre(bw, self.C, self.C) == self.ident
            self.at_B[v] = bw
        assert set(self.at_B) == set(self.H)
        self.alpha, self.beta, self.gamma = [self.edge(self.c, p)[1] for p in (self.A, self.B, self.C)]
        self.single_choices = {}
        if self.o in (6, 34):
            for x in self.H:
                for y in self.H:
                    z = mul(x, y)
                    v = mul(inv(x), inv(y), x, y)
                    cost = sum(len(self.returns[t]) for t in (x, y, z))
                    if v not in self.single_choices or cost < self.single_choices[v][0]:
                        self.single_choices[v] = (cost, x, y, z)
            assert len(self.single_choices) == (5 if self.o == 6 else 60)
            assert max(t[0] for t in self.single_choices.values()) == (8 if self.o == 6 else 5)
        self._prepared = True

    def direct_cycle(self, xyz, g):
        node = self.slp.conj(self.c, self.word(invword(g)))
        maps = [self.fibre(g, p, q) for p, q in zip(xyz, (self.A, self.B, self.C))]
        base = [self.edge(self.c, p)[1] for p in (self.A, self.B, self.C)]
        return self.check(node, {p: (xyz[(i + 1) % 3], mul(maps[i], base[i], inv(maps[(i + 1) % 3])))
                                 for i, p in enumerate(xyz)})

    def twisted_cycle(self, xyz, g, L, finalized):
        self.prepare()
        maps = [self.fibre(g, p, q) for p, q in zip(xyz, (self.A, self.B, self.C))]
        u, v = self.alpha, self.beta
        if xyz[1] in finalized:
            u = mul(inv(maps[0]), self.needed(L, xyz[0], xyz[1]), maps[1])
        if xyz[2] in finalized:
            v = mul(inv(maps[1]), self.needed(L, xyz[1], xyz[2]), maps[2])
        y = next(y for y in self.H if mul(self.alpha, y) == u)
        x = next(x for x in self.H if mul(inv(y), self.beta, x) == v)
        r = self.at_B[y] + self.returns[x]
        node = self.slp.conj(self.c, self.word(r + invword(g)))
        base = (mul(self.alpha, y), mul(inv(y), self.beta, x), mul(inv(x), self.gamma))
        expected = {p: (xyz[(i + 1) % 3], mul(maps[i], base[i], inv(maps[(i + 1) % 3])))
                    for i, p in enumerate(xyz)}
        for i in (0, 1):
            if xyz[i + 1] in finalized:
                assert expected[xyz[i]][1] == self.needed(L, xyz[i], xyz[i + 1])
        return self.check(node, expected)

    def _pair(self, X, Y, g, v, other=None):
        """Transport Q_x to X,Y by g taking X,Y to B,C."""
        a, delta = self.fibre(g, X, self.B), self.fibre(g, Y, self.C)
        x = next(x for x in self.H if mul(a, self.beta, x, inv(self.beta), inv(a)) == v)
        conj = self.slp.conj(self.c, self.word(self.returns[x]))
        q = self.prod([(conj, 1), (self.c, -1)])
        node = self.slp.conj(q, self.word(invword(g)))
        vv = mul(delta, inv(x), inv(delta))
        if other is not None:
            assert vv == other
        return self.check(node, {X: (X, v), Y: (Y, vv)})

    def direct_pair(self, X, v):
        self.prepare()
        return self._pair(X, self.C, list(self.routes['f'][X]), v)

    def c2_pair(self, X, Y, v, other):
        self.prepare()
        assert self.group_name == 'C2'
        w1 = list(self.routes['b'][Y])
        x = int(self.R.sp[self.R.map_through([self.lab[X][0]], w1)[0]])
        g = w1 + list(self.routes['f'][x])
        return self._pair(X, Y, g, v, other)

    def frame_triple(self, X, Y, va, vb):
        self.prepare()
        xyz = (X, Y, self.C)
        g = self.setup(*xyz)
        a, b, c = [self.fibre(g, p, q) for p, q in zip(xyz, (self.A, self.B, self.C))]
        y = next(y for y in self.H if mul(a, self.alpha, y, inv(self.alpha), inv(a)) == va)
        x = next(x for x in self.H if mul(b, inv(y), self.beta, x, inv(self.beta), inv(b)) == vb)
        r = self.at_B[y] + self.returns[x]
        twisted = self.slp.conj(self.c, self.word(r))
        triple = self.prod([(twisted, 1), (self.c, -1)])
        node = self.slp.conj(triple, self.word(invword(g)))
        return self.check(node, {X: (X, va), Y: (Y, vb), self.C: (self.C, mul(c, inv(x), inv(c)))})

    def cleanup_A(self, v):
        self.prepare()
        x = next(x for x in self.H if mul(inv(self.gamma), inv(x), self.gamma) == v)
        cx = self.slp.conj(self.c, self.word(self.returns[x]))
        # c^-1 r_x^-1 c r_x: cancellation avoids another pair of seeds.
        node = self.prod([(self.c, -1), (cx, 1)])
        return self.check(node, {self.A: (self.A, v), self.C: (self.C, x)})

    def cleanup_B(self, v):
        self.prepare()
        x = next(x for x in self.H if mul(self.beta, x, inv(self.beta)) == v)
        cx = self.slp.conj(self.c, self.word(self.returns[x]))
        node = self.prod([(cx, 1), (self.c, -1)])
        return self.check(node, {self.B: (self.B, v), self.C: (self.C, inv(x))})

    def single(self, v):
        self.prepare()
        _, x, y, z = self.single_choices[v]
        cx, cy, cz = [self.slp.conj(self.c, self.word(self.returns[t])) for t in (x, y, z)]
        # Q_x Q_y Q_xy^-1, with the adjacent c^-1 c removed (four seeds).
        node = self.prod([(cx, 1), (self.c, -1), (cy, 1), (cz, -1)])
        return self.check(node, {self.C: (self.C, v)})

    def bound(self):
        """Reproduce T11-05 using the computed distances/returns/swap lengths."""
        self.prepare()
        n, z = len(self.lab), self.s
        D = self.routes['e_max'] + max(self.R.trees[self.o]['depth'])
        K = self.routes['a_max'] + D
        if self.group_name == '1':
            return n // 2 * (z + 2 * K)
        P = 2 * z + 2 * self.h + 2 * self.routes['f_max']
        Q = (2 * z + 2 * self.h + 2 * (self.routes['b_max'] + self.routes['f_max'])
             if self.group_name == 'C2' else 2 * z + 4 * self.h + 4 * len(self.swap) + 2 * K)
        W = z + 4 * self.h + 4 * len(self.swap) + 2 * D
        C = 4 * z + 4 * self.h
        if self.o in (6, 34):
            C += 4 * z + (16 if self.o == 6 else 10)

        def frames(j):
            return j // 2 * Q + (j % 2) * P if Q < 2 * P else j * P
        worst = frames(n - 3)
        for m in range(3, n + 1):
            k = m // 2
            k -= (k - m) % 2
            for i in (0, 1):
                if i == 0 and m == n:
                    continue
                worst = max(worst, (m + k - 2 * i) // 2 * W + frames(n - m - (1 - i)))
        return worst + C


class Schedule11:
    """The retained cascade R; solve mutates labels and returns SLP factors."""
    def __init__(self, R, slp, realign):
        self.R, self.slp, self.realign = R, slp, realign
        self.cascade = list(realign['cascade_order'])
        assert sorted(self.cascade) == list(range(35))
        self.scratch = np.arange(NS, dtype=np.int64)
        self.kits, self.stats = {}, {}
        records = {d['orbit']: d for d in realign['orbits']}
        for rank, o in enumerate(self.cascade):
            rec, t = records[o], R.trees[o]
            if rec['replaced']:
                raw = slp.word(rec['choice']['word'])
                c = slp.conj(raw, slp.word([], net=EMPTY))
            else:
                raw = slp.word(t['seed'])
                c = slp.conj(raw, slp.word(t['relocation'], net=EMPTY))
            assert slp.length[c] == rec['s']
            collateral = set(int(x) for x in R.so[slp.net[c][0]]) - {o}
            assert collateral <= set(self.cascade[rank + 1:])
            if rec['replaced']:
                assert collateral == set(rec['choice']['collateral'])
            k = self.kits[o] = _Orbit11(self, o, c, collateral, rec)
            k.check_support(slp.net[c], {k.A, k.B, k.C})
            for X, Y in ((k.A, k.B), (k.B, k.C), (k.C, k.A)):
                assert k.edge(c, X)[0] == Y

    def solve_orbit(self, o, L):
        k = self.kits[o]
        cur = {p: int(self.R.sp[L[sl[0]]]) for p, sl in k.lab.items()}
        fixed_errors = [p for p in k.lab if p not in (k.A, k.B, k.C) and cur[p] == p
                        and k.needed(L, p, p) != k.ident]
        fs, counts = [], {}
        start = len(self.slp.nodes)

        def apply(node, kind):
            nonlocal start
            s, d = self.slp.net[node]
            L[d] = L[s]
            fs.append((node, 1))
            counts[kind] = counts.get(kind, 0) + 1
            # Completed factors are not reused.  Keep the nodes and lengths for
            # the independent evaluator, but never accumulate their exact nets.
            for j in range(start, len(self.slp.nodes)):
                self.slp.net[j] = EMPTY
            start = len(self.slp.nodes)

        loop = _loop_cycles(cur, k.A)
        if k.group_name == '1':
            candidates = []
            for method, triples in (('direct-loop', loop), ('fewest-three-cycles', _fewest_cycles(cur))):
                plan = [(xyz, k.setup(*xyz)) for xyz in triples]
                candidates.append((sum(k.s + 2 * len(g) for _, g in plan), method, plan))
            _, method, plan = min(candidates, key=lambda c: c[0])
            for xyz, g in plan:
                apply(k.direct_cycle(xyz, g), 'direct-three-cycle')
        else:
            k.prepare()
            method = ('twisted-loop+c2-pairs' if k.group_name == 'C2' else
                      'twisted-loop+direct-pairs' if o in (6, 17) else 'twisted-loop+frame-triples')
            for xyz in loop:
                A, X, Y = xyz
                finalized = set()
                if cur[A] == X:
                    finalized.add(X)
                if cur[X] == Y:
                    finalized.add(Y)
                apply(k.twisted_cycle(xyz, k.setup(*xyz), L, finalized), 'twisted-three-cycle')
                cur[A], cur[X], cur[Y] = cur[Y], cur[A], cur[X]
                for p in finalized:
                    assert k.needed(L, p, p) == k.ident
            # Only initially stationary errors remain outside the three buffers.
            for p in k.lab:
                if p not in (k.A, k.B, k.C) and p not in fixed_errors:
                    assert k.needed(L, p, p) == k.ident, (o, p, 'missed by position pass')
            if o in (6, 17):
                for X in fixed_errors:
                    v = k.needed(L, X, X)
                    if v != k.ident:
                        apply(k.direct_pair(X, v), 'direct-pair')
            else:
                j = 0
                while j + 1 < len(fixed_errors):
                    X, Y = fixed_errors[j:j + 2]
                    va, vb = k.needed(L, X, X), k.needed(L, Y, Y)
                    node = (k.c2_pair(X, Y, va, vb) if k.group_name == 'C2'
                            else k.frame_triple(X, Y, va, vb))
                    apply(node, 'c2-pair' if k.group_name == 'C2' else 'frame-triple')
                    j += 2
                if j < len(fixed_errors):
                    X = fixed_errors[j]
                    apply(k.direct_pair(X, k.needed(L, X, X)), 'direct-pair')
            for X, build, kind in ((k.A, k.cleanup_A, 'cleanup-A'), (k.B, k.cleanup_B, 'cleanup-B')):
                v = k.needed(L, X, X)
                if v != k.ident:
                    apply(build(v), kind)
            v = k.needed(L, k.C, k.C)
            if v != k.ident:
                assert o in (6, 34), (o, 'nonzero abelian frame invariant')
                apply(k.single(v), 'single')
        idx = np.concatenate(list(k.lab.values()))
        assert np.array_equal(L[idx], idx), 'orbit %d not restored' % o
        self.stats[o] = {'method': method, 'length': sum(self.slp.length[i] for i, _ in fs), 'counts': counts}
        return fs

    def solve(self, L):
        return [factor for o in self.cascade for factor in self.solve_orbit(o, L)]

    def orbit_bounds(self):
        return {o: self.kits[o].bound() for o in self.cascade}
