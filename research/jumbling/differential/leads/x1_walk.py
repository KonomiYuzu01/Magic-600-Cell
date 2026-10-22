"""Time the *recorded measurement walk*, never a product cap-selection aid.

python -B research/jumbling/differential/leads/x1_walk.py --out DIR [--steps N ...]
    [--jobs K] [--rest PATH]

The default covers all 201 requests of seed 7101. --rest supplies the density
file for the other 48 repair steps; those comparisons are explicitly absent
without it. Workers split steps, replay with census.walk and time their own
work only. Output is refused anywhere inside the repository.
"""

import sys

sys.dont_write_bytecode = True

import argparse
from concurrent.futures import ProcessPoolExecutor
import gzip
import json
import multiprocessing
from pathlib import Path
from time import perf_counter

if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from differential.leads import census
from differential.leads.repair_list import (compare_recorded, output_directory,
                                            repair_list, run_metadata)

FINDABILITY = census.CENSUS / 'runs' / 'findability'
DENSITY = FINDABILITY / 'ia-k40-s7101-density.jsonl.gz'


def step_records():
    path = FINDABILITY / 'ia-k40-s7101-steps.jsonl'
    return {row['step']: row for line in path.read_text(encoding='utf-8').splitlines()
            if (row := json.loads(line)).get('rule') == 'conj'}


def density_records(rest=None):
    records = {}
    for path in [DENSITY] + ([Path(rest)] if rest is not None else []):
        with gzip.open(path, 'rt', encoding='utf-8') as source:
            for line in source:
                row = json.loads(line)
                if 'x' in row:
                    records.setdefault((row['step'], row['c']), []).append(row)
    return records


def walk_caps(S, repair_cap):
    """The census input's decreasing-N_c walk, through its recorded repair cap."""
    survey = S.survey()
    order = sorted(survey, key=lambda c: (-survey[c][0], c))
    return order[:order.index(repair_cap) + 1]


def _expected_pairs(record):
    # findability's pairs_before includes the partial walk at the repair cap.
    # Remove that partial cap and add its entire 59-opening list instead.
    before = (record['pairs_before']
              - (record['x_rank_in_menu'] - 1) * record['neighbours_of_c']
              - (record['index_d'] - 1))
    return before + 59 * record['neighbours_of_c']


def measure_step(step, *, max_pairs=None, recorded=None, density=None):
    """Measure one census Step; max_pairs is only for the bounded test smoke."""
    started = perf_counter()
    if step.rule != 'conj':
        raise ValueError(f'step {step.n} is not a repair step')
    recorded = step_records()[step.n] if recorded is None else recorded
    S = step.S
    (c, x), (d, y), (c2, z) = step.word
    if c2 != c or [list(v) for v in step.word] != recorded['word']:
        raise AssertionError(f'step {step.n}: the census word differs from findability')
    caps = walk_caps(S, c)
    order_seconds = perf_counter() - started
    requests, results = [], []
    for place, cap in enumerate(caps, 1):
        result = repair_list(S, cap, max_pairs=max_pairs)
        if result.status == 'error':
            raise AssertionError(f'step {step.n}, cap {cap}: {result.message}')
        pairs, seconds = result.coverage.pairs_tried, result.timings.total
        requests.append({'step': step.n, 'cap': cap, 'place': place,
                         'N_c': S.scores(cap)[0],
                         'meeting_misaligned_caps': list(result.coverage.meeting_caps),
                         'pairs': pairs, 'listed_rows': result.rows, 'status': result.status,
                         'input_digest': result.input_digest,
                         'seconds': seconds, 'seconds_per_pair': seconds / pairs if pairs else None})
        results.append(result)
    complete = all(r.status == 'completed' for r in results)
    prior = results[:-1]
    prior_empty = (all(not r.rows for r in prior)
                   if all(r.status == 'completed' for r in prior) else None)
    own = (x, d, y, z, step.record['N_after'] - step.record['N_before'])
    first = bool(results[-1].rows and results[-1].rows[0] == own) if results[-1].status == 'completed' else None
    density_ok = None
    if density is not None and (rows := density.get((step.n, c))) is not None:
        if results[-1].status == 'completed':
            compare_recorded(results[-1], rows, step=step.n)
            density_ok = True
    pairs = sum(r['pairs'] for r in requests)
    seconds = sum(r['seconds'] for r in requests)
    checks = {'request_count': len(requests) == recorded['rank_c'],
              'repair_cap_place': len(caps) == recorded['rank_c'],
              'pair_count': pairs == _expected_pairs(recorded) if complete else None,
              'before_completed_empty': prior_empty, 'record_pair_first': first,
              'density_outcomes': density_ok}
    if any(value is False for value in checks.values()):
        raise AssertionError(f'step {step.n}: walk checks failed: {checks}')
    return requests, {'step': step.n, 'requests': len(requests), 'pairs': pairs,
                      'expected_pairs': _expected_pairs(recorded), 'seconds': seconds,
                      'seconds_per_pair': seconds / pairs if pairs else None,
                      'order_seconds': order_seconds, 'wall_seconds': perf_counter() - started,
                      'listed_rows': sum(len(r.rows or ()) for r in results),
                      'status': 'completed' if complete else 'cut_off', 'checks': checks}


def summarize(requests, per_step, *, records=None):
    records = step_records() if records is None else records
    selected = {s['step'] for s in per_step}
    complete = all(r['status'] == 'completed' for r in requests)
    all_steps = selected == set(records)
    expected_requests = 201 if all_steps else sum(records[n]['rank_c'] for n in selected)
    expected_pairs = 128502 if all_steps else sum(_expected_pairs(records[n]) for n in selected)
    pairs = sum(r['pairs'] for r in requests)
    seconds = sum(r['seconds'] for r in requests)

    def aggregate(key):
        values = [s['checks'][key] for s in per_step]
        return all(values) if all(v is not None for v in values) else None

    checks = {'requests': len(requests) == expected_requests,
              'pairs': pairs == expected_pairs if complete else None,
              'repair_cap_places': aggregate('repair_cap_place'),
              'before_completed_empty': aggregate('before_completed_empty'),
              'record_pair_first': aggregate('record_pair_first'),
              'all_completed': complete,
              'density_steps_checked': [s['step'] for s in per_step if s['checks']['density_outcomes'] is True],
              'density_steps_unchecked': [s['step'] for s in per_step if s['checks']['density_outcomes'] is None]}
    if checks['requests'] is False or checks['pairs'] is False:
        raise AssertionError(f'walk totals differ from the recorded input: {checks}')
    return {'status': 'completed' if complete else 'cut_off', 'per_step': per_step,
            'overall': {'steps': len(selected), 'requests': len(requests), 'pairs': pairs,
                        'listed_rows': sum(s['listed_rows'] for s in per_step), 'seconds': seconds,
                        'seconds_per_pair': seconds / pairs if pairs else None},
            'expected': {'requests': expected_requests, 'pairs': expected_pairs}, 'checks': checks}


def measure_walk(steps, *, max_pairs=None, rest=None):
    """Consume census.walk states, leaving each one unchanged for its replay."""
    records, density = step_records(), density_records(rest)
    requests, per_step = [], []
    for step in steps:
        if step.rule == 'conj':
            lines, summary = measure_step(step, max_pairs=max_pairs,
                                          recorded=records[step.n], density=density)
            requests.extend(lines)
            per_step.append(summary)
    return requests, summarize(requests, per_step, records=records)


def _worker(arguments):
    selected, rest, max_pairs = arguments
    started = perf_counter()
    states = (s for s in census.walk(7101, last=max(selected)) if s.n in selected)
    requests, summary = measure_walk(states, max_pairs=max_pairs, rest=rest)
    return requests, summary['per_step'], perf_counter() - started


def run_walk(*, steps=None, jobs=1, rest=None, max_pairs=None):
    records = step_records()
    selected = sorted(records if steps is None else set(steps))
    if not selected or set(selected) - set(records):
        raise ValueError('--steps must name repair steps of the seed 7101 record')
    if type(jobs) is not int or jobs < 1:
        raise ValueError('--jobs must be a positive integer')
    count = min(jobs, len(selected))
    arguments = [(selected[k::count], rest, max_pairs) for k in range(count)]
    if count == 1:
        measured = [_worker(arguments[0])]
    else:
        with ProcessPoolExecutor(max_workers=count, mp_context=multiprocessing.get_context('spawn')) as pool:
            measured = list(pool.map(_worker, arguments))
    requests = sorted([r for lines, _, _ in measured for r in lines], key=lambda r: (r['step'], r['place']))
    per_step = sorted([s for _, summaries, _ in measured for s in summaries], key=lambda s: s['step'])
    summary = summarize(requests, per_step, records=records)
    summary.update(run_metadata())
    summary['jobs'] = count
    summary['worker_seconds_including_replay'] = [seconds for _, _, seconds in measured]
    return requests, summary


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True)
    parser.add_argument('--steps', type=int, nargs='+')
    parser.add_argument('--jobs', type=int, default=1)
    parser.add_argument('--rest')
    args = parser.parse_args(argv)
    try:
        directory = output_directory(args.out)
        requests, summary = run_walk(steps=args.steps, jobs=args.jobs, rest=args.rest)
    except ValueError as error:
        parser.error(str(error))
    directory.mkdir(parents=True, exist_ok=True)
    with (directory / 'x1-requests.jsonl').open('w', encoding='utf-8') as target:
        for row in requests:
            target.write(json.dumps(row) + '\n')
    (directory / 'x1-summary.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()
