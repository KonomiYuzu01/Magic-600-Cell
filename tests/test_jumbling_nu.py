"""EN2-I: ν references, EH-11 corrections and A5 label checkpoints.

Only setUpModule imports NumPy/J1/research. Shared unittest commands use one
isolated worker. Every runtime write is audited; outputs use external temp
directories. Timings stay in _CLASS_SECONDS/_MEASUREMENTS for the packet
report; success prints only unittest's summary. JUMBLING_FULL_REPLAY=1 adds
all 40 CERT-LB-V2 word replays (the default checks its complete input set and
replays a short word of each encoding).
"""

import sys

sys.dont_write_bytecode = True

from contextlib import redirect_stderr
import copy
from dataclasses import replace
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import random
import subprocess
import tempfile
from time import perf_counter
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / 'research' / 'jumbling'
_IN_PROCESS = __name__ == '__main__'
_FULL = os.environ.get('JUMBLING_FULL_REPLAY') == '1'
_AUDIT = False
_TEMP_ROOT = None
_CLASS_SECONDS = {}
_MEASUREMENTS = {'corrections': []}


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
        raise AssertionError('the ν tests must not create links')
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
            raise AssertionError('the ν tests attempted a write inside the repository')


def setUpModule():
    global np, sim, census, nu, C, R, j1_state, store, menu_order, _CTX, _MENU, _LIB, _E3
    global _TEMP_ROOT, _AUDIT
    started = perf_counter()
    temporary = tempfile.TemporaryDirectory(prefix='jumbling-nu-')
    _TEMP_ROOT = Path(temporary.name).resolve()
    if _TEMP_ROOT.is_relative_to(ROOT):
        temporary.cleanup()
        _TEMP_ROOT = None
        raise AssertionError('the temporary directory must be outside the repository')

    def cleanup():
        global _AUDIT, _TEMP_ROOT
        try:
            temporary.cleanup()
        finally:
            _AUDIT, _TEMP_ROOT = False, None

    unittest.addModuleCleanup(cleanup)
    sys.addaudithook(_no_repository_writes)
    _AUDIT = True
    try:
        import numpy as np
    except ImportError as error:
        raise unittest.SkipTest('NumPy is required for the ν and correction references') from error
    sys.path.insert(0, str(JUMBLING))
    from differential.leads import census  # first: isolate the research centroid cache
    from differential.references import nu, corrections as C, nu_refs as R
    from differential import j1_state, menu_order
    from differential.candidate import store
    import sim
    _CTX, _MENU = census.restore.ctx, census.menu()
    _LIB = C._library()
    _E3, _ = R.table_state(R.TABLES['E3'])
    if Path(census.seams.CACHE).resolve().is_relative_to(ROOT):
        raise AssertionError('the centroid cache is inside the repository')
    _MEASUREMENTS['setup_seconds'] = perf_counter() - started


class TimedCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._class_started = perf_counter()

    @classmethod
    def tearDownClass(cls):
        _CLASS_SECONDS[cls.__name__] = perf_counter() - cls._class_started

    def unchanged(self, state, before):
        self.assertEqual(state.snapshot(), before)


class RetainedWordTests(TimedCase):
    def _label_exit(self, state):
        certificate = nu.label_certificate(state)
        self.assertEqual(certificate['nu'], list(nu.ZERO))
        self.assertEqual(certificate['pose_digest'], state.digest())
        self.assertEqual(certificate['labels_digest'], nu.labels_digest(nu.labels(state)))
        identities = {'model': copy.deepcopy(_CTX.data.identity), 'menu': menu_order.I_A_IDENTITY,
                      'menu_order': menu_order.I_A_ORDER_DIGEST, 'contract_revision': nu.CONTRACT_REVISION}
        record = dict(j1_state.state_record(state), revision=len(state.journal))
        opened = store.cross_on({'format': store.SESSION_FORMAT, 'revision': 0, 'retained': {}},
                                {'kind': 'word', 'word': []}, identities, record)
        opened['revision'] = len(state.journal) + 1
        opened['jumbling']['episodes'][0]['journal'] = copy.deepcopy(state.journal)
        closed = store.cross_off(opened, certificate, j1_state.state_record(state), {})
        store.validate(closed)
        self.assertEqual(closed['jumbling']['episodes'][0]['end']['certificate'], certificate)

    def test_solved_and_three_seeded_retained_words(self):
        solved = sim.State(_CTX, menu=_MENU)
        self.assertEqual(nu.nu(solved), nu.ZERO)
        self.assertTrue(solved.checkpoint([])['checkpoint'])
        self._label_exit(solved)
        for seed in (1103, 2718, 9011):
            with self.subTest(seed=seed):
                rng = random.Random(seed)
                word = [rng.choice((-1, 1)) * rng.randint(1, 1200) for _ in range(30)]
                state = sim.State(_CTX, menu=_MENU)
                for mid in word:
                    self.assertTrue(state.apply(sim.primitive(_CTX, mid)).applied)
                checkpoint = state.checkpoint(word)
                self.assertTrue(checkpoint['checkpoint'], checkpoint)
                self.assertTrue(checkpoint['replay_equal'])
                self.assertEqual(nu.nu(state), nu.ZERO)
                self._label_exit(state)


class TableNuTests(TimedCase):
    def test_three_tables_digest_lattice_and_both_recorded_rows(self):
        matched = R.table_nu_rows()
        self.assertEqual(len(matched), 3)
        self.assertEqual({row.path for row in matched}, set(R.TABLES.values()))
        for path in R.TABLES.values():
            with self.subTest(table=path.name, parent=path.parent.name):
                state, record = R.table_state(path)
                self.assertEqual(state.digest(), record['digest'])
                self.assertTrue(state.is_lattice())
                comparison = R.compare_table(path)
                self.assertTrue(comparison.ok, comparison.difference)
                self.assertEqual(comparison.value, comparison.table_row)
                self.assertEqual(comparison.value, comparison.summary_row)
                self.assertEqual(nu.nu(state), comparison.value)
                nu.check_fixed(comparison.value)

    def test_planted_coordinate_names_each_reference_and_both_values(self):
        path = R.TABLES['E3']
        rows = R.table_nu_rows()
        first = next(row for row in rows if row.path == path)
        changed = list(first.row)
        changed[nu.NAMES.index('b4')] ^= 1
        planted = tuple(replace(row, row=tuple(changed)) if row.path == path else row for row in rows)
        before = _E3.snapshot()
        with patch.object(R, 'table_nu_rows', return_value=planted):
            result = R.compare_table(path)
        self.assertFalse(result.ok)
        self.assertEqual(result.difference, {'reference': 'TABLE-NU', 'coordinate': 'b4',
                                            'candidate': first.row[4], 'reference_value': changed[4]})
        summary = R.restore_nu_rows()
        digest = _E3.digest()
        summary[digest] = tuple(changed)
        with patch.object(R, 'restore_nu_rows', return_value=summary):
            result = R.compare_table(path)
        self.assertFalse(result.ok)
        self.assertEqual(result.difference, {'reference': 'restore-summary.json:nu_input', 'coordinate': 'b4',
                                            'candidate': first.row[4], 'reference_value': changed[4]})
        self.unchanged(_E3, before)

    def test_plain_table_and_planted_state_digest(self):
        original = json.loads(gzip.decompress(R.TABLES['E3'].read_bytes()))
        target = _TEMP_ROOT / 'pose-table.json'
        target.write_text(json.dumps(original), encoding='utf-8')
        state, record = R.table_state(target)
        self.assertEqual(state.digest(), record['digest'])
        record['digest'] = '0' * 64
        target.write_text(json.dumps(record), encoding='utf-8')
        with self.assertRaises(R.TableDigestError) as refused:
            R.table_state(target)
        self.assertEqual(refused.exception.candidate, original['digest'])
        self.assertEqual(refused.exception.reference, '0' * 64)
        self.assertIn(original['digest'], str(refused.exception))
        self.assertIn('0' * 64, str(refused.exception))

    def test_unmatched_table_and_unmatched_reference_are_errors(self):
        target = _TEMP_ROOT / 'table-nu.txt'
        records = [json.loads(line) for line in R.TABLE_NU.read_text(encoding='utf-8').splitlines()]
        target.write_text('\n'.join(json.dumps(row) for row in records[1:]), encoding='utf-8')
        with patch.object(R, 'TABLE_NU', target), self.assertRaisesRegex(R.ReferenceDataError, 'matches no TABLE-NU row'):
            R.table_nu_rows()
        extra = dict(records[0], sha256='0' * 64)
        target.write_text('\n'.join(json.dumps(row) for row in records + [extra]), encoding='utf-8')
        with patch.object(R, 'TABLE_NU', target), self.assertRaisesRegex(R.ReferenceDataError, 'row matches no table'):
            R.table_nu_rows()


class LibraryTests(TimedCase):
    def test_pinned_digest_all_36_rows_records_and_letters(self):
        self.assertEqual(_LIB.sha256, C.LIBRARY_SHA256)
        self.assertEqual(len(_LIB), 36)
        self.assertEqual([item.id for item in _LIB], list(range(36)))
        for correction in _LIB:
            with self.subTest(cid=correction.id):
                self.assertEqual(len(correction.row), 45)
                self.assertEqual(len(correction.records), correction.length)
                self.assertEqual(len(correction.letters), correction.length)
                self.assertTrue(correction.family)
                nu.check_fixed(correction.row)
        summary = json.loads(R.RESTORE_SUMMARY.read_text(encoding='utf-8'))
        self.assertEqual(_LIB[0].row, nu.row_from_research(summary['nu_w9']))

    def test_one_changed_decompressed_byte_names_both_digests(self):
        raw = bytearray(gzip.decompress(C.LIBRARY.read_bytes()))
        raw[0] ^= 1  # invalid JSON too: the digest must be checked before parsing
        actual = hashlib.sha256(raw).hexdigest()
        target = _TEMP_ROOT / 'bad-library.json.gz'
        target.write_bytes(gzip.compress(raw))
        with self.assertRaises(C.LibraryDigestError) as refused:
            C.load_library(target)
        self.assertEqual(refused.exception.candidate, actual)
        self.assertEqual(refused.exception.reference, C.LIBRARY_SHA256)
        self.assertIn(actual, str(refused.exception))
        self.assertIn(C.LIBRARY_SHA256, str(refused.exception))


class CorrectionRowTests(TimedCase):
    def test_every_bounded_row_and_c5_28_from_solved_and_E3(self):
        selected = [item.id for item in _LIB if item.length <= 512] + [30]
        self.assertEqual(selected, list(range(28)) + [30])
        for name, state in (('solved', sim.State(_CTX, menu=_MENU)), ('E3', _E3)):
            before = state.snapshot()
            value = nu.nu(state)
            for cid in selected:
                with self.subTest(start=name, correction=cid, length=_LIB[cid].length):
                    started = perf_counter()
                    result = C.check_row(state, cid)
                    _MEASUREMENTS['corrections'].append({'from': name, 'id': cid, 'length': _LIB[cid].length,
                                                         'seconds': perf_counter() - started})
                    self.assertTrue(result.ok, result.difference)
                    self.assertEqual(result.before, value)
                    self.assertEqual(result.row, _LIB[cid].row)
                    self.assertEqual(result.after, nu.add(value, _LIB[cid].row))
                    if name == 'solved':
                        self.assertEqual(result.after, _LIB[cid].row)
                    self.unchanged(state, before)

    def test_planted_row_fails_names_correction_and_coordinate_without_mutation(self):
        cid = 2
        changed = list(_LIB[cid].row)
        changed[4] ^= 1
        rows = tuple(replace(item, row=tuple(changed)) if item.id == cid else item for item in _LIB)
        for state in (sim.State(_CTX, menu=_MENU), _E3):
            with self.subTest(digest=state.digest()):
                before = state.snapshot()
                with patch.object(C, '_library', return_value=replace(_LIB, corrections=rows)):
                    result = C.check_row(state, cid)
                self.assertFalse(result.ok)
                self.assertEqual(result.difference, {'correction_id': cid, 'reference': 'library',
                                                     'coordinate': 'b4', 'candidate': 0, 'reference_value': 1})
                self.unchanged(state, before)

    def test_refused_prefix_rolls_back_the_whole_set(self):
        state = C._clone(_E3)
        before = state.snapshot()
        original = sim.State.apply
        count = 0

        def refuse_second_correction(trial, twist, *args, **kwargs):
            nonlocal count
            count += 1
            if count == _LIB[0].length + 2:
                from sim.state import Outcome
                return Outcome('blocked', twist.grip)
            return original(trial, twist, *args, **kwargs)

        # Force fresh prefix witnesses so the planted refusal is observed.
        C._lattice_action.cache_clear()
        with patch.object(sim.State, 'apply', refuse_second_correction):
            with self.assertRaises(C.InadmissiblePrefix) as refused:
                C.play(state, [0, 2])
        self.assertEqual(refused.exception.correction_id, 2)
        self.assertEqual(refused.exception.letter_index, 1)
        self.assertEqual(refused.exception.status, 'blocked')
        self.assertIn('correction 2, letter index 1', str(refused.exception))
        self.unchanged(state, before)
        with patch.object(sim.State, 'apply', side_effect=AssertionError('played before validating ids')):
            with self.assertRaisesRegex(ValueError, 'unknown correction id: 36'):
                C.play(state, [0, 36])
        self.unchanged(state, before)

    def test_transported_action_equals_literal_J1_poses_and_journal(self):
        # Independent literal replay checks the shape-invariance shortcut,
        # including exact frames and every collateral piece, not just ν.
        for cid in (0, 2):
            with self.subTest(correction=cid):
                literal = C._clone(_E3)
                for record in _LIB[cid].records:
                    outcome = literal.apply(sim.Twist.from_record(_CTX, record))
                    self.assertTrue(outcome.applied)
                transported = C._clone(_E3)
                C.play(transported, _LIB[cid])
                self.assertEqual(transported.digest(), literal.digest())
                self.assertEqual(transported.snapshot(), literal.snapshot())
                played = C._clone(_E3)
                C.play(played, _LIB[cid], literal=True)
                self.assertEqual(played.snapshot(), literal.snapshot())
                self.assertEqual(C.check_row(_E3, cid, literal=True), C.check_row(_E3, cid))


class DraftedSetTests(TimedCase):
    def test_named_set_in_two_orders_matches_row_sum_without_search(self):
        value = nu.nu(_E3)
        ids = [20, 22, 27]
        with patch.object(sim.State, 'apply', side_effect=AssertionError('drafted_nu played a twist')):
            drafted = C.drafted_nu(value, ids)
            self.assertEqual(C.drafted_nu(value, []), value)
        expected = value
        for cid in ids:
            expected = nu.add(expected, _LIB[cid].row)
        self.assertEqual(drafted, expected)
        before = _E3.snapshot()
        for order in (ids, [27, 20, 22]):
            with self.subTest(order=order):
                state = C._clone(_E3)
                self.assertEqual(C.play(state, order), drafted)
                self.assertEqual(nu.nu(state), drafted)
                self.assertEqual(C.drafted_nu(value, order), drafted)
                self.assertEqual(len(state.journal), sum(_LIB[cid].length for cid in order))
        self.unchanged(_E3, before)


class FixedCoordinateTests(TimedCase):
    def test_fixed_bits_are_consistency_errors_other_coordinates_are_allowed(self):
        for name in ('b0', 'f6'):
            value = list(nu.ZERO)
            value[nu.NAMES.index(name)] = 1
            with self.subTest(name=name), self.assertRaisesRegex(nu.NuConsistencyError, name):
                nu.check_fixed(value)
        for name in ('b4', 'C5_17'):
            value = list(nu.ZERO)
            value[nu.NAMES.index(name)] = 1
            nu.check_fixed(value)

    def test_global_order_three_displaces_centres_and_off_lattice_is_undefined(self):
        kp = _CTX.kplus
        identity = np.arange(600)
        rotation = next(k for k, p in enumerate(kp.perms) if k and
                        np.array_equal(p[p[p]], identity))
        permutation = kp.perms[rotation]
        self.assertFalse(np.array_equal(permutation, identity))
        np.testing.assert_array_equal(permutation[permutation[permutation]], identity)
        global_state = sim.State.from_global_rotation(_CTX, rotation)
        self.assertTrue(global_state.is_lattice())
        self.assertFalse(nu.centres_home(global_state))
        before = global_state.snapshot()
        with self.assertRaises(nu.LabelCheckpointRefused) as refused:
            nu.label_certificate(global_state)
        self.assertEqual(refused.exception.fact, 'centre_displaced')
        self.unchanged(global_state, before)
        state = sim.State(_CTX, menu=_MENU)
        twist = next(t for t in _MENU.for_cap(0) if not t.retained)
        self.assertTrue(state.apply(twist).applied)
        self.assertFalse(state.is_lattice())
        before = state.snapshot()
        with self.assertRaises(nu.LabelCheckpointRefused) as refused:
            nu.label_certificate(state)
        self.assertEqual(refused.exception.fact, 'off_lattice')
        with self.assertRaises(nu.NuUndefined):
            nu.nu(state)
        with patch.object(sim.State, 'apply', side_effect=AssertionError('off-lattice row check played a twist')):
            with self.assertRaises(nu.NuUndefined):
                C.check_row(state, 0)
        self.unchanged(state, before)


class CertificateTests(TimedCase):
    def test_complete_cert_inputs_and_both_encodings_or_full_replay(self):
        pairs = R.cert_words()
        self.assertEqual(len(pairs), 40)
        self.assertEqual(sum('letters' in word for word, _ in pairs), 27)
        self.assertEqual(sum('records' in word for word, _ in pairs), 13)
        selected = (pairs if _FULL else
                    tuple(next(pair for pair in pairs if kind in pair[0] and pair[1]['length'] <= 9)
                          for kind in ('letters', 'records')))
        for word, record in selected:
            with self.subTest(word=word['name']):
                result = R.replay_cert_word(word, record)
                self.assertTrue(result['ok'], result['difference'])
                self.assertTrue(result['admissible_every_prefix'])
                self.assertTrue(result['end_on_lattice'])
                self.assertTrue(result['nu_agrees_with_certificate'])
        word, record = next(pair for pair in pairs if pair[1]['length'] <= 9)
        changed = copy.deepcopy(record)
        changed['nu']['4'] = [1, 0]
        result = R.replay_cert_word(word, changed)
        self.assertFalse(result['ok'])
        self.assertEqual(result['difference'], {'word': word['name'], 'reference': 'CERT-LB-V2',
                                                'coordinate': 'b4', 'candidate': 0, 'reference_value': 1})


class CLISmokeTests(TimedCase):
    def test_repository_output_refused_before_loading_or_replay(self):
        for module in (C, R):
            with self.subTest(module=module.__name__), patch.object(R, '_runtime') as runtime:
                error = io.StringIO()
                with redirect_stderr(error), self.assertRaises(SystemExit) as stopped:
                    module.main(['--out', str(ROOT / 'work' / 'forbidden-nu-output')])
                self.assertEqual(stopped.exception.code, 2)
                self.assertIn('--out must be outside the repository', error.getvalue())
                runtime.assert_not_called()

    def test_output_file_resolving_inside_repository_is_refused(self):
        original = Path.resolve
        for module, name in ((C, 'corrections-solved.jsonl'), (R, 'table-nu.json')):
            def resolve(path, strict=False, name=name):
                return ROOT / 'assets' / 'manifest.json' if path.name == name else original(path, strict)

            with self.subTest(module=module.__name__), patch.object(Path, 'resolve', resolve), \
                    patch.object(R, '_runtime') as runtime:
                error = io.StringIO()
                with redirect_stderr(error), self.assertRaises(SystemExit):
                    module.main(['--out', str(_TEMP_ROOT)])
                self.assertIn(f'{name} resolves inside it', error.getvalue())
                runtime.assert_not_called()

    def test_corrections_script_two_rows_and_summary(self):
        target = _TEMP_ROOT / 'cli'
        # Run the actual script as __main__, with the same audit in its child
        # process too (Python audit hooks do not survive subprocess creation).
        worker = '\n'.join((
            'import importlib, runpy, sys',
            "checks = importlib.import_module('tests.test_jumbling_nu')",
            'checks._TEMP_ROOT = checks.Path(sys.argv[2]).resolve()',
            'checks._AUDIT = True',
            'sys.addaudithook(checks._no_repository_writes)',
            "sys.argv = [sys.argv[1], '--ids', '0', '2', '--out', sys.argv[2]]",
            'try:',
            "    runpy.run_path(sys.argv[0], run_name='__main__')",
            'finally:',
            '    checks._AUDIT = False',
        ))
        completed = subprocess.run([sys.executable, '-B', '-c', worker, str(Path(C.__file__)), str(target)],
                                   cwd=ROOT, capture_output=True, text=True)
        self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
        self.assertEqual(completed.stdout, '')
        self.assertEqual(completed.stderr, '')
        self.assertEqual({path.name for path in target.iterdir()},
                         {'corrections-solved.jsonl', 'corrections-solved-summary.json'})
        rows = [json.loads(line) for line in (target / 'corrections-solved.jsonl').read_text(encoding='utf-8').splitlines()]
        self.assertEqual([row['id'] for row in rows], [0, 2])
        for row in rows:
            self.assertLessEqual({'id', 'length', 'seconds', 'nu_before', 'nu_after', 'ok'}, set(row))
            self.assertEqual(row['length'], _LIB[row['id']].length)
            self.assertEqual(row['nu_before'], {})
            self.assertEqual(row['nu_after'], nu.nonzero(_LIB[row['id']].row))
            self.assertTrue(row['ok'])
            self.assertGreaterEqual(row['seconds'], 0)
        summary = json.loads((target / 'corrections-solved-summary.json').read_text(encoding='utf-8'))
        self.assertEqual((summary['requested'], summary['checked'], summary['passed'], summary['failed']), (2, 2, 2, 0))
        self.assertEqual(summary['library_sha256'], C.LIBRARY_SHA256)
        self.assertEqual(summary['numpy'], np.__version__)
        self.assertEqual(summary['python'], '.'.join(map(str, sys.version_info[:3])))
        self.assertRegex(summary['commit'] or '0' * 40, r'^[0-9a-f]{40}$')
        self.assertTrue(summary['ok'])


def load_tests(loader, tests, pattern):
    if _IN_PROCESS:
        return unittest.TestSuite(loader.loadTestsFromTestCase(case) for case in
                                  (RetainedWordTests, TableNuTests, LibraryTests, CorrectionRowTests,
                                   DraftedSetTests, FixedCoordinateTests, CertificateTests, CLISmokeTests))
    from tests import isolated_numpy
    return isolated_numpy.suite('test_jumbling_nu', 'ν and correction references in an isolated NumPy process',
                                'NumPy is required for the ν and correction references')


if __name__ == '__main__':
    unittest.main()
