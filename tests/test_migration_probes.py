"""Headless tests for the migration probes (tools/migration_probes): decision rule, backup bounds,
verdicts and path guards. The Windows probes themselves run only through run_probes.py."""
from __future__ import annotations

import hashlib
import json
import os
import shutil
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools' / 'migration_probes'))
import fixtures as F  # noqa: E402
import p1_source as S  # noqa: E402
import pipeline  # noqa: E402
import platform_ops as P  # noqa: E402
import run_probes  # noqa: E402
import sanitize  # noqa: E402

META = dict(size=10, links=1, directory=False, attributes=32, created=1, written=2, changed=3, id='a')
SNAPSHOT = {'.': dict(META, directory=True, attributes=16), 'engine.lock': dict(META, size=1),
            'session.sqlite3': dict(META)}
DIGESTS = {'engine.lock': 'a', 'session.sqlite3': 'b'}


def watch(**changes) -> P.WatchResult:
    result = P.WatchResult(armed_before_snapshot=True)
    for key, value in changes.items():
        setattr(result, key, value)
    return result


class FakeClock:
    def __init__(self):
        self.now = 0.0

    def __call__(self):
        return self.now


class DecisionTests(unittest.TestCase):
    def test_clean_attempt_is_accepted(self):
        decision = P.decide(watch(), SNAPSHOT, dict(SNAPSHOT), DIGESTS, dict(DIGESTS))
        self.assertTrue(decision.accepted)
        self.assertEqual(decision.reasons, [])

    def test_every_watch_failure_rejects_with_equal_snapshots(self):
        # MPR-03: the same rule for watch and deny mode; equal snapshots and digests never excuse these.
        cases = {
            'event': watch(events=[(1.0, 'modified', 'session.sqlite3-shm')]),
            'overflow': watch(overflows=1, overflow_forms=['ok-zero-bytes']),
            'watch-error': watch(errors=['error-87']),
            'watch-not-armed': P.WatchResult(armed_before_snapshot=False),
        }
        for reason, result in cases.items():
            with self.subTest(reason=reason):
                decision = P.decide(result, SNAPSHOT, dict(SNAPSHOT), DIGESTS, dict(DIGESTS))
                self.assertFalse(decision.accepted)
                self.assertIn(reason, decision.reasons)
                self.assertEqual(decision.snapshot_changes, [])
                self.assertEqual(decision.digest_changes, [])

    def test_snapshot_and_digest_differences_reject(self):
        final = {name: dict(meta) for name, meta in SNAPSHOT.items()}
        final['engine.lock']['size'] = 2
        final['session.sqlite3-shm'] = dict(META)
        decision = P.decide(watch(), SNAPSHOT, final, DIGESTS, dict(DIGESTS, **{'engine.lock': 'c'}))
        self.assertFalse(decision.accepted)
        self.assertEqual(decision.reasons, ['snapshot-diff', 'digest-diff'])
        self.assertEqual(decision.snapshot_changes, ['engine.lock:size', 'session.sqlite3-shm:added'])
        self.assertEqual(decision.digest_changes, ['engine.lock'])

    def test_decision_ignores_event_timing_and_names_no_writer(self):
        # MPR-06: the same events at any time give the same decision, and the decision has no writer field.
        early = P.decide(watch(events=[(-1e9, 'modified', 'engine.lock')]), SNAPSHOT, SNAPSHOT, DIGESTS, DIGESTS)
        late = P.decide(watch(events=[(1e9, 'modified', 'engine.lock')]), SNAPSHOT, SNAPSHOT, DIGESTS, DIGESTS)
        self.assertEqual(early, late)
        self.assertEqual(set(vars(early)), {'accepted', 'reasons', 'snapshot_changes', 'digest_changes'})


class StrategyVerdictTests(unittest.TestCase):
    def attempts(self, **override):
        records = [dict(passed=True, inconclusive=False, mode=mode, outcome=outcome)
                   for mode in S.MODES for outcome in S.OUTCOMES for _ in F.CASES]
        records[-1].update(override)
        return records

    def test_all_passed(self):
        self.assertEqual(S.strategy_verdict(self.attempts())['verdict'], 'pass')

    def test_one_deny_mode_overflow_fails_the_strategy(self):
        verdict = S.strategy_verdict(self.attempts(passed=False, reasons=['overflow']))
        self.assertEqual(verdict['verdict'], 'fail')

    def test_resource_bound_without_change_is_inconclusive(self):
        verdict = S.strategy_verdict(self.attempts(passed=False, inconclusive=True))
        self.assertEqual(verdict['verdict'], 'inconclusive')

    def test_no_attempts_is_not_a_pass(self):
        self.assertNotEqual(S.strategy_verdict([])['verdict'], 'pass')


class BackupBoundTests(unittest.TestCase):
    def setUp(self):
        self.clock = FakeClock()
        self.bound = P.BackupBound(clock=self.clock)

    def step(self, at, status, remaining):
        self.clock.now = at
        self.bound.check(status, remaining)

    def test_normal_trace_completes(self):
        remaining = 158
        for index in range(20):
            remaining = max(0, remaining - 8)
            self.step(index * 0.05, 0 if remaining else 101, remaining)

    def test_late_first_busy_is_not_a_stall(self):
        self.step(5.10, 0, 150)
        self.step(5.11, P.SQLITE_BUSY, 150)
        self.step(9.00, P.SQLITE_BUSY, 150)
        self.step(9.10, 0, 142)

    def test_progress_between_busy_intervals_resets_the_stall(self):
        for start in (0.0, 4.0, 8.0):
            self.step(start, P.SQLITE_BUSY, 150)
            self.step(start + 3.9, P.SQLITE_LOCKED, 150)
            self.step(start + 3.95, 0, 150 - int(start))

    def test_long_stall_is_bounded(self):
        self.step(1.0, P.SQLITE_BUSY, 150)
        with self.assertRaises(P.BackupStalled):
            self.step(6.5, P.SQLITE_BUSY, 150)

    def test_restart_loop_is_bounded(self):
        self.step(0.0, 0, 150)
        with self.assertRaises(P.BackupStalled) as raised:
            for index in range(P.BACKUP_NO_PROGRESS_STEPS):
                self.step(0.01 * (index + 1), 0, 150)
        self.assertEqual(str(raised.exception), 'backup-no-progress')

    def test_completion_at_the_deadline_is_not_rejected(self):
        self.step(1.0, 0, 8)
        self.step(P.BACKUP_DEADLINE_SECONDS + 2, 101, 0)

    def test_unfinished_backup_past_the_deadline_is_bounded(self):
        self.step(1.0, 0, 100)
        with self.assertRaises(P.BackupStalled) as raised:
            self.step(P.BACKUP_DEADLINE_SECONDS + 2, 0, 92)
        self.assertEqual(str(raised.exception), 'backup-deadline')


class PipelineVerdictTests(unittest.TestCase):
    def record(self, **override):
        base = dict(accepted=False, published=False, destination_unchanged=True, reasons=[], events=[],
                    snapshot_changes=[], digest_changes=[])
        base.update(override)
        return base

    def test_f21_requires_equal_snapshots(self):
        self.assertTrue(pipeline.verdict('F21-zero', self.record(reasons=['overflow'])))
        self.assertFalse(pipeline.verdict('F21-zero', self.record(reasons=['overflow', 'snapshot-diff'],
                                                                  snapshot_changes=['.:written'])))

    def test_publication_after_rejection_fails(self):
        self.assertFalse(pipeline.verdict('F15', self.record(reasons=['snapshot-diff'], published=True)))

    def test_f18_needs_both_sidecar_events(self):
        events = [('added', 'session.sqlite3-shm'), ('removed', 'session.sqlite3-shm')]
        self.assertTrue(pipeline.verdict('F18', self.record(reasons=['event'], events=events)))
        self.assertFalse(pipeline.verdict('F18', self.record(reasons=['event'], events=events[:1])))

    def test_baseline_must_publish(self):
        self.assertTrue(pipeline.verdict('baseline-null', self.record(accepted=True, published=True)))
        self.assertFalse(pipeline.verdict('baseline-null', self.record(accepted=True, published=False)))


class GuardTests(unittest.TestCase):
    def setUp(self):
        self.base = Path(tempfile.gettempdir()).resolve() / ('migration-probe-test-%d' % os.getpid())
        self.addCleanup(shutil.rmtree, self.base, True)

    def test_run_root_is_fresh_marked_and_confined(self):
        run_root = F.create_run_root(self.base / 'run')
        self.assertTrue((run_root / F.MARKER).is_file())
        with self.assertRaises(F.ProbeRefusal):
            F.create_run_root(self.base / 'run')
        inside = F.guard(run_root / 'a' / 'b', run_root)
        self.assertTrue(inside.is_relative_to(run_root))
        with self.assertRaises(F.ProbeRefusal):
            F.guard(self.base / 'elsewhere', run_root)
        with self.assertRaises(F.ProbeRefusal):
            F.guard(run_root, run_root)

    def test_unmarked_root_and_outside_bases_are_refused(self):
        (self.base / 'unmarked').mkdir(parents=True)
        with self.assertRaises(F.ProbeRefusal):
            F.guard(self.base / 'unmarked' / 'x', self.base / 'unmarked')
        with self.assertRaises(F.FixtureRefusal):
            F.create_run_root(Path(ROOT.anchor) / 'migration-probe-outside')

    def test_fixture_child_refuses_outside_and_existing_directories(self):
        # Review finding MPR-A02: refused before any 0.4 Session is constructed.
        run_root = F.create_run_root(self.base / 'run')
        outside = self.base / 'outside-session'
        with self.assertRaises(F.ProbeRefusal):
            F._child('clean', run_root, [outside])
        self.assertFalse(outside.exists())
        existing = run_root / 'existing'
        existing.mkdir()
        with self.assertRaises(F.ProbeRefusal):
            F._child('clean', run_root, [existing])
        self.assertEqual(list(existing.iterdir()), [])

    def test_request_child_refuses_outside_or_existing_results(self):
        run_root = F.create_run_root(self.base / 'run')
        work = run_root / 'work'
        work.mkdir()
        request = work / 'request.json'
        request.write_text(json.dumps(dict(run_root=str(run_root))), encoding='utf-8')
        outside = self.base / 'outside-result.json'
        with self.assertRaises(F.ProbeRefusal):
            F.load_request(request, outside)
        (work / 'result.json').write_text('{}', encoding='utf-8')
        with self.assertRaises(F.ProbeRefusal):
            F.load_request(request, work / 'result.json')
        loaded, root, result = F.load_request(request, work / 'fresh.json')
        self.assertEqual((root, result), (run_root, work / 'fresh.json'))
        stray = self.base / 'stray-request.json'
        stray.write_text(json.dumps(dict(run_root=str(run_root))), encoding='utf-8')
        with self.assertRaises(F.ProbeRefusal):
            F.load_request(stray, work / 'other.json')


@unittest.skipUnless(os.name == 'nt', 'Windows lock calls')
class LockGuardOrderTests(unittest.TestCase):
    """Review finding MPR-A01: the WriteGuard is taken before the lock and released after it."""

    def setUp(self):
        calls = self.calls = []

        class FakeGuard:
            def __init__(self, directory, entries):
                calls.append('guard')

            def release(self):
                calls.append('guard-release')

        class FakeHandle:
            def close(self):
                calls.append('close')

        def open_handle(*args, **kwargs):
            calls.append('open')
            return FakeHandle()

        for target, value in (('WriteGuard', FakeGuard),):
            patcher = mock.patch.object(P, target, value)
            patcher.start()
            self.addCleanup(patcher.stop)
        for name, value in (('open_handle', open_handle), ('lock_range', lambda *a: calls.append('lock')),
                            ('unlock_range', lambda *a: calls.append('unlock'))):
            patcher = mock.patch.object(P.W, name, value)
            patcher.start()
            self.addCleanup(patcher.stop)

    def test_guard_brackets_the_lock(self):
        P.ReadOnlyLock(ROOT, guard_entries={}).release()
        self.assertEqual(self.calls, ['guard', 'open', 'lock', 'unlock', 'close', 'guard-release'])

    def test_refused_lock_releases_the_guard(self):
        def refuse(*args):
            raise OSError(None, 'lock violation', None, P.W.ERROR_LOCK_VIOLATION)
        with mock.patch.object(P.W, 'lock_range', refuse):
            with self.assertRaises(P.LockRefused) as caught:
                P.ReadOnlyLock(ROOT, guard_entries={})
        self.assertEqual(caught.exception.reason, 'engine-running')
        self.assertEqual(self.calls, ['guard', 'open', 'close', 'guard-release'])


@unittest.skipUnless(os.name == 'nt', 'Windows handles')
class StagedSourceTests(unittest.TestCase):
    """C3 (owner decision 2026-10-03): only the staged copy is opened by SQLite."""

    def test_source_directory_is_only_read(self):
        base = Path(tempfile.gettempdir()).resolve() / ('migration-probe-staged-%d' % os.getpid())
        self.addCleanup(shutil.rmtree, base, True)
        source = base / 'source'
        source.mkdir(parents=True)
        database = source / 'session.sqlite3'
        connection = sqlite3.connect(database)
        connection.execute('CREATE TABLE t (x)')
        connection.execute('INSERT INTO t VALUES (7)')
        connection.commit()
        connection.close()
        before = sorted(item.name for item in source.iterdir())
        staged = P.StagedSource(database, base / 'staged')
        try:
            value = staged.connection.execute('SELECT x FROM t').fetchone()[0]
        finally:
            staged.close()
        self.assertEqual(value, 7)
        self.assertEqual(sorted(item.name for item in source.iterdir()), before)
        self.assertEqual(staged.copied, {'session.sqlite3': hashlib.sha256(database.read_bytes()).hexdigest()})


class SanitizeTests(unittest.TestCase):
    def test_paths_and_identifiers_are_replaced(self):
        run_root = Path(tempfile.gettempdir()).resolve() / 'probe-run'
        home = str(Path.home())
        value = {'source': str(run_root / 'p1' / 'x'), 'uri': (run_root / 'db').as_uri() + '?mode=ro',
                 'error': 'OperationalError: unable to open %s/other' % home,
                 'account': 'S-1-5-21-1111111111-2222222222-3333333333-1001', 'n': 3}
        clean = sanitize.sanitize(value, {'<run>': run_root})
        self.assertEqual(clean['source'], '<run>' + os.sep + 'p1' + os.sep + 'x')
        self.assertTrue(clean['uri'].startswith('<run>'))
        self.assertEqual(clean['account'], '<sid>')
        self.assertEqual(clean['n'], 3)
        self.assertEqual(sanitize.leaks(repr(clean).replace('\\', '/')), [])

    def test_leaks_are_found(self):
        self.assertIn('absolute-path', sanitize.leaks('C:/x/y'))
        self.assertIn('sid', sanitize.leaks('S-1-5-21-1-2-3-4'))
        self.assertEqual(sanitize.leaks('<run>/p1 session.sqlite3-shm:added'), [])

    def test_uri_and_forward_slash_unc_forms(self):
        # Review finding MPB-01.
        forms = ['file://review-host/share/private.bin', '//review-host/share/private.bin',
                 'file:///C:/x/private.bin', '\\\\?\\C:\\x\\private.bin', '\\\\review-host\\share\\x']
        for form in forms:
            self.assertIn('absolute-path', sanitize.leaks(form), form)
            clean = sanitize.sanitize({'error': 'open failed: ' + form}, {})
            self.assertNotIn('review-host', clean['error'])
            self.assertNotIn('private.bin', clean['error'])
            self.assertEqual(sanitize.leaks(json.dumps(clean)), [], form)
        self.assertEqual(sanitize.leaks('see https://www.sqlite.org/wal.html'), [])

    def test_short_identities_and_every_sid_family(self):
        # Review finding MPB-02.
        environment = dict(USERNAME='Qx', COMPUTERNAME='Z9', USERDOMAIN='D8')
        with mock.patch.dict(os.environ, environment), \
                mock.patch('pathlib.Path.home', return_value=Path('C:/Users/Qx')):
            text = 'User Qx on Z9 in D8; Qxylophone stays'
            self.assertEqual(sanitize.leaks(text), ['private-word'])
            clean = sanitize.sanitize({'e': text, 's': 'S-1-12-1-1111111111-2222222222-3333333333-4444444444'}, {})
            self.assertEqual(clean['e'], 'User <private> on <private> in <private>; Qxylophone stays')
            self.assertEqual(clean['s'], '<sid>')
            self.assertEqual(sanitize.leaks(json.dumps(clean)), [])

    def test_merged_output_is_checked_whole(self):
        # Review finding MPB-03: an older probe entry with a private path blocks the write.
        base = Path(tempfile.gettempdir()).resolve() / ('migration-probe-merge-%d' % os.getpid())
        base.mkdir()
        self.addCleanup(shutil.rmtree, base, True)
        out = base / 'results.json'
        old = json.dumps(dict(runs={}, commands={}, probes={'p1': dict(run='r0', result=dict(
            error='R:/Users/SyntheticPerson/private.db'))}))
        out.write_text(old, encoding='utf-8')
        public = dict(run='r1', environment={}, probes={'p2': dict(passed=True)}, commands={'p2': 'command'})
        self.assertIn('absolute-path', run_probes.merge_out(out, public))
        self.assertEqual(out.read_text(encoding='utf-8'), old)
        out.write_text(json.dumps(dict(runs={}, commands={}, probes={})), encoding='utf-8')
        self.assertEqual(run_probes.merge_out(out, public), [])
        self.assertEqual(json.loads(out.read_text(encoding='utf-8'))['probes']['p2']['run'], 'r1')

    def test_json_escaping_hides_nothing(self):
        # Review finding MPB-03, verification round 1: escaped newlines and non-ASCII names.
        base = Path(tempfile.gettempdir()).resolve() / ('migration-probe-escape-%d' % os.getpid())
        base.mkdir()
        self.addCleanup(shutil.rmtree, base, True)
        out = base / 'results.json'
        public = dict(run='r1', environment={}, probes={'p2': dict(passed=True)}, commands={'p2': 'command'})
        cases = [({}, 'open failed:\n//review-host/share/private.bin', 'absolute-path'),
                 ({}, 'account\nS-1-5-21-1111111111-2222222222-3333333333-1001', 'sid'),
                 (dict(USERNAME='Qx'), 'user\nQx', 'private-word'),
                 (dict(USERNAME='Jörg'), 'User Jörg', 'private-word')]
        for environment, error, kind in cases:
            old = json.dumps(dict(runs={}, commands={}, probes={'p1': dict(run='r0', result=dict(error=error))}))
            out.write_text(old, encoding='utf-8')
            with mock.patch.dict(os.environ, environment):
                self.assertIn(kind, run_probes.merge_out(out, public), error)
                self.assertIn(kind, sanitize.leaks_in({'key': [error]}), error)
            self.assertEqual(out.read_text(encoding='utf-8'), old)


class AttemptVerdictTests(unittest.TestCase):
    """Review finding MPR-A04: a C3 copy that differs from the bytes under the lock never passes."""

    def run_attempt(self, outcome: str, copied_digest: str) -> dict:
        class Watch:
            def __init__(self, source):
                pass

            def start(self):
                pass

            def stop_and_drain(self):
                return watch()

        class Lock:
            def __init__(self, source, guard_entries=None):
                pass

            def release(self):
                pass

        class Staged:
            copied = {'session.sqlite3': copied_digest}
            statuses = [0]

            def model(self):
                return 'model'

            def backup(self, copy, fail_after_first_step, after_first_step=None):
                if fail_after_first_step:
                    raise P.InjectedFailure('injected')

            def close(self):
                pass

        base = Path(tempfile.gettempdir()).resolve() / ('migration-probe-attempt-%d' % os.getpid())
        self.addCleanup(shutil.rmtree, base, True)
        with mock.patch.object(P, 'DirectoryWatch', Watch), mock.patch.object(P, 'ReadOnlyLock', Lock), \
                mock.patch.object(P, 'snapshot', lambda source: SNAPSHOT), \
                mock.patch.object(P, 'digests', lambda *args, **kwargs: DIGESTS), \
                mock.patch.object(P, 'open_source_for_backup', lambda *args, **kwargs: Staged()), \
                mock.patch.object(S, 'verify_copy', lambda copy, expected: dict(ok=True)):
            return S.attempt(base / 'source', base / ('copy-%s-%s' % (outcome, copied_digest)), 'C3', outcome,
                             dict(model='model', head=1, state_hash='h'))

    def test_digest_mismatch_fails_both_outcomes(self):
        for outcome in ('success', 'failure'):
            matching = self.run_attempt(outcome, DIGESTS['session.sqlite3'])
            self.assertTrue(matching['passed'], outcome)
            different = self.run_attempt(outcome, 'other')
            self.assertFalse(different['staged']['matches_lock_digests'])
            self.assertTrue(different['preserved'])
            self.assertFalse(different['passed'], outcome)
            self.assertEqual(S.strategy_verdict([matching, different])['verdict'], 'fail')


@unittest.skipUnless(os.name == 'nt', 'Windows watch classification')
class WatchRecordTests(unittest.TestCase):
    def test_completion_forms(self):
        watch_ = P.DirectoryWatch(ROOT)
        watch_.inject_zero_completion()
        watch_.record(True, 0, 1022)
        watch_.record(False, 0, 1022)
        watch_.inject_error()
        self.assertEqual(watch_.result.overflows, 3)
        self.assertEqual(watch_.result.overflow_forms,
                         ['injected-ok-zero-bytes', 'ok-zero-bytes-last-error-1022', 'error-1022'])
        self.assertEqual(watch_.result.errors, ['injected-error-87'])
        self.assertFalse(watch_.result.coverage_valid)


if __name__ == '__main__':
    unittest.main(verbosity=2)
