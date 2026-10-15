"""Exact frame ghost: follow chosen contents through a word without the simulator's full state.

A content is a region that some piece occupies: a chamber q of the tiling in the frame of a domain gamma K+ (the region
gamma(q)). Each content carries (domain, chamber in the domain's frame, alive). A twist (c, h) decides, for each domain
present, whether its contents are inside cap c:
- if gamma^-1 n_c is a pole e (rim.py, restoration.md section 2), H_c is the lattice cut H_e in gamma's frame: no
  content straddles, and q is inside iff e is in q's signature (exact signature bit);
- otherwise each content is classified exactly against H_c (sim.kernel.Evaluator), and a straddling content dies
  (the twist would be blocked by it).
Inside contents move to the frame h gamma = gamma' k (gamma' a stored domain representative, k in K+) and to the
chamber k(q). Every decision is exact (Q(sqrt5) arithmetic, signature bits).

Contents are independent: a twist is admissible for a configuration iff no content present in it straddles
(Proposition S). So the ghost of a chamber set gives, for every chamber z, whether a lattice piece in z would follow a
clean path and where (and in which domain) it would end; the real off pieces are followed as their own contents.
"""
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve() if __name__ == '__main__' else None

import sim  # noqa: E402
from exact import identity, matmul, matvec, transpose  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

ctx = sim.get_context()
kp, reg, data = ctx.kplus, ctx.regions, ctx.data
IDK = kp.index_of_matrix(to_tuple(identity(4)))


def chamber_image(k, Q):
    Q = np.asarray(Q, np.int64)
    if not len(Q):
        return Q
    return reg.piece_of[reg.orbit_of[Q], kp.compose_vec(np.full(len(Q), k, np.int64), reg.transport[Q])]


class Frames:
    """Domain representatives (exact matrices) with coset and pole lookups, shared by many ghosts."""

    def __init__(self):
        self.reps = [identity(4)]
        self._pole, self._mul, self._side = {}, {}, {}

    def domain_of(self, G):
        for j, R in enumerate(self.reps):
            kk = kp.index_of_matrix(to_tuple(matmul(transpose(R), G)))
            if kk >= 0:
                return j, kk
        self.reps.append(G)
        return len(self.reps) - 1, IDK

    def pole(self, d, c):
        key = (d, c)
        if key not in self._pole:
            self._pole[key] = data.POLE.get(tuple(matvec(transpose(self.reps[d]), data.N[c])), -1)
        return self._pole[key]

    def mul(self, hkey, h, d):
        """(d2, k) with reps[d2] k = h reps[d]."""
        key = (hkey, d)
        if key not in self._mul:
            self._mul[key] = self.domain_of(matmul(h, self.reps[d]))
        return self._mul[key]

    def sides(self, d, c, Q):
        """Exact side of the regions reps[d](q) against H_c: 1 in, 0 out, -1 straddle (cached)."""
        cache = self._side.setdefault((d, c), {})
        need = [int(q) for q in Q if int(q) not in cache]
        if need:
            ev = Evaluator(pullback_form(self.reps[d], data.N[c]))
            for q in need:
                r = ev.classify(reg.vform(q))[0]
                cache[q] = -1 if r == 'straddle' else (1 if r == 'in' else 0)
        return np.array([cache[int(q)] for q in Q], np.int64)


class Ghost:
    def __init__(self, F, dom, cham):
        self.F = F
        self.dom = np.asarray(dom, np.int64).copy()
        self.q = np.asarray(cham, np.int64).copy()
        self.alive = np.ones(len(self.dom), bool)
        self.died_at = np.full(len(self.dom), -1, np.int64)
        self.step = 0

    def apply(self, c, h, hkey):
        F = self.F
        dom0, alive0 = self.dom.copy(), self.alive.copy()
        for d in np.unique(dom0[alive0]).tolist():
            idx = np.flatnonzero(alive0 & (dom0 == d))
            e = F.pole(d, c)
            if e >= 0:
                ins = data.in_sig(self.q[idx], np.full(len(idx), e, np.int64))
            else:
                s = F.sides(d, c, self.q[idx])
                dead = s < 0
                if dead.any():
                    self.alive[idx[dead]] = False
                    self.died_at[idx[dead]] = self.step
                ins = s == 1
            mv = idx[ins]
            if len(mv):
                d2, k = F.mul(hkey, h, d)
                self.dom[mv] = d2
                self.q[mv] = chamber_image(k, self.q[mv])
        self.step += 1
