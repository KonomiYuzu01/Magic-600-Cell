"""X5: exact checks of one released item, with an untimed oracle comparison.

python -B research/jumbling/differential/leads/x5_release.py --out DIR

N_c after is evaluated for the released item only. The research scores()
oracle runs afterwards, outside every reported check timing. Height counts
use exact hypothetical poses of the inside pieces without changing the state.
Evidence is source/fixture replay and cloud timings of research code.
"""

import sys

sys.dont_write_bytecode = True

import argparse
import json
from pathlib import Path
from statistics import median
from time import perf_counter

if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from differential.leads import census
from differential.leads.repair_list import contract_revision, output_directory, run_metadata
from differential.menu_order import I_A_IDENTITY, I_A_ORDER_DIGEST, check_menu
from exact import matmul
import numpy as np
import rim
import seams
import sim
from sim.kplus import to_tuple
import trees

COUNT_KEYS = ('n_0', 'n_2', 'n_4', 'n_6', 'n_above_6')
PARTS = ('classification_and_domains', 'cut_sets', 'one_item', 'heights', 'total')


def _height_counts(S, inside, g):
    """Sparse left-height counts, using dist(factors(M)[0]) (no branch work)."""
    st = S.st
    counts = np.bincount(st.pose_id, minlength=len(st._poses))
    moved = np.bincount(st.pose_id[inside], minlength=len(st._poses))
    before = dict.fromkeys(COUNT_KEYS, 0)
    after = dict.fromkeys(COUNT_KEYS, 0)
    before['max_above_6'] = after['max_above_6'] = None
    memo = {}

    def left_height(matrix):
        key = to_tuple(matrix)
        if key not in memo:
            memo[key] = trees.dist(trees.factors(matrix)[0])
        return memo[key]

    def add(histogram, height, count):
        if not count:
            return
        if height in (0, 2, 4, 6):
            histogram[f'n_{height}'] += count
        elif height > 6:
            histogram['n_above_6'] += count
            histogram['max_above_6'] = max(histogram['max_above_6'] or 0, height)
        else:
            raise AssertionError(f'I_a pose has unexpected left height {height}')

    for pid in np.flatnonzero(counts).tolist():
        pose = st._poses[pid]
        matrix = st._pose_matrix(pid)
        height = 0 if pose.kidx >= 0 else left_height(matrix)
        n, k = int(counts[pid]), int(moved[pid])
        add(before, height, n)
        add(after, height, n - k)
        if k:
            add(after, left_height(matmul(g, matrix)), k)
    return before, after


def release_check(S, c, x) -> dict:
    """Check just (c, x) at S's current revision; never call S.scores()."""
    if type(c) is not int or not 0 <= c < 600:
        raise ValueError('cap must be an integer in 0..599')
    if type(x) is not int or not 0 <= x < S.n_items:
        raise ValueError('item must be a pinned menu index')
    started = perf_counter()
    check_menu(S.M.record(), identity=I_A_IDENTITY, order=I_A_ORDER_DIGEST)
    digest = S.st.digest()
    part = perf_counter()
    # Deliberately fresh: the classification/domain timing is not a cache hit
    # from the research survey. Neither operation scores any alternative item.
    classification = S.st.classify(c, need_inside=True, stop_at_straddle=True)
    if classification.straddle is not None or classification.uncertain:
        raise ValueError(f'cap {c} is blocked or uncertain')
    D = rim.Domains(S.st)
    times = {'classification_and_domains': perf_counter() - part}
    part = perf_counter()
    mask = np.zeros(rim.NP, bool)
    mask[classification.inside] = True
    rs = D.rim_sets(c, mask)
    now = D.misalignment(c, mask, rs=rs)
    n_in = sum(len(a) for _, a, _ in rs.values())
    n_out = sum(len(b) for _, _, b in rs.values())
    floor = abs(n_in - n_out)
    times['cut_sets'] = perf_counter() - part
    part = perf_counter()
    g = S.g(c, x)
    after = D.misalignment(c, mask, (c, x), g, rs)
    times['one_item'] = perf_counter() - part
    part = perf_counter()
    heights_before, heights_after = _height_counts(S, classification.inside, g)
    delta_n_h = {key: heights_after[key] - heights_before[key] for key in COUNT_KEYS}
    times['heights'] = perf_counter() - part
    times['total'] = perf_counter() - started
    return {'cap': c, 'item': x, 'input_digest': digest,
            'menu_identity': I_A_IDENTITY, 'order_digest': I_A_ORDER_DIGEST,
            'contract_revision': contract_revision(),
            'N_c_now': now, 'N_c_after': after, 'floor': floor, 'delta_N': after - now,
            'heights_before': heights_before, 'heights_after': heights_after,
            'delta_n_h': delta_n_h, 'seconds': times}


def _verified_check(S, c, x):
    result = release_check(S, c, x)
    # All 60 scores are a reference comparison only, outside release_check.
    now, after = S.scores(c)
    expected = after[x] if after else now
    if result['N_c_now'] != now or result['N_c_after'] != expected:
        raise AssertionError(f'release ({c}, {x}) differs from the research scores')
    if result['floor'] != abs(S.balance(c)[0] - S.balance(c)[1]):
        raise AssertionError(f'release ({c}, {x}) differs from the research floor')
    result['oracle_checked'] = True
    return result


def measure_census(steps):
    """Check the first twist of each supplied repair Step from census.walk."""
    rows = []
    for step in steps:
        if step.rule == 'conj':
            c, x = step.word[0]
            row = _verified_check(step.S, c, x)
            row.update(source='seed-7101', step=step.n)
            rows.append(row)
    return rows


def fixture_end():
    """Replay the fixture's exact journal under the pinned research I_a menu."""
    path = census.JUMBLING / 'fixtures' / 'wj-I_a.json'
    fixture = json.loads(path.read_text(encoding='utf-8'))
    check_menu(fixture['menu'], identity=I_A_IDENTITY, order=I_A_ORDER_DIGEST)
    if fixture['journal']['contract_revision'] != contract_revision():
        raise AssertionError('wj-I_a contract revision differs')
    M = census.menu()
    st = sim.State(rim.ctx, menu=M)
    for index, record in enumerate(fixture['journal']['records'], 1):
        outcome = st.apply(sim.Twist.from_record(rim.ctx, record))
        if not outcome.applied or outcome.moved != record['moved']:
            raise AssertionError(f'wj-I_a record {index} did not replay')
    if st.digest() != fixture['stages']['end']['digest']:
        raise AssertionError('wj-I_a end digest differs from the fixture')
    return seams.SeamRestorer(st, M)


def summarize(rows):
    if not rows:
        raise ValueError('no release checks were measured')
    return {'checks': len(rows), 'oracle_checks_passed': all(r['oracle_checked'] for r in rows),
            'seconds': {part: {'median': median(r['seconds'][part] for r in rows),
                               'largest': max(r['seconds'][part] for r in rows)} for part in PARTS}}


def run_release():
    rows = measure_census(census.walk(7101))
    if len(rows) != 63:
        raise AssertionError(f'expected 63 census repair releases, got {len(rows)}')
    S = fixture_end()
    # Select caps by the current cut only, avoiding all-item scores before the
    # timed check. The oracle is allowed to score alternatives afterwards.
    for c in range(600):
        inside = S.inside(c)
        if inside is None:
            continue
        mask = np.zeros(rim.NP, bool)
        mask[inside] = True
        if S.domains().misalignment(c, mask) > 0:
            row = _verified_check(S, c, 1)
            row.update(source='wj-I_a-end', step=None)
            rows.append(row)
    summary = summarize(rows)
    summary.update(run_metadata())
    summary['census_checks'] = 63
    summary['fixture_end_checks'] = len(rows) - 63
    return rows, summary


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True)
    args = parser.parse_args(argv)
    try:
        directory = output_directory(args.out)
    except ValueError as error:
        parser.error(str(error))
    rows, summary = run_release()
    directory.mkdir(parents=True, exist_ok=True)
    with (directory / 'x5-checks.jsonl').open('w', encoding='utf-8') as target:
        for row in rows:
            target.write(json.dumps(row) + '\n')
    (directory / 'x5-summary.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()
