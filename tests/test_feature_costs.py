"""H-06 paired costs: synthetic files and frame-state simulations only."""
from __future__ import annotations

import contextlib
import io
import json
import math
import os
import random
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'tools/perf'))
import renderer_gate as gate  # noqa: E402
import feature_costs as costs  # noqa: E402

CAMERA = {'plane': [0, 3], 'step_rad': 0.002, 'per': 'frame'}
FREQ = 1_000_000
SIMULATION_RESULTS = []


def environment(msaa=1):
    return {'presentation_interval': 0, 'vsync': False, 'tearing': True, 'warp': False,
            'presenting_adapter': 'Fixture GPU; driver 1.2', 'power_source': 'mains',
            'power_mode': 'performance', 'msaa': msaa,
            'display': {'width': 2560, 'height': 1600, 'refresh_hz': 60},
            'backbuffer': {'width': 2560, 'height': 1600},
            'declared': {'frame_generation': False, 'upscaling': False,
                         'driver_vsync': False, 'vendor_mode': 'default'}}


def metadata(run_id, feature='none', scene='w3f', turn_frames=2, cycle_frames=20):
    return {'format': 'magic600-renderer-run-v1', 'run_id': run_id, 'candidate': 's-b',
            'feature': feature, 'scene': scene, 'build': {'build_identity': 'fixture-build'},
            'qpc_frequency': FREQ, 'turn_ms': 200, 'turn_frames': turn_frames,
            'cycle_frames': cycle_frames, 'camera': dict(CAMERA),
            'presentmon': {'process_id': 42, 'swap_chain': '0xA'},
            'markers': {'trace_start_qpc': 4 * FREQ, 'trace_stop_qpc': 4 * FREQ},
            'label_check': {'status': 'pass'}, 'environment': environment(4 if feature == 'msaa4' else 1),
            'window': {'visible_throughout': True, 'foreground_throughout': True}}


def simulate(run, cost, seed=None):
    """A four-second preroll advances no state; entry k starts frame k's draw."""
    rng = random.Random(seed)
    start = run['markers']['trace_start_qpc']
    stop = start + (gate.WARMUP_S + gate.INTERVAL_S + 1) * FREQ
    entries, durations, elapsed = [], [], 0.0
    k, T, n = 0, run['turn_frames'], run['cycle_frames']
    while start + round(elapsed * FREQ / 1000) <= stop:
        qpc = start + round(elapsed * FREQ / 1000)
        phase, angle, turn = (k % T) / T, ((k % n) + 1) * 0.002, k // T
        entries.append({'frame': k, 'qpc': qpc, 'camera': (k % n) + 1,
                        'turn': turn, 'revision': turn, 'phase': phase})
        duration = cost(angle, phase, turn) * (rng.uniform(0.98, 1.02) if seed is not None else 1)
        durations.append(duration)
        elapsed += duration
        k += 1
    run['markers']['trace_stop_qpc'] = stop
    return entries, durations


def measured(run, entries, durations, timing, lag=0):
    begin, end = gate.interval_ticks(run)
    if timing == 'trace':
        kept = [i for i in range(1, len(entries)) if begin <= entries[i]['qpc'] < end]
        return ([(entries[i]['qpc'] - entries[i - 1]['qpc']) * 1000 / FREQ for i in kept],
                [entries[i]['qpc'] for i in kept])
    kept = [i for i in range(lag, len(entries) - 1) if begin <= entries[i + 1]['qpc'] - 1 < end]
    return [durations[i - lag] for i in kept], [entries[i + 1]['qpc'] - 1 for i in kept]


class CostTests(unittest.TestCase):
    def setUp(self):
        if os.name == 'nt':
            mkdir = os.mkdir
            with mock.patch.object(os, 'mkdir', side_effect=lambda path, mode: mkdir(path)):
                temporary = tempfile.TemporaryDirectory(prefix='.feature-test-', dir=ROOT)
        else:
            temporary = tempfile.TemporaryDirectory(prefix='.feature-test-', dir=ROOT)
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.sequence = 0
        patcher = mock.patch.multiple(gate, WARMUP_S=1, INTERVAL_S=2)
        patcher.start()
        self.addCleanup(patcher.stop)

    def write_run(self, run, entries, durations):
        directory = self.base / run['run_id']
        directory.mkdir()
        (directory / 'run.json').write_text(json.dumps(run), encoding='utf-8')
        (directory / 'trace.jsonl').write_text(''.join(json.dumps(e) + '\n' for e in entries), encoding='utf-8')
        (directory / 'presentmon.csv').write_text('ProcessID,SwapChainAddress,QPCTime,msBetweenPresents\n'
            + ''.join(f"42,0xA,{entries[i + 1]['qpc'] - 1},{durations[i]}\n" for i in range(len(entries) - 1)),
            encoding='utf-8')
        return directory

    def run_dir(self, feature='none', ms=25, scene='w3f', edit=None, trace=None):
        self.sequence += 1
        run = metadata(f'r{self.sequence}', feature, scene)
        if edit:
            edit(run)
        entries, durations = simulate(run, lambda *state: ms)
        if scene == 'w3':
            for entry in entries:
                elapsed_ms = (entry['qpc'] - run['markers']['trace_start_qpc']) * 1000 / FREQ
                turn = int(elapsed_ms // run['turn_ms'])
                entry.update(turn=turn, revision=turn, phase=(elapsed_ms % run['turn_ms']) / run['turn_ms'])
        if trace:
            entries = [trace(entry) for entry in entries]
        return self.write_run(run, entries, durations)

    def group(self, feature='none', count=3, **options):
        return [self.run_dir(feature=feature, **options) for _ in range(count)]

    def feature(self, result, name='fog'):
        return next(row for row in result['features'] if row['feature'] == name)

    def test_h0601_gaps_cost_and_gate_use_different_variants(self):
        for baseline_ms, variant_ms, cost, verdict, variant_verdict in (
                (31, 34, -3, 'met', 'not-met'), (34, 31, 3, 'not-met', 'met')):
            with self.subTest(baseline_ms=baseline_ms):
                dirs = self.group(ms=baseline_ms) + self.group('no-gaps', ms=variant_ms)
                dirs += self.group(ms=baseline_ms, scene='w3') + self.group('no-gaps', ms=variant_ms, scene='w3')
                result = costs.summarize(dirs)
                row = self.feature(result, 'gaps')
                self.assertEqual(row['variant'], 'no-gaps')
                self.assertAlmostEqual(row['cost_mean_ms'], cost)
                self.assertAlmostEqual(row['cost_p99_ms'], cost)
                self.assertEqual((row['verdict'], row['variant_verdict']), (verdict, variant_verdict))
                self.assertEqual(row['breaks_gate'], verdict == 'not-met')
                self.assertAlmostEqual(row['mean_frame_ms'], variant_ms)
                self.assertEqual(row['gate_runs'], 3)
                self.assertEqual(row['status'], 'measured')
                self.assertIsNone(result['baseline']['cost_mean_ms'])

    def test_h0602_known_controls_must_match(self):
        baseline = self.group()

        def resolution(run):
            for key in ('display', 'backbuffer'):
                run['environment'][key].update(width=1920, height=1200)

        cases = [(lambda r: r['environment'].update(presentation_interval=1), ['presentation_interval']),
                 (resolution, ['display.width', 'display.height', 'backbuffer.width', 'backbuffer.height']),
                 (lambda r: r['environment'].update(msaa=4), ['msaa']),
                 (lambda r: r.update(cycle_frames=40), ['cycle_frames'])]
        for edit, names in cases:
            with self.subTest(names=names):
                row = self.feature(costs.summarize(baseline + self.group('fog', edit=edit)))
                self.assertEqual(row['status'], 'not-comparable')
                self.assertIn('controls-differ', row['reasons'])
                self.assertEqual(set(row['differing_controls']), set(names))
                self.assertIsNone(row['cost_mean_ms'])
                self.assertIsNotNone(row['paired_mean_ms'])
        own = self.group('fog', count=2) + self.group('fog', count=1,
                  edit=lambda r: r['environment'].update(power_mode='balanced'))
        row = self.feature(costs.summarize(baseline + own))
        self.assertEqual(row['differing_controls'], ['power_mode'])
        self.assertEqual(row['status'], 'not-comparable')
        row = self.feature(costs.summarize(baseline + self.group('msaa4')), 'msaa4')
        self.assertEqual((row['status'], row['cost_mean_ms']), ('measured', 0))

    def test_h0602_unknown_controls_never_match(self):
        cases = [(lambda r: r['environment']['declared'].pop('driver_vsync'), 'declared.driver_vsync'),
                 (lambda r: r['environment'].update(presentation_interval=None), 'presentation_interval'),
                 (lambda r: r['environment'].update(power_mode='changed'), 'power_mode'),
                 (lambda r: r['environment'].update(
                     presenting_adapter='Fixture GPU; driver UMD version unavailable (0x80004005)'), 'presenting_adapter')]
        for edit, name in cases:
            with self.subTest(name=name):
                result = costs.summarize(self.group(edit=edit) + self.group('fog', edit=edit))
                row = self.feature(result)
                self.assertEqual(row['status'], 'not-comparable')
                self.assertIn('controls-unknown', row['reasons'])
                self.assertEqual(row['unknown_controls'], [name])
                self.assertIsNone(row['cost_mean_ms'])
                self.assertIsNone(row['cost_p99_ms'])
                self.assertEqual(result['baseline']['status'], 'not-comparable')

    def test_all_features_measured_and_unmeasured_never_estimated(self):
        dirs = self.group() + [directory for variant in ('no-gaps', 'outlines', 'transparency', 'fog', 'dof', 'ao', 'msaa4')
                              for directory in self.group(variant, ms=30)]
        result = costs.summarize(dirs)
        self.assertEqual([r['feature'] for r in result['features']],
                         ['gaps', 'outlines', 'transparency', 'fog', 'dof', 'ao', 'msaa4'])
        self.assertTrue(all(r['status'] == 'measured' and r['runs'] == 3 for r in result['features']))
        self.assertEqual(self.feature(result, 'gaps')['cost_mean_ms'], -5)
        self.assertEqual(self.feature(result)['cost_mean_ms'], 5)
        empty = self.feature(costs.summarize(dirs[:3]))
        self.assertEqual((empty['status'], empty['runs']), ('unmeasured', 0))
        for key in ('paired_mean_ms', 'paired_p99_ms', 'cost_mean_ms', 'cost_p99_ms'):
            self.assertIsNone(empty[key])
        self.assertEqual((empty['verdict'], empty['variant_verdict'], empty['breaks_gate']), ('no-data', 'no-data', None))

    def test_builds_and_gate_build_alignment(self):
        baseline = self.group()
        other = lambda r: r['build'].update(build_identity='other-build')
        mixed = self.group('fog', count=2) + self.group('fog', count=1, edit=other)
        row = self.feature(costs.summarize(baseline + mixed))
        self.assertEqual(row['status'], 'not-comparable')
        self.assertIn('mixed-builds', row['reasons'])
        self.assertIsNone(row['build_identity'])
        self.assertIsNone(row['cost_mean_ms'])
        row = self.feature(costs.summarize(baseline + self.group('fog', edit=other)))
        self.assertEqual(row['status'], 'not-comparable')
        self.assertIn('build_identity', row['differing_controls'])
        row = self.feature(costs.summarize(baseline + self.group('fog') + self.group('fog', scene='w3', edit=other)))
        self.assertIn('gate-other-build', row['reasons'])
        for key in ('gate_runs', 'fps', 'mean_frame_ms', 'p99_ms', 'variant_verdict', 'verdict', 'breaks_gate'):
            self.assertIsNone(row[key])
        # Gate fields for an unmeasured variant use the baseline cost build.
        row = self.feature(costs.summarize(baseline + self.group('fog', scene='w3')))
        self.assertEqual((row['status'], row['verdict']), ('unmeasured', 'met'))

    def test_missing_baseline_and_fewer_runs(self):
        row = self.feature(costs.summarize(self.group('fog')))
        self.assertEqual(row['status'], 'not-comparable')
        self.assertIsNone(row['cost_mean_ms'])
        for left, right in ((2, 3), (3, 2)):
            result = costs.summarize(self.group(count=left) + self.group('fog', count=right))
            self.assertEqual(self.feature(result)['status'], 'preliminary')
        result = costs.summarize(self.group(count=2))
        self.assertEqual(result['baseline']['status'], 'preliminary')

    def test_baseline_control_drift_and_partial_cost_failures(self):
        baseline = self.group(count=2) + self.group(count=1, edit=lambda r: r['environment'].update(power_mode='balanced'))
        result = costs.summarize(baseline + self.group('fog'))
        for row in (result['baseline'], self.feature(result)):
            self.assertEqual(row['status'], 'not-comparable')
            self.assertEqual(row['differing_controls'], ['power_mode'])
        broken = lambda e: {**e, 'revision': e['revision'] + 1}
        result = costs.summarize(self.group() + self.group('fog', count=2) + self.group('fog', count=1, trace=broken))
        row = self.feature(result)
        self.assertEqual((row['status'], row['runs']), ('preliminary', 2))
        self.assertAlmostEqual(row['cost_mean_ms'], 0)
        self.assertEqual(len(result['cost_invalid_runs']), 1)

    def test_additional_optional_control_types_are_known_before_matching(self):
        baseline = self.group()
        cases = [(lambda r: r['environment'].update(msaa=True), 'msaa'),
                 (lambda r: r['environment'].update(vsync=0), 'vsync'),
                 (lambda r: r['environment'].update(tearing=None), 'tearing'),
                 (lambda r: r['environment'].pop('warp'), 'warp'),
                 (lambda r: r['environment']['display'].update(refresh_hz=60.0), 'display.refresh_hz'),
                 (lambda r: r['environment']['declared'].update(vendor_mode=''), 'declared.vendor_mode'),
                 (lambda r: r['environment'].update(power_mode='unknown'), 'power_mode')]
        for edit, name in cases:
            with self.subTest(name=name):
                row = self.feature(costs.summarize(baseline + self.group('fog', edit=edit)))
                self.assertEqual((row['status'], row['unknown_controls']), ('not-comparable', [name]))
                self.assertIsNone(row['cost_mean_ms'])

    def test_trace_timing_keeps_label_and_window_rules(self):
        baseline = self.group()
        bad = self.group('fog', edit=lambda r: r['label_check'].update(status='fail'))
        hidden = self.group('ao', edit=lambda r: r['window'].update(visible_throughout=False))
        good = self.group('outlines')
        for directory in baseline + bad + hidden + good:
            (directory / 'presentmon.csv').unlink()
        result = costs.summarize(baseline + bad + hidden + good, timing='trace')
        self.assertEqual((result['timing_source'], result['preliminary']), ('probe-trace', True))
        self.assertEqual(self.feature(result, 'outlines')['status'], 'preliminary')
        self.assertEqual(self.feature(result)['status'], 'unmeasured')
        self.assertEqual(self.feature(result, 'ao')['status'], 'unmeasured')
        self.assertEqual({tuple(r['reasons']) for r in result['invalid_runs']},
                         {('label-check-failed',), ('window-not-visible',)})

    def test_cost_state_checks_and_whole_cycle_coverage(self):
        baseline = self.group()
        edits = [lambda e: {**e, 'camera': e['frame'] + 1},
                 lambda e: {**e, 'phase': ((e['qpc'] / FREQ) % 1)},
                 lambda e: {**e, 'frame': e['frame'] + 1},
                 lambda e: {**e, 'turn': e['turn'] + 1},
                 lambda e: {**e, 'revision': e['revision'] + 1}]
        for trace in edits:
            result = costs.summarize(baseline + self.group('fog', trace=trace))
            self.assertEqual(self.feature(result)['status'], 'not-comparable')
            self.assertEqual({tuple(r['reasons']) for r in result['cost_invalid_runs']}, {('state-mismatch',)})
        bad_camera = self.group('fog', edit=lambda r: r['camera'].update(per='clock'))
        self.assertIn('state-mismatch', self.feature(costs.summarize(baseline + bad_camera))['reasons'])
        slow = self.group('fog', ms=70, edit=lambda r: r.update(turn_frames=157, cycle_frames=3140))
        result = costs.summarize(baseline + slow + self.group('fog', ms=70, scene='w3'))
        row = self.feature(result)
        self.assertEqual(row['status'], 'not-comparable')
        self.assertIn('cycle-coverage', row['reasons'])
        self.assertIsNone(row['cost_mean_ms'])
        self.assertEqual(row['verdict'], 'not-met')
        self.assertEqual({tuple(r['reasons']) for r in result['cost_invalid_runs']}, {('cycle-coverage',)})

    def test_centered_window_requires_consecutive_nominal_entries(self):
        run = metadata('window', turn_frames=2, cycle_frames=4)
        entries = [{'frame': k, 'qpc': k * 100, 'camera': k % 4 + 1, 'turn': k // 2,
                    'phase': k % 2 / 2, 'revision': k // 2} for k in range(16)]
        values = list(range(11))
        selected, cycles, reasons = costs.cost_window(run, entries, values, [e['qpc'] for e in entries[2:13]], 'trace')
        self.assertEqual((selected, cycles, reasons), (values[1:9], 2, []))
        for ticks in ([e['qpc'] for e in entries[2:5]] + [e['qpc'] for e in entries[6:14]],
                      [entries[2]['qpc']] * 11):
            self.assertEqual(costs.cost_window(run, entries, values, ticks, 'presentmon')[2], ['state-mismatch'])

    def test_cli_outputs_sanitize_and_refuse_overwrites(self):
        dirs = self.group() + self.group('fog')
        out, markdown = self.base / 'costs.json', self.base / 'costs.md'
        args = [*map(str, dirs), '--out', str(out), '--markdown', str(markdown)]
        with mock.patch.object(gate, 'summarize_private', wraps=gate.summarize_private) as called:
            self.assertEqual(costs.main(args), 0)
        self.assertEqual(called.call_count, 1)
        result = json.loads(out.read_text(encoding='utf-8'))
        self.assertEqual(result['format'], 'magic600-h06-feature-costs-v1')
        text = markdown.read_text(encoding='utf-8')
        for name in ('none', 'gaps', 'outlines', 'transparency', 'fog', 'dof', 'ao', 'msaa4'):
            self.assertIn(name, text)
        self.assertIn('costs are paired: every variant draws the same W3 states (scene w3f), in whole cycles; gate fields come from W3 runs', text)
        self.assertIn('unmeasured features are not estimated; not-comparable features have no cost', text)
        before = out.read_bytes(), markdown.read_bytes()
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr), self.assertRaises(SystemExit):
            costs.main(args)
        self.assertIn('existing-output', stderr.getvalue())
        self.assertEqual((out.read_bytes(), markdown.read_bytes()), before)
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            costs.main([*map(str, dirs), '--out', str(self.base / 'new.json'), '--markdown', str(markdown)])
        self.assertFalse((self.base / 'new.json').exists())
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(costs.main([str(self.base / 'missing'), '--out', str(self.base / 'empty.json')]), 1)
        self.assertFalse((self.base / 'empty.json').exists())
        private = lambda r: r['environment'].update(presenting_adapter=r'C:\Users\sensitive-owner\gpu')
        with mock.patch.dict(os.environ, {'USERNAME': 'sensitive-owner'}):
            result = costs.summarize(self.group(edit=private) + self.group('fog', edit=private))
        for payload in (json.dumps(result), costs.render_markdown(result)):
            self.assertNotIn('sensitive-owner', payload)
            self.assertNotIn(str(self.base), payload)
            self.assertNotIn('C:', payload)
        self.assertEqual(result['controls']['presenting_adapter'], 'redacted')
        self.assertNotIn('presenting_adapter', result['features'][3])

    def test_joins_use_raw_identifiers_when_sanitized_ones_collide(self):
        # Candidates and run IDs that contain the user name all sanitize to 'redacted'.
        def owner(candidate):
            return lambda r: r.update(candidate=candidate, run_id=f"synthetic-owner-{r['run_id']}")
        with mock.patch.dict(os.environ, {'USERNAME': 'synthetic-owner'}):
            dirs = (self.group(ms=25, edit=owner('synthetic-owner')) + self.group('fog', ms=30, edit=owner('synthetic-owner'))
                    + self.group('fog', ms=30, scene='w3', edit=owner('synthetic-owner')))
            before = self.feature(costs.summarize(dirs, candidate='synthetic-owner'))
            other = self.group('fog', ms=70, scene='w3', edit=owner('synthetic-owner-z'))
            after = self.feature(costs.summarize(dirs + other, candidate='synthetic-owner'))
        self.assertEqual((before['cost_mean_ms'], before['p99_ms'], before['verdict'], before['breaks_gate']), (5, 30, 'met', False))
        self.assertEqual(after, before)

    def test_cli_candidate_trace_timing_and_readable_invalid_runs(self):
        edit = lambda r: r.update(candidate='qt')
        directories = self.group(ms=31) + self.group('fog', ms=34)
        directories += self.group(ms=25, edit=edit) + self.group('fog', ms=30, edit=edit)
        out = self.base / 'trace-table.json'
        self.assertEqual(costs.main([*map(str, directories), '--candidate', 'qt', '--timing', 'trace', '--out', str(out)]), 0)
        result = json.loads(out.read_text(encoding='utf-8'))
        self.assertEqual((result['candidate'], result['timing_source']), ('qt', 'probe-trace'))
        self.assertEqual((self.feature(result)['status'], self.feature(result)['cost_mean_ms']), ('preliminary', 5))
        failed = self.group(edit=lambda r: r['label_check'].update(status='fail'))
        out = self.base / 'invalid-table.json'
        self.assertEqual(costs.main([*map(str, failed), '--out', str(out)]), 0)
        self.assertEqual(json.loads(out.read_text(encoding='utf-8'))['baseline']['status'], 'unmeasured')

    def test_turn_phase_simulation_end_to_end_in_both_timings(self):
        with mock.patch.multiple(gate, WARMUP_S=10, INTERVAL_S=180):
            directories = []
            for variant in ('none', 'fog'):
                for i in range(3):
                    run = metadata(f'phase-{variant}-{i}', variant, turn_frames=157, cycle_frames=3140)
                    entries, durations = simulate(run, lambda a, p, t: 1.3 + (15 if p < 0.5 else 0)
                                                  + (5 if variant == 'fog' else 0), seed=800 + i)
                    directories.append(self.write_run(run, entries, durations))
            for timing in ('presentmon', 'trace'):
                result = costs.summarize(directories, timing=timing)
                self.assertEqual(result['invalid_runs'], [])
                self.assertEqual(result['cost_invalid_runs'], [])
                row = self.feature(result)
                self.assertAlmostEqual(row['cost_mean_ms'], 5, delta=0.025)
                self.assertEqual(row['status'], 'measured' if timing == 'presentmon' else 'preliminary')


class SimulationTests(unittest.TestCase):
    def test_h0603_whole_cycles_remove_all_state_confounds(self):
        cases = [
            ('camera', lambda a, p, t: 1.3, lambda a, p, t: 30 if a < math.pi else 0),
            ('turn phase', lambda a, p, t: 1.3 + (15 if p < 0.5 else 0), lambda a, p, t: 5),
            ('direction', lambda a, p, t: 1.3 + (10 if t % 2 else 0), lambda a, p, t: 5),
            ('smooth camera', lambda a, p, t: 1.3 + 0.3 * math.sin(a), lambda a, p, t: 10 * (1 + math.sin(a))),
            ('within a phase bin', lambda a, p, t: 3.8 + 2.5 * math.sin(16 * math.pi * p), lambda a, p, t: 5),
            ('constant', lambda a, p, t: 1.3, lambda a, p, t: 30),
        ]
        for name, baseline, added in cases:
            truth = math.fsum(added((k + 1) * 0.002, (k % 157) / 157, k // 157) for k in range(3140)) / 3140
            for timing, lags in (('presentmon', (0, 0)), ('presentmon', (0, 2)),
                                 ('presentmon', (2, 0)), ('trace', (0, 0))):
                pooled, cycles = [[], []], [[], []]
                for group in range(2):
                    for i in range(3):
                        run = metadata('simulation', turn_frames=157, cycle_frames=3140)
                        cost = lambda a, p, t, g=group: baseline(a, p, t) + (added(a, p, t) if g else 0)
                        entries, durations = simulate(run, cost, seed=800 + i)
                        values, ticks = measured(run, entries, durations, timing, lags[group])
                        window, count, reasons = costs.cost_window(run, entries, values, ticks, timing)
                        self.assertEqual(reasons, [], (name, timing, lags))
                        self.assertEqual(len(window), count * 3140)
                        pooled[group].extend(window)
                        cycles[group].append(count)
                stats = [costs.paired_statistics(values) for values in pooled]
                actual = stats[1]['paired_mean_ms'] - stats[0]['paired_mean_ms']
                with self.subTest(case=name, timing=timing, lags=lags):
                    self.assertAlmostEqual(actual, truth, delta=max(0.02, abs(truth) * 0.005))
                SIMULATION_RESULTS.append({'case': name, 'timing': timing, 'lags': lags,
                                           'truth_ms': truth, 'cost_ms': actual, 'cycles': cycles})

    def test_full_interval_without_a_whole_cycle_is_refused(self):
        run = metadata('slow', turn_frames=157, cycle_frames=3140)
        entries, durations = simulate(run, lambda *state: 70)
        for timing in ('presentmon', 'trace'):
            values, ticks = measured(run, entries, durations, timing)
            self.assertEqual(costs.cost_window(run, entries, values, ticks, timing), ([], 0, ['cycle-coverage']))


if __name__ == '__main__':
    unittest.main()
