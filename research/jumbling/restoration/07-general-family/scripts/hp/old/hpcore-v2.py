"""Exact contents of a jumbled state with incremental single-letter application and per-cap rows.

A state is held as contents (dom[p], q[p]) for every piece p (glib.World conventions: dom 0 is the lattice, the region
of p is reps[dom](chamber q)). Float vertices are kept for the off pieces only and serve to select which contents get
an exact test; every decision (inside, outside, straddle, admissibility, landing frame and chamber) is exact
(fastghost.Frames: signature bits when the pulled-back normal is a pole, the exact evaluator otherwise).

Row of cap c at a state: straddlers (number of off contents straddling cap c), inside (off contents inside cap c),
occ (lattice pieces inside cap c), frames {domain: count} of the inside contents.
"""
import numpy as np

import glib

EPS = 1e-6


class Geo:
    """Cap masks, chamber lists and the cap-overlap relation (exact: caps are unions of chambers)."""

    def __init__(self, W):
        self.W = W
        self.capm = np.stack([W.GL.cap(c) for c in range(600)])
        self.cham = [np.flatnonzero(self.capm[c]) for c in range(600)]
        pairs_c = np.concatenate([np.full(len(z), c, np.int64) for c, z in enumerate(self.cham)])
        pairs_z = np.concatenate(self.cham)
        order = np.argsort(pairs_z, kind='stable')
        self._zc = pairs_c[order]
        self._zs = np.searchsorted(pairs_z[order], np.arange(W.NP + 1))
        self._ov = {}
        self._let = {}

    def caps_of(self, Z):
        Z = np.unique(np.asarray(Z, np.int64))
        if not len(Z):
            return np.zeros(0, np.int64)
        parts = [self._zc[self._zs[z]:self._zs[z + 1]] for z in Z.tolist()]
        return np.unique(np.concatenate(parts))

    def overlap(self, c):
        if c not in self._ov:
            self._ov[c] = self.caps_of(self.cham[c])
        return self._ov[c]

    def letters(self, c, d):
        """Letters at grip c that put frame d on K+ (exact)."""
        key = (int(c), int(d))
        if key not in self._let:
            W = self.W
            self._let[key] = [i for i in range(W.n_items)
                              if W.F.mul(W.to_tuple(W.E(c, i)), W.E(c, i), d)[0] == 0]
        return self._let[key]


class HState:
    def __init__(self, W, G, dom, q, verts):
        self.W, self.G = W, G
        self.dom, self.q, self.verts = dom, q, verts
        self._off = None
        self._occ = None
        self._offv = None

    @classmethod
    def from_world(cls, W, G):
        dom, q = W.dom.copy(), W.q.copy()
        verts = np.full((W.NP, W.NV, 4), np.nan)
        off = np.flatnonzero(dom != 0)
        if len(off):
            verts[off] = W.content_vertices(dom[off], q[off])
        return cls(W, G, dom, q, verts)

    @property
    def off(self):
        if self._off is None:
            self._off = np.flatnonzero(self.dom != 0)
        return self._off

    def occ(self):
        if self._occ is None:
            o = np.zeros(self.W.NP, bool)
            lat = np.flatnonzero(self.dom == 0)
            o[self.q[lat]] = True
            self._occ = o
        return self._occ

    def offv(self):
        if self._offv is None:
            self._offv = self.verts[self.off]
        return self._offv

    def near(self, c):
        """Off contents not clearly outside cap c (float selection), as indices into self.off."""
        val = self.offv() @ self.W.NF[c] - self.W.KAP
        hi = np.nanmax(val, axis=1)
        return np.flatnonzero(hi >= -EPS)

    def row(self, c):
        W = self.W
        off = self.off
        nr = self.near(c)
        s = W.sides(self.dom[off[nr]], self.q[off[nr]], c) if len(nr) else np.zeros(0, np.int64)
        inside = off[nr[s == 1]]
        doms, cnt = np.unique(self.dom[inside], return_counts=True)
        return {'cap': int(c), 'strad': int((s < 0).sum()), 'inside': inside,
                'occ': int((self.G.capm[c] & self.occ()).sum()),
                'frames': {int(d): int(k) for d, k in zip(doms, cnt)}}

    def apply(self, word):
        """New state after the word (exact), or None if some content straddles a letter's cap. The new state carries
        `moved` (every piece some letter moved) and `grips` (the word's grips)."""
        st = self
        moved = []
        for c, X in word:
            st = st._apply1(int(c), int(X))
            if st is None:
                return None
            moved.append(st._mv)
        st.moved = np.unique(np.concatenate(moved)) if moved else np.zeros(0, np.int64)
        st.grips = sorted({int(c) for c, _ in word})
        return st

    def _apply1(self, c, X):
        W, G = self.W, self.G
        off = self.off
        nr = self.near(c)
        s = W.sides(self.dom[off[nr]], self.q[off[nr]], c) if len(nr) else np.zeros(0, np.int64)
        if (s < 0).any():
            return None
        mv_off = off[nr[s == 1]]
        lat = np.flatnonzero(self.dom == 0)
        mv_lat = lat[G.capm[c][self.q[lat]]]
        mv = np.concatenate([mv_off, mv_lat])
        dom, q, verts = self.dom.copy(), self.q.copy(), self.verts.copy()
        if len(mv):
            d2, q2 = glib.moved_contents(W, dom[mv], q[mv], W.E(c, X))
            dom[mv], q[mv] = d2, q2
            now_off = mv[d2 != 0]
            if len(now_off):
                verts[now_off] = W.content_vertices(dom[now_off], q[now_off])
            verts[mv[d2 == 0]] = np.nan
        out = HState(W, G, dom, q, verts)
        out._mv = mv
        return out


def gains(row, G):
    """Single-letter candidates of a row: (gain, cap, frame, n_frame, occ) for each frame with a restoring letter."""
    out = []
    if row['strad'] or not row['frames']:
        return out
    c = row['cap']
    for d, n in row['frames'].items():
        if G.letters(c, d):
            out.append((n - row['occ'], c, d, n, row['occ']))
    return out


class Table:
    """Rows of all 600 caps at a state, recomputed only for caps that a step may change."""

    def __init__(self, st, rows=None):
        self.st = st
        self.rows = rows if rows is not None else [st.row(c) for c in range(600)]

    def affected(self, st2):
        """Caps whose row may differ between this state and st2 = this state after a word: caps sharing a chamber
        with a grip of the word, and caps that some moved content (before or after) is not clearly outside of."""
        G, W = st2.G, st2.W
        aff = set(G.caps_of(np.concatenate([G.cham[c] for c in st2.grips])).tolist())
        mv = st2.moved
        for st, sel in ((self.st, mv[self.st.dom[mv] != 0]), (st2, mv[st2.dom[mv] != 0])):
            if len(sel):
                val = np.einsum('nvi,ci->nvc', st.verts[sel], W.NF) - W.KAP
                hi = np.nanmax(val, axis=1)
                aff.update(np.flatnonzero((hi >= -EPS).any(axis=0)).tolist())
        return sorted(aff)

    def after(self, st2):
        rows = list(self.rows)
        aff = self.affected(st2)
        for c in aff:
            rows[c] = st2.row(c)
        t = Table(st2, rows)
        t.aff = aff
        return t

    def local_best(self, caps):
        """(best gain, caps with a positive gain) among the given caps."""
        best, npos = None, 0
        for c in caps:
            g = gains(self.rows[c], self.st.G)
            if g:
                m = max(x[0] for x in g)
                best = m if best is None else max(best, m)
                npos += m > 0
        return best, npos

    def candidates(self):
        out = []
        for r in self.rows:
            out.extend(gains(r, self.st.G))
        out.sort(key=lambda t: (-t[0], t[4], t[1], t[2]))
        return out

    def best_gain(self):
        cs = self.candidates()
        return cs[0][0] if cs else None
