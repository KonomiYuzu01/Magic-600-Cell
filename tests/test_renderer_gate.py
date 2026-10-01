"""Stage 2.4 renderer gate adapter, using synthetic files only (no renderer runs)."""
from __future__ import annotations

import contextlib
import io
import json
import math
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'tools/perf'))
import renderer_gate as gate  # noqa: E402

FREQ = 1000          # QPC ticks per second: one tick per millisecond keeps the fixtures readable
START = 50_000
TURN_MS = 1_500
HEADER = ('Application,ProcessID,SwapChainAddress,Runtime,SyncInterval,PresentFlags,Dropped,TimeInSeconds,'
          'msInPresentAPI,msBetweenPresents,AllowsTearing,PresentMode,msUntilRenderComplete,msUntilDisplayed,'
          'msBetweenDisplayChange,msFlipDelay,msUntilRenderStart,msGPUActive,msSinceInput,QPCTime').split(',')


def row(process, chain, qpc, ms):
    values = dict.fromkeys(HEADER, '0')
    values.update(Application='probe.exe', ProcessID=str(process), SwapChainAddress=chain, QPCTime=str(qpc),
                  msBetweenPresents=str(ms), msBetweenDisplayChange=str(ms))
    return [values[name] for name in HEADER]


def steady(ms):
    return lambda elapsed_ms, index: ms


class GateTests(unittest.TestCase):
    def setUp(self):
        if os.name == 'nt':
            # Same ACL workaround as test_b412_summary: inherit the workspace ACL.
            mkdir = os.mkdir
            with mock.patch.object(os, 'mkdir', side_effect=lambda path, mode: mkdir(path)):
                temporary = tempfile.TemporaryDirectory(prefix='.gate-test-', dir=ROOT)
        else:
            temporary = tempfile.TemporaryDirectory(prefix='.gate-test-', dir=ROOT)
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)

    def run_dir(self, run_id, frame_ms=steady(25), scene='w3', seconds=gate.WARMUP_S + gate.INTERVAL_S + 1,
                stop=None, label='pass', build='b1', trace=None, candidate='sb', rows=None, edit=None, freq=FREQ):
        """One run directory: run.json, presentmon.csv and trace.jsonl. `trace` edits each trace
        entry (None drops it), `rows` edits the list of PresentMon rows (process, chain, qpc, ms)
        and `edit` edits run.json."""
        directory = self.base / run_id
        directory.mkdir()
        # Frame times are fractional milliseconds; QPC ticks are rounded at `freq` ticks per second,
        # independently of the adapter's constants.
        frames, entries, elapsed, index = [], [], 0.0, 0
        start = START * freq // 1000
        while elapsed < seconds * 1000:
            ms = frame_ms(elapsed, index)
            elapsed += ms
            index += 1
            ticks = start + round(elapsed * freq / 1000)
            frames.append((4242, '0xA', ticks, ms))
            turn = int(elapsed // TURN_MS)
            entry = {'qpc': ticks, 'turn': turn, 'phase': (elapsed % TURN_MS) / TURN_MS, 'revision': turn}
            entry = trace(entry) if trace else entry
            if entry is not None:
                entries.append(entry)
        frames.append((9999, '0xB', start + 20 * freq, 5))     # another process: ignored
        frames = rows(frames) if rows else frames
        (directory / 'presentmon.csv').write_text(
            # The header of a real PresentMon 2.6 capture with --v1_metrics --qpc_time.
            '﻿' + ','.join(HEADER) + '\n'
            + ''.join(','.join(row(p, c, q, m)) + '\n' for p, c, q, m in frames), encoding='utf-8')
        (directory / 'trace.jsonl').write_text(''.join(json.dumps(entry) + '\n' for entry in entries), encoding='utf-8')
        run = {'format': 'magic600-renderer-run-v1', 'run_id': run_id, 'scene': scene, 'candidate': candidate,
               'qpc_frequency': freq, 'presentmon': {'process_id': 4242, 'swap_chain': '0xA'},
               'build': {'build_identity': build}, 'turn_ms': TURN_MS,
               'markers': {'trace_start_qpc': start, 'trace_stop_qpc': stop if stop is not None else ticks},
               'label_check': {'status': label},
               'environment': {'power_source': 'mains', 'display': {'width': 2560, 'height': 1600},
                               'backbuffer': {'width': 2560, 'height': 1600},
                               'declared': {'frame_generation': False, 'upscaling': False}}}
        if edit:
            edit(run)
        (directory / 'run.json').write_text(json.dumps(run), encoding='utf-8')
        return directory

    def three(self, prefix='r', **options):
        return gate.summarize([self.run_dir(f'{prefix}{i}', **options) for i in range(3)])

    def reasons(self, result):
        return result['invalid_runs'][0]['reasons']

    def scene(self, result, key='sb/w3'):
        found = [scene for scene in result['scenes'] if f"{scene['candidate']}/{scene['scene']}" == key]
        self.assertEqual(len(found), 1, key)
        return found[0]

    def test_three_steady_runs_meet_the_gate(self):
        result = gate.summarize([self.run_dir(f'r{i}') for i in range(3)])
        scene = self.scene(result)
        self.assertEqual(scene['verdict'], 'met')
        self.assertEqual(len(scene['runs']), 3)
        self.assertAlmostEqual(scene['pooled']['fps'], 40.0)
        self.assertEqual(result['invalid_runs'], [])
        # Every frame of the 180 s interval counts, not a fixed sample.
        self.assertGreater(scene['runs'][0]['n'], 7000)

    def test_a_late_slowdown_fails(self):
        # The plan check counterexample: 100 fast frames after warmup, then 50 ms frames.
        warm = gate.WARMUP_S * FREQ

        def late(elapsed, index):
            return 17 if elapsed < warm + 100 * 17 else 50
        scene = self.scene(gate.summarize([self.run_dir(f'r{i}', late) for i in range(3)]))
        self.assertEqual(scene['verdict'], 'not-met')
        self.assertLess(scene['pooled']['fps'], 30)
        self.assertEqual(scene['pooled']['p99_ms'], 50)

    def test_p99_alone_can_fail_the_gate(self):
        spiky = lambda elapsed, index: 40 if index % 50 == 0 else 20   # 2 % of frames at 40 ms
        scene = self.scene(gate.summarize([self.run_dir(f'r{i}', spiky) for i in range(3)]))
        self.assertGreaterEqual(scene['pooled']['fps'], 30)
        self.assertEqual(scene['pooled']['p99_ms'], 40)
        self.assertEqual(scene['verdict'], 'not-met')

    def test_one_failing_run_fails_even_when_pooled_passes(self):
        spiky = lambda elapsed, index: 40 if index % 50 == 0 else 20   # this run's p99 is 40 ms; pooled stays below 1 %
        dirs = [self.run_dir('r0'), self.run_dir('r1'), self.run_dir('r2', spiky)]
        scene = self.scene(gate.summarize(dirs))
        self.assertTrue(scene['pooled']['met'])
        self.assertFalse(scene['runs'][2]['met'])
        self.assertEqual(scene['verdict'], 'not-met')

    def test_boundaries_are_inclusive(self):
        exact = lambda elapsed, index: 33.3                 # 30.03 fps, p99 exactly 33.3 ms
        self.assertEqual(self.scene(gate.summarize([self.run_dir(f'r{i}', exact) for i in range(3)]))['verdict'], 'met')
        tie = gate.judge([1000 / 30] * 10)
        self.assertTrue(tie['fps'] >= 30 - 1e-9)

    def test_fewer_than_three_runs_are_insufficient(self):
        scene = self.scene(gate.summarize([self.run_dir('r0'), self.run_dir('r1')]))
        self.assertEqual(scene['verdict'], 'insufficient-runs')

    def test_runs_of_different_builds_do_not_pool(self):
        dirs = [self.run_dir('r0'), self.run_dir('r1'), self.run_dir('r2', build='b2')]
        self.assertEqual(self.scene(gate.summarize(dirs))['verdict'], 'mixed-builds')

    def test_short_captures_are_invalid(self):
        short = self.run_dir('short', seconds=gate.WARMUP_S + 60)
        early_stop = self.run_dir('stop', stop=START + (gate.WARMUP_S + 100) * FREQ)
        result = gate.summarize([short, early_stop, self.run_dir('r0')])
        reasons = {run['run_id']: run['reasons'] for run in result['invalid_runs']}
        self.assertIn('capture-not-covered', reasons['short'])
        self.assertIn('capture-short', reasons['stop'])
        self.assertEqual(self.scene(result)['verdict'], 'insufficient-runs')

    def test_a_missing_marker_is_refused(self):
        directory = self.run_dir('r0')
        run = json.loads((directory / 'run.json').read_text(encoding='utf-8'))
        del run['markers']['trace_start_qpc']
        (directory / 'run.json').write_text(json.dumps(run), encoding='utf-8')
        result = gate.summarize([directory])
        self.assertEqual(result['unreadable'], [{'reason': 'missing-marker'}])

    def test_repeated_runs_are_refused_not_pooled(self):
        first = self.run_dir('r0')
        with self.assertRaisesRegex(ValueError, 'duplicate-run'):
            gate.summarize([first, self.run_dir('r1'), first])
        copy = self.run_dir('copy')
        run = json.loads((copy / 'run.json').read_text(encoding='utf-8'))
        run['run_id'] = 'r0'
        (copy / 'run.json').write_text(json.dumps(run), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'duplicate-run'):
            gate.summarize([first, copy, self.run_dir('r2')])

    def test_an_idle_gap_invalidates_a_gate_run(self):
        def idle(entry):
            if 60_000 <= entry['qpc'] - START < 62_000:
                return {**entry, 'turn': None, 'phase': None}
            return entry
        result = gate.summarize([self.run_dir('r0', trace=idle)])
        self.assertIn('trace-idle', result['invalid_runs'][0]['reasons'])

    def test_a_missed_label_adoption_invalidates_a_gate_run(self):
        def late(entry):
            # Turn 40 starts but its frames still show the previous revision.
            return {**entry, 'revision': 39} if entry['turn'] == 40 else entry
        result = gate.summarize([self.run_dir('r0', trace=late)])
        self.assertEqual(result['invalid_runs'][0]['reasons'], ['label-adoption-missed'])

    def test_a_skipped_turn_invalidates_a_gate_run(self):
        def skip(entry):
            turn = entry['turn'] + 1 if entry['turn'] >= 40 else entry['turn']
            return {**entry, 'turn': turn, 'revision': turn}
        result = gate.summarize([self.run_dir('r0', trace=skip)])
        self.assertEqual(result['invalid_runs'][0]['reasons'], ['trace-turn-gap'])

    def test_a_failed_label_check_invalidates_a_gate_run(self):
        result = gate.summarize([self.run_dir('r0', label='fail')])
        self.assertEqual(result['invalid_runs'][0]['reasons'], ['label-check-failed'])

    def test_attribution_scenes_need_no_trace(self):
        directory = self.run_dir('w2', scene='w2')
        (directory / 'trace.jsonl').unlink()
        scene = self.scene(gate.summarize([directory]), 'sb/w2')
        self.assertEqual(scene['verdict'], 'attribution')

    def test_candidates_and_scenes_are_judged_separately(self):
        dirs = [self.run_dir(f'a{i}') for i in range(3)] + [self.run_dir(f'b{i}', candidate='qt') for i in range(2)]
        result = gate.summarize(dirs)
        self.assertEqual(self.scene(result, 'sb/w3')['verdict'], 'met')
        self.assertEqual(self.scene(result, 'qt/w3')['verdict'], 'insufficient-runs')

    def test_cli_writes_once_and_refuses_an_existing_output(self):
        dirs = [str(self.run_dir(f'r{i}')) for i in range(3)]
        out = self.base / 'gate.json'
        self.assertEqual(gate.main([*dirs, '--out', str(out)]), 0)
        self.assertEqual(self.scene(json.loads(out.read_text(encoding='utf-8')), 'sb/w3')['verdict'], 'met')
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as raised:
            gate.main([*dirs, '--out', str(out)])
        self.assertEqual(raised.exception.code, 2)

    def test_private_paths_are_redacted(self):
        directory = self.run_dir('r0')
        run = json.loads((directory / 'run.json').read_text(encoding='utf-8'))
        run['environment']['presenting_adapter'] = 'C:\\Users\\someone\\gpu'
        (directory / 'run.json').write_text(json.dumps(run), encoding='utf-8')
        result = gate.summarize([directory])
        self.assertEqual(result['environment'][0]['presenting_adapter'], 'redacted')

    # Review findings A1-A9 (call 20261001T021649Z-00f0235d): each was a false pass or a leak.

    def test_missing_presentmon_rows_invalidate_a_run(self):
        # A1: rows of 60 s removed inside the interval, boundary rows kept.
        cut = lambda frames: [f for f in frames if not 70_000 <= f[2] - START < 130_000]
        self.assertIn('presentmon-rows-missing', self.reasons(gate.summarize([self.run_dir('cut', rows=cut)])))
        ends = lambda frames: [f for f in frames if f[0] != 4242 or abs(f[2] - (START + 10_000)) < 30
                               or abs(f[2] - (START + 190_000)) < 30]
        result = gate.summarize([self.run_dir('ends', rows=ends)])
        self.assertIn('presentmon-rows-missing', self.reasons(result))
        self.assertEqual(self.scene(self.three(), 'sb/w3')['verdict'], 'met')   # the complete capture passes

    def test_an_incomplete_trace_invalidates_a_gate_run(self):
        # A2: one trace entry, or a trace missing its start, end or interior.
        for name, keep in (('one', lambda e: e if e['qpc'] - START in range(10_000, 10_030) else None),
                           ('start', lambda e: e if e['qpc'] - START >= 20_000 else None),
                           ('end', lambda e: e if e['qpc'] - START < 170_000 else None),
                           ('interior', lambda e: None if 90_000 <= e['qpc'] - START < 91_000 else e)):
            with self.subTest(name):
                result = gate.summarize([self.run_dir(name, trace=keep)])
                self.assertIn('trace-incomplete', self.reasons(result))

    def test_a_frozen_animation_invalidates_a_gate_run(self):
        # A3: turn, phase and revision frozen for the whole capture.
        frozen = lambda e: {**e, 'turn': 0, 'phase': 0.5, 'revision': 0}
        self.assertIn('trace-too-few-turns', self.reasons(gate.summarize([self.run_dir('r0', trace=frozen)])))
        backwards = lambda e: {**e, 'phase': 1 - e['phase']}
        self.assertIn('trace-phase-backwards', self.reasons(gate.summarize([self.run_dir('r1', trace=backwards)])))
        slow = lambda run: run.update(turn_ms=gate.TURN_MS_MAX + 1)
        self.assertEqual(gate.summarize([self.run_dir('r2', edit=slow)])['unreadable'], [{'reason': 'bad-turn-duration'}])

    def test_only_the_declared_swap_chain_counts(self):
        # A4: a faster second chain in the same process must not stand in for the workload.
        def second(frames):
            fast = [(4242, '0xB', START + 10_000 + 10 * i, 10) for i in range(18_000)]
            return frames + fast
        result = self.three(frame_ms=steady(50), rows=second)
        scene = self.scene(result, 'sb/w3')
        self.assertEqual(scene['verdict'], 'not-met')
        self.assertAlmostEqual(scene['pooled']['fps'], 20.0)
        self.assertEqual(scene['runs'][0]['swap_chains_ignored'], [{'swap_chain': '0xB', 'rows': 18_000}])
        undeclared = lambda run: run['presentmon'].pop('swap_chain')
        self.assertEqual(gate.summarize([self.run_dir('u', edit=undeclared)])['unreadable'],
                         [{'reason': 'missing-swap-chain'}])

    def test_gate_conditions_are_required(self):
        # A5: frame generation, upscaling, battery power or a scaled backbuffer make a run invalid,
        # and so does a missing declaration.
        cases = {
            'framegen': (lambda run: run['environment']['declared'].update(frame_generation=True), 'conditions-not-met'),
            'dlss': (lambda run: run['environment']['declared'].update(upscaling=True), 'conditions-not-met'),
            'battery': (lambda run: run['environment'].update(power_source='battery'), 'conditions-not-met'),
            'scaled': (lambda run: run['environment']['backbuffer'].update(width=1920), 'conditions-not-met'),
            'undeclared': (lambda run: run['environment']['declared'].pop('frame_generation'), 'conditions-missing'),
            'no-display': (lambda run: run['environment'].pop('display'), 'conditions-missing'),
        }
        for name, (edit, reason) in cases.items():
            with self.subTest(name):
                self.assertEqual(self.reasons(gate.summarize([self.run_dir(name, edit=edit)])), [reason])

    def test_candidate_names_cannot_carry_paths(self):
        # A6: the candidate becomes a result key, which the sanitizer does not clean.
        for name in ('C:\\Users\\someone\\probe', '\\\\host\\share', '/home/someone', 'SB', ''):
            with self.subTest(name):
                directory = self.run_dir(f'c{len(name)}', candidate=name)
                result = gate.summarize([directory])
                self.assertEqual(result['unreadable'], [{'reason': 'bad-candidate'}])
                self.assertNotIn('someone', json.dumps(result))

    def test_build_identities_must_not_be_empty(self):
        # A7
        for value in ('', '   '):
            with self.subTest(repr(value)):
                result = gate.summarize([self.run_dir(f'b{len(value)}', build=value)])
                self.assertEqual(result['unreadable'], [{'reason': 'missing-build-identity'}])

    def test_label_scenes_need_a_passing_label_check(self):
        # A8: W4 is attribution, but its label check still counts.
        result = gate.summarize([self.run_dir('w4', scene='w4', label='fail')])
        self.assertEqual(self.reasons(result), ['label-check-failed'])
        missing = lambda run: run.pop('label_check')
        self.assertEqual(gate.summarize([self.run_dir('w4b', scene='w4', edit=missing)])['unreadable'],
                         [{'reason': 'missing-label-check'}])
        self.assertEqual(self.scene(gate.summarize([self.run_dir('w1', scene='w1', edit=missing)]), 'sb/w1')['verdict'],
                         'attribution')

    def test_fps_is_frames_over_total_time(self):
        # A9: 300 frames in exactly 10 s is exactly 30 fps; one millisecond more is not.
        self.assertTrue(gate.judge([33] * 297 + [66, 66, 67])['met'])
        self.assertEqual(gate.judge([33] * 297 + [66, 66, 67])['fps'], 30.0)
        self.assertFalse(gate.judge([33] * 297 + [66, 66, 68])['met'])

    # Review findings BT2-BT5 (call 20261001T021649Z-110253f7): tests that a relaxed or shifted
    # adapter would still pass. Thresholds and the interval are written out here on purpose.

    def test_thresholds_are_exactly_30_fps_and_33_3_ms(self):
        self.assertFalse(gate.judge([33.31] * 1000)['met'])               # p99 33.31 ms, fps 30.02
        self.assertTrue(gate.judge([33.3] * 1000)['met'])
        self.assertFalse(gate.judge([30] * 990 + [400] * 10)['met'])       # p99 30 ms, fps 29.67
        self.assertTrue(gate.judge([30] * 990 + [330] * 10)['met'])        # p99 30 ms, fps 30.30
        self.assertEqual(self.scene(self.three(frame_ms=steady(33.31)), 'sb/w3')['verdict'], 'not-met')

    def test_the_clock_frequency_is_converted(self):
        # 10 MHz QPC: 180 s of 50 ms frames must be counted as such.
        result = self.three(frame_ms=steady(50), freq=10_000_000)
        scene = self.scene(result, 'sb/w3')
        self.assertEqual(scene['verdict'], 'not-met')
        self.assertEqual(scene['runs'][0]['n'], 3600)
        self.assertEqual(self.scene(self.three('ok', freq=10_000_000), 'sb/w3')['verdict'], 'met')

    def test_the_interval_is_10_to_190_seconds_after_the_trace_start(self):
        # Frames whose present falls in [10 s, 190 s) take 25 ms, all others 40 ms. A frame's
        # timestamp is its end, so the frame times are chosen by where each frame ends.
        regions = lambda elapsed, index: 40 if elapsed < 9_960 else 25 if elapsed + 25 < 190_000 else 40
        scene = self.scene(self.three(frame_ms=regions, seconds=192), 'sb/w3')
        self.assertEqual(scene['pooled']['max_ms'], 25)
        self.assertIn(scene['runs'][0]['n'], (7199, 7200, 7201))

    def test_scenes_are_judged_separately(self):
        dirs = [self.run_dir('w3a'), self.run_dir('w3b'), self.run_dir('w5a', scene='w5')]
        result = gate.summarize(dirs)
        self.assertEqual(self.scene(result, 'sb/w3')['verdict'], 'insufficient-runs')
        self.assertEqual(self.scene(result, 'sb/w5')['verdict'], 'insufficient-runs')
        frozen = lambda e: {**e, 'turn': 0, 'phase': 0.5, 'revision': 0}
        self.assertIn('trace-too-few-turns', self.reasons(gate.summarize([self.run_dir('w5b', scene='w5', trace=frozen)])))

    def test_adoption_one_frame_late_invalidates_a_gate_run(self):
        first = {}

        def late(entry):
            if entry['turn'] == 40 and 40 not in first:
                first[40] = True
                return {**entry, 'revision': 39}                          # only the first frame of turn 40
            return entry
        self.assertEqual(self.reasons(gate.summarize([self.run_dir('late', trace=late)])), ['label-adoption-missed'])

    # Verification findings SV1-SV4 (call 20261001T023026Z-6c6bb716): the reviewer's counterexamples.

    def test_each_present_step_holds_exactly_one_trace_entry(self):
        # SV1: a single missing or repeated interior entry, not only a long gap.
        missing = lambda e: None if e['turn'] == 40 and e['phase'] == 0 else e
        result = self.three('m', trace=missing)
        self.assertEqual(result['invalid_runs'][0]['reasons'], ['trace-incomplete'])
        self.assertEqual(self.scene(result)['verdict'], 'no-data')
        repeated = []

        def twice(entry):
            if entry['turn'] == 40 and entry['phase'] == 0:
                repeated.append({**entry, 'qpc': entry['qpc'] - 1})
            return entry
        directory = self.run_dir('d', trace=twice)
        with (directory / 'trace.jsonl').open('a', encoding='utf-8') as stream:
            stream.write(json.dumps(repeated[0]) + '\n')
        self.assertEqual(self.reasons(gate.summarize([directory])), ['trace-incomplete'])

    def test_the_phase_must_follow_the_clock(self):
        # SV2: a phase frozen within every turn, a slowed animation and turns cut short.
        frozen = lambda e: {**e, 'phase': 0.5}
        self.assertEqual(self.reasons(self.three('f', trace=frozen)), ['trace-off-clock'])
        stalled = lambda e: {**e, 'phase': 0.5} if e['turn'] == 40 and e['phase'] > 0.5 else e   # turn starts stay in step
        self.assertEqual(self.reasons(gate.summarize([self.run_dir('h', trace=stalled)])), ['trace-off-clock'])
        slowed = lambda e: {**e, 'phase': e['phase'] * 0.8}
        self.assertEqual(self.reasons(gate.summarize([self.run_dir('s', trace=slowed)])), ['trace-off-clock'])

        def short(entry):                       # each turn ends at phase 0.8 and the next begins
            elapsed = entry['qpc'] - START
            turn = elapsed // 1_200
            return {**entry, 'turn': turn, 'phase': (elapsed % 1_200) / TURN_MS, 'revision': turn}
        self.assertEqual(self.reasons(gate.summarize([self.run_dir('c', trace=short)])), ['trace-off-clock'])

    def test_native_resolution_must_be_a_real_size(self):
        # SV3: zero, negative or boolean sizes are not a declaration.
        for value in (0, -1, True):
            with self.subTest(value):
                def edit(run):
                    run['environment']['display'] = {'width': value, 'height': value}
                    run['environment']['backbuffer'] = {'width': value, 'height': value}
                self.assertEqual(self.reasons(gate.summarize([self.run_dir(f'z{value}', edit=edit)])),
                                 ['conditions-missing'])

    def test_candidate_names_matching_private_values_are_redacted(self):
        # SV4: a candidate that equals the user name must not survive as a mapping key.
        with mock.patch.dict(os.environ, {'USERNAME': 'synthetic-owner'}):
            result = self.three(candidate='synthetic-owner')
        self.assertNotIn('synthetic-owner', json.dumps(result))
        self.assertEqual([scene['verdict'] for scene in result['scenes']], ['met'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
