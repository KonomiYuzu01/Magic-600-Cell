"""Exact count-only vacancy gain: synthetic search replay and the committed step-97 state.

Run from the repository root with python -I -B. All geometry caches live in a
temporary directory; this imports mechanics directly and never runs the runner.
"""
import json
import os
from pathlib import Path
import random
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest import mock

for key in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[key] = '1'
sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[5]
GENERAL = ROOT / 'research/jumbling/restoration/09-general-method/scripts'
sys.path[:0] = [str(GENERAL / 'echain'), str(GENERAL / 'gm')]

import numpy as np  # noqa: E402
import vacate as VA  # noqa: E402
from vacate2 import Vacate2  # noqa: E402


class SyntheticAlphabet:
    """Random cap permutations with explicit inverses and independent full maps."""

    def __init__(self, seed=7101):
        rng = np.random.default_rng(seed)
        self.NP = 40
        self.safe = [1, 3, 5, 7, 9, 11]
        caps = [range(10), range(4, 14), range(9, 20), range(22, 27), range(28, 33), range(34, 38)]
        self.capx = {c: np.array(xs, np.int64) for c, xs in zip(self.safe, caps)}
        self.home = np.zeros((self.NP, max(self.safe) + 1), bool)
        self.letters, self.by_grip, self.permutations, inverse = [], {}, [], []
        for c in self.safe:
            xs = self.capx[c]
            self.home[xs, c] = True
            self.by_grip[c] = []
            for _ in range(2):
                p = rng.permutation(len(xs))
                first = len(self.letters)
                for q in (p, np.argsort(p)):
                    n = len(self.letters)
                    self.letters.append((c, len(self.by_grip[c]), n, xs[q]))
                    self.by_grip[c].append(n)
                    full = np.arange(self.NP)
                    full[xs] = xs[q]
                    self.permutations.append(full)
                inverse.extend([first + 1, first])
        self.inv = np.array(inverse, np.int64)

    def images(self, word, points):
        points = np.array(points, np.int64)
        for n in word:
            points = self.permutations[n][points]
        return points

    def image(self, word, point):
        return int(self.images(word, [point])[0])


def commutator(G, x, y):
    return [x, y, int(G.inv[x]), int(G.inv[y])]


def synthetic_words(G):
    words = [[n] for n in range(len(G.letters))]
    for x in range(len(G.letters)):
        for y in range(len(G.letters)):
            words.extend([commutator(G, x, y), commutator(G, y, x)])
    rng = random.Random(7101)
    for _ in range(256):
        x, y, z = [rng.randrange(len(G.letters)) for _ in range(3)]
        words.append([z] + commutator(G, x, y) + [int(G.inv[z])])
    # Multiple letters of one cap, cancellations and words longer than the scans.
    for c in G.safe:
        x, y = G.by_grip[c][:2]
        words.extend([[x, x, x], [x, y], [x, y, x, y, x]])
    words.extend([[rng.randrange(len(G.letters)) for _ in range(24)] for _ in range(32)])
    return words


class SyntheticGainTests(unittest.TestCase):
    def setUp(self):
        self.G = SyntheticAlphabet()
        self.bad = np.zeros(self.G.NP, bool)
        self.bad[[0, 2, 5, 10, 15, 20, 28, 30, 35, 39]] = True
        self.holes = np.array([1, 2, 6, 9, 14, 19, 28, 29, 31, 33])

    def test_gain_agrees_for_all_synthetic_word_families(self):
        words = synthetic_words(self.G)
        outside = np.zeros(self.G.NP, bool)
        outside[[20, 21, 38, 39]] = True
        cases = [(self.bad, self.holes), (self.bad, []), (self.bad, range(self.G.NP)),
                 (np.zeros(self.G.NP, bool), self.holes), (np.ones(self.G.NP, bool), self.holes),
                 (outside, self.holes)]
        for case, (bad, holes) in enumerate(cases):
            va = Vacate2(self.G, bad, holes)
            for word in words:
                result = va.gain(word)
                self.assertEqual(result, (VA.Vacate.gain(va, word)[0], None, None), (case, word))
        # These isolated supports cover all combinations of targets and holes.
        self.assertFalse(self.bad[self.G.capx[7]].any())
        self.assertFalse(np.isin(self.G.capx[7], self.holes).any())
        self.assertTrue(self.bad[self.G.capx[9]].any())
        self.assertTrue(np.isin(self.G.capx[9], self.holes).any())
        self.assertTrue(self.bad[self.G.capx[11]].any())
        self.assertFalse(np.isin(self.G.capx[11], self.holes).any())

    def test_gain_maps_only_targets_in_support(self):
        va = Vacate2(self.G, self.bad, self.holes)
        x, y = self.G.by_grip[1][0], self.G.by_grip[3][0]
        word = [x, x, y, int(self.G.inv[x]), x]
        expected = np.flatnonzero(self.bad & self.G.home[:, [1, 3]].any(axis=1))
        with mock.patch.object(va, 'support', side_effect=AssertionError('full support built')):
            with mock.patch.object(self.G, 'images', wraps=self.G.images) as images:
                va.gain(word)
        images.assert_called_once()
        inverse, points = images.call_args.args
        self.assertEqual(inverse, [int(self.G.inv[n]) for n in reversed(word)])
        np.testing.assert_array_equal(points, expected)

    def test_run_matches_base_gains_and_candidate_order(self):
        movable = self.bad.copy()
        movable[[20, 35, 39]] = False
        full = np.zeros(self.G.NP, bool)
        full[0] = True
        scenarios = [(self.bad, self.holes), (movable, self.holes), (full, []),
                     (np.zeros(self.G.NP, bool), self.holes)]
        seen_lengths, seen_flags = set(), set()
        for bad, holes in scenarios:
            for limit in (0, 1, 4, 20):
                results, traces, occupancies = [], [], []
                for base in (False, True):
                    va = Vacate2(self.G, bad, holes)
                    gain = VA.Vacate.gain.__get__(va) if base else va.gain
                    trace = []

                    def traced(word):
                        result = gain(word)
                        trace.append((tuple(word), result[0]))
                        return result

                    va.gain = traced
                    with mock.patch.object(VA, 'log'):
                        results.append(va.run(max_steps=limit))
                    traces.append(trace)
                    occupancies.append(va.occ.copy())
                self.assertEqual(results[0], results[1])
                self.assertEqual(traces[0], traces[1])
                np.testing.assert_array_equal(occupancies[0], occupancies[1])
                seen_lengths.update(len(w) for w, _ in traces[0])
                seen_flags.add(results[0][2])
        self.assertEqual(seen_lengths, {1, 4, 6})
        self.assertEqual(seen_flags, {False, True})

    def test_deadline_wrapper_interrupts_cheap_scan(self):
        va = Vacate2(self.G, self.bad, self.holes)
        before = va.occ.copy()
        gain0, deadline = va.gain, time.time() - 1

        def gain(word):
            if time.time() > deadline:
                raise TimeoutError
            return gain0(word)

        va.gain = gain
        with self.assertRaises(TimeoutError):
            va.run(max_steps=2)
        np.testing.assert_array_equal(va.occ, before)

    def test_deadline_wrapper_interrupts_conjugate_scan(self):
        bad = np.zeros(self.G.NP, bool)
        bad[0] = True
        va = Vacate2(self.G, bad, [])  # No gaining word, so the fallback is reached.
        gain0 = va.gain
        calls = []

        def gain(word):
            calls.append(len(word))
            if len(word) == 6:
                raise TimeoutError
            return gain0(word)

        va.gain = gain
        with self.assertRaises(TimeoutError):
            va.run(max_steps=2)
        self.assertEqual(set(calls), {1, 4, 6})

    def test_apply_matches_base_and_gain_reads_current_occupancy(self):
        va = Vacate2(self.G, self.bad, self.holes)
        reference = VA.Vacate(self.G, self.bad, self.holes)
        words = synthetic_words(self.G)
        rng = random.Random(97)
        for word in rng.sample(words, 40):
            old_count = int((va.occ & va.bad).sum())
            gain = va.gain(word)[0]
            with mock.patch.object(va, 'gain', side_effect=AssertionError('apply called wrapped gain')):
                va.apply(word)
            reference.apply(word)
            np.testing.assert_array_equal(va.occ, reference.occ)
            self.assertEqual(old_count - int((va.occ & va.bad).sum()), gain)
            for probe in rng.sample(words, 12):
                self.assertEqual(va.gain(probe)[0], reference.gain(probe)[0])

    def test_apply_checks_full_permutation_before_mutating(self):
        for outside in (False, True):
            G = SyntheticAlphabet()
            c, i, k, ys = G.letters[0]
            ys = ys.copy()
            ys[0] = G.NP - 1 if outside else ys[1]
            G.letters[0] = (c, i, k, ys)
            G.permutations[0][G.capx[c]] = ys
            va = Vacate2(G, np.zeros(G.NP, bool), self.holes)
            before = va.occ.copy()
            # Even a word touching no targets must be checked when it is applied.
            self.assertEqual(va.gain([0]), (0, None, None))
            with mock.patch.object(va, 'gain', side_effect=AssertionError('wrapped gain called')):
                with self.assertRaisesRegex(AssertionError, 'word does not permute its support'):
                    va.apply([0])
            np.testing.assert_array_equal(va.occ, before)


class RealStateGainTests(unittest.TestCase):
    def test_real_state_gains_and_timings(self):
        with tempfile.TemporaryDirectory(prefix='vacate2-gain-') as temporary:
            cache = Path(temporary) / 'home_centroids.npy'
            # Set the environment before any mechanics import, then redirect the
            # seams module as Mechanics does, without importing or running runner.
            with mock.patch.dict(os.environ, {'JUMBLING_HOME_CACHE': str(cache)}):
                theory = ROOT / 'research/jumbling/theory'
                sys.path[:0] = [str(theory), str(theory.parent), str(theory.parent / 'fixtures')]
                import seams
                seams.CACHE = cache
                import buffer as B
                import orient
                import restore
                import sim

                menu = restore.menu('I_a')
                state = sim.State(restore.ctx, menu=menu)
                table = (ROOT / 'research/jumbling/restoration/13-tutorial-runner/runs/dev/'
                         'd1-dev-s7101/run1-interrupted/tables/00097.json')
                orient.set_pose_table(state, json.loads(table.read_bytes()))
                self.assertEqual(state.digest(), 'cc41accae7cac3add5ab1c3b734b5ba7bee72c1008ec666caf27a1150fdb9938')
                G = B.Alphabet(SimpleNamespace(st=state), menu)
                self.assertEqual(len(G.holes), 1082)
                rng = random.Random(7101)
                # Include occupied and vacant chambers in movable caps, rather
                # than relying on rare holes in a uniform sample of all chambers.
                movable = G.home[:, G.safe].any(axis=1)
                occupied = np.flatnonzero(movable & G.occupied).tolist()
                holes = np.flatnonzero(movable & ~G.occupied).tolist()
                targets = rng.sample(occupied, 150) + rng.sample(holes, 50)
                bad = np.zeros(G.NP, bool)
                bad[targets] = True
                va = Vacate2(G, bad, G.holes)
                self.assertEqual(int((va.occ & bad).sum()), 150)

                words = []
                for _ in range(800):
                    x, y, z = [rng.randrange(len(G.letters)) for _ in range(3)]
                    while G.letters[x][0] == G.letters[y][0]:
                        y = rng.randrange(len(G.letters))
                    words.append([x])
                    words.append(commutator(G, x, y))
                    words.append([z] + commutator(G, x, y) + [int(G.inv[z])])
                self.assertEqual(len(words), 2400)
                # Time the same words at the same occupancy, excluding setup,
                # comparisons and the full check done separately during apply.
                started = time.perf_counter()
                old = [VA.Vacate.gain(va, w)[0] for w in words]
                old_seconds = time.perf_counter() - started
                started = time.perf_counter()
                new = [va.gain(w)[0] for w in words]
                new_seconds = time.perf_counter() - started
                self.assertEqual(new, old)
                print(json.dumps({'real_state': 'step-97-s7101', 'words': len(words),
                                  'targets': len(targets), 'occupied_targets': 150,
                                  'python': sys.version.split()[0], 'numpy': np.__version__,
                                  'old_mean_us': old_seconds * 1e6 / len(words),
                                  'new_mean_us': new_seconds * 1e6 / len(words)}), flush=True)


if __name__ == '__main__':
    started = time.perf_counter()
    result = unittest.main(verbosity=2, exit=False).result
    elapsed = time.perf_counter() - started
    print(f'Acceptance wall seconds: {elapsed:.3f} (limit 550)', flush=True)
    sys.exit(0 if result.wasSuccessful() and elapsed < 550 else 1)
