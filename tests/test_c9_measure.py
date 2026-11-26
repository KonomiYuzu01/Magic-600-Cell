"""Tests for the C9 measurement tools in tools/perf/c9 (standard library only, synthetic inputs only).

No test starts a process, a window, a GPU device or a capture, and none runs the quiet-machine gate.
"""
from __future__ import annotations

import contextlib
import io
import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools/perf/c9'))
import cold_start  # noqa: E402
import evidence  # noqa: E402
import foreground  # noqa: E402
import key_to_frame  # noqa: E402
import lx2  # noqa: E402
import lx4  # noqa: E402
import presentmon_capture  # noqa: E402
import runbook  # noqa: E402
import ui_skew  # noqa: E402
from b412_summary import nearest_rank  # noqa: E402

FIXTURES = ROOT / 'tests/fixtures/c9_measure'
F = 10_000_000
BUILD = 'synthetic-build-0001'
COLD = {'new_process': True, 'no_product_process_at_launch': True, 'fresh_session_copy': True,
        'seconds_since_previous_exit': None}


def forbidden(*args, **kwargs):
    raise AssertionError('a test tried to start a process')


def e2_rows(latencies_ms, *, build=BUILD, synthetic=True, unconfirmed=(), no_frame=(), extra=(), header=None):
    """One press per second, each carried by its own frame submitted 1 ms after the press."""
    rows = [{'t': 'run', 'format': key_to_frame.LOG_FORMAT, 'run_id': 'generated', 'build_identity': build,
             'qpc_frequency': F, 'synthetic': synthetic, 'foreground_throughout': True, 'correctness': 'pass',
             **(header or {})}]
    frame = 0
    for index, latency in enumerate(latencies_ms):
        press = F * (index + 1)
        if index in no_frame:
            rows.append({'t': 'key', 'seq': index, 'qpc': press, 'frame_id': None})
            continue
        frame += 1
        rows.append({'t': 'key', 'seq': index, 'qpc': press, 'frame_id': frame})
        rows.append({'t': 'frame', 'frame_id': frame, 'qpc': press + F // 1000, 'device_epoch': 1, 'surface': 1})
        if index not in unconfirmed:
            rows.append({'t': 'presented', 'frame_id': frame, 'device_epoch': 1, 'surface': 1, 'sequence': frame,
                         'display_qpc': press + round(latency * F / 1000)})
    rows.extend(extra)
    return rows


def e3_rows(seconds, *, build=BUILD, synthetic=True, cold=None, drop=(), check='pass'):
    start = 100 * F
    rows = [{'t': 'run', 'format': cold_start.LOG_FORMAT, 'run_id': 'generated', 'build_identity': build,
             'qpc_frequency': F, 'synthetic': synthetic, 'cold': {**COLD, **(cold or {})}},
            {'t': 'process_start', 'qpc': start},
            {'t': 'session_opened', 'qpc': start + F // 2},
            {'t': 'runtime_loaded', 'qpc': start + F // 2 + 1},
            {'t': 'first_correct_frame', 'frame_id': 3, 'display_qpc': start + round(seconds * F), 'check': check}]
    return [row for row in rows if row['t'] not in drop]


def write_jsonl(path, rows):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(''.join(json.dumps(row) + '\n' for row in rows).encode())


class NoProcessCase(unittest.TestCase):
    def setUp(self):
        for name in ('run', 'Popen', 'call', 'check_call', 'check_output'):
            patcher = mock.patch.object(subprocess, name, side_effect=forbidden)
            patcher.start()
            self.addCleanup(patcher.stop)
        self.temp = Path(tempfile.mkdtemp(prefix='c9-measure-'))

    def tearDown(self):
        for path in sorted(self.temp.rglob('*'), key=lambda item: len(item.parts), reverse=True):
            path.rmdir() if path.is_dir() else path.unlink()
        self.temp.rmdir()


class KeyToFrameTests(NoProcessCase):
    def test_the_fixture_matches_keys_by_frame_and_cross_checks_presentmon(self):
        run = key_to_frame.read_run(FIXTURES / 'e2-run')
        self.assertEqual(run['status'], 'valid', run['reasons'])
        self.assertEqual([(entry['frame_id'], entry['status']) for entry in run['per_key']],
                         [(1, 'confirmed'), (3, 'confirmed'), (5, 'confirmed')])
        self.assertEqual([round(entry['latency_ms'], 6) for entry in run['per_key']], [45.0, 52.0, 60.0])
        self.assertEqual(run['presentmon'], {'frames_checked': 3, 'disagreements': 0, 'presentmon_presents': 5})

    def test_a_key_is_matched_by_its_frame_id_not_by_time(self):
        rows = e2_rows([20.0, 30.0])
        # A later frame presented first in the log changes nothing: matching is by frame id.
        rows.append({'t': 'frame', 'frame_id': 3, 'qpc': 3 * F, 'device_epoch': 1, 'surface': 1})
        rows.append({'t': 'presented', 'frame_id': 3, 'device_epoch': 1, 'surface': 1, 'sequence': 3,
                     'display_qpc': 3 * F + 5})
        run = key_to_frame.analyze(rows)
        self.assertEqual(run['status'], 'valid', run['reasons'])
        self.assertEqual([entry['frame_id'] for entry in run['per_key']], [1, 2])
        self.assertEqual([round(value, 6) for value in run['latencies_ms']], [20.0, 30.0])

    def test_an_unconfirmed_key_gets_no_time_and_counts_as_a_failed_key(self):
        # Owner, 20 November 2026, item 2 (docs/wiki/decisions/owner-decisions-2026-11-20-lp3-exit.md): the run stays
        # valid and the key counts above 100 ms.
        run = key_to_frame.analyze(e2_rows([20.0, 30.0, 40.0, 50.0], unconfirmed={1}, no_frame={3}))
        self.assertEqual(run['status'], 'valid', run['reasons'])
        self.assertNotIn('unconfirmed-keys', run['reasons'])
        self.assertEqual([entry['status'] for entry in run['per_key']],
                         ['confirmed', 'unconfirmed', 'confirmed', 'no-frame'])
        self.assertIsNone(run['per_key'][1]['latency_ms'])
        self.assertEqual((run['confirmed'], run['unconfirmed'], run['no_frame']), (2, 2, 1))
        self.assertEqual(run['unconfirmed_keys'], [1, 3])
        self.assertEqual(len(run['latencies_ms']), 2)  # never filled in from a later frame
        stats = run['stats']
        self.assertEqual((stats['n'], stats['confirmed'], stats['failed_keys']), (4, 2, 2))
        self.assertEqual(round(stats['p50'], 6), 40.0)
        self.assertEqual((stats['p95'], stats['p99'], stats['mean'], stats['max'], stats['p95_met']),
                         (None, None, None, None, False))
        self.assertNotIn('unconfirmed key invalidates its run', key_to_frame.DRAFTS)

    def test_a_failed_key_counts_above_the_bar_in_its_run_and_in_the_pool(self):
        # 100 confirmed keys and 5 failed: nearest rank 100 of 105 is a confirmed time; 6 failed: rank 101 of 106 is
        # above the bar, where a failed key stands.
        five = key_to_frame.analyze(e2_rows([10.0] * 105, unconfirmed={0, 1, 2}, no_frame={3, 4}))
        self.assertEqual(five['status'], 'valid', five['reasons'])
        self.assertEqual(round(five['stats']['p95'], 6), 10.0)
        self.assertEqual(key_to_frame.judge([five])[0], 'met')
        six = key_to_frame.analyze(e2_rows([10.0] * 106, unconfirmed={0, 1, 2}, no_frame={3, 4, 5}))
        self.assertEqual(six['status'], 'valid', six['reasons'])
        self.assertIsNone(six['stats']['p95'])
        verdict, reasons, pooled = key_to_frame.judge([six])
        self.assertEqual((verdict, reasons), ('not-met', ['p95-over-bar']))
        self.assertEqual((pooled['n'], pooled['confirmed'], pooled['failed_keys'], pooled['p95']), (106, 100, 6, None))
        # Every run's p95 counts its own failed keys: 2 failed in 20 fail that run, though the pool meets the bar.
        good = key_to_frame.analyze(e2_rows([10.0] * 100))
        short = key_to_frame.analyze(e2_rows([10.0] * 20, no_frame={0, 1}))
        verdict, reasons, pooled = key_to_frame.judge([good, short])
        self.assertTrue(pooled['p95_met'])
        self.assertEqual((verdict, reasons), ('not-met', ['p95-over-bar']))
        # Fewer than 100 confirmed keys stay insufficient, failed keys or not.
        self.assertEqual(key_to_frame.judge([key_to_frame.analyze(e2_rows([10.0] * 100, no_frame={0}))])[:2],
                         ('insufficient', ['too-few-observations']))

    def test_unconfirmed_keys_are_reported_loudly(self):
        write_jsonl(self.temp / 'run' / 'events.jsonl', e2_rows([20.0, 30.0], unconfirmed={0}))
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = key_to_frame.main([str(self.temp / 'run')])
        self.assertIn('UNCONFIRMED', err.getvalue())
        self.assertIn('seq 0', err.getvalue())
        self.assertIn('count as failed keys', err.getvalue())
        self.assertIn('p95 above 100 ms (failed keys)', out.getvalue())
        self.assertEqual(code, 5)  # a synthetic record

    def test_percentiles_are_nearest_rank_and_the_bar_is_p95_over_100_keys(self):
        values = [float(n) for n in range(1, 101)]
        run = key_to_frame.analyze(e2_rows(values))
        self.assertEqual(run['status'], 'valid', run['reasons'])
        stats = run['stats']
        self.assertEqual([round(stats[key], 6) for key in ('p50', 'p95', 'p99', 'max')], [50.0, 95.0, 99.0, 100.0])
        self.assertEqual(stats['p95'], nearest_rank(run['latencies_ms'], 95))
        self.assertEqual(key_to_frame.judge([run])[0], 'met')
        slow = key_to_frame.analyze(e2_rows(values[:94] + [150.0] * 6))
        self.assertEqual(key_to_frame.judge([slow])[:2], ('not-met', ['p95-over-bar']))
        few = key_to_frame.analyze(e2_rows([10.0] * 99))
        self.assertEqual(key_to_frame.judge([few])[:2], ('insufficient', ['too-few-observations']))

    def test_every_valid_run_must_meet_the_bar_and_invalid_runs_are_never_pooled(self):
        good = key_to_frame.analyze(e2_rows([10.0] * 100))
        bad = key_to_frame.analyze(e2_rows([10.0] * 18 + [200.0] * 2))
        self.assertLessEqual(nearest_rank(good['latencies_ms'] + bad['latencies_ms'], 95), 100)
        self.assertEqual(key_to_frame.judge([good, bad])[0], 'not-met')
        invalid = key_to_frame.analyze(e2_rows([500.0] * 100, header={'foreground_throughout': False}))
        self.assertEqual(invalid['status'], 'invalid')
        self.assertEqual(key_to_frame.judge([good, invalid])[0], 'met')
        self.assertEqual(key_to_frame.judge([invalid])[:2], ('invalid', ['no-valid-run']))
        other = key_to_frame.analyze(e2_rows([10.0] * 100, build='synthetic-build-0002'))
        self.assertEqual(key_to_frame.judge([good, other])[:2], ('invalid', ['mixed-builds']))

    def test_a_presented_row_for_a_frame_the_log_never_named_is_a_contradiction(self):
        extra = [{'t': 'presented', 'frame_id': 99, 'device_epoch': 1, 'surface': 1, 'sequence': 99,
                  'display_qpc': 99 * F}]
        run = key_to_frame.analyze(e2_rows([20.0, 30.0], extra=extra))
        self.assertEqual(run['status'], 'invalid')
        self.assertIn('contradiction', run['reasons'])
        self.assertEqual(run['contradictions'], ['presented-unnamed-frame'])

    def test_other_contradictions(self):
        cases = {
            'presented-twice': [{'t': 'presented', 'frame_id': 1, 'device_epoch': 1, 'surface': 1, 'sequence': 1,
                                 'display_qpc': F + 2 * F // 100}],
            'presented-mismatch': [{'t': 'frame', 'frame_id': 9, 'qpc': 9 * F, 'device_epoch': 1, 'surface': 1},
                                   {'t': 'presented', 'frame_id': 9, 'device_epoch': 2, 'surface': 1,
                                    'sequence': 1, 'display_qpc': 9 * F + 1}],
            'displayed-before-submit': [{'t': 'frame', 'frame_id': 9, 'qpc': 9 * F, 'device_epoch': 1, 'surface': 1},
                                        {'t': 'presented', 'frame_id': 9, 'device_epoch': 1, 'surface': 1,
                                         'sequence': 9, 'display_qpc': 9 * F - 1}],
            'key-names-unknown-frame': [{'t': 'key', 'seq': 2, 'qpc': 5 * F, 'frame_id': 77}],
            'frame-before-key': [{'t': 'frame', 'frame_id': 9, 'qpc': 4 * F, 'device_epoch': 1, 'surface': 1},
                                 {'t': 'key', 'seq': 2, 'qpc': 5 * F, 'frame_id': 9}],
            'key-order': [{'t': 'key', 'seq': 5, 'qpc': 5 * F, 'frame_id': None}],
        }
        for contradiction, extra in cases.items():
            with self.subTest(contradiction=contradiction):
                run = key_to_frame.analyze(e2_rows([20.0, 30.0], extra=extra))
                self.assertEqual(run['status'], 'invalid')
                self.assertIn(contradiction, run['contradictions'])

    def test_presentmon_disagreement_and_run_conditions_invalidate_a_run(self):
        # The press is at F, its frame submitted at F + 1 ms and confirmed displayed at F + 20 ms.
        ms = F // 1000
        for name, ticks in (('none', []), ('before-submit', [F]), ('after-display', [F + 21 * ms]),
                            ('only-before-and-after', [F, F + 25 * ms])):
            with self.subTest(presents=name):
                run = key_to_frame.analyze(e2_rows([20.0]), present_ticks=ticks)
                self.assertEqual(run['presentmon']['disagreements'], 1)
                self.assertIn('presentmon-disagrees', run['reasons'])
        for ticks in ([F + ms], [F + 20 * ms], [F, F + 2 * ms, F + 25 * ms]):
            agree = key_to_frame.analyze(e2_rows([20.0]), present_ticks=ticks)
            self.assertEqual(agree['status'], 'valid', agree['reasons'])
        for header, reason in (({'foreground_throughout': False}, 'not-foreground'),
                               ({'correctness': 'fail'}, 'correctness-failed')):
            with self.subTest(reason=reason):
                self.assertIn(reason, key_to_frame.analyze(e2_rows([20.0], header=header))['reasons'])
        self.assertIn('no-keys', key_to_frame.analyze(e2_rows([]))['reasons'])

    def test_an_unreadable_run_is_kept_as_invalid(self):
        write_jsonl(self.temp / 'run' / 'events.jsonl', [{'t': 'key', 'seq': 0, 'qpc': 1, 'frame_id': None}])
        run = key_to_frame.read_run(self.temp / 'run')
        self.assertEqual((run['status'], run['reasons']), ('unreadable', ['bad-header']))
        self.assertEqual(key_to_frame.read_run(self.temp / 'missing')['reasons'], ['missing-file'])


class ColdStartTests(NoProcessCase):
    def test_the_fixture_run_times_process_start_to_the_first_correct_frame(self):
        run = cold_start.read_run(FIXTURES / 'e3-run')
        self.assertEqual(run['status'], 'valid', run['reasons'])
        self.assertAlmostEqual(run['seconds'], 2.15)

    def test_three_cold_runs_within_3_s_meet_the_bar_and_one_over_fails_it(self):
        runs = [cold_start.analyze(e3_rows(seconds)) for seconds in (1.2, 2.5, 3.0)]
        self.assertEqual(cold_start.judge(runs), ('met', []))
        runs[1] = cold_start.analyze(e3_rows(3.2))
        self.assertEqual(cold_start.judge(runs), ('not-met', ['over-bar']))
        self.assertEqual(cold_start.judge(runs[:2]), ('insufficient', ['too-few-runs']))

    def test_a_missing_first_correct_frame_fails_the_run_and_the_bar(self):
        missing = cold_start.analyze(e3_rows(1.0, drop=('first_correct_frame',)))
        self.assertEqual((missing['status'], missing['reasons'], missing['seconds']),
                         ('failed', ['no-first-correct-frame'], None))
        good = [cold_start.analyze(e3_rows(1.0)) for _ in range(3)]
        self.assertEqual(cold_start.judge(good + [missing]), ('not-met', ['run-failed']))
        wrong = cold_start.analyze(e3_rows(1.0, check='fail'))
        self.assertEqual(wrong['status'], 'failed')
        unloaded = cold_start.analyze(e3_rows(1.0, drop=('runtime_loaded',)))
        self.assertEqual(unloaded['reasons'], ['runtime-loaded-missing'])

    def test_a_run_that_is_not_cold_is_invalid_and_not_counted(self):
        warm = cold_start.analyze(e3_rows(1.0, cold={'seconds_since_previous_exit': 5}))
        self.assertEqual((warm['status'], warm['reasons']), ('invalid', ['not-cold']))
        reused = cold_start.analyze(e3_rows(1.0, cold={'fresh_session_copy': False}))
        self.assertEqual(reused['status'], 'invalid')
        good = [cold_start.analyze(e3_rows(1.0)) for _ in range(2)]
        self.assertEqual(cold_start.judge(good + [warm]), ('insufficient', ['too-few-runs']))

    def test_unreadable_logs_and_mixed_builds(self):
        write_jsonl(self.temp / 'run' / 'startup.jsonl', e3_rows(1.0, drop=('process_start',)))
        run = cold_start.read_run(self.temp / 'run')
        self.assertEqual((run['status'], run['reasons']), ('unreadable', ['missing-process-start']))
        runs = [cold_start.analyze(e3_rows(1.0)), cold_start.analyze(e3_rows(1.0)),
                cold_start.analyze(e3_rows(1.0, build='synthetic-build-0002'))]
        self.assertEqual(cold_start.judge(runs), ('invalid', ['mixed-builds']))
        late = e3_rows(1.0)
        late[2]['qpc'] = late[4]['display_qpc'] + 1
        self.assertIn('loaded-after-first-frame', cold_start.analyze(late)['reasons'])


class RecordTests(NoProcessCase):
    def test_a_synthetic_record_never_passes_a_bar(self):
        directories = []
        for index in range(3):
            write_jsonl(self.temp / f'e3-{index}' / 'startup.jsonl', e3_rows(1.0))
            directories.append(self.temp / f'e3-{index}')
        self.assertEqual(cold_start.judge([cold_start.read_run(path) for path in directories])[0], 'met')
        record = cold_start.build(directories, [])
        self.assertEqual((record['verdict'], record['passes_bar'], record['synthetic']), ('synthetic', False, True))
        self.assertIn('synthetic-input', record['reasons'])
        for index, path in enumerate(directories):
            write_jsonl(self.temp / f'e2-{index}' / 'events.jsonl', e2_rows([10.0] * 100, synthetic=False))
        e2_dirs = [self.temp / f'e2-{index}' for index in range(3)]
        self.assertEqual(key_to_frame.judge([key_to_frame.read_run(path) for path in e2_dirs])[0], 'met')
        flagged = key_to_frame.build(e2_dirs, ['--synthetic'], synthetic_flag=True)
        self.assertEqual((flagged['verdict'], flagged['passes_bar']), ('synthetic', False))
        fixture = key_to_frame.build([FIXTURES / 'e2-run'], [])
        self.assertEqual((fixture['verdict'], fixture['passes_bar']), ('synthetic', False))
        self.assertTrue(evidence.is_synthetic([FIXTURES / 'e3-run' / 'startup.jsonl'], [False]))
        forced = evidence.make_record('m', 'performance', synthetic=True, build_identity=BUILD, inputs=[], tools={},
                                      command=[], bar={}, result={}, verdict='met')
        self.assertFalse(forced['passes_bar'])

    def test_records_name_their_evidence_kind_inputs_and_tools_without_paths(self):
        argv = [str(FIXTURES / 'e2-run'), '--out', str(self.temp / 'record.json')]
        record = key_to_frame.build([FIXTURES / 'e2-run'], argv)
        self.assertEqual(record['format'], evidence.FORMAT)
        self.assertEqual(record['evidence_kind'], 'performance')
        self.assertEqual(record['build_identity'], BUILD)
        self.assertEqual(sorted(entry['name'] for entry in record['inputs']), ['events.jsonl', 'presentmon.csv'])
        self.assertTrue(all(re.fullmatch(r'[0-9a-f]{64}', entry['sha256']) for entry in record['inputs']))
        self.assertEqual(record['tools']['tool']['name'], 'key_to_frame.py')
        self.assertIn('renderer_gate.py', record['tools']['helpers'])
        self.assertEqual(record['command'][:4], ['python', '-B', 'tools/perf/c9/key_to_frame.py',
                                                 'tests/fixtures/c9_measure/e2-run'])
        self.assertTrue(record['command'][5].startswith('outside-repository:'))
        self.assertEqual(record['bar']['limit_ms'], 100.0)
        self.assertEqual(record['redactions'], 0)
        text = json.dumps(record, ensure_ascii=False)
        self.assertIsNone(re.search(r'[A-Za-z]:(\\\\|/)', text))
        self.assertNotIn(ROOT.as_posix(), text)
        self.assertNotIn(self.temp.name, text)
        with self.assertRaises(ValueError):
            evidence.make_record('m', 'source', synthetic=False, build_identity=BUILD, inputs=[], tools={},
                                 command=[], bar={}, result={}, verdict='met')
        unbuilt = evidence.make_record('m', 'performance', synthetic=False, build_identity=None, inputs=[], tools={},
                                       command=[], bar={}, result={}, verdict='met')
        self.assertEqual((unbuilt['verdict'], unbuilt['passes_bar']), ('invalid', False))

    def test_a_record_is_written_once_with_lf_endings(self):
        out = self.temp / 'record.json'
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(cold_start.main([str(FIXTURES / 'e3-run'), '--out', str(out)]), 5)
            self.assertEqual(cold_start.main([str(FIXTURES / 'e3-run'), '--out', str(out)]), 1)
        data = out.read_bytes()
        self.assertNotIn(b'\r', data)
        self.assertEqual(json.loads(data)['verdict'], 'synthetic')


class PresentMonTests(NoProcessCase):
    def pinned(self):
        path = self.temp.joinpath('pf', *presentmon_capture.PINNED_RELATIVE)
        path.parent.mkdir(parents=True)
        path.write_bytes(b'')
        return {'ProgramFiles': str(self.temp / 'pf'), 'LOCALAPPDATA': str(self.temp / 'local')}

    def dry_run(self, environ, version, *extra):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = presentmon_capture.main(
                ['--process-name', 'Godot.exe', '--output', str(self.temp / 'capture.csv'), '--seconds', '120',
                 *extra], environ=environ, which=lambda name: None, version_reader=lambda path: (version, 'test'),
                run=forbidden, is_admin=lambda: False)
        return code, out.getvalue(), err.getvalue()

    def test_the_command_line_filters_one_process_and_times_the_capture(self):
        command = presentmon_capture.build_command(Path('PresentMon-2.6.0-x64.exe'), Path('out.csv'), 120,
                                                   process_name='Godot.exe')
        self.assertEqual(command[1:], ['--v1_metrics', '--qpc_time', '--process_name', 'Godot.exe', '--output_file',
                                       'out.csv', '--session_name', 'magic600-c9-capture', '--timed', '120',
                                       '--terminate_after_timed'])
        self.assertEqual(presentmon_capture.check_command(command), [])
        by_id = presentmon_capture.build_command(Path('PresentMon.exe'), Path('out.csv'), 5, process_id=4242)
        self.assertEqual(by_id[3:5], ['--process_id', '4242'])
        self.assertEqual(presentmon_capture.check_command(by_id), [])
        for kwargs in ({}, {'process_name': 'a.exe', 'process_id': 1}, {'process_name': 'a.exe', 'seconds': 0},
                       {'process_name': 'a.exe', 'output': Path('out.txt')}, {'process_id': -3}):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                arguments = {'output': Path('out.csv'), 'seconds': 10, **kwargs}
                presentmon_capture.build_command(Path('PresentMon.exe'), arguments.pop('output'),
                                                 arguments.pop('seconds'), **arguments)

    def test_the_check_rejects_incomplete_or_unsafe_command_lines(self):
        good = presentmon_capture.build_command(Path('PresentMon.exe'), Path('out.csv'), 10, process_name='a.exe')
        cases = {
            'exactly one process filter is needed': [a for a in good if a not in ('--process_name', 'a.exe')],
            f'a timed capture of 1 to {presentmon_capture.SECONDS_MAX} s is needed':
                [a for a in good if a not in ('--timed', '10')],
            '--terminate_after_timed is missing': good[:-1],
            'the output file must be a .csv file': [('out.txt' if a == 'out.csv' else a) for a in good],
            'unexpected argument --start_etl_logging': good + ['--start_etl_logging'],
            'the executable is not PresentMon': ['notepad.exe'] + good[1:],
        }
        for problem, command in cases.items():
            with self.subTest(problem=problem):
                self.assertIn(problem, presentmon_capture.check_command(command))
        both = good + ['--process_id', '7']
        self.assertIn('exactly one process filter is needed', presentmon_capture.check_command(both))

    def test_a_capture_covers_u8s_shell_run(self):
        """U8's shell runs up to 10,500 s per puzzle; the 3600 s cap cut its capture short (C9, 20 Nov 2026)."""
        command = presentmon_capture.build_command(Path('PresentMon.exe'), Path('out.csv'), 10_800,
                                                   process_name='a.exe')
        self.assertEqual(presentmon_capture.check_command(command), [])
        with self.assertRaises(ValueError):
            presentmon_capture.build_command(Path('PresentMon.exe'), Path('out.csv'),
                                             presentmon_capture.SECONDS_MAX + 1, process_name='a.exe')

    def test_a_dry_run_without_presentmon_exits_nonzero(self):
        environ = {'ProgramFiles': str(self.temp / 'none'), 'LOCALAPPDATA': str(self.temp / 'none')}
        code, out, err = self.dry_run(environ, '2.6.0.0', '--dry-run')
        self.assertEqual(code, 1)
        self.assertIn('PresentMon: missing', out)
        self.assertIn('missing', err)
        self.assertIn('--terminate_after_timed', out)

    def test_a_dry_run_checks_the_pinned_version_and_captures_nothing(self):
        environ = self.pinned()
        code, out, _ = self.dry_run(environ, '2.6.0.0', '--dry-run')
        self.assertEqual(code, 0)
        self.assertIn('pinned location; version 2.6.0.0', out)
        self.assertIn('command check: ok', out)
        self.assertEqual(self.dry_run(environ, '2.5.0.0', '--dry-run')[0], 1)
        self.assertEqual(self.dry_run(environ, None, '--dry-run')[0], 1)
        self.assertFalse((self.temp / 'capture.csv').exists())
        # Without --dry-run and without administrator rights nothing runs either.
        self.assertEqual(self.dry_run(environ, '2.6.0.0')[0], 1)

    def test_versions_and_capture_completion(self):
        self.assertTrue(presentmon_capture.same_version('2.6.0', '2.6.0.0'))
        self.assertFalse(presentmon_capture.same_version('2.6.1.0', '2.6.0.0'))
        self.assertFalse(presentmon_capture.same_version(None))
        output = self.temp / 'capture.csv'
        command = presentmon_capture.build_command(Path('PresentMon.exe'), output, 10, process_name='a.exe')
        calls = []

        def finished(arguments, **kwargs):
            calls.append(arguments)
            output.write_bytes(b'Application,ProcessID\na.exe,1\n')
            return subprocess.CompletedProcess(arguments, 0)

        self.assertEqual(presentmon_capture.capture(command, 10, output, finished), 0)

        def hung(arguments, **kwargs):
            calls.append(arguments)
            if '--terminate_existing_session' not in arguments:
                raise subprocess.TimeoutExpired(arguments, kwargs['timeout'])
            return subprocess.CompletedProcess(arguments, 0)

        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(presentmon_capture.capture(command, 10, output, hung), 1)
        self.assertEqual(calls[-1], ['PresentMon.exe', '--session_name', 'magic600-c9-capture',
                                     '--terminate_existing_session'])


class RunbookTests(NoProcessCase):
    def test_the_attended_steps_come_first_in_the_owner_order(self):
        ids = [step.id for step in runbook.STEPS]
        self.assertEqual(ids, ['shell-launch', 'b1-input', 'e2-key-to-frame', 'core-gpu', 'b3-probe', 'b2-skew',
                               'wj-w3', 'rpi79-gate', 'e3-cold-start', 'lx2', 'lx4'])
        self.assertEqual([runbook.LABELS[step_id] for step_id in ids],
                         ['A1', 'A2', 'A3', 'U1', 'U2', 'U3', 'U4', 'U5', 'U6', 'U7', 'U8'])
        for step_id in ('b2-skew', 'lx4'):
            step = next(step for step in runbook.STEPS if step.id == step_id)
            self.assertTrue(step.built and any(kind == 'mentions' for kind, _ in step.prerequisites))
        attended = [step.attended for step in runbook.STEPS]
        self.assertEqual(attended, sorted(attended, reverse=True))
        self.assertEqual(attended.count(True), 3)
        step = next(step for step in runbook.STEPS if step.id == 'lx2')
        self.assertTrue(any('--injection' in command for command in step.commands))
        self.assertTrue(all('--features device --bin c600-lx2-' in command for command in step.commands
                            if '-p c600-lx2 ' in command))
        # U7 (b): the shell's LX-2 module and lx2.py measure conditions 3 and 4, so the step is built.
        self.assertIsNone(step.missing)
        self.assertEqual(step.minutes, 79)
        self.assertTrue(any('--lx2-injection' in command and 'lx2.py post' in command for command in step.commands))
        self.assertTrue(any('lx2.py analyze' in command and '--faults' in command for command in step.commands))
        for token in ('pub fn main_with', 'main_with(native::point)', 'Magic600.LX2.Phase', 'WriteString("t", "lx2")',
                      '"--lx2-injection"'):
            argument = next(argument for kind, argument in step.prerequisites
                            if kind == 'mentions' and argument[1] == token)
            self.assertTrue(runbook.evaluate('mentions', argument)[0], argument)
        self.assertTrue(runbook.evaluate('imports', 'lx2')[0])
        gpu = next(step for step in runbook.STEPS if step.id == 'core-gpu')
        self.assertTrue(gpu.built and gpu.commands[0].startswith('python -B renderer/core/check_core.py --gpu'))
        self.assertEqual(next(step for step in runbook.STEPS if step.id == 'lx4').minutes, 240)

    def test_u4_prepares_route_b_and_u6_rebuilds_and_checks_before_the_cold_runs(self):
        wj = next(step for step in runbook.STEPS if step.id == 'wj-w3')
        self.assertTrue(wj.built)
        self.assertTrue(wj.commands[0].startswith('python -B renderer/acceptance/prepare_host.py --route surface'))
        self.assertTrue(wj.commands[1].startswith('python -B renderer/acceptance/prepare_data.py'))
        self.assertIn(f'{runbook.DECLARE} -DryRun', wj.commands[2])
        self.assertTrue(all('-Build <route-B build>' in command and runbook.DECLARE in command
                            for command in wj.commands[2:-1]))
        self.assertIn(('dir', runbook.WJ_DATA), wj.prerequisites)
        self.assertIn(('compiles', runbook.PREPARE_DATA), wj.prerequisites)
        self.assertIn(('build', 'surface'), wj.prerequisites)
        e3 = next(step for step in runbook.STEPS if step.id == 'e3-cold-start')
        self.assertTrue(e3.commands[0].startswith('python -B renderer/shell/build_product.py'))
        self.assertIn('--cold-start --dry-run', e3.commands[1])
        # One untimed launch of the same build before the three timed runs; its time is recorded apart.
        self.assertTrue(e3.commands[2].startswith('python -B renderer/shell/launch.py --out <warm-up folder> '
                                                  '--cold-start   # once, untimed'))
        self.assertIn('three times', e3.commands[3])
        self.assertTrue(e3.commands[4].startswith('python -B tools/perf/c9/cold_start.py <run 1> <run 2> <run 3>'))
        self.assertNotIn('warm-up', e3.commands[4])
        self.assertTrue(e3.commands[5].startswith('python -B tools/perf/c9/cold_start.py <warm-up folder>'))
        self.assertIn(('compiles', runbook.BUILD_PRODUCT), e3.prerequisites)
        self.assertIn(('mentions', (runbook.SHELL, '"m6h_preload"')), e3.prerequisites)
        self.assertIn(('product-build', None), e3.prerequisites)
        self.assertIn('Earlier launches of the same build are allowed', ' '.join(cold_start.__doc__.split()))
        for kind, argument in e3.prerequisites:
            if kind != 'product-build':
                with self.subTest(prerequisite=(kind, argument)):
                    self.assertTrue(runbook.evaluate(kind, argument)[0])

    def test_the_build_check_names_the_route_and_the_default_build_is_route_b(self):
        root = self.temp / 'root'
        builds = root / runbook.BUILDS
        identity = {'format': runbook.IDENTITY_FORMAT, 'source': {'head': 'b' * 40}, 'abi_version': 5}
        for name, route in (('20261119T100000Z', 'surface'), ('20261119T110000Z', None)):
            (builds / name).mkdir(parents=True)
            value = dict(identity, **({} if route is None else {'route': route}))
            (builds / name / runbook.IDENTITY).write_bytes(json.dumps(value).encode())
        self.assertEqual(runbook.build_route(builds / '20261119T110000Z'), 'texture')
        self.assertIsNone(runbook.build_route(builds / 'absent'))
        self.assertEqual(runbook.newest_build(root).name, '20261119T110000Z')
        self.assertEqual(runbook.newest_build(root, 'surface').name, '20261119T100000Z')
        self.assertIsNone(runbook.newest_build(self.temp / 'empty', 'surface'))
        ok, detail = runbook.evaluate('build', 'surface', build=builds / '20261119T110000Z')
        self.assertFalse(ok)
        self.assertIn('route texture, not surface: python -B renderer/acceptance/prepare_host.py --route surface',
                      detail)
        ok, detail = runbook.evaluate('build', 'surface', build=builds / '20261119T100000Z')
        self.assertTrue(ok)
        self.assertIn('route surface', detail)
        self.assertTrue(runbook.evaluate('build', None, build=builds / '20261119T110000Z')[0])
        # One build identity: with the checkout's HEAD given, a build made from another commit is refused.
        self.assertTrue(runbook.evaluate('build', 'surface', build=builds / '20261119T100000Z', head='b' * 40)[0])
        ok, detail = runbook.evaluate('build', 'surface', build=builds / '20261119T100000Z', head='d' * 40)
        self.assertFalse(ok)
        self.assertIn(f"not HEAD {'d' * 12}: python -B renderer/acceptance/prepare_host.py --route surface", detail)

    def test_the_product_build_must_come_from_head_and_a_clean_tree(self):
        root = self.temp / 'root'
        products = root / runbook.PRODUCT_BUILDS
        record = {'head': 'e' * 40, 'dirty': False, 'identity': 'c9-product-0123456789abcdef'}
        for name, value in (('20261121T010000Z', dict(record, head='f' * 40)), ('20261121T020000Z', record)):
            (products / name).mkdir(parents=True)
            (products / name / runbook.PRODUCT_RECORD).write_bytes(json.dumps(value).encode())
        with mock.patch.object(runbook, 'ROOT', root):
            ok, detail = runbook.evaluate('product-build', None, head='e' * 40)
            self.assertTrue(ok)
            self.assertIn('product build 20261121T020000Z, c9-product-0123456789abcdef', detail)
            ok, detail = runbook.evaluate('product-build', None, head='f' * 40)
            self.assertFalse(ok)
            self.assertIn(f"not HEAD {'f' * 12}: python -B renderer/shell/build_product.py", detail)
            self.assertTrue(runbook.evaluate('product-build', None)[0])
            (products / '20261121T020000Z' / runbook.PRODUCT_RECORD).write_bytes(
                json.dumps(dict(record, dirty=True)).encode())
            ok, detail = runbook.evaluate('product-build', None, head='e' * 40)
            self.assertFalse(ok)
            self.assertIn('built from a changed tree', detail)
        with mock.patch.object(runbook, 'ROOT', self.temp / 'empty'):
            ok, detail = runbook.evaluate('product-build', None)
        self.assertFalse(ok)
        self.assertIn('no product build under work/build/c9-product', detail)
        # Every step that launches the product build checks it.
        for step in runbook.STEPS:
            launches = any(runbook.LAUNCH in command for command in step.commands)
            self.assertEqual(('product-build', None) in step.prerequisites, launches, step.id)

    def test_the_checkout_head_is_read_once_with_git(self):
        calls = []

        def run(arguments, **kwargs):
            calls.append(arguments)
            return subprocess.CompletedProcess(arguments, 0, stdout='a' * 40 + '\n')

        self.assertEqual(runbook.checkout_head(run), 'a' * 40)
        self.assertEqual(calls, [['git', '-C', str(runbook.ROOT), 'rev-parse', 'HEAD']])
        self.assertIsNone(runbook.checkout_head(lambda arguments, **kwargs: subprocess.CompletedProcess(
            arguments, 128, stdout='')))

        def missing(arguments, **kwargs):
            raise OSError('no git')

        self.assertIsNone(runbook.checkout_head(missing))

    def test_the_steps_follow_the_merged_e2_route_b_rerun_and_preload_changes(self):
        steps = {step.id: step for step in runbook.STEPS}
        # PR 380 (owner item 2): an unconfirmed key is one failed key and its run stays valid; 100 confirmed keys.
        a3 = ' '.join(steps['e2-key-to-frame'].commands + steps['e2-key-to-frame'].run_time)
        self.assertIn('about 120 twists', a3)
        self.assertIn('counts as one failed key above 100 ms, and its run stays valid', a3)
        # PR 385 (item 9): route B's display evidence is the core's presented.jsonl, checked by the finalizer.
        for step_id in ('wj-w3', 'rpi79-gate'):
            step = steps[step_id]
            self.assertIn(runbook.ROUTE_B_DISPLAY, step.run_time)
            self.assertIn(('mentions', (runbook.FINALIZE, "'presented.jsonl'")), step.prerequisites)
        self.assertIn(('mentions', ('renderer/core/src', 'presented.jsonl')), steps['wj-w3'].prerequisites)
        # PR 386: the rerun endpoint; PR 387: the preload.
        self.assertLessEqual({('mentions', (runbook.SHELL, '"lx4_rerun_ended"')),
                              ('mentions', (runbook.SHELL, '"m6h_run_view"'))}, set(steps['lx4'].prerequisites))
        self.assertIn('the host does not draw the run\'s value yet', ' '.join(steps['lx4'].run_time))
        for step_id in ('wj-w3', 'rpi79-gate', 'e3-cold-start', 'lx4'):
            for kind, argument in steps[step_id].prerequisites:
                if kind in ('mentions', 'compiles'):
                    with self.subTest(step=step_id, prerequisite=argument):
                        self.assertTrue(runbook.evaluate(kind, argument)[0])

    def test_u5_runs_w3_three_times_on_its_own_stage_build(self):
        step = next(step for step in runbook.STEPS if step.id == 'rpi79-gate')
        self.assertTrue(step.built)
        self.assertTrue(step.commands[0].startswith(
            'python -B renderer/acceptance/prepare_host.py --route surface --stages renderer/acceptance/stages'))
        self.assertIn(f'-Scene w3 -Build <stage build> {runbook.DECLARE} -DryRun', step.commands[1])
        self.assertIn('three times', step.commands[2])
        # A stage build runs W3 only: no W-J scene and no other build in U5's commands.
        for command in step.commands[1:3]:
            self.assertIn('-Scene w3 -Build <stage build>', command)
        self.assertTrue(step.commands[3].startswith('python -B tools/perf/renderer_gate.py <run 1> <run 2> <run 3>'))
        self.assertIn(('stage-build', 'surface'), step.prerequisites)
        self.assertIn(('dir', runbook.STAGES), step.prerequisites)
        for kind, argument in step.prerequisites:
            if kind in ('file', 'dir', 'compiles', 'mentions'):
                with self.subTest(prerequisite=(kind, argument)):
                    self.assertTrue(runbook.evaluate(kind, argument)[0])

    def test_a_stage_build_is_u5s_and_never_u4s_default(self):
        root = self.temp / 'root'
        builds = root / runbook.BUILDS
        identity = {'format': runbook.IDENTITY_FORMAT, 'source': {'head': 'c' * 40}, 'abi_version': 5,
                    'route': 'surface'}
        stages = {'overdraw': 108, 'dxc': {'version': '1.9.2609'}}
        for name, value in (('20261120T100000Z', identity), ('20261120T110000Z', dict(identity, stages=stages)),
                            ('20261120T120000Z', dict(identity, route='texture'))):
            (builds / name).mkdir(parents=True)
            (builds / name / runbook.IDENTITY).write_bytes(json.dumps(value).encode())
        self.assertEqual(runbook.build_stages(builds / '20261120T110000Z'), stages)
        self.assertIsNone(runbook.build_stages(builds / '20261120T100000Z'))
        self.assertEqual(runbook.newest_build(root, 'surface').name, '20261120T110000Z')
        self.assertEqual(runbook.newest_build(root, 'surface', stages=False).name, '20261120T100000Z')
        self.assertEqual(runbook.newest_build(root, 'surface', stages=True).name, '20261120T110000Z')
        # U4's build check refuses a stage build (W3 only) and accepts the plain route-B build.
        ok, detail = runbook.evaluate('build', 'surface', build=builds / '20261120T110000Z')
        self.assertFalse(ok)
        self.assertIn('with user stages (W3 only): python -B renderer/acceptance/prepare_host.py --route surface',
                      detail)
        self.assertTrue(runbook.evaluate('build', 'surface', build=builds / '20261120T100000Z')[0])
        # U5's check finds the stage build itself, whatever --build names.
        with mock.patch.object(runbook, 'ROOT', root):
            ok, detail = runbook.evaluate('stage-build', 'surface', build=builds / '20261120T100000Z')
            self.assertTrue(runbook.evaluate('stage-build', 'surface', head='c' * 40)[0])
            refused, why = runbook.evaluate('stage-build', 'surface', head='d' * 40)
        self.assertTrue(ok)
        self.assertIn('build 20261120T110000Z', detail)
        self.assertIn('overdraw 108, DXC 1.9.2609', detail)
        self.assertFalse(refused)
        self.assertIn(f"DXC 1.9.2609, not HEAD {'d' * 12}: python -B renderer/acceptance/prepare_host.py "
                      '--route surface --stages renderer/acceptance/stages', why)
        with mock.patch.object(runbook, 'ROOT', self.temp / 'empty'):
            ok, detail = runbook.evaluate('stage-build', 'surface')
        self.assertFalse(ok)
        self.assertIn('--route surface --stages renderer/acceptance/stages', detail)

    def test_shell_runs_start_the_capture_and_the_poster_before_the_shell(self):
        """The shell starts last, so it gets the focus (C9, 20 Nov 2026: a capture started after it took it away)."""
        steps = {step.id: step for step in runbook.STEPS}
        for step_id in ('b2-skew', 'lx2', 'lx4'):
            with self.subTest(step=step_id):
                commands = ' '.join(steps[step_id].commands)
                self.assertNotIn('alongside', commands)
                shell = [command for command in steps[step_id].commands if runbook.LAUNCH in command]
                self.assertTrue(shell)
                for command in shell:
                    self.assertLess(command.index('presentmon_capture.py --process-name'),
                                    command.index(runbook.LAUNCH))
                    self.assertIn('--build <product build>', command)
                    self.assertIn(f'unattended.ps1 -Steps {runbook.LABELS[step_id]}', command)
        analyze = steps['lx4'].commands[-1]
        self.assertIn('--gate-reference <U4 W3 gate summary>.json', analyze)
        self.assertNotIn('--gate <', analyze)
        self.assertIn('tools/perf/c9/unattended.ps1', runbook.__doc__)

    def test_every_step_has_a_time_and_sources(self):
        for step in runbook.STEPS:
            with self.subTest(step=step.id):
                self.assertTrue(step.minutes > 0 and step.sources)
                self.assertTrue(step.built or step.missing)

    def test_u8_needs_the_launcher_to_hand_the_module_its_library_and_fixtures(self):
        step = next(step for step in runbook.STEPS if step.id == 'lx4')
        mentions = {argument for kind, argument in step.prerequisites if kind == 'mentions'}
        self.assertLessEqual({(runbook.LAUNCH, '"--lx4-fixtures"'), (runbook.LAUNCH, 'M600_LX4_DLL'),
                              (runbook.LAUNCH, 'M600_LX4_FIXTURES'), (runbook.BUILD_PRODUCT, 'c600-shell-lx4')},
                             mentions)
        self.assertTrue(any('--lx4 --lx4-fixtures <fixtures>' in command for command in step.commands))
        # A mention is searched in one file as well as in a folder.
        self.assertTrue(runbook.evaluate('mentions', (runbook.LAUNCH, '"--lx4"'))[0])
        self.assertFalse(runbook.evaluate('mentions', (runbook.LAUNCH, 'a token no launcher writes'))[0])

    def test_u8_runs_both_puzzles_once_the_build_holds_the_jumble_on_fixture_and_c600_render_poses(self):
        step = next(step for step in runbook.STEPS if step.id == 'lx4')
        mentions = {argument for kind, argument in step.prerequisites if kind == 'mentions'}
        self.assertLessEqual({(runbook.BUILD_PRODUCT, '"session-fixture-jumble-on"'),
                              (runbook.RENDER_BACKEND, 'Puzzle::Jumbling =>')}, mentions)
        for puzzle in ('jumble-off', 'jumble-on'):
            launches = [command for command in step.commands
                        if f'python -B {runbook.LAUNCH} ' in command and f'--puzzle {puzzle},' in command]
            self.assertEqual(len(launches), 1, puzzle)
            self.assertIn(f'tools/perf/c9/lx4.py post --fixtures <fixtures> --puzzle {puzzle} --run', launches[0])
        self.assertFalse(any('refused until' in command for command in step.commands))
        # Both prerequisites hold on this source: the launcher's build writes the fixture and c600-render poses.
        self.assertTrue(runbook.evaluate('mentions', (runbook.BUILD_PRODUCT, '"session-fixture-jumble-on"'))[0])
        self.assertTrue(runbook.evaluate('mentions', (runbook.RENDER_BACKEND, 'Puzzle::Jumbling =>'))[0])

    def test_a_dry_run_prints_one_line_per_step_and_never_runs_the_quiet_gate(self):
        lines = []
        with mock.patch.object(runbook, 'quiet_machine', side_effect=forbidden):
            code = runbook.dry_run(check=lambda kind, argument: (True, 'ok'), out=lines.append)
        self.assertEqual(code, 0)
        self.assertEqual(len(lines), len(runbook.STEPS) + 1)
        for line, step in zip(lines, runbook.STEPS):
            self.assertTrue(line.startswith(f'{runbook.STEPS.index(step) + 1}. {step.id} '))
            self.assertEqual('quiet gate' in line, not step.attended)
            self.assertEqual('NOT BUILT YET' in line, not step.built)

    def test_the_exit_code_follows_the_prerequisites_of_built_steps_only(self):
        steps = (runbook.Step('ready', 'A', True, 1, None, prerequisites=(('file', 'a'),), sources=('s',)),
                 runbook.Step('later', 'B', False, 1, 'component', prerequisites=(('file', 'b'),), sources=('s',)))
        lines = []
        self.assertEqual(runbook.dry_run(steps, lambda kind, name: (name != 'b', name), lines.append), 0)
        self.assertIn('NOT BUILT YET: component (also missing: b)', lines[1])
        lines.clear()
        self.assertEqual(runbook.dry_run(steps, lambda kind, name: (name != 'a', name), lines.append), 1)
        self.assertIn('MISSING: a', lines[0])
        expected = int(any(step.built and step.prerequisites for step in runbook.STEPS))
        self.assertEqual(runbook.dry_run(check=lambda kind, name: (False, 'x'), out=lambda line: None), expected)

    def test_prerequisite_checks_read_files_only(self):
        self.assertTrue(runbook.evaluate('compiles', 'tools/perf/c9/runbook.py')[0])
        self.assertFalse(runbook.evaluate('file', 'tools/perf/c9/no-such-file')[0])
        self.assertFalse(runbook.evaluate('build', None, build=None)[0])
        build = self.temp / 'build'
        build.mkdir()
        identity = {'format': runbook.IDENTITY_FORMAT, 'source': {'head': 'a' * 40}, 'abi_version': 4}
        (build / runbook.IDENTITY).write_bytes(json.dumps(identity).encode())
        self.assertTrue(runbook.evaluate('build', None, build=build)[0])
        (build / runbook.IDENTITY).write_bytes(b'{"format": "other"}')
        self.assertFalse(runbook.evaluate('build', None, build=build)[0])
        with contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(runbook.main(['--list']), 0)
        self.assertNotIn('not built yet', out.getvalue())
        self.assertIn('(U8)', out.getvalue())
        later = runbook.Step('later', 'B', False, 1, 'component', commands=('run b',), sources=('s',))
        with contextlib.redirect_stdout(io.StringIO()) as out:
            runbook.listing((later,))
        self.assertIn('not built yet: component', out.getvalue())
        self.assertIn('planned: run b', out.getvalue())
        self.assertTrue(runbook.evaluate('mentions', ('tools/perf/c9', 'OTHER_ROWS'))[0])
        self.assertFalse(runbook.evaluate('mentions', ('tools/perf/c9', 'no' + ' such token'))[0])


def skew_rows(n, skew_ms, *, header=None, unknown=(), drawn_ms=None):
    """n 3D frames 1/60 s apart, each displayed 20 ms after submission, with a UI frame 1 ms after each (and its
    draw ending `drawn_ms` later)."""
    step = F // 60
    rows = [{'t': 'run', 'format': key_to_frame.LOG_FORMAT, 'run_id': 'skew', 'build_identity': BUILD,
             'qpc_frequency': F, 'synthetic': True, 'foreground_throughout': True, 'correctness': 'pass',
             'process_id': 4242, 'ui_swap_chain': '0xUI', **(header or {})}]
    for index in range(n):
        start = F + index * step
        rows.append({'t': 'frame', 'frame_id': index + 1, 'qpc': start, 'device_epoch': 1, 'surface': 1})
        rows.append({'t': 'presented', 'frame_id': index + 1, 'device_epoch': 1, 'surface': 1, 'sequence': index + 1,
                     'display_qpc': start + F // 50})
        rows.append({'t': 'ui_frame', 'ui_seq': index, 'qpc': start + F // 1000,
                     'frame_id': 9999 if index in unknown else index + 1})
        if drawn_ms is not None:
            rows[-1]['drawn_qpc'] = start + F // 1000 + round(drawn_ms * F / 1000)
    return rows


def skew_csv(n, skew_ms, *, missing=()):
    """The UI chain's presents, each called 2 ms after its 3D frame's submission."""
    step = F // 60
    head = ('Application,ProcessID,SwapChainAddress,Runtime,SyncInterval,PresentFlags,Dropped,TimeInSeconds,'
            'msInPresentAPI,msBetweenPresents,AllowsTearing,PresentMode,msUntilRenderComplete,msUntilDisplayed,'
            'msBetweenDisplayChange,msFlipDelay,msUntilRenderStart,msGPUActive,msSinceInput,QPCTime')
    lines = [head]
    for index in range(n):
        if index in missing:
            continue
        start = F + index * step + 2 * F // 1000
        until = 20.0 + skew_ms - 2.0
        lines.append(f'shell.exe,4242,0xUI,DXGI,1,0,0,0,0.1,16.67,0,Composed: Flip,1.0,{until},16.67,0,0.5,2,0,'
                     f'{start}')
    lines.append(f'shell.exe,4242,0xOTHER,DXGI,1,0,0,0,0.1,16.67,0,Composed: Flip,1.0,1.0,16.67,0,0.5,2,0,{F + 5}')
    return '\n'.join(lines) + '\n'


class UiSkewTests(NoProcessCase):
    def run_dir(self, rows, csv_text, name='skew'):
        directory = self.temp / name
        write_jsonl(directory / 'events.jsonl', rows)
        if csv_text is not None:
            (directory / 'presentmon.csv').write_bytes(csv_text.encode())
        return directory

    def test_each_ui_frame_pairs_with_its_ui_present_and_its_3d_frame(self):
        run = ui_skew.read_run(self.run_dir(skew_rows(120, 5.0), skew_csv(120, 5.0)))
        self.assertEqual(run['status'], 'valid', run['reasons'])
        self.assertEqual(run['pairs'], 120)
        self.assertAlmostEqual(run['stats']['signed']['p95'], 5.0, places=3)
        self.assertAlmostEqual(run['stats']['refresh_ms'], 1000 / 60, places=2)
        self.assertEqual(run['stats']['within_one_refresh'], 1.0)
        negative = ui_skew.read_run(self.run_dir(skew_rows(120, -4.0), skew_csv(120, -4.0), 'negative'))
        self.assertAlmostEqual(negative['stats']['signed']['p50'], -4.0, places=3)
        self.assertAlmostEqual(negative['stats']['absolute']['max'], 4.0, places=3)

    def test_unmatched_frames_get_no_value_and_too_few_pairs_invalidate(self):
        run = ui_skew.read_run(self.run_dir(skew_rows(110, 3.0), skew_csv(110, 3.0, missing={4, 5})))
        self.assertEqual(run['pairs'], 108)
        self.assertEqual(run['statuses'], {'matched': 108, 'ui-present-unmatched': 2})
        self.assertEqual(run['status'], 'valid')
        few = ui_skew.read_run(self.run_dir(skew_rows(50, 3.0), skew_csv(50, 3.0), 'few'))
        self.assertEqual((few['status'], few['reasons']), ('invalid', ['too-few-pairs']))

    def test_contradictions_conditions_and_a_missing_capture(self):
        run = ui_skew.read_run(self.run_dir(skew_rows(120, 1.0, unknown={7}), skew_csv(120, 1.0)))
        self.assertIn('contradiction', run['reasons'])
        lost = ui_skew.read_run(self.run_dir(skew_rows(120, 1.0, header={'foreground_throughout': False}),
                                             skew_csv(120, 1.0), 'lost'))
        self.assertIn('not-foreground', lost['reasons'])
        bare = ui_skew.read_run(self.run_dir(skew_rows(120, 1.0), None, 'bare'))
        self.assertEqual((bare['status'], bare['reasons']), ('unreadable', ['no-capture']))

    def test_a_null_ui_chain_is_the_one_other_chain_of_the_process(self):
        core = {'ui_swap_chain': None, 'swap_chain': '0xDEADBEEF'}
        printed = skew_csv(120, 5.0).replace('0xOTHER', '0x00000000deadbeef')
        run = ui_skew.read_run(self.run_dir(skew_rows(120, 5.0, header=core), printed))
        self.assertEqual(run['status'], 'valid', run['reasons'])
        self.assertEqual((run['pairs'], run['ui_swap_chain'], run['ui_swap_chain_inferred']), (120, '0xui', True))
        declared = ui_skew.read_run(self.run_dir(skew_rows(120, 5.0), skew_csv(120, 5.0), 'declared'))
        self.assertEqual((declared['ui_swap_chain'], declared['ui_swap_chain_inferred']), ('0xui', False))
        lost = ui_skew.read_run(self.run_dir(skew_rows(120, 5.0, header={'ui_swap_chain': None, 'swap_chain': None}),
                                             skew_csv(120, 5.0), 'lost'))
        self.assertEqual((lost['status'], lost['reasons']), ('unreadable', ['core-swap-chain-unknown']))
        core = {'ui_swap_chain': None, 'swap_chain': '0xother'}
        third = skew_csv(120, 5.0) + ('shell.exe,4242,0xTHIRD,DXGI,1,0,0,0,0.1,16.67,0,Composed: Flip,1.0,1.0,16.67,'
                                      '0,0.5,2,0,9\n')
        many = ui_skew.read_run(self.run_dir(skew_rows(120, 5.0, header=core), third, 'many'))
        self.assertEqual((many['status'], many['reasons']), ('unreadable', ['ui-swap-chain-ambiguous']))
        alone = {'ui_swap_chain': None, 'swap_chain': '0xUI'}
        only = skew_csv(120, 5.0).replace('0xOTHER', '0xUI')
        none = ui_skew.read_run(self.run_dir(skew_rows(120, 5.0, header=alone), only, 'alone'))
        self.assertEqual((none['status'], none['reasons']), ('unreadable', ['ui-swap-chain-absent']))
        bare = ui_skew.read_run(self.run_dir(skew_rows(120, 5.0, header={'ui_swap_chain': None}), skew_csv(120, 5.0),
                                             'undeclared'))
        self.assertEqual(bare['reasons'], ['ui-swap-chain-undeclared'])

    def test_each_run_reports_whether_the_ui_present_falls_inside_the_draw(self):
        inside = ui_skew.read_run(self.run_dir(skew_rows(120, 5.0, drawn_ms=3.0), skew_csv(120, 5.0)))
        self.assertEqual(inside['present_in_draw'], {'checked': 120, 'within': 120})
        late = ui_skew.read_run(self.run_dir(skew_rows(120, 5.0, drawn_ms=0.5), skew_csv(120, 5.0), 'late'))
        self.assertEqual((late['status'], late['present_in_draw']), ('valid', {'checked': 120, 'within': 0}))
        old = ui_skew.read_run(self.run_dir(skew_rows(120, 5.0), skew_csv(120, 5.0), 'old'))
        self.assertEqual(old['present_in_draw'], {'checked': 0, 'within': 0})
        early = skew_rows(120, 5.0, drawn_ms=3.0)
        first = next(row for row in early if row['t'] == 'ui_frame')
        first['drawn_qpc'] = first['qpc'] - 1
        bad = ui_skew.read_run(self.run_dir(early, skew_csv(120, 5.0), 'early'))
        self.assertEqual((bad['status'], bad['reasons']), ('unreadable', ['bad-ui_frame']))

    def test_the_record_has_no_bar_and_a_synthetic_input_never_passes(self):
        directory = self.run_dir(skew_rows(120, 2.0), skew_csv(120, 2.0))
        self.assertEqual(ui_skew.judge([ui_skew.read_run(directory)])[0], 'no-bar')
        record = ui_skew.build([directory], [str(directory)])
        self.assertEqual((record['verdict'], record['passes_bar'], record['bar']['limit_ms']),
                         ('synthetic', False, None))
        self.assertEqual(evidence.exit_code(record), 5)


def lx4_fixtures(directory):
    """A small fixture folder in c600-lx4's format: S, M, L and W00, the four edit kinds at each."""
    directory.mkdir(parents=True)
    modules, edits = [], []
    for name in ('S', 'M', 'L', 'W00'):
        source = (f'edition "6cs-draft-1";\nmodule lx4_{name.lower()} requires [];\n\n'
                  f'export fn exported() -> Nat effects [] {{\n    return 1;\n}}\n')
        path = f'{name}.6cs'
        (directory / path).write_bytes(source.encode())
        sha = lx4.sha256_bytes(source.encode())
        modules.append({'name': name, 'path': path, 'sha256': sha})
        body, signature = source.index('    return 1;'), source.index('fn exported() -> Nat')
        for kind, replacements in (
                ('inside-body', [(body, '    return 1;', '    return exact_add(1, 7);')]),
                ('exported-signature', [(signature, 'fn exported() -> Nat', 'fn exported() -> Text'),
                                        (body, '    return 1;', '    return "edited";')]),
                ('completion-trigger', [(len(source), '', 'fn c() -> Nat effects [] {\n    return exact_')]),
                ('syntax-break', [(body + 12, ';', '')])):
            edit = {'target': name, 'path': path, 'kind': kind, 'base_sha256': sha,
                    'replacements': [{'offset': o, 'delete': d, 'insert': i} for o, d, i in replacements]}
            edit['result_sha256'] = lx4.sha256_bytes(lx4.apply(source, edit).encode())
            edits.append(edit)
    (directory / 'MANIFEST.json').write_bytes(json.dumps({'format': lx4.FIXTURES_FORMAT, 'modules': modules}).encode())
    (directory / 'EDITS.json').write_bytes(json.dumps({'format': lx4.EDITS_FORMAT, 'edits': edits}).encode())
    return lx4.load_fixtures(directory)


def lx4_logs(edits, digests, puzzle, latency, *, wrong=(), build=BUILD):
    """A complete run: one timed post per sample, the endpoint's frame confirmed `latency(sample)` ms later."""
    events = [{'t': 'run', 'format': key_to_frame.LOG_FORMAT, 'run_id': puzzle, 'build_identity': build,
               'qpc_frequency': F, 'synthetic': True, 'foreground_throughout': True, 'correctness': 'pass',
               'puzzle': puzzle, 'window': 66}]
    posts = [{'t': 'run', 'format': lx4.POSTS_FORMAT, 'run_id': puzzle, 'puzzle': puzzle, 'build_identity': build,
              'qpc_frequency': F, **digests}]
    frame = 0
    for sample in lx4.schedule(edits, puzzle):
        if sample['metric'] == 'turn':
            continue
        frame += 1
        start = F * (sample['id'] + 1)
        posts.append({'t': 'post', 'sample': sample['id'], 'index': 0, 'message': 1, 'qpc': start, 'ok': True,
                      'timed': True})
        posts.append({'t': 'sample', 'sample': sample['id'], 'status': 'done', 'foreground': True})
        events.append({'t': 'frame', 'frame_id': frame, 'qpc': start + 1, 'device_epoch': 1, 'surface': 1})
        events.append({'t': 'presented', 'frame_id': frame, 'device_epoch': 1, 'surface': 1, 'sequence': frame,
                       'display_qpc': start + round(latency(sample) * F / 1000)})
        result = 'f' * 64 if sample['id'] in wrong else edits[sample['edit']]['result_sha256']
        events.append({'t': 'lx4', 'sample': sample['id'], 'endpoint': sample['metric'], 'frame_id': frame,
                       'buffer_sha256': result})
    return events, posts


class Lx4Tests(NoProcessCase):
    def setUp(self):
        super().setUp()
        self.manifest, self.edits, self.digests = lx4_fixtures(self.temp / 'fixtures')
        self.e2 = {'verdict': 'met', 'build_identity': BUILD, 'inputs': [{'sha256': 'events'}]}
        self.gate = {'format': 'magic600-renderer-gate-v1',
                     'scenes': [{'verdict': 'met', 'runs': [{'build_identity': BUILD}]}]}

    def runs(self, latency, **kwargs):
        return [lx4.analyze_run(*lx4_logs(self.edits, self.digests, puzzle, latency, **kwargs), self.edits,
                                self.digests) for puzzle in lx4.PUZZLES]

    def test_the_schedule_accounts_for_10800_sml_samples_apart_from_w_turns_and_smoke(self):
        counts = lx4.accounting([lx4.schedule(self.edits, puzzle) for puzzle in lx4.PUZZLES])
        self.assertEqual(counts, {'sml_samples': 10_800, 'w_samples': 600, 'turns': 600, 'smoke_samples': 40})
        samples = lx4.schedule(self.edits, 'jumble-on')
        self.assertEqual([sample['id'] for sample in samples], list(range(len(samples))))
        series = [sample for sample in samples if sample['series'] == 2]
        self.assertTrue(all(sample['metric'] == 'turn' for sample in series[19::20]))

    def test_messages_reset_select_and_type_and_time_the_metric_key(self):
        samples = {(sample['size'], sample['metric']): sample for sample in lx4.schedule(self.edits, 'jumble-off')}
        sent, timed = lx4.messages(self.edits, samples[('S', 'echo')])
        self.assertEqual(sent[0], (lx4.RESET, samples[('S', 'echo')]['edit'], samples[('S', 'echo')]['id']))
        self.assertEqual(sent[1][0], lx4.SELECT)
        self.assertEqual((sent[timed][0], timed), (lx4.WM_CHAR, 2))
        # Only a rerun sample's Reset carries the flag; it re-runs the same inside-body edit, timed at the last key.
        rerun = samples[('S', 'rerun')]
        sent, timed = lx4.messages(self.edits, rerun)
        self.assertEqual(rerun['edit'], samples[('S', 'echo')]['edit'])
        self.assertEqual(sent[0], (lx4.RESET, rerun['edit'] | lx4.RERUN_FLAG, rerun['id']))
        self.assertEqual(timed, len(sent) - 1)
        for metric in ('echo', 'syntax', 'completion', 'diagnostics', 'stale'):
            self.assertLess(lx4.messages(self.edits, samples[('M', metric)])[0][0][1], lx4.RERUN_FLAG, metric)
        sent, timed = lx4.messages(self.edits, samples[('S', 'syntax')])
        self.assertEqual(sent[timed:timed + 2], [(lx4.WM_KEYDOWN, lx4.VK_BACK, 0x000E0001),
                                                 (lx4.WM_KEYUP, lx4.VK_BACK, 0xC00E0001)])
        sent, timed = lx4.messages(self.edits, samples[('M', 'completion')])
        self.assertIn((lx4.WM_CHAR, 13, 1), sent)
        self.assertTrue(all(lparam == 1 for message, _, lparam in sent if message == lx4.WM_CHAR))
        self.assertEqual(timed, len(sent) - 1)
        sent, _ = lx4.messages(self.edits, samples[('L', 'diagnostics')])
        selects = [wparam for message, wparam, _ in sent if message == lx4.SELECT]
        self.assertEqual(selects, sorted(selects, reverse=True))
        turn = next(sample for sample in lx4.schedule(self.edits, 'jumble-off') if sample['metric'] == 'turn')
        self.assertEqual(lx4.messages(self.edits, turn),
                         ([(lx4.WM_KEYDOWN, 0x4A, 0x00240001), (lx4.WM_KEYUP, 0x4A, 0xC0240001)], 0))
        self.assertEqual(lx4.messages(self.edits, turn, 'KeyL')[0][0], (lx4.WM_KEYDOWN, 0x4C, 0x00260001))
        with self.assertRaises(lx4.FixtureError):
            lx4.messages(self.edits, turn, 'KeyT')

    def test_the_shell_module_matches_its_library_and_the_harness(self):
        """LX-4's C# module, its Rust library and lx4.py name the same endpoints, exports, codes and row layout."""
        module = (ROOT / 'renderer/shell/project/Lx4/Lx4Module.cs').read_text(encoding='utf-8')
        library = (ROOT / 'renderer/bridge/lx4/src/lib.rs').read_text(encoding='utf-8')
        # The module row that lx4.py's live poster reads, and the live log's name as the shell's EventLog spells it.
        for needle in ('json.WriteString("t", "module");', 'json.WriteString("module", "lx4");',
                       'json.WriteNumber("window", window);'):
            self.assertIn(needle, module)
        self.assertIn(f'public const string BodyName = "{lx4.LIVE}";',
                      (ROOT / 'renderer/shell/project/EventLog.cs').read_text(encoding='utf-8'))

        def csharp_list(name):
            return re.findall(r'"(\w+)"', re.search(name + r' = new\[\]\s*\{(.*?)\};', module, re.S).group(1))

        endpoints = re.findall(r'"(\w+)"', re.search(r'pub const ENDPOINTS: \[&str; \d+\] =\s*\[(.*?)\];', library,
                                                     re.S).group(1))
        self.assertEqual(csharp_list('Endpoints'), endpoints)
        self.assertEqual(set(endpoints), {*lx4.METRICS, lx4.W_METRIC[0], lx4.READY})
        self.assertEqual(csharp_list('Exports'), re.findall(r'pub (?:unsafe )?extern "C" fn (lx4_\w+)', library))
        for csharp, rust in (('AbiVersion', 'LX4_ABI_VERSION'), ('Ok', 'LX4_OK'),
                             ('InvalidArgument', 'LX4_E_INVALID_ARGUMENT'), ('FixturesRefused', 'LX4_E_FIXTURES'),
                             ('StateRefused', 'LX4_E_STATE'), ('Poisoned', 'LX4_E_POISONED'),
                             ('TextEcho', 'LX4_TEXT_ECHO'), ('TextStatus', 'LX4_TEXT_STATUS'),
                             ('TextCompletion', 'LX4_TEXT_COMPLETION'), ('TextBuffer', 'LX4_TEXT_BUFFER')):
            self.assertEqual(re.search(rf'const \w+ {csharp} = (\d+);', module).group(1),
                             re.search(rf'pub const {rust}: \w+ = (\d+);', library).group(1), csharp)
        # The re-run: the Reset flag lx4.py posts and the entry the module runs are the library's.
        self.assertEqual(1 << int(re.search(r'pub const LX4_RESET_RERUN: u32 = 1 << (\d+);', library).group(1)),
                         lx4.RERUN_FLAG)
        self.assertEqual(re.search(r'public const string ReferenceEntry = "(\w+)";', module).group(1),
                         re.search(r'pub const REFERENCE_ENTRY: &str = "(\w+)";', library).group(1))
        types = {'u64': 'ulong', 'u32': 'uint', '[u8; 64]': 'fixed byte[64]'}
        for csharp, rust in (('Update', 'Lx4Update'), ('Shown', 'Lx4Shown')):
            fields = re.search(rf'private struct {csharp}\s*\{{(.*?)\}}', module, re.S).group(1)
            fields = [kind + size for kind, size in re.findall(r'public (ulong|uint|fixed byte) \w+(\[64\])?;', fields)]
            native = re.search(rf'pub struct {rust} \{{(.*?)\}}', library, re.S).group(1)
            self.assertEqual(fields, [types[kind] for kind in re.findall(r'pub \w+: (u64|u32|\[u8; 64\])', native)],
                             csharp)

    def test_changed_fixtures_are_refused(self):
        (self.temp / 'fixtures' / 'M.6cs').write_bytes(b'changed\n')
        with self.assertRaises(lx4.FixtureError):
            lx4.load_fixtures(self.temp / 'fixtures')

    def test_every_bar_with_e2_and_the_gate_in_the_same_runs_is_an_all_bars_pass(self):
        verdict, reasons, decision, table = lx4.judge(self.runs(lambda sample: 10.0), self.e2, self.gate, {'events'})
        self.assertEqual((verdict, reasons, decision), ('met', [], 'all-bars'))
        self.assertEqual(sum(row['stats']['n'] for row in table if row['series'] and row['size'] != 'W'), 10_800)

    def test_rte53_decides_by_size(self):
        def late(size, metric, ms):
            return lambda sample: ms if (sample['size'], sample['metric'], sample['series']) == (size, metric, 2) \
                else 10.0
        self.assertEqual(lx4.judge(self.runs(late('L', 'echo', 120.0)), self.e2, self.gate, {'events'})[2],
                         'owner-l-or-w')
        self.assertEqual(lx4.judge(self.runs(late('W', 'dependents', 1600.0)), self.e2, self.gate, {'events'})[:3],
                         ('not-met', ['bar-missed'], 'owner-l-or-w'))
        self.assertEqual(lx4.judge(self.runs(late('S', 'syntax', 120.0)), self.e2, self.gate, {'events'})[2],
                         'fail-s-m')
        smoke = lambda sample: 900.0 if sample['series'] == 0 else 10.0  # noqa: E731
        self.assertEqual(lx4.judge(self.runs(smoke), self.e2, self.gate, {'events'})[0], 'met')

    def test_every_bar_below_100_ms_is_100_ms(self):
        # Owner decision, 22 November 2026, item 1: echo at S, M and L and syntax at S and M; nothing else moves.
        for metric, sizes in lx4.BARS.items():
            for size, (p95, p99) in sizes.items():
                self.assertGreaterEqual(min(p95, p99), 100, (metric, size))
        self.assertEqual({size: lx4.BARS['echo'][size] for size in lx4.SIZES}, dict.fromkeys(lx4.SIZES, (100, 100)))
        self.assertEqual(lx4.BARS['syntax'], {'S': (100, 100), 'M': (100, 100), 'L': (100, 150)})
        self.assertEqual(lx4.BARS['completion'], {'S': (100, 150), 'M': (100, 150), 'L': (150, 250)})
        self.assertEqual(lx4.BARS['dependents'], {'W': (1000, 1500)})

        def at(ms):
            return lambda sample: ms if (sample['size'], sample['metric']) == ('M', 'syntax') else 10.0
        self.assertEqual(lx4.judge(self.runs(at(100.0)), self.e2, self.gate, {'events'})[2], 'all-bars')
        self.assertEqual(lx4.judge(self.runs(at(100.5)), self.e2, self.gate, {'events'})[2], 'fail-s-m')

    def test_an_unconfirmed_sample_is_counted_apart_and_the_bars_judge_the_confirmed_ones(self):
        # Owner decision, 22 November 2026, item 10, which replaces item 2's E2-style counting: a sample without a
        # confirmed frame is counted apart, in the confirmation gap, a known limit; its run stays valid.
        def unconfirm(events, samples):
            frames = {row['frame_id'] for row in events if row['t'] == 'lx4' and row['sample'] in samples}
            return [row for row in events if not (row['t'] == 'presented' and row['frame_id'] in frames)]

        def runs_with(gap, latency=lambda sample: 10.0):
            runs = []
            for puzzle, samples in (('jumble-off', gap), ('jumble-on', set())):
                events, posts = lx4_logs(self.edits, self.digests, puzzle, latency)
                runs.append(lx4.analyze_run(unconfirm(events, samples), posts, self.edits, self.digests))
            return runs

        def echo_row(table):
            return next(row for row in table if (row['puzzle'], row['series'], row['size'], row['metric'])
                        == ('jumble-off', 1, 'S', 'echo'))

        cell = [sample['id'] for sample in lx4.schedule(self.edits, 'jumble-off')
                if (sample['series'], sample['size'], sample['metric']) == (1, 'S', 'echo')]
        runs = runs_with(set(cell[:2]))
        self.assertEqual((runs[0]['status'], runs[0]['failures'], runs[0]['unconfirmed']), ('valid', {}, 2))
        # Two of 100 without a confirmed frame no longer put p99 on a failed sample: every bar holds.
        verdict, reasons, decision, table = lx4.judge(runs, self.e2, self.gate, {'events'})
        self.assertEqual((verdict, reasons, decision), ('met', [], 'all-bars'))
        row = echo_row(table)
        self.assertEqual({key: row['stats'][key] for key in ('n', 'timed', 'unconfirmed', 'mean', 'p95', 'p99', 'max')},
                         {'n': 100, 'timed': 98, 'unconfirmed': 2, 'mean': 10.0, 'p95': 10.0, 'p99': 10.0,
                          'max': 10.0})
        self.assertTrue(row['met'])
        gap = lx4.confirmation_gap(runs)
        self.assertEqual((gap['source'], gap['judged']), ('owner decision 2026-11-22 (LX bars), item 10', False))
        self.assertEqual([(entry['puzzle'], entry['unconfirmed'], entry['cells']) for entry in gap['runs']],
                         [('jumble-off', 2, [{'series': 1, 'size': 'S', 'metric': 'echo', 'unconfirmed': 2}]),
                          ('jumble-on', 0, [])])
        # The confirmed samples still carry the bar: a slow cell misses whatever its gap.
        slow = lambda sample: 100.5 if (sample['series'], sample['size'], sample['metric']) == (1, 'S', 'echo') \
            else 10.0  # noqa: E731
        self.assertEqual(lx4.judge(runs_with(set(cell[:2]), slow), self.e2, self.gate, {'events'})[:3],
                         ('not-met', ['bar-missed'], 'fail-s-m'))
        # A cell with no confirmed sample has no percentile and misses; its samples are all accounted for.
        verdict, reasons, decision, table = lx4.judge(runs_with(set(cell)), self.e2, self.gate, {'events'})
        self.assertEqual((verdict, reasons, decision), ('not-met', ['bar-missed'], 'fail-s-m'))
        row = echo_row(table)
        self.assertEqual((row['stats']['n'], row['stats']['timed'], row['stats']['unconfirmed'], row['stats']['p99'],
                          row['met']), (100, 0, 100, None, False))
        self.assertFalse(hasattr(lx4, 'counted_percentile'))
        self.assertIn('owner decision 2026-11-22 (LX bars, items 1, 2 and 10)', lx4.BAR['source'])
        self.assertNotIn('unconfirmed sample invalidates its run', lx4.DRAFTS)

    def test_e2_and_the_gate_must_hold_in_the_same_runs(self):
        runs = self.runs(lambda sample: 10.0)
        self.assertEqual(lx4.judge(runs, None, self.gate, {'events'})[:3],
                         ('insufficient', ['e2-record-missing'], None))
        self.assertEqual(lx4.judge(runs, self.e2, self.gate, {'other runs'})[2], 'fail-gate-or-e2')
        failed = {**self.gate, 'scenes': [{'verdict': 'not-met', 'runs': [{'build_identity': BUILD}]}]}
        self.assertEqual(lx4.judge(runs, self.e2, failed, {'events'})[2], 'fail-gate-or-e2')

    def test_a_gate_reference_counts_as_the_gate_only_where_the_owner_accepted_it(self):
        """renderer_gate.py reads acceptance builds only; another build's summary is a reference (C9, 20 Nov 2026),
        and the gate only for the summary and build the owner accepted (22 Nov 2026, item 2)."""
        directories = []
        for puzzle in lx4.PUZZLES:
            events, posts = lx4_logs(self.edits, self.digests, puzzle, lambda sample: 10.0)
            write_jsonl(self.temp / puzzle / 'events.jsonl', events)
            write_jsonl(self.temp / puzzle / lx4.POSTS, posts)
            directories.append(self.temp / puzzle)
        e2 = self.temp / 'e2.json'
        e2.write_bytes(json.dumps({'verdict': 'met', 'build_identity': BUILD, 'inputs': [
            {'sha256': evidence.sha256(directory / 'events.jsonl')} for directory in directories]}).encode())
        reference = self.temp / 'w3-summary.json'
        reference.write_bytes(json.dumps({'format': 'magic600-renderer-gate-v1', 'scenes': [
            {'scene': 'w3', 'verdict': 'met', 'runs': [{'build_identity': 'acceptance-build'}]}]}).encode())
        digest = evidence.sha256(reference)
        record = lx4.build(self.temp / 'fixtures', directories, ['analyze'], e2_path=e2, reference_path=reference)
        self.assertEqual(record['result']['gate_reference'],
                         {'counted_as_gate': False, 'accepted_by': None, 'sha256': digest,
                          'build_identities': ['acceptance-build'], 'verdicts': {'w3': 'met'}})
        self.assertIn('gate-reference', [entry['role'] for entry in record['inputs']])
        # The record adds `synthetic-input` for these synthetic runs; the gate's reason is the only other one.
        self.assertEqual(record['reasons'], ['gate-summary-other-build', 'synthetic-input'])
        self.assertIsNone(record['result']['decision'])
        self.assertEqual(lx4.ACCEPTED_GATE_REFERENCES, {
            ('c7a43ae68505f1c4df5d59400189070346765dad5a119333533e6a2ac968a0ca', 'c9-product-2efda50d54592f20'):
                'owner decision 2026-11-22 (LX bars), item 2: U4 W3 summary of the C9 sitting'})
        with mock.patch.dict(lx4.ACCEPTED_GATE_REFERENCES, {(digest, BUILD): 'the owner'}):
            accepted = lx4.build(self.temp / 'fixtures', directories, ['analyze'], e2_path=e2,
                                 reference_path=reference)
            self.assertEqual(accepted['result']['gate_reference']['counted_as_gate'], True)
            self.assertEqual(accepted['result']['gate_reference']['accepted_by'], 'the owner')
            self.assertEqual((accepted['reasons'], accepted['result']['decision']), (['synthetic-input'], 'all-bars'))
            # A summary with a scene not met never counts, even where accepted.
            refused = json.loads(reference.read_text(encoding='utf-8'))
            refused['scenes'][0]['verdict'] = 'not-met'
            self.assertFalse(lx4.reference_counts({**lx4.gate_reference(refused, digest)}, BUILD))
        with mock.patch.dict(lx4.ACCEPTED_GATE_REFERENCES, {(digest, 'another-build'): 'the owner'}):
            other = lx4.build(self.temp / 'fixtures', directories, ['analyze'], e2_path=e2, reference_path=reference)
        self.assertEqual((other['result']['gate_reference']['counted_as_gate'], other['reasons']),
                         (False, ['gate-summary-other-build', 'synthetic-input']))
        plain = lx4.build(self.temp / 'fixtures', directories, ['analyze'])
        self.assertIsNone(plain['result']['gate_reference'])
        self.assertIn('gate-summary-missing', plain['reasons'])
        reference.write_bytes(b'{"format": "something else"}')
        with self.assertRaises(lx4.FixtureError):
            lx4.build(self.temp / 'fixtures', directories, ['analyze'], reference_path=reference)
        with contextlib.redirect_stderr(io.StringIO()) as err:
            code = lx4.main(['analyze', '--fixtures', str(self.temp / 'fixtures'), *map(str, directories),
                             '--gate-reference', str(reference)])
        self.assertEqual(code, 2)
        self.assertIn('not a renderer gate summary', err.getvalue())

    def test_a_wrong_buffer_invalidates_its_run(self):
        runs = self.runs(lambda sample: 10.0, wrong={5})
        self.assertEqual((runs[0]['failures'], runs[0]['reasons']), ({'wrong-buffer': 1}, ['failed-samples']))
        self.assertEqual(lx4.judge(runs, self.e2, self.gate, {'events'})[:2],
                         ('invalid', ['one-valid-run-per-puzzle-needed']))

    def test_the_record_of_synthetic_runs_never_passes(self):
        directories = []
        for puzzle in lx4.PUZZLES:
            events, posts = lx4_logs(self.edits, self.digests, puzzle, lambda sample: 10.0)
            write_jsonl(self.temp / puzzle / 'events.jsonl', events)
            write_jsonl(self.temp / puzzle / lx4.POSTS, posts)
            directories.append(self.temp / puzzle)
        record = lx4.build(self.temp / 'fixtures', directories, ['analyze'])
        self.assertEqual((record['verdict'], record['passes_bar']), ('synthetic', False))
        self.assertEqual(record['result']['timed_sml_samples'], 10_800)
        self.assertEqual(record['result']['limitations'], lx4.LIMITATIONS)
        self.assertIn('NoHost', lx4.LIMITATIONS[0])
        self.assertIn("does not draw the run's value", lx4.LIMITATIONS[1])

    def test_a_dry_run_posts_nothing(self):
        with contextlib.redirect_stdout(io.StringIO()) as out:
            code = lx4.main(['post', '--fixtures', str(self.temp / 'fixtures'), '--puzzle', 'jumble-on', '--dry-run'],
                            system_factory=forbidden)
        self.assertEqual(code, 0)
        self.assertIn('turn key KeyJ: virtual key 0x4A, scan code 0x24', out.getvalue())
        self.assertIn('nothing was posted', out.getvalue())

    def test_the_poster_waits_for_ready_and_each_endpoint_and_logs_every_post(self):
        run = self.temp / 'run'
        # The live log: frame rows, then the module row with the window (no run row before the shell's close).
        write_jsonl(run / lx4.LIVE, [{'t': 'frame', 'frame_id': 1, 'qpc': 1, 'device_epoch': 1, 'surface': 1},
                                     {'t': 'module', 'module': 'lx4', 'window': 66, 'run_id': 'live',
                                      'build_identity': BUILD, 'puzzle': 'jumble-off', 'qpc_frequency': F}])
        events = run / lx4.LIVE

        def lx4_row(sample, endpoint):
            with events.open('a') as stream:
                stream.write(json.dumps({'t': 'lx4', 'sample': sample, 'endpoint': endpoint}) + '\n')

        class Fake:
            """The shell: a Reset's ready row comes only at the harness's next wait, so an input posted before the
            harness reads it counts as early."""

            def __init__(self, answer=True, acknowledge=True):
                self.answer, self.ticks, self.pending, self.reset, self.early = answer, 0, None, None, 0
                self.acknowledge, self.sample, self.selects, self.keys = acknowledge, None, 0, []

            def register(self, name):
                return {lx4.RESET: 0xC001, lx4.SELECT: 0xC002}[name]

            def qpc(self):
                self.ticks += 1
                return self.ticks

            def frequency(self):
                return F

            def post(self, hwnd, message, wparam, lparam):
                if message == 0xC001:
                    self.pending = lparam if self.answer else None
                    self.sample, self.selects = lparam, 0
                elif message == 0xC002:
                    self.early += self.pending is not None
                    if self.answer and self.acknowledge:
                        self.selects += 1
                        with events.open('a') as stream:
                            stream.write(json.dumps({'t': lx4.SELECTED, 'sample': self.sample,
                                                     'select': self.selects}) + '\n')
                elif message in (lx4.WM_CHAR, lx4.WM_KEYDOWN) and wparam != 0x4A:
                    self.keys.append(self.sample)
                    if self.reset is not None:
                        for metric in (*lx4.METRICS, 'dependents'):
                            lx4_row(self.reset, metric)
                        self.reset = None
                if self.answer and message == lx4.WM_KEYDOWN and wparam == 0x4A and lparam >> 16 & 0xFF == 0x24:
                    with events.open('a') as stream:
                        stream.write(json.dumps({'t': 'key', 'seq': 0, 'qpc': 1, 'frame_id': None}) + '\n')
                return hwnd == 66

            def sleep(self, seconds):
                if self.pending is not None:
                    lx4_row(self.pending, lx4.READY)
                    self.reset, self.pending = self.pending, None

            def foreground(self, hwnd):
                return True

            def alive(self, hwnd):
                return True

        clock = iter(range(10**9))
        with mock.patch.object(lx4, 'PER_SERIES', 2), mock.patch.object(lx4, 'SERIES', 1):
            samples = lx4.schedule(self.edits, 'jumble-off')
            fake = Fake()
            code = lx4.post_run(self.temp / 'fixtures', 'jumble-off', run, 'KeyJ', fake,
                                clock=lambda: next(clock) / 100, sleep=fake.sleep, out=lambda line: None)
            self.assertEqual((code, fake.early), (0, 0))
            rows = evidence.read_jsonl(run / lx4.POSTS)
            self.assertEqual(rows[0]['turn_key'], 'KeyJ')
            self.assertEqual(sum(row['t'] == 'sample' and row['status'] == 'done' for row in rows), len(samples))
            self.assertEqual(sum(row['t'] == 'post' and row['timed'] for row in rows), len(samples))
            (run / lx4.POSTS).unlink()
            fake = Fake(answer=False)
            code = lx4.post_run(self.temp / 'fixtures', 'jumble-off', run, 'KeyJ', fake,
                                clock=lambda: next(clock) / 2, sleep=fake.sleep, out=lambda line: None)
            self.assertEqual(code, 3)
            rows = evidence.read_jsonl(run / lx4.POSTS)
            status = {row['sample']: row['status'] for row in rows if row['t'] == 'sample'}
            self.assertEqual(status, {sample['id']: 'timeout' if sample['metric'] == 'turn' else 'not-ready'
                                      for sample in samples})
            # Without a ready row, nothing after the Reset is posted.
            posts = [row for row in rows if row['t'] == 'post' and status[row['sample']] == 'not-ready']
            self.assertEqual(sorted(row['sample'] for row in posts),
                             sorted(sample for sample, state in status.items() if state == 'not-ready'))
            self.assertTrue(all(row['message'] == 0xC001 for row in posts))
            # C9 fix, 20 Nov 2026: without a Select's lx4-select row, no key of its replacement is posted.
            (run / lx4.POSTS).unlink()
            fake = Fake(acknowledge=False)
            code = lx4.post_run(self.temp / 'fixtures', 'jumble-off', run, 'KeyJ', fake,
                                clock=lambda: next(clock) / 2, sleep=fake.sleep, out=lambda line: None)
            self.assertEqual((code, fake.keys), (3, []))
            rows = evidence.read_jsonl(run / lx4.POSTS)
            status = {row['sample']: row['status'] for row in rows if row['t'] == 'sample'}
            self.assertEqual(status, {sample['id']: 'done' if sample['metric'] == 'turn' else 'not-selected'
                                      for sample in samples})
            last = {row['sample']: row['message'] for row in rows if row['t'] == 'post'}
            self.assertTrue(all(last[sample] == 0xC002 for sample, state in status.items() if state == 'not-selected'))

    def test_a_sample_without_its_ready_row_invalidates_its_run(self):
        events, posts = lx4_logs(self.edits, self.digests, 'jumble-off', lambda sample: 10.0)
        first = next(row['sample'] for row in posts if row['t'] == 'sample')
        posts = [row for row in posts if not (row['t'] == 'post' and row['sample'] == first and row['index'] > 0)]
        posts = [{**row, 'status': 'not-ready'} if row['t'] == 'sample' and row['sample'] == first else row
                 for row in posts]
        run = lx4.analyze_run(events, posts, self.edits, self.digests)
        self.assertEqual((run['status'], run['failures']), ('invalid', {'not-ready': 1}))


CHILD_SHA = 'c' * 64
PRESENTMON_HEAD = ('Application,ProcessID,SwapChainAddress,Runtime,SyncInterval,PresentFlags,Dropped,TimeInSeconds,'
                   'msInPresentAPI,msBetweenPresents,AllowsTearing,PresentMode,msUntilRenderComplete,msUntilDisplayed,'
                   'msBetweenDisplayChange,msFlipDelay,msUntilRenderStart,msGPUActive,msSinceInput,QPCTime')
LX2_CODES = {'alloc_metered': 'KN7:cancelled.revision_replaced', 'recursion_metered': 'KN7:cancelled.revision_replaced',
             'loop_metered': 'KN7:cancelled.revision_replaced', 'forced_kill': lx2.RUNTIME_LOST,
             'alloc_ceiling': lx2.RUNTIME_LOST, 'recursion_native': lx2.RUNTIME_LOST,
             'loop_native': 'KN7:resource_limit.deadline'}


def lx2_window_rows(fault, kind, start):
    """One effective window of `fault` and `kind` as LX-2's module writes it, from its start row to its recovered row;
    a limit window's job ends at its meter's limit."""
    ms = F // 1000
    rows = [{'event': 'start', 'qpc': start, 'ticket': 1, 'armed': lx2.ARMED.get(fault), 'ended_before': 0}]
    end = start + 600 * ms
    if kind == 'limit':
        return rows + [{'event': 'fault_end', 'qpc': start + 400 * ms, 'status': 'refused', 'code': lx2.BUDGET[fault]},
                       {'event': 'recovered', 'qpc': end, 'probes': 1}]
    rows.append({'event': 'turn', 'qpc': start + 300 * ms, 'ok': True})
    if fault in lx2.METERED:
        rows.append({'event': 'fault_end', 'qpc': start + 350 * ms})
    elif fault == 'forced_kill':
        rows += [{'event': 'kill', 'qpc': start + 250 * ms, 'killed': 1},
                 {'event': 'fault_end', 'qpc': start + 260 * ms}]
    elif fault in lx2.NATIVE_LOST:
        rows.append({'event': 'fault_end', 'qpc': start + 200 * ms})
    else:
        exit_qpc = start + 55 * F
        rows += [{'event': 'child_exit', 'qpc': exit_qpc + ms, 'found': True, 'pid': 7, 'exit_code': 1,
                  'exit_qpc': exit_qpc},
                 {'event': 'fault_end', 'qpc': exit_qpc + 2 * ms}]
        end = exit_qpc + 500 * ms
    for row in rows:
        if row['event'] == 'fault_end':
            row.update(status='refused', code=LX2_CODES[fault])
    rows.append({'event': 'recovered', 'qpc': end, 'probes': 1})
    return rows


def lx2_run(directory, edits, digests, *, windows=4, e2_ms=30.0, echo_ms=20.0, gate_ms=16.0, change=None,
            build=BUILD):
    """A run directory with `windows` effective windows per phase, each with one echo sample 100 ms after its start
    and, in a turn window, one turn at 300 ms: events.jsonl, lx2-posts.jsonl, launch.json and presentmon.csv.
    `change(fault, kind, index, rows)` may edit a window's lx2 rows; presents between windows are slow, so that only
    in-window ones pass."""
    ms = F // 1000
    edit = lx2.echo_edit(edits)
    events = [{'t': 'run', 'format': key_to_frame.LOG_FORMAT, 'run_id': 'lx2-run', 'build_identity': build,
               'qpc_frequency': F, 'synthetic': True, 'foreground_throughout': True, 'correctness': 'pass',
               'process_id': 4242, 'swap_chain': '0xCORE'},
              {'t': 'module', 'module': 'lx2', 'window': 66, 'run_id': 'lx2-run', 'build_identity': build}]
    posts = [{'t': 'run', 'format': lx2.POSTS_FORMAT, 'run_id': 'lx2-run', 'build_identity': build,
              'qpc_frequency': F, 'synthetic': True, 'faults': list(lx2.FAULTS), 'echo_edit': edit, **digests}]
    lines = [PRESENTMON_HEAD]
    state = {'frame': 0, 'key': 0, 'sample': 0, 'window': 0}

    def present(qpc, interval):
        lines.append(f'shell.exe,4242,0xCORE,DXGI,1,0,0,0,0.1,{interval},0,Composed: Flip,1.0,0.0,{interval},0,0.5,'
                     f'2,0,{qpc}')

    def framed(submit, display):
        state['frame'] += 1
        frame = state['frame']
        events.append({'t': 'frame', 'frame_id': frame, 'qpc': submit, 'device_epoch': 1, 'surface': 1})
        events.append({'t': 'presented', 'frame_id': frame, 'device_epoch': 1, 'surface': 1, 'sequence': frame,
                       'display_qpc': display})
        present(display, gate_ms)
        return frame

    t = F
    for _, fault, kind in lx2.PHASES:
        events.append({'t': 'lx2', 'event': 'phase', 'qpc': t, 'window': state['window'], 'fault': fault,
                       'kind': kind})
        t += ms
        for index in range(windows):
            state['window'] += 1
            rows = lx2_window_rows(fault, kind, t)
            if change:
                change(fault, kind, index, rows)
            state['sample'] += 1
            sample, anchor = state['sample'], t + 100 * ms
            for post, qpc in enumerate((anchor - ms, anchor)):
                posts.append({'t': 'post', 'sample': sample, 'fault': fault, 'kind': kind, 'index': post,
                              'message': 0xC001 + post, 'wparam': edit, 'lparam': sample, 'qpc': qpc, 'ok': True,
                              'timed': post == 1})
            posts.append({'t': 'sample', 'sample': sample, 'fault': fault, 'kind': kind, 'status': 'done',
                          'foreground': True})
            echo = framed(anchor + ms, anchor + round(echo_ms * ms))
            events.append({'t': 'lx4', 'sample': sample, 'endpoint': 'echo', 'frame_id': echo,
                           'buffer_sha256': edits[edit]['result_sha256']})
            for offset in (50, 150, 450):
                present(t + offset * ms, gate_ms)
            for row in rows:
                if row['event'] == 'turn' and row.get('ok'):
                    key = {'t': 'key', 'seq': state['key'], 'qpc': row['qpc'] + ms}
                    state['key'] += 1
                    key['frame_id'] = framed(row['qpc'] + 2 * ms, row['qpc'] + ms + round(e2_ms * ms))
                    events.append(key)
            events.extend({'t': 'lx2', 'window': state['window'], 'fault': fault, 'kind': kind, **row}
                          for row in sorted(rows, key=lambda row: row['qpc']))
            end = max(row['qpc'] for row in rows)
            present(end + 100 * ms, 100.0)
            t = end + 200 * ms
        events.append({'t': 'lx2', 'event': 'phase_end', 'qpc': t, 'window': state['window'], 'fault': fault,
                       'kind': kind, 'windows': windows})
    write_jsonl(directory / key_to_frame.EVENTS, events)
    write_jsonl(directory / lx2.POSTS, posts)
    (directory / key_to_frame.CAPTURE).write_bytes(('\n'.join(lines) + '\n').encode())
    (directory / lx2.LAUNCH).write_bytes(json.dumps(
        {'format': 'magic600-c9-launch-v1', 'run_id': 'lx2-run', 'build_identity': build, 'lx4': True,
         'lx2_injection': {'name': 'c600-lx2-child.exe', 'sha256': CHILD_SHA}}).encode())
    return directory


def lx2_faults(path, *, disagree=(), sha=CHILD_SHA, sessions=True):
    trials = ('control', *lx2.FAULTS)
    path.write_bytes(json.dumps({
        'format': lx2.FAULTS_FORMAT, 'agrees': not disagree,
        'runtime': {'file': 'c600-runtime.exe', 'sha256': 'a' * 64,
                    'injection_device': {'file': 'c600-lx2-child.exe', 'sha256': sha}},
        'trials': [{'trial': trial, 'agrees': trial not in disagree} for trial in trials],
        'sessions': [{'trial': trial, 'open': True, 'agrees_with_control': trial not in disagree}
                     for trial in trials] if sessions else []}).encode())
    return path


class Lx2Tests(NoProcessCase):
    def setUp(self):
        super().setUp()
        self.manifest, self.edits, self.digests = lx4_fixtures(self.temp / 'fixtures')
        for name, value in (('OBSERVATIONS_MIN', 4), ('GATE_FRAMES_MIN', 10)):
            patcher = mock.patch.object(lx2, name, value)
            patcher.start()
            self.addCleanup(patcher.stop)
        self.count = 0

    def record(self, *, disagree=(), sha=CHILD_SHA, sessions=True, **kwargs):
        self.count += 1
        directory = lx2_run(self.temp / f'run{self.count}', self.edits, self.digests, **kwargs)
        faults = lx2_faults(self.temp / f'faults{self.count}.json', disagree=disagree, sha=sha, sessions=sessions)
        return lx2.build(self.temp / 'fixtures', directory, faults, ['analyze'])

    def test_the_module_and_the_tool_agree_on_faults_timing_and_variables(self):
        source = (ROOT / 'renderer/shell/project/Lx2/Lx2Module.cs').read_text(encoding='utf-8')
        block = re.search(r'string\[\] Faults =\s*\{(.*?)\};', source, re.S)
        self.assertEqual(tuple(re.findall(r'"(\w+)"', block.group(1))), lx2.FAULTS)
        for name, seconds in (('LeadMs', lx2.LEAD_S), ('PaceMs', lx2.PACE_S), ('GapMs', lx2.GAP_S)):
            value = re.search(rf'const int {name} = (\d+);', source)
            self.assertEqual(int(value.group(1)), round(seconds * 1000), name)
        self.assertIn('public const long CeilingBytes = 32L << 20;', source)
        self.assertEqual(lx2.ARMED['alloc_ceiling'], f'C600_LX2_CEILING={32 << 20}')
        for fault, text in (('recursion_native', 'recursion@0'), ('loop_native', 'loop@0')):
            self.assertEqual(lx2.ARMED[fault], 'C600_LX2_FAULT=' + text)
            self.assertIn(f'(FaultVariable, "{text}")', source)
        self.assertIn(f'Register("{lx2.PHASE}")', source)
        # Limit windows: wParam len(FAULTS) plus a metered fault's number, and the kind each row carries.
        constants = dict(re.findall(r'private const int (\w+) = (\d+);', source))
        self.assertEqual([int(constants[name]) for name in ('AllocMetered', 'RecursionMetered', 'LoopMetered')],
                         [lx2.FAULTS.index(fault) + 1 for fault in lx2.METERED])
        self.assertIn('if (wParam > (nuint)(Faults.Length + LoopMetered))', source)
        self.assertIn('_limit = _phase > Faults.Length;', source)
        self.assertIn('var kind = _limit ? "limit" : "turn";', source)
        self.assertIn('if (!_limit && !_turned && !_faultFinal && elapsed >= LeadMs)', source)
        self.assertEqual(lx2.KINDS, ('limit', 'turn'))
        self.assertEqual([number for number, fault, kind in lx2.PHASES if kind == 'limit'],
                         [len(lx2.FAULTS) + lx2.FAULTS.index(fault) + 1 for fault in lx2.METERED])
        self.assertIn('json.WriteString("module", "lx2");', source)
        self.assertIn('lx2', key_to_frame.OTHER_ROWS)

    def test_a_complete_synthetic_run_meets_every_condition_but_stays_synthetic(self):
        record = self.record()
        run = record['result']['run']
        self.assertEqual((run['status'], run['reasons']), ('valid', []))
        self.assertEqual(lx2.judge(run), ('met', []))
        self.assertEqual((record['verdict'], record['passes_bar']), ('synthetic', False))
        for fault in lx2.FAULTS:
            result = run['faults'][fault]
            kinds = lx2.KINDS if fault in lx2.METERED else ('turn',)
            self.assertEqual(list(result['windows']), list(kinds), fault)
            for kind in kinds:
                self.assertEqual((result['windows'][kind]['windows'], result['windows'][kind]['effective']), (4, 4),
                                 (fault, kind))
            self.assertEqual((result['e2']['turns'], result['e2']['confirmed'], result['echo']['timed']), (4, 4, 4),
                             fault)
            over = 'limit' if fault in lx2.METERED else 'turn'
            self.assertEqual((result['e2']['over'], result['echo']['over'], result['gate']['over']),
                             ('turn', over, over), fault)
            # Per window the echo's frame, the turn's frame (a turn window) and three other presents; the slow
            # presents between windows are left out.
            self.assertEqual(result['gate']['presents'], 16 if fault in lx2.METERED else 20, fault)
            self.assertEqual(result['gate']['result']['max_ms'], 16.0, fault)
        self.assertEqual(run['faults']['loop_metered']['windows']['limit']['ends'], {'KN7:budget.steps': 4})
        self.assertEqual(run['conditions_1_2'], {'state': 'met', 'reasons': []})
        self.assertIn('limitations', record['result'])
        self.assertFalse(any('revision_replaced' in text for text in record['result']['limitations']))
        ends = {fault: [lx2.BUDGET[fault], lx2.SAFETY_STOP] for fault in lx2.METERED}
        ends['alloc_metered'].append(lx2.KERNEL_COST)
        self.assertEqual(record['bar']['limit_ends'], ends)
        self.assertEqual(record['result']['limit_windows']['ends'], ends)
        self.assertTrue(any('60 s view deadline' in text for text in record['result']['limit_windows']['facts']))

    def test_a_limit_window_counts_only_when_its_job_ended_at_its_limit_with_no_turn(self):
        def change(fault, kind, index, rows):
            if kind != 'limit':
                return
            end = next(row for row in rows if row['event'] == 'fault_end')
            if index == 0:
                end['code'] = lx2.SAFETY_STOP
            elif index == 1 and fault == 'alloc_metered':
                rows.insert(1, {'event': 'turn', 'qpc': end['qpc'] - 100 * (F // 1000), 'ok': True})
            elif index == 1 and fault == 'recursion_metered':
                end['code'] = 'KN7:cancelled.revision_replaced'

        # The host's view deadline is the limit that stops the job in the product, so it counts; a turn or another
        # end does not.
        run = self.record(change=change)['result']['run']
        why = {'alloc_metered': {'turned': 1}, 'recursion_metered': {'limit-not-reached': 1}, 'loop_metered': {}}
        for fault in lx2.METERED:
            limit = run['faults'][fault]['windows']['limit']
            self.assertEqual((limit['effective'], limit['ineffective']), (4 - sum(why[fault].values()), why[fault]),
                             fault)
            # Condition 3 comes from the limit windows alone; the turn windows still carry condition 4.
            self.assertEqual(run['faults'][fault]['echo']['timed'], limit['effective'], fault)
            state = 'met' if limit['effective'] == 4 else 'insufficient'
            self.assertEqual(run['faults'][fault]['echo']['state'], state, fault)
            self.assertEqual(run['faults'][fault]['e2']['state'], 'met', fault)
        self.assertEqual(run['faults']['loop_metered']['windows']['limit']['ends'],
                         {'KN7:budget.steps': 3, lx2.SAFETY_STOP: 1})
        self.assertEqual(lx2.judge(run)[0], 'insufficient')
        # LIMIT_ENDS alone decides: with the meter's codes only, the safety stop does not count.
        with mock.patch.dict(lx2.LIMIT_ENDS, {fault: (lx2.BUDGET[fault],) for fault in lx2.METERED}):
            run = self.record(change=change)['result']['run']
        self.assertEqual(run['faults']['loop_metered']['windows']['limit']['ineffective'], {'limit-not-reached': 1})
        self.assertEqual(run['faults']['alloc_metered']['windows']['limit']['ineffective'],
                         {'limit-not-reached': 1, 'turned': 1})

    def test_each_fault_counts_only_windows_where_it_took_effect(self):
        def change(fault, kind, index, rows):
            if index or kind != 'turn':
                return
            events = {row['event']: row for row in rows}
            if fault in lx2.METERED:
                events['turn']['qpc'] = events['fault_end']['qpc'] + 1
            elif fault == 'forced_kill':
                events['kill']['killed'] = 0
            elif fault == 'alloc_ceiling':
                events['start']['armed'] = None
            elif fault == 'recursion_native':
                events['fault_end']['code'] = 'KN7:resource_limit.depth'
            else:
                events['child_exit']['exit_qpc'] = events['start']['qpc'] + 50 * F

        run = self.record(change=change)['result']['run']
        why = {'alloc_metered': 'turn-after-the-job-ended', 'recursion_metered': 'turn-after-the-job-ended',
               'loop_metered': 'turn-after-the-job-ended', 'forced_kill': 'nothing-killed',
               'alloc_ceiling': 'not-armed', 'recursion_native': 'child-not-lost', 'loop_native': 'child-ended-early'}
        for fault in lx2.FAULTS:
            result = run['faults'][fault]
            turn = result['windows']['turn']
            self.assertEqual((turn['effective'], turn['ineffective']), (3, {why[fault]: 1}), fault)
            self.assertEqual(result['e2']['state'], 'insufficient', fault)
        self.assertEqual(lx2.judge(run)[0], 'insufficient')

    def test_a_window_without_recovery_or_a_refused_job_does_not_count(self):
        def change(fault, kind, index, rows):
            if fault == 'alloc_metered' and index == 0:
                rows[:] = [row for row in rows if row['event'] != 'recovered']
            if fault in ('forced_kill', 'loop_metered') and index == 0:
                next(row for row in rows if row['event'] == 'fault_end')['status'] = 'published'

        run = self.record(change=change)['result']['run']
        for kind in lx2.KINDS:
            self.assertEqual(run['faults']['alloc_metered']['windows'][kind]['ineffective'], {'not-recovered': 1})
            self.assertEqual(run['faults']['loop_metered']['windows'][kind]['ineffective'], {'job-not-refused': 1})
        self.assertEqual(run['faults']['forced_kill']['windows']['turn']['ineffective'], {'job-not-refused': 1})

    def test_a_key_without_a_confirmed_frame_is_a_failed_key_of_condition_4_and_the_run_stays_valid(self):
        # Condition 4 is charter E2 during fault windows, so it counts as E2 counts (owner, 20 November 2026, item 2).
        directory = lx2_run(self.temp / 'run-failed', self.edits, self.digests, windows=5)
        path = directory / key_to_frame.EVENTS
        rows = [json.loads(line) for line in path.read_text(encoding='utf-8').splitlines() if line.strip()]
        frame = next(row['frame_id'] for row in rows if row['t'] == 'key')
        write_jsonl(path, [row for row in rows if not (row['t'] == 'presented' and row['frame_id'] == frame)])
        faults = lx2_faults(self.temp / 'faults-failed.json')
        run = lx2.build(self.temp / 'fixtures', directory, faults, ['analyze'])['result']['run']
        self.assertEqual(run['status'], 'valid', run['reasons'])
        self.assertEqual((run['e2_run']['unconfirmed'], run['e2_run']['status']), (1, 'valid'))
        hit = [fault for fault in lx2.FAULTS if run['faults'][fault]['e2']['unconfirmed']]
        self.assertEqual(len(hit), 1)
        e2 = run['faults'][hit[0]]['e2']
        # Five keys, one failed: nearest rank 5 of 5 lies above the bar.
        self.assertEqual((e2['confirmed'], e2['stats']['failed_keys'], e2['stats']['p95'], e2['state']),
                         (4, 1, None, 'not-met'))
        for fault in lx2.FAULTS:
            if fault not in hit:
                self.assertEqual(run['faults'][fault]['e2']['state'], 'met', fault)

    def test_slow_commits_a_slow_echo_or_slow_presents_fail_their_condition(self):
        for kwargs, condition in (({'e2_ms': 150.0}, 'e2'), ({'echo_ms': 120.0}, 'echo'), ({'gate_ms': 40.0}, 'gate')):
            run = self.record(**kwargs)['result']['run']
            self.assertEqual(run['status'], 'valid', kwargs)
            verdict, reasons = lx2.judge(run)
            self.assertEqual(verdict, 'not-met', kwargs)
            self.assertEqual({reason.split(':')[1] for reason in reasons}, {f'{condition}-not-met'}, kwargs)
            # The gate is not required in the faults that lose the runtime (owner, 22 November 2026, item 4).
            faults = {reason.split(':')[0] for reason in reasons}
            self.assertEqual(faults, set(lx2.METERED) if condition == 'gate' else set(lx2.FAULTS), kwargs)

    def test_the_echo_bar_is_100_ms_at_p95_and_p99(self):
        # Owner decision, 22 November 2026, item 1: LX-4's echo bar, 50/67 ms before, is 100/100 ms.
        self.assertEqual(lx2.ECHO_BAR_MS, (100, 100))
        self.assertEqual(lx2.judge(self.record(echo_ms=99.0)['result']['run']), ('met', []))
        run = self.record(echo_ms=101.0)['result']['run']
        self.assertEqual({reason.split(':')[1] for reason in lx2.judge(run)[1]}, {'echo-not-met'})

    def test_the_gate_inside_the_runtime_loss_faults_is_a_known_limit_not_a_condition(self):
        record = self.record(gate_ms=40.0)
        run = record['result']['run']
        self.assertEqual(lx2.RUNTIME_LOSS, ('forced_kill', 'alloc_ceiling', 'recursion_native', 'loop_native'))
        self.assertEqual(record['bar']['gate_not_required'], list(lx2.RUNTIME_LOSS))
        for fault in lx2.FAULTS:
            gate = run['faults'][fault]['gate']
            # Still computed: the figures are recorded either way.
            self.assertEqual((gate['state'], gate['required']), ('not-met', fault not in lx2.RUNTIME_LOSS), fault)
            self.assertAlmostEqual(gate['result']['fps'], 25.0, places=3, msg=fault)
        limits = [entry for entry in record['result']['known_limits'] if entry['condition'] == 'gate']
        self.assertEqual([entry['fault'] for entry in limits], list(lx2.RUNTIME_LOSS))
        self.assertTrue(all(entry['state'] == 'not-met' and entry['required'] is False and entry['p99_ms'] == 40.0
                            and entry['source'] == 'owner decision 2026-11-22 (LX bars), item 4' for entry in limits))
        # The echo bar and condition 4 still apply in those windows.
        run = self.record(echo_ms=120.0)['result']['run']
        self.assertTrue({f'{fault}:echo-not-met' for fault in lx2.RUNTIME_LOSS} <= set(lx2.judge(run)[1]))
        self.assertEqual([entry['state'] for entry in self.record()['result']['known_limits']
                          if entry['condition'] == 'gate'], ['met'] * len(lx2.RUNTIME_LOSS))

    def test_alloc_metered_counts_its_kernel_cost_end_and_only_that_fault_does(self):
        def change(fault, kind, index, rows):
            if kind == 'limit' and index == 0:
                next(row for row in rows if row['event'] == 'fault_end')['code'] = lx2.KERNEL_COST

        run = self.record(change=change)['result']['run']
        limit = {fault: run['faults'][fault]['windows']['limit'] for fault in lx2.METERED}
        self.assertEqual((limit['alloc_metered']['effective'], limit['alloc_metered']['ends']),
                         (4, {'KN7:budget.kernel_cost': 1, 'KN7:budget.memory': 3}))
        for fault in ('recursion_metered', 'loop_metered'):
            self.assertEqual((limit[fault]['effective'], limit[fault]['ineffective']), (3, {'limit-not-reached': 1}),
                             fault)
        self.assertEqual(lx2.LIMIT_ENDS['alloc_metered'],
                         ('KN7:budget.memory', 'KN7:resource_limit.safety_stop', 'KN7:budget.kernel_cost'))

    def test_only_the_owner_accepted_echo_count_of_one_fault_of_one_run_is_enough(self):
        def change(fault, kind, index, rows):
            if fault == 'alloc_ceiling' and index == 0:
                rows[0]['armed'] = None

        # alloc_ceiling keeps 3 effective windows, so 3 echo samples, below OBSERVATIONS_MIN (4 here).
        run = self.record(change=change)['result']['run']
        echo = run['faults']['alloc_ceiling']['echo']
        self.assertEqual((echo['timed'], echo['samples'], echo['state']), (3, 'insufficient', 'insufficient'))
        for accepted, state in ((3, 'met'), (2, 'insufficient')):
            with mock.patch.dict(lx2.ACCEPTED_ECHO_SAMPLES, {('lx2-run', 'alloc_ceiling'): accepted}):
                run = self.record(change=change)['result']['run']
            echo = run['faults']['alloc_ceiling']['echo']
            self.assertEqual((echo['samples'], echo['state']),
                             ('accepted-by-owner' if state == 'met' else 'insufficient', state), accepted)
        # Accepted for that fault only: another fault with the same count stays insufficient.
        with mock.patch.dict(lx2.ACCEPTED_ECHO_SAMPLES, {('lx2-run', 'forced_kill'): 3}):
            run = self.record(change=change)['result']['run']
        self.assertEqual(run['faults']['alloc_ceiling']['echo']['samples'], 'insufficient')
        self.assertEqual(lx2.ACCEPTED_ECHO_SAMPLES, {('20261121T131440Z', 'alloc_ceiling'): 84})

    def test_an_unconfirmed_echo_sample_is_counted_apart_and_the_bar_judges_the_confirmed_ones(self):
        # Owner decision, 22 November 2026, item 10: the confirmation gap is a known limit, not a failed sample; any
        # other in-window failure still fails the bar, and the minimum counts confirmed samples.
        def record(name, unconfirmed=0, wrong=0):
            directory = lx2_run(self.temp / name, self.edits, self.digests, windows=5)
            path = directory / key_to_frame.EVENTS
            rows = [json.loads(line) for line in path.read_text(encoding='utf-8').splitlines() if line.strip()]
            echoes = [row for row in rows if row['t'] == 'lx4'][:unconfirmed + wrong]
            gone = {row['frame_id'] for row in echoes[:unconfirmed]}
            for row in echoes[unconfirmed:]:
                row['buffer_sha256'] = '0' * 64
            write_jsonl(path, [row for row in rows if not (row['t'] == 'presented' and row['frame_id'] in gone)])
            return lx2.build(self.temp / 'fixtures', directory, lx2_faults(self.temp / f'{name}.json'), ['analyze'])

        result = record('run-gap', unconfirmed=1)
        run = result['result']['run']
        self.assertEqual(run['status'], 'valid', run['reasons'])
        echo = run['faults']['alloc_metered']['echo']
        self.assertEqual({key: echo[key] for key in ('samples_in_windows', 'timed', 'unconfirmed', 'failed', 'samples',
                                                     'state')},
                         {'samples_in_windows': 5, 'timed': 4, 'unconfirmed': 1, 'failed': [], 'samples': 'sufficient',
                          'state': 'met'})
        self.assertNotIn('failed_counted', echo)
        self.assertEqual(lx2.judge(run), ('met', []))
        gaps = {entry['fault']: entry for entry in result['result']['known_limits']
                if entry['condition'] == 'confirmation-gap'}
        self.assertEqual(list(gaps), list(lx2.FAULTS))
        self.assertEqual({key: gaps['alloc_metered'][key] for key in ('samples', 'unconfirmed', 'judged', 'source')},
                         {'samples': 5, 'unconfirmed': 1, 'judged': 4,
                          'source': 'owner decision 2026-11-22 (LX bars), item 10'})
        self.assertEqual(gaps['loop_metered']['unconfirmed'], 0)
        # Below the minimum on confirmed samples: insufficient, never met.
        echo = record('run-gap-2', unconfirmed=2)['result']['run']['faults']['alloc_metered']['echo']
        self.assertEqual((echo['timed'], echo['unconfirmed'], echo['state']), (3, 2, 'insufficient'))
        # Another failure in a window still fails the bar.
        run = record('run-wrong', wrong=1)['result']['run']
        echo = run['faults']['alloc_metered']['echo']
        self.assertEqual((echo['unconfirmed'], echo['failed'], echo['state']), (0, ['wrong-buffer'], 'not-met'))
        self.assertIn('alloc_metered:echo-not-met', lx2.judge(run)[1])
        self.assertIn('an in-window echo failure other than a missing confirmation fails the bar', lx2.DRAFTS)

    def test_conditions_1_and_2_come_from_the_faults_record_of_the_same_child(self):
        run = self.record(disagree=('loop_native',))['result']['run']
        self.assertEqual(run['conditions_1_2']['state'], 'not-met')
        self.assertIn('trial-disagrees:loop_native', run['conditions_1_2']['reasons'])
        self.assertEqual(lx2.judge(run)[0], 'not-met')
        run = self.record(sha='d' * 64)['result']['run']
        self.assertEqual(run['conditions_1_2'], {'state': 'insufficient', 'reasons': ['other-injection-child']})
        run = self.record(sessions=False)['result']['run']
        self.assertEqual(run['conditions_1_2']['state'], 'insufficient')
        self.assertIn('session-missing:control', run['conditions_1_2']['reasons'])
        self.assertEqual(lx2.judge_faults(None, None)['reasons'], ['faults-record-missing'])

    def test_a_module_error_or_another_run_invalidates_the_run(self):
        def change(fault, kind, index, rows):
            if fault == 'loop_native' and index == 3:
                rows.append({'event': 'error', 'qpc': rows[-1]['qpc'] + 1, 'reason': 'probe-attempts'})

        record = self.record(change=change)
        self.assertEqual((record['verdict'], record['result']['run']['reasons']), ('synthetic', ['module-error']))
        self.assertEqual(lx2.judge(record['result']['run']), ('invalid', ['module-error']))
        directory = lx2_run(self.temp / 'other', self.edits, self.digests)
        launch = json.loads((directory / lx2.LAUNCH).read_text())
        (directory / lx2.LAUNCH).write_text(json.dumps({**launch, 'build_identity': 'another-build'}))
        record = lx2.build(self.temp / 'fixtures', directory, lx2_faults(self.temp / 'f.json'), ['analyze'])
        self.assertEqual(record['result']['run']['reasons'], ['launch-does-not-match-shell'])

    def test_the_dry_run_prints_the_plan_and_posts_nothing(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = lx2.main(['post', '--fixtures', str(self.temp / 'fixtures'), '--dry-run'],
                            system_factory=forbidden)
        self.assertEqual(code, 0)
        text = out.getvalue()
        self.assertIn('nothing was posted', text)
        self.assertIn('limit windows count with: alloc_metered KN7:budget.memory or KN7:resource_limit.safety_stop or '
                      'KN7:budget.kernel_cost; recursion_metered KN7:budget.memory or KN7:resource_limit.safety_stop; ',
                      text)
        for _, fault, kind in lx2.PHASES:
            self.assertIn(f'  {fault} ({kind} windows): about ', text)
        plan = {(fault, kind): nominal for fault, kind, nominal, bound in lx2.estimate()}
        self.assertEqual(list(plan), [(fault, kind) for _, fault, kind in lx2.PHASES])
        self.assertEqual(plan['alloc_metered', 'turn'], round(lx2.TARGET * (lx2.LEAD_S + lx2.RECOVERY_S + lx2.GAP_S)))
        # A limit window runs to the host's view deadline; one holds TARGET echo samples at ECHO_PACE_S.
        self.assertEqual(plan['loop_metered', 'limit'], round(lx2.VIEW_DEADLINE_S + lx2.RECOVERY_S + lx2.GAP_S))
        self.assertLess(plan['loop_native', 'turn'], 70)

    def test_the_poster_runs_each_phase_to_its_targets_and_closes_the_shell(self):
        modules = [{'t': 'module', 'module': name, 'window': 66, 'run_id': 'live', 'build_identity': BUILD,
                    'puzzle': 'jumble-off', 'qpc_frequency': F} for name in ('lx4', 'lx2')]

        class Fake:
            """The shell with LX-2's and LX-4's modules: a phase opens a window; each Reset turns once in an open turn
            window (or closes a window that has turned and opens the next); in a limit window every second Reset closes
            it and opens the next, with no turn; phase 0 closes the window and ends the phase."""

            def __init__(self, run, error_at=None):
                self.ticks, self.fault, self.window, self.turned, self.error_at = 0, None, 0, False, error_at
                self.kind, self.resets = None, 0
                self.posted, self.live = [], run / lx4.LIVE
                write_jsonl(self.live, modules)

            def append(self, row):
                with self.live.open('a') as stream:
                    stream.write(json.dumps(row) + '\n')

            def register(self, name):
                return {lx4.RESET: 0xC001, lx4.SELECT: 0xC002, lx2.PHASE: 0xC003}[name]

            def qpc(self):
                self.ticks += 1
                return self.ticks

            def frequency(self):
                return F

            def row(self, event, **fields):
                self.append({'t': 'lx2', 'event': event, 'qpc': self.qpc(), 'window': self.window,
                             'fault': self.fault, 'kind': self.kind, **fields})

            def post(self, hwnd, message, wparam, lparam):
                self.posted.append((message, wparam))
                if message == 0xC003 and wparam:
                    limit = wparam > len(lx2.FAULTS)
                    self.fault = lx2.FAULTS[wparam - len(lx2.FAULTS) - 1 if limit else wparam - 1]
                    self.kind = 'limit' if limit else 'turn'
                    self.window, self.turned, self.resets = self.window + 1, False, 0
                    self.row('phase')
                    self.row('start')
                elif message == 0xC003:
                    self.row('recovered')
                    self.row('phase_end')
                    self.fault = self.kind = None
                elif message == 0xC001:
                    self.append({'t': 'lx4', 'sample': lparam, 'endpoint': lx4.READY})
                    self.selects = 0
                    if self.fault is not None:
                        self.resets += 1
                        if self.turned or (self.kind == 'limit' and self.resets > 2):
                            self.row('recovered')
                            self.window += 1
                            self.resets = 1
                            self.row('start')
                        if self.kind == 'turn':
                            self.row('turn', ok=True)
                            self.turned = True
                        if self.error_at == self.window:
                            self.row('error', reason='probe-attempts')
                    self.sample = lparam
                elif message == 0xC002:
                    self.selects = getattr(self, 'selects', 0) + 1
                    self.append({'t': lx4.SELECTED, 'sample': self.sample, 'select': self.selects})
                    self.append({'t': 'lx4', 'sample': self.sample, 'endpoint': 'echo'})
                return hwnd == 66

            def foreground(self, hwnd):
                return True

            def alive(self, hwnd):
                return True

        clock = iter(range(10**9))
        with mock.patch.object(lx2, 'TARGET', 3):
            run = self.temp / 'live'
            fake = Fake(run)
            code = lx2.post_run(self.temp / 'fixtures', run, fake, clock=lambda: next(clock) / 100,
                                sleep=lambda seconds: None, out=lambda line: None)
            self.assertEqual(code, 0)
            rows = evidence.read_jsonl(run / lx2.POSTS)
            self.assertEqual(rows[0]['format'], lx2.POSTS_FORMAT)
            summary = {(row['fault'], row['kind']): row for row in rows if row['t'] == 'fault'}
            self.assertEqual(list(summary), [(fault, kind) for _, fault, kind in lx2.PHASES])
            for (fault, kind), row in summary.items():
                self.assertFalse(row['failed_samples'], (fault, kind))
                if kind == 'limit':
                    self.assertEqual(row['turns'], 0)
                    self.assertGreaterEqual(row['echo_samples'], 3)
                else:
                    self.assertGreaterEqual(row['turns'], 3)
                    if fault not in lx2.METERED:
                        self.assertGreaterEqual(row['echo_samples'], 3)
            phases = [wparam for message, wparam in fake.posted if message == 0xC003]
            self.assertEqual(phases, [wparam for number, _, _ in lx2.PHASES for wparam in (number, 0)])
            self.assertEqual(phases[:4], [8, 0, 1, 0])
            # LX-2's echo samples never ask for LX-4's re-run, whose view job would supersede LX-2's.
            resets = [wparam for message, wparam in fake.posted if message == 0xC001]
            self.assertTrue(resets and all(wparam < lx4.RERUN_FLAG for wparam in resets))
            self.assertEqual(fake.posted[-1], (lx2.WM_CLOSE, 0))
            self.assertEqual(rows[-1]['t'], 'close')
            # The module stops with an error: phase 0, and no close.
            run = self.temp / 'error'
            fake = Fake(run, error_at=2)
            code = lx2.post_run(self.temp / 'fixtures', run, fake, clock=lambda: next(clock) / 100,
                                sleep=lambda seconds: None, out=lambda line: None)
            self.assertEqual(code, 4)
            self.assertEqual(fake.posted[-1], (0xC003, 0))
            self.assertNotIn('close', [row['t'] for row in evidence.read_jsonl(run / lx2.POSTS)])
            # A shell started without --lx2-injection: no lx2 module row, nothing posted.
            only_lx4 = self.temp / 'only-lx4'
            write_jsonl(only_lx4 / lx4.LIVE, modules[:1])
            code = lx2.post_run(self.temp / 'fixtures', only_lx4, forbidden_system(), clock=lambda: next(clock),
                                sleep=lambda seconds: None, out=lambda line: None)
            self.assertEqual(code, 2)


def forbidden_system():
    class System:
        def __getattr__(self, name):
            raise AssertionError(f'the poster called {name} without a module row')
    return System()


class FakeDesktop:
    """foreground.py's system: scripted per check (index = how often last_input was read)."""

    def __init__(self, *, inputs=(), window_at=1, foreground=None, taken=True, closed_at=None, pid=77, tick=1000):
        self.inputs, self.window_at, self.closed_at, self.pid, self.tick = dict(inputs), window_at, closed_at, pid, tick
        self.foreground, self.taken, self.check, self.requests, self.raised = foreground or {}, taken, -1, [], []

    def last_input(self):
        self.check += 1
        return self.inputs.get(self.check, self.tick)

    def find_window(self, exe):
        return (0x5150, self.pid) if exe == 'Godot.exe' and self.check >= self.window_at else None

    def topmost(self, window):
        self.raised.append(window)

    def is_topmost(self, window):
        return window in self.raised

    def set_foreground(self, window):
        self.requests.append(window)
        return self.taken

    def foreground_pid(self):
        if self.check in self.foreground:
            return self.foreground[self.check]
        return self.pid if self.taken else 4

    def visible(self, window):
        return self.closed_at is None or self.check < self.closed_at


class ForegroundTests(NoProcessCase):
    def watch(self, desktop, hold=True, wait_s=30):
        lines, clock = [], iter(range(10_000))
        code = foreground.watch(desktop, 'Godot.exe', hold, wait_s, clock=lambda: next(clock), sleep=lambda s: None,
                                out=lines.append)
        return code, [json.loads(line) for line in lines]

    def test_it_asks_once_for_the_foreground_and_ends_when_the_window_closes(self):
        desktop = FakeDesktop(window_at=3, closed_at=40)
        code, rows = self.watch(desktop)
        self.assertEqual(code, 0)
        self.assertEqual((desktop.requests, desktop.raised), ([0x5150], [0x5150]))
        self.assertEqual([row['t'] for row in rows], ['start', 'take', 'end'])
        self.assertEqual((rows[1]['foreground'], rows[1]['topmost'], rows[-1]['reason']), (True, True, 'window-closed'))

    def test_owner_input_ends_the_watch_and_posted_messages_do_not_count(self):
        code, rows = self.watch(FakeDesktop(inputs={20: 1001}, closed_at=40))
        self.assertEqual((code, rows[-1]['reason']), (foreground.OWNER_INPUT, 'owner-input'))
        # Without --hold only input counts: another process in the foreground is no abort.
        code, rows = self.watch(FakeDesktop(foreground={n: 4 for n in range(100)}, closed_at=40), hold=False)
        self.assertEqual((code, [row['t'] for row in rows]), (0, ['start', 'window', 'end']))

    def test_the_foreground_counts_as_lost_at_two_checks_in_a_row(self):
        code, _ = self.watch(FakeDesktop(foreground={10: 4, 12: None}, closed_at=40))
        self.assertEqual(code, 0)
        code, rows = self.watch(FakeDesktop(foreground={10: 4, 11: 4}, closed_at=40))
        self.assertEqual((code, rows[-1]['reason'], rows[-1]['holder']), (foreground.FOREGROUND, 'foreground-lost', 4))

    def test_a_refused_request_and_a_missing_window(self):
        desktop = FakeDesktop(taken=False)
        code, rows = self.watch(desktop)
        self.assertEqual((code, rows[-1]['reason'], desktop.requests), (foreground.FOREGROUND, 'not-taken', [0x5150]))
        code, rows = self.watch(FakeDesktop(window_at=10_000), wait_s=30)
        self.assertEqual((code, rows[-1]['reason']), (foreground.NO_WINDOW, 'no-window'))

    def test_the_input_tick_wraps(self):
        self.assertTrue(foreground.newer(5, 2**32 - 5))
        self.assertFalse(foreground.newer(2**32 - 5, 5))
        self.assertFalse(foreground.newer(1000, 1000))

    def test_one_request_and_no_foreground_lock_workaround(self):
        source = (ROOT / 'tools/perf/c9/foreground.py').read_text(encoding='utf-8')
        # The code without the module docstring (which names what is avoided) and comments.
        code = '\n'.join(line for line in source.split('\"\"\"', 2)[2].splitlines() if not line.lstrip().startswith('#'))
        body = code.split('def watch(', 1)[1].split('\nclass Windows', 1)[0]
        self.assertEqual(body.count('set_foreground('), 1)
        self.assertEqual(code.count('.SetForegroundWindow('), 1)
        for word in ('AttachThreadInput', 'SystemParametersInfo', 'keybd_event', 'SendInput', 'mouse_event',
                     'AllowSetForegroundWindow', 'LockSetForegroundWindow', 'BringWindowToTop'):
            self.assertNotIn(word, code)


class RunnerTests(unittest.TestCase):
    """tools/perf/c9/unattended.ps1, read as text (the tests start no PowerShell)."""

    @classmethod
    def setUpClass(cls):
        cls.data = (ROOT / 'tools/perf/c9/unattended.ps1').read_bytes()
        cls.text = cls.data.decode('ascii')

    def function(self, name):
        return self.text.split(f'function {name} ', 1)[1].split('\nfunction ', 1)[0]

    def test_it_is_ascii_with_lf_endings_and_names_no_build_stamp(self):
        self.assertNotIn(b'\r', self.data)
        self.assertIsNone(re.search(r'\d{8}T\d{6}Z', self.text))
        for name in ('SceneBuild', 'StageBuild', 'ProductBuild'):
            self.assertIn(f"[string]${name} = ''", self.text)
            self.assertIn(f'-{name} is needed for', self.function('Test-Inputs').replace('-$key', f'-{name}'))
        run = self.function('Invoke-ShellRun')
        self.assertIn('$product = if ($Build) { $Build } else { $ProductBuild }', run)
        self.assertIn("'--build', $product", run)
        self.assertNotIn("'--build', $ProductBuild", run)

    def test_the_suffix_and_the_retries(self):
        self.assertIn("[ValidatePattern('^[A-Za-z0-9][A-Za-z0-9_-]*$')]\n    [string]$Suffix", self.text)
        self.assertIn('[ValidateRange(0, 2)]\n    [int]$Retries = 2', self.text)
        self.assertIn('for ($attempt = 0; $attempt -le $Retries; $attempt++)', self.function('Invoke-ShellStep'))
        for output in ('"$Runs\\u3$Tag"', '"$Acceptance\\w3-summary$Tag.json"', '"$Runs\\u8-off$Tag"',
                       '"$Runs\\e3$Tag-$i"', '"$Runs\\u7$Tag-faults.json"'):
            self.assertIn(output, self.text)

    def test_the_watcher_capture_and_poster_start_before_the_shell(self):
        run = self.function('Invoke-ShellRun').split('Wait-Gate $Step\n    Write-Host', 1)[1]
        order = [run.index(label) for label in ('"$label-watch"', '"$label-presentmon"', '"$label-poster"',
                                                '"$label-launch"')]
        self.assertEqual(order, sorted(order))
        for step in ('U3', 'U7', 'U8'):
            self.assertIn('-Hold', self.function(f'Invoke-{step}'))
        self.assertNotIn('-Hold', self.function('Invoke-U6'))

    def test_an_abort_sets_the_run_aside_and_stops_its_poster(self):
        run = self.function('Invoke-ShellRun')
        self.assertIn("$WatchAbort = @{ 10 = 'owner-input'; 11 = 'foreground-lost' }", self.text)
        self.assertEqual((foreground.OWNER_INPUT, foreground.FOREGROUND), (10, 11))
        for needle in ('$measuring = -not ($posterProcess -and $posterProcess.HasExited)',
                       'Write-Log $Step \'aborted\' $run', '"$run.aborted-$n"',
                       '"$Runs\\$Name.aborted-$n.presentmon.csv"', 'if (-not $watch.HasExited) { Stop-Tree $watch.Id }'):
            self.assertIn(needle, run)

    def test_it_changes_no_system_setting_and_takes_no_foreground_itself(self):
        # The code without the header (which says what the watcher does) and comments.
        code = '\n'.join(line for line in self.text.split('#>', 1)[1].splitlines()
                         if not line.lstrip().startswith('#'))
        for word in ('SetForegroundWindow', 'AttachThreadInput', 'Set-ExecutionPolicy', 'ExecutionPolicy',
                     'HKLM:', 'HKCU:', 'Set-ItemProperty', 'WerAddExcludedApplication', 'ForegroundLockTimeout'):
            self.assertNotIn(word, code)

    def test_u7_alone_keeps_error_reporting_out_and_restores_the_mode(self):
        # The error mode is this process's own, inherited by the processes it starts; no WER or registry setting, and
        # no Windows call in the device child (CT-RPI-20 keeps those in c600-platform).
        code = '\n'.join(line for line in self.text.split('#>', 1)[1].splitlines()
                         if not line.lstrip().startswith('#'))
        u7 = self.function('Invoke-U7')
        self.assertEqual(code.count('::SetErrorMode('), 2)
        self.assertEqual(u7.count('::SetErrorMode('), 2)
        quiet, restore = 'SetErrorMode([uint32]($previous -bor 3))', 'SetErrorMode([uint32]$previous)'
        before, after = u7.split('} finally {', 1)
        self.assertLess(before.index(quiet), before.index('try {'))
        self.assertLess(before.index('try {'), before.index("Invoke-Step 'U7/faults'"))
        self.assertLess(before.index("Invoke-Step 'U7/faults'"), before.index("Invoke-ShellStep -Step 'U7'"))
        self.assertIn(restore, after.split("'U7/analyze'", 1)[0])
        self.assertIn('if ($DryRun) {', before.split(quiet, 1)[0])
        for word in ('WerSetFlags', 'WerAddExcludedApplication', 'LocalDumps', 'DontShowUI', 'reg.exe', 'reg add',
                     'New-ItemProperty'):
            self.assertNotIn(word, code)
        child = (ROOT / 'rust/c600-lx2/src/bin/c600-lx2-child.rs').read_text(encoding='utf-8')
        self.assertNotIn('SetErrorMode', child)

    def test_u4_runs_a_scene_again_until_three_runs_exit_0(self):
        self.assertIn('[ValidateRange(0, 3)]\n    [int]$SceneRetries = 2', self.text)
        u4 = self.function('Invoke-U4')
        self.assertIn('while ($passed -lt $count -and $i -lt $count + $SceneRetries)', u4)
        self.assertIn('if ("$($script:SceneCode)" -eq \'0\') { $passed++ }', u4)
        # Every run made goes to the gate, a refused one too (renderer_gate.py lists it as invalid).
        self.assertIn('if ($run) { $made += $run }', u4)
        self.assertIn('Invoke-Gate "U4/$scene/gate" $made', u4)
        scene = self.text.split('function Invoke-Scene(', 1)[1].split('\nfunction ', 1)[0]
        self.assertIn('$script:SceneCode = $code', scene)
        self.assertIn('$script:SceneCode = 0', scene.split('if ($DryRun) {', 1)[1].split('\n    }\n', 1)[0])

    def test_gap_is_named_only_and_compares_builds_with_their_own_checkouts(self):
        self.assertIn("[ValidateSet('U1', 'U2', 'U3', 'U4', 'U5', 'U6', 'U7', 'U8', 'GAP')]", self.text)
        # The default steps stay the runbook's; GAP runs only when named, after U8.
        self.assertIn("[string[]]$Steps = @('U1', 'U2', 'U3', 'U4', 'U5', 'U6', 'U7', 'U8'),", self.text)
        self.assertIn("$order = @('U1', 'U2', 'U3', 'U4', 'U5', 'U6', 'U7', 'U8', 'GAP')", self.text)
        plan = self.function('Get-GapPlan')
        self.assertIn('$from = Get-Checkout $build', plan)
        self.assertIn("Poster = (Join-Path $from 'tools\\perf\\c9\\lx4.py')", plan)
        self.assertIn('Fixtures = (Join-Path $from $Fixtures)', plan)
        self.assertLess(plan.index('foreach ($build in $CompareBuilds)'), plan.index('Window = @()'))
        self.assertLess(plan.index('Window = @()'), plan.index("Window = @('--resolution', $GapResolution)"))
        self.assertIn("$GapResolution = '2400x1400'", self.text)
        gap = self.function('Invoke-GAP')
        self.assertIn('-Build $entry.Build -From $entry.From', gap)
        self.assertIn('-CloseAfter 600', gap)
        self.assertIn('-Hold', gap)
        self.assertIn("'--puzzle', 'jumble-off'", gap)
        self.assertIn("$Share = 'work/experiments/owner-sitting-20261125/presented_share.py'", self.text)
        self.assertTrue((ROOT / 'work/experiments/owner-sitting-20261125/presented_share.py').is_file())
        self.assertIn('"$Runs\\gap$Tag-share.json"', gap)
        run = self.function('Invoke-ShellRun')
        self.assertIn("$launcher = if ($From) { Join-Path $From 'renderer\\shell\\launch.py' } else { 'renderer/shell/launch.py' }", run)
        self.assertIn("'-B', '-u', $launcher, '--build', $product", run)

    def test_gap_builds_skip_the_head_check_but_not_their_checkouts(self):
        inputs = self.function('Test-Inputs')
        self.assertIn("ProductBuild = @('U3', 'U6', 'U7', 'U8', 'GAP')", inputs)
        gap = inputs.split("if ($Steps -contains 'GAP') {", 1)[1].split("if ($Steps -contains 'U4'", 1)[0]
        self.assertIn('[IO.Path]::IsPathRooted($build)', gap)
        self.assertIn('$record.dirty -ne $false', gap)
        self.assertIn('& git -C $from diff --quiet $record.head HEAD -- renderer/shell', gap)
        self.assertNotIn('$script:HeadFull', gap)
        self.assertIn('"$Runs\\gap$Tag-$_"; "$Runs\\gap$Tag-$_.presentmon.csv"', inputs)
        # The log names the compared builds by identity and commit, never by their full path.
        self.assertIn("$compared = \"; compared builds:", self.text)
        self.assertNotIn('$CompareBuilds -join', self.text)

    def test_u8_records_the_gate_summary_as_a_reference(self):
        u8 = self.function('Invoke-U8')
        self.assertIn("'--gate-reference'", u8)
        self.assertNotIn("'--gate'", u8)

    def test_u6_launches_once_untimed_before_its_three_runs(self):
        u6 = self.function('Invoke-U6')
        warmup = u6.index("Invoke-ShellStep -Step 'U6.warmup' -Name \"e3$Tag-warmup\" -LaunchArgs @('--cold-start')")
        self.assertLess(warmup, u6.index('foreach ($i in 1..3)'))
        self.assertIn("\"$Runs\\e3$Tag-warmup\", '--out', \"$Runs\\e3$Tag-warmup.json\"", u6)
        # The timed record reads the three runs only.
        timed = u6.split("Invoke-Step 'U6/cold_start'", 1)[1]
        self.assertNotIn('warmup', timed)
        self.assertIn('"$Runs\\e3$Tag-warmup", "$Runs\\e3$Tag-warmup.json"', self.function('Test-Inputs'))

    def test_every_build_must_come_from_this_checkouts_head(self):
        inputs = self.function('Test-Inputs')
        self.assertIn('$script:HeadFull = (& git -C $Root rev-parse HEAD 2>$null)', self.text)
        self.assertIn("$file = if ($key -eq 'ProductBuild') { 'product.json' } else { 'c9-build-identity.json' }",
                      inputs)
        self.assertIn('elseif ($built -ne $script:HeadFull) {', inputs)
        head = self.text.split('function Get-BuildHead(', 1)[1].split('\nfunction ', 1)[0]
        self.assertIn("if ($record.dirty -ne $false) { return 'a changed tree' }", head)
        self.assertIn('return [string]$record.source.head', head)


if __name__ == '__main__':
    unittest.main(verbosity=2)
