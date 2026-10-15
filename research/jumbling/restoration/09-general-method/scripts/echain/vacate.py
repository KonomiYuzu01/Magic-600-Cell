"""Greedy SAFE-ALL vacancy search for a target chamber set with few occupied chambers.

At a fixed state, with the SAFE-ALL alphabet G (buffer.Alphabet), a target mask ``bad`` and the hole mask, find a
word V of SAFE-ALL letters after which no lattice piece is in a target chamber: for every occupied chamber z,
pi_V(z) is not a target chamber. Each greedy step lowers the number of occupied target chambers and never raises
it; candidate steps, shortest first, are single letters, commutators [x, y] = x y x^-1 y^-1, and conjugates
z [x, y] z^-1, with x a letter whose cap holds an occupied target chamber. Every step is checked on the full
chamber permutation of its letters. Only the holes and the geometry are read; no simulator state is changed.
"""
import json
import sys
import time

import numpy as np

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


class Vacate:
    def __init__(self, G, bad, holes):
        self.G, self.NP = G, G.NP
        self.bad = np.asarray(bad, bool)
        self.occ = np.ones(G.NP, bool)
        self.occ[np.asarray(holes, np.int64)] = False
        self.ngrip = {}
        self.safe = np.array(G.safe, np.int64)

    def letters_at(self, x):
        G = self.G
        caps = self.safe[G.home[x, self.safe]]
        return [n for c in caps.tolist() for n in G.by_grip[c]]

    def neighbours(self, c):
        """Grips whose SAFE caps share a chamber with cap c."""
        if c not in self.ngrip:
            G = self.G
            self.ngrip[c] = self.safe[G.home[np.ix_(G.capx[c], self.safe)].any(axis=0)].tolist()
        return self.ngrip[c]

    def support(self, word):
        G = self.G
        caps = sorted({G.letters[n][0] for n in word})
        return np.unique(np.concatenate([G.capx[c] for c in caps]))

    def gain(self, word):
        """(occupied target chambers before - after) on the word's support, the support and its images."""
        G = self.G
        U = self.support(word)
        img = G.images(word, U)
        if not np.array_equal(np.sort(img), U):
            raise AssertionError('word does not permute its support')
        oc = self.occ[U]
        before = int((oc & self.bad[U]).sum())
        after = int(self.bad[img[oc]].sum())
        return before - after, U, img

    def apply(self, word):
        _, U, img = self.gain(word)
        new = self.occ.copy()
        new[U] = False
        new[img[self.occ[U]]] = True
        if new.sum() != self.occ.sum():
            raise AssertionError('occupancy count changed')
        self.occ = new

    def step(self, t, max_conj=60):
        """Shortest gaining word around target t: letter, commutator [x, y], or z [x, y] z^-1.

        y ranges over letters whose cap holds t or x(t) (otherwise [x, y] leaves t's content in place)."""
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
        if best is not None:
            return best
        # conjugated commutators z [x, y] z^-1: z^-1 first carries t's content to t' = z^-1(t)
        Z = [z for c in self.neighbours(G.letters[X[0]][0])[:max_conj] for z in G.by_grip[c]] if X else []
        for z in Z:
            zi = int(G.inv[z])
            tz = G.image([zi], t)
            if tz == t:
                continue
            for x in self.letters_at(tz):
                xt = G.image([x], tz)
                for y in sorted(set(self.letters_at(tz)) | set(self.letters_at(xt))):
                    if G.letters[y][0] == G.letters[x][0]:
                        continue
                    w = [zi, x, y, int(G.inv[x]), int(G.inv[y]), z]
                    g, _, _ = self.gain(w)
                    if g > 0:
                        return w, g
        return None, 0

    def run(self, max_steps=500):
        word, steps = [], []
        while True:
            targets = np.flatnonzero(self.occ & self.bad)
            if not len(targets):
                return word, steps, True
            if len(steps) >= max_steps:
                return word, steps, False
            done = False
            for t in targets.tolist():
                w, g = self.step(t)
                if w is not None:
                    self.apply(w)
                    word.extend(w)
                    steps.append({'target': t, 'length': len(w), 'gain': g,
                                  'left': int((self.occ & self.bad).sum())})
                    log(step=len(steps), target=t, length=len(w), gain=g, left=steps[-1]['left'])
                    done = True
                    break
            if not done:
                return word, steps, False
