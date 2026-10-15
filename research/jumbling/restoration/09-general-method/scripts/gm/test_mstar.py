"""Tests of mstar.py with synthetic states in place of the simulator. The outer loop: the scenarios of the milestone
audit (SENIOR-REVIEW.md section 9, AU-01), where a state is its number of off pieces and a step lowers it by one. The
word search: order, completeness against a brute force, the length bound, blocked caps, and a word longer than the
interpreter's recursion limit. Run: python3 -I -B test_mstar.py
"""
import itertools
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import mstar  # noqa: E402


class World:
    def __init__(self):
        self.calls = []

    def units_of(self, table, k):
        return {'off': table}, ([{'key': 1, 'size': table}] if table else [])

    def step(self, name, hit=lambda table, k: True):
        def f(table, k, info, units):
            self.calls.append((name, k))
            return table - 1 if hit(table, k) else None
        return f

    def miss(self, name):
        return self.step(name, lambda table, k: False)


class DriveTest(unittest.TestCase):
    def test_start_on_the_lattice(self):
        w = World()
        self.assertEqual(mstar.drive(0, w.units_of, [w.step('h')], w.step('fb')), (0, 0, 'lattice'))
        self.assertEqual(w.calls, [])

    def test_every_heuristic_misses(self):
        w = World()
        out = mstar.drive(3, w.units_of, [w.miss('driver'), w.miss('pure')], w.step('fb'))
        self.assertEqual(out, (0, 3, 'lattice'))
        self.assertEqual(w.calls, [(n, k) for k in range(3) for n in ('driver', 'pure', 'fb')])

    def test_a_limited_heuristic_falls_back(self):
        # a heuristic that misses a usable candidate (as --pure-max can) leaves the step to the fallback
        w = World()
        out = mstar.drive(4, w.units_of, [w.step('pure', lambda table, k: table % 2 == 0)], w.step('fb'))
        self.assertEqual(out, (0, 4, 'lattice'))
        self.assertEqual([n for n, _ in w.calls], ['pure', 'pure', 'fb', 'pure', 'pure', 'fb'])

    def test_last_permitted_step_reaches_the_lattice(self):
        w = World()
        self.assertEqual(mstar.drive(3, w.units_of, [w.step('h')], w.step('fb'), budget=3), (0, 3, 'lattice'))

    def test_budget_stops_before_the_lattice(self):
        w = World()
        self.assertEqual(mstar.drive(5, w.units_of, [w.step('h')], w.step('fb'), budget=2), (3, 2, 'budget'))

    def test_no_step_limit_by_default(self):
        w = World()
        self.assertEqual(mstar.drive(200, w.units_of, [w.step('h')], w.step('fb')), (0, 200, 'lattice'))

    def test_fallback_bound(self):
        w = World()
        out = mstar.drive(2, w.units_of, [w.miss('h')], lambda table, k, info, units: None)
        self.assertEqual(out, (2, 0, 'fallback bound'))

    def test_a_failing_heuristic_is_a_miss(self):
        w = World()

        def crash(table, k, info, units):
            w.calls.append(('crash', k))
            raise ValueError('partial output')
        out = mstar.drive(2, w.units_of, [crash, w.miss('pure')], w.step('fb'))
        self.assertEqual(out, (0, 2, 'lattice'))
        self.assertEqual([n for n, _ in w.calls], ['crash', 'pure', 'fb'] * 2)

    def test_a_failing_fallback_ends_the_run(self):
        w = World()

        def broken(table, k, info, units):
            raise RuntimeError('fallback failed')
        with self.assertRaises(RuntimeError):
            mstar.drive(2, w.units_of, [w.miss('h')], broken)

    def test_first_hit_wins(self):
        w = World()
        mstar.drive(1, w.units_of, [w.step('a'), w.step('b')], w.step('fb'))
        self.assertEqual(w.calls, [('a', 0)])


class Word:
    """A synthetic puzzle: the state is the list of applied letters; a letter (c, i) is blocked when blocked(c, depth)
    holds; the end state is accepted when accept(word) holds."""

    def __init__(self, blocked, accept):
        self.w, self.blocked, self.ok = [], blocked, accept

    def apply(self, c, i):
        if self.blocked(c, len(self.w)):
            return False
        self.w.append((c, i))
        return True

    def undo(self):
        self.w.pop()

    def accept(self):
        return self.ok(self.w)


def fold(word):
    x = 1
    for c, i in word:
        x = (x * (c + 2) + i) % 13
    return x


class SearchTest(unittest.TestCase):
    def search(self, pz, caps, items, max_length=0):
        lengths = []

        def log(length, tried, found):
            lengths.append(length)
            if not found:
                self.assertEqual(pz.w, [])  # every letter undone after a length without a word
        return mstar.word_search(pz.apply, pz.undo, pz.accept, caps, items, max_length, log), lengths

    def test_first_word_in_order_against_a_brute_force(self):
        def blocked(c, depth):
            return (c + depth) % 4 == 3
        pz = Word(blocked, lambda w: fold(w) == 0)
        (word, _), lengths = self.search(pz, 4, 3)
        expect = None
        for n in itertools.count(1):
            for w in itertools.product([(c, i) for c in range(4) for i in range(1, 3)], repeat=n):
                if all(w[k][0] != w[k - 1][0] for k in range(1, n)) and not any(blocked(c, k) for k, (c, _) in
                                                                                enumerate(w)) and fold(w) == 0:
                    expect = list(w)
                    break
            if expect:
                break
        self.assertEqual(word, expect)
        self.assertEqual(pz.w, expect)
        self.assertEqual(lengths, list(range(1, len(expect) + 1)))
        self.assertGreater(len(expect), 1)

    def test_the_length_bound(self):
        pz = Word(lambda c, depth: (c + depth) % 4 == 3, lambda w: fold(w) == 0)
        (word, tried), lengths = self.search(pz, 4, 3, max_length=1)
        self.assertEqual((word, pz.w, lengths), (None, [], [1]))
        self.assertGreater(tried, 0)

    def test_a_blocked_cap_skips_its_items(self):
        pz = Word(lambda c, depth: c == 0, lambda w: w == [(1, 3)])
        (word, tried), _ = self.search(pz, 2, 4)
        self.assertEqual((word, tried), ([(1, 3)], 4))  # (0, 1) blocked, then (1, 1), (1, 2), (1, 3)

    def test_no_two_consecutive_letters_at_one_cap(self):
        pz = Word(lambda c, depth: False, lambda w: len(w) == 3)
        (word, tried), _ = self.search(pz, 2, 2)
        self.assertEqual(word, [(0, 1), (1, 1), (0, 1)])

    def test_a_word_longer_than_the_recursion_limit(self):
        n = sys.getrecursionlimit() + 100
        pz = Word(lambda c, depth: c != depth % 3, lambda w: len(w) == n)
        (word, _), lengths = self.search(pz, 3, 2)
        self.assertEqual(word, [(k % 3, 1) for k in range(n)])
        self.assertEqual(lengths, list(range(1, n + 1)))


class ClosureTest(unittest.TestCase):
    def test_a_whole_record_and_broken_ones(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            (d / 'word-full.json').write_text(json.dumps({'word': [[0, 1]], 'input_digest': 'a', 'output_digest': 'b'}))
            (d / 'pre.json').write_text('{}')
            self.assertFalse(mstar.closure_ok(d))  # no end table: a process killed before or during the export
            table = json.dumps({'poses': [], 'pose_id': [0, 0], 'digest': 'b', 'journal': []})
            (d / 'end-table.json').write_text(table[:-5])
            self.assertFalse(mstar.closure_ok(d))  # a truncated end table
            (d / 'end-table.json').write_text(table.replace('"b"', '"c"'))
            self.assertFalse(mstar.closure_ok(d))  # an end table of another state
            (d / 'end-table.json').write_text(table)
            self.assertTrue(mstar.closure_ok(d))
            (d / 'pre.json').write_text('{"blocks": [')
            self.assertFalse(mstar.closure_ok(d))  # a truncated pre.json


if __name__ == '__main__':
    unittest.main()
