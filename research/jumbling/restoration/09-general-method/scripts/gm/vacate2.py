"""Two-pass greedy SAFE-ALL vacancy search (a reordering of vacate.py; same moves, same checks).

vacate.Vacate.run takes the occupied target chambers in order and, for each, tries a letter, then the commutators
[x, y], then the conjugates z [x, y] z^-1, before moving to the next chamber; one chamber without a short gaining word
costs the full conjugate scan. Here every step first tries the letters and commutators of all occupied target
chambers, and only then the conjugates. Candidate gain evaluation is count-only: it applies the inverse word to
the target chambers in its support. The apply path still checks every applied word on the full chamber permutation
of its letters through vacate.Vacate.gain, so the result has the same guarantees.
"""
import numpy as np

import vacate as VA


class Vacate2(VA.Vacate):
    def __init__(self, G, bad, holes):
        super().__init__(G, bad, holes)
        self._targets = np.flatnonzero(self.bad)
        # Geometry and the target set stay fixed; occupancy is always read afresh.
        self._target_caps = {c: G.home[self._targets, c] for c in G.safe}

    def gain(self, word):
        """Exact gain on target preimages; support and image arrays are not returned."""
        G = self.G
        inside = np.zeros(len(self._targets), bool)
        for c in {G.letters[n][0] for n in word}:
            inside |= self._target_caps[c]
        targets = self._targets[inside]
        preimages = G.images([int(G.inv[n]) for n in reversed(word)], targets)
        # SAFE-ALL words permute their support U, so counting occupied preimages
        # of bad intersect U equals counting occupied sources sent into bad.
        return int(self.occ[targets].sum()) - int(self.occ[preimages].sum()), None, None

    def apply(self, word):
        # Bypass count-only gain and any instance deadline wrapper for this check.
        _, U, img = VA.Vacate.gain(self, word)
        new = self.occ.copy()
        new[U] = False
        new[img[self.occ[U]]] = True
        if new.sum() != self.occ.sum():
            raise AssertionError('occupancy count changed')
        self.occ = new

    def cheap(self, t):
        """A gaining letter at t, else the best gaining commutator [x, y] around t, else (None, 0)."""
        G = self.G
        X = self.letters_at(t)
        for x in X:
            g, _, _ = self.gain([x])
            if g > 0:
                return [x], g
        best = None
        for x in X:
            xt = G.image([x], t)
            Y = sorted(set(self.letters_at(t)) | set(self.letters_at(xt)))
            for y in Y:
                if G.letters[y][0] == G.letters[x][0]:
                    continue
                for w in ([x, y, int(G.inv[x]), int(G.inv[y])], [y, x, int(G.inv[y]), int(G.inv[x])]):
                    g, _, _ = self.gain(w)
                    if g > 0 and (best is None or g > best[1]):
                        best = (w, g)
        return best if best is not None else (None, 0)

    def run(self, max_steps=500):
        word, steps = [], []
        while True:
            targets = np.flatnonzero(self.occ & self.bad).tolist()
            if not targets:
                return word, steps, True
            if len(steps) >= max_steps:
                return word, steps, False
            found = None
            for t in targets:
                w, g = self.cheap(t)
                if w is not None:
                    found = (t, w, g)
                    break
            if found is None:
                for t in targets:
                    w, g = self.step(t)
                    if w is not None:
                        found = (t, w, g)
                        break
            if found is None:
                return word, steps, False
            t, w, g = found
            self.apply(w)
            word.extend(w)
            steps.append({'target': t, 'length': len(w), 'gain': g, 'left': int((self.occ & self.bad).sum())})
            VA.log(step=len(steps), target=t, length=len(w), gain=g, left=steps[-1]['left'])
