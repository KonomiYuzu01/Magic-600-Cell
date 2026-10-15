"""Independent retained-model helpers for the card-1 certificates and the lattice restorer.

Reads assets/model.npz, assets/primitives.npz, assets/seed_atlas.json and
assets/execution_trees.json.gz read-only. Does not import core.py.
Conventions: a word is a list of signed primitive ids (+-1..+-1200), executed left to right
(chronological). A permutation is source -> destination on the 259,800 sticker slots.
A state is a label array L with L[slot] = home slot of the sticker now there; a move with
(src, dst) acts by L'[dst] = L[src].
"""
import gzip
import hashlib
import json
from pathlib import Path

import numpy as np

NS = 259800


def sha(b):
    return hashlib.sha256(b).hexdigest()


class Retained:
    def __init__(self, root):
        self.root = Path(root)
        a = self.root / 'assets'
        self.files = {}
        for name in ('model.npz', 'primitives.npz', 'seed_atlas.json', 'execution_trees.json.gz'):
            self.files['assets/' + name] = sha((a / name).read_bytes())
        z = np.load(a / 'model.npz', allow_pickle=False)
        self.sp = z['slot_piece'].astype(np.int64)
        self.oid = z['orbit_id'].astype(np.int64)
        self.so = self.oid[self.sp]
        self.mo = z['mask_offsets'].astype(np.int64)
        self.masks = z['mask_values'].astype(np.int64)
        self.fo = z['face_offsets'].astype(np.int64)
        self.np = len(self.oid)
        self.psorted = np.argsort(self.sp, kind='stable')
        cnt = np.bincount(self.sp, minlength=self.np)
        self.poff = np.concatenate([[0], np.cumsum(cnt)])
        p = np.load(a / 'primitives.npz', allow_pickle=False)
        self.src = p['src'].astype(np.int64)
        self.dst = p['dst'].astype(np.int64)
        self.off = p['offsets'].astype(np.int64)
        assert len(self.off) == 1201
        self._mv = {}
        self.atlas = json.loads((a / 'seed_atlas.json').read_text())
        with gzip.open(a / 'execution_trees.json.gz', 'rt') as f:
            self.trees = json.load(f)

    # ---- pieces -------------------------------------------------------------------------
    def slots(self, p):
        return self.psorted[self.poff[p]:self.poff[p + 1]]

    def caps(self, p):
        return set(int(x) for x in self.masks[self.mo[p]:self.mo[p + 1]])

    def orbit_pieces(self, o):
        return np.flatnonzero(self.oid == o)

    # ---- moves --------------------------------------------------------------------------
    def move(self, m):
        """(src, dst) of signed primitive m, src sorted ascending."""
        r = self._mv.get(m)
        if r is None:
            i = abs(m) - 1
            if not (isinstance(m, int) and 0 <= i < 1200):
                raise ValueError('bad primitive id %r' % (m,))
            s, d = self.src[self.off[i]:self.off[i + 1]], self.dst[self.off[i]:self.off[i + 1]]
            if m < 0:
                s, d = d, s
            ix = np.argsort(s)
            r = (s[ix], d[ix])
            self._mv[m] = r
        return r

    @staticmethod
    def cap_of(m):
        return (abs(m) - 1) // 2

    def apply_word(self, L, w):
        """Apply a word to a label array in place."""
        for m in w:
            s, d = self.move(m)
            L[d] = L[s]
        return L

    def map_through(self, x, w):
        """Image of slot array x under the word w (each slot followed forward)."""
        x = np.array(x, np.int64, copy=True)
        for m in w:
            s, d = self.move(m)
            ix = np.searchsorted(s, x)
            ix = np.minimum(ix, len(s) - 1)
            hit = s[ix] == x
            x[hit] = d[ix[hit]]
        return x

    def word_perm(self, w):
        """Complete permutation P (P[source] = destination) of a word, by full evaluation."""
        L = np.arange(NS, dtype=np.int64)
        self.apply_word(L, w)
        P = np.empty(NS, np.int64)
        P[L] = np.arange(NS, dtype=np.int64)   # sticker from L[y] ends at y
        return P


def invword(w):
    return [-int(x) for x in reversed(w)]


def sparse(P):
    """Support (s, d) of a dense permutation, s sorted."""
    s = np.flatnonzero(P != np.arange(len(P)))
    return s, P[s]


def dense(s, d, n=NS):
    P = np.arange(n, dtype=np.int64)
    P[s] = d
    return P


def compose(P, Q):
    """Chronological product: P first, then Q."""
    return Q[P]


def inverse(P):
    R = np.empty_like(P)
    R[P] = np.arange(len(P))
    return R


def perm_order_on(P, idx):
    """Order of P restricted to the invariant set idx (lcm of cycle lengths)."""
    from math import lcm
    seen = set()
    o = 1
    for x in idx:
        x = int(x)
        if x in seen:
            continue
        n = 0
        y = x
        while True:
            seen.add(y)
            y = int(P[y])
            n += 1
            if y == x:
                break
        o = lcm(o, n)
    return o
