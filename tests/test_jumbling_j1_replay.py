"""J1 fixture references and real-engine checks of the differential harness.

Set JUMBLING_FULL_REPLAY=1 for complete paired W-J replays and end surveys.
All inputs are read-only; no bytecode or generated evidence is written here.
Run directly, or through unittest alongside the NumPy-free harness checks;
the latter runs these tests in one isolated worker so those checks keep their
import proof.
"""

import sys

sys.dont_write_bytecode = True

from copy import deepcopy
from dataclasses import replace
import hashlib
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[1]
JUMBLING = REPO / 'research' / 'jumbling'
sys.path.insert(0, str(JUMBLING))

from differential import harness as H
from differential import independence as I
from differential import menu_order as M

FULL = os.environ.get('JUMBLING_FULL_REPLAY') == '1'
_IN_PROCESS = __name__ == '__main__'
CTX = None
FIXTURES = {}
ACCEPTANCE = None


def setUpModule():
    global CTX, FIXTURES, ACCEPTANCE, np, sim, J1Engine
    try:
        import numpy as np
    except ImportError as error:
        raise unittest.SkipTest('NumPy is required for the J1 adapter and replays') from error
    import sim
    from differential.j1_adapter import J1Engine
    CTX = sim.get_context()
    FIXTURES = {name: json.loads((JUMBLING / 'fixtures' / f'wj-{name}.json').read_text(
        encoding='utf-8')) for name in ('S4', 'I_a', 'I_b')}
    ACCEPTANCE = json.loads((JUMBLING / 'sim' / 'acceptance.json').read_text(encoding='utf-8'))


def changed_hash(value):
    return ('1' if value[0] == '0' else '0') + value[1:]


class CountingJ1:
    """Count adapter requests so a mismatch cannot silently replay later steps."""

    def __init__(self, name):
        self.engine = J1Engine(CTX, name=name)
        self.name = name
        self.calls = []
        self.previous = None
        self.surveys = {}

    def start(self, menu):
        self.calls = []
        self.surveys = {}
        self.previous = self.engine.start(menu)
        return self.previous

    def apply(self, step):
        self.calls.append(len(self.calls) + 1)
        self.previous = self.engine.apply(step)
        return self.previous

    def survey(self, grips):
        result = self.engine.survey(grips)
        self.surveys[len(self.calls)] = result
        return result


class CheckedJ1(CountingJ1):
    """Check repository record counts/digests on each side of a paired replay."""

    def __init__(self, case, name, *, digests=None, heights=None):
        super().__init__(name)
        self.case = case
        self.digests = digests
        self.heights = heights

    def start(self, menu):
        result = super().start(menu)
        self.case.assertEqual(result.status, 'start')
        if self.digests is not None:
            self.case.assertEqual(result.digest, self.digests[0], f'{self.name}: step 0')
        return result

    def apply(self, step):
        result = super().apply(step)
        index = len(self.calls)
        where = f'{self.name}: step {index}'
        self.case.assertEqual(result.status, 'admissible', where)
        self.case.assertEqual(result.moved, step['record']['moved'], where)
        if self.digests is not None:
            self.case.assertEqual(result.digest, self.digests[index], where)
        if self.heights is not None and index % 2 == 0:
            centre = int(CTX.data.centre_pieces[CTX.data.centre_poles == 0][0])
            height = max(max(abs(x.a), abs(x.b), x.d)
                         for row in self.engine.state.pose(centre) for x in row)
            self.case.assertEqual(height, self.heights[index // 2 - 1], where)
        return result


class FaultJ1(CountingJ1):
    def __init__(self, fault, at):
        super().__init__('fault-' + fault)
        self.fault = fault
        self.at = at

    def apply(self, step):
        index = len(self.calls) + 1
        if index == self.at and self.fault == 'skip':
            self.calls.append(index)
            return replace(self.previous, status='admissible')
        result = super().apply(step)
        if index != self.at:
            return result
        if self.fault == 'digest':
            return replace(result, digest=changed_hash(result.digest))
        if self.fault == 'moved_set':
            return replace(result, moved_set=changed_hash(result.moved_set))
        if self.fault == 'refusal':
            return replace(result, status='blocked', moved=0, moved_set=H.EMPTY_MOVED_SET,
                           digest=changed_hash(result.digest))
        raise AssertionError('unknown planted fault')


class J1AdapterTests(unittest.TestCase):
    def test_commit_inside_set_and_lattice_encoding(self):
        engine = J1Engine(name='observed-J1')
        self.assertEqual(engine.name, 'observed-J1')
        self.assertEqual(J1Engine(CTX).name, 'J1')
        start = engine.start(None)
        self.assertEqual(start.digest, H.EMPTY_MOVED_SET)
        self.assertEqual(start.moved, 0)
        self.assertEqual(start.moved_set, H.EMPTY_MOVED_SET)
        self.assertEqual(start.lattice, hashlib.sha256(bytes([1]) * len(engine.state.pose_id)).hexdigest())
        step = H.load_steps(ACCEPTANCE['grouped_per_piece']['E2']['journal']).steps[0]
        before = engine.state.pose_id.copy()
        with patch.object(engine.state, 'classify', wraps=engine.state.classify) as classify:
            result = engine.apply(step)
        self.assertEqual(classify.call_count, 1)
        self.assertEqual(result.status, 'admissible')
        self.assertEqual(result.moved, step['record']['moved'])
        inside = np.flatnonzero(engine.state.pose_id != before).tolist()
        self.assertEqual(result.moved_set, H.moved_set_hash(inside))
        self.assertEqual(result.digest, engine.state.digest())
        self.assertEqual(result.lattice, hashlib.sha256(
            engine.state.lattice_flags().astype(np.uint8).tobytes()).hexdigest())
        with self.assertRaises(AttributeError):
            engine.state = sim.State(CTX)
        self.assertEqual(engine.start(None), start)

    def test_outside_menu_record_is_invalid_and_changes_nothing(self):
        engine = J1Engine(CTX)
        engine.start(FIXTURES['I_a']['menu'])
        before = engine.apply(H.load_steps(FIXTURES['I_a']).steps[0])
        snapshot = engine.state.snapshot()
        twist = sim.cayley(CTX, 0, [1, 2, 3])
        self.assertFalse(engine.state.menu.contains(twist))
        refused = engine.apply({'kind': 'record', 'record': twist.record()})
        self.assertEqual(refused, replace(before, status='invalid', moved=0, moved_set=H.EMPTY_MOVED_SET))
        self.assertEqual(engine.state.snapshot(), snapshot)
        self.assertEqual(engine.state.digest(), before.digest)
        self.assertEqual(hashlib.sha256(engine.state.lattice_flags().astype(np.uint8).tobytes()).hexdigest(),
                         before.lattice)

    def test_bad_record_reconstruction_is_invalid(self):
        engine = J1Engine(CTX)
        before = engine.start(None)
        valid = ACCEPTANCE['grouped_per_piece']['E2']['journal']['records'][0]
        bad_matrix = deepcopy(valid)
        bad_matrix['matrix'][0][0][2] = 0  # ValueError in the exact JSON reader
        bad_flag = deepcopy(valid)
        bad_flag['retained'] = not valid['retained']  # TwistError in from_record
        for record in (bad_matrix, bad_flag):
            with self.subTest(record=record is bad_matrix):
                snapshot = engine.state.snapshot()
                result = engine.apply({'kind': 'record', 'record': record})
                self.assertEqual(result, replace(before, status='invalid'))
                self.assertEqual(engine.state.snapshot(), snapshot)

    def test_letter_requires_started_menu(self):
        engine = J1Engine(CTX)
        with self.assertRaises(H.HarnessInputError):
            engine.apply({'kind': 'letter', 'grip': 0, 'item': 0})
        engine.start(None)
        with self.assertRaises(H.HarnessInputError):
            engine.apply({'kind': 'letter', 'grip': 0, 'item': 0})

    def test_swapped_menu_keeps_identity_but_changes_order(self):
        record = deepcopy(FIXTURES['I_a']['menu'])
        self.assertEqual(M.order_digest(record), M.I_A_ORDER_DIGEST)
        record['base'][0], record['base'][1] = record['base'][1], record['base'][0]
        self.assertEqual(sim.TwistMenu.from_record(CTX, record).identity, record['identity'])
        self.assertNotEqual(M.order_digest(record), M.I_A_ORDER_DIGEST)
        with self.assertRaises(M.MenuOrderError):
            M.check_menu(record, order=M.I_A_ORDER_DIGEST)
        # Loading a deliberately reordered menu is valid when no signed order is cited.
        J1Engine(CTX).start(record)

    def test_duplicate_menu_item_is_rejected(self):
        record = deepcopy(FIXTURES['I_a']['menu'])
        record['base'].append(deepcopy(record['base'][0]))
        with self.assertRaisesRegex(H.HarnessInputError, 'duplicate'):
            J1Engine(CTX).start(record)

    def test_adapter_checks_loaded_item_order(self):
        record = FIXTURES['I_a']['menu']
        swapped = deepcopy(record)
        swapped['base'][0], swapped['base'][1] = swapped['base'][1], swapped['base'][0]
        loaded = sim.TwistMenu.from_record(CTX, swapped)
        with patch.object(sim.TwistMenu, 'from_record', return_value=loaded):
            with self.assertRaisesRegex(H.HarnessInputError, 'order'):
                J1Engine(CTX).start(record)

    def test_survey_returns_only_requested_letters(self):
        engine = J1Engine(CTX)
        engine.start(None)
        self.assertEqual(engine.survey([599, 0, 17]), {599: 'a', 0: 'a', 17: 'a'})
        self.assertEqual(engine.survey([]), {})

    def test_import_independence_includes_adapter(self):
        paths = I.harness_files()
        self.assertIn(JUMBLING / 'differential' / 'j1_adapter.py', paths)
        self.assertEqual(I.scan_paths(paths, I.HARNESS_POLICY), [])


class WJReplayTests(unittest.TestCase):
    def test_paired_fixture_replays(self):
        for name, fixture in FIXTURES.items():
            with self.subTest(fixture=name, full=FULL):
                steps = H.load_steps(fixture)
                if not FULL:
                    steps = replace(steps, steps=steps.steps[:60])
                reference, candidate = CheckedJ1(self, name + '-reference'), CheckedJ1(self, name + '-candidate')
                last = len(steps.steps)
                report = H.compare(reference, candidate, steps, survey={last} if FULL else None)
                self.assertTrue(report.passed, report.as_dict())
                self.assertEqual(report.steps_compared, last + 1)
                self.assertEqual(report.surveys_compared, int(FULL))
                if FULL:
                    for engine in (reference, candidate):
                        self.assertEqual(''.join(engine.surveys[last][g] for g in range(600)),
                                         fixture['end_survey']['status'])

    def test_i_a_prefix_surveys_all_grips(self):
        steps = H.load_steps(FIXTURES['I_a'])
        steps = replace(steps, steps=steps.steps[:30])
        report = H.compare(CheckedJ1(self, 'reference'), CheckedJ1(self, 'candidate'),
                           steps, survey={0, 30})
        self.assertTrue(report.passed, report.as_dict())
        self.assertEqual(report.steps_compared, 31)
        self.assertEqual(report.surveys_compared, 2)

    def test_single_replays_against_stage_sweep_and_survey_references(self):
        for name, fixture in FIXTURES.items():
            with self.subTest(fixture=name, full=FULL):
                steps = H.load_steps(fixture)
                engine = J1Engine(CTX)
                result = engine.start(steps.menu)
                self.assertEqual(result.digest, fixture['stages']['start']['digest'])
                limit = len(steps.steps) if name == 'I_a' or FULL else fixture['stages']['mid']['records']
                marks = {stage['records']: stage['digest'] for stage in fixture['stages'].values()
                         if 0 < stage['records'] <= limit}
                seen = set()
                sweep = fixture['sweep']
                for index, step in enumerate(steps.steps[:limit]):
                    where = f'{name}: step {index + 1}'
                    if index == sweep['record']:
                        self.assertEqual(result.digest, sweep['before']['digest'], where)
                    result = engine.apply(step)
                    self.assertEqual(result.status, 'admissible', where)
                    self.assertEqual(result.moved, step['record']['moved'], where)
                    if index + 1 in marks:
                        self.assertEqual(result.digest, marks[index + 1], where)
                        seen.add(index + 1)
                    if index == sweep['record']:
                        self.assertEqual(result.digest, sweep['after']['digest'], where)
                        self.assertEqual(result.moved, sweep['moved'], where)
                        self.assertEqual(result.moved_set, sweep['moved_sha256'], where)
                self.assertEqual(seen, set(marks))
                if limit == len(steps.steps):
                    self.assertLess(sweep['record'], limit)
                    survey = engine.survey(range(600))
                    self.assertEqual(''.join(survey[g] for g in range(600)), fixture['end_survey']['status'])


class J1JournalReplayTests(unittest.TestCase):
    def test_four_free_mode_acceptance_journals(self):
        for name in ('E2', 'E3', 'random_0', 'random_1'):
            with self.subTest(journal=name):
                steps = H.load_steps(ACCEPTANCE['grouped_per_piece'][name]['journal'])
                self.assertIsNone(steps.menu)
                report = H.compare(CheckedJ1(self, name + '-reference'),
                                   CheckedJ1(self, name + '-candidate'), steps)
                self.assertTrue(report.passed, report.as_dict())
                self.assertEqual(report.steps_compared, len(steps.steps) + 1)

    def test_negative_control_digests_and_entry_heights(self):
        recorded = ACCEPTANCE['negative_control']
        steps = H.load_steps(recorded['journal'], menu=recorded['menu'])
        self.assertEqual(len(steps.steps), 24)
        self.assertEqual(len(recorded['digests']), 25)
        self.assertEqual(len(set(recorded['digests'])), 25)
        engines = [CheckedJ1(self, name, digests=recorded['digests'],
                             heights=recorded['cap_0_pose_entry_heights'])
                   for name in ('reference', 'candidate')]
        report = H.compare(*engines, steps)
        self.assertTrue(report.passed, report.as_dict())
        self.assertEqual(report.steps_compared, 25)


class J1PlantedFaultTests(unittest.TestCase):
    def test_candidate_faults_stop_at_first_bad_step(self):
        steps = H.load_steps(FIXTURES['S4'])
        steps = replace(steps, steps=steps.steps[:60])
        at = 5  # Different moved count from step 4, so a skipped apply fails at 'moved'.
        self.assertNotEqual(steps.steps[at - 1]['record']['moved'], steps.steps[at - 2]['record']['moved'])
        for fault, field in (('digest', 'digest'), ('moved_set', 'moved_set'),
                             ('skip', 'moved'), ('refusal', 'refusal_changed_state')):
            with self.subTest(fault=fault):
                reference, candidate = CountingJ1('reference'), FaultJ1(fault, at)
                report = H.compare(reference, candidate, steps)
                self.assertFalse(report.passed)
                self.assertEqual(report.first_mismatch.step, at)
                self.assertEqual(report.first_mismatch.step_index_in_input, at - 1)
                self.assertEqual(report.first_mismatch.field, field)
                self.assertEqual(report.steps_compared, at + 1)
                self.assertEqual(reference.calls, list(range(1, at + 1)))
                self.assertEqual(candidate.calls, reference.calls)
                if fault == 'skip':
                    self.assertEqual(len(candidate.engine.state.journal), at - 1)

    def test_reference_refusal_violation_is_a_contract_error(self):
        steps = H.load_steps(FIXTURES['S4'])
        steps = replace(steps, steps=steps.steps[:60])
        reference, candidate = FaultJ1('refusal', 5), CountingJ1('candidate')
        with self.assertRaisesRegex(H.ReferenceContractError, 'step 5: refused step changed'):
            H.compare(reference, candidate, steps)
        self.assertEqual(reference.calls, [1, 2, 3, 4, 5])
        self.assertEqual(candidate.calls, reference.calls)


def load_tests(loader, tests, pattern):
    if _IN_PROCESS:
        return tests
    from tests import isolated_numpy
    return isolated_numpy.suite('test_jumbling_j1_replay', 'J1 adapter and replays in an isolated NumPy process',
                                'NumPy is required for the J1 adapter and replays')


if __name__ == '__main__':
    unittest.main()
