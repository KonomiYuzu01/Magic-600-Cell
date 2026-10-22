"""EN2-K: revision-bound words, stored records and adverse inputs.

Mechanical imports occur only in setUpModule, in an isolated NumPy worker
under a shared unittest command. _CLASS_SECONDS and _MEASUREMENTS retain
timings for the packet report; success prints only unittest's summary.
"""

import sys

sys.dont_write_bytecode = True

from contextlib import redirect_stderr
import copy
from dataclasses import FrozenInstanceError, replace
import hashlib
import io
import json
import os
from pathlib import Path
import re
import tempfile
from time import perf_counter
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / 'research/jumbling'
_IN_PROCESS = __name__ == '__main__'
_AUDIT = False
_TEMP_ROOT = None
_CLASS_SECONDS = {}
_MEASUREMENTS = {}
_STEPS = {}
LIBRARY_DIGEST = '8a13eb6c4f4359ce4dd58a0018cf86eed3089111e11f20b7c00476f6393a661a'


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
        raise AssertionError('the word tests must not create links')
    for path, fd in paths:
        if isinstance(path, int):
            continue
        path = Path(os.fsdecode(path))
        if not path.is_absolute() and fd not in (None, -1):
            status = os.fstat(fd)
            directories = [] if _TEMP_ROOT is None else [_TEMP_ROOT] + [
                p for p in _TEMP_ROOT.rglob('*') if p.is_dir()]
            for directory in directories:
                candidate = directory.stat()
                if (status.st_dev, status.st_ino) == (candidate.st_dev, candidate.st_ino):
                    path = directory / path
                    break
            else:
                raise AssertionError('unrecognised directory fd in test write')
        if path.resolve().is_relative_to(ROOT):
            raise AssertionError('the word tests attempted a write inside the repository')


def setUpModule():
    global np, sim, census, W, R, repair, covers, menu_order, replay_check, store, _END, _AUDIT
    sys.addaudithook(_no_repository_writes)
    _AUDIT = True
    unittest.addModuleCleanup(lambda: globals().__setitem__('_AUDIT', False))
    try:
        import numpy as np
    except ImportError as error:
        raise unittest.SkipTest('NumPy is required for the jumbling word checks') from error
    sys.path.insert(0, str(JUMBLING))
    # First local import: isolates the centroid cache before research modules.
    from differential.leads import census
    from differential.references import word_check as W, records as R
    from differential.leads import repair_list as repair, covers
    from differential import menu_order
    from differential.candidate import store
    import replay_check
    import sim
    if Path(census.seams.CACHE).resolve().is_relative_to(ROOT):
        raise AssertionError('centroid cache is inside the repository')
    started = perf_counter()
    for step in census.walk(7101, end=True):
        if step.n in (44, 70):
            _STEPS[step.n] = (W.copy_state(step.S.st), list(step.word))
        if step.rule is None:
            _END = W.copy_state(step.S.st)
    _MEASUREMENTS['census_seconds'] = perf_counter() - started
    if _END.digest() != '7e128820541abc82e45c683db92f38413acba6e282c00f3185585e507b4e6818':
        raise AssertionError('census did not reproduce u1 input digest')


class TimedCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._started = perf_counter()

    @classmethod
    def tearDownClass(cls):
        _CLASS_SECONDS[cls.__name__] = perf_counter() - cls._started


class ResearchWordTests(TimedCase):
    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        cls.doc = json.loads((JUMBLING / 'restoration/12-low-height/runs/lc13-k40/steps/u1.json').read_text())
        cls.record = json.loads((JUMBLING / 'restoration/12-low-height/runs/lc13-k40/replay-b01.json').read_text())
        with redirect_stderr(io.StringIO()):
            units = W.units_at(_END)
        cls.unit = units[1]
        cls.before = _END.snapshot()
        cls.checked = W.word_check(_END, cls.doc['word'], cls.unit)
        # Invoke the actual research check, substituting only its start loader
        # so the identical already-validated census state needs no disk table.
        args = SimpleNamespace(table=None, start_words=[],
                               words=[JUMBLING / 'restoration/12-low-height/runs/lc13-k40/steps/u1.json'],
                               depth_every=1000000, claims=None, export=None)
        restorer = census.seams.SeamRestorer(W.copy_state(_END), census.menu())
        with patch.object(replay_check.O, 'start', return_value=(restorer, census.menu())), \
                redirect_stderr(io.StringIO()):
            cls.replayed = replay_check.check(args)

    def test_u1_every_shared_field_against_both_research_formats(self):
        check = self.checked
        self.assertTrue(check.admissible)
        self.assertTrue(check.step_flag)
        self.assertEqual(len(self.unit), 72)
        self.assertEqual(check.off_before, self.record['off'])
        self.assertEqual(check.start_digest, self.record['digest'])
        self.assertEqual(check.end_digest, self.doc['output_digest'])
        comparison = W.compare_step(check, self.record['rows'][0], W.EB4_MAPPING)
        self.assertTrue(comparison.passed, comparison.message)
        self.assertTrue(self.replayed['pass'], self.replayed)
        self.assertEqual(self.replayed['start_digest'], check.start_digest)
        for row in (self.replayed['files'][0], self.replayed['total']):
            comparison = W.compare_step(check, row, W.REPLAY_MAPPING)
            self.assertTrue(comparison.passed, comparison.message)
        self.assertEqual(check.word_digest, hashlib.sha256(json.dumps(
            self.doc['word'], separators=(',', ':')).encode('ascii')).hexdigest())
        self.assertEqual(check.letters, 624)
        self.assertEqual(sum(check.heights_before[key] for key in W.COUNT_KEYS), len(_END.pose_id))
        self.assertEqual(sum(check.delta_n_h.values()), 0)
        self.assertIsNone(check.delta_u)
        self.assertEqual(_END.snapshot(), self.before)
        with self.assertRaises(FrozenInstanceError):
            check.step_flag = False

    def test_planted_others_exact_and_every_mapped_replay_field_fail_by_name(self):
        planted = dict(self.record['rows'][0], others_exact=1120)
        result = W.compare_step(self.checked, planted, W.EB4_MAPPING)
        self.assertFalse(result.passed)
        self.assertEqual((result.field, result.expected, result.actual), ('others_exact', 1120, 1121))
        self.assertIn('others_exact', result.message)
        for field in W.REPLAY_MAPPING:
            row = copy.deepcopy(self.replayed['total'])
            value = row[field]
            row[field] = value + 1 if type(value) is int else ([1] if isinstance(value, list) else 'planted')
            with self.subTest(field=field):
                result = W.compare_step(self.checked, row, W.REPLAY_MAPPING)
                self.assertFalse(result.passed)
                self.assertEqual(result.field, field)
        missing = dict(self.record['rows'][0])
        del missing['others_exact']
        self.assertEqual(W.compare_step(self.checked, missing, W.EB4_MAPPING).field, 'others_exact')
        self.assertFalse(W.compare_step(self.checked, {}, {}).passed)


class ReleaseAgreementTests(TimedCase):
    def test_two_census_releases_agree_in_every_shared_field(self):
        for number in (44, 70):
            state, word = _STEPS[number]
            before = state.snapshot()
            c, x = word[0]
            S = census.seams.SeamRestorer(state, census.menu())
            with self.subTest(step=number), patch.object(S, 'scores', side_effect=AssertionError('scored alternatives')):
                released = W.release_fields(S, c, x, cap=6)
                checked = W.word_check(state, [[c, x]], (), cap=6)
                shared = ('input_digest', 'start_digest', 'end_digest', 'word_digest', 'delta_N', 'delta_n_h',
                          'heights_before', 'heights_after', 'u_before', 'u_after', 'delta_u')

                def first_difference(row):
                    return next((field for field in shared if getattr(checked, field) != row[field]), None)

                self.assertIsNone(first_difference(released), f'step {number}')
                heights = dict(released['heights_before'], n_0=released['heights_before']['n_0'] + 1)
                self.assertEqual(first_difference(dict(released, heights_before=heights)), 'heights_before')
                self.assertEqual(checked.delta_N, released['N_c_after'] - released['N_c_now'])
                self.assertEqual(checked.delta_u, checked.u_after - checked.u_before)
                self.assertEqual(set(checked.delta_n_h), set(W.COUNT_KEYS))
            self.assertEqual(state.snapshot(), before)


class PreviewCommitTests(TimedCase):
    def test_admissible_collateral_is_shown_and_committed_from_preview(self):
        before = _END.snapshot()
        letters = [[0, 12]]  # caller-named synthetic jumble at a clear census-end cap
        shown = W.preview(_END, letters)
        self.assertTrue(shown.check.admissible)
        self.assertFalse(shown.check.step_flag)
        self.assertGreater(shown.check.lattice_pieces_lifted_off, 0)
        self.assertEqual(shown.check.lattice_pieces_lifted_off, len(shown.check.collateral_ids))
        with patch.object(sim.State, 'apply', side_effect=AssertionError('commit replayed the word')):
            committed = W.commit(_END, shown, letters)
        self.assertEqual(committed.digest(), shown.end_digest)
        self.assertEqual(_END.snapshot(), before)
        self.assertIsNot(committed, shown.end_state)
        # The committed state equals a direct application, journal included.
        from differential.j1_state import twists_from_letters
        direct = W.copy_state(_END)
        for twist in twists_from_letters(_END.menu or census.menu(), letters):
            self.assertTrue(direct.apply(twist).applied)
        self.assertEqual(committed.snapshot(), direct.snapshot())

    def test_blocked_prefix_names_index_and_never_commits_partial_state(self):
        letters = [[0, 1], [6, 1]]
        before = _END.snapshot()
        shown = W.preview(_END, letters)
        self.assertFalse(shown.check.admissible)
        self.assertEqual(shown.check.applied_letters, 1)
        self.assertEqual((shown.check.failing_letter_index, shown.check.failing_status), (1, 'blocked'))
        self.assertFalse(shown.check.step_flag)
        self.assertFalse(W.compare_step(shown.check, {}, W.EB4_MAPPING).passed)
        with self.assertRaises(Raised := W.BlockedWordError) as refused:
            W.commit(_END, shown, letters)
        self.assertIsInstance(refused.exception, Raised)
        self.assertEqual((refused.exception.index, refused.exception.status), (1, 'blocked'))
        self.assertEqual(_END.snapshot(), before)

    def test_uncertain_prefix_is_refused_and_state_is_unchanged(self):
        before = _END.snapshot()
        classify = sim.State.classify

        def without_budget(state, *args, **kwargs):
            kwargs['budget'] = 0
            return classify(state, *args, **kwargs)

        with patch.object(sim.State, 'classify', without_budget):
            shown = W.preview(_END, [[0, 1]])
        self.assertEqual((shown.check.failing_letter_index, shown.check.failing_status), (0, 'uncertain'))
        with self.assertRaises(W.BlockedWordError) as refused:
            W.commit(_END, shown, [[0, 1]])
        self.assertEqual(refused.exception.status, 'uncertain')
        self.assertEqual(_END.snapshot(), before)

    def test_changed_revision_changed_word_and_edited_end_are_stale(self):
        letters = [[0, 12]]
        shown = W.preview(_END, letters)
        changed = W.copy_state(shown.end_state)
        for state, word, field in ((changed, letters, 'start_digest'), (_END, [[0, 13]], 'word_digest')):
            before = state.snapshot()
            with self.subTest(field=field), self.assertRaises(W.StalePreviewError) as refused:
                W.commit(state, shown, word)
            self.assertEqual(refused.exception.field, field)
            self.assertEqual(state.snapshot(), before)
        # The preview's mutable J1 state is checked too; it cannot silently
        # supply a new endpoint after the displayed digest was recorded.
        before = _END.snapshot()
        shown.end_state.pose_id[0] = 0 if shown.end_state.pose_id[0] else 1
        with self.assertRaises(W.StalePreviewError) as refused:
            W.commit(_END, shown, letters)
        self.assertEqual(refused.exception.field, 'end_digest')
        self.assertEqual(_END.snapshot(), before)
        # J1's digest excludes the journal, so the journal of the end state is checked on its own.
        shown = W.preview(_END, letters)
        shown.end_state.journal.clear()
        with self.assertRaises(W.StalePreviewError) as refused:
            W.commit(_END, shown, letters)
        self.assertEqual(refused.exception.field, 'end_journal')
        self.assertEqual(_END.snapshot(), before)

    def test_moves_that_keep_the_poses_and_a_reordered_menu_are_stale(self):
        letters = [[0, 1]]
        solved = sim.State(_END.ctx, menu=census.menu())
        shown = W.preview(solved, letters)
        for generator in (1, -1):
            self.assertTrue(solved.apply(sim.primitive(solved.ctx, generator)).applied)
        # The poses are back where the preview started; only the journal shows the two moves.
        self.assertEqual(solved.digest(), shown.check.start_digest)
        before = solved.snapshot()
        with self.assertRaises(W.StalePreviewError) as refused:
            W.commit(solved, shown, letters)
        self.assertEqual(refused.exception.field, 'start_journal')
        self.assertEqual(solved.snapshot(), before)
        # A valid menu with items 1 and 2 swapped keeps its identity but turns
        # letter [0, 1] into another twist.
        record = copy.deepcopy(census.menu().record())
        record['base'][1], record['base'][2] = record['base'][2], record['base'][1]
        reordered = sim.State(_END.ctx, menu=sim.TwistMenu.from_record(_END.ctx, record))
        shown = W.preview(sim.State(_END.ctx, menu=census.menu()), letters)
        self.assertEqual(reordered.digest(), shown.check.start_digest)
        self.assertEqual(reordered.menu.identity, shown.menu_identity)
        replayed = W.preview(reordered, letters)
        self.assertNotEqual(replayed.end_digest, shown.end_digest)
        before = reordered.snapshot()
        with self.assertRaises(W.StalePreviewError) as refused:
            W.commit(reordered, shown, letters)
        self.assertEqual(refused.exception.field, 'order_digest')
        self.assertEqual(reordered.snapshot(), before)

    def test_complete_input_validation_precedes_replay(self):
        before = _END.snapshot()
        for word in ([[0, 1], [True, 2]], [[0, 1], [600, 1]], [[0, 1], [0, -1]]):
            with self.subTest(word=word), patch.object(sim.State, 'apply') as apply:
                with self.assertRaisesRegex(ValueError, 'letter 1'):
                    W.preview(_END, word)
                apply.assert_not_called()
        self.assertEqual(_END.snapshot(), before)


class RecordTests(TimedCase):
    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        digest, menu = _END.digest(), census.menu()
        order, contract = menu_order.order_digest(menu.record()), repair.contract_revision()
        cls.available = dict(input_digest=digest, menu_identity=menu.identity, order_digest=order,
                             contract_revision=contract, library_digest=LIBRARY_DIGEST)
        # Typed synthetic completed envelopes exercise the reader, not the
        # already-covered expensive repair enumeration / covers computation.
        cls.repair = repair.RepairListResult('completed', digest,
            repair.RepairParameters(458, menu.identity, order, repair.TIE_RULE, contract),
            repair.RepairCoverage(1, (6,), (513,)), ((6, 513, 7, 0, -2),), (), repair.RepairTimings({}, 0.0))
        cls.covers = covers.CoversResult('completed', digest,
            dict(cap=458, menu_identity=menu.identity, menu_order_digest=order), (), (), 0.0)
        cls.stored = dict(name='synthetic stored word', menu_size=60, letters=store.encode_ids([12]),
                          recorded_at=digest, identities=dict(model=dict(_END.ctx.data.identity),
                              menu=menu.identity, menu_order=order, contract_revision=contract))
        cls.library = R.read_against(JUMBLING / 'restoration/09-general-method/data/library-merged.json.gz',
                                     library_digest=LIBRARY_DIGEST)

    def test_valid_completed_records_and_real_decompressed_library(self):
        for record, fields in ((self.repair, tuple(self.available)[:4]),
                               (self.covers, tuple(self.available)[:3]),
                               (self.stored, tuple(self.available)[:4]), (self.library, ('library_digest',))):
            self.assertIs(R.read_against(record, **{k: self.available[k] for k in fields}), record)
        self.assertEqual(self.library.library_digest, LIBRARY_DIGEST)
        self.assertEqual(len(self.library.data['macros']), 36)

    def test_every_identity_wrong_in_turn_on_each_record_is_a_named_error(self):
        for kind, record in (('repair', self.repair), ('covers', self.covers),
                             ('stored', self.stored), ('library', self.library)):
            for field, good in self.available.items():
                wrong = 'planted-' + good
                with self.subTest(record=kind, field=field), self.assertRaises(R.RecordMismatch) as error:
                    R.read_against(record, **{field: wrong})
                self.assertEqual(error.exception.field, field)
                self.assertEqual(error.exception.expected, wrong)
                self.assertIn(field, str(error.exception))
                self.assertNotEqual(error.exception.actual, wrong)

    def test_missing_identities_are_not_filled_from_current_revision(self):
        # The current CoversResult has no contract_revision; source records
        # without a requested identity are refused, not silently upgraded.
        for record, field in ((self.covers, 'contract_revision'), (self.repair, 'library_digest'),
                              (self.library, 'input_digest')):
            with self.subTest(field=field), self.assertRaises(R.RecordMismatch) as error:
                R.read_against(record, **{field: self.available[field]})
            self.assertIsNone(error.exception.actual)
        stored = copy.deepcopy(self.stored)
        stored['identities']['menu_order'] = 'planted'
        with self.assertRaises(R.RecordMismatch) as error:
            R.read_against(stored, order_digest=self.available['order_digest'])
        self.assertEqual((error.exception.field, error.exception.actual), ('order_digest', 'planted'))

    def test_conflicting_spellings_of_an_identity_are_a_mismatch(self):
        # The stored word keeps a stale recorded_at or identity; a second
        # spelling that agrees with the caller cannot override it.
        stale = {'input_digest': ('recorded_at',), 'menu_identity': ('identities', 'menu'),
                 'order_digest': ('identities', 'menu_order'),
                 'contract_revision': ('identities', 'contract_revision')}
        for field, path in stale.items():
            stored = copy.deepcopy(self.stored)
            target = stored if len(path) == 1 else stored[path[0]]
            target[path[-1]] = 'stale'
            stored[field] = self.available[field]
            with self.subTest(field=field), self.assertRaises(R.RecordMismatch) as error:
                R.read_against(stored, **{field: self.available[field]})
            self.assertEqual(error.exception.field, field)
            self.assertEqual(set(error.exception.actual), {'stale', self.available[field]})
        agreeing = dict(copy.deepcopy(self.stored), input_digest=self.available['input_digest'])
        self.assertIs(R.read_against(agreeing, input_digest=self.available['input_digest']), agreeing)

    def test_cancelled_cut_off_and_error_jobs_are_never_empty_lists(self):
        for status in ('cancelled', 'cut_off', 'error'):
            for result in (replace(self.repair, status=status, rows=None), replace(self.covers, status=status)):
                with self.subTest(status=status), self.assertRaises(R.IncompleteRecordError) as error:
                    R.read_against(result)
                self.assertEqual((error.exception.field, error.exception.actual), ('status', status))


class MenuRotationTests(TimedCase):
    def test_record_closure_and_j1_refuse_the_same_bad_menus(self):
        menu = census.menu()
        record = menu.record()
        checked = R.check_menu_closure(record)
        self.assertEqual(checked.identity, menu.identity)
        inverse = menu_order.inverse_index(record)
        removed = next(i for i in range(12, 60) if inverse[i] != i)
        missing_inverse = copy.deepcopy(record)
        del missing_inverse['base'][removed]
        first_bad = next(i for i, b in enumerate(missing_inverse['base']) if
            menu_order.item_key([[b['matrix'][j][k] for j in range(4)] for k in range(4)]) not in
            {menu_order.item_key(x['matrix']) for x in missing_inverse['base']})
        not_invariant = copy.deepcopy(record)
        not_invariant['base'] = [record['base'][i] for i in sorted(set(range(12)) | {12, inverse[12]})]
        identity_only = copy.deepcopy(record)
        identity_only['base'] = record['base'][:1]
        for rec, condition, index in ((missing_inverse, 'not inverse-closed', first_bad),
                                      (not_invariant, 'not A4-conjugation-invariant', 12),
                                      (identity_only, 'does not contain A4', 1)):
            rec['identity'] = menu_order.menu_identity(rec)
            with self.subTest(condition=condition), self.assertRaises(R.MenuClosureError) as error:
                R.check_menu_closure(rec)
            self.assertEqual((error.exception.condition, error.exception.index), (condition, index))
            with self.assertRaises(sim.TwistError):
                sim.TwistMenu.from_record(_END.ctx, rec)

    def test_a1_non_menu_rotation_and_unrepresentable_twist_change_nothing(self):
        before = _END.snapshot()
        with self.assertRaises(R.TwistRefused) as error:
            R.request_twist(_END, 0, [1, 0, 0], 13, menu=census.menu())
        self.assertIn('not an exact menu element', str(error.exception))
        self.assertEqual(_END.snapshot(), before)
        impossible = sim.UnrepresentableTwist(0, np.eye(4))
        self.assertEqual(_END.apply(impossible).status, 'uncertain')
        self.assertEqual(_END.snapshot(), before)
        with patch('sim.twists.cayley_axis_angle', return_value=(impossible, {})):
            with self.assertRaises(R.TwistRefused) as error:
                R.request_twist(_END, 0, [1, 0, 0], 13, menu=census.menu())
        self.assertEqual(error.exception.status, 'uncertain')
        self.assertEqual(_END.snapshot(), before)

    def test_eh03_identity_and_order_match_the_direct_fixture_record(self):
        text = (ROOT / 'docs/progress/1.0/jumbling-midstage.md').read_text(encoding='utf-8')
        identity = re.search(r'\| I_a \| 60 \| `([0-9a-f]{64})`', text).group(1)
        fixture = json.loads((JUMBLING / 'fixtures/wj-I_a.json').read_text(encoding='utf-8'))
        actual = census.restore.menu('I_a')
        self.assertEqual(actual.identity, identity)
        self.assertEqual(menu_order.order_digest(actual.record()), menu_order.order_digest(fixture['menu']))


class CLISmokeTests(TimedCase):
    def test_repository_out_or_extract_refused_before_any_computation(self):
        args = ([ '--out', str(ROOT / 'work/forbidden-word-check')],
                ['--out', '/tmp/en2-k-unused', '--extract', str(ROOT)])
        for command in args:
            with self.subTest(command=command), patch.object(W, 'run_cases') as run, \
                    patch.object(W, '_research', side_effect=AssertionError('imported research')):
                error = io.StringIO()
                with redirect_stderr(error), self.assertRaises(SystemExit) as stopped:
                    W.main(command)
                self.assertEqual(stopped.exception.code, 2)
                self.assertIn('must be outside the repository', error.getvalue())
                run.assert_not_called()

    def test_output_file_resolving_into_repository_is_refused_before_work(self):
        real = Path.resolve

        def resolve(path, strict=False):
            return ROOT / 'assets/manifest.json' if path.name == 'word-summary.json' else real(path, strict)

        with patch.object(Path, 'resolve', resolve), patch.object(W, 'run_cases') as run:
            with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                W.main(['--out', '/tmp/en2-k-unused'])
            run.assert_not_called()

    def test_unavailable_full_input_is_never_a_pass(self):
        # No draft inputs are read by tests. A missing extract is a reported
        # unperformed comparison, not a passing empty selection.
        rows, summary = W.run_cases(['b20'])
        self.assertEqual(rows[0]['status'], 'unavailable')
        self.assertIsNone(rows[0]['comparison'])
        self.assertFalse(summary['passed'])
        self.assertEqual(summary['compared'], 0)
        self.assertEqual(summary['unavailable'], ['b20'])

    def test_input_that_does_not_rebuild_is_an_error_not_a_pass(self):
        with patch.object(W, 'case_input', side_effect=ValueError('b12 setup digest: expected a, actual b')):
            rows, summary = W.run_cases(['b14'])
        self.assertEqual(rows[0]['status'], 'error')
        self.assertIn('b12 setup digest', rows[0]['reason'])
        self.assertIsNone(rows[0]['comparison'])
        self.assertFalse(summary['passed'])
        self.assertEqual((summary['errors'], summary['unavailable']), (['b14'], []))


def load_tests(loader, tests, pattern):
    if _IN_PROCESS:
        return unittest.TestSuite(loader.loadTestsFromTestCase(case) for case in
            (ResearchWordTests, ReleaseAgreementTests, PreviewCommitTests, RecordTests, MenuRotationTests, CLISmokeTests))
    from tests import isolated_numpy
    return isolated_numpy.suite('test_jumbling_word_check', 'word checks in an isolated NumPy process',
                                'NumPy is required for the jumbling word checks')


if __name__ == '__main__':
    unittest.main()
