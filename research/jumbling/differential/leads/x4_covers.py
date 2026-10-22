"""X4: exact covers versus the research float relation and recorded flags.

Run with python -B and --out outside the repository. --jobs parallelizes
independent census records. These cloud research timings are cost leads.
"""

import sys

sys.dont_write_bytecode = True

import argparse
from collections import Counter
from concurrent.futures import ProcessPoolExecutor
from dataclasses import asdict
import hashlib
import json
from pathlib import Path
from statistics import median
from time import perf_counter

if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from differential.leads import census
from differential.leads import covers
from differential.leads.xm1_seats import provenance

import numpy as np
import seams

FLAGS_PATH = census.CENSUS / 'runs/findability/ia-k40-s7101-steps.jsonl'


class RecordedFlagError(AssertionError):
    """The research float test did not reproduce a recorded flag."""


def recorded_flags():
    rows = [json.loads(line) for line in FLAGS_PATH.read_text(encoding='utf-8').splitlines()]
    repairs = {row['step']: row for row in rows if row.get('rule') == 'conj'}
    assert len(repairs) == 63, 'expected the 63 seed-7101 repair flags'
    assert sum(r['c_covers_d'] for r in repairs.values()) == 30
    assert sum(r['d_covers_c'] for r in repairs.values()) == 26
    return repairs


def disagreement_trace(S, c, d, exact, pos, inside_d):
    """Trace a differing 'd covers c' answer to individual membership decisions."""
    P = S.face(c)[0]
    exact_inside = np.isin(P, inside_d.pieces)
    margins = pos[P] @ seams.NF[d] - seams.KAPPA
    differing = exact_inside != (margins > 0)
    straddlers = set(exact.straddlers)
    pieces = [{'piece': int(p), 'exact_inside': bool(e), 'float_inside': bool(m > 0),
               'float_margin': float(m), 'straddles_d': int(p) in straddlers}
              for p, e, m in zip(P[differing], exact_inside[differing], margins[differing])]
    if inside_d.blocked and pieces and all(p['straddles_d'] for p in pieces):
        reason = 'blocked_d'
    elif not inside_d.blocked and pieces and all(abs(p['float_margin']) < 1e-9 for p in pieces):
        reason = 'rounding'
    else:
        reason = 'unexplained'
    return {'reason': reason, 'blocked_d': inside_d.blocked, 'differing_pieces': pieces,
            'face_straddlers_of_d': sorted(straddlers)}


def compare_pair(S, c, d, pos, inside_d):
    """One directional relation, with a trace only when the truth values differ."""
    exact = covers.exact_covers(S, c, d, inside_d)
    floating = covers.float_covers(S, c, d, pos)
    agrees = exact.covers == floating.covers
    return {'covered': c, 'coverer': d, 'exact': asdict(exact), 'float': asdict(floating),
            'agrees': agrees,
            'trace': None if agrees else disagreement_trace(S, c, d, exact, pos, inside_d)}


def compare_recorded_flags(step, flags, *, validate=True):
    """Both pre-repair flags, preserving the record's direction convention.

    A planted changed flag fails here with the step and field named. The
    optional non-raising mode lets the full lead retain a mismatch as evidence.
    """
    (c, x), (d, y), (c2, z) = step.word
    if step.rule != 'conj' or c2 != c or step.n != flags['step']:
        raise ValueError(f'step {step.n}: flags do not describe this repair')
    if [list(m) for m in step.word] != flags['word']:
        raise ValueError(f'step {step.n}: the repair word differs from the flag record')
    S = step.S
    pos = seams.positions(S.st)
    classified = covers.inside_sets(S, (c, d))
    rows = []
    for covered, coverer, name in ((d, c, 'c_covers_d'), (c, d, 'd_covers_c')):
        floating = covers.float_covers(S, covered, coverer, pos)
        matches = floating.covers == flags[name]
        if validate and not matches:
            raise RecordedFlagError(f"step {step.n}: {name}: recorded {flags[name]}, "
                                    f'float test {floating.covers}')
        row = compare_pair(S, covered, coverer, pos, classified[coverer])
        row.update(recorded_flag=name, recorded=flags[name], recorded_reproduced=matches)
        rows.append(row)
    return rows


def _research_choice(S, survey, relation):
    """Reproduce cover_tops followed by the largest drop, as a research fact.

    Keep this selection in the measurement script, outside the named-cap job.
    Only the research rule selects a cap; no new rule or recommendation is made.
    """
    tops = [c for c in survey if not relation[c] and any(c in relation[q] for q in survey)]
    rows = S._rows(tops)
    return rows[0][1] if rows else None


def cover_step(step):
    S = step.S
    started = perf_counter()
    survey = S.survey()
    needed = {d for c in survey for d in covers.meeting_caps(c)}
    classified = covers.inside_sets(S, needed)
    pos = seams.positions(S.st)
    exact_graph, float_graph = {c: set() for c in survey}, {c: set() for c in survey}
    disagreements, compared, agree = [], 0, 0
    for c in sorted(survey):
        for d in covers.meeting_caps(c):
            row = compare_pair(S, c, d, pos, classified[d])
            compared += 1
            agree += row['agrees']
            if row['exact']['covers']:
                exact_graph[c].add(d)
            if row['float']['covers']:
                float_graph[c].add(d)
            if not row['agrees']:
                disagreements.append(row)
    float_choice = _research_choice(S, survey, float_graph)
    exact_choice = _research_choice(S, survey, exact_graph)
    recorded_cap = step.word[0][0]
    summary = {'kind': 'step_summary', 'rule': 'cover', 'step': step.n, 'input_digest': S.st.digest(),
               'domain': {'covered': 'admissible misaligned caps', 'coverers': '80 meeting caps per covered cap'},
               'misaligned_caps': sorted(survey), 'pairs_compared': compared, 'pairs_agree': agree,
               'disagreements': len(disagreements),
               'traces': dict(Counter(r['trace']['reason'] for r in disagreements)),
               'research_rule': {'recorded_cap': recorded_cap, 'float_cap': float_choice,
                                 'exact_cap': exact_choice, 'would_change': exact_choice != recorded_cap,
                                 'float_reproduces_record': float_choice == recorded_cap,
                                 'float_to_exact_change': exact_choice != float_choice},
               'seconds': perf_counter() - started}
    return disagreements, summary


def _run_seed(seed):
    started = perf_counter()
    path, record = census.load(seed)
    flags = recorded_flags() if seed == 7101 else {}
    rows, summaries, query_times, flag_mismatches = [], [], [], []
    for step in census.walk(seed):
        if step.rule == 'cover':
            disagreements, summary = cover_step(step)
        elif seed == 7101 and step.rule == 'conj':
            # Measure before comparing pairs: the query's inside/face caches
            # have not been warmed for this state by this lead.
            query, parts = covers.timed_covers_query(step.S, step.word[0][0])
            if query.status != 'completed':
                raise AssertionError(f'step {step.n}: repair-cap query failed: {query.message}')
            query_times.append({'step': step.n, 'seconds': query.seconds, 'parts': parts})
            pairs = compare_recorded_flags(step, flags[step.n], validate=False)
            disagreements = [r for r in pairs if not r['agrees']]
            mismatches = [{'step': step.n, 'flag': r['recorded_flag'], 'recorded': r['recorded'],
                           'float': r['float']['covers']} for r in pairs if not r['recorded_reproduced']]
            flag_mismatches.extend(mismatches)
            summary = {'kind': 'step_summary', 'rule': 'conj', 'step': step.n,
                       'input_digest': step.S.st.digest(), 'word': step.word, 'pairs_compared': 2,
                       'pairs_agree': sum(r['agrees'] for r in pairs), 'disagreements': len(disagreements),
                       'pairs': pairs, 'traces': dict(Counter(r['trace']['reason'] for r in disagreements)),
                       'recorded_flag_mismatches': mismatches, 'query_seconds': query.seconds,
                       'query_parts': parts}
        else:
            continue
        summary['seed'] = seed
        summaries.append(summary)
        for row in disagreements:
            rows.append({'kind': 'disagreement', 'seed': seed, 'step': step.n, 'rule': step.rule, **row})
        rows.append(summary)
    assert len([r for r in summaries if r['rule'] == 'cover']) == sum(s['rule'] == 'cover' for s in record['steps'])
    if seed == 7101:
        assert len(query_times) == 63
    return {'rows': rows, 'summaries': summaries, 'queries': query_times, 'flag_mismatches': flag_mismatches,
            'source': {'seed': seed, 'record': census.RECORDS[seed],
                       'sha256': hashlib.sha256(path.read_bytes()).hexdigest()},
            'seconds': perf_counter() - started}


def _timing_summary(queries):
    def stats(values):
        return {'median': median(values), 'largest': max(values)} if values else {'median': None, 'largest': None}
    return {'queries': len(queries), 'seconds': stats([q['seconds'] for q in queries]),
            'parts': {p: stats([q['parts'][p] for q in queries]) for p in ('inside_sets', 'faces', 'answers')},
            'measurements': queries}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True)
    parser.add_argument('--seeds', nargs='+', type=int, choices=tuple(census.RECORDS),
                        default=list(census.RECORDS))
    parser.add_argument('--jobs', type=int, default=1)
    args = parser.parse_args(argv)
    if args.jobs < 1:
        parser.error('--jobs must be positive')
    try:
        out = covers.output_directory(args.out)
    except ValueError as error:
        parser.error(str(error))
    started = perf_counter()
    out.mkdir(parents=True, exist_ok=True)
    seeds = list(dict.fromkeys(args.seeds))
    if args.jobs > 1:
        # Forked workers must not race while writing the shared centroid cache.
        # Spawned workers get census's own isolated temporary cache instead.
        seams.home_centroids()
    pool = ProcessPoolExecutor(max_workers=min(args.jobs, len(seeds))) if args.jobs > 1 else None
    summaries, sources, queries, mismatches = [], [], [], []
    try:
        results = pool.map(_run_seed, seeds) if pool else map(_run_seed, seeds)
        with (out / 'x4-pairs.jsonl').open('w', encoding='utf-8') as stream:
            for result in results:
                for row in result['rows']:
                    stream.write(json.dumps(row) + '\n')
                summaries.extend(result['summaries'])
                sources.append(result['source'])
                queries.extend(result['queries'])
                mismatches.extend(result['flag_mismatches'])
    finally:
        if pool:
            pool.shutdown()
    traces = Counter()
    for row in summaries:
        traces.update(row['traces'])
    summary = {**provenance(), 'sources': sources, 'jobs': args.jobs, 'threshold': covers.THRESHOLD,
               'steps': len(summaries), 'cover_steps': sum(r['rule'] == 'cover' for r in summaries),
               'repair_steps': sum(r['rule'] == 'conj' for r in summaries),
               'pairs_compared': sum(r['pairs_compared'] for r in summaries),
               'pairs_agree': sum(r['pairs_agree'] for r in summaries),
               'disagreements': sum(r['disagreements'] for r in summaries), 'traces': dict(traces),
               'unexplained': traces['unexplained'], 'recorded_flag_mismatches': mismatches,
               'research_choice_changes': [{'seed': r['seed'], 'step': r['step'], **r['research_rule']}
                                           for r in summaries if r['rule'] == 'cover' and r['research_rule']['would_change']],
               'one_query': _timing_summary(queries), 'seconds': perf_counter() - started}
    if 7101 in seeds:
        summary['recorded_flags_source'] = {'record': str(FLAGS_PATH.relative_to(census.CENSUS)),
                                             'sha256': hashlib.sha256(FLAGS_PATH.read_bytes()).hexdigest()}
    (out / 'x4-summary.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
    return summary


if __name__ == '__main__':
    main()
