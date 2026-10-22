"""NumPy-free EN2-A acceptance; generated files belong only in tempfile."""

import sys

sys.dont_write_bytecode = True

from copy import deepcopy
from dataclasses import replace
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[1]
JUMBLING = REPO / 'research' / 'jumbling'
sys.path.insert(0, str(JUMBLING))

from differential import harness as H
from differential import independence as I
from differential import menu_order as M


def fixture(name='I_a'):
    return json.loads((JUMBLING / 'fixtures' / f'wj-{name}.json').read_text(encoding='utf-8'))


def changed_hash(value):
    return ('1' if value[0] == '0' else '0') + value[1:]


def toy_matrix(turn):
    """Four exact rotations for a six-piece synthetic puzzle."""
    cosine, sine = ((1, 0), (0, 1), (-1, 0), (0, -1))[turn]
    rows = ((cosine, -sine, 0, 0), (sine, cosine, 0, 0), (0, 0, 1, 0), (0, 0, 0, 1))
    return [[[entry, 0, 1] for entry in row] for row in rows]


def toy_menu():
    menu = {'base': [{'label': str(i), 'matrix': toy_matrix(i)} for i in range(4)]}
    menu['identity'] = M.menu_identity(menu)
    return menu


def toy_steps():
    return H.load_steps({'format': 'jumbling-journal-1', 'records': [
        {'grip': 1, 'item': 1}, {'grip': 4, 'item': 3},
        {'grip': 2, 'item': 0}, {'grip': 5, 'item': 2},
        {'grip': 3, 'item': 1, 'outcome': 'blocked'},
        {'grip': 4, 'item': 3, 'outcome': 'invalid'},
        {'grip': 0, 'item': 2, 'outcome': 'uncertain'},
        {'grip': 2, 'item': 1}]}, toy_menu())


class ToyEngine:
    """Deterministic toy state with the prescribed digest encodings.

    Its synthetic lattice subgroup is {0, 2}; this is not a J1 replay.
    """

    def __init__(self, name='toy', *, mutate_inputs=False):
        self.name = name
        self.mutate_inputs = mutate_inputs
        self.apply_calls = []
        self.survey_calls = []
        self.index = 0

    def _observe(self, status, moved=()):
        groups = {}
        for piece, pose in enumerate(self.poses):
            if pose != 0:
                groups.setdefault(pose, []).append(piece)
        pairs = [(json.dumps(toy_matrix(pose)), b''.join(struct.pack('<i', p) for p in ids))
                 for pose, ids in groups.items()]
        digest = hashlib.sha256()
        for key, ids in sorted(pairs):
            digest.update(key.encode('utf-8'))
            digest.update(ids)
        lattice = hashlib.sha256(bytes(int(p in (0, 2)) for p in self.poses)).hexdigest()
        return H.StepResult(status, len(moved), H.moved_set_hash(moved), digest.hexdigest(), lattice)

    def start(self, menu):
        self.menu = menu
        if menu is not None:
            assert '_adapter_seen' not in menu
            if self.mutate_inputs:
                menu['_adapter_seen'] = self.name
        self.poses = [0] * 6
        self.index = 0
        self.apply_calls.clear()
        self.survey_calls.clear()
        return self._observe('start')

    def apply(self, step):
        self.index += 1
        self.apply_calls.append(self.index)
        record = step['record'] if step['kind'] == 'record' else step
        assert '_adapter_seen' not in record
        if self.mutate_inputs:
            record['_adapter_seen'] = self.name
        status = record.get('outcome', 'admissible')
        moved = ()
        if status == 'admissible' and record['item'] != 0:
            moved = tuple(sorted((record['grip'] % 6, (record['grip'] + 1) % 6)))
            for piece in moved:
                self.poses[piece] = (self.poses[piece] + record['item']) % 4
        return self._observe(status, moved)

    def survey(self, grips):
        self.survey_calls.append((self.index, tuple(grips)))
        # Engines can have unrelated private certificates; only these letters leave the adapter.
        self.private_certificate = {'engine': self.name, 'chosen_point': object()}
        return {g: ('a', 'b', 'u')[(g + sum(self.poses)) % 3] for g in grips}


class FaultyEngine:
    """Change one observation at a planted step, retaining a real toy delegate."""

    def __init__(self, engine, at, *, result=None, survey=None):
        self.engine = engine
        self.name = engine.name
        self.at = at
        self.result_fault = result
        self.survey_fault = survey

    def _change(self, value):
        if self.engine.index == self.at and self.result_fault is not None:
            return self.result_fault(value)
        return value

    def start(self, menu):
        return self._change(self.engine.start(menu))

    def apply(self, step):
        return self._change(self.engine.apply(step))

    def survey(self, grips):
        result = self.engine.survey(grips)
        if self.engine.index == self.at and self.survey_fault is not None:
            return self.survey_fault(result)
        return result


class MenuOrderTests(unittest.TestCase):
    def test_fixture_identities(self):
        for name in ('I_a', 'S4', 'I_b'):
            with self.subTest(menu=name):
                doc = fixture(name)
                self.assertEqual(M.menu_identity(doc['menu']), doc['menu']['identity'])
                self.assertEqual(M.menu_identity(doc['menu']), doc['journal']['menu_identity'])
                M.check_menu(doc['menu'], identity=doc['journal']['menu_identity'])

    def test_i_a_constants(self):
        menu = fixture()['menu']
        self.assertEqual(M.menu_identity(menu), M.I_A_IDENTITY)
        self.assertEqual(M.order_digest(menu), M.I_A_ORDER_DIGEST)
        M.check_menu(menu, identity=M.I_A_IDENTITY, order=M.I_A_ORDER_DIGEST)

    def test_swap_changes_only_order(self):
        menu = fixture()['menu']
        swapped = deepcopy(menu)
        swapped['base'][1], swapped['base'][2] = swapped['base'][2], swapped['base'][1]
        self.assertEqual(M.menu_identity(swapped), M.menu_identity(menu))
        self.assertNotEqual(M.order_digest(swapped), M.order_digest(menu))
        with self.assertRaises(M.MenuOrderError):
            M.check_menu(swapped, order=M.I_A_ORDER_DIGEST)

    def test_changed_matrix_fails_identity(self):
        menu = fixture()['menu']
        menu['base'][0]['matrix'][0][0][0] += 1
        with self.assertRaises(M.MenuIdentityError):
            M.check_menu(menu)

    def test_expected_identity_and_record_identity_are_checked(self):
        menu = toy_menu()
        with self.assertRaises(M.MenuIdentityError):
            M.check_menu(menu, identity='0' * 64)
        del menu['identity']
        with self.assertRaises(M.MenuIdentityError):
            M.check_menu(menu)

    def test_inverse_indices(self):
        for name in ('I_a', 'S4', 'I_b'):
            with self.subTest(menu=name):
                menu = fixture(name)['menu']
                inverses = M.inverse_index(menu)
                self.assertEqual(inverses[0], 0)
                self.assertEqual([inverses[i] for i in inverses], list(range(len(inverses))))
                for i, j in enumerate(inverses):
                    matrix = menu['base'][i]['matrix']
                    self.assertEqual(menu['base'][j]['matrix'],
                                     [[matrix[c][r] for c in range(4)] for r in range(4)])

    def test_missing_inverse(self):
        menu = toy_menu()
        menu['base'].pop(3)
        with self.assertRaises(M.MenuRecordError):
            M.inverse_index(menu)

    def test_non_integer_entries_and_denominators(self):
        for entry in ([1.0, 0, 1], [True, 0, 1], [1, False, 1], ['1', 0, 1],
                      [1, 0, 0], [1, 0, -1], [1, 0, True], [1, 0, 1.0]):
            with self.subTest(entry=entry):
                menu = toy_menu()
                menu['base'][0]['matrix'][0][0] = entry
                with self.assertRaises(M.MenuRecordError):
                    M.check_menu(menu)

    def test_malformed_shapes(self):
        for record in (None, {}, {'base': []}, {'base': [None]}, {'base': [{}]},
                       {'base': [{'label': 'bad', 'matrix': []}]}):
            with self.subTest(record=record), self.assertRaises(M.MenuRecordError):
                M.menu_identity(record)
        for matrix in ((), [], [[1] * 4] * 4, [[[0, 0]] * 4] * 4,
                       [[[0, 0, 1, 2]] * 4] * 4, tuple(toy_matrix(0))):
            with self.subTest(matrix=matrix), self.assertRaises(M.MenuRecordError):
                M.item_key(matrix)

    def test_exact_encoding_ignores_labels_and_keeps_triples(self):
        menu = toy_menu()
        renamed = deepcopy(menu)
        renamed['base'][0]['label'] = 'renamed'
        self.assertEqual(M.order_digest(menu), M.order_digest(renamed))
        matrix = toy_matrix(0)
        matrix[0][0] = [2, 0, 2]
        self.assertEqual(M.item_key(matrix), json.dumps(matrix, separators=(',', ':')))
        self.assertNotEqual(M.item_key(matrix), M.item_key(toy_matrix(0)))


class InputTests(unittest.TestCase):
    def test_load_each_fixture_with_identities(self):
        for name, count in (('I_a', 841), ('S4', 640), ('I_b', 885)):
            with self.subTest(menu=name):
                doc = fixture(name)
                loaded = H.load_steps(JUMBLING / 'fixtures' / f'wj-{name}.json')
                self.assertEqual(len(loaded.steps), count)
                self.assertEqual(loaded.menu, doc['menu'])
                self.assertEqual(loaded.menu_identity, doc['journal']['menu_identity'])
                self.assertEqual(loaded.model_identity, doc['journal']['model_identity'])
                self.assertEqual(loaded.contract_revision, doc['journal']['contract_revision'])
                self.assertEqual(loaded.menu_order_digest, M.order_digest(doc['menu']))
                self.assertEqual(loaded.steps[0], {'kind': 'record', 'record': doc['journal']['records'][0]})
                self.assertEqual(loaded.steps[-1]['record'], doc['journal']['records'][-1])

    def test_journal_without_menu_preserves_identities(self):
        doc = fixture()['journal']
        loaded = H.load_steps(doc)
        self.assertIsNone(loaded.menu)
        self.assertIsNone(loaded.menu_order_digest)
        self.assertEqual(loaded.menu_identity, doc['menu_identity'])
        self.assertEqual(loaded.model_identity, doc['model_identity'])
        self.assertEqual(len(loaded.steps), len(doc['records']))

    def test_minimal_journal_and_empty_word(self):
        journal = H.load_steps({'format': 'jumbling-journal-1', 'records': []})
        self.assertEqual(journal.steps, ())
        self.assertIsNone(journal.model_identity)
        self.assertIsNone(journal.menu_identity)
        self.assertIsNone(journal.contract_revision)
        self.assertEqual(H.load_steps({'from': 'solved', 'word': []}, toy_menu()).steps, ())

    def test_h6030_with_menu(self):
        path = JUMBLING / 'restoration/10-x1-unbounded/witnesses/H6030.json'
        loaded = H.load_steps(str(path), fixture()['menu'])
        self.assertEqual(len(loaded.steps), 3156)
        self.assertTrue(all(s['kind'] == 'letter' for s in loaded.steps))
        self.assertEqual(loaded.steps[0], {'kind': 'letter', 'grip': 521, 'item': 35})
        self.assertEqual(loaded.menu_identity, M.I_A_IDENTITY)
        self.assertEqual(loaded.menu_order_digest, M.I_A_ORDER_DIGEST)
        self.assertIsNone(loaded.model_identity)

    def test_h6030_needs_menu(self):
        with self.assertRaises(H.HarnessInputError):
            H.load_steps(JUMBLING / 'restoration/10-x1-unbounded/witnesses/H6030.json')

    def test_unknown_formats(self):
        for source in ({}, [], None, {'format': 'unknown'},
                       {'from': 'elsewhere', 'word': []},
                       {'format': 'unknown', 'from': 'solved', 'word': []}):
            with self.subTest(source=source), self.assertRaises(H.HarnessInputError):
                H.load_steps(source)

    def test_bad_journal_records(self):
        for records in (None, {}, (), [None], [0], [{}], [{'grip': True}],
                        [{'grip': 600}], [{'grip': -1}], [{'grip': 1.0}], [{'grip': '1'}]):
            with self.subTest(records=records), self.assertRaises(H.HarnessInputError):
                H.load_steps({'format': 'jumbling-journal-1', 'records': records})

    def test_bad_letter_shapes_grips_and_indices(self):
        menu = toy_menu()
        for word in (None, {}, (), [None], [[0]], [[0, 1, 2]], [(0, 1)],
                     [[True, 0]], [[600, 0]], [[-1, 0]], [[1.0, 0]],
                     [[0, len(menu['base'])]], [[0, -1]], [[0, True]], [[0, 1.0]]):
            with self.subTest(word=word), self.assertRaises(H.HarnessInputError):
                H.load_steps({'from': 'solved', 'word': word}, menu)

    def test_valid_grip_and_item_boundaries(self):
        loaded = H.load_steps({'from': 'solved', 'word': [[0, 0], [599, 3]]}, toy_menu())
        self.assertEqual(loaded.steps[-1], {'kind': 'letter', 'grip': 599, 'item': 3})

    def test_invalid_metadata(self):
        for key, value in (('model_identity', 'wrong'), ('model_identity', {'asset': True}),
                           ('menu_identity', 'wrong'), ('contract_revision', 2)):
            with self.subTest(field=key), self.assertRaises(H.HarnessInputError):
                H.load_steps({'format': 'jumbling-journal-1', 'records': [], key: value})

    def test_menu_mismatch_and_malformed_fixture(self):
        doc = fixture()
        doc['journal']['menu_identity'] = '0' * 64
        with self.assertRaises(H.HarnessInputError):
            H.load_steps(doc)
        for changes in ({'journal': []}, {'journal': {'format': 'wrong'}}, {'menu': None}):
            doc = fixture()
            doc.update(changes)
            with self.subTest(changes=changes), self.assertRaises(H.HarnessInputError):
                H.load_steps(doc)
        swapped = fixture()['menu']
        swapped['base'][0], swapped['base'][1] = swapped['base'][1], swapped['base'][0]
        with self.assertRaises(H.HarnessInputError):
            H.load_steps(fixture(), swapped)

    def test_load_does_not_alias_input(self):
        doc = fixture()
        original = deepcopy(doc)
        loaded = H.load_steps(doc)
        loaded.steps[0]['record']['grip'] = 0
        loaded.menu['base'][0]['label'] = 'private'
        loaded.model_identity.clear()
        self.assertEqual(doc, original)

    def test_bad_file_inputs_use_temporary_directory(self):
        with tempfile.TemporaryDirectory(prefix='jumbling-input-') as directory:
            root = Path(directory)
            self.assertFalse(root.is_relative_to(REPO))
            path = root / 'bad.json'
            path.write_text('{bad', encoding='utf-8')
            with self.assertRaises(H.HarnessInputError):
                H.load_steps(path)
            with self.assertRaises(H.HarnessInputError):
                H.load_steps(root / 'absent.json')


class ComparisonTests(unittest.TestCase):
    def setUp(self):
        self.steps = toy_steps()

    def assert_stopped(self, reference, candidate, report, at, field):
        self.assertFalse(report.passed)
        self.assertEqual(report.first_mismatch.step, at)
        self.assertEqual(report.first_mismatch.step_index_in_input, None if at == 0 else at - 1)
        self.assertEqual(report.first_mismatch.field, field)
        self.assertEqual(report.steps_compared, at + 1)
        self.assertEqual(reference.apply_calls, list(range(1, at + 1)))
        self.assertEqual(candidate.apply_calls, list(range(1, at + 1)))

    def test_equal_engines_compare_every_step(self):
        reference, candidate = ToyEngine('reference'), ToyEngine('candidate')
        report = H.compare(reference, candidate, self.steps)
        self.assertTrue(report.passed)
        self.assertEqual(report.engines, ('reference', 'candidate'))
        self.assertEqual(report.steps_compared, len(self.steps.steps) + 1)
        self.assertEqual(report.surveys_compared, 0)
        self.assertIsNone(report.first_mismatch)
        self.assertEqual(len(reference.apply_calls), len(self.steps.steps))
        self.assertEqual(reference.apply_calls, candidate.apply_calls)

    def test_all_surveys_default_to_600_grips_and_only_letters(self):
        reference, candidate = ToyEngine('reference'), ToyEngine('candidate')
        report = H.compare(reference, candidate, self.steps, survey='all')
        self.assertTrue(report.passed)
        self.assertEqual(report.surveys_compared, len(self.steps.steps) + 1)
        self.assertEqual(reference.survey_calls[0], (0, tuple(range(600))))
        self.assertNotEqual(reference.private_certificate, candidate.private_certificate)

    def test_selected_surveys_sort_and_deduplicate_grips(self):
        reference, candidate = ToyEngine(), ToyEngine()
        report = H.compare(reference, candidate, self.steps, survey={0, 3, 8}, grips=[599, 7, 3, 7])
        self.assertTrue(report.passed)
        self.assertEqual(report.surveys_compared, 3)
        self.assertEqual(reference.survey_calls, [(i, (3, 7, 599)) for i in (0, 3, 8)])
        self.assertEqual(reference.survey_calls, candidate.survey_calls)

    def test_each_planted_step_field_stops_at_first_difference(self):
        faults = {
            'status': lambda r: replace(r, status='blocked'),
            'moved': lambda r: replace(r, moved=r.moved + 1),
            'moved_set': lambda r: replace(r, moved_set=changed_hash(r.moved_set)),
            'digest': lambda r: replace(r, digest=changed_hash(r.digest)),
            'lattice': lambda r: replace(r, lattice=changed_hash(r.lattice)),
        }
        for field, fault in faults.items():
            with self.subTest(field=field):
                reference, candidate = ToyEngine(), ToyEngine()
                report = H.compare(reference, FaultyEngine(candidate, 3, result=fault), self.steps, survey='all')
                self.assert_stopped(reference, candidate, report, 3, field)
                self.assertEqual(report.surveys_compared, 3)

    def test_planted_grip_difference_is_first_ascending_grip(self):
        def fault(result):
            for grip in (599, 7):
                result[grip] = 'b' if result[grip] == 'a' else 'a'
            return result
        reference, candidate = ToyEngine(), ToyEngine()
        report = H.compare(reference, FaultyEngine(candidate, 3, survey=fault), self.steps,
                           survey='all', grips=[599, 7, 1])
        self.assert_stopped(reference, candidate, report, 3, 'grips')
        self.assertIn('grip 7', report.first_mismatch.detail)
        self.assertEqual(report.surveys_compared, 4)

    def test_field_priority(self):
        for fields in (('moved', 'moved_set', 'digest', 'lattice'),
                       ('moved_set', 'digest', 'lattice'), ('digest', 'lattice')):
            def fault(result):
                values = {field: (1 if field == 'moved' else changed_hash(getattr(result, field)))
                          for field in fields}
                return replace(result, **values)
            with self.subTest(fields=fields):
                report = H.compare(ToyEngine(), FaultyEngine(ToyEngine(), 3, result=fault), self.steps)
                self.assertEqual(report.first_mismatch.field, fields[0])

    def test_step_zero_difference_applies_no_records(self):
        reference, candidate = ToyEngine(), ToyEngine()
        report = H.compare(reference, FaultyEngine(candidate, 0,
                           result=lambda r: replace(r, digest=changed_hash(r.digest))), self.steps)
        self.assert_stopped(reference, candidate, report, 0, 'digest')

    def test_refused_steps_preserve_each_engine_state(self):
        for field in ('moved', 'moved_set', 'digest', 'lattice'):
            def fault(result):
                value = 1 if field == 'moved' else changed_hash(getattr(result, field))
                return replace(result, **{field: value})
            for at in (5, 6, 7):
                with self.subTest(field=field, step=at):
                    reference, candidate = ToyEngine(), ToyEngine()
                    report = H.compare(reference, FaultyEngine(candidate, at, result=fault), self.steps)
                    self.assert_stopped(reference, candidate, report, at, 'refusal_changed_state')
                    self.assertIn(field, report.first_mismatch.detail)
                    with self.assertRaises(H.ReferenceContractError):
                        H.compare(FaultyEngine(ToyEngine(), at, result=fault), ToyEngine(), self.steps)

    def test_bad_step_results_are_contract_mismatches(self):
        faults = [lambda r: None, lambda r: {'status': r.status}]
        for status in ('wrong', 'start', True, ['admissible']):
            faults.append(lambda r, status=status: replace(r, status=status))
        for moved in (-1, True, 1.0, '0'):
            faults.append(lambda r, moved=moved: replace(r, moved=moved))
        for field in ('moved_set', 'digest', 'lattice'):
            for value in ('f' * 63, 'f' * 65, 'A' * 64, 'g' * 64, None, 0):
                faults.append(lambda r, field=field, value=value: replace(r, **{field: value}))
        for index, fault in enumerate(faults):
            with self.subTest(fault=index):
                reference, candidate = ToyEngine(), ToyEngine()
                report = H.compare(reference, FaultyEngine(candidate, 3, result=fault), self.steps)
                self.assert_stopped(reference, candidate, report, 3, 'contract')
                self.assertTrue(report.first_mismatch.detail)
                json.dumps(report.as_dict())
                with self.assertRaises(H.ReferenceContractError):
                    H.compare(FaultyEngine(ToyEngine(), 3, result=fault), ToyEngine(), self.steps)

    def test_step_zero_requires_start_status(self):
        for status in H.STATUSES:
            with self.subTest(status=status):
                report = H.compare(ToyEngine(), FaultyEngine(ToyEngine(), 0,
                                   result=lambda r: replace(r, status=status)), self.steps)
                self.assertEqual(report.first_mismatch.field, 'contract')
                self.assertEqual(report.first_mismatch.step, 0)

    def test_bad_surveys_are_contract_mismatches(self):
        faults = (
            lambda result: dict(result, **{'extra': 'a'}),
            lambda result: {**result, 42: 'a'},
            lambda result: {g: v for g, v in result.items() if g != 1},
            lambda result: {True: result[1], 7: result[7]},
            lambda result: {**result, 7: 'admissible'},
            lambda result: {**result, 7: {'status': 'a', 'certificate': object()}},
            lambda result: list(result.items()),
        )
        for index, fault in enumerate(faults):
            with self.subTest(fault=index):
                reference, candidate = ToyEngine(), ToyEngine()
                report = H.compare(reference, FaultyEngine(candidate, 3, survey=fault), self.steps,
                                   survey='all', grips=[1, 7])
                self.assert_stopped(reference, candidate, report, 3, 'contract')
                self.assertEqual(report.surveys_compared, 4)
                self.assertTrue(report.first_mismatch.detail)
                json.dumps(report.as_dict())
                with self.assertRaises(H.ReferenceContractError):
                    H.compare(FaultyEngine(ToyEngine(), 3, survey=fault), ToyEngine(), self.steps,
                              survey='all', grips=[1, 7])

    def test_reference_refusal_violation_cannot_be_hidden_by_candidate(self):
        reference = FaultyEngine(ToyEngine(), 5, result=lambda r: replace(r, moved=1))
        candidate = FaultyEngine(ToyEngine(), 5, result=lambda r: None)
        with self.assertRaises(H.ReferenceContractError):
            H.compare(reference, candidate, self.steps)

    def test_report_roundtrips_as_plain_json(self):
        reports = [H.compare(ToyEngine('r'), ToyEngine('c'), self.steps),
                   H.compare(ToyEngine('r'), FaultyEngine(ToyEngine('c'), 5,
                             result=lambda r: replace(r, digest=changed_hash(r.digest))), self.steps)]
        for report in reports:
            with self.subTest(passed=report.passed):
                self.assertEqual(json.loads(json.dumps(report.as_dict())), report.as_dict())
                self.assertEqual(report.as_dict()['engines'], ['r', 'c'])

    def test_mutating_adapter_inputs_are_isolated(self):
        before = deepcopy(self.steps)
        reference = ToyEngine('r', mutate_inputs=True)
        candidate = ToyEngine('c', mutate_inputs=True)
        self.assertTrue(H.compare(reference, candidate, self.steps).passed)
        self.assertEqual(self.steps, before)
        self.assertIsNot(reference.menu, candidate.menu)

    def test_letter_steps_and_empty_replay(self):
        for word in ([], [[0, 1], [599, 3], [5, 0]]):
            loaded = H.load_steps({'from': 'solved', 'word': word}, toy_menu())
            with self.subTest(word=word):
                report = H.compare(ToyEngine(), ToyEngine(), loaded, survey='all')
                self.assertTrue(report.passed)
                self.assertEqual(report.steps_compared, len(word) + 1)

    def test_invalid_comparison_arguments(self):
        for survey in ('bad', [], {True}, {-1}, {len(self.steps.steps) + 1}, {1.0}):
            with self.subTest(survey=survey), self.assertRaises(H.HarnessInputError):
                H.compare(ToyEngine(), ToyEngine(), self.steps, survey=survey)
        for grips in ([True], [-1], [600], [1.0], 1):
            with self.subTest(grips=grips), self.assertRaises(H.HarnessInputError):
                H.compare(ToyEngine(), ToyEngine(), self.steps, survey='all', grips=grips)
        with self.assertRaises(H.HarnessInputError):
            H.compare(ToyEngine(), ToyEngine(), [])

    def test_moved_set_hash_encoding(self):
        self.assertEqual(H.EMPTY_MOVED_SET, hashlib.sha256(b'').hexdigest())
        self.assertEqual(H.moved_set_hash([]), H.EMPTY_MOVED_SET)
        expected = hashlib.sha256(struct.pack('<III', 0, 257, 2**32 - 1)).hexdigest()
        self.assertEqual(H.moved_set_hash([257, 2**32 - 1, 0, 257]), expected)
        for piece in (True, -1, 2**32, 0.0, '1'):
            with self.subTest(piece=piece), self.assertRaises(H.HarnessInputError):
                H.moved_set_hash([piece])

    def test_toy_hashes_use_canonical_pose_and_piece_order(self):
        engine = ToyEngine()
        initial = engine.start(None)
        self.assertEqual(initial.digest, hashlib.sha256(b'').hexdigest())
        self.assertEqual(initial.lattice, hashlib.sha256(b'\x01' * 6).hexdigest())
        engine.poses = [2, 1, 0, 1, 2, 0]
        result = engine._observe('admissible')
        expected = hashlib.sha256()
        for key, members in sorted([(json.dumps(toy_matrix(1)), struct.pack('<ii', 1, 3)),
                                    (json.dumps(toy_matrix(2)), struct.pack('<ii', 0, 4))]):
            expected.update(key.encode('utf-8'))
            expected.update(members)
        self.assertEqual(result.digest, expected.hexdigest())
        self.assertEqual(result.lattice, hashlib.sha256(bytes([1, 0, 1, 0, 1, 1])).hexdigest())


class IndependenceTests(unittest.TestCase):
    def test_harness_files_pass(self):
        files = I.harness_files()
        self.assertTrue({'__init__.py', 'harness.py', 'menu_order.py', 'independence.py'}
                        <= {p.name for p in files})
        self.assertTrue(all(p.parent == JUMBLING / 'differential' for p in files))
        self.assertEqual(I.scan_paths(files, I.HARNESS_POLICY), [])

    def test_candidate_files_pass_if_present(self):
        files = I.candidate_files()
        self.assertTrue(all(p.suffix == '.py' and (JUMBLING / 'differential/candidate') in p.parents for p in files))
        self.assertEqual(I.scan_paths(files, I.CANDIDATE_POLICY), [])

    def test_planted_harness_imports(self):
        for source in ('import seams', 'from differential.candidate import store',
                       'import differential.candidate.store', 'from .candidate import store',
                       'from . import candidate', 'from differential import candidate',
                       'from . import seams', 'import scipy'):
            with self.subTest(source=source):
                violations = I.scan_source('\n' + source, I.HARNESS_POLICY)
                self.assertTrue(violations)
                self.assertTrue(all(v.line == 2 and v.path == '<string>' for v in violations))

    def test_planted_candidate_imports(self):
        for source in ('import sim', 'from sim.state import State', 'import exact',
                       'from differential import harness', 'import differential.menu_order',
                       'from .. import harness', 'from ..menu_order import order_digest', 'import seams'):
            with self.subTest(source=source):
                violations = I.scan_source('\n' + source, I.CANDIDATE_POLICY)
                self.assertTrue(violations)
                self.assertTrue(all(v.line == 2 for v in violations))

    def test_dynamic_code_and_sys_path_fail_both_policies(self):
        for policy in (I.HARNESS_POLICY, I.CANDIDATE_POLICY):
            for source in ("importlib.import_module('x')", "__import__('x')",
                           "sys.path.insert(0, 'x')", "runpy.run_module('x')",
                           "exec('x')", "eval('x')", "compile('x', 'x', 'exec')"):
                with self.subTest(policy=policy.name, source=source):
                    violations = I.scan_source('\n' + source, policy)
                    self.assertTrue(violations)
                    self.assertTrue(all(v.line == 2 for v in violations))

    def test_aliases_of_dynamic_access_are_flagged(self):
        for source in ('import importlib as loader', 'from importlib import import_module as load',
                       'from builtins import eval as evaluate', 'from sys import path as paths',
                       "import sys as runtime\nruntime.path.append('x')"):
            for policy in (I.HARNESS_POLICY, I.CANDIDATE_POLICY):
                with self.subTest(source=source, policy=policy.name):
                    self.assertTrue(I.scan_source(source, policy))

    def test_standard_library_and_numpy_imports_pass_both(self):
        for policy in (I.HARNESS_POLICY, I.CANDIDATE_POLICY):
            for source in ('import json', 'from pathlib import Path', 'import numpy',
                           'from numpy.linalg import norm', 'import sys'):
                with self.subTest(source=source, policy=policy.name):
                    self.assertEqual(I.scan_source(source, policy), [])

    def test_harness_allowed_imports(self):
        for source in ('import sim', 'from sim.state import State', 'import exact',
                       'import differential', 'from differential import harness',
                       'from . import menu_order', 'from .harness import Engine'):
            with self.subTest(source=source):
                self.assertEqual(I.scan_source(source, I.HARNESS_POLICY), [])

    def test_candidate_allowed_imports(self):
        for source in ('import differential.candidate.store', 'from differential.candidate import store',
                       'from . import store', 'from .store import Store',
                       'from differential import candidate'):
            with self.subTest(source=source):
                self.assertEqual(I.scan_source(source, I.CANDIDATE_POLICY), [])

    def test_nested_relative_imports_stay_inside_candidate(self):
        path = 'differential/candidate/helpers/worker.py'
        self.assertEqual(I.scan_source('from ..store import Store', I.CANDIDATE_POLICY, path=path), [])
        violations = I.scan_source('\nfrom ... import harness', I.CANDIDATE_POLICY, path=path)
        self.assertEqual(len(violations), 1)
        self.assertEqual(violations[0].line, 2)
        self.assertEqual(violations[0].path, path)
        self.assertIn('differential.harness', violations[0].message)

    def test_syntax_error_and_package_escape_fail(self):
        self.assertTrue(I.scan_source('from .. import json', I.HARNESS_POLICY))
        violations = I.scan_source('import (', I.CANDIDATE_POLICY)
        self.assertEqual(len(violations), 1)
        self.assertEqual(violations[0].line, 1)

    def test_scan_paths_reports_temporary_source_line(self):
        with tempfile.TemporaryDirectory(prefix='jumbling-scan-') as directory:
            root = Path(directory)
            self.assertFalse(root.is_relative_to(REPO))
            path = root / 'planted.py'
            path.write_text('import json\nimport seams\n', encoding='utf-8')
            violations = I.scan_paths([path], I.HARNESS_POLICY)
            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].path, str(path))
            self.assertEqual(violations[0].line, 2)
            self.assertIn('seams', violations[0].message)

    def test_acceptance_does_not_load_numpy_or_sim(self):
        self.assertFalse(any(name == 'numpy' or name.startswith('numpy.') for name in sys.modules))
        self.assertFalse(any(name == 'sim' or name.startswith('sim.') for name in sys.modules))


if __name__ == '__main__':
    unittest.main(verbosity=2)
