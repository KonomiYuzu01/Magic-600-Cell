"""Exact batched vacancy search: synthetic replay and committed step-97 checks.

Run from the repository root with python3 -I -B. No runner or closure script is
executed. All mechanics caches are redirected into a temporary directory.
"""
import ast
from itertools import islice
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
sys.path[:0] = [str(Path(__file__).resolve().parent), str(GENERAL / 'echain'), str(GENERAL / 'gm')]

import numpy as np  # noqa: E402
import vacate as VA  # noqa: E402
from vacate2 import Vacate2  # noqa: E402
from vacate3 import Vacate3  # noqa: E402
from test_vacate2_gain import SyntheticAlphabet, commutator, synthetic_words  # noqa: E402


def scenarios(G):
    bad = np.zeros(G.NP, bool)
    bad[[0, 2, 5, 10, 15, 20, 28, 30, 35, 39]] = True
    holes = np.array([1, 2, 6, 9, 14, 19, 28, 29, 31, 33])
    outside = np.zeros(G.NP, bool)
    outside[[20, 21, 38, 39]] = True
    movable = bad.copy()
    movable[[20, 35, 39]] = False
    full = np.zeros(G.NP, bool)
    full[0] = True
    letter_hole = next(h for x in G.by_grip[1]
                       if (h := G.image([int(G.inv[x])], 0)) != 0)
    return [('mixed', bad, holes), ('no-holes', bad, []), ('all-holes', bad, range(G.NP)),
            ('no-targets', np.zeros(G.NP, bool), holes), ('all-targets', np.ones(G.NP, bool), holes),
            ('outside', outside, holes), ('movable', movable, holes), ('full', full, []),
            ('letter', full, [letter_hole]), ('conjugate', full, [2])]


def commutators(va, t, both=True, coverage=None):
    """Independent transcription of the base candidate loops (no batch helpers)."""
    G = va.G
    for x in va.letters_at(t):
        xt = G.image([x], t)
        for y in sorted(set(va.letters_at(t)) | set(va.letters_at(xt))):
            if G.letters[y][0] == G.letters[x][0]:
                if coverage is not None:
                    coverage.add('same-cap')
                continue
            yield commutator(G, x, y)
            if both:
                yield commutator(G, y, x)


def conjugate_words(va, t, max_conj=60, coverage=None):
    G = va.G
    X = va.letters_at(t)
    Z = [z for c in va.neighbours(G.letters[X[0]][0])[:max_conj]
         for z in G.by_grip[c]] if X else []
    for z in Z:
        zi = int(G.inv[z])
        tz = G.image([zi], t)
        if tz == t:
            if coverage is not None:
                coverage.add('fixed-z')
            continue
        for word in commutators(va, tz, both=False, coverage=coverage):
            yield [zi] + word + [z]


def sequential_conjugates(va, t, max_conj=60, coverage=None):
    for word in conjugate_words(va, t, max_conj, coverage):
        gain = Vacate2.gain(va, word)[0]
        if gain > 0:
            return word, gain
    return None, 0


class SyntheticBatchTests(unittest.TestCase):
    def setUp(self):
        self.G = SyntheticAlphabet()

    def test_all_synthetic_gains_mixed_batches_and_batches_of_one(self):
        words = synthetic_words(self.G) + [[]]
        random.Random(97).shuffle(words)
        self.assertEqual(set(map(len, words)), {0, 1, 2, 3, 4, 5, 6, 24})
        for name, bad, holes in scenarios(self.G):
            with self.subTest(scenario=name):
                reference = Vacate2(self.G, bad, holes)
                va = Vacate3(self.G, bad, holes)
                self.assertIsNone(va._positions)
                expected = [reference.gain(w)[0] for w in words]
                np.testing.assert_array_equal(va.gains(words), expected)
                np.testing.assert_array_equal([int(va.gains([w])[0]) for w in words], expected)
                np.testing.assert_array_equal(va.gains(iter(words)), expected)
                self.assertEqual(va.gains([]).tolist(), [])

    def test_per_target_cheap_and_conjugates_match_sequential_reference(self):
        coverage = set()
        for name, bad, holes in scenarios(self.G):
            reference = Vacate2(self.G, bad, holes)
            va = Vacate3(self.G, bad, holes)
            for t in np.flatnonzero(reference.occ & bad).tolist():
                with self.subTest(scenario=name, target=t):
                    self.assertEqual(va.cheap(t), reference.cheap(t))
                    evaluate = va._gains

                    def checked(words, targets, membership, before, endpoint=None):
                        gains = evaluate(words, targets, membership, before, endpoint)
                        full = ([int(self.G.inv[endpoint])] + w + [endpoint] for w in words)
                        np.testing.assert_array_equal(gains, [reference.gain(w)[0] for w in full])
                        return gains

                    # Compare every hoisted, support-restricted batch with the
                    # unpruned full conjugates, including its later candidates.
                    with mock.patch.object(va, '_gains', side_effect=checked):
                        self.assertEqual(va.conjugates(t), sequential_conjugates(reference, t, coverage=coverage))
                    self.assertEqual(va.step(t), reference.step(t))
        self.assertEqual(coverage, {'same-cap', 'fixed-z'})

    def test_runs_steps_occupancies_and_logs_match(self):
        seen_lengths, seen_flags = set(), set()
        for name, bad, holes in scenarios(self.G):
            for limit in (0, 1, 4, 20):
                with self.subTest(scenario=name, limit=limit):
                    reference = Vacate2(self.G, bad, holes)
                    va = Vacate3(self.G, bad, holes)
                    with mock.patch.object(VA, 'log') as log:
                        expected = reference.run(max_steps=limit)
                        expected_log = log.call_args_list[:]
                        log.reset_mock()
                        result = va.run(max_steps=limit)
                        self.assertEqual(log.call_args_list, expected_log)
                    self.assertEqual(result, expected)
                    np.testing.assert_array_equal(va.occ, reference.occ)
                    seen_lengths.update(row['length'] for row in result[1])
                    seen_flags.add(result[2])
        self.assertEqual(seen_lengths, {1, 4, 6})
        self.assertEqual(seen_flags, {False, True})

    def test_first_maximal_commutator_wins_ties_across_batches(self):
        ties = []
        for name, bad, holes in scenarios(self.G):
            reference = Vacate2(self.G, bad, holes)
            for _ in range(20):
                for t in np.flatnonzero(reference.occ & bad).tolist():
                    if any(reference.gain([x])[0] > 0 for x in reference.letters_at(t)):
                        continue
                    words = list(commutators(reference, t))
                    gains = [reference.gain(w)[0] for w in words]
                    best = max(gains, default=0)
                    winners = [i for i, g in enumerate(gains) if g == best and g > 0]
                    if len({tuple(words[i]) for i in winners}) > 1:
                        va = Vacate3(self.G, bad, holes)
                        va.occ = reference.occ.copy()
                        # A boundary between tied candidates must not replace
                        # the first winner with a later equal-gain word.
                        va.BATCH_WORDS = 1
                        self.assertEqual(va.cheap(t), (words[winners[0]], best))
                        ties.append((name, t))
                        break
                if ties:
                    break
                with mock.patch.object(VA, 'log'):
                    result = reference.run(max_steps=1)
                if not result[1]:
                    break
            if ties:
                break
        self.assertTrue(ties, 'scenarios must exercise a positive commutator tie')

    def test_gains_follow_current_occupancy_and_apply_is_inherited(self):
        _, bad, holes = scenarios(self.G)[0]
        va, reference = Vacate3(self.G, bad, holes), Vacate2(self.G, bad, holes)
        self.assertIs(Vacate3.apply, Vacate2.apply)
        words = synthetic_words(self.G)
        rng = random.Random(7101)
        for word in rng.sample(words, 20):
            probes = rng.sample(words, 32)
            np.testing.assert_array_equal(va.gains(probes), [reference.gain(w)[0] for w in probes])
            with mock.patch.object(va, 'gain', side_effect=AssertionError('count-only apply')):
                va.apply(word)
            reference.apply(word)
            np.testing.assert_array_equal(va.occ, reference.occ)

    def test_empty_alphabet_and_target_outside_every_support(self):
        _, bad, holes = scenarios(self.G)[5]
        va = Vacate3(self.G, bad, holes)
        words = synthetic_words(self.G)
        np.testing.assert_array_equal(va.gains(words), np.zeros(len(words), np.int64))
        G = SimpleNamespace(NP=3, safe=[], letters=[], inv=np.array([], np.int64), capx={},
                            by_grip={}, home=np.zeros((3, 0), bool))
        for holes in ([], [1]):
            reference, va = Vacate2(G, [False, True, False], holes), Vacate3(G, [False, True, False], holes)
            self.assertEqual(va.gains([[]]).tolist(), [0])
            self.assertEqual(va.cheap(1), (None, 0))
            self.assertEqual(va.conjugates(1), (None, 0))
            self.assertEqual(va.run(), reference.run())
            np.testing.assert_array_equal(va.occ, reference.occ)


class DeadlineTests(unittest.TestCase):
    def setUp(self):
        self.G = SyntheticAlphabet()
        self.bad = np.zeros(self.G.NP, bool)
        self.bad[0] = True

    def test_past_deadline_in_first_pass_and_conjugate_scan(self):
        for operation in (lambda va: va.run(), lambda va: va.cheap(0), lambda va: va.conjugates(0),
                          lambda va: va.gains([[0]])):
            va = Vacate3(self.G, self.bad, [], deadline=time.time() - 1)
            before = va.occ.copy()
            with self.assertRaises(TimeoutError):
                operation(va)
            np.testing.assert_array_equal(va.occ, before)

    def test_second_pass_timeout_keeps_the_steps_occupancy(self):
        va = Vacate3(self.G, self.bad, [])  # Full occupancy: no first-pass gain.
        before, scan = va.occ.copy(), va.conjugates

        def expire(t):
            va.deadline = time.time() - 1
            return scan(t)

        with mock.patch.object(va, 'conjugates', side_effect=expire) as conjugates:
            with self.assertRaises(TimeoutError):
                va.run()
        conjugates.assert_called_once_with(0)
        np.testing.assert_array_equal(va.occ, before)

    def test_deadline_checked_between_batches_and_before_apply(self):
        _, bad, holes = scenarios(self.G)[0]
        va = Vacate3(self.G, bad, holes, deadline=time.time() + 60)
        va.BATCH_WORDS = 1
        before, evaluate, batches = va.occ.copy(), va._gains, []

        def expire_after_batch(*args, **kwargs):
            result = evaluate(*args, **kwargs)
            batches.append(1)
            va.deadline = time.time() - 1
            return result

        with mock.patch.object(va, '_gains', side_effect=expire_after_batch):
            with self.assertRaises(TimeoutError):
                va.gains([[0], [1]])
        self.assertEqual(len(batches), 1)
        np.testing.assert_array_equal(va.occ, before)
        va.deadline = time.time() + 60
        with mock.patch.object(va, '_gains', side_effect=expire_after_batch):
            with self.assertRaises(TimeoutError):
                va.run()
        np.testing.assert_array_equal(va.occ, before)

    def test_tclose_constructs_vacate3_with_deadline_and_keeps_report_fields(self):
        tree = ast.parse((GENERAL / 'gm/tclose2.py').read_text())
        calls = [n for n in ast.walk(tree) if isinstance(n, ast.Call)
                 and isinstance(n.func, ast.Attribute) and n.func.attr == 'Vacate3']
        self.assertEqual(len(calls), 1)
        self.assertEqual(ast.unparse(calls[0]), 'VA3.Vacate3(G, mask, G.holes, deadline=deadline)')
        reports = [n.value for n in ast.walk(tree) if isinstance(n, ast.Assign)
                   and any(ast.unparse(t) == "pre['greedy']" for t in n.targets)]
        self.assertEqual(len(reports), 1)
        self.assertEqual([k.value for k in reports[0].keys],
                         ['ok', 'steps', 'length', 'left', 'timed_out', 'seconds_limit'])
        self.assertFalse(any(isinstance(n, ast.Assign) and any(ast.unparse(t) == 'va.gain' for t in n.targets)
                             for n in ast.walk(tree)))


class RealStateBatchTests(unittest.TestCase):
    def test_real_state_searches_and_same_candidate_timings(self):
        with tempfile.TemporaryDirectory(prefix='vacate3-gain-') as temporary:
            cache = Path(temporary) / 'home_centroids.npy'
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
                movable = G.home[:, G.safe].any(axis=1)
                occupied = np.flatnonzero(movable & G.occupied).tolist()
                holes = np.flatnonzero(movable & ~G.occupied).tolist()
                targets = rng.sample(occupied, 150) + rng.sample(holes, 50)
                bad = np.zeros(G.NP, bool)
                bad[targets] = True
                reference, va = Vacate2(G, bad, G.holes), Vacate3(G, bad, G.holes)
                occupied_targets = np.flatnonzero(reference.occ & bad).tolist()
                self.assertEqual(len(occupied_targets), 150)
                for t in occupied_targets[:10]:
                    with self.subTest(cheap_target=t):
                        self.assertEqual(va.cheap(t), reference.cheap(t))
                # Two neighbour grips keep the sequential reference bounded;
                # candidates and selection order within that prefix are exact.
                for t in occupied_targets[:2]:
                    with self.subTest(conjugate_target=t):
                        self.assertEqual(va.conjugates(t, max_conj=2),
                                         sequential_conjugates(reference, t, max_conj=2))

                words = []
                for t in occupied_targets[:10]:
                    words.extend([x] for x in reference.letters_at(t))
                    words.extend(commutators(reference, t))
                # Include six-letter candidates and use the same set for both
                # evaluators, with geometry setup and correctness checks outside
                # the timed warm-cache calls.
                words.extend(islice(conjugate_words(reference, occupied_targets[0], max_conj=2), 4096))
                expected = [reference.gain(w)[0] for w in words]
                np.testing.assert_array_equal(va.gains(words), expected)
                old_times, new_times = [], []
                for _ in range(3):
                    started = time.perf_counter()
                    old = [reference.gain(w)[0] for w in words]
                    old_times.append(time.perf_counter() - started)
                    started = time.perf_counter()
                    new = va.gains(words)
                    new_times.append(time.perf_counter() - started)
                    np.testing.assert_array_equal(new, old)
                old_mean = sum(old_times) / (len(words) * len(old_times))
                new_mean = sum(new_times) / (len(words) * len(new_times))
                ratio = old_mean / new_mean
                extra = (va._positions.nbytes + va._images.nbytes + va._caps.nbytes
                         + va._offsets.nbytes + va._inverse.nbytes + va._target_rows.nbytes
                         + sum(a.nbytes for a in va._target_caps.values()))
                print(json.dumps({'real_state': 'step-97-s7101', 'candidates': len(words),
                                  'targets': len(targets), 'occupied_targets': 150,
                                  'python': sys.version.split()[0], 'numpy': np.__version__,
                                  'vacate2_mean_us': old_mean * 1e6, 'vacate3_mean_us': new_mean * 1e6,
                                  'ratio': ratio, 'extra_cache_bytes': extra,
                                  'batch_words': va.BATCH_WORDS, 'batch_pairs': va.BATCH_PAIRS}), flush=True)
                self.assertLess(extra, 1_000_000_000)
                self.assertGreaterEqual(ratio, 10)


if __name__ == '__main__':
    started = time.perf_counter()
    result = unittest.main(verbosity=2, exit=False).result
    elapsed = time.perf_counter() - started
    print(f'Acceptance wall seconds: {elapsed:.3f} (limit 900)', flush=True)
    sys.exit(0 if result.wasSuccessful() and elapsed < 900 else 1)
