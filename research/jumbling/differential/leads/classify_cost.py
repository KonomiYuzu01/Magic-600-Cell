"""Cost of J1 exact classification against amendment A2's filtered sign test.

At each cap both modes first run once untimed, so that neither timed call
builds a cache (J1's anchor groups, the Regions geometry) that the other then
reuses. Then the exact call runs, the filtered call second, on the same state
and cap. Statuses, inside sets and blocked flags must agree at every
observation; any disagreement is retained as a failure. These are cloud
research timings, never product performance evidence. The original state's
filter setting is restored.
"""

import sys

sys.dont_write_bytecode = True

import argparse
import hashlib
import json
from pathlib import Path
from time import perf_counter

ROOT = Path(__file__).resolve().parents[4]
JUMBLING = ROOT / 'research' / 'jumbling'
if str(JUMBLING) not in sys.path:
    sys.path.insert(0, str(JUMBLING))

import sim  # noqa: E402

from differential.leads.x2_straddlers import environment, output_directory, selected_caps, times  # noqa: E402


CACHES = ('anchors', 'cap_forms', 'cap_float_rows', 'vforms', 'float_rows', 'vertices')


def _cache_sizes(state):
    regions = state.ctx.regions
    return (len(state._anchor_cache), len(regions._capcache), len(regions._capfloat),
            len(regions._vcache), len(regions._fcache), len(regions._xcache))


def _observation(state, cap, filtered):
    state.filtered = filtered
    before = _cache_sizes(state)
    started = perf_counter()
    cl = state.classify(cap, need_inside=True, stop_at_straddle=True)
    elapsed = perf_counter() - started
    # Entries added during the timed call (the Regions caches are LRUs, so a full
    # one can still miss without growing; the anchor and cap caches never evict).
    grown = [after - first for first, after in zip(before, _cache_sizes(state))]
    return {'seconds': elapsed, 'status': cl.status, 'blocked': cl.straddle is not None,
            'inside': None if cl.inside is None else cl.inside.tolist(),
            'counts': dict(cl.counts), 'uncertain_pieces': cl.uncertain,
            'exact_evaluations': cl.exact_evaluations,
            'filtered_evaluations': cl.filtered_evaluations,
            'cache_entries_added': dict(zip(CACHES, grown))}


def measure(state, caps=None):
    rows = []
    previous = state.filtered
    try:
        for cap in selected_caps(caps):
            for mode in (False, True):  # untimed warm-up of both modes
                state.filtered = mode
                state.classify(cap, need_inside=True, stop_at_straddle=True)
            exact = _observation(state, cap, False)
            filtered = _observation(state, cap, True)
            agreement = {key: exact[key] == filtered[key] for key in ('status', 'inside', 'blocked')}
            rows.append({'cap': cap, 'exact': exact, 'filtered': filtered,
                         'agreement': agreement, 'passed': all(agreement.values())})
    finally:
        state.filtered = previous
    summary = {'state_digest': state.digest(), 'off_lattice': state.off_lattice_count(),
               'caps': len(rows), 'exact_seconds': times(rows, 'exact'),
               'filtered_seconds': times(rows, 'filtered'),
               'evaluation_counts': {mode: {kind: sum(row[mode][kind] for row in rows)
                                            for kind in ('exact_evaluations', 'filtered_evaluations')}
                                     for mode in ('exact', 'filtered')},
               'cache_entries_added_while_timed': {
                   mode: {kind: sum(row[mode]['cache_entries_added'][kind] for row in rows)
                          for kind in CACHES}
                   for mode in ('exact', 'filtered')},
               'passed': all(row['passed'] for row in rows),
               'failures': [{'cap': row['cap'], 'fields': [key for key, ok in row['agreement'].items() if not ok]}
                            for row in rows if not row['passed']]}
    return rows, summary


def run(states, out, caps=None):
    """Measure (label, J1 state) pairs as yielded, before advancing census.walk."""
    directory = output_directory(out, ('classify-caps.jsonl', 'classify-summary.json'))
    caps = selected_caps(caps)
    metadata = environment()
    rows, summaries = [], []
    with (directory / 'classify-caps.jsonl').open('w', encoding='utf-8') as stream:
        for label, state in states:
            cap_rows, summary = measure(state, caps)
            summary['state'] = label
            summaries.append(summary)
            for row in cap_rows:
                row.update(state=label, state_digest=summary['state_digest'])
                rows.append(row)
                stream.write(json.dumps(row, sort_keys=True) + '\n')
    summary = {'states': summaries, 'environment': metadata,
               'passed': all(item['passed'] for item in summaries),
               'failures': [{'state': item['state'], **failure}
                            for item in summaries for failure in item['failures']],
               'evidence': 'J1 source/fixture replay; cloud research timings, not product performance',
               'timing': ('both modes warmed untimed at each cap, then exact first, filtered second; '
                          'includes J1 certificate construction')}
    (directory / 'classify-summary.json').write_text(json.dumps(summary, indent=2, sort_keys=True) + '\n',
                                                   encoding='utf-8')
    return rows, summary


def input_states(*, fixture=None, census_seed=None, scramble=None):
    """Fixture end, every census step (including end), and/or a scramble state."""
    if fixture is None and census_seed is None and scramble is None:
        fixture = 'I_a'
    if fixture is not None:
        if fixture not in ('I_a', 'S4', 'I_b'):
            raise ValueError('fixture must be I_a, S4 or I_b')
        path = JUMBLING / 'fixtures' / f'wj-{fixture}.json'
        doc = json.loads(path.read_text(encoding='utf-8'))
        ctx = sim.get_context()
        menu = sim.TwistMenu.from_record(ctx, doc['menu'])
        state = sim.State.replay(doc['journal'], ctx, menu=menu)
        if state.digest() != doc['stages']['end']['digest']:
            raise ValueError('rebuilt fixture end digest differs from its recorded value')
        yield f'fixture:{fixture}:end:sha256:{hashlib.sha256(path.read_bytes()).hexdigest()}', state
    if census_seed is not None or scramble is not None:
        from differential.leads import census

        if census_seed is not None:
            for step in census.walk(census_seed, end=True):
                yield f'census:{census_seed}:step:{step.n}', step.S.st
        if scramble is not None:
            k, seed = scramble
            if k < 0:
                raise ValueError('scramble length must be non-negative')
            state, _ = census.restore.scramble(census.menu(), k, seed)
            yield f'scramble:{k}:{seed}', state


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path)
    parser.add_argument('--fixture', choices=('I_a', 'S4', 'I_b'))
    parser.add_argument('--census', type=int, dest='census_seed')
    parser.add_argument('--scramble', nargs=2, type=int, metavar=('K', 'SEED'))
    parser.add_argument('--caps', nargs='+', type=int)
    args = parser.parse_args(argv)
    try:
        output_directory(args.out, ('classify-caps.jsonl', 'classify-summary.json'))
        caps = selected_caps(args.caps)
        states = input_states(fixture=args.fixture, census_seed=args.census_seed, scramble=args.scramble)
        _, summary = run(states, args.out, caps)
    except ValueError as error:
        parser.error(str(error))
    return 0 if summary['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
