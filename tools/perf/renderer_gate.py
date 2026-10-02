"""Judge stage 2.4 renderer runs against the selection gate, using every frame of a fixed interval.

The gate (docs/wiki/decisions/renderer-candidates.md) needs three things: an average of at
least 30 fps, a 99th-percentile frame time of at most 33.3 ms, and each scene running for
several minutes, three times.

A run counts the PresentMon frames of the workload's declared swap chain whose
timestamps fall in [start + 10 s, start + 190 s), where start is the probe's trace-start
marker. The frames must cover that interval without missing rows. The gate is met only if
every one of at least three valid runs of one build meets both thresholds, and the pooled
frames of those runs meet them as well.

Every run must declare the gate conditions: mains power, no frame generation, no
upscaling, and a backbuffer at the display's native size. Gate scenes (w3, w5) must also
show, in the probe's per-frame trace log, that the measured interval is one unbroken
sequence of turns of the declared duration, with the label revision adopted after every
turn. The probe writes one trace entry per frame, with a QPC time read after the previous
Present returned and before this frame's Present, so each step between two presents holds
exactly one entry. Its phase is the elapsed fraction of the current turn on the QPC clock. Label scenes (w3, w4, w5) must carry a passing exact label check.

The PresentMon parsing, the nearest-rank percentile and the output sanitizing come from
b412_summary.py.
"""
from __future__ import annotations

import argparse
import csv
import json
import bisect
import math
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from b412_summary import (PRESENTMON_COLUMNS, RunError, csv_integer, csv_interval,  # noqa: E402
                          integer, nearest_rank, number, public_environment, read_json,
                          require, sanitize)

SCENES = ('w1', 'w2', 'w3', 'w4', 'w5')
GATE_SCENES = ('w3', 'w5')
LABEL_SCENES = ('w3', 'w4', 'w5')
WARMUP_S = 10
INTERVAL_S = 180
EDGE_S = 0.5           # the first and last kept frame must lie this close to the interval edges
GAP_SLACK_MS = 1.0     # a timestamp step may exceed its frame interval by this much plus 1 %
TURN_MS_MAX = 10_000   # a declared turn longer than this would leave too few turns and uploads
PHASE_TOL_MS = 50.0    # every entry of a turn must place the turn's start within this window
FPS_MIN = 30.0
P99_MAX_MS = 33.3
RUNS_MIN = 3
VRAM_BUDGET_MB = 7168
NAME = re.compile(r'[a-z0-9][a-z0-9-]{0,31}')
# The CSV of PresentMon 2.6 with `--v1_metrics --qpc_time`, checked against a real capture on
# 1 October 2026: QPC times are integers in `QPCTime`, and the interval columns start with `ms`.
CAPTURE_COLUMNS = {**PRESENTMON_COLUMNS, 'time': 'QPCTime', 'frame_interval': 'msBetweenPresents',
                   'display_interval': 'msBetweenDisplayChange'}
METHOD = ('every frame of the declared swap chain in [trace start + 10 s, + 190 s), with no '
          'missing rows; fps = frames x 1000 / total frame time; nearest-rank p99; no outlier '
          'removal; met only if every run and the pooled frames meet fps >= 30 and '
          'p99 <= 33.3 ms, with at least three valid runs of one build; optional vram_peak_mb '
          'must be positive and finite; vram_risk flags peaks above 7168 MB without changing '
          'the verdict; this input contract previously ignored vram_peak_mb (including 0); '
          'no committed run carried that key before this change')


def validate_run(run):
    require(isinstance(run, dict))
    require(run.get('format') == 'magic600-renderer-run-v1', 'bad-format')
    require(isinstance(run.get('run_id'), str) and bool(run['run_id'].strip()))
    require(run.get('scene') in SCENES)
    require(isinstance(run.get('candidate'), str) and NAME.fullmatch(run['candidate']) is not None, 'bad-candidate')
    require(integer(run.get('qpc_frequency')) and run['qpc_frequency'] > 0)
    markers = run.get('markers')
    require(isinstance(markers, dict), 'missing-marker')
    require(integer(markers.get('trace_start_qpc')) and integer(markers.get('trace_stop_qpc')), 'missing-marker')
    require(markers['trace_stop_qpc'] >= markers['trace_start_qpc'], 'missing-marker')
    presentmon = run.get('presentmon')
    require(isinstance(presentmon, dict) and integer(presentmon.get('process_id')))
    require(isinstance(presentmon.get('swap_chain'), str) and bool(presentmon['swap_chain'].strip()), 'missing-swap-chain')
    build = run.get('build')
    require(isinstance(build, dict) and isinstance(build.get('build_identity'), str)
            and bool(build['build_identity'].strip()), 'missing-build-identity')
    if run['scene'] in GATE_SCENES:
        require(number(run.get('turn_ms')) and 0 < run['turn_ms'] <= TURN_MS_MAX, 'bad-turn-duration')
    if run['scene'] in LABEL_SCENES:
        check = run.get('label_check')
        require(isinstance(check, dict) and check.get('status') in ('pass', 'fail'), 'missing-label-check')
    if 'vram_peak_mb' in run:
        require(number(run['vram_peak_mb']) and run['vram_peak_mb'] > 0, 'bad-vram')


def condition_reasons(environment):
    """The gate conditions every run must declare: missing ones and violated ones."""
    if not isinstance(environment, dict):
        return ['conditions-missing']
    declared, display, backbuffer = (environment.get(key) for key in ('declared', 'display', 'backbuffer'))
    facts = {
        'power_source': environment.get('power_source'),
        'frame_generation': declared.get('frame_generation') if isinstance(declared, dict) else None,
        'upscaling': declared.get('upscaling') if isinstance(declared, dict) else None,
        'display': display if isinstance(display, dict) else None,
        'backbuffer': backbuffer if isinstance(backbuffer, dict) else None,
    }
    if (facts['power_source'] is None or type(facts['frame_generation']) is not bool
            or type(facts['upscaling']) is not bool or facts['display'] is None or facts['backbuffer'] is None
            or not all(integer(facts[key].get(axis)) and facts[key][axis] > 0
                       for key in ('display', 'backbuffer') for axis in ('width', 'height'))):
        return ['conditions-missing']
    met = (facts['power_source'] == 'mains' and not facts['frame_generation'] and not facts['upscaling']
           and all(facts['display'][axis] == facts['backbuffer'][axis] for axis in ('width', 'height')))
    return [] if met else ['conditions-not-met']


def interval_ticks(run):
    start = run['markers']['trace_start_qpc'] + WARMUP_S * run['qpc_frequency']
    return start, start + INTERVAL_S * run['qpc_frequency']


def read_frames(directory, run, columns):
    """Frame intervals (ms) of the declared swap chain in the measured interval, the frames'
    timestamps, the other chains of the process, and invalid reasons."""
    begin, end = interval_ticks(run)
    declared = run['presentmon']['swap_chain'].strip()
    frames, others = [], {}
    try:
        with (directory / 'presentmon.csv').open(encoding='utf-8-sig', newline='') as stream:
            reader = csv.DictReader(stream, strict=True)
            header = reader.fieldnames or []
            require(all(columns[key] in header for key in
                        ('process', 'swap_chain', 'time', 'frame_interval')), 'presentmon-missing-column')
            for row in reader:
                if csv_integer(row.get(columns['process'])) != run['presentmon']['process_id']:
                    continue
                ticks = csv_integer(row.get(columns['time']))
                if not begin <= ticks < end:
                    continue
                chain = row.get(columns['swap_chain'])
                require(isinstance(chain, str) and bool(chain.strip()))
                if chain.strip() == declared:
                    frames.append((ticks, csv_interval(row.get(columns['frame_interval']))))
                else:
                    others[chain.strip()] = others.get(chain.strip(), 0) + 1
    except OSError:
        raise RunError('missing-file') from None
    except (csv.Error, UnicodeError):
        raise RunError('bad-field') from None
    ignored = [{'swap_chain': chain, 'rows': rows} for chain, rows in sorted(others.items())]
    if not frames:
        return None, [], ignored, ['presentmon-no-rows']
    frames.sort()
    edge = EDGE_S * run['qpc_frequency']
    reasons = []
    if run['markers']['trace_stop_qpc'] < end:
        reasons.append('capture-short')
    if frames[0][0] > begin + edge or frames[-1][0] < end - edge:
        reasons.append('capture-not-covered')
    # Every step between kept presents must be explained by the later frame's own interval;
    # a longer step means rows are missing and their time would escape the statistics.
    to_ms = 1000 / run['qpc_frequency']
    for (before, _), (after, interval) in zip(frames, frames[1:]):
        if (after - before) * to_ms > interval * 1.01 + GAP_SLACK_MS:
            reasons.append('presentmon-rows-missing')
            break
    return [interval for _, interval in frames], [ticks for ticks, _ in frames], ignored, reasons


def read_trace(directory):
    try:
        with (directory / 'trace.jsonl').open(encoding='utf-8') as stream:
            entries = [json.loads(line) for line in stream if line.strip()]
    except (ValueError, UnicodeError):
        raise RunError('bad-json') from None
    except OSError:
        raise RunError('missing-file') from None
    for entry in entries:
        require(isinstance(entry, dict) and integer(entry.get('qpc')) and integer(entry.get('revision')), 'bad-trace')
        require(entry.get('turn') is None or integer(entry['turn']), 'bad-trace')
        require(entry.get('phase') is None or (number(entry['phase']) and 0 <= entry['phase'] <= 1), 'bad-trace')
    return sorted(entries, key=lambda entry: entry['qpc'])


def trace_reasons(entries, run, frame_ticks):
    """Each step between two measured presents holds exactly one trace entry and the trace
    covers the interval; every entry animates a turn; the phase follows the QPC clock at the
    declared duration and turns follow one another without a pause; and each frame shows the
    label revision of the turns completed before it (revision == turn)."""
    begin, end = interval_ticks(run)
    window = [entry for entry in entries if begin <= entry['qpc'] < end]
    if not window:
        return ['trace-empty']
    reasons = set()
    edge = EDGE_S * run['qpc_frequency']
    times = [entry['qpc'] for entry in entries]
    steps = zip(frame_ticks, frame_ticks[1:])
    if (any(bisect.bisect_right(times, after) - bisect.bisect_right(times, before) != 1 for before, after in steps)
            or window[0]['qpc'] > begin + edge or window[-1]['qpc'] < end - edge):
        reasons.add('trace-incomplete')
    # Each entry places its turn's start at qpc - phase x duration. A frozen, slowed or
    # frame-stepped animation spreads those starts; a pause between turns delays the next start.
    turn_ticks = run['turn_ms'] * run['qpc_frequency'] / 1000
    tolerance = PHASE_TOL_MS * run['qpc_frequency'] / 1000
    starts = {}
    previous = None
    for entry in window:
        if entry.get('turn') is None or entry.get('phase') is None:
            reasons.add('trace-idle')
            previous = None
            continue
        if previous is not None:
            if entry['turn'] not in (previous['turn'], previous['turn'] + 1):
                reasons.add('trace-turn-gap')
            elif entry['turn'] == previous['turn'] and entry['phase'] < previous['phase']:
                reasons.add('trace-phase-backwards')
        if entry['revision'] != entry['turn']:
            reasons.add('label-adoption-missed')
        starts.setdefault(entry['turn'], []).append(entry['qpc'] - entry['phase'] * turn_ticks)
        previous = entry
    if any(max(values) - min(values) > tolerance for values in starts.values()):
        reasons.add('trace-off-clock')
    for turn in starts:
        if turn + 1 in starts and abs(min(starts[turn + 1]) - min(starts[turn]) - turn_ticks) > 2 * tolerance:
            reasons.add('trace-off-clock')
    animated = [entry['turn'] for entry in window if entry.get('turn') is not None]
    expected = INTERVAL_S * 1000 / run['turn_ms']
    if not animated or max(animated) - min(animated) < math.floor(expected) - 1:
        reasons.add('trace-too-few-turns')
    return sorted(reasons)


def judge(values):
    total = math.fsum(values)
    require(total > 0)
    fps, p99 = len(values) * 1000 / total, nearest_rank(values, 99)
    return {'n': len(values), 'fps': fps, 'p99_ms': p99, 'max_ms': max(values),
            'met': fps >= FPS_MIN and p99 <= P99_MAX_MS}


def summarize(directories, columns=None):
    columns = {**CAPTURE_COLUMNS, **(columns or {})}
    resolved = [os.path.normcase(str(Path(directory).resolve())) for directory in directories]
    if len(set(resolved)) != len(resolved):
        raise ValueError('duplicate-run')
    result = {'format': 'magic600-renderer-gate-v1', 'method': METHOD,
              'interval_s': INTERVAL_S, 'warmup_s': WARMUP_S, 'scenes': [],
              'invalid_runs': [], 'unreadable': [], 'environment': []}
    scenes, frames, run_ids = {}, {}, set()
    for directory in map(Path, directories):
        try:
            run = read_json(directory / 'run.json')
            validate_run(run)
            if run['run_id'] in run_ids:
                raise ValueError('duplicate-run')
            run_ids.add(run['run_id'])
            values, ticks, ignored, reasons = read_frames(directory, run, columns)
            reasons += condition_reasons(run.get('environment'))
            if run['scene'] in GATE_SCENES:
                reasons += trace_reasons(read_trace(directory), run, ticks)
            if run['scene'] in LABEL_SCENES and run['label_check']['status'] != 'pass':
                reasons.append('label-check-failed')
            environment = public_environment(run.get('environment', {}))
        except RunError as error:
            result['unreadable'].append({'reason': str(error)})
            continue
        key = (run['candidate'], run['scene'])
        scene = scenes.setdefault(key, {'candidate': run['candidate'], 'scene': run['scene'],
                                                  'gate_scene': run['scene'] in GATE_SCENES,
                                                  'runs': [], 'pooled': None, 'verdict': 'no-data',
                                                  'vram_peak_mb': None})
        if reasons:
            result['invalid_runs'].append({'run_id': run['run_id'], 'candidate': run['candidate'],
                                           'scene': run['scene'], 'reasons': sorted(set(reasons))})
            continue
        vram_peak = run.get('vram_peak_mb')
        scene['runs'].append({'run_id': run['run_id'], 'build_identity': run['build']['build_identity'],
                              'swap_chain': run['presentmon']['swap_chain'], 'swap_chains_ignored': ignored,
                              'vram_peak_mb': vram_peak,
                              'vram_risk': vram_peak is not None and vram_peak > VRAM_BUDGET_MB,
                              **judge(values)})
        if vram_peak is not None:
            scene['vram_peak_mb'] = max(scene['vram_peak_mb'] or 0, vram_peak)
        frames.setdefault(key, []).extend(values)
        result['environment'].append({'run_id': run['run_id'], **environment})
    # A list, not a mapping keyed by candidate: the sanitizer redacts values, not keys.
    result['scenes'] = [scenes[key] for key in sorted(scenes)]
    for key, scene in sorted(scenes.items()):
        if not scene['runs']:
            continue
        scene['pooled'] = judge(frames[key])
        if not scene['gate_scene']:
            scene['verdict'] = 'attribution'
        elif len(scene['runs']) < RUNS_MIN:
            scene['verdict'] = 'insufficient-runs'
        elif len({run['build_identity'] for run in scene['runs']}) != 1:
            scene['verdict'] = 'mixed-builds'
        else:
            # Per-run passes imply the pooled pass (summed nearest ranks, mediant of fps); the
            # pooled check is kept as the documented rule and as a guard against later changes.
            scene['verdict'] = 'met' if scene['pooled']['met'] and all(run['met'] for run in scene['runs']) else 'not-met'
    return sanitize(result)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directories', type=Path, nargs='+')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args(argv)
    if args.out.exists():
        parser.error('existing-output')
    try:
        result = summarize(args.directories)
    except ValueError as error:
        parser.error(str(error))
    if len(result['unreadable']) == len(args.directories):
        print('no-readable-runs', file=sys.stderr)
        return 1
    try:
        with args.out.open('x', encoding='utf-8', newline='\n') as stream:
            stream.write(json.dumps(result, indent=2, ensure_ascii=False, allow_nan=False) + '\n')
    except OSError:
        parser.error('cannot-create-output')
    return 0


if __name__ == '__main__':
    sys.exit(main())
