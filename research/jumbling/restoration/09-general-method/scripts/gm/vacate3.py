"""Batched, exact two-pass SAFE-ALL vacancy search, in Vacate2's candidate order.

For a word w and target set T, gain is sum(occ[T]) - sum(occ[w^-1(T)]).
Targets outside the union of w's caps are fixed and contribute zero. A batch
therefore packs only (candidate, target) pairs inside that union, applies each
candidate's inverse letters, and sums the integer occupancy differences by row.
Lazy cap-position rows and concatenated letter images replace searchsorted and
per-word NumPy calls. Batches contain at most 1,024 words, scaled down to a
262,144-pair membership budget (with a minimum of one word). A batch first
removes targets outside every candidate's caps, then packs each word's support;
both restrictions remove only fixed points whose contribution is zero.

For a conjugate w = z^-1 c z, first map T to Tz = z^-1(T), once per z.
Only Tz inside c's caps is needed: outside them c^-1 fixes the point and the
outer letters cancel. Its contribution is occ[T] - occ[z(c^-1(Tz))]. This is
an exact support restriction, including targets outside every candidate's caps.

The first pass chooses the first gaining letter, otherwise the first maximally
gaining commutator, at the first gaining target. The second pass goes directly
to conjugates: Vacate.step would retry the same letters and commutators that
failed in the first pass, at an unchanged occupancy, so all still gain nothing.
Conjugates retain their sequential order and the first positive gain wins.
Every chosen word uses inherited Vacate2.apply, including its full permutation
check; step rows and VA.log calls are unchanged. No geometry cache holds occ.
"""
from itertools import islice
import time

import numpy as np

import vacate as VA
from vacate2 import Vacate2


class Vacate3(Vacate2):
    BATCH_WORDS = 1024
    BATCH_PAIRS = 262_144

    def __init__(self, G, bad, holes, deadline=None):
        super().__init__(G, bad, holes)
        self.deadline = deadline
        self._identity = len(G.letters)
        cap_index = {c: i for i, c in enumerate(G.safe)}
        self._caps = np.array([cap_index[l[0]] for l in G.letters] + [len(G.safe)], np.intp)
        self._inverse = np.append(G.inv, self._identity)
        lengths = np.array([len(l[3]) for l in G.letters], np.intp)
        self._offsets = np.concatenate(([0], np.cumsum(lengths)))
        self._positions = self._images = None
        self._ready_caps, self._ready_letters = set(), set()
        self._target_rows = self._target_membership(self._targets)

    def _check_deadline(self):
        if self.deadline is not None and time.time() > self.deadline:
            raise TimeoutError

    def _target_membership(self, targets):
        rows = np.zeros((len(self.safe) + 1, len(targets)), bool)
        rows[:-1] = self.G.home[np.ix_(targets, self.safe)].T
        return rows

    def _prepare(self, letters):
        """Allocate lookup storage on demand; fill only used caps and letters."""
        if self._positions is None:
            dtype = np.int32 if self.NP <= np.iinfo(np.int32).max else np.int64
            self._positions = np.empty((len(self.safe) + 1, self.NP), dtype)
            self._positions[-1].fill(-1)  # Padding letters act as the identity.
            self._images = np.empty(int(self._offsets[-1]) + 1, dtype)
            self._images[-1] = 0
        for n in np.unique(letters).tolist():
            if n == self._identity or n in self._ready_letters:
                continue
            self._check_deadline()
            c, _, _, images = self.G.letters[n]
            if c not in self._ready_caps:
                xs = self.G.capx[c]
                row = self._positions[self._caps[n]]
                row.fill(-1)
                row[xs] = np.arange(len(xs), dtype=row.dtype)
                self._ready_caps.add(c)
            self._images[self._offsets[n]:self._offsets[n + 1]] = images
            self._ready_letters.add(n)

    def _map(self, letters, points):
        """Map flat points with one (possibly different) letter per point."""
        positions = self._positions[self._caps[letters], points]
        inside = positions >= 0
        points[inside] = self._images[self._offsets[letters[inside]] + positions[inside]]

    def _batches(self, words, target_count):
        # Bounding the dense membership work also bounds packed mapping work,
        # even for unusually large target masks. No candidate is discarded.
        size = min(self.BATCH_WORDS, max(1, self.BATCH_PAIRS // max(1, target_count)))
        words = iter(words)
        while True:
            self._check_deadline()
            batch = list(islice(words, size))
            if not batch:
                return
            yield batch

    def _gains(self, words, targets, membership, before, endpoint=None):
        self._check_deadline()
        count = len(words)
        lengths = [len(w) for w in words]
        width = max(lengths, default=0)
        if not width or not len(targets):
            return np.zeros(count, np.int64)
        if min(lengths) == width:
            inverse = self._inverse[np.asarray(words, np.intp)[:, ::-1]]
        else:
            inverse = np.full((count, width), self._identity, np.intp)
            for row, word in enumerate(words):
                inverse[row, :len(word)] = self._inverse[word[::-1]]
        caps = self._caps[inverse]
        needed = membership[np.unique(caps)].any(axis=0)
        if not needed.any():
            return np.zeros(count, np.int64)
        targets, before = targets[needed], before[needed]
        membership = membership[:, needed]
        self._prepare(inverse)
        if endpoint is not None:
            self._prepare([endpoint])
        inside = np.zeros((count, len(targets)), bool)
        seen = []
        for column in caps.T:
            # Inverse commutators use each cap twice. Arbitrary mixed words
            # work too: equal columns contribute the same membership mask.
            if not any(np.array_equal(column, previous) for previous in seen):
                inside |= membership[column]
                seen.append(column)
        rows, columns = np.nonzero(inside)
        points = targets[columns].copy()
        for column in inverse.T:
            self._map(column[rows], points)
        if endpoint is not None:
            self._map(np.full(len(points), endpoint, np.intp), points)
        differences = before[columns].astype(np.int8) - self.occ[points]
        # Integer counts, without floating point weights or empty-row issues.
        return (np.bincount(rows[differences > 0], minlength=count)
                - np.bincount(rows[differences < 0], minlength=count))

    def gains(self, words):
        """Exact integer gains for an iterable of words, including mixed lengths."""
        before = self.occ[self._targets]
        chunks = [self._gains(batch, self._targets, self._target_rows, before)
                  for batch in self._batches(words, len(self._targets))]
        return np.concatenate(chunks) if chunks else np.zeros(0, np.int64)

    def _commutators(self, t, both):
        G = self.G
        X = self.letters_at(t)
        at_t = set(X)
        for x in X:
            xt = G.image([x], t)
            for y in sorted(at_t | set(self.letters_at(xt))):
                if G.letters[y][0] == G.letters[x][0]:
                    continue
                xi, yi = int(G.inv[x]), int(G.inv[y])
                yield [x, y, xi, yi]
                if both:
                    yield [y, x, yi, xi]

    def cheap(self, t):
        self._check_deadline()
        before = self.occ[self._targets]
        for batch in self._batches(([x] for x in self.letters_at(t)), len(self._targets)):
            gains = self._gains(batch, self._targets, self._target_rows, before)
            positive = np.flatnonzero(gains > 0)
            if len(positive):
                i = int(positive[0])
                return batch[i], int(gains[i])
        best, best_gain = None, 0
        for batch in self._batches(self._commutators(t, both=True), len(self._targets)):
            gains = self._gains(batch, self._targets, self._target_rows, before)
            i = int(np.argmax(gains))
            if gains[i] > best_gain:
                best, best_gain = batch[i], int(gains[i])
        return best, best_gain

    def conjugates(self, t, max_conj=60):
        """Only the conjugate part of Vacate.step, in precisely its loop order."""
        self._check_deadline()
        G = self.G
        X = self.letters_at(t)
        Z = [z for c in self.neighbours(G.letters[X[0]][0])[:max_conj]
             for z in G.by_grip[c]] if X else []
        before = self.occ[self._targets]
        for z in Z:
            self._check_deadline()
            zi = int(G.inv[z])
            tz = G.image([zi], t)
            if tz == t:
                continue
            targets = G.images([zi], self._targets)
            membership = self._target_membership(targets)
            for batch in self._batches(self._commutators(tz, both=False), len(targets)):
                gains = self._gains(batch, targets, membership, before, endpoint=z)
                positive = np.flatnonzero(gains > 0)
                if len(positive):
                    i = int(positive[0])
                    return [zi] + batch[i] + [z], int(gains[i])
        return None, 0

    def step(self, t, max_conj=60):
        word, gain = self.cheap(t)
        return (word, gain) if word is not None else self.conjugates(t, max_conj)

    def run(self, max_steps=500):
        word, steps = [], []
        while True:
            targets = np.flatnonzero(self.occ & self.bad).tolist()
            if not targets:
                return word, steps, True
            if len(steps) >= max_steps:
                return word, steps, False
            found = None
            for scan in (self.cheap, self.conjugates):
                for t in targets:
                    self._check_deadline()
                    w, g = scan(t)
                    if w is not None:
                        found = t, w, g
                        break
                if found is not None:
                    break
            if found is None:
                return word, steps, False
            t, w, g = found
            self._check_deadline()
            self.apply(w)
            word.extend(w)
            steps.append({'target': t, 'length': len(w), 'gain': g, 'left': int((self.occ & self.bad).sum())})
            VA.log(step=len(steps), target=t, length=len(w), gain=g, left=steps[-1]['left'])
