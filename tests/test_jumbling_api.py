"""JI.52: the public J1 API module, and a source scan of the harness bindings.

The scan reads the bindings' source with `ast` and finds no private member of J1 or of the theory code.
The API tests call each function of `research/jumbling/sim/api.py` once on cheap states.
"""

import sys

sys.dont_write_bytecode = True

import ast
import json
from pathlib import Path
import types
import unittest
import unittest.mock

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / 'research' / 'jumbling'
DIFFERENTIAL = JUMBLING / 'differential'
sys.path.insert(0, str(JUMBLING))

import numpy as np

import sim
from sim import api
from sim.twists import Twist

# Bindings: the adapters, the candidate and the references, and two lead modules they import. The other
# lead scripts are recorded measurements and keep their private uses.
BINDINGS = sorted(
    list(DIFFERENTIAL.glob('*.py'))
    + list((DIFFERENTIAL / 'candidate').rglob('*.py'))
    + list((DIFFERENTIAL / 'references').rglob('*.py'))
    + [DIFFERENTIAL / 'leads' / 'census.py', DIFFERENTIAL / 'leads' / 'x5_release.py'])

_GETTERS = ('getattr', 'setattr', 'hasattr', 'delattr')


def _private(name):
    """A single leading underscore (not a dunder), or the instance dictionary."""
    return name == '__dict__' or (name.startswith('_') and not name.startswith('__'))


def _own_module(source_tree):
    """Names bound to modules of the differential package, whose private names are the harness's own."""
    names = set()
    for node in ast.walk(source_tree):
        if isinstance(node, ast.ImportFrom) and (node.level > 0 or (node.module or '').split('.')[0] == 'differential'):
            names.update(alias.asname or alias.name for alias in node.names)
        elif isinstance(node, ast.Import):
            names.update(alias.asname or alias.name.split('.')[0]
                         for alias in node.names if alias.name.split('.')[0] == 'differential')
    return names


def scan_source(source, filename='<source>'):
    """Return (line, text) findings: attribute access to a private name of an object that is not
    self, cls, super() or a differential module, and `from <outside module> import _name`."""
    tree = ast.parse(source, filename)
    own = _own_module(tree)
    found = []

    def allowed(value):
        if isinstance(value, ast.Name):
            return value.id in ('self', 'cls') or value.id in own
        return (isinstance(value, ast.Call) and isinstance(value.func, ast.Name)
                and value.func.id == 'super')

    for node in ast.walk(tree):
        if isinstance(node, ast.Attribute) and _private(node.attr):
            if node.attr == '__dict__' and isinstance(node.value, ast.Name) and node.value.id in ('self', 'cls'):
                continue
            if not allowed(node.value):
                found.append((node.lineno, f'.{node.attr}'))
        elif (isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id in _GETTERS
              and len(node.args) >= 2 and isinstance(node.args[1], ast.Constant)
              and isinstance(node.args[1].value, str) and _private(node.args[1].value)
              and not allowed(node.args[0])):
            found.append((node.lineno, f'{node.func.id}(..., {node.args[1].value!r})'))
        elif isinstance(node, ast.ImportFrom) and node.level == 0 and (node.module or '').split('.')[0] != 'differential':
            for alias in node.names:
                if _private(alias.name):
                    found.append((node.lineno, f'from {node.module} import {alias.name}'))
    return sorted(found)


class BindingScan(unittest.TestCase):
    def test_scanner_finds_planted_private_access(self):
        planted = (
            'import sim\n'
            'from sim.state import _hidden\n'                 # 2: private import
            'def f(state):\n'
            '    a = state._poses\n'                          # 4: private attribute of a J1 object
            '    b = state.ctx._intern\n'                     # 5: nested object
            '    c = getattr(state, "_lookup")\n'             # 6: string access
            '    state.__dict__.update({})\n'                 # 7: instance dictionary
            '    return a, b, c\n')
        self.assertEqual([line for line, _ in scan_source(planted)], [2, 4, 5, 6, 7])

    def test_scanner_allows_the_harness_own_names(self):
        allowed = (
            'import sim\n'
            'from . import nu_refs\n'
            'from differential.leads import census\n'
            'from .harness import _own_helper\n'
            'from sim.kplus import matrix_json as _alias\n'
            'class A(sim.State):\n'
            '    def f(self):\n'
            '        self._state = 1\n'
            '        super()._commit(1)\n'
            '        census._thing()\n'
            '        return nu_refs._runtime(), self.__dict__\n'
            'class B:\n'
            '    @classmethod\n'
            '    def g(cls):\n'
            '        return cls._x, __file__.__len__()\n')
        self.assertEqual(scan_source(allowed), [])

    def test_the_bindings_use_no_private_member_of_j1_or_the_theory_code(self):
        self.assertGreaterEqual(len(BINDINGS), 20)
        for path in BINDINGS:
            self.assertTrue(path.is_file(), path)
            with self.subTest(path=str(path.relative_to(ROOT))):
                self.assertEqual(scan_source(path.read_text(encoding='utf-8'), str(path)), [])


class ApiContracts(unittest.TestCase):
    """Each api.py function once, on cheap I_a states (the contracts of JI.52's table)."""

    @classmethod
    def setUpClass(cls):
        cls.ctx = sim.get_context()
        cls.records = json.loads((JUMBLING / 'fixtures' / 'wj-I_a.json').read_text(encoding='utf-8'))['journal']['records']
        cls.state, cls.menu = api.fixture_state(cls.ctx, 'I_a', 4)

    def twist(self, index):
        return Twist.from_record(self.ctx, self.records[index])

    def test_apply_with_inside_returns_the_committed_set(self):
        state = sim.State(self.ctx, menu=self.menu)
        reference = sim.State(self.ctx, menu=self.menu)
        outcome, inside = api.apply_with_inside(state, self.twist(0))
        self.assertTrue(outcome.applied)
        self.assertTrue(reference.apply(self.twist(0)).applied)
        self.assertEqual(state.digest(), reference.digest())
        self.assertEqual(inside.tolist(), np.flatnonzero(state.pose_id != 0).tolist())
        self.assertEqual(int(inside.size), outcome.moved)
        self.assertNotIn('_commit', state.__dict__)           # the capture is removed again

    def test_a_refusal_returns_the_empty_set_and_changes_nothing(self):
        state = sim.State(self.ctx, menu=self.menu)
        before = (state.digest(), state.snapshot())
        outcome, inside = api.apply_with_inside(state, sim.plane(self.ctx, 0, 13, degrees=10))
        self.assertFalse(outcome.applied)
        self.assertEqual(inside.size, 0)
        self.assertEqual((state.digest(), state.snapshot()), before)
        self.assertNotIn('_commit', state.__dict__)

    def test_a_failing_commit_restores_the_state_and_the_hook(self):
        state = sim.State(self.ctx, menu=self.menu)
        with unittest.mock.patch.object(sim.State, '_commit', side_effect=RuntimeError('commit failed')):
            with self.assertRaises(RuntimeError):
                api.apply_with_inside(state, self.twist(0))
        self.assertNotIn('_commit', state.__dict__)

    def test_pose_table_reads(self):
        state = self.state
        entries = api.pose_entries(state)
        self.assertIsInstance(entries, tuple)
        self.assertEqual((entries[0].kidx, entries[0].matrix, entries[0].key), (0, None, ('K', 0)))
        self.assertTrue(all(e.kidx >= 0 or e.matrix is not None for e in entries))
        self.assertTrue(any(e.kidx < 0 for e in entries))
        kplus = api.piece_kplus(state)
        self.assertEqual((kplus >= 0).tolist(), state.lattice_flags().tolist())
        for piece in (0, int(np.flatnonzero(~state.lattice_flags())[0])):
            pid = int(state.pose_id[piece])
            self.assertEqual(api.pose_matrix(state, pid), state.pose(piece))
            self.assertEqual(entries[pid].kidx, int(kplus[piece]))

    def test_groups_anchors_evaluation_and_certificate(self):
        from sim.kernel import Evaluator, pullback_form
        from sim.state import Classification
        state, data, reg = self.state, self.state.ctx.data, self.state.ctx.regions
        off = np.flatnonzero(~state.lattice_flags())
        groups = api.pose_groups(state, off)
        self.assertEqual(sorted(np.concatenate([m for _, m in groups]).tolist()), off.tolist())
        pid, members = groups[0]
        parts = api.anchors(state, pid, members)
        self.assertEqual(sorted(np.concatenate([m for _, m in parts]).tolist()), sorted(members.tolist()))
        anchor, _ = parts[0]
        cap = 0
        evaluator = Evaluator(pullback_form(api.pose_matrix(state, pid), data.N[cap]))
        cl = Classification(cap)
        result = api.evaluate(state, evaluator, reg.cap_vform(anchor), lambda: reg.cap_frows(anchor), cl, None)
        again = Evaluator(pullback_form(api.pose_matrix(state, pid), data.N[cap])).classify(reg.cap_vform(anchor), None)
        self.assertEqual(result, again)
        self.assertEqual(cl.exact_evaluations, result[1])
        # A blocked cap's certificate is reproduced by the same call from its own vertex pair.
        for cap, row in state.survey(range(0, 600, 60)).items():
            if row['status'] == 'blocked':
                cert = row['certificate']
                again = api.straddle_certificate(state, cert['piece'], int(state.pose_id[cert['piece']]),
                                                 cap, cert['vertex_below'], cert['vertex_above'])
                self.assertEqual(again, cert)
                break
        else:
            self.fail('no blocked cap among the sampled grips')

    def test_copy_state_is_independent_and_adopt_hands_over(self):
        state = self.state
        before = (state.digest(), state.snapshot())
        trial = api.copy_state(state)
        self.assertIsNot(trial.pose_id, state.pose_id)
        self.assertIs(trial.ctx, state.ctx)
        self.assertIs(trial.menu, state.menu)
        self.assertEqual(trial.journal, state.journal)
        self.assertIsNot(trial.journal, state.journal)
        outcome = trial.apply(self.twist(4))
        self.assertTrue(outcome.applied)
        self.assertEqual((state.digest(), state.snapshot()), before)      # the original is untouched
        self.assertNotEqual(trial.digest(), state.digest())
        # A refusal on the copy is simply not adopted: the original stays as it was.
        failed = api.copy_state(state)
        self.assertFalse(failed.apply(sim.plane(self.ctx, 0, 13, degrees=10)).applied)
        self.assertEqual(state.snapshot(), before[1])
        original = api.copy_state(state)
        self.assertIs(api.adopt(original, trial), original)
        self.assertEqual((original.digest(), original.snapshot()), (trial.digest(), trial.snapshot()))
        self.assertIs(original.pose_id, trial.pose_id)

    def test_from_pose_table_round_trips_a_state(self):
        state = self.state
        entries = api.pose_entries(state)
        rebuilt = api.from_pose_table(self.ctx, entries, state.pose_id, state.menu)
        self.assertEqual(rebuilt.digest(), state.digest())
        self.assertEqual(rebuilt.journal, [])
        self.assertEqual(api.pose_entries(rebuilt), entries)
        self.assertEqual(rebuilt.pose_id.dtype, np.int32)
        # Exact matrices alone (a K+ matrix is interned as its K+ pose), and an unused entry:
        matrices = [(-1, api.pose_matrix(state, pid)) for pid in range(len(entries))]
        self.assertEqual(api.from_pose_table(self.ctx, matrices, state.pose_id, state.menu).digest(), state.digest())
        unused = next(k for k in range(1, 100) if k not in {e.kidx for e in entries})
        padded = list(entries) + [(unused, None)]
        self.assertEqual(len(api.pose_entries(api.from_pose_table(self.ctx, padded, state.pose_id))), len(entries))
        kept = api.from_pose_table(self.ctx, padded, state.pose_id, compact=False)
        self.assertEqual(len(api.pose_entries(kept)), len(entries) + 1)
        self.assertEqual(kept.digest(), state.digest())
        with self.assertRaises(ValueError):
            api.from_pose_table(self.ctx, entries, state.pose_id[:10])
        with self.assertRaises(ValueError):
            api.from_pose_table(self.ctx, entries[:1], state.pose_id)

    def test_seam_restorer_reads_the_supplied_sets(self):
        inside = {7: np.array([1, 2, 3], np.int64), 8: None}
        restorer = api.seam_restorer(self.state, self.menu, None, inside)
        self.assertIs(restorer.inside(7), inside[7])
        self.assertIsNone(restorer.inside(8))
        marker = object()
        self.assertIs(api.seam_restorer(self.state, self.menu, marker).domains(), marker)

    def test_fixture_state_replays_a_prefix_with_an_empty_journal(self):
        state, menu = api.fixture_state(self.ctx, 'I_a', 3)
        self.assertEqual(state.journal, [])
        self.assertIs(state.menu, menu)
        replayed = sim.State.replay(self.records[:3], self.ctx, menu=menu)
        self.assertEqual(state.digest(), replayed.digest())
        refused = types.SimpleNamespace(applied=False)
        with unittest.mock.patch.object(sim.State, 'apply', return_value=refused):
            with self.assertRaises(RuntimeError):
                api.fixture_state(self.ctx, 'I_a', 1)


if __name__ == '__main__':
    unittest.main()
