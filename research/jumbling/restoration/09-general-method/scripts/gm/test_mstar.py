"""Control-flow tests of the M* outer loop (mstar.py) with synthetic steps in place of the simulator: the scenarios of
the milestone audit (SENIOR-REVIEW.md section 9, AU-01). A state is its number of off pieces, and a step lowers it by
one. Run: python3 -I -B test_mstar.py
"""
import sys
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


if __name__ == '__main__':
    unittest.main()
