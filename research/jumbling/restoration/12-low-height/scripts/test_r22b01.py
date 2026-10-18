# Small exact controls for R22B-01; no census, geometry, or simulator imports.
# Run: python3 -I -B research/jumbling/restoration/12-low-height/scripts/test_r22b01.py
import itertools
import random
import sys
import time
import unittest
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import r22b01_cert as C

C.np = np


class SmallAlphabet:
    """Independent Alt(7) factors, or the diagonal Alt(7), on two orbits."""

    def __init__(self, diagonal=False, coupled=False):
        self.NP = 14
        self.letters, self.maps, self.by_grip, self.capx = [], [], {}, {}
        self.inv = []
        triples = list(itertools.combinations(range(7), 3))
        cycles = [[t, tuple(x + 7 for x in t)] for t in triples] if diagonal else (
            [[tuple(x + shift for x in t)] for shift in (0, 7) for t in triples])
        if coupled:
            cycles = ([[t, tuple(x + 7 for x in t)] for t in triples] +
                      [[tuple(x + 7 for x in t)] for t in triples])
        for grip, group in enumerate(cycles):
            xs = np.array(sorted(x for t in group for x in t), np.int64)
            self.capx[grip] = xs
            self.by_grip[grip] = []
            for item in range(2):
                p = np.arange(self.NP)
                for t in group:
                    t = t if item == 0 else t[::-1]
                    p[list(t)] = [t[1], t[2], t[0]]
                self.by_grip[grip].append(len(self.letters))
                self.letters.append((grip, item, None, p[xs]))
                self.maps.append(p)
            n = len(self.letters)
            self.inv.extend([n - 1, n - 2])
        self.inv = np.array(self.inv)
        self.index = {(g, i): n for n, (g, i, _, _) in enumerate(self.letters)}
        self.home = np.zeros((self.NP, len(cycles)), bool)
        for c, xs in self.capx.items():
            self.home[xs, c] = True
        self.occupied = np.ones(self.NP, bool)
        self.occupied[[3, 4, 5, 10, 12, 13]] = False
        self.holes = np.flatnonzero(~self.occupied)

    def pairs(self, word):
        return [[self.letters[n][0], self.letters[n][1]] for n in word]

    def numbers(self, pairs):
        return [self.index[tuple(pair)] for pair in pairs]

    def images(self, word, points):
        p = np.array(points, np.int64)
        for n in word:
            p = self.maps[n][p]
        return p

    def image(self, word, point):
        return int(self.images(word, [point])[0])

    def perm(self, word):
        return self.images(word, np.arange(self.NP))

    def orbits(self):
        return np.repeat([0, 1], 7), np.array([7, 7])


class CertificateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.G = SmallAlphabet(coupled=True)
        cls.result = {'complete': False, 'orbits': [], 'V': [], 'V_length': 0, 'bound': 0}
        cls.T = [0, 1, 2, 7, 8, 9]
        C.build(cls.G, {'T': cls.T}, cls.result, time.monotonic() + 10)

    def test_complete_certificate_and_full_vacancy_replay(self):
        G, doc = self.G, self.result
        self.assertTrue(doc['complete'])
        self.assertEqual(len(doc['orbits']), 2)
        first_seed = G.numbers(doc['orbits'][0]['seed'])
        self.assertFalse(np.array_equal(G.images(first_seed, range(7, 14)), range(7, 14)))
        V = G.numbers(doc['V'])
        self.assertEqual(doc['V_length'], len(V))
        final = np.empty(G.NP, bool)
        final[G.perm(V)] = G.occupied
        self.assertFalse(final[self.T].any())
        earlier, bound = [], 0
        for row in doc['orbits']:
            D = np.arange(row['least'], row['least'] + row['size'])
            a, b, c = row['anchors']
            seed = G.numbers(row['seed'])
            want = D.copy()
            want[np.searchsorted(D, [a, b, c])] = [b, c, a]
            np.testing.assert_array_equal(G.images(seed, D), want)
            np.testing.assert_array_equal(G.images(seed, earlier), earlier)
            depths = []
            for key, root, fixed, forward in (('P', b, [a], True), ('Q', c, [a, b], False)):
                table = {x: (g, i, y) for x, g, i, y in row[key]}
                self.assertEqual(set(table), set(D) - set(fixed) - {root})
                depth = 0
                for point in table:
                    seen, x = set(), point
                    while x != root:
                        self.assertNotIn(x, seen)
                        seen.add(x)
                        g, i, y = table[x]
                        n = G.index[(g, i)]
                        np.testing.assert_array_equal(G.images([n], fixed), fixed)
                        self.assertEqual(G.image([n], x if forward else y), y if forward else x)
                        x = y
                    depth = max(depth, len(seen))
                depths.append(depth)
            bound += 2 * row['k'] * (len(seed) + 2 * sum(depths))
            earlier.extend(D.tolist())
        self.assertEqual(doc['bound'], bound)
        self.assertLessEqual(len(V), bound)

    def test_every_ordered_three_cycle_including_anchor_and_inverse(self):
        G = self.G
        for row in self.result['orbits']:
            seed = G.numbers(row['seed'])
            P, Q = C.decode_tree(G, row['P']), C.decode_tree(G, row['Q'])
            D = np.arange(row['least'], row['least'] + 7)
            for p, q, r in itertools.permutations(D.tolist(), 3):
                word = C.three_cycle(G, row, seed, P, Q, p, q, r)
                want = D.copy()
                want[np.searchsorted(D, [p, q, r])] = [q, r, p]
                np.testing.assert_array_equal(G.images(word, D), want)
                np.testing.assert_array_equal(G.images(word, range(row['least'])),
                                              range(row['least']))

    def test_sparse_support_covers_entire_commutator(self):
        G = SmallAlphabet(diagonal=True)
        rng = random.Random(22)
        membership = np.ones(G.NP, bool)
        for arity in (2, 3):
            for _ in range(50):
                x = rng.randrange(len(G.letters))
                right = [rng.randrange(len(G.letters)) for _ in range(arity - 1)]
                caps = [G.letters[n][0] for n in right]
                points, p = C.commutator_action(G, x, right, caps, membership)
                word = [x] + right + [int(G.inv[x])] + C.inverse(G, right)
                full = G.perm(word)
                reconstructed = np.arange(G.NP)
                reconstructed[points] = points[p]
                np.testing.assert_array_equal(reconstructed, full)

    def test_power_kills_other_cycles_and_rejects_other_multiples_of_three(self):
        pts = np.arange(10)
        p = np.array([1, 2, 0, 4, 3, 6, 7, 8, 9, 5])  # 3, 2, 5
        self.assertEqual(C.seed_power(pts, p, True), (10, [0, 1, 2]))
        self.assertIsNone(C.seed_power(pts, p, False))
        self.assertIsNone(C.seed_power(np.arange(6), np.array([1, 2, 0, 4, 5, 3]), True))
        self.assertIsNone(C.seed_power(np.arange(6), np.array([1, 2, 3, 4, 5, 0]), True))

    def test_diagonal_action_cannot_supply_a_seed_fixing_the_earlier_orbit(self):
        G = SmallAlphabet(diagonal=True)
        safe = np.array(sorted(G.by_grip))
        D, earlier = np.arange(7, 14), np.arange(7)
        counts = C.overlap_counts(G, D, safe, time.monotonic() + 1)
        found, trials = C.find_seed(G, D, earlier, counts, counts, safe,
                                   time.monotonic() + 0.1, random.Random(1))
        self.assertGreater(trials, 0)
        self.assertIsNone(found)

    def test_disconnected_stabilizer_tables_are_reported(self):
        G = SmallAlphabet(diagonal=True)
        # Retain just one diagonal 3-cycle and its inverse. No retained letter
        # fixes the anchor it moves, hence neither stabilizer graph connects.
        G.letters, G.maps, G.inv = G.letters[:2], G.maps[:2], G.inv[:2]
        G.by_grip = {0: [0, 1]}
        G.index = {(0, 0): 0, (0, 1): 1}
        D = np.arange(7)
        end = time.monotonic() + 1
        edges = C.orbit_edges(G, D, end)
        first, missing = C.letter_tree(G, D, edges, 1, [0], True, end)
        self.assertIsNone(first)
        self.assertEqual(missing, 5)
        second, missing = C.letter_tree(G, D, edges, 2, [0, 1], False, end)
        self.assertIsNone(second)
        self.assertEqual(missing, 4)


if __name__ == '__main__':
    unittest.main()
