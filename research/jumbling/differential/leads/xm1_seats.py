"""XM1: face-match closing sets in the pinned I_a seats.

Run with python -B and --out outside the repository. Evidence is source and
fixture replay with J1 and the research restorer, not product performance.
"""

import sys

sys.dont_write_bytecode = True

import argparse
from collections import Counter
from dataclasses import dataclass
from functools import lru_cache
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
from time import perf_counter

if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from differential.leads import census
from differential.leads.covers import output_directory
from differential.menu_order import I_A_ORDER_DIGEST, order_digest

import numpy as np
from exact import identity, matmul
from sim.kplus import matrix_json, to_tuple

EXPECTED_SEATS = (
    frozenset((12, 16, 20, *range(24, 33))),
    frozenset((13, 17, 19, *range(34, 43))),
    frozenset((14, 21, 23, *range(43, 52))),
    frozenset((15, 18, 22, 33, *range(52, 60))),
)
CASES = ('outside_lattice', 'inside_lattice_only', 'both', 'neither')


@dataclass(frozen=True)
class Cosets:
    right: tuple
    left: tuple
    seat: tuple
    left_coset: tuple


@lru_cache(maxsize=1)
def seat_cosets():
    """Compute A4_0*s and s*A4_0 by exact pose composition (s first).

    Retained items are seat 0; other cosets are ordered by their least base
    item, 12, 13, 14, 15. A cap's frame transports the entire menu, so the
    item's base index, not a world-frame matrix comparison, decides its seat.
    """
    M = census.menu()
    assert order_digest(M.record()) == I_A_ORDER_DIGEST
    matrices = tuple(m for _, m in M.items)
    index = {m: i for i, m in enumerate(matrices)}
    assert matrices[0] == to_tuple(identity(4))
    H = frozenset(i for i, m in enumerate(matrices) if M.ctx.kplus.index_of_matrix(m) >= 0)
    assert H == frozenset(range(12)), 'the first twelve items must be retained A4_0'

    def cosets(right):
        remaining = set(range(len(matrices)))
        groups = []
        while remaining:
            s = min(remaining)
            group = frozenset(index[to_tuple(matmul(matrices[a], matrices[s]) if right else
                                             matmul(matrices[s], matrices[a]))] for a in H)
            assert len(group) == 12 and group <= remaining
            groups.append(group)
            remaining -= group
        return tuple(groups)

    right, left = cosets(True), cosets(False)
    assert right[0] == left[0] == H
    assert right[1:] == EXPECTED_SEATS, 'computed right cosets differ from the pinned seats'
    seat = tuple(next(j for j, group in enumerate(right) if i in group) for i in range(60))
    left_coset = tuple(next(j for j, group in enumerate(left) if i in group) for i in range(60))
    return Cosets(right, left, seat, left_coset)


def _side(P, side, lattice, domains, sign):
    pieces = P[side == sign]
    return {'count': len(pieces), 'on_lattice': int(lattice[pieces].sum()),
            'domains': [{'domain': int(d), 'pieces': pieces[domains[pieces] == d].tolist()}
                        for d in np.unique(domains[pieces]).tolist()]}


def analyze_step(step, seed=None):
    """One face-match step's cut, closing set, and seat/coset observations."""
    if step.rule != 'seam' or len(step.word) != 1:
        raise ValueError(f'step {step.n}: expected one face-match twist')
    S = step.S
    c, item = step.word[0]
    P, aligned, side = S.face(c)
    D, lattice = S.domains(), S.st.lattice_flags()
    inside = _side(P, side, lattice, D.dom, 1)
    outside = _side(P, side, lattice, D.dom, -1)
    in_lattice = inside['count'] == inside['on_lattice']
    out_lattice = outside['count'] == outside['on_lattice']
    case = ('both' if in_lattice and out_lattice else 'outside_lattice' if out_lattice else
            'inside_lattice_only' if in_lattice else 'neither')
    balance = tuple(int(n) for n in S.balance(c))
    floor = abs(balance[0] - balance[1])
    now, after = S.scores(c)
    assert min(after.values()) >= floor, f'step {step.n}: a score is below the floor'
    # Identity is pinned at 0; record every other item reaching the floor.
    Z = sorted(int(x) for x, n in after.items() if x != 0 and n == floor)
    if item not in Z:
        raise AssertionError(f'step {step.n}: record item {item} is not in the closing set {Z}')
    cosets = seat_cosets()
    counts = Counter(cosets.seat[x] for x in Z)
    return {'seed': seed, 'step': step.n, 'cap': c, 'input_digest': S.st.digest(),
            'face': {'pieces': P.tolist(), 'aligned': aligned.tolist(), 'side': side.tolist(),
                     'inside': inside, 'outside': outside},
            'domains': [{'domain': int(d), 'representative': matrix_json(D.reps[d])}
                        for d in np.unique(D.dom[P]).tolist()],
            'case': case, 'N_c': int(now), 'balance': list(balance), 'floor': floor,
            'closing_set': Z, 'closing_count': len(Z),
            'seats': [{'item': x, 'seat': cosets.seat[x]} for x in Z],
            'one_right_coset': bool(Z) and len(counts) == 1,
            'one_left_coset': bool(Z) and len({cosets.left_coset[x] for x in Z}) == 1,
            'seat_counts': {str(s): n for s, n in sorted(counts.items())},
            'record_twist': {'cap': c, 'item': item, 'seat': cosets.seat[item]}}


def provenance():
    """Sanitized tool/build identity for these research leads."""
    try:
        commit = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=census.ROOT, check=True,
                                capture_output=True, text=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        commit = None  # no Git checkout: the commit is unknown, never guessed
    return {'evidence_kind': 'source and fixture replay: J1 and research restorer',
            'timing_kind': 'cloud research code; not product performance evidence',
            'cpu_count': os.cpu_count(), 'load_average': list(os.getloadavg()) if hasattr(os, 'getloadavg') else None,
            'python': platform.python_version(), 'numpy': np.__version__, 'commit': commit,
            'menu_identity': census.menu().identity, 'menu_order_digest': I_A_ORDER_DIGEST}


def summarize(rows):
    cases = {}
    for case in CASES:
        selected = [r for r in rows if r['case'] == case]
        splits = Counter('+'.join(str(n) for n in sorted(r['seat_counts'].values()))
                         for r in selected if not r['one_right_coset'])
        right = sum(r['one_right_coset'] for r in selected)
        left = sum(r['one_left_coset'] for r in selected)
        cases[case] = {'steps': len(selected), 'one_seat': right, 'one_left_coset': left,
                       'split_over_seats': len(selected) - right, 'split_sizes': dict(sorted(splits.items())),
                       'answer': f'{case}: {len(selected)} steps; {right} in one seat; '
                                 f'{left} in one left coset; {len(selected) - right} split over seats '
                                 f'({dict(sorted(splits.items()))}).'}
    return cases


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True)
    parser.add_argument('--seeds', nargs='+', type=int, choices=tuple(census.RECORDS),
                        default=list(census.RECORDS))
    args = parser.parse_args(argv)
    try:
        out = output_directory(args.out)
    except ValueError as error:
        parser.error(str(error))
    started = perf_counter()
    out.mkdir(parents=True, exist_ok=True)
    rows, sources = [], []
    with (out / 'xm1-steps.jsonl').open('w', encoding='utf-8') as stream:
        for seed in dict.fromkeys(args.seeds):
            path, _ = census.load(seed)
            sources.append({'seed': seed, 'record': census.RECORDS[seed],
                            'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
            for step in census.walk(seed):
                if step.rule == 'seam':
                    row = analyze_step(step, seed)
                    rows.append(row)
                    stream.write(json.dumps(row) + '\n')
    cosets = seat_cosets()
    summary = {**provenance(), 'sources': sources, 'steps': len(rows), 'cases': summarize(rows),
               'seats': [{'seat': i, 'name': min(group), 'items': sorted(group)}
                         for i, group in enumerate(cosets.right)], 'seconds': perf_counter() - started}
    (out / 'xm1-summary.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
    return summary


if __name__ == '__main__':
    main()
