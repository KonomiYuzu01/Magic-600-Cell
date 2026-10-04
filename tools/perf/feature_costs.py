"""Build the H-06 table from W3 gate runs and whole-cycle W3f feature cost runs."""
from __future__ import annotations

import argparse
import bisect
import json
import math
import sys
from contextlib import ExitStack
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import renderer_gate  # noqa: E402
from renderer_gate import (CAPTURE_COLUMNS, NAME, RUNS_MIN, condition_reasons, integer,  # noqa: E402
                           nearest_rank, number, read_json, read_trace, run_frames, trace_step_reasons,
                           validate_run)
from b412_summary import RunError, markdown_text, sanitize  # noqa: E402

FEATURES = ('gaps', 'outlines', 'transparency', 'fog', 'dof', 'ao', 'msaa4')
CAMERA = {'plane': [0, 3], 'step_rad': 0.002, 'per': 'frame'}
METHOD = ('costs are paired: every variant draws the same W3 states (scene w3f), in whole '
          'cycles; gate fields come from W3 runs. Windows contain the centred whole cycles '
          'of consecutive frames; pooled arithmetic mean and nearest-rank p99; cost is '
          'variant minus baseline, reversed for gaps. A fixed presentation lag of up to two '
          'frames preserves the state mix; changing lag can move the ends by at most two '
          'frames. Machine speed is assumed steady during the warmed-up interval under '
          'the recorded power conditions. Unmeasured features are not estimated; '
          'not-comparable features have no cost')


def camera_known(value):
    return (isinstance(value, dict) and value == CAMERA
            and isinstance(value.get('plane'), list) and all(integer(axis) for axis in value['plane']))


def cost_window(run, entries, values, ticks, timing):
    """Validate every state, pair nominal entries and retain centred whole cycles."""
    T, n = run['turn_frames'], run['cycle_frames']
    if not camera_known(run.get('camera')):
        return [], 0, ['state-mismatch']
    for k, entry in enumerate(entries):
        if (not all(integer(entry.get(key)) for key in ('frame', 'camera', 'turn', 'revision'))
                or entry['frame'] != k or entry['camera'] != k % n + 1
                or entry['turn'] != k // T or entry['revision'] != k // T
                or not number(entry.get('phase')) or abs(entry['phase'] - (k % T) / T) > 1e-9):
            return [], 0, ['state-mismatch']
    times = [entry['qpc'] for entry in entries]
    nominal = []
    for tick in ticks:
        index = bisect.bisect_right(times, tick) - 1
        if index < 0 or (timing == 'trace' and times[index] != tick):
            return [], 0, ['state-mismatch']
        nominal.append(entries[index]['frame'])
    if len(values) != len(nominal) or any(after != before + 1 for before, after in zip(nominal, nominal[1:])):
        return [], 0, ['state-mismatch']
    cycles = len(values) // n
    if not cycles:
        return [], 0, ['cycle-coverage']
    length = cycles * n
    offset = (len(values) - length) // 2
    return values[offset:offset + length], cycles, []


def paired_statistics(values):
    if not values:
        return {'paired_mean_ms': None, 'paired_p99_ms': None}
    return {'paired_mean_ms': math.fsum(values) / len(values),
            'paired_p99_ms': nearest_rank(values, 99)}


def control_values(run):
    """Only the comparison controls; dotted names identify individual nested fields."""
    controls = {'build_identity': run['build']['build_identity'],
                'turn_frames': run['turn_frames'], 'cycle_frames': run['cycle_frames']}
    camera = run.get('camera')
    controls['camera'] = ({key: camera.get(key) for key in CAMERA} if isinstance(camera, dict) else None)
    environment = run['environment']
    for key in ('presentation_interval', 'vsync', 'tearing', 'warp', 'presenting_adapter',
                'power_source', 'power_mode', 'msaa'):
        controls[key] = environment.get(key)
    for key, fields in (('display', ('width', 'height', 'refresh_hz')),
                        ('backbuffer', ('width', 'height')),
                        ('declared', ('frame_generation', 'upscaling', 'driver_vsync', 'vendor_mode'))):
        value = environment.get(key)
        for field in fields:
            controls[f'{key}.{field}'] = value.get(field) if isinstance(value, dict) else None
    return controls


def control_known(name, value, controls):
    if name in ('build_identity', 'presenting_adapter', 'declared.vendor_mode'):
        return (isinstance(value, str) and bool(value.strip())
                and (name != 'presenting_adapter' or 'version unavailable' not in value.casefold()))
    if name == 'camera':
        return camera_known(value)
    if name == 'turn_frames':
        return integer(value) and value >= 2
    if name == 'cycle_frames':
        T = controls['turn_frames']
        return integer(value) and value > 0 and integer(T) and T >= 2 and value % (2 * T) == 0
    if name == 'presentation_interval':
        return integer(value) and value >= 0
    if name in ('vsync', 'tearing', 'warp', 'declared.frame_generation', 'declared.upscaling', 'declared.driver_vsync'):
        return type(value) is bool
    if name == 'power_source':
        return value == 'mains'
    if name == 'power_mode':
        return isinstance(value, str) and value not in ('changed', 'unknown')
    # MSAA, display dimensions/refresh rate and backbuffer dimensions.
    return integer(value) and value >= 1


def compare_controls(baseline, group, variant):
    controls = baseline + group
    unknown = {name for values in controls for name, value in values.items()
               if not control_known(name, value, values)}
    differing = set()
    for name in (controls[0] if controls else {}):
        if name in unknown:
            continue
        if name == 'msaa':
            if (any(values[name] != 1 for values in baseline)
                    or any(values[name] != (4 if variant == 'msaa4' else 1) for values in group)):
                differing.add(name)
        elif any(values[name] != controls[0][name] for values in controls[1:]):
            differing.add(name)
    return sorted(unknown), sorted(differing)


def gate_fields(scene, builds, expected):
    empty = {'gate_runs': 0, 'fps': None, 'mean_frame_ms': None, 'p99_ms': None, 'variant_verdict': 'no-data'}
    if scene is None or not scene['runs']:
        return empty, []
    if expected is None or builds != {expected}:
        return dict.fromkeys(empty), ['gate-other-build']
    pooled = scene['pooled']
    return {'gate_runs': len(scene['runs']), 'fps': pooled['fps'], 'mean_frame_ms': 1000 / pooled['fps'],
            'p99_ms': pooled['p99_ms'], 'variant_verdict': scene['verdict']}, []


def summarize(directories, timing='presentmon', candidate='s-b'):
    if not isinstance(candidate, str) or NAME.fullmatch(candidate) is None:
        raise ValueError('bad-candidate')
    directories = list(map(Path, directories))
    # Join on raw identifiers: sanitized ones can collide (two candidates or run IDs that
    # contain the user name both read 'redacted'). Only the complete table is sanitized.
    gate = renderer_gate.summarize_private(directories, timing=timing)
    scenes = {(scene['scene'], scene['feature']): scene for scene in gate['scenes']
              if scene['candidate'] == candidate}
    valid_ids = {key: {(r['run_id'], r['build_identity']) for r in scene['runs']} for key, scene in scenes.items()}
    variants = ('none', 'no-gaps', *FEATURES[1:])
    groups = {variant: {'accepted': [], 'controls': [], 'values': [], 'runs': 0, 'cycles': 0, 'reasons': set()}
              for variant in variants}
    gate_builds = {variant: set() for variant in variants}
    cost_invalid = []
    for directory in directories:
        try:
            run = read_json(directory / 'run.json')
            validate_run(run)
        except RunError:
            continue
        variant = run.get('feature', 'none')
        if run['candidate'] != candidate or variant not in groups or run['scene'] not in ('w3', 'w3f'):
            continue
        if (run['run_id'], run['build']['build_identity']) not in valid_ids.get((run['scene'], variant), set()):
            continue
        if run['scene'] == 'w3':
            gate_builds[variant].add(run['build']['build_identity'])
            continue
        values, ticks, _, reasons = run_frames(directory, run, CAPTURE_COLUMNS, timing)
        entries = read_trace(directory)
        # The gate accepted this run; its imported rules are applied again as a guard.
        # W3f never uses the W3 clock check.
        reasons += condition_reasons(run.get('environment'))
        reasons += trace_step_reasons(entries, run, ticks)
        window = run.get('window', {})
        if (reasons or run['label_check']['status'] != 'pass'
                or (timing == 'trace' and (not isinstance(window, dict)
                    or window.get('visible_throughout') is not True or window.get('foreground_throughout') is not True))):
            continue
        group = groups[variant]
        group['accepted'].append(run)
        group['controls'].append(control_values(run))
        values, cycles, reasons = cost_window(run, entries, values, ticks, timing)
        if reasons:
            cost_invalid.append({'run_id': run['run_id'], 'candidate': candidate, 'scene': 'w3f',
                                 'feature': variant, 'reasons': reasons})
            group['reasons'].update(reasons)
        else:
            group['runs'] += 1
            group['cycles'] += cycles
            group['values'].extend(values)
    for group in groups.values():
        builds = {run['build']['build_identity'] for run in group['accepted']}
        group['build_identity'] = next(iter(builds)) if len(builds) == 1 else None
        if len(builds) > 1:
            group['reasons'].add('mixed-builds')
        group.update(paired_statistics(group['values']))
    baseline = groups['none']

    def row(feature, variant):
        group = groups[variant]
        reasons = set()
        unknown, differing = [], []
        if not group['accepted']:
            status = 'unmeasured'
        else:
            if not group['values']:
                reasons.update(group['reasons'])
            if not baseline['values']:
                reasons.add('baseline-no-cost-frames')
            reasons.update(reason for g in (baseline, group) for reason in g['reasons'] if reason == 'mixed-builds')
            unknown, differing = compare_controls(baseline['controls'], group['controls'], variant)
            if unknown:
                reasons.add('controls-unknown')
            if differing:
                reasons.add('controls-differ')
            status = ('not-comparable' if reasons else 'preliminary'
                      if timing == 'trace' or min(baseline['runs'], group['runs']) < RUNS_MIN else 'measured')
        comparable = status in ('measured', 'preliminary') and variant != 'none'
        sign = -1 if feature == 'gaps' else 1
        expected = group['build_identity'] if group['accepted'] else baseline['build_identity']
        fields, gate_reasons = gate_fields(scenes.get(('w3', variant)), gate_builds[variant], expected)
        verdict = fields['variant_verdict']
        if feature == 'gaps':
            on, on_reasons = gate_fields(scenes.get(('w3', 'none')), gate_builds['none'], expected)
            verdict = on['variant_verdict']
            gate_reasons += on_reasons
        reasons.update(gate_reasons)
        return {'feature': feature, 'variant': variant, 'runs': group['runs'], 'build_identity': group['build_identity'],
                'cycles': group['cycles'], 'paired_mean_ms': group['paired_mean_ms'], 'paired_p99_ms': group['paired_p99_ms'],
                'cost_mean_ms': sign * (group['paired_mean_ms'] - baseline['paired_mean_ms']) if comparable else None,
                'cost_p99_ms': sign * (group['paired_p99_ms'] - baseline['paired_p99_ms']) if comparable else None,
                **fields, 'verdict': verdict, 'breaks_gate': True if verdict == 'not-met' else False if verdict == 'met' else None,
                'status': status, 'reasons': sorted(reasons), 'unknown_controls': unknown, 'differing_controls': differing}

    controls = {name: values if all(c[name] == values for c in baseline['controls']) else None
                for name, values in (baseline['controls'][0].items() if baseline['controls'] else [])}
    return sanitize({'format': 'magic600-h06-feature-costs-v1',
                     'timing_source': 'probe-trace' if timing == 'trace' else 'presentmon',
                     'preliminary': timing == 'trace', 'candidate': candidate, 'method': METHOD,
                     'turn_frames': controls.get('turn_frames'), 'cycle_frames': controls.get('cycle_frames'),
                     'controls': controls, 'baseline': row('none', 'none'),
                     'features': [row(feature, 'no-gaps' if feature == 'gaps' else feature) for feature in FEATURES],
                     'invalid_runs': gate['invalid_runs'], 'cost_invalid_runs': cost_invalid, 'unreadable': gate['unreadable']})


def render_markdown(result):
    build = result['baseline']['build_identity']
    lines = ['Timing source: ' + result['timing_source'],
             'Build identity: ' + markdown_text(build[:12] if build else 'unknown'),
             'costs are paired: every variant draws the same W3 states (scene w3f), in whole cycles; gate fields come from W3 runs',
             'unmeasured features are not estimated; not-comparable features have no cost', '',
             '| Feature | Variant | Runs | Build | Cycles | Paired mean ms | Paired p99 ms | Cost mean ms | Cost p99 ms | Gate runs | W3 fps | W3 mean ms | W3 p99 ms | Variant verdict | Feature-on verdict | Breaks gate | Status | Reasons |',
             '| ' + ' | '.join(['---'] * 18) + ' |']
    keys = ('feature', 'variant', 'runs', 'build_identity', 'cycles', 'paired_mean_ms', 'paired_p99_ms', 'cost_mean_ms', 'cost_p99_ms',
            'gate_runs', 'fps', 'mean_frame_ms', 'p99_ms', 'variant_verdict', 'verdict', 'breaks_gate', 'status')
    for row in [result['baseline'], *result['features']]:
        cells = []
        for key in keys:
            value = row[key]
            cells.append('-' if value is None else f'{value:.3f}' if type(value) is float else markdown_text(value))
        cells.append(markdown_text(', '.join(row['reasons'] + row['unknown_controls'] + row['differing_controls'])) or '-')
        lines.append('| ' + ' | '.join(cells) + ' |')
    return '\n'.join(lines) + '\n'


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directories', type=Path, nargs='+')
    parser.add_argument('--timing', choices=('presentmon', 'trace'), default='presentmon')
    parser.add_argument('--candidate', default='s-b')
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--markdown', type=Path)
    args = parser.parse_args(argv)
    outputs = [args.out] + ([args.markdown] if args.markdown else [])
    if any(path.exists() for path in outputs):
        parser.error('existing-output')
    if len({path.resolve() for path in outputs}) != len(outputs):
        parser.error('duplicate-output')
    try:
        result = summarize(args.directories, timing=args.timing, candidate=args.candidate)
    except ValueError as error:
        parser.error(str(error))
    if len(result['unreadable']) == len(args.directories):
        print('no-readable-runs', file=sys.stderr)
        return 1
    contents = [json.dumps(result, indent=2, ensure_ascii=False, allow_nan=False) + '\n']
    if args.markdown:
        contents.append(render_markdown(result))
    created = []
    try:
        with ExitStack() as stack:
            streams = []
            for path in outputs:
                streams.append(stack.enter_context(path.open('x', encoding='utf-8', newline='\n')))
                created.append(path)
            for stream, content in zip(streams, contents):
                stream.write(content)
    except OSError:
        for path in created:
            path.unlink()
        parser.error('cannot-create-output')
    return 0


if __name__ == '__main__':
    sys.exit(main())
