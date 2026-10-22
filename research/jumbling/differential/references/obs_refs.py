"""Recorded observation values and checked J1 state builders.

python -B research/jumbling/differential/references/obs_refs.py --out DIR \
    [--states fixtures stop40 deep] [--extract DIR --height4]

Outputs observation-<state>.json and summary.json. Each state includes counts,
rim_summary, partition_sizes, off_grip_groups, comparisons and seconds.
Timings are cloud research timings, never product-engine performance evidence.
The deep replay alone measured 147.62 s here; default unit tests leave it to
this CLI's full mode. External stage-12 Python is parsed as literal data only,
never imported or executed. Tests never read the draft-branch extraction.
"""

import sys

sys.dont_write_bytecode = True

import argparse
import ast
from collections import Counter
from dataclasses import asdict
from functools import lru_cache
import hashlib
import json
from pathlib import Path
import re
from time import perf_counter

ROOT = Path(__file__).resolve().parents[4]
JUMBLING = ROOT / 'research' / 'jumbling'
THEORY_RESULTS = JUMBLING / 'theory' / 'results'
CHAIN = JUMBLING / 'restoration/09-general-method/earlier/stop62-860-to-0/chain/chain.json'
STAGE12 = Path('research/jumbling/restoration/12-low-height')
PROSE = {
    'cells': 135, 'group_sizes': [59, 37, 25, 14],
    'group_59_common_caps': [30, 31, 34, 73, 89, 145],
    'holding_caps': 20,
    'blocked_examples': [31, 73, 96, 152, 153, 295, 302],
    'free_aligned_examples': [64, 89, 183, 283, 290, 301, 319],
}


def _read(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


@lru_cache(maxsize=1)
def _modules():
    if str(JUMBLING) not in sys.path:
        sys.path.insert(0, str(JUMBLING))
    # Before any theory module that might use the research centroid cache.
    from differential.leads import census
    from differential.references import observations
    import numpy
    import sim
    import trees
    return census, observations, numpy, sim, trees


def _checked(field, actual, expected):
    if actual != expected:
        raise AssertionError(f'{field}: reference={expected!r}, rebuilt={actual!r}')


def _tree_summary(state):
    _, O, np, _, trees = _modules()
    pieces, classes, left = Counter(), Counter(), Counter()
    for pid, count in zip(*np.unique(state.pose_id, return_counts=True)):
        pid, count = int(pid), int(count)
        heights = ((0, 0) if state._poses[pid].kidx >= 0 else
                   tuple(trees.dist(q) for q in trees.factors(state._pose_matrix(pid))))
        key = '%d,%d' % heights
        pieces[key] += count
        classes[key] += 1
        left[heights[0]] += count
    return {'pieces_by_heights': dict(sorted(pieces.items())),
            'pose_classes_by_heights': dict(sorted(classes.items())),
            'height_counts': O.left_counts(left)}


def trees_reference(menu, t):
    """A complete recorded row; unknown menus or unrecorded prefixes fail."""
    rows = _read(THEORY_RESULTS / 'trees-fixtures.json')
    if menu not in rows:
        raise ValueError(f'no trees reference for menu {menu!r}')
    for row in rows[menu]:
        if row['t'] == t:
            return row
    raise ValueError(f'no trees reference for {menu}@{t}')


def stop40_reference():
    record = _read(THEORY_RESULTS / 'seams-I_a-40-stop.json')
    return {'digest': _read(CHAIN)['start_digest'], 'off': 860,
            'n_total': record['N_end'], 'grip_n': record['grip_N_end'],
            'moves': record['moves'], 'prose': dict(PROSE)}


def deep_reference():
    return _read(THEORY_RESULTS / 'deep-scramble-I_a.json')['scramble']


def fixture_state(menu, t):
    """Replay the specified prefix and check both recorded height histograms."""
    census, _, _, _, _ = _modules()
    expected = trees_reference(menu, t)
    fixture = _read(JUMBLING / 'fixtures' / f'wj-{menu}.json')
    records = fixture['journal']['records']
    if type(t) is not int or not 0 < t <= len(records):
        raise ValueError('fixture prefix is outside the recorded journal')
    state, _ = census.seams._fixture_state(menu, t)
    actual = _tree_summary(state)
    for key in ('pieces_by_heights', 'pose_classes_by_heights'):
        _checked(f'{menu}@{t}.{key}', actual[key], expected[key])
    # Where the fixture also records a digest, check that independent value.
    for stage in fixture['stages'].values():
        if stage.get('records') == t or stage.get('t') == t:
            _checked(f'{menu}@{t}.digest', state.digest(), stage['digest'])
    return state


def stop40_state():
    """I_a@40 followed chronologically by the 62 recorded restorer twists."""
    census, _, _, _, _ = _modules()
    reference = stop40_reference()
    state, menu = census.seams._fixture_state('I_a', 40)
    restorer = census.seams.SeamRestorer(state, menu)
    for c, i in reference['moves']:
        restorer.apply(c, i)
    _checked('stop40.digest', state.digest(), reference['digest'])
    _checked('stop40.off', int(state.off_lattice_count()), reference['off'])
    return state


def deep_state():
    """Replay the stored 360 letters, without rerunning scramble selection."""
    census, _, _, sim, _ = _modules()
    reference = deep_reference()
    menu = census.menu()
    state = sim.State(census.restore.ctx, menu=menu)
    _play(state, reference['moves'], 'deep')
    for field, actual in (('digest', state.digest()),
                          ('off_lattice_pieces', int(state.off_lattice_count())),
                          ('moved_pieces', int(state.moved_count()))):
        _checked(f'deep.{field}', actual, reference[field])
    return state


def _play(state, letters, label):
    census, _, _, sim, _ = _modules()
    for step, (c, i) in enumerate(letters, 1):
        outcome = state.apply(sim.Twist(state.ctx, c, census.restore.element(state.menu, c, i),
                                       'menu', {'menu': state.menu.name}), journal=False)
        if not outcome.applied:
            raise AssertionError(f'{label}, step {step}, cap {c}: {outcome.status}')


def _comparison(field, actual, expected):
    return {'field': field, 'reference': expected, 'actual': actual,
            'result': 'pass' if actual == expected else 'fail'}


def off_grip_summary(groups):
    """Summaries distinguish per-cell union from per-domain common holders."""
    rows = []
    all_caps, common_caps = set(), set()
    for key, cells in groups.items():
        holders = [set(cell.holding_caps) for cell in cells]
        common, union = set.intersection(*holders), set.union(*holders)
        statuses = {c: cell.holding_caps[c] for cell in cells for c in cell.holding_caps}
        all_caps.update(union)
        common_caps.update(common)
        rows.append({'domain_key': key, 'cells': len(cells),
                     'common_holding_caps': sorted(common),
                     'holding_caps': {c: statuses[c] for c in sorted(union)}})
    return {'cells': sum(row['cells'] for row in rows),
            'group_sizes': sorted((row['cells'] for row in rows), reverse=True),
            'groups': rows, 'holding_caps': sorted(all_caps),
            'whole_group_holding_caps': sorted(common_caps),
            'reading': 'one occupied side per domain/chamber pair; exact non-grip plane; '
                       'a holder contains every vertex of the closed 3-cell'}


def prose_comparisons(summary, observation):
    common59 = next((g['common_holding_caps'] for g in summary['groups'] if g['cells'] == 59), None)
    rows = [_comparison('prose.cells', summary['cells'], PROSE['cells']),
            _comparison('prose.group_sizes', summary['group_sizes'], PROSE['group_sizes']),
            _comparison('prose.group_59_common_caps', common59, PROSE['group_59_common_caps']),
            _comparison('prose.holding_caps_union', len(summary['holding_caps']), PROSE['holding_caps']),
            _comparison('prose.holding_caps_common', len(summary['whole_group_holding_caps']),
                        PROSE['holding_caps'])]
    for c in PROSE['blocked_examples']:
        rows.append(_comparison(f'prose.blocked.cap_{c}', observation.rim[c].status, 'blocked'))
    for c in PROSE['free_aligned_examples']:
        rows.append(_comparison(f'prose.free_aligned.cap_{c}',
                                [observation.rim[c].status, observation.rim[c].N_c], ['admissible', 0]))
    return rows


def _source_digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def measure_state(label, builder, *, fixture=None, stop=False):
    _, O, np, _, _ = _modules()
    started = perf_counter()
    state = builder()
    seconds = {'replay': perf_counter() - started}
    started = perf_counter()
    observation = O.observe(state)
    seconds['observe'] = perf_counter() - started
    started = perf_counter()
    alternate = O.observe(state, rim_route='seams')
    difference = O.compare(alternate, observation)
    comparisons = [{'field': 'rim_vs_seams', 'result': 'pass' if difference is None else 'fail',
                    'difference': None if difference is None else asdict(difference)}]
    seconds['observe_seams'] = perf_counter() - started
    started = perf_counter()
    off_grip = off_grip_summary(O.off_grip_seams(state))
    seconds['off_grip'] = perf_counter() - started
    domains = O.rim.Domains(state)
    partition = observation.domain_partition()
    partition_matches = all(partition[O.domain_key(O.to_tuple(domains.reps[d]))] == tuple(mem.tolist())
                            for d, mem in enumerate(domains.members) if len(mem))
    comparisons.append(_comparison('domain_partition', partition_matches, True))
    histograms = O.height_histograms(state, observation)
    sources = {}
    if fixture is not None:
        menu, t = fixture
        reference = trees_reference(menu, t)
        for field in ('pieces_by_heights', 'pose_classes_by_heights'):
            comparisons.append(_comparison(field, histograms[field], reference[field]))
        sources['theory/results/trees-fixtures.json'] = _source_digest(THEORY_RESULTS / 'trees-fixtures.json')
        sources[f'fixtures/wj-{menu}.json'] = _source_digest(JUMBLING / 'fixtures' / f'wj-{menu}.json')
    if stop:
        reference = stop40_reference()
        for field in ('digest', 'n_total', 'grip_n'):
            comparisons.append(_comparison(field, getattr(observation, field), reference[field]))
        comparisons.append(_comparison('misaligned_caps', sum((r.N_c or 0) > 0
                                                             for r in observation.rim.values()), 15))
        sources['theory/results/seams-I_a-40-stop.json'] = _source_digest(
            THEORY_RESULTS / 'seams-I_a-40-stop.json')
        sources['restoration/09-general-method/earlier/stop62-860-to-0/chain/chain.json'] = _source_digest(CHAIN)
    if label == 'deep':
        reference = deep_reference()
        comparisons.extend([_comparison('digest', observation.digest, reference['digest']),
                            _comparison('off', int(state.off_lattice_count()), reference['off_lattice_pieces'])])
        sources['theory/results/deep-scramble-I_a.json'] = _source_digest(THEORY_RESULTS / 'deep-scramble-I_a.json')
    return {'state': label, 'digest': observation.digest,
            'counts': dict(observation.height_counts, off=int(state.off_lattice_count())),
            **histograms, 'n_total': observation.n_total, 'grip_n': observation.grip_n,
            'rim_summary': {c: dict(asdict(row), blocked=row.blocked) for c, row in observation.rim.items()},
            'partition_sizes': {key: len(members) for key, members in partition.items()},
            'off_grip_groups': off_grip, 'comparisons': comparisons,
            'prose_comparisons': prose_comparisons(off_grip, observation) if stop else [],
            'seconds': seconds, 'input_sha256': sources,
            'python': '.'.join(map(str, sys.version_info[:3])), 'numpy': np.__version__,
            'evidence_kind': 'source and fixture replay; cloud research timings',
            'status': 'pass' if all(r['result'] == 'pass' for r in comparisons) else 'fail'}


def _outside(path, option):
    result = Path(path).resolve()
    if result.is_relative_to(ROOT):
        raise ValueError(f'{option} must be outside the repository')
    return result


def _output_file(directory, filename):
    path = directory / filename
    if path.resolve().is_relative_to(ROOT):
        raise ValueError(f'--out must be outside the repository; {filename} resolves inside it')
    return path


def _literal(node, names):
    """Small data-only subset needed by the extracted STATES assignment."""
    if isinstance(node, ast.Constant):
        return node.value
    if isinstance(node, ast.Name):
        return names[node.id]
    if isinstance(node, (ast.List, ast.Tuple)):
        items = [_literal(item, names) for item in node.elts]
        return tuple(items) if isinstance(node, ast.Tuple) else items
    if isinstance(node, ast.Dict):
        return {_literal(k, names): _literal(v, names) for k, v in zip(node.keys, node.values)}
    if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Add):
        return _literal(node.left, names) + _literal(node.right, names)
    raise ValueError('unsupported expression in extracted state data')


def _extracted_file(extract, relative):
    path = extract / relative
    resolved = _outside(path, '--extract input')
    if not resolved.is_relative_to(extract):
        raise ValueError('extracted input escapes --extract')
    return path


def height4_inputs(extract):
    """Read six descriptors and the README table without executing draft code."""
    extract = _outside(extract, '--extract')
    # The extraction root contains its research/... subtree, as in the packet.
    script = _extracted_file(extract, STAGE12 / 'scripts/round22/r22d25_d19_stops.py')
    readme = _extracted_file(extract, STAGE12 / 'README.md')
    names = {}
    for node in ast.parse(script.read_text(encoding='utf-8')).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            name = node.targets[0].id
            try:
                names[name] = _literal(node.value, names)
            except (ValueError, KeyError, TypeError):
                continue
            if name == 'STATES':
                break
    states = names.get('STATES', {})
    if len(states) != 6:
        raise ValueError('extracted script does not describe six height-4 states')
    recorded = {}
    for line in readme.read_text(encoding='utf-8').splitlines():
        columns = [c.strip() for c in line.split('|')]
        if len(columns) != 9 or not re.fullmatch(r'`[0-9a-f]{8}…`', columns[1]):
            continue
        tag = columns[1][1:9]
        if tag not in states or not re.fullmatch(r'\([\d, ]+\)', columns[3]):
            continue
        heights = [int(n.strip().replace(',', '')) for n in columns[3][1:-1].split(', ')]
        if len(heights) != 3:
            raise ValueError(f'height-4 README: malformed height counts for {tag}')
        recorded[tag] = dict(zip(('n_2', 'n_4', 'n_6'), heights))
        recorded[tag].update(zip(('off', 'height4_units', 'height2_units', 'blocked_caps'),
                                (int(c.replace(',', '')) for c in columns[4:8])))
    if set(recorded) != set(states):
        raise ValueError('extracted README does not contain all six recorded height-4 rows')
    return states, recorded, {'README.md': _source_digest(readme),
                              'r22d25_d19_stops.py': _source_digest(script)}


def height4_state(extract, descriptor):
    """Replay a state from the extraction: its stage-12 or stage-11 start record is on #112's branch only."""
    census, _, _, sim, _ = _modules()
    source, stop, prefix, checkpoints, expected = descriptor
    relative = Path(source)
    stage11 = Path('research/jumbling/restoration/11-scramble-census')
    if not (relative.is_relative_to(STAGE12) or relative == stage11 / 'runs/heights/ia-k3000-s7302.json'):
        raise ValueError('height-4 descriptor names an input outside its allowed source directories')
    path = _extracted_file(_outside(extract, '--extract'), relative)
    record = _read(path)
    state = sim.State(census.restore.ctx, menu=census.menu())
    _play(state, record['history'] + record['moves'], source)
    _checked(f'{source}.stop_digest', state.digest(), stop)
    _checked(f'{source}.off', int(state.off_lattice_count()), record['off_now'])
    for step, letter in enumerate(prefix, 1):
        _play(state, [letter], f'height4 prefix {step}')
        if step in checkpoints:
            _checked(f'height4 prefix {step}.digest', state.digest(), checkpoints[step])
    _checked('height4.digest', state.digest(), expected)
    return state, {source: _source_digest(path)}


def measure_height4(extract):
    states, references, source_digests = height4_inputs(extract)
    rows = []
    for tag, descriptor in states.items():
        started = perf_counter()
        try:
            state, inputs = height4_state(extract, descriptor)
            replay = perf_counter() - started
            result = measure_state(f'height4-{tag}', lambda: state)
            actual = {key: result['counts'][key] for key in ('n_2', 'n_4', 'n_6', 'off')}
            actual['blocked_caps'] = sum(row['blocked'] for row in result['rim_summary'].values())
            checks = [_comparison(key, value, references[tag][key]) for key, value in actual.items()]
            result['counts']['blocked_caps'] = actual['blocked_caps']
            result['recorded'] = references[tag]
            result['comparisons'].extend(checks)
            result['seconds'].update(replay=replay, total=perf_counter() - started)
            result['input_sha256'].update(dict(source_digests, **inputs))
            result['status'] = 'pass' if all(c['result'] == 'pass' for c in result['comparisons']) else 'fail'
            rows.append(result)
        except (OSError, ValueError, AssertionError, KeyError) as error:
            # Only the exception type for OS errors: do not publish private extraction paths.
            reason = type(error).__name__ if isinstance(error, OSError) else str(error)
            rows.append({'state': f'height4-{tag}', 'status': 'unreconstructed', 'reason': reason,
                         'comparisons': [], 'seconds': {'total': perf_counter() - started},
                         'input_sha256': source_digests})
    return rows


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True)
    parser.add_argument('--states', nargs='*', choices=('fixtures', 'stop40', 'deep'),
                        default=['fixtures', 'stop40', 'deep'])
    parser.add_argument('--extract')
    parser.add_argument('--height4', action='store_true')
    args = parser.parse_args(argv)
    try:
        directory = _outside(args.out, '--out')
        extract = _outside(args.extract, '--extract') if args.extract is not None else None
        if args.height4 and extract is None:
            raise ValueError('--height4 requires --extract')
        if not args.states and not args.height4:
            raise ValueError('no states selected')
        labels = []
        if 'fixtures' in args.states:
            labels += [f'{name}-{row["t"]}' for name, rows in
                       _read(THEORY_RESULTS / 'trees-fixtures.json').items() for row in rows]
        labels += [s for s in ('stop40', 'deep') if s in args.states]
        if args.height4:
            # Validate the whole external descriptor/table before any replay.
            labels += [f'height4-{tag}' for tag in height4_inputs(extract)[0]]
        for name in ['summary.json'] + [f'observation-{label}.json' for label in labels]:
            _output_file(directory, name)
    except (ValueError, OSError) as error:
        parser.error(str(error))

    directory.mkdir(parents=True, exist_ok=True)
    results = []

    def write(result):
        path = _output_file(directory, f'observation-{result["state"]}.json')
        path.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
        results.append({'state': result['state'], 'status': result['status'],
                        'seconds': result['seconds'], 'file': path.name})

    if 'fixtures' in args.states:
        for name, rows in _read(THEORY_RESULTS / 'trees-fixtures.json').items():
            for row in rows:
                t = row['t']
                write(measure_state(f'{name}-{t}', lambda: fixture_state(name, t), fixture=(name, t)))
    if 'stop40' in args.states:
        write(measure_state('stop40', stop40_state, stop=True))
    if 'deep' in args.states:
        write(measure_state('deep', deep_state))
    if args.height4:
        for result in measure_height4(extract):
            write(result)
    summary = {'states': results, 'status': 'pass' if results and all(
        r['status'] == 'pass' for r in results) else 'fail',
        'prose_discrepancies_are_separate': True,
        'height4_units': 'recorded only; the requested comparisons cover heights, off and blocked caps',
        'evidence_kind': 'source and fixture replay; cloud research timings'}
    _output_file(directory, 'summary.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
    return 0 if summary['status'] == 'pass' else 1


if __name__ == '__main__':
    raise SystemExit(main())
