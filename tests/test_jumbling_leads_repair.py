"""EN2-E: recorded repairs, typed interruptions, the X1 smoke and X5 checks.

All mechanical imports happen in setUpModule. Shared unittest commands use
one isolated NumPy worker so the differential harness retains its import proof.
No generated files or bytecode belong in the repository. Class/job timings
are retained in _CLASS_SECONDS/_MEASUREMENTS for the packet's return report;
normal success prints only unittest's summary.
"""

import sys

sys.dont_write_bytecode = True

from collections import Counter
from contextlib import redirect_stderr
import copy
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
_STEPS = {}


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
        raise AssertionError('the repair tests must not create links')
    for path, directory_fd in paths:
        if isinstance(path, int):
            continue
        path = Path(os.fsdecode(path))
        if not path.is_absolute() and directory_fd not in (None, -1):
            # shutil's POSIX cleanup unlinks basenames relative to an open
            # tempfile directory, not to the repository working directory.
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
            raise AssertionError('the repair tests attempted a write inside the repository')


def _clone(S):
    # The shared context, immutable matrices and pose entries need no deep copy.
    st = copy.copy(S.st)
    st.pose_id = S.st.pose_id.copy()
    st._poses = list(S.st._poses)
    st._lookup = dict(S.st._lookup)
    st.journal = list(S.st.journal)
    st._anchor_cache = {}
    result = seams.SeamRestorer(st, S.M)
    result.moves = list(S.moves)
    return result


def _state_key(S):
    return tuple(S.moves), S.st.digest(), S.st.snapshot()


def setUpModule():
    global np, sim, rim, seams, trees, census, R, X1, X5, _AUDIT, _DENSITY, _INITIAL
    sys.addaudithook(_no_repository_writes)
    _AUDIT = True
    unittest.addModuleCleanup(lambda: globals().__setitem__('_AUDIT', False))
    try:
        import numpy as np
    except ImportError as error:
        raise unittest.SkipTest('NumPy is required for the repair-list and release-check leads') from error
    sys.path.insert(0, str(JUMBLING))
    # census comes before anything that could use seams.positions/cache.
    from differential.leads import census
    from differential.leads import repair_list as R, x1_walk as X1, x5_release as X5
    import sim
    import rim
    import seams
    import trees
    if Path(census.seams.CACHE).resolve().is_relative_to(ROOT):
        raise AssertionError('the centroid cache is inside the repository')
    started = perf_counter()
    for step in census.walk(7101):
        if step.n in (44, 70, 97):
            _STEPS[step.n] = census.Step(step.n, step.rule, list(step.word), step.record, _clone(step.S))
    _MEASUREMENTS['replay_seconds'] = perf_counter() - started
    _DENSITY = X1.density_records()[(97, 458)]
    _INITIAL = _state_key(_STEPS[97].S)


class TimedCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._class_started = perf_counter()

    @classmethod
    def tearDownClass(cls):
        _CLASS_SECONDS[cls.__name__] = perf_counter() - cls._class_started

    def unchanged(self, S, before, label):
        self.assertEqual(_state_key(S), before, label)


class RecordedRepairTests(TimedCase):
    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        global _RESULT, _OPENINGS
        S = _STEPS[97].S
        with patch.object(S, 'apply', wraps=S.apply) as apply:
            _RESULT = R.repair_list(S, 458)
        _OPENINGS = [tuple(call.args) for call in apply.call_args_list if call.args[0] == 458]
        _MEASUREMENTS['step_97'] = {'pairs': _RESULT.coverage.pairs_tried,
                                     'seconds': _RESULT.timings.total,
                                     'seconds_per_pair': _RESULT.timings.total / _RESULT.coverage.pairs_tried}

    def test_recorded_nonempty_list_and_every_outcome(self):
        result = _RESULT
        self.assertEqual(result.status, 'completed')
        self.assertEqual(len(_DENSITY), 472)
        R.compare_recorded(result, _DENSITY, step=97)
        self.assertEqual(result.rows, ((6, 513, 7, 0, -2),))
        self.assertEqual(result.coverage.pairs_tried, 472)
        self.assertEqual(result.coverage.meeting_caps, (349, 399, 416, 469, 484, 513, 523, 571))
        self.assertEqual(result.coverage.items_done, tuple(range(1, 60)))
        self.assertEqual(_OPENINGS, [(458, x) for x in range(1, 60)])
        self.assertEqual(result.input_digest, _INITIAL[1])
        self.assertEqual(result.parameters.cap, 458)
        self.assertEqual(result.parameters.menu_identity, R.I_A_IDENTITY)
        self.assertEqual(result.parameters.order_digest, R.I_A_ORDER_DIGEST)
        self.assertEqual(result.parameters.tie_rule, 'lowest value, then smaller pinned index')
        fixture = json.loads((JUMBLING / 'fixtures' / 'wj-I_a.json').read_text(encoding='utf-8'))
        self.assertEqual(result.parameters.contract_revision, fixture['journal']['contract_revision'])
        self.assertEqual(set(result.timings.per_x), set(range(1, 60)))
        self.assertTrue(all(seconds >= 0 for seconds in result.timings.per_x.values()))
        self.assertGreaterEqual(result.timings.total, sum(result.timings.per_x.values()))
        self.unchanged(_STEPS[97].S, _INITIAL, 'completed list changed its input')

    def test_planted_empty_and_changed_fields_name_the_first_row(self):
        row = _RESULT.rows[0]
        changed_row = row[:-1] + (row[-1] - 1,)
        for rows in ((), (changed_row,)):
            with self.assertRaisesRegex(AssertionError, r'step 97: first differing listed row 1'):
                R.compare_recorded(replace(_RESULT, rows=rows), _DENSITY, step=97)
        outcome = _RESULT.outcomes[0]
        changed_outcome = (outcome[0], outcome[1] + 1, outcome[2], outcome[3])
        changed = replace(_RESULT, outcomes=(changed_outcome,) + _RESULT.outcomes[1:])
        with self.assertRaisesRegex(AssertionError, r'step 97: first differing outcome row 1'):
            R.compare_recorded(changed, _DENSITY, step=97)


class TypedResultTests(TimedCase):
    def assert_incomplete(self, result, status, pairs):
        self.assertEqual(result.status, status)
        self.assertNotEqual(result.status, 'completed')
        self.assertIsNone(result.rows)
        self.assertEqual(result.coverage.pairs_tried, pairs)
        self.assertEqual(len(result.outcomes), pairs)
        with self.assertRaisesRegex(ValueError, status):
            R.completed_rows(result, step=97)
        with self.assertRaisesRegex(ValueError, status):
            R.compare_recorded(result, _DENSITY, step=97)

    def test_fifty_pair_cutoff_and_ten_pair_cancellation(self):
        S = _STEPS[97].S
        result = R.repair_list(S, 458, max_pairs=50)
        self.assert_incomplete(result, 'cut_off', 50)
        self.assertEqual(result.coverage.items_done, tuple(range(1, 7)))
        self.unchanged(S, _INITIAL, 'cut-off changed its input')
        calls = 0

        def cancel():
            nonlocal calls
            calls += 1
            return calls > 10

        result = R.repair_list(S, 458, cancel=cancel)
        self.assert_incomplete(result, 'cancelled', 10)
        self.unchanged(S, _INITIAL, 'cancellation changed its input')

    def test_blocked_closed_and_zero_misalignment_are_errors(self):
        S = _STEPS[97].S
        self.assertIsNone(S.inside(6))
        self.assertIsNotNone(S.inside(0))
        self.assertEqual(S.scores(0)[0], 0)
        solved = seams.SeamRestorer(sim.State(rim.ctx, menu=census.menu()), census.menu())
        self.assertEqual(solved.scores(458)[0], 0)
        for state, cap in ((S, 6), (S, 0), (solved, 458)):
            before = _state_key(state)
            result = R.repair_list(state, cap)
            self.assert_incomplete(result, 'error', 0)
            self.assertTrue(result.message)
            self.unchanged(state, before, f'error at cap {cap} changed its input')

    def test_zero_budget_and_exceptional_cancel_restore_state(self):
        S = _STEPS[97].S
        with patch.object(S, 'apply', wraps=S.apply) as apply:
            self.assert_incomplete(R.repair_list(S, 458, max_pairs=0), 'cut_off', 0)
            self.assertEqual(apply.call_count, 0)
        calls = 0

        def broken_cancel():
            nonlocal calls
            calls += 1
            if calls == 2:
                raise RuntimeError('planted cancellation failure')
            return False

        with self.assertRaisesRegex(RuntimeError, 'planted cancellation failure'):
            R.repair_list(S, 458, cancel=broken_cancel)
        self.unchanged(S, _INITIAL, 'exceptional cancellation changed its input')


@lru_cache(maxsize=1)
def _walk_smoke():
    started = perf_counter()
    result = X1.measure_walk([_STEPS[97]], max_pairs=1)
    _MEASUREMENTS['walk_smoke_seconds'] = perf_counter() - started
    return result


class WalkTests(TimedCase):
    def test_step_97_recorded_cap_order_and_first_cap_cutoff(self):
        S = _STEPS[97].S
        survey = S.survey()
        order = sorted(survey, key=lambda c: (-survey[c][0], c))
        caps = X1.walk_caps(S, 458)
        self.assertEqual(caps, order[:28])
        self.assertEqual(len(caps), X1.step_records()[97]['rank_c'])
        self.assertEqual(caps[-1], 458)
        self.assertEqual(caps[0], 91)
        self.assertEqual([(-survey[c][0], c) for c in caps], sorted((-survey[c][0], c) for c in caps))
        result = R.repair_list(S, caps[0], max_pairs=2)
        self.assertEqual(result.status, 'cut_off')
        self.assertEqual(result.coverage.pairs_tried, 2)
        self.assertIsNone(result.rows)
        self.unchanged(S, _INITIAL, 'first-cap smoke changed its input')

    def test_budgeted_walk_requests_and_summary_are_not_empty_lists(self):
        requests, summary = _walk_smoke()
        self.assertEqual(len(requests), 28)
        self.assertEqual([r['place'] for r in requests], list(range(1, 29)))
        self.assertEqual(requests[-1]['cap'], 458)
        required = {'step', 'cap', 'place', 'N_c', 'meeting_misaligned_caps', 'pairs',
                    'listed_rows', 'status', 'seconds', 'seconds_per_pair', 'input_digest'}
        for row in requests:
            self.assertLessEqual(required, set(row))
            self.assertEqual(row['step'], 97)
            self.assertEqual(row['status'], 'cut_off')
            self.assertEqual(row['pairs'], 1)
            self.assertIsNone(row['listed_rows'])
            self.assertEqual(row['meeting_misaligned_caps'], sorted(row['meeting_misaligned_caps']))
            self.assertGreaterEqual(row['seconds_per_pair'], 0)
        self.assertEqual(summary['overall']['pairs'], 28)
        self.assertEqual(summary['expected'], {'requests': 28, 'pairs': 18113})
        self.assertEqual(summary['status'], 'cut_off')
        self.assertTrue(summary['checks']['requests'])
        self.assertTrue(summary['checks']['repair_cap_places'])
        self.assertFalse(summary['checks']['all_completed'])
        self.assertIsNone(summary['checks']['pairs'])
        self.assertIsNone(summary['checks']['before_completed_empty'])
        self.assertIsNone(summary['checks']['record_pair_first'])
        self.assertEqual(summary['checks']['density_steps_unchecked'], [97])
        records = X1.step_records()
        self.assertEqual(len(records), 63)
        self.assertEqual(sum(r['rank_c'] for r in records.values()), 201)
        self.assertEqual(sum(X1._expected_pairs(r) for r in records.values()), 128502)
        json.dumps({'requests': requests, 'summary': summary})
        self.unchanged(_STEPS[97].S, _INITIAL, 'walk smoke changed its input')


def _reference_heights(st):
    # Actual committed state, independent of the check's hypothetical-pose sum.
    histogram = Counter()
    for pid, count in zip(*np.unique(st.pose_id, return_counts=True)):
        pose = st._poses[int(pid)]
        height = 0 if pose.kidx >= 0 else trees.dist(trees.factors(st._pose_matrix(int(pid)))[0])
        histogram[height] += int(count)
    return {'n_0': histogram[0], 'n_2': histogram[2], 'n_4': histogram[4], 'n_6': histogram[6],
            'n_above_6': sum(n for h, n in histogram.items() if h > 6),
            'max_above_6': max((h for h in histogram if h > 6), default=None)}


@lru_cache(maxsize=1)
def _release_smoke():
    return X5.measure_census([_STEPS[44], _STEPS[70]])


class ReleaseCheckTests(TimedCase):
    def test_three_releases_match_oracle_and_evaluate_one_item_only(self):
        original = rim.Domains.misalignment
        for n in (44, 70, 97):
            with self.subTest(step=n):
                step = _STEPS[n]
                S, (c, x) = step.S, step.word[0]
                before = _state_key(S)
                twists = []

                def observed(domains, cap, inside, gkey='I', g=None, rs=None):
                    if g is not None:
                        twists.append(gkey)
                    return original(domains, cap, inside, gkey, g, rs)

                with patch.object(rim.Domains, 'misalignment', observed), patch.object(
                        S, 'scores', side_effect=AssertionError('a release check surveyed alternatives')):
                    result = X5.release_check(S, c, x)
                self.assertEqual(twists, [(c, x)])
                now, after = S.scores(c)
                self.assertEqual(result['N_c_now'], now)
                self.assertEqual(result['N_c_after'], after[x])
                n_in, n_out, _ = S.balance(c)
                self.assertEqual(result['floor'], abs(n_in - n_out))
                self.assertEqual(result['delta_N'], result['N_c_after'] - result['N_c_now'])
                self.assertEqual(result['input_digest'], before[1])
                actual = _clone(S)
                self.assertEqual(result['heights_before'], _reference_heights(actual.st))
                actual.apply(c, x)
                self.assertEqual(result['heights_after'], _reference_heights(actual.st))
                self.assertEqual(result['delta_n_h'], {key: result['heights_after'][key] - result['heights_before'][key]
                                                      for key in X5.COUNT_KEYS})
                self.assertEqual(sum(result['delta_n_h'].values()), 0)
                self.assertEqual(sum(result['heights_before'][key] for key in X5.COUNT_KEYS), rim.NP)
                self.assertTrue(all(result['seconds'][key] >= 0 for key in X5.PARTS))
                self.unchanged(S, before, f'release check at step {n} changed its input')

    def test_two_state_smoke_and_part_summary(self):
        rows = _release_smoke()
        self.assertEqual([r['step'] for r in rows], [44, 70])
        self.assertTrue(all(r['oracle_checked'] for r in rows))
        summary = X5.summarize(rows)
        self.assertEqual(summary['checks'], 2)
        self.assertTrue(summary['oracle_checks_passed'])
        self.assertEqual(set(summary['seconds']), set(X5.PARTS))
        for part in summary['seconds'].values():
            self.assertGreaterEqual(part['largest'], part['median'])
            self.assertGreaterEqual(part['median'], 0)
        json.dumps({'rows': rows, 'summary': summary})


class CensusTests(TimedCase):
    def _copy(self, directory, record, **changes):
        target = Path(directory) / 'record.json'
        target.write_text(json.dumps(dict(record, **changes)), encoding='utf-8')
        return target

    def test_scramble_off_count_and_round_start_are_checked(self):
        global _TEMP_ROOT
        _, record = census.load(7001)
        try:
            with tempfile.TemporaryDirectory(prefix='jumbling-census-record-') as temporary:
                _TEMP_ROOT = Path(temporary).resolve()
                for changes, message in (({'off_scramble': 0}, 'pieces off'),
                                         ({'N_start': record['N_start'] + 1}, "round's start")):
                    with self.subTest(changes=changes), self.assertRaisesRegex(census.CensusError, message):
                        next(census.walk(self._copy(temporary, record, **changes)))
                missing = {key: value for key, value in record.items() if key != 'off_scramble'}
                with self.assertRaisesRegex(census.CensusError, 'no off_scramble'):
                    next(census.walk(self._copy(temporary, missing)))
                with self.assertRaisesRegex(census.CensusError, 'not a step boundary'):
                    next(census.walk(self._copy(temporary, record, continued_from_moves=len(record['moves']) + 1)))
                # A round starting at the record's end is checked after the last step.
                at_end = dict(continued_from_moves=len(record['moves']), N_start=record['N_now'],
                              off_start=record['off_now'])
                self.assertEqual(len(list(census.walk(self._copy(temporary, record, **at_end)))),
                                 len(record['steps']))
                with self.assertRaisesRegex(census.CensusError, "the end: .*round's start"):
                    list(census.walk(self._copy(temporary, record, **dict(at_end, N_start=record['N_now'] + 1))))
                self.assertEqual(next(census.walk(self._copy(temporary, record))).n, 1)
        finally:
            _TEMP_ROOT = None

    def test_centroid_cache_inside_repository_is_refused(self):
        before = census.seams.CACHE
        for path in (ROOT, ROOT / 'work', JUMBLING / 'theory'):
            with self.subTest(path=path), self.assertRaisesRegex(census.CensusError, 'outside the repository'):
                census.isolate_cache(path)
        saved = census._CACHE_DIR
        try:
            census._CACHE_DIR = None
            with patch.object(census.tempfile, 'gettempdir', return_value=str(ROOT / 'work')), \
                    patch.object(census.tempfile, 'mkdtemp', side_effect=AssertionError('made a directory')) as made:
                with self.assertRaisesRegex(census.CensusError, 'outside the repository'):
                    census.isolate_cache()
                made.assert_not_called()
        finally:
            census._CACHE_DIR = saved
        self.assertEqual(census.seams.CACHE, before)


class CLISmokeTests(TimedCase):
    def test_output_file_resolving_into_repository_is_refused(self):
        real = Path.resolve
        for module, function, name in ((X1, 'run_walk', 'x1-summary.json'), (X5, 'run_release', 'x5-checks.jsonl')):
            def resolve(path, strict=False, name=name):
                return ROOT / 'assets' / 'manifest.json' if path.name == name else real(path, strict)

            with self.subTest(module=module.__name__), \
                    tempfile.TemporaryDirectory(prefix='jumbling-repair-out-') as temporary, \
                    patch.object(Path, 'resolve', resolve), patch.object(module, function) as run:
                error = io.StringIO()
                with redirect_stderr(error), self.assertRaises(SystemExit) as stopped:
                    module.main(['--out', temporary])
                self.assertEqual(stopped.exception.code, 2)
                self.assertIn(f'{name} resolves inside it', error.getvalue())
                run.assert_not_called()
                self.assertEqual(list(Path(temporary).iterdir()), [])

    def test_output_inside_repository_is_refused_before_measurement(self):
        for module, function in ((X1, 'run_walk'), (X5, 'run_release')):
            with self.subTest(module=module.__name__), patch.object(module, function) as run:
                error = io.StringIO()
                with redirect_stderr(error), self.assertRaises(SystemExit) as stopped:
                    module.main(['--out', str(ROOT / 'work' / 'forbidden-en2-e-output')])
                self.assertEqual(stopped.exception.code, 2)
                self.assertIn('--out must be outside the repository', error.getvalue())
                run.assert_not_called()

    def test_cli_writers_use_external_output_and_report_versions(self):
        global _TEMP_ROOT
        requests, walk = _walk_smoke()
        rows = _release_smoke()
        metadata = R.run_metadata()
        self.assertEqual(metadata['python'], '.'.join(map(str, sys.version_info[:3])))
        self.assertEqual(metadata['numpy'], np.__version__)
        self.assertEqual(metadata['cpu_count'], os.cpu_count())
        # Outside a Git checkout the commit is None, never guessed.
        self.assertRegex(metadata['commit'] or '0' * 40, r'^[0-9a-f]{40}$')
        try:
            with tempfile.TemporaryDirectory(prefix='jumbling-repair-cli-') as temporary:
                directory = _TEMP_ROOT = Path(temporary).resolve()
                self.assertFalse(directory.is_relative_to(ROOT))
                with patch.object(X1, 'run_walk', return_value=(requests, dict(walk, **metadata))) as run:
                    X1.main(['--steps', '97', '--jobs', '2', '--out', temporary])
                    self.assertEqual(run.call_args.kwargs, {'steps': [97], 'jobs': 2, 'rest': None})
                with patch.object(X5, 'run_release', return_value=(rows, dict(X5.summarize(rows), **metadata))):
                    X5.main(['--out', temporary])
                self.assertEqual(len((directory / 'x1-requests.jsonl').read_text(encoding='utf-8').splitlines()), 28)
                self.assertEqual(len((directory / 'x5-checks.jsonl').read_text(encoding='utf-8').splitlines()), 2)
                for name in ('x1-summary.json', 'x5-summary.json'):
                    summary = json.loads((directory / name).read_text(encoding='utf-8'))
                    self.assertEqual(summary['commit'], metadata['commit'])
        finally:
            _TEMP_ROOT = None


def load_tests(loader, tests, pattern):
    if _IN_PROCESS:
        return unittest.TestSuite(loader.loadTestsFromTestCase(case) for case in
                                  (RecordedRepairTests, TypedResultTests, WalkTests, ReleaseCheckTests, CensusTests,
                                   CLISmokeTests))
    from tests import isolated_numpy
    return isolated_numpy.suite('test_jumbling_leads_repair', 'repair and release leads in an isolated NumPy process',
                                'NumPy is required for the repair-list and release-check leads')


if __name__ == '__main__':
    unittest.main()
