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
import key_to_frame  # noqa: E402
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
        self.assertEqual(run['presentmon'], {'frames_checked': 3, 'disagreements': 0, 'tolerance_ms': 2.0,
                                             'presentmon_displays': 4})

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

    def test_an_unconfirmed_key_gets_no_time_and_invalidates_its_run(self):
        run = key_to_frame.analyze(e2_rows([20.0, 30.0, 40.0, 50.0], unconfirmed={1}, no_frame={3}))
        self.assertEqual(run['status'], 'invalid')
        self.assertIn('unconfirmed-keys', run['reasons'])
        self.assertEqual([entry['status'] for entry in run['per_key']],
                         ['confirmed', 'unconfirmed', 'confirmed', 'no-frame'])
        self.assertIsNone(run['per_key'][1]['latency_ms'])
        self.assertEqual((run['confirmed'], run['unconfirmed'], run['no_frame']), (2, 2, 1))
        self.assertEqual(run['unconfirmed_keys'], [1, 3])
        self.assertEqual(len(run['latencies_ms']), 2)  # never filled in from a later frame

    def test_unconfirmed_keys_are_reported_loudly(self):
        write_jsonl(self.temp / 'run' / 'events.jsonl', e2_rows([20.0, 30.0], unconfirmed={0}))
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = key_to_frame.main([str(self.temp / 'run')])
        self.assertIn('UNCONFIRMED', err.getvalue())
        self.assertIn('seq 0', err.getvalue())
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
        invalid = key_to_frame.analyze(e2_rows([500.0] * 100, unconfirmed={0}))
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
        run = key_to_frame.analyze(e2_rows([20.0]), display_ticks=[F + 25 * F // 1000])
        self.assertEqual(run['presentmon']['disagreements'], 1)
        self.assertIn('presentmon-disagrees', run['reasons'])
        agree = key_to_frame.analyze(e2_rows([20.0]), display_ticks=[F + 21 * F // 1000])
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
            'a timed capture of 1 to 3600 s is needed': [a for a in good if a not in ('--timed', '10')],
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
        lx2 = next(step for step in runbook.STEPS if step.id == 'lx2')
        self.assertTrue(any('--injection' in command for command in lx2.commands))
        gpu = next(step for step in runbook.STEPS if step.id == 'core-gpu')
        self.assertTrue(gpu.built and gpu.commands[0].startswith('python -B renderer/core/check_core.py --gpu'))
        self.assertEqual(next(step for step in runbook.STEPS if step.id == 'lx4').minutes, 240)
        for step in runbook.STEPS:
            with self.subTest(step=step.id):
                self.assertTrue(step.minutes > 0 and step.sources)
                self.assertTrue(step.built or step.missing)

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
        self.assertIn('not built yet', out.getvalue())
        self.assertIn('(U8)', out.getvalue())
        self.assertTrue(runbook.evaluate('mentions', ('tools/perf/c9', 'OTHER_ROWS'))[0])
        self.assertFalse(runbook.evaluate('mentions', ('tools/perf/c9', 'no' + ' such token'))[0])


def skew_rows(n, skew_ms, *, header=None, unknown=()):
    """n 3D frames 1/60 s apart, each displayed 20 ms after submission, with a UI frame 1 ms after each."""
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
        self.assertEqual(sent[0][0], lx4.RESET)
        self.assertEqual(sent[1][0], lx4.SELECT)
        self.assertEqual((sent[timed][0], timed), (lx4.WM_CHAR, 2))
        sent, timed = lx4.messages(self.edits, samples[('S', 'syntax')])
        self.assertEqual(sent[timed][:2], (lx4.WM_KEYDOWN, lx4.VK_BACK))
        sent, timed = lx4.messages(self.edits, samples[('M', 'completion')])
        self.assertIn((lx4.WM_CHAR, 13, 1), sent)
        self.assertEqual(timed, len(sent) - 1)
        sent, _ = lx4.messages(self.edits, samples[('L', 'diagnostics')])
        selects = [wparam for message, wparam, _ in sent if message == lx4.SELECT]
        self.assertEqual(selects, sorted(selects, reverse=True))
        turn = next(sample for sample in lx4.schedule(self.edits, 'jumble-off') if sample['metric'] == 'turn')
        with self.assertRaises(lx4.FixtureError):
            lx4.messages(self.edits, turn)

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
        self.assertEqual(lx4.judge(self.runs(late('L', 'echo', 60.0)), self.e2, self.gate, {'events'})[2],
                         'owner-l-or-w')
        self.assertEqual(lx4.judge(self.runs(late('W', 'dependents', 1600.0)), self.e2, self.gate, {'events'})[:3],
                         ('not-met', ['bar-missed'], 'owner-l-or-w'))
        self.assertEqual(lx4.judge(self.runs(late('S', 'syntax', 80.0)), self.e2, self.gate, {'events'})[2],
                         'fail-s-m')
        smoke = lambda sample: 900.0 if sample['series'] == 0 else 10.0  # noqa: E731
        self.assertEqual(lx4.judge(self.runs(smoke), self.e2, self.gate, {'events'})[0], 'met')

    def test_e2_and_the_gate_must_hold_in_the_same_runs(self):
        runs = self.runs(lambda sample: 10.0)
        self.assertEqual(lx4.judge(runs, None, self.gate, {'events'})[:3],
                         ('insufficient', ['e2-record-missing'], None))
        self.assertEqual(lx4.judge(runs, self.e2, self.gate, {'other runs'})[2], 'fail-gate-or-e2')
        failed = {**self.gate, 'scenes': [{'verdict': 'not-met', 'runs': [{'build_identity': BUILD}]}]}
        self.assertEqual(lx4.judge(runs, self.e2, failed, {'events'})[2], 'fail-gate-or-e2')

    def test_a_wrong_buffer_or_an_unconfirmed_sample_invalidates_its_run(self):
        runs = self.runs(lambda sample: 10.0, wrong={5})
        self.assertEqual(runs[0]['failures'], {'wrong-buffer': 1})
        self.assertEqual(lx4.judge(runs, self.e2, self.gate, {'events'})[:2],
                         ('invalid', ['one-valid-run-per-puzzle-needed']))
        events, posts = lx4_logs(self.edits, self.digests, 'jumble-off', lambda sample: 10.0)
        events = [row for row in events if not (row['t'] == 'presented' and row['frame_id'] == 3)]
        run = lx4.analyze_run(events, posts, self.edits, self.digests)
        self.assertEqual((run['status'], run['failures']), ('invalid', {'unconfirmed': 1}))

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

    def test_a_dry_run_posts_nothing(self):
        with contextlib.redirect_stdout(io.StringIO()) as out:
            code = lx4.main(['post', '--fixtures', str(self.temp / 'fixtures'), '--puzzle', 'jumble-on', '--dry-run'],
                            system_factory=forbidden)
        self.assertEqual(code, 0)
        self.assertIn("turn key is not known", out.getvalue())
        self.assertIn('nothing was posted', out.getvalue())

    def test_the_poster_waits_for_each_endpoint_and_logs_every_post(self):
        run = self.temp / 'run'
        write_jsonl(run / 'events.jsonl', [{'t': 'run', 'format': key_to_frame.LOG_FORMAT, 'run_id': 'live',
                                            'build_identity': BUILD, 'puzzle': 'jumble-off', 'window': 66}])
        events = run / 'events.jsonl'

        class Fake:
            def __init__(self, answer=True):
                self.answer, self.ticks = answer, 0

            def register(self, name):
                return {lx4.RESET: 0xC001, lx4.SELECT: 0xC002}[name]

            def qpc(self):
                self.ticks += 1
                return self.ticks

            def frequency(self):
                return F

            def post(self, hwnd, message, wparam, lparam):
                if self.answer and message == 0xC001:
                    with events.open('a') as stream:
                        for metric in (*lx4.METRICS, 'dependents'):
                            stream.write(json.dumps({'t': 'lx4', 'sample': lparam, 'endpoint': metric}) + '\n')
                if self.answer and message == lx4.WM_KEYDOWN and wparam == 0x54:
                    with events.open('a') as stream:
                        stream.write(json.dumps({'t': 'key', 'seq': 0, 'qpc': 1, 'frame_id': None}) + '\n')
                return hwnd == 66

            def foreground(self, hwnd):
                return True

            def alive(self, hwnd):
                return True

        clock = iter(range(10**9))
        with mock.patch.object(lx4, 'PER_SERIES', 2), mock.patch.object(lx4, 'SERIES', 1):
            samples = lx4.schedule(self.edits, 'jumble-off')
            code = lx4.post_run(self.temp / 'fixtures', 'jumble-off', run, 0x54, Fake(),
                                clock=lambda: next(clock) / 100, sleep=lambda seconds: None, out=lambda line: None)
            self.assertEqual(code, 0)
            rows = evidence.read_jsonl(run / lx4.POSTS)
            self.assertEqual(sum(row['t'] == 'sample' and row['status'] == 'done' for row in rows), len(samples))
            self.assertEqual(sum(row['t'] == 'post' and row['timed'] for row in rows), len(samples))
            (run / lx4.POSTS).unlink()
            code = lx4.post_run(self.temp / 'fixtures', 'jumble-off', run, 0x54, Fake(answer=False),
                                clock=lambda: next(clock) / 2, sleep=lambda seconds: None, out=lambda line: None)
            self.assertEqual(code, 3)


if __name__ == '__main__':
    unittest.main(verbosity=2)
