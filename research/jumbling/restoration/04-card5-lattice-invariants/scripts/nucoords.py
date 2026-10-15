"""Coherent coordinates (execution-tree transport) and the invariant nu on labelled lattice states."""
import gzip
import json
from pathlib import Path

import numpy as np

NS = 259800
HG = {0: 'C2', 3: 'C2', 8: 'C2', 11: 'C2', 22: 'C2', 25: 'C2', 32: 'C2', 17: 'C5', 28: 'C5', 6: 'D5', 34: 'A5'}


def _pmul(x, y):
    return tuple(y[x[i]] for i in range(len(x)))


def _pord(x):
    e, y, n = tuple(range(len(x))), x, 1
    while y != e:
        y, n = _pmul(y, x), n + 1
    return n


class NuCoords:
    def __init__(self, root):
        a = Path(root) / 'assets'
        z = np.load(a / 'model.npz', allow_pickle=False)
        self.sp = z['slot_piece'].astype(np.int64)
        self.oid = z['orbit_id'].astype(np.int64)
        atlas = json.loads((a / 'seed_atlas.json').read_text())
        with gzip.open(a / 'execution_trees.json.gz', 'rt') as f:
            trees = json.load(f)
        self.pos_of_slot = np.full(NS, -1, np.int64)
        self.lab_of_slot = np.full(NS, -1, np.int64)
        self.first_slot = {}
        self.k = {}
        self.ab = {}
        self.lab = {}
        for o in range(35):
            t, at = trees[o], atlas[o]
            A, B, C = t['buffers'][0], t['buffers'][1], t['third']
            lab = {A: at['frames'][0], B: at['frames'][1]}
            Hs = set()
            fc = at['frames'][2]
            for n, p in enumerate(t['positions']):
                if p not in lab:
                    lab[p] = t['frames'][n]
                if p == C:
                    inv = {x: j for j, x in enumerate(fc)}
                    Hs.add(tuple(inv[x] for x in t['frames'][n]))
            self.k[o] = len(fc)
            for p, sl in lab.items():
                sl = np.array(sl, np.int64)
                self.pos_of_slot[sl] = p
                self.lab_of_slot[sl] = np.arange(len(sl))
                self.lab[p] = sl
            k = len(fc)
            if o not in HG:
                self.ab[o] = (None, 1)
            elif HG[o] == 'C2':
                self.ab[o] = ({h: int(h != tuple(range(k))) for h in Hs}, 2)
            elif HG[o] == 'D5':
                self.ab[o] = ({h: int(_pord(h) == 2) for h in Hs}, 2)
            elif HG[o] == 'C5':
                r0 = min(h for h in Hs if _pord(h) == 5)
                m, x = {}, tuple(range(k))
                for i in range(5):
                    m[x] = i
                    x = _pmul(x, r0)
                self.ab[o] = (m, 5)
            else:
                self.ab[o] = ({h: 0 for h in Hs}, 1)
        assert np.all((self.pos_of_slot >= 0) == (self.oid[self.sp] >= 0))

    def nu(self, L):
        """{orbit: (sign bit, frame class)} for the non-zero orbits of labelled lattice state L."""
        moved = np.flatnonzero(L != np.arange(NS))
        moved = moved[self.oid[self.sp[moved]] >= 0]
        X = np.unique(self.pos_of_slot[moved])
        out = {}
        by = {}
        for x in X.tolist():
            by.setdefault(int(self.oid[x]), []).append(x)
        for o, xs in by.items():
            cur = {x: int(self.sp[L[self.lab[x][0]]]) for x in xs}   # home position of piece at x
            seen, par = set(), 0
            for x in xs:
                if x in seen:
                    continue
                n, y = 0, x
                while y not in seen:
                    seen.add(y)
                    y = cur.get(y, y)
                    n += 1
                par ^= (n - 1) & 1
            tab, mod = self.ab[o]
            tot = 0
            if tab is not None:
                for x in xs:
                    v = [0] * self.k[o]
                    for t_here, s in enumerate(self.lab[x]):
                        v[int(self.lab_of_slot[L[s]])] = t_here
                    tot += tab[tuple(v)]
            if par or tot % mod:
                out[o] = (par, tot % mod)
        return out


# vector form: 2-part bits (35 sign bits + 8 frame bits), 5-part (orbits 17, 28)
C2F = [0, 3, 6, 8, 11, 22, 25, 32]
C5F = [17, 28]


def vec(nu):
    b2 = [0] * 43
    b5 = [0, 0]
    for o, (s, c) in nu.items():
        o = int(o)
        b2[o] = s & 1
        if o in C2F:
            b2[35 + C2F.index(o)] = c & 1
        if o in C5F:
            b5[C5F.index(o)] = c % 5
    return b2, b5


class Span:
    """Row-reduced spans over F2 (43 coordinates) and F5 (2 coordinates) with witness combinations."""

    def __init__(self):
        self.r2 = []   # (vector, combination bitmask over library ids)
        self.r5 = []

    def add(self, i, b2, b5):
        new = False
        x, cx = list(b2), {i: 1}
        for v, cv in self.r2:
            piv = v.index(1)
            if x[piv]:
                x = [(a ^ b) for a, b in zip(x, v)]
                cx = _xor(cx, cv)
        if any(x):
            self.r2.append((x, cx))
            new = True
        y, cy = list(b5), {i: 1}
        for v, cv in self.r5:
            piv = next(j for j, t in enumerate(v) if t)
            if y[piv]:
                f = y[piv] * pow(v[piv], -1, 5) % 5
                y = [(a - f * b) % 5 for a, b in zip(y, v)]
                cy = _lin(cy, cv, -f, 5)
        if any(y):
            self.r5.append((y, cy))
            new = True
        return new

    def residual(self, b2, b5):
        """The 2-part and 5-part left after eliminating against the span (zero iff in the span)."""
        x = list(b2)
        for v, _ in self.r2:
            if x[v.index(1)]:
                x = [(a ^ b) for a, b in zip(x, v)]
        y = list(b5)
        for v, _ in self.r5:
            piv = next(j for j, t in enumerate(v) if t)
            if y[piv]:
                f = y[piv] * pow(v[piv], -1, 5) % 5
                y = [(a - f * b) % 5 for a, b in zip(y, v)]
        return x, y

    def solve(self, b2, b5):
        """Combination (library id -> multiplicity mod 10) with the given 2- and 5-parts, or None."""
        x, cx = list(b2), {}
        for v, cv in self.r2:
            piv = v.index(1)
            if x[piv]:
                x = [(a ^ b) for a, b in zip(x, v)]
                cx = _xor(cx, cv)
        if any(x):
            return None
        y, cy = list(b5), {}
        for v, cv in self.r5:
            piv = next(j for j, t in enumerate(v) if t)
            if y[piv]:
                f = y[piv] * pow(v[piv], -1, 5) % 5
                y = [(a - f * b) % 5 for a, b in zip(y, v)]
                cy = _lin(cy, cv, f, 5)
        if any(y):
            return None
        # CRT: n = a (mod 2), n = b (mod 5) -> n in 0..9
        out = {}
        for i in set(cx) | set(cy):
            a, b = cx.get(i, 0) % 2, cy.get(i, 0) % 5
            n = next(n for n in range(10) if n % 2 == a and n % 5 == b)
            if n:
                out[i] = n
        return out


def _xor(a, b):
    c = dict(a)
    for k, v in b.items():
        c[k] = (c.get(k, 0) + v) % 2
        if not c[k]:
            del c[k]
    return c


def _lin(a, b, f, m):
    c = dict(a)
    for k, v in b.items():
        c[k] = (c.get(k, 0) + f * v) % m
        if not c[k]:
            del c[k]
    return c
