"""Summarize B4-12 run records without changing inputs or collecting measurements.

PresentMon mapping keys: process, swap_chain, time, frame_interval, display_interval.
Context covered_seconds is the first-to-last retained timestamp span. A formal
series has at least three counted runs, each contributing exactly 100 samples.
"""
from __future__ import annotations

import argparse
import csv
import json
import math
import os
import re
import socket
import sys
from contextlib import ExitStack
from pathlib import Path

SERIES = ('m1-active', 'm1-all', 'm2-bank', 'm2-local', 'm3')
METHOD = ('nearest-rank percentiles; no outlier removal; '
          'a target is met only if the pooled value and every run meet it')
PRESENTMON_COLUMNS = {
    'process': 'ProcessID', 'swap_chain': 'SwapChainAddress', 'time': 'CPUStartQPC',
    'frame_interval': 'MsBetweenPresents', 'display_interval': 'MsBetweenDisplayChange',
}
PUBLIC_SCALARS = (
    'build_identity', 'executable_sha256', 'package_sha256', 'presenting_adapter',
    'cpu_name', 'ram_bytes', 'presentation_interval', 'power_source', 'power_mode',
)
PUBLIC_OBJECTS = {
    'display': ('width', 'height', 'refresh_hz', 'dpi'),
    'backbuffer': ('width', 'height'),
    'declared': ('vendor_mode', 'frame_generation', 'driver_vsync', 'overlays'),
}


class RunError(ValueError):
    """A fixed, publishable reason code for an unreadable run."""


def require(condition, reason='bad-field'):
    if not condition:
        raise RunError(reason)


def integer(value):
    return type(value) is int


def number(value):
    if type(value) not in (int, float):
        return False
    try:
        return math.isfinite(value)
    except OverflowError:
        return False


def read_json(path):
    try:
        with path.open(encoding='utf-8') as stream:
            return json.load(stream)
    except (ValueError, UnicodeError):
        raise RunError('bad-json') from None
    except OSError:
        raise RunError('missing-file') from None


def validate_run(run):
    require(isinstance(run, dict))
    require(run.get('format') == 'magic600-b412-run-v1', 'bad-format')
    require(isinstance(run.get('run_id'), str))
    require(run.get('series') in SERIES)
    require(type(run.get('attribution')) is bool)
    require(run.get('status') in ('valid', 'invalid'))
    reasons = run.get('invalid_reasons')
    require(isinstance(reasons, list) and all(isinstance(reason, str) for reason in reasons))
    require(integer(run.get('qpc_frequency')) and run['qpc_frequency'] > 0)
    require(integer(run.get('warmups')) and run['warmups'] >= 0)
    require(integer(run.get('measured')) and run['measured'] >= 0)
    require(isinstance(run.get('build'), dict) and isinstance(run['build'].get('build_identity'), str))
    if run['series'] == 'm3':
        rotation = run.get('rotation')
        require(isinstance(rotation, dict))
        require(integer(rotation.get('start_qpc')) and integer(rotation.get('stop_qpc')))
        require(rotation['stop_qpc'] >= rotation['start_qpc'])
        require(isinstance(run.get('presentmon'), dict))
        require(integer(run['presentmon'].get('process_id')))
    else:
        require(run['measured'] > 0)


def public_environment(environment):
    require(isinstance(environment, dict))

    def leaf(value):
        if isinstance(value, list):
            return [leaf(item) for item in value]
        require(value is None or type(value) in (str, bool) or number(value))
        return value

    def fields(value, allowed):
        require(isinstance(value, dict))
        return {key: leaf(value[key]) for key in allowed if key in value}

    public = {key: leaf(environment[key]) for key in PUBLIC_SCALARS if key in environment}
    for key, allowed in PUBLIC_OBJECTS.items():
        if key in environment:
            public[key] = fields(environment[key], allowed)
    if 'gpus' in environment:
        require(isinstance(environment['gpus'], list))
        public['gpus'] = [fields(gpu, ('name', 'driver_version')) for gpu in environment['gpus']]
    if 'gpu_processes_at_start' in environment:
        value = environment['gpu_processes_at_start']
        require(integer(value) and value >= 0)
        public['gpu_processes_at_start'] = value
    return public


def read_samples(directory, run):
    try:
        with (directory / 'samples.jsonl').open(encoding='utf-8') as stream:
            samples = [json.loads(line) for line in stream]
    except (ValueError, UnicodeError):
        raise RunError('bad-json') from None
    except OSError:
        raise RunError('missing-file') from None
    for sample in samples:
        require(isinstance(sample, dict))
        require(sample.get('format') == 'magic600-b412-sample-v1', 'bad-format')
        require(sample.get('series') == run['series'])
        require(integer(sample.get('index')))
        require(type(sample.get('warmup')) is bool)
        require(number(sample.get('ms')) and sample['ms'] >= 0)
        require(integer(sample.get('input_ticks')) and integer(sample.get('end_ticks')))
        expected_ms = (sample['end_ticks'] - sample['input_ticks']) * 1000 / run['qpc_frequency']
        require(math.isclose(sample['ms'], expected_ms, rel_tol=1e-6, abs_tol=0), 'sample-ms-mismatch')
    require(len(samples) == run['warmups'] + run['measured'], 'sample-count-mismatch')
    require(sum(sample['warmup'] for sample in samples) == run['warmups'], 'sample-count-mismatch')
    for index, sample in enumerate(samples):
        require(sample['index'] == index and sample['warmup'] == (index < run['warmups']), 'sample-order')
    return [sample['ms'] for sample in samples if not sample['warmup']]


def nearest_rank(values, percentile):
    ordered = sorted(values)
    rank = ((999 * len(ordered) + 999) // 1000 if percentile == 99.9
            else (percentile * len(ordered) + 99) // 100)
    return ordered[rank - 1]


def statistics(values):
    return {'n': len(values), 'mean': math.fsum(values) / len(values),
            'p50': nearest_rank(values, 50), 'p95': nearest_rank(values, 95),
            'p99': nearest_rank(values, 99), 'max': max(values)}


def context_statistics(frames, frequency):
    if not frames:
        return {'n': 0, 'covered_seconds': 0, 'mean_fps': None,
                'p50': None, 'p95': None, 'p99': None, 'max': None}
    stats = statistics([interval for _, interval in frames])
    mean = stats.pop('mean')
    return {**stats, 'covered_seconds': (frames[-1][0] - frames[0][0]) / frequency,
            'mean_fps': 1000 / mean if mean else None}


def csv_integer(value):
    require(isinstance(value, str) and re.fullmatch(r'[+-]?\d+', value.strip()) is not None)
    return int(value)


def csv_interval(value):
    try:
        result = float(value)
    except (ValueError, TypeError):
        raise RunError('bad-field') from None
    require(number(result) and result >= 0)
    return result


def read_presentmon(directory, run, columns):
    start, stop = run['rotation']['start_qpc'], run['rotation']['stop_qpc']
    chains = {}
    try:
        with (directory / 'presentmon.csv').open(encoding='utf-8-sig', newline='') as stream:
            reader = csv.DictReader(stream, strict=True)
            header = reader.fieldnames or []
            require(all(columns[key] in header for key in
                        ('process', 'swap_chain', 'time', 'frame_interval')), 'presentmon-missing-column')
            display_available = columns['display_interval'] in header
            for row in reader:
                if csv_integer(row.get(columns['process'])) != run['presentmon']['process_id']:
                    continue
                ticks = csv_integer(row.get(columns['time']))
                if start <= ticks <= stop:
                    chain = row.get(columns['swap_chain'])
                    require(isinstance(chain, str) and bool(chain.strip()))
                    chains.setdefault(chain, []).append((ticks, row))
    except OSError:
        raise RunError('missing-file') from None
    except (csv.Error, UnicodeError):
        raise RunError('bad-field') from None
    require(bool(chains), 'presentmon-no-rows')
    # Ties are deterministic; counts include the complete marker interval.
    chosen = min(chains, key=lambda chain: (-len(chains[chain]), chain))
    ignored = [{'swap_chain': chain, 'rows': len(chains[chain])}
               for chain in sorted(chains) if chain != chosen]
    boundary = start + 5 * run['qpc_frequency']
    kept = [(ticks, row) for ticks, row in sorted(chains[chosen], key=lambda frame: frame[0])
            if ticks >= boundary]
    frames = [(ticks, csv_interval(row.get(columns['frame_interval']))) for ticks, row in kept]
    if len(frames) < 100:
        return None, [], ['m3-too-few-frames']
    measured = [interval for _, interval in frames[:100]]
    mean = math.fsum(measured) / 100
    require(mean > 0)
    context_stop = min(stop, boundary + 60 * run['qpc_frequency'])
    context = context_statistics([(ticks, interval) for ticks, interval in frames
                                  if ticks <= context_stop], run['qpc_frequency'])
    if display_available:
        display, missing = [], 0
        for ticks, row in kept:
            if ticks > context_stop:
                break
            value = row.get(columns['display_interval'])
            try:
                interval = float(value)
            except (ValueError, TypeError):
                missing += 1
                continue
            if not number(interval):
                missing += 1
                continue
            require(interval >= 0)
            display.append((ticks, interval))
        context['display'] = context_statistics(display, run['qpc_frequency'])
        context['display_missing'] = missing
    stats = {'fps': 1000 / mean, 'm3b_p99_ms': nearest_rank(measured, 99),
             'context': context, 'swap_chains_ignored': ignored}
    return stats, measured, []


def spread(values):
    if len(values) < 2:
        return None, False
    if min(values) == 0:
        return None, True
    difference = max(values) / min(values) - 1
    return difference, difference > 0.20


def sanitize(result):
    private = [os.environ.get(key, '') for key in ('USERNAME', 'USER', 'COMPUTERNAME', 'HOSTNAME')]
    private.append(socket.gethostname())
    private = [value.casefold() for value in private if len(value) >= 3]
    paths = re.compile(r'[A-Za-z]:[\\/]|^\\\\|(?<!\w)/[A-Za-z]')
    redactions = 0

    def clean(value):
        nonlocal redactions
        if isinstance(value, str):
            if paths.search(value) or any(token in value.casefold() for token in private):
                redactions += 1
                return 'redacted'
        elif isinstance(value, list):
            return [clean(item) for item in value]
        elif isinstance(value, dict):
            return {key: clean(item) for key, item in value.items()}
        return value

    result = clean(result)
    result['redactions'] = redactions
    return result


def summarize(directories, columns=None):
    columns = {**PRESENTMON_COLUMNS, **(columns or {})}
    result = {'format': 'magic600-b412-summary-v1', 'method': METHOD,
              'series': {}, 'invalid_runs': [], 'unreadable': [],
              'attribution_runs': [], 'environment': []}
    samples = {series: [] for series in SERIES}
    builds = {}
    for series in SERIES:
        result['series'][series] = {'runs': [], 'pooled': None, 'spread': None,
                                   'needs_extra_run': False,
                                   'target': ('fps >= 30' if series == 'm3' else
                                              'p95 <= 100 ms' if series.startswith('m1-') else 'p95 < 50 ms'),
                                   'verdict': 'no-data', 'formal': False}
    result['series']['m3']['m3b_verdict'] = 'no-data'
    for directory in directories:
        directory = Path(directory)
        try:
            run = read_json(directory / 'run.json')
            validate_run(run)
            environment = public_environment(read_json(directory / 'environment.json'))
            if run['series'] == 'm3':
                stats, values, extra_reasons = read_presentmon(directory, run, columns)
            else:
                values = read_samples(directory, run)
                stats, extra_reasons = statistics(values), []
        except RunError as error:
            result['unreadable'].append({'directory': directory.name, 'reason': str(error)})
            continue
        series, identity = run['series'], run['build']['build_identity']
        if series in builds and builds[series] != identity:
            raise ValueError('mixed-builds')
        builds[series] = identity
        result['series'][series]['build_identity'] = identity
        result['environment'].append({'run_id': run['run_id'], **environment})
        if run['status'] == 'invalid' or extra_reasons:
            reasons = list(run['invalid_reasons'])
            reasons.extend(reason for reason in extra_reasons if reason not in reasons)
            result['invalid_runs'].append({'run_id': run['run_id'], 'series': series, 'reasons': reasons})
        elif run['attribution']:
            result['attribution_runs'].append({'run_id': run['run_id'], 'series': series, **stats})
        else:
            result['series'][series]['runs'].append({'run_id': run['run_id'], **stats})
            samples[series].append(values)
    for series, item in result['series'].items():
        if not item['runs']:
            continue
        values = [value for run in samples[series] for value in run]
        item['formal'] = len(item['runs']) >= 3 and all(len(run) == 100 for run in samples[series])
        if series == 'm3':
            item['pooled'] = {'n': len(values), 'fps': 1000 / (math.fsum(values) / len(values)),
                              'm3b_p99_ms': nearest_rank(values, 99)}
            key, target = 'fps', lambda value: value >= 30
            item['m3b_verdict'] = 'report'
        else:
            item['pooled'] = statistics(values)
            key = 'p95'
            target = (lambda value: value <= 100) if series.startswith('m1-') else (lambda value: value < 50)
        run_values = [run[key] for run in item['runs']]
        item['spread'], item['needs_extra_run'] = spread(run_values)
        item['verdict'] = 'met' if target(item['pooled'][key]) and all(map(target, run_values)) else 'not-met'
    return sanitize(result)


def markdown_text(value):
    return ' '.join(str(value).split()).replace('\\', '\\\\').replace('|', '\\|')


def render_markdown(result):
    count = max(3, *(len(item['runs']) for item in result['series'].values()))
    headers = ['Metric', *(f'Run {index} p95' for index in range(1, count + 1)),
               'Pooled p95 (n=…)', 'p99', 'Target', 'Met?']
    lines = ['| ' + ' | '.join(headers) + ' |', '| ' + ' | '.join(['---'] * len(headers)) + ' |']
    rows = [('m1-active', 'M1 instant turn (active orbit)', False),
            ('m1-all', 'M1 instant turn (all pieces)', False),
            ('m2-bank', 'M2 bank navigation', False),
            ('m2-local', 'M2 Local-centre navigation', False),
            ('m3', 'M3 full-detail fps (mean)', False), ('m3', 'M3b frame time p99', True)]
    for series, label, m3b in rows:
        item = result['series'][series]
        fps = series == 'm3' and not m3b
        key = 'fps' if fps else 'm3b_p99_ms' if m3b else 'p95'
        decimals = 1 if fps else 2
        cells = [label, *[f'{run[key]:.{decimals}f}' for run in item['runs']]]
        cells.extend(['–'] * (count - len(item['runs'])))
        pooled = item['pooled']
        if pooled:
            cells.append(f'{pooled[key]:.{decimals}f} (n={pooled["n"]})')
            p99 = pooled['m3b_p99_ms'] if series == 'm3' else pooled['p99']
            cells.append(f'{p99:.2f}')
        else:
            cells.extend(['–', '–'])
        cells.extend(['report' if m3b else item['target'], item['m3b_verdict'] if m3b else item['verdict']])
        lines.append('| ' + ' | '.join(cells) + ' |')
    lines.extend(['', 'Method: ' + METHOD + '.', ''])
    for series, item in result['series'].items():
        identity = markdown_text(item.get('build_identity', '–'))
        statement = 'formal 3 x 100 result' if item['formal'] else 'baseline, not a formal 3 x 100 result'
        lines.append(f'{series}: {len(item["runs"])} counted runs; build identity: {identity}; {statement}.')
        if item['needs_extra_run']:
            lines.append(f'{series}: needs one extra run (run spread greater than 20% or a zero minimum).')
    lines.append('')
    for run in result['invalid_runs']:
        reasons = ', '.join(markdown_text(reason) for reason in run['reasons'])
        lines.append(f'Invalid {markdown_text(run["run_id"])}: {reasons}')
    for run in result['unreadable']:
        lines.append(f'Unreadable {markdown_text(run["directory"])}: {run["reason"]}')
    return '\n'.join(lines) + '\n'


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directories', type=Path, nargs='+')
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--markdown', type=Path)
    parser.add_argument('--presentmon-columns', type=Path)
    args = parser.parse_args(argv)
    outputs = [args.out] + ([args.markdown] if args.markdown else [])
    if any(path.exists() for path in outputs):
        parser.error('existing-output')
    if len({path.resolve() for path in outputs}) != len(outputs):
        parser.error('duplicate-output')
    columns = None
    if args.presentmon_columns:
        try:
            columns = read_json(args.presentmon_columns)
            require(isinstance(columns, dict))
            require(all(key in PRESENTMON_COLUMNS and isinstance(value, str) and value.strip()
                        for key, value in columns.items()))
        except RunError:
            parser.error('bad-presentmon-columns')
    try:
        result = summarize(args.directories, columns)
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
