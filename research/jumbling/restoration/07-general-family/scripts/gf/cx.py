"""Cap commutator core [X, V] = X V X^-1 V^-1 (Lemma CX).

Cap c unstraddled; T = the off contents inside cap c that the letter X at c sends to K+ (any number of blocks); L =
everything else inside cap c (lattice pieces and off contents of other frames). X puts T on the lattice and moves L
to X(L). V is a word of K+ letters, each at a grip unstraddled when applied, that
  (1) moves every piece of T (now lattice pieces) out of cap c,
  (2) moves no content of X(L) (no content of X(L) inside the cap of any letter of V), and
  (3) moves no other content into cap c or across its cut.
Then X^-1 is admissible, sends X(L) back to L exactly and does not touch T; V^-1 brings T's pieces back into cap c
(they stay on the lattice: K+ letters) and every content V carried back to where it was. Net: T on the lattice,
every other piece unchanged except lattice pieces moved inside the lattice. No Bad set: nothing else ever leaves
the lattice for good.

The word is checked by the exact frame ghost of all contents (VState.apply) before it is executed.
"""
import numpy as np

import glib


class Evacuator:
    def __init__(self, W, clearer):
        self.W = W
        self.C = clearer  # for kidx / capm / permute_occ

    def inside_strict(self, vs, c, contents):
        """Exact side of each content against cap c (1 in, 0 out, -1 straddle)."""
        return self.W.sides(vs.dom[contents], vs.q[contents], c)

    def plan(self, vs, c, X, depth=8, beam=4, expand=14, log=None):
        """Search V for the cap commutator at cap c with letter X. Returns (word, T) or None."""
        W, C = self.W, self.C
        off = vs.off
        lo, hi = W.cap_range(vs.off_vertices(), [c])
        near = np.flatnonzero(hi[:, 0] >= -1e-6)
        s = W.sides(vs.dom[off[near]], vs.q[off[near]], c) if len(near) else np.zeros(0, np.int64)
        if (s < 0).any():
            return None
        inside = off[near[s == 1]]
        if not len(inside):
            return None
        Xm = W.E(c, X)
        key = W.to_tuple(Xm)
        dmap = {d: W.F.mul(key, Xm, d)[0] for d in sorted(set(vs.dom[inside].tolist()))}
        T = inside[np.array([dmap[int(d)] == 0 for d in vs.dom[inside]], bool)]
        if not len(T):
            return None
        vs1 = vs.apply([(c, X)])
        if vs1 is None:
            return None
        XL_off = np.setdiff1d(vs1.off, off[~np.isin(off, inside)])  # off contents inside cap c after X
        capc = C.capm(c)

        def score(vsx):
            tin = int(capc[vsx.q[T]].sum())
            # lattice pieces other than T's inside cap c (X^-1 would take them off the lattice)
            entered = int((capc & vsx.occ()).sum()) - tin
            return tin, entered

        def usable(vsx):
            out = []
            Tch = np.zeros(W.NP, bool)
            Tch[vsx.q[T]] = True
            Tch &= capc
            if not Tch.any():
                return out
            vXL = W.content_vertices(vsx.dom[XL_off], vsx.q[XL_off]) if len(XL_off) else None
            rng = W.cap_range(vXL) if vXL is not None else None
            for c2 in range(600):
                if c2 == c or not (C.capm(c2) & Tch).any():
                    continue
                if vsx.straddled(c2):
                    continue
                if rng is not None and (rng[1][:, c2] >= -1e-6).any():
                    sx = W.sides(vsx.dom[XL_off], vsx.q[XL_off], c2)
                    if (sx != 0).any():
                        continue
                out.append(c2)
            return out

        states = [(score(vs1), [], vs1)]
        seen = set()
        for d in range(depth):
            nxt = []
            for sc0, V, vsx in states:
                cands = []
                for c2 in usable(vsx):
                    m = C.capm(c2)
                    for Y in W.kit(c2):
                        k = C.kidx(c2, Y)
                        qT = vsx.q[T].copy()
                        inm = m[qT]
                        qT[inm] = W.FG.chamber_image(k, qT[inm])
                        cands.append((int(capc[qT].sum()), c2, Y))
                cands.sort()
                for tin, c2, Y in cands[:expand]:
                    if tin >= sc0[0] and d > 0:
                        break
                    key2 = tuple(V + [(c2, Y)])
                    if key2 in seen:
                        continue
                    seen.add(key2)
                    vs2 = vsx.apply([(c2, Y)])
                    if vs2 is None:
                        continue
                    sc2 = score(vs2)
                    if sc2[0] == 0:
                        word = [(c, X)] + V + [(c2, Y)] + [(c, W.inv(c, X))] + W.inverse_word(V + [(c2, Y)])
                        ok = self.check(vs, word, T)
                        if log is not None:
                            log(event='cx_try', cap=c, X=X, T=int(len(T)), V=len(V) + 1, entered=sc2[1], ok=ok)
                        if ok:
                            return word, T
                        continue
                    nxt.append((sc2, V + [(c2, Y)], vs2))
            if not nxt:
                return None
            nxt.sort(key=lambda t: (t[0][0] + 4 * t[0][1], len(t[1])))
            states = nxt[:beam]
            if log is not None:
                log(event='cx_level', cap=c, X=X, depth=d + 1, T_in_cap=states[0][0][0], entered=states[0][0][1])
        return None

    def check(self, vs, word, T):
        """Exact ghost of the word on all contents: T on K, every other off content unchanged, lattice on K."""
        end = vs.apply(word)
        if end is None:
            return False
        inT = np.zeros(self.W.NP, bool)
        inT[T] = True
        if (end.dom[T] != 0).any():
            return False
        oth = vs.off[~inT[vs.off]]
        if (end.dom[oth] != vs.dom[oth]).any() or (end.q[oth] != vs.q[oth]).any():
            return False
        lat = np.flatnonzero(vs.dom == 0)
        return bool((end.dom[lat] == 0).all())
