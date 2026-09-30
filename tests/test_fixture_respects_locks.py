"""The E1 practice fixture must not break captured position locks or protected orbits (screening C-01)."""
from pathlib import Path
import sys, tempfile, threading, unittest
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / 'work/experiments/magic600-04'))
from core import Model
from session import Session
import adapter

MODEL = Model()
LOCKED = 35778  # a Home-block position that E1 moves


class FixtureLockTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='c600-fixture-locks-')
        self.addCleanup(self.tmp.cleanup)
        self.s = Session(MODEL, Path(self.tmp.name) / 'session')
        self.addCleanup(self.s.close)
        self.w = adapter.Workbench(self.s, threading.RLock())

    def run_command(self, body):
        return self.w.command(body, response_snapshot=False)

    def assert_refused(self, message):
        before = (self.s.head, self.s.st.hash, self.w.position_locks(), self.s.prefs['protected'])
        with self.assertRaisesRegex(ValueError, message):
            self.run_command({'action': 'fixture', 'name': 'e1'})
        self.assertEqual((self.s.head, self.s.st.hash, self.w.position_locks(), self.s.prefs['protected']), before)
        self.assertIsNone(self.s.pending)

    def test_exact_lock_refuses_fixture(self):
        self.run_command({'action': 'block-protect', 'position': LOCKED, 'mode': 'exact'})
        self.assertTrue(self.w.position_locks())
        self.assert_refused('Release all position locks')

    def test_position_lock_refuses_fixture(self):
        self.run_command({'action': 'block-protect', 'position': LOCKED, 'mode': 'position'})
        self.assertEqual([x['mode'] for x in self.w.position_locks()], ['position'])
        self.assert_refused('Release all position locks')

    def test_lock_in_inactive_orbit_context_refuses_fixture(self):
        self.run_command({'action': 'orbit', 'orbit': 33})
        self.run_command({'action': 'block-protect', 'position': LOCKED, 'mode': 'exact'})
        self.run_command({'action': 'orbit', 'orbit': 0})
        self.assertEqual(self.w.w['orbit'], 0)
        self.assertEqual([x['work_orbits'] for x in self.w.position_locks()], [[33]])
        self.assert_refused('Release all position locks')

    def test_protected_orbit_refuses_fixture(self):
        self.s.save_prefs({'protected': [33]})
        self.assert_refused('protected orbit')

    def test_fixture_loads_without_requirements(self):
        root = self.s.st.hash
        self.run_command({'action': 'fixture', 'name': 'e1'})
        self.assertEqual(self.s.head, 1)
        self.assertNotEqual(self.s.st.hash, root)
        self.assertEqual(self.w.w['orbit'], 33)


if __name__ == '__main__':
    unittest.main(verbosity=2)
