"""Clearing carriers for same-frame caps (Lemma V' with an S-type core).

A cap c whose off contents are all sent to K+ by one letter X, and which no off region straddles, but which holds
lattice pieces: find a word V of K+ letters, each at a grip that no off region straddles when it is applied, such that
at V(S) cap c holds no lattice piece, is unstraddled, and its off contents are still all sent to K+ by one letter.
Then W = V (c, X) V^-1 restores those contents and changes nothing else (Lemma V').

Search: beam search over K+ letters at grips whose cap meets the occupied chambers of cap c; a candidate letter is
ranked by the occupied chambers of cap c after it (exact chamber permutation of the letter), and the best ones are
applied to all contents (exact frame ghost) and checked.
"""
import numpy as np

import glib


class Clearer:
    def __init__(self, W, letters_for):
        self.W = W
        self.letters_for = letters_for
        self._kidx = {}
        self._capm = {}

    def kidx(self, c, i):
        key = (c, i)
        if key not in self._kidx:
            self._kidx[key] = int(self.W.kp.index_of_matrix(self.W.to_tuple(self.W.E(c, i))))
        return self._kidx[key]

    def capm(self, c):
        if c not in self._capm:
            self._capm[c] = self.W.GL.cap(c)
        return self._capm[c]

    def permute_occ(self, occ, c2, Y):
        """Occupancy after the K+ letter (c2, Y): the lattice piece in z (in cap c2) moves to k(z)."""
        m = self.capm(c2)
        z = np.flatnonzero(m)
        z2 = self.W.FG.chamber_image(self.kidx(c2, Y), z)
        out = occ.copy()
        out[z] = False
        out[z2] = occ[z]
        return out

    def target_ok(self, vs, c):
        """(ok, inside, letters, occupied) for cap c at vs."""
        W = self.W
        off = vs.off
        nocc = int((self.capm(c) & vs.occ()).sum())
        lo, hi = W.cap_range(vs.off_vertices(), [c])
        near = np.flatnonzero(hi[:, 0] >= -1e-6)
        s = W.sides(vs.dom[off[near]], vs.q[off[near]], c) if len(near) else np.zeros(0, np.int64)
        if (s < 0).any():
            return False, None, [], nocc
        inside = off[near[s == 1]]
        lets = self.letters_for(c, sorted(set(vs.dom[inside].tolist()))) if len(inside) else []
        return bool(lets) and len(inside) > 0, inside, lets, nocc

    def usable(self, vs, c, U):
        """Grips c2 != c, unstraddled at vs, whose cap contains no content of U and meets an occupied chamber of
        cap c."""
        W = self.W
        occ = vs.occ()
        occ_c = self.capm(c) & occ
        vU = W.content_vertices(vs.dom[U], vs.q[U])
        lo, hi = W.cap_range(vU)
        out = []
        for c2 in range(600):
            if c2 == c or not (self.capm(c2) & occ_c).any():
                continue
            if (hi[:, c2] >= -1e-6).any():  # some content of U not clearly outside: check exactly
                s = W.sides(vs.dom[U], vs.q[U], c2)
                if (s != 0).any():
                    continue
            if vs.straddled(c2):
                continue
            out.append(c2)
        return out

    def search(self, vs0, c, depth=8, beam=4, expand=12, log=None):
        """Beam search for V. Returns (V, vs at V(S), inside, letters) or None."""
        ok, U, lets, nocc = self.target_ok(vs0, c)
        if U is None or not len(U):
            return None
        if ok and nocc == 0:
            return [], vs0, U, lets
        states = [(nocc, [], vs0)]
        seen = set()
        for d in range(depth):
            nxt = []
            for _, V, vs in states:
                ok, U, lets, nocc = self.target_ok(vs, c)
                cands = []
                occ = vs.occ()
                for c2 in self.usable(vs, c, U):
                    for Y in self.W.kit(c2):
                        o2 = self.permute_occ(occ, c2, Y)
                        cands.append((int((self.capm(c) & o2).sum()), c2, Y))
                cands.sort()
                for sc, c2, Y in cands[:expand]:
                    if sc >= nocc:
                        break
                    key = tuple(V + [(c2, Y)])
                    if key in seen:
                        continue
                    seen.add(key)
                    vs2 = vs.apply([(c2, Y)])
                    if vs2 is None:
                        continue
                    ok2, U2, lets2, nocc2 = self.target_ok(vs2, c)
                    if U2 is None:
                        continue
                    if ok2 and nocc2 == 0:
                        return V + [(c2, Y)], vs2, U2, lets2
                    if ok2:
                        nxt.append((nocc2, V + [(c2, Y)], vs2))
            if not nxt:
                return None
            nxt.sort(key=lambda t: t[0])
            states = nxt[:beam]
            if log is not None:
                log(event='clear_level', cap=c, depth=d + 1, best=states[0][0], word=len(states[0][1]))
        return None
