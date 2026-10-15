"""General block family: carrier-conjugated cores on a virtual state of contents (menu I_a).

A state is held as contents: every piece p has (dom[p], q[p]), the frame domain (index into a shared
fastghost.Frames table) and the chamber in that frame; lattice pieces have dom 0. A letter acts on the contents inside
its cap exactly as on the pieces (fastghost.Ghost: exact signature bits when the pulled-back normal is a pole, exact
Evaluator classification otherwise). So a carrier word V can be applied to the contents alone, and every later test
(cut status of the block, straddled grips, the core's ghost, Bad and its occupancy) is made at V(S) without touching
the simulator. Float arithmetic is used only to select which contents need an exact test; every decision that is
reported is exact. A found word is confirmed by replay on the simulator.

Usage as a library: W = World(table [, blocks_file]); see the methods.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
_TH = None


def setup(theory_dir, echain_dir):
    global _TH
    th = Path(theory_dir).resolve()
    for p in (str(th), str(th.parent), str(Path(echain_dir).resolve())):
        if p not in sys.path:
            sys.path.insert(0, p)
    _TH = th


T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


class World:
    """A pose table loaded once, with exact menus, contents of every piece and float helpers."""

    def __init__(self, table, blocks_file=None):
        import ghostlib as GL
        from exact import matmul, matvec, transpose
        from sim.kplus import to_tuple
        self.GL, self.FG = GL, GL.FG
        self.matmul, self.matvec, self.transpose, self.to_tuple = matmul, matvec, transpose, to_tuple
        ctx = GL.BL.CL.ctx
        self.data, self.kp, self.reg = ctx.data, ctx.kplus, ctx.regions
        self.NP = GL.NP
        self.R, self.M = GL.O.start(table)
        self.st = self.R.st
        self.n_items = len(self.M.items)
        D = GL.rim.Domains(self.st)
        self.F = GL.FG.Frames()
        self.dom = np.zeros(self.NP, np.int64)
        self.q = np.zeros(self.NP, np.int64)
        for j, rep in enumerate(D.reps):
            mem = D.members[j]
            if not len(mem):
                continue
            d2, k = self.F.domain_of(rep)
            self.dom[mem] = d2
            self.q[mem] = GL.FG.chamber_image(k, D.cham[mem])
        self.off = np.flatnonzero(self.dom != 0)
        self.digest = self.st.digest()
        self._el, self._inv, self._kit, self._inK = {}, {}, {}, {}
        self.ident = to_tuple(self.E(0, 0))
        if blocks_file == '-':
            self.blocks = []
        elif blocks_file is not None:
            bl = json.loads(Path(blocks_file).read_text())['blocks']
            offset = np.zeros(self.NP, bool)
            offset[self.off] = True
            self.blocks = [np.asarray(sorted(int(p) for p in b['pieces']), np.int64) for b in bl]
            self.blocks = [b if offset[b].all() else np.zeros(0, np.int64) for b in self.blocks]
        else:
            self.blocks = [np.asarray(b, np.int64) for b in GL.BL.Blocks(self.R, self.M).blocks]
        self.block_of = np.full(self.NP, -1, np.int64)
        for j, b in enumerate(self.blocks):
            self.block_of[b] = j
        self._float_setup()
        log(world=Path(table).name, off=int(len(self.off)), domains=len(self.F.reps), blocks=len(self.blocks))

    # ------------------------------------------------------------------ menu
    def E(self, c, i):
        key = (int(c), int(i))
        if key not in self._el:
            self._el[key] = self.GL.restore.element(self.M, key[0], key[1])
        return self._el[key]

    def inv(self, c, i):
        key = (int(c), int(i))
        if key not in self._inv:
            t = self.to_tuple(self.transpose(self.E(c, i)))
            self._inv[key] = next(j for j in range(self.n_items) if self.to_tuple(self.E(c, j)) == t)
        return self._inv[key]

    def inK(self, g):
        key = self.to_tuple(g)
        if key not in self._inK:
            self._inK[key] = self.kp.index_of_matrix(key) >= 0
        return self._inK[key]

    def kit(self, c):
        """Non-identity K+ letters at grip c."""
        if c not in self._kit:
            self._kit[c] = [i for i in range(self.n_items)
                            if self.inK(self.E(c, i)) and self.to_tuple(self.E(c, i)) != self.ident]
        return self._kit[c]

    def inverse_word(self, w):
        return [(c, self.inv(c, i)) for c, i in reversed(w)]

    # ------------------------------------------------------------------ float helpers (selection only)
    def _float_setup(self):
        reg, kp = self.reg, self.kp
        nv = max(len(v) for v in reg.rep_vertices)
        self.NV = nv
        fl = lambda m: np.array([[float(x) for x in row] for row in m])  # noqa: E731
        self._fl = fl
        self.Kf = np.stack([fl(kp.matrix(k)) for k in range(len(kp.perms))])  # (7200, 4, 4)
        repv = np.full((len(reg.rep_vertices), nv, 4), np.nan)
        for o, vs in enumerate(reg.rep_vertices):
            for j, v in enumerate(vs):
                repv[o, j] = [float(x) for x in v]
        self.repv = repv
        self.NF = np.array([[float(x) for x in self.data.N[c]] for c in range(600)])
        self.KAP = float(self.data.KAPPA)
        self._repf = {}

    def chamber_vertices(self, Q):
        """Float vertices (n, NV, 4) of chambers Q (NaN padded)."""
        Q = np.asarray(Q, np.int64)
        o = self.reg.orbit_of[Q]
        K = self.Kf[self.reg.transport[Q]]
        return np.einsum('nij,nvj->nvi', K, self.repv[o])

    def repf(self, d):
        if d not in self._repf:
            self._repf[d] = self._fl(self.F.reps[d])
        return self._repf[d]

    def content_vertices(self, dom, q):
        out = np.empty((len(dom), self.NV, 4))
        for d in np.unique(dom).tolist():
            idx = np.flatnonzero(dom == d)
            out[idx] = self.chamber_vertices(q[idx]) @ self.repf(d).T
        return out

    def cap_range(self, verts, caps=None):
        """(min, max) over each content's vertices of x.N_c - kappa, for caps (default all): arrays (n, ncaps)."""
        NF = self.NF if caps is None else self.NF[caps]
        val = np.einsum('nvi,ci->nvc', verts, NF) - self.KAP
        return np.nanmin(val, axis=1), np.nanmax(val, axis=1)

    # ------------------------------------------------------------------ exact sides
    def sides(self, dom, q, c):
        """Exact side of each content against cap c: 1 inside, 0 outside, -1 straddle."""
        out = np.empty(len(dom), np.int64)
        for d in np.unique(dom).tolist():
            idx = np.flatnonzero(dom == d)
            e = self.F.pole(d, c)
            if e >= 0:
                out[idx] = self.data.in_sig(q[idx], np.full(len(idx), e, np.int64)).astype(np.int64)
            else:
                out[idx] = self.F.sides(d, c, q[idx])
        return out

    def status(self, dom, q, c):
        """0 outside, 1 inside, 2 some content straddles, 3 contents on both sides (exact)."""
        s = self.sides(dom, q, c)
        if (s < 0).any():
            return 2
        if (s == 1).all():
            return 1
        if (s == 0).all():
            return 0
        return 3

    def statuses(self, dom, q, caps=None, margin=1e-6):
        """Exact status of a content set for every cap in caps (default all 600); float ranges select the caps that
        need an exact test (those not clearly outside)."""
        caps = list(range(600)) if caps is None else list(caps)
        verts = self.content_vertices(dom, q)
        lo, hi = self.cap_range(verts, caps)
        res = {}
        for j, c in enumerate(caps):
            if (hi[:, j] < -margin).all():
                res[c] = 0
            elif (lo[:, j] > margin).all():
                res[c] = 1
            else:
                res[c] = self.status(dom, q, c)
        return res


class VState:
    """Contents of every piece at a state reached from the world's state by a carrier word (virtual)."""

    def __init__(self, W, dom=None, q=None, word=()):
        self.W = W
        self.dom = W.dom.copy() if dom is None else dom
        self.q = W.q.copy() if q is None else q
        self.word = list(word)
        self._occ = None
        self._offv = None
        self._off = None
        self._strad = {}

    @property
    def off(self):
        """Off contents here (pieces whose content is not in the lattice domain)."""
        if self._off is None:
            self._off = np.flatnonzero(self.dom != 0)
        return self._off

    def occ(self):
        """Chambers holding a lattice piece."""
        if self._occ is None:
            lat = np.flatnonzero(self.dom == 0)
            o = np.zeros(self.W.NP, bool)
            o[self.q[lat]] = True
            self._occ = o
        return self._occ

    def apply(self, word):
        """New VState after the word (every letter acting on all contents), or None if some content straddles."""
        W = self.W
        g = W.FG.Ghost(W.F, self.dom, self.q)
        for c, i in word:
            g.apply(int(c), W.E(c, i), (int(c), int(i)))
            if not g.alive.all():
                return None
        return VState(W, g.dom, g.q, self.word + [tuple(x) for x in word])

    def off_vertices(self):
        if self._offv is None:
            off = self.off
            self._offv = self.W.content_vertices(self.dom[off], self.q[off])
        return self._offv

    def straddled(self, c, margin=1e-6):
        """Whether some off content straddles cap c here (exact on the contents the float range cannot exclude)."""
        if c not in self._strad:
            off = self.off
            lo, hi = self.W.cap_range(self.off_vertices(), [c])
            near = np.flatnonzero((lo[:, 0] <= margin) & (hi[:, 0] >= -margin))
            if not len(near):
                self._strad[c] = False
            else:
                s = self.W.sides(self.dom[off[near]], self.q[off[near]], c)
                self._strad[c] = bool((s < 0).any())
        return self._strad[c]

    def straddlers(self, c, margin=1e-6):
        off = self.off
        lo, hi = self.W.cap_range(self.off_vertices(), [c])
        near = np.flatnonzero((lo[:, 0] <= margin) & (hi[:, 0] >= -margin))
        if not len(near):
            return np.zeros(0, np.int64)
        s = self.W.sides(self.dom[off[near]], self.q[off[near]], c)
        return off[near[s < 0]]

    def block_status(self, T, caps=None):
        return self.W.statuses(self.dom[T], self.q[T], caps)

    def movers(self, T, st=None):
        st = self.block_status(T) if st is None else st
        return [c for c, s in st.items() if s == 1 and not self.straddled(c)]

    def domains(self, T):
        return sorted(set(self.dom[T].tolist()))


def ghost_score(vs, word, T, extra=None):
    """Exact ghost of the word at the virtual state vs: T's contents, every other off content, and the lattice
    contents of the chambers in the word's caps. Returns a row (T on K, other off pieces dead or moved, their blocks,
    Bad(word) and its occupancy here)."""
    W = vs.W
    off = vs.off
    T = np.asarray(T, np.int64)
    inT = np.zeros(W.NP, bool)
    inT[T] = True
    prot = off[~inT[off]]
    grips = sorted({int(c) for c, _ in word})
    U = np.flatnonzero(np.logical_or.reduce([W.GL.cap(c) for c in grips]))
    dom0 = np.concatenate([vs.dom[T], vs.dom[prot], np.zeros(len(U), np.int64)])
    q0 = np.concatenate([vs.q[T], vs.q[prot], U])
    g = W.FG.Ghost(W.F, dom0, q0)
    for c, i in word:
        g.apply(int(c), W.E(c, i), (int(c), int(i)))
    nT, nP = len(T), len(T) + len(prot)
    pa = g.alive[nT:nP]
    mv = pa & ((g.dom[nT:nP] != vs.dom[prot]) | (g.q[nT:nP] != vs.q[prot]))
    badp = ~pa | mv
    bad = U[~g.alive[nP:] | (g.dom[nP:] != 0)]
    occ = vs.occ()
    row = {'T': nT, 'T_on': int((g.alive[:nT] & (g.dom[:nT] == 0)).sum()), 'T_dead': int((~g.alive[:nT]).sum()),
           'prot_dead': int((~pa).sum()), 'prot_moved': int(mv.sum()),
           'conflict_blocks': sorted(set(W.block_of[prot[badp]].tolist())),
           'bad': int(len(bad)), 'bad_occupied': int(occ[bad].sum())}
    row['clean'] = row['T_on'] == nT and row['prot_dead'] == 0 and row['prot_moved'] == 0
    row['ok'] = row['clean'] and row['bad_occupied'] == 0
    if extra is not None:
        extra['bad'] = bad
        extra['ghost'] = g
    return row


def core_word(W, e, Bi, c, Xi, a=-1, Ai=-1):
    """D-type Q = A, B, X, B^-1, X^-1, A^-1, X, B, X^-1, B^-1 or C-type [X, B] = X, B, X^-1, B^-1 (a = -1)."""
    J = [(e, Bi), (c, Xi), (e, W.inv(e, Bi)), (c, W.inv(c, Xi))]
    Jinv = [(c, Xi), (e, Bi), (c, W.inv(c, Xi)), (e, W.inv(e, Bi))]
    return Jinv if a < 0 else [(a, Ai)] + J + [(a, W.inv(a, Ai))] + Jinv


def moved_contents(W, dom, q, g):
    """Contents of a content set moved by the left factor g (exact): each domain d goes to (d2, k) with
    reps[d2] k = g reps[d], and the chamber q to k(q)."""
    key = W.to_tuple(g)
    d_out, q_out = dom.copy(), q.copy()
    for d in np.unique(dom).tolist():
        idx = np.flatnonzero(dom == d)
        d2, k = W.F.mul(key, g, d)
        d_out[idx] = d2
        q_out[idx] = W.FG.chamber_image(k, q[idx])
    return d_out, q_out


def core_candidates(vs, T, d_types=True, c_types=False, movers=None, need_pole=False, limit=None):
    """Lemma Q (D-type) and Lemma Q' (C-type) candidates for the contents of T at the virtual state vs.
    Q1: caps e (and a) hold T and are unstraddled here; Q2: A(T) outside caps c and e; Q3: T outside cap c, B(T)
    inside cap c, c unstraddled; Q4: X^-1 B maps every domain of T to the lattice domain."""
    W = vs.W
    T = np.asarray(T, np.int64)
    dT, qT = vs.dom[T], vs.q[T]
    doms = sorted(set(dT.tolist()))
    st0 = W.statuses(dT, qT)
    mv = [c for c, s in st0.items() if s == 1 and not vs.straddled(c)] if movers is None else movers
    out = []
    for e in mv:
        for Bi in W.kit(e):
            Bm = W.E(e, Bi)
            dB, qB = moved_contents(W, dT, qT, Bm)
            stB = W.statuses(dB, qB)
            for c in [cc for cc, s in stB.items() if s == 1 and st0.get(cc) == 0]:
                if vs.straddled(c):
                    continue
                Xs = []
                for Xi in range(W.n_items):
                    Xm = W.E(c, Xi)
                    if W.inK(Xm):
                        continue
                    h = W.matmul(W.transpose(Xm), Bm)
                    hk = W.to_tuple(h)
                    if all(W.F.mul(hk, h, d)[0] == 0 for d in doms):
                        Xs.append(Xi)
                if not Xs:
                    continue
                if c_types:
                    for Xi in Xs:
                        out.append((e, Bi, c, Xi, -1, -1))
                if d_types:
                    for a in mv:
                        if a == e:
                            continue
                        for Ai in W.kit(a):
                            dA, qA = moved_contents(W, dT, qT, W.E(a, Ai))
                            if W.status(dA, qA, c) != 0 or W.status(dA, qA, e) != 0:
                                continue
                            for Xi in Xs:
                                out.append((e, Bi, c, Xi, a, Ai))
                if limit is not None and len(out) >= limit:
                    return out, mv
    return out, mv
