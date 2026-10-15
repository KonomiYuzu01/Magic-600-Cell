"""Pure-element machinery on the retained group, recorded as a straight-line program (SLP).

SLP nodes (chronological products; u^-1 x u carries the support of x forward along u):
  {"op": "word", "w": [signed primitive ids]}
  {"op": "conj", "x": id, "u": id, "us": +1|-1}     u^-us ... i.e. (u^us)^-1 x (u^us)
  {"op": "prod", "f": [[id, +1|-1], ...]}           f1 f2 ... (first f1)
Every node also carries its exact sparse net (support s sorted, images d), computed here by
exact sticker following; the independent checker recomputes everything from the words.
"""
import numpy as np

from rlib import NS, invword


class SLP:
    def __init__(self, R):
        self.R = R
        self.nodes = []
        self.net = []
        self.length = []          # expanded word length
        self.word_of = {}         # node id -> word, for word and path nodes

    def _add(self, node, net, length):
        self.nodes.append(node)
        self.net.append(net)
        self.length.append(int(length))
        return len(self.nodes) - 1

    # sparse helpers ----------------------------------------------------------------------
    @staticmethod
    def _sorted(s, d):
        ix = np.argsort(s)
        return s[ix], d[ix]

    def signed(self, i, sg):
        s, d = self.net[i]
        return (s, d) if sg > 0 else self._sorted(d, s)

    @staticmethod
    def apply_sparse(x, net):
        s, d = net
        x = np.array(x, np.int64, copy=True)
        if len(s) == 0:
            return x
        ix = np.minimum(np.searchsorted(s, x), len(s) - 1)
        hit = s[ix] == x
        x[hit] = d[ix[hit]]
        return x

    # node constructors -------------------------------------------------------------------
    def word(self, w, net=None):
        w = [int(m) for m in w]
        if net is None:
            P = self.R.word_perm(w)
            s = np.flatnonzero(P != np.arange(NS))
            net = (s, P[s])
        i = self._add({'op': 'word', 'w': w}, net, len(w))
        self.word_of[i] = w
        return i

    def conj(self, x, u, us=1):
        s, d = self.net[x]
        if u in self.word_of:
            w = self.word_of[u] if us > 0 else invword(self.word_of[u])
            s2, d2 = self.R.map_through(s, w), self.R.map_through(d, w)
        else:
            un = self.signed(u, us)
            s2, d2 = self.apply_sparse(s, un), self.apply_sparse(d, un)
        return self._add({'op': 'conj', 'x': x, 'u': u, 'us': us}, self._sorted(s2, d2),
                         self.length[x] + 2 * self.length[u])

    def prod(self, factors, scratch=None):
        L = np.arange(NS, dtype=np.int64) if scratch is None else scratch
        touched = []
        for i, sg in factors:
            s, d = self.signed(i, sg)
            L[d] = L[s]
            touched.append(d)
        t = np.unique(np.concatenate(touched)) if touched else np.zeros(0, np.int64)
        lt = L[t]
        moved = lt != t
        # L[y] = x means the sticker from x is at y: x -> y
        s, d = lt[moved], t[moved]
        if scratch is not None:
            L[t] = t
        net = self._sorted(s, d)
        return self._add({'op': 'prod', 'f': [[int(i), int(sg)] for i, sg in factors]}, net,
                         sum(self.length[i] for i, _ in factors))


class OrbitKit:
    """Stars, pairs and singles on one orbit, built from its relocated pure seed c."""

    def __init__(self, R, slp, o, c, allowed=None, compact_paths=False):
        """allowed=None: c is pure and every derived element must be pure on orbit o.
        allowed=set of orbits: c may also move those orbits (cascade mode)."""
        self.R, self.slp, self.o, self.c = R, slp, o, c
        self.allowed = allowed
        self.compact_paths = compact_paths
        t = R.trees[o]
        at = R.atlas[o]
        self.A, self.B = int(t['buffers'][0]), int(t['buffers'][1])
        self.C = int(t['third'])
        self.parent, self.mv, self.pos = t['parent'], t['move'], t['positions']
        self.lab = {self.A: np.array(at['frames'][0], np.int64), self.B: np.array(at['frames'][1], np.int64)}
        self.node_at = {}
        for n, p in enumerate(self.pos):
            if p not in self.node_at:
                self.node_at[p] = n
                self.lab[p] = np.array(t['frames'][n], np.int64)
        assert self.node_at[self.C] == 0 and list(self.lab[self.C]) == list(at['frames'][2])
        self.k = len(self.lab[self.C])
        self.positions = R.orbit_pieces(o)
        assert set(self.lab) == set(int(p) for p in self.positions)
        # slot -> (position, label)
        self.slot_pos = {}
        for p, sl in self.lab.items():
            for j, x in enumerate(sl):
                self.slot_pos[int(x)] = (p, j)
        self.path_node = {}
        self.star_node = {}
        self.pair_by_value = None
        self.single_by_value = {}
        self.other_C_nodes = [n for n, p in enumerate(self.pos) if p == self.C and n != 0]

    def path(self, n):
        out = []
        while self.parent[n] >= 0:
            out.append(int(self.mv[n]))
            n = self.parent[n]
        return out[::-1]

    def path_word_node(self, n):
        i = self.path_node.get(n)
        if i is None:
            w = self.path(n)
            # nets of setup words are never needed densely; record the word only
            node = {'op': 'path', 'orbit': self.o, 'node': int(n)} if self.compact_paths else {'op': 'word', 'w': w}
            i = self.slp._add(node, (np.zeros(0, np.int64), np.zeros(0, np.int64)), len(w))
            self.slp.word_of[i] = w
            self.path_node[n] = i
        return i

    def star(self, X):
        """t_X = u_X^-1 c u_X, the frame-preserving cycle (A B X)."""
        if X == self.C:
            return self.c
        i = self.star_node.get(X)
        if i is None:
            i = self.slp.conj(self.c, self.path_word_node(self.node_at[X]))
            self.star_node[X] = i
        return i

    def cycle_factors(self, X, Y):
        """Factors realising the position cycle A -> X -> Y -> A."""
        A, B = self.A, self.B
        assert len({A, X, Y}) == 3
        if X == B:
            return [(self.star(Y), 1)]
        if Y == B:
            return [(self.star(X), -1)]
        return [(self.star(X), -1), (self.star(Y), 1)]

    # frames ------------------------------------------------------------------------------
    def value_at(self, net, X):
        s, d = net
        img = SLP.apply_sparse(self.lab[X], (s, d))
        inv = {int(x): j for j, x in enumerate(self.lab[X])}
        return tuple(inv[int(y)] for y in img)

    def _pairs(self):
        if self.pair_by_value is not None:
            return
        self.pair_by_value = {}
        for n in self.other_C_nodes:
            g = self.path_word_node(n)
            q = self.slp.prod([(self.slp.conj(self.c, g), 1), (self.c, -1)])
            net = self.slp.net[q]
            self.check_support(net, {self.B, self.C})
            for sg in (1, -1):
                v = self.value_at(self.slp.signed(q, sg), self.B)
                self.pair_by_value.setdefault(v, (q, sg))

    def pair_to(self, v, X):
        """A pure frame element with value v at X and its compensation at C (X != C)."""
        self._pairs()
        q, sg = self.pair_by_value[v]
        if X == self.B:
            node = q if sg > 0 else self.slp.prod([(q, -1)])
        else:
            base = q if sg > 0 else self.slp.prod([(q, -1)])
            if X == self.A:
                Z = self.some_other()
                node = self.slp.conj(base, self.star(Z), -1)
            else:
                node = self.slp.conj(base, self.star(X), 1)
        assert self.value_at(self.slp.net[node], X) == v
        return node

    def check_support(self, net, pieces):
        s = net[0]
        on = self.R.so[s] == self.o
        assert set(self.R.sp[s[on]].tolist()) <= pieces, 'orbit part outside %r' % (pieces,)
        if self.allowed is None:
            assert on.all(), 'element is not pure'
        else:
            assert set(self.R.so[s[~on]].tolist()) <= self.allowed, 'collateral outside the allowed orbits'

    def some_other(self):
        for p in self.lab:
            if p not in (self.A, self.B, self.C):
                return p

    def single_at_C(self, v):
        """A pure frame element with value v at C only."""
        if v in self.single_by_value:
            return self.single_by_value[v]
        self._pairs()
        Z = self.some_other()
        ident = tuple(range(self.k))
        vals = [u for u in self.pair_by_value if u != ident]
        for x in vals:
            for y in vals:
                a = self.pair_to(x, self.B)
                b = self.pair_to(y, Z)
                sgl = self.slp.prod([(a, -1), (b, -1), (a, 1), (b, 1)])
                net = self.slp.net[sgl]
                self.check_support(net, {self.C})
                w = self.value_at(net, self.C)
                self.single_by_value.setdefault(w, sgl)
                if w == v:
                    return sgl
        raise RuntimeError('no single for value %r' % (v,))

    # solving -----------------------------------------------------------------------------
    def solve(self, L):
        """Return factors whose chronological product undoes the orbit part of label state L
        (L[slot] = home slot of the sticker now there); applies them to L as it goes."""
        R, o = self.R, self.o
        f_all = []

        def apply(fs):
            for i, sg in fs:
                s, d = self.slp.signed(i, sg)
                L[d] = L[s]
            f_all.extend(fs)

        def home(P):
            return int(R.sp[L[self.lab[P][0]]])

        cur = {P: home(P) for P in self.lab}
        unsolved = set(P for P, h in cur.items() if h != P)
        A = self.A
        guard = 0
        while unsolved:
            guard += 1
            assert guard < 10 * len(self.lab) + 10
            h = cur[A]
            if h == A:
                X = next(iter(unsolved))
                Y = cur[X]
                fs = self.cycle_factors(X, Y)
            else:
                X = h
                Y = cur[X]
                if Y == A:
                    Z = next((p for p in unsolved if p not in (A, X)), None)
                    if Z is None:
                        raise ValueError('odd position permutation on orbit %d' % o)
                    Y = Z
                fs = self.cycle_factors(X, Y)
            apply(fs)
            for P in (A, X, Y):
                cur[P] = home(P)
                if cur[P] == P:
                    unsolved.discard(P)
                else:
                    unsolved.add(P)
        # frames
        ident = tuple(range(self.k))
        for P in self.lab:
            if P == self.C:
                continue
            inv = {int(x): j for j, x in enumerate(self.lab[P])}
            e = tuple(inv[int(L[x])] for x in self.lab[P])
            if e != ident:
                apply([(self.pair_to(e, P), 1)])
                inv2 = tuple(inv[int(L[x])] for x in self.lab[P])
                assert inv2 == ident, (o, P, e, inv2)
        inv = {int(x): j for j, x in enumerate(self.lab[self.C])}
        e = tuple(inv[int(L[x])] for x in self.lab[self.C])
        if e != ident:
            apply([(self.single_at_C(e), 1)])
        idx = np.concatenate([self.lab[P] for P in self.lab])
        assert np.array_equal(L[idx], idx), 'orbit %d not restored' % o
        return f_all
