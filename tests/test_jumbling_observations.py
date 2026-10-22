"""EN2-J: exact observation values and planted first differences.

Mechanical imports live in setUpModule; shared unittest commands use the
isolated NumPy worker. Success prints only unittest's summary. Class and job
times remain available in _CLASS_SECONDS/_MEASUREMENTS for the packet report.
The deep replay measured 147.62 s by itself and belongs to full mode; these
tests never read the draft-branch height-4 extraction.
"""

import sys

sys.dont_write_bytecode = True

from collections import Counter
from contextlib import redirect_stderr
from dataclasses import replace
from functools import lru_cache
import io
import json
import os
from pathlib import Path
import tempfile
from time import perf_counter
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / 'research' / 'jumbling'
_IN_PROCESS = __name__ == '__main__'
_AUDIT = False
_TEMP_ROOT = None
_CLASS_SECONDS = {}
_MEASUREMENTS = {}


def _no_repository_writes(event, args):
    if not _AUDIT:
        return
    paths = []
    if event == 'open':
        path, mode, flags = args
        if flags & (os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC | os.O_APPEND):
            paths = [(path, None)]
    elif event in ('os.mkdir', 'os.chmod'):
        paths = [(args[0], args[2])]
    elif event in ('os.remove', 'os.rmdir'):
        paths = [(args[0], args[1])]
    elif event == 'os.truncate':
        paths = [(args[0], None)]
    elif event == 'os.rename':
        paths = [(args[0], args[2]), (args[1], args[3])]
    elif event in ('os.link', 'os.symlink'):
        raise AssertionError('the observation tests must not create links')
    for path, directory_fd in paths:
        if isinstance(path, int):
            continue
        path = Path(os.fsdecode(path))
        if not path.is_absolute() and directory_fd not in (None, -1):
            status = os.fstat(directory_fd)
            directories = ([] if _TEMP_ROOT is None else
                           [_TEMP_ROOT] + [p for p in _TEMP_ROOT.rglob('*') if p.is_dir()])
            for directory in directories:
                candidate = directory.stat()
                if (status.st_dev, status.st_ino) == (candidate.st_dev, candidate.st_ino):
                    path = directory / path
                    break
            else:
                raise AssertionError('test write uses an unrecognised directory fd')
        if path.resolve().is_relative_to(ROOT):
            raise AssertionError('the observation tests attempted a write inside the repository')


def setUpModule():
    global np, census, O, R, rim, seams, sim, trees, _AUDIT
    sys.addaudithook(_no_repository_writes)
    _AUDIT = True
    unittest.addModuleCleanup(lambda: globals().__setitem__('_AUDIT', False))
    try:
        import numpy as np
    except ImportError as error:
        raise unittest.SkipTest('NumPy is required for the jumbling observations') from error
    sys.path.insert(0, str(JUMBLING))
    from differential.leads import census
    from differential.references import observations as O, obs_refs as R
    import rim
    import seams
    import sim
    import trees
    if Path(census.seams.CACHE).resolve().is_relative_to(ROOT):
        raise AssertionError('the centroid cache is inside the repository')


class TimedCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._started = perf_counter()

    @classmethod
    def tearDownClass(cls):
        _CLASS_SECONDS[cls.__name__] = perf_counter() - cls._started

    def difference(self, actual, reference, field, **where):
        difference = O.compare(actual, reference)
        self.assertIsNotNone(difference)
        self.assertEqual(difference.field, field)
        for key, value in where.items():
            self.assertEqual(getattr(difference, key), value)
            self.assertIn(f'{key} {value}', str(difference))
        self.assertIn(field, str(difference))
        self.assertNotEqual(difference.reference, difference.candidate)
        return difference


@lru_cache(maxsize=1)
def _stop():
    started = perf_counter()
    state = R.stop40_state()
    _MEASUREMENTS['stop40_replay'] = perf_counter() - started
    started = perf_counter()
    before = state.digest(), state.snapshot()
    with patch.object(seams.SeamRestorer, 'scores', side_effect=AssertionError('scored menu items')), \
            patch.object(seams.SeamRestorer, 'survey', side_effect=AssertionError('scored a survey')):
        observation = O.observe(state)
    _MEASUREMENTS['observe_stop40'] = perf_counter() - started
    if before != (state.digest(), state.snapshot()):
        raise AssertionError('observe changed the state or journal')
    return state, observation


@lru_cache(maxsize=1)
def _off_grip():
    state, _ = _stop()
    started = perf_counter()
    groups = O.off_grip_seams(state)
    _MEASUREMENTS['off_grip_stop40'] = perf_counter() - started
    return groups


class TreeHeightTests(TimedCase):
    def test_I_a_prefixes_match_both_recorded_histograms_and_left_counts(self):
        for t in (10, 20, 60):
            with self.subTest(prefix=t):
                started = perf_counter()
                state = R.fixture_state('I_a', t)
                _MEASUREMENTS[f'fixture_I_a_{t}_replay'] = perf_counter() - started
                started = perf_counter()
                observation = O.observe(state)
                _MEASUREMENTS[f'observe_I_a_{t}'] = perf_counter() - started
                actual = O.height_histograms(state, observation)
                reference = R.trees_reference('I_a', t)
                for field in ('pieces_by_heights', 'pose_classes_by_heights'):
                    self.assertEqual(actual[field], reference[field], field)
                left = Counter()
                for pair, count in reference['pieces_by_heights'].items():
                    left[int(pair.split(',')[0])] += count
                self.assertEqual(observation.height_counts, O.left_counts(left))
                self.assertEqual(sum(observation.height_counts[key] for key in O.COUNT_KEYS), rim.NP)
                counts = dict(observation.height_counts, n_2=observation.height_counts['n_2'] + 1)
                difference = self.difference(replace(observation, height_counts=counts), observation,
                                             'height_counts.n_2')
                self.assertEqual(difference.reference, observation.height_counts['n_2'])
                self.assertEqual(difference.candidate, counts['n_2'])

    def test_sparse_bins_keep_the_maximum_and_S4_odd_heights(self):
        self.assertEqual(O.left_counts({0: 1, 2: 3, 8: 2, 12: 4}),
                         {'n_0': 1, 'n_2': 3, 'n_4': 0, 'n_6': 0,
                          'n_above_6': 6, 'max_above_6': 12})
        counts = O.left_counts({0: 5, 1: 3, 3: 2})
        self.assertEqual((counts['n_1'], counts['n_3'], counts['max_above_6']), (3, 2, None))
        with self.assertRaisesRegex(ValueError, 'no trees reference'):
            R.trees_reference('I_a', 11)


class DomainTests(TimedCase):
    def test_canonical_partition_lattice_key_and_exact_coset_samples(self):
        from exact import identity, matmul, matvec, transpose
        from sim.kplus import to_tuple
        state, observation = _stop()
        domains = rim.Domains(state)
        expected = {O.domain_key(to_tuple(matrix)): tuple(mem.tolist())
                    for matrix, mem in zip(domains.reps, domains.members) if len(mem)}
        self.assertEqual(observation.domain_partition(), expected)
        self.assertEqual(O.LATTICE_KEY, O.domain_key(to_tuple(identity(4))))
        self.assertTrue(all(observation.domain_keys[p] == O.LATTICE_KEY for p in domains.members[0]))
        # The four exact generators generate K+; this checks that the 120
        # vertex coordinates used by the key have the retained orientation.
        for k in rim.kp.gen_idx[:4]:
            self.assertEqual({tuple(matvec(rim.kp.matrix(int(k)), v)) for v in trees.UNITS}, set(trees.UNITS))
        sample = [int(np.flatnonzero(state.pose_id == pid)[0]) for pid in np.unique(state.pose_id)]
        tested_equal = tested_different = 0
        for a in sample:
            # One pair in each domain, plus one with the next sampled pose.
            same = next(p for p in reversed(sample)
                        if observation.domain_keys[p] == observation.domain_keys[a])
            other = sample[(sample.index(a) + 1) % len(sample)]
            for b in (same, other):
                matrix = matmul(transpose(state._pose_matrix(int(state.pose_id[a]))),
                                state._pose_matrix(int(state.pose_id[b])))
                exact_same = rim.kp.index_of_matrix(to_tuple(matrix)) >= 0
                self.assertEqual(exact_same, observation.domain_keys[a] == observation.domain_keys[b], (a, b))
                tested_equal += exact_same
                tested_different += not exact_same
        self.assertGreater(tested_equal, 0)
        self.assertGreater(tested_different, 0)

    def test_one_piece_moved_to_another_key_names_that_piece(self):
        _, reference = _stop()
        piece = 0
        keys = list(reference.domain_keys)
        keys[piece] = next(key for key in reference.domain_partition() if key != keys[piece])
        difference = self.difference(replace(reference, domain_keys=tuple(keys)), reference,
                                     'partition', piece=piece)
        self.assertEqual((difference.reference, difference.candidate),
                         (reference.domain_keys[piece], keys[piece]))


class RimSummaryTests(TimedCase):
    def test_two_routes_recorded_N_and_J1_survey_without_item_scores(self):
        state, reference = _stop()
        started = perf_counter()
        with patch.object(seams.SeamRestorer, 'scores', side_effect=AssertionError('scored menu items')):
            actual = O.observe(state, rim_route='seams')
        _MEASUREMENTS['observe_stop40_seams'] = perf_counter() - started
        self.assertIsNone(O.compare(actual, reference))
        self.assertEqual((reference.n_total, reference.grip_n), (1192, 1057))
        self.assertEqual(sum((row.N_c or 0) > 0 for row in reference.rim.values()), 15)
        letters = {'admissible': 'a', 'blocked': 'b', 'uncertain': 'u'}
        survey = {c: letters[row['status']] for c, row in state.survey().items()}
        self.assertEqual({c: letters[row.status] for c, row in reference.rim.items()}, survey)
        floor_caps = []
        for c, row in reference.rim.items():
            self.assertEqual(row.blocked, survey[c] == 'b')
            if row.status == 'admissible':
                self.assertEqual(row.N_c, row.In_c + row.Out_c - 2 * row.A_c, c)
                self.assertEqual(row.F_c, abs(row.In_c - row.Out_c), c)
                self.assertEqual(row.at_floor, row.N_c == row.F_c, c)
                if row.N_c > 0 and row.at_floor:
                    floor_caps.append(c)
            else:
                self.assertIsNone(row.N_c, c)
        self.assertEqual(floor_caps, [17, 34, 50, 162, 187, 237, 315, 390])

    def test_changed_rim_N_names_cap_and_both_values(self):
        _, reference = _stop()
        cap = next(c for c, row in reference.rim.items() if row.N_c and row.N_c > 0)
        rows = dict(reference.rim)
        rows[cap] = replace(rows[cap], N_c=rows[cap].N_c + 1)
        difference = self.difference(replace(reference, rim=rows), reference, 'rim.N_c', cap=cap)
        self.assertEqual((difference.reference, difference.candidate),
                         (reference.rim[cap].N_c, rows[cap].N_c))

    def test_first_difference_order_and_incomplete_inputs_cannot_pass(self):
        _, reference = _stop()
        self.difference(replace(reference, digest='0' * 64, n_total=0), reference, 'digest')
        pose = next(iter(reference.heights))
        heights = dict(reference.heights)
        heights[pose] = (99, 99)
        self.difference(replace(reference, heights=heights, n_total=0), reference, 'heights', pose=pose)
        self.difference(replace(reference, n_total=1193), reference, 'n_total')
        self.difference(replace(reference, grip_n=1058), reference, 'grip_n')
        self.difference(replace(reference, rim={}), reference, 'rim', cap=0)
        counts = dict(reference.pose_counts)
        counts[pose] += 1
        difference = self.difference(replace(reference, pose_counts=counts), reference, 'pose_counts', pose=pose)
        self.assertEqual((difference.reference, difference.candidate),
                         (reference.pose_counts[pose], counts[pose]))
        with self.assertRaisesRegex(ValueError, 'complete'):
            O.compare(replace(reference, rim={}), replace(reference, rim={}))
        admissible = next(c for c, row in reference.rim.items() if row.status == 'admissible')
        blocked = next(c for c, row in reference.rim.items() if row.status != 'admissible')

        def without(mapping, key):
            return {k: v for k, v in mapping.items() if k != key}

        def rows(cap, row):
            return {**reference.rim, cap: row}

        partials = {
            'no heights': {'heights': {}},
            'no counts': {'height_counts': {}},
            'counts off by one bin': {'height_counts': dict(reference.height_counts, n_2=0)},
            'one pose class without heights': {'heights': without(reference.heights, pose)},
            'one pose class dropped': {'heights': without(reference.heights, pose),
                                       'pose_counts': without(reference.pose_counts, pose)},
            'admissible cap without values': {'rim': rows(admissible, O.RimSummary('admissible'))},
            'blocked cap with a value': {'rim': rows(blocked, O.RimSummary(reference.rim[blocked].status, N_c=0))},
            'floor flag inconsistent': {'rim': rows(admissible, replace(
                reference.rim[admissible], at_floor=not reference.rim[admissible].at_floor))},
            'grip N not the sum': {'grip_n': reference.grip_n + 1},
            'height pairs cut to the left height': {
                'heights': {p: (pair[0],) for p, pair in reference.heights.items()}},
            'right heights missing': {'heights': {p: (pair[0], None) for p, pair in reference.heights.items()}},
            'a piece without a domain key': {'domain_keys': (None,) + reference.domain_keys[1:]},
            'a cap without a status': {'rim': rows(blocked, O.RimSummary(None))},
            'no digest': {'digest': None},
            'no N': {'n_total': None},
        }
        for name, changes in partials.items():
            with self.subTest(partial=name), self.assertRaisesRegex(ValueError, 'complete'):
                partial = replace(reference, **changes)
                O.compare(partial, partial)
        self.difference(replace(reference, domain_keys=reference.domain_keys[:-1]), reference,
                        'partition', piece=rim.NP - 1)


class CutCellTests(TimedCase):
    def test_three_states_partition_the_named_cut_and_reproduce_its_counts(self):
        state, observation = _stop()
        cap = next(c for c, row in observation.rim.items() if row.N_c and row.N_c > 0)
        cells = O.cut_cells(state, cap)
        self.assertTrue(cells)
        identities = {(row.domain_key, row.chamber_pair) for row in cells}
        self.assertEqual(len(identities), len(cells))
        cl = state.classify(cap, need_inside=True, stop_at_straddle=True)
        mask = np.zeros(rim.NP, bool)
        mask[cl.inside] = True
        domains = rim.Domains(state)
        reference = {}
        for d, (e, ins, outside) in domains.rim_sets(cap, mask).items():
            key = O.domain_key(O.to_tuple(domains.reps[d]))
            for cell in ins | outside:
                upper = int(rim.partner([cell], [e], add=True)[0])
                reference[key, (cell, upper)] = ('matched' if cell in ins & outside else
                                                'inside_only' if cell in ins else 'outside_only')
        self.assertEqual({(row.domain_key, row.chamber_pair): row.state for row in cells}, reference)
        counts = Counter(row.state for row in cells)
        self.assertEqual(set(counts), {'matched', 'inside_only', 'outside_only'})
        row = observation.rim[cap]
        self.assertEqual((counts['matched'] + counts['inside_only'],
                          counts['matched'] + counts['outside_only'], counts['matched']),
                         (row.In_c, row.Out_c, row.A_c))
        blocked = next(c for c, row in observation.rim.items() if row.blocked)
        with self.assertRaisesRegex(ValueError, f'cap {blocked} is blocked'):
            O.cut_cells(state, blocked)


class OffGripSeamTests(TimedCase):
    def test_exact_cells_groups_holders_and_explicit_prose_discrepancies(self):
        from exact import Q5, matvec
        state, observation = _stop()
        groups = _off_grip()
        summary = R.off_grip_summary(groups)
        self.assertEqual(summary['cells'], 135)
        self.assertEqual(summary['group_sizes'], [59, 37, 25, 14])
        self.assertEqual(observation.n_total - observation.grip_n, summary['cells'])
        self.assertEqual(len(summary['holding_caps']), 39)
        self.assertEqual(len(summary['whole_group_holding_caps']), 25)
        group59 = next(g for g in summary['groups'] if g['cells'] == 59)
        self.assertEqual(group59['common_holding_caps'], [30, 31, 34, 73, 89, 145])
        for cells in groups.values():
            for cell in cells:
                self.assertNotIn(tuple(Q5(*q) for q in cell.hyperplane[0]), rim.data.POLE)
                self.assertEqual(cell.hyperplane[1], (1452, 484, 125))
                self.assertTrue(cell.holding_caps)
                for cap, status in cell.holding_caps.items():
                    self.assertEqual(status, observation.rim[cap].status)
        # Independently compare the vector integer containment test with Q5
        # scalar evaluation, on all caps for one of the actual seam cells.
        cell = next(iter(groups.values()))[0]
        frame = [[Q5(*q) for q in row] for row in cell.frame]
        normal = tuple(Q5(*q) for q in cell.hyperplane[0])
        vertices = [matvec(frame, v) for v in rim.reg.vertices(cell.chamber_pair[1])]
        face = [v for v in vertices if O.dot(normal, v) == rim.data.KAPPA]
        self.assertGreaterEqual(len(face), 4)
        expected = {c for c in range(600) if all(rim.data.h(c, v).sign() >= 0 for v in face)}
        self.assertEqual(set(cell.holding_caps), expected)
        comparisons = {row['field']: row for row in R.prose_comparisons(summary, observation)}
        for field in ('prose.cells', 'prose.group_sizes', 'prose.group_59_common_caps'):
            self.assertEqual(comparisons[field]['result'], 'pass', field)
        for field, actual in (('prose.holding_caps_union', 39), ('prose.holding_caps_common', 25)):
            self.assertEqual(comparisons[field], {'field': field, 'reference': 20,
                                                  'actual': actual, 'result': 'fail'})
        for cap in R.PROSE['blocked_examples']:
            self.assertEqual(observation.rim[cap].status, 'blocked')
        for cap in R.PROSE['free_aligned_examples']:
            self.assertEqual((observation.rim[cap].status, observation.rim[cap].N_c), ('admissible', 0))


class DeepReferenceTests(TimedCase):
    def test_recorded_deep_input_for_the_separate_full_replay(self):
        # This is a record-shape check, not evidence of a mechanical replay.
        reference = R.deep_reference()
        self.assertEqual((reference['seed'], len(reference['moves'])), (7, 360))
        self.assertEqual((reference['off_lattice_pieces'], reference['moved_pieces']), (165529, 171840))
        self.assertEqual(reference['digest'],
                         '3061502ba1f6f56555d9404a420d9da14b1cd3f5918d749a8e4e0d19cfefe12a')


class CLISmokeTests(TimedCase):
    def test_repository_out_or_extract_is_refused_before_research_imports(self):
        for args, option in ((['--out', str(ROOT / 'work/forbidden-observations')], '--out'),
                             (['--out', tempfile.gettempdir(), '--extract', str(ROOT)], '--extract')):
            with self.subTest(option=option), patch.object(R, '_modules', side_effect=AssertionError('computed')):
                error = io.StringIO()
                with redirect_stderr(error), self.assertRaises(SystemExit) as stopped:
                    R.main(args)
                self.assertEqual(stopped.exception.code, 2)
                self.assertIn(f'{option} must be outside the repository', error.getvalue())

    def test_stop40_cli_writes_actual_observations_with_every_result(self):
        global _TEMP_ROOT
        try:
            with tempfile.TemporaryDirectory(prefix='jumbling-observations-cli-') as temporary:
                directory = _TEMP_ROOT = Path(temporary).resolve()
                started = perf_counter()
                self.assertEqual(R.main(['--states', 'stop40', '--out', temporary]), 0)
                _MEASUREMENTS['cli_stop40'] = perf_counter() - started
                self.assertEqual({p.name for p in directory.iterdir()},
                                 {'summary.json', 'observation-stop40.json'})
                result = json.loads((directory / 'observation-stop40.json').read_text(encoding='utf-8'))
                self.assertLessEqual({'counts', 'rim_summary', 'partition_sizes', 'off_grip_groups',
                                      'comparisons', 'seconds', 'input_sha256'}, set(result))
                self.assertEqual(result['status'], 'pass')
                self.assertEqual((result['n_total'], result['grip_n']), (1192, 1057))
                self.assertEqual(len(result['rim_summary']), 600)
                self.assertEqual(sum(result['partition_sizes'].values()), rim.NP)
                self.assertEqual(result['off_grip_groups']['cells'], 135)
                self.assertTrue(result['comparisons'])
                self.assertTrue(all(row['result'] == 'pass' for row in result['comparisons']))
                self.assertEqual(sum(row['result'] == 'fail' for row in result['prose_comparisons']), 2)
                self.assertTrue(all(value >= 0 for value in result['seconds'].values()))
                self.assertEqual(result['numpy'], np.__version__)
                summary = json.loads((directory / 'summary.json').read_text(encoding='utf-8'))
                self.assertEqual(summary['status'], 'pass')
                self.assertEqual(len(summary['states']), 1)
                _MEASUREMENTS['cli_seconds'] = result['seconds']
        finally:
            _TEMP_ROOT = None


_CASES = (TreeHeightTests, DomainTests, RimSummaryTests, CutCellTests, OffGripSeamTests,
          DeepReferenceTests, CLISmokeTests)


def load_tests(loader, tests, pattern):
    if _IN_PROCESS:
        return unittest.TestSuite(loader.loadTestsFromTestCase(case) for case in _CASES)
    from tests import isolated_numpy
    return isolated_numpy.suite('test_jumbling_observations', 'observations in an isolated NumPy process',
                                'NumPy is required for the jumbling observations')


if __name__ == '__main__':
    unittest.main()
