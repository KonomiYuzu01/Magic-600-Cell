"""A failed preference save must not split the workspace orbit from the session orbit (screening C-02)."""
from pathlib import Path
import json, sys, tempfile, threading, unittest
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / 'work/experiments/magic600-04'))
from core import Model
from session import Session
import adapter

MODEL = Model()


class OrbitSwitchTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='c600-orbit-switch-')
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name) / 'session'
        self.s = Session(MODEL, self.path)
        self.w = adapter.Workbench(self.s, threading.RLock())
        self.run_command({'action': 'orbit', 'orbit': 33})

    def tearDown(self):
        self.s.close()

    def run_command(self, body):
        return self.w.command(body, response_snapshot=False)

    def orbits(self):
        """Live workspace, session preferences, stored preferences, stored workspace, reopened workspace."""
        stored = json.loads(self.s._get('prefs'))
        values = [self.w.w['orbit'], self.s.prefs['orbit'], stored['orbit'], stored['layout']['magic600_experiment']['orbit']]
        self.s.close()
        self.s = Session(MODEL, self.path)
        reopened = self.reopened = adapter.Workbench(self.s, threading.RLock())
        return values + [reopened.w['orbit'], self.s.prefs['orbit']]

    def fail_save(self, when):
        original, calls = self.s.save_prefs, []
        def save_prefs(changes):
            calls.append(changes)
            if when(changes, len(calls)):
                raise OSError('injected preference-save failure before writing')
            return original(changes)
        self.s.save_prefs = save_prefs

    def fail_status_once(self, skip=0):
        original, state = self.s.status, {'calls': 0}
        def status():
            state['calls'] += 1
            if state['calls'] == skip + 1:
                raise OSError('injected status failure after the write')
            return original()
        self.s.status = status

    def test_failure_before_the_write_keeps_everything(self):
        before = json.dumps(self.w.w, sort_keys=True)
        self.fail_save(lambda changes, n: 'orbit' in changes)
        with self.assertRaisesRegex(OSError, 'before writing'):
            self.run_command({'action': 'orbit', 'orbit': 0})
        self.assertEqual(json.dumps(self.w.w, sort_keys=True), before)
        self.assertEqual(self.orbits(), [33] * 6)

    def test_status_failure_after_the_write_adopts_the_new_orbit(self):
        self.fail_status_once()
        result = self.run_command({'action': 'orbit', 'orbit': 0})
        self.assertIn('reading their result failed', result['warning'])
        self.assertEqual(self.orbits(), [0] * 6)

    def test_later_workspace_save_failure_keeps_orbits_together(self):
        self.fail_save(lambda changes, n: n == 2)  # the combined orbit write succeeds; the workspace save fails
        with self.assertRaisesRegex(OSError, 'before writing'):
            self.run_command({'action': 'orbit', 'orbit': 0})
        self.assertEqual(self.orbits(), [0] * 6)

    def test_fixture_failure_before_the_write_keeps_orbits_together(self):
        self.run_command({'action': 'orbit', 'orbit': 0})
        self.fail_save(lambda changes, n: 'rules' in changes)
        with self.assertRaisesRegex(OSError, 'before writing'):
            self.run_command({'action': 'fixture', 'name': 'e1'})
        self.assertEqual(self.orbits(), [0] * 6)

    def test_fixture_status_failure_adopts_the_fixture_orbit(self):
        self.run_command({'action': 'orbit', 'orbit': 0})
        self.fail_status_once(skip=1)  # the fixture's own commit reads status first
        result = self.run_command({'action': 'fixture', 'name': 'e1'})
        self.assertIn('reading their result failed', result['warning'])
        self.assertEqual(self.orbits(), [33] * 6)

    def pin_next_in_orbit_0(self):
        identity = next(i for i in range(MODEL.np) if MODEL.oid[i] == 0)
        self.run_command({'action': 'next-pin', 'identity': identity})
        return identity

    def workspaces(self):
        """Live, stored and reopened (next, current, orbit), read before orbits() reopens the session."""
        pick = lambda w: (w['next'], w['current'], w['orbit'])
        stored = json.loads(self.s._get('prefs'))['layout']['magic600_experiment']
        return [pick(self.w.w), pick(stored)]

    def test_next_activation_failure_before_the_write_keeps_next_pinned(self):
        identity = self.pin_next_in_orbit_0()
        self.fail_save(lambda changes, n: 'orbit' in changes)
        with self.assertRaisesRegex(OSError, 'before writing'):
            self.run_command({'action': 'next-activate'})
        pinned = (dict(identity=identity, target=identity), None, 33)
        self.assertEqual(self.workspaces(), [pinned, pinned])
        self.assertEqual(self.orbits(), [33] * 6)
        self.assertEqual(self.reopened.w['next'], dict(identity=identity, target=identity))

    def test_next_activation_is_stored_complete_when_the_final_save_fails(self):
        identity = self.pin_next_in_orbit_0()
        self.fail_save(lambda changes, n: n == 2)  # the combined orbit write succeeds; the workspace save fails
        with self.assertRaisesRegex(OSError, 'before writing'):
            self.run_command({'action': 'next-activate'})
        activated = (None, identity, 0)
        self.assertEqual(self.workspaces(), [activated, activated])
        self.assertEqual(self.orbits(), [0] * 6)
        self.assertEqual((self.reopened.w['next'], self.reopened.w['current'], self.reopened.w['target']), (None, identity, identity))


if __name__ == '__main__':
    unittest.main(verbosity=2)
