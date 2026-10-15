"""Shared helpers: load a pose table into frame-ghost contents, and ghost a word over chosen chambers."""
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
import blocks as BL  # noqa: E402
import fastghost as FG  # noqa: E402
import rim  # noqa: E402

O, restore, data = BL.O, BL.restore, BL.CL.ctx.data
NP = len(data.sigbits)
_CAP = {}


def cap(c):
    if c not in _CAP:
        _CAP[c] = data.in_sig(np.arange(NP), np.full(NP, c, np.int64))
    return _CAP[c]


class Ctx:
    def __init__(self, table):
        self.R, self.M = O.start(table)
        self.st = self.R.st
        self.bl = BL.Blocks(self.R, self.M)
        self.D = rim.Domains(self.st)
        self.F = FG.Frames()
        self.dmap = {j: self.F.domain_of(rep) for j, rep in enumerate(self.D.reps)}
        lat = np.flatnonzero(self.D.dom == 0)
        self.occ = np.zeros(NP, bool)
        self.occ[self.D.cham[lat]] = True
        self.off = np.flatnonzero(self.st._kpose()[self.st.pose_id] < 0)
        self._el = {}

    def E(self, c, i):
        if (c, i) not in self._el:
            self._el[(c, i)] = restore.element(self.M, c, i)
        return self._el[(c, i)]

    def contents(self, pieces):
        dom, q = [], []
        for p in np.asarray(pieces).tolist():
            d2, k = self.dmap[int(self.D.dom[p])]
            dom.append(d2)
            q.append(int(FG.chamber_image(k, np.array([self.D.cham[p]]))[0]))
        return np.array(dom, np.int64), np.array(q, np.int64)

    def block(self, size):
        return np.asarray(next(p for p in self.bl.blocks if len(p) == size), np.int64)

    def ghost(self, word, pieces=(), chambers=None):
        """Ghost of the off pieces `pieces` and of lattice contents in `chambers` (default: union of the word's
        caps). Returns (ghost, n_pieces, chambers)."""
        pieces = np.asarray(pieces, np.int64)
        if chambers is None:
            grips = sorted({c for c, _ in word})
            chambers = np.flatnonzero(np.logical_or.reduce([cap(c) for c in grips]))
        pd, pq = self.contents(pieces) if len(pieces) else (np.zeros(0, np.int64), np.zeros(0, np.int64))
        g = FG.Ghost(self.F, np.concatenate([pd, np.zeros(len(chambers), np.int64)]),
                     np.concatenate([pq, chambers]))
        for c, i in word:
            g.apply(c, self.E(c, i), (c, i))
        return g, len(pieces), chambers

    def bad(self, word, chambers=None):
        """Chambers whose lattice content would straddle or end off the lattice under the word."""
        g, n, ch = self.ghost(word, (), chambers)
        bad = ~g.alive | (g.dom != 0)
        return ch[bad], g, ch
