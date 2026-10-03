"""Headless tests for the migration probes (tools/migration_probes): decision rule, backup bounds,
verdicts and path guards. The Windows probes themselves run only through run_probes.py."""
from __future__ import annotations

import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools' / 'migration_probes'))
import fixtures as F  # noqa: E402
import p1_source as S  # noqa: E402
import pipeline  # noqa: E402
import platform_ops as P  # noqa: E402

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
