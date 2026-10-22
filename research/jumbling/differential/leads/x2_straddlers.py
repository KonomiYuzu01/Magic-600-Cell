"""X2: early-exit classifications against full certified straddler sets.

Source/fixture evidence and cloud timings of research J1 code only, never
product performance evidence. Each full-set time includes J1's certificate
construction; the additional independent point checks run outside that time.
Outputs and the census centroid cache stay outside the repository.
"""

import sys

sys.dont_write_bytecode = True

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
from time import perf_counter

ROOT = Path(__file__).resolve().parents[4]
JUMBLING = ROOT / 'research' / 'jumbling'
if str(JUMBLING) not in sys.path:
    sys.path.insert(0, str(JUMBLING))

import numpy as np  # noqa: E402

from differential.straddlers import check_certificate, straddler_set  # noqa: E402


def output_directory(out, filenames):
    """Validate the directory and each destination before creating any output."""
    directory = Path(out).resolve()
    for path in (directory, *(directory / name for name in filenames)):
        if path.resolve().is_relative_to(ROOT):
            raise ValueError('--out and its output files must be outside the repository')
    directory.mkdir(parents=True, exist_ok=True)
    return directory


def selected_caps(caps=None):
    values = list(range(600) if caps is None else caps)
    if any(type(c) is not int or not 0 <= c < 600 for c in values):
        raise ValueError('caps must be integers in 0..599')
    return sorted(set(values))


def environment():
    try:
        commit = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=ROOT,
                                capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        commit = None
    return {'cpu_count': os.cpu_count(), 'python': platform.python_version(),
            'numpy': np.__version__, 'commit': commit,
            'load_average': list(os.getloadavg()) if hasattr(os, 'getloadavg') else None}


def times(rows, field):
    values = [row[field]['seconds'] for row in rows]
    largest = max(rows, key=lambda row: row[field]['seconds']) if rows else None
    return {'total': sum(values), 'median': statistics.median(values) if values else None,
            'largest': None if largest is None else
            {'cap': largest['cap'], 'seconds': largest[field]['seconds']}}


def measure(state, caps=None):
    """Measure one state; return per-cap observations and their summary."""
    rows = []
    for cap in selected_caps(caps):
        started = perf_counter()
        cl = state.classify(cap, need_inside=False, stop_at_straddle=True)
        elapsed = perf_counter() - started
        first = None if cl.straddle is None else cl.straddle['piece']
        early = {'seconds': elapsed, 'status': cl.status, 'blocked': cl.straddle is not None,
                 'piece': first, 'uncertain_pieces': cl.uncertain, 'counts': dict(cl.counts),
                 'exact_evaluations': cl.exact_evaluations,
                 'filtered_evaluations': cl.filtered_evaluations}
        full = straddler_set(state, cap)
        # straddler_set's clock has stopped before any of these independent checks.
        bad = [p for p in full['pieces']
               if not check_certificate(state, cap, p, full['certificates'][p])]
        # Retain the full piece set and check results, not 600 caps' point data
        # in memory. Every certificate has already been constructed and tested.
        del full['certificates']
        full['u'] = len(full['pieces'])
        agreement = {'blocked': early['blocked'] == full['blocked'],
                     'first_piece': first in full['pieces'] if early['blocked'] else first is None,
                     'certain': cl.uncertain == 0 and full['uncertain_pieces'] == 0}
        rows.append({'cap': cap, 'early_exit': early, 'full_set': full,
                     'certificates_checked': not bad, 'invalid_certificates': bad,
                     'agreement': agreement, 'passed': not bad and all(agreement.values())})
    largest = max(rows, key=lambda row: row['full_set']['u']) if rows else None
    summary = {'state_digest': state.digest(), 'off_lattice': state.off_lattice_count(),
               'caps': len(rows), 'blocked_caps': [row['cap'] for row in rows if row['full_set']['blocked']],
               'u_distribution': dict(sorted(Counter(str(row['full_set']['u']) for row in rows).items(),
                                             key=lambda item: int(item[0]))),
               'largest_u': None if largest is None else
               {'u': largest['full_set']['u'], 'cap': largest['cap']},
               'early_exit_seconds': times(rows, 'early_exit'),
               'full_set_seconds': times(rows, 'full_set'),
               'passed': all(row['passed'] for row in rows),
               'failures': [row['cap'] for row in rows if not row['passed']]}
    return rows, summary


def run(state, out, caps=None, *, expect=None, source=None):
    """Write one X2 measurement, refusing an in-repository output destination."""
    directory = output_directory(out, ('x2-caps.jsonl', 'x2-summary.json'))
    if expect is not None and state.digest() != expect:
        raise ValueError('the measured state digest differs from --expect')
    metadata = environment()
    rows, summary = measure(state, caps)
    summary.update(environment=metadata, source=source,
                   evidence='J1 source/fixture replay; cloud research timings, not product performance',
                   filtered=bool(state.filtered),
                   timing='early exit first; full set includes certificate construction; independent checks excluded')
    with (directory / 'x2-caps.jsonl').open('w', encoding='utf-8') as stream:
        for row in rows:
            stream.write(json.dumps(row, sort_keys=True) + '\n')
    (directory / 'x2-summary.json').write_text(json.dumps(summary, indent=2, sort_keys=True) + '\n',
                                             encoding='utf-8')
    return rows, summary


def build_state(*, scramble=None, record=None, letters=None):
    """Build the scramble or the checked census end, then apply supplied letters."""
    from differential.leads import census

    if (scramble is None) == (record is None):
        raise ValueError('supply exactly one of --scramble and --record')
    if letters is not None and record is None:
        raise ValueError('--letters requires --record')
    if scramble is not None:
        k, seed = scramble
        if k < 0:
            raise ValueError('scramble length must be non-negative')
        state, _ = census.restore.scramble(census.menu(), k, seed)
        source = {'kind': 'scramble', 'k': k, 'seed': seed}
    else:
        state = None
        for step in census.walk(record, end=True):
            state = step.S.st
        if state is None:
            raise ValueError('census record did not yield an end state')
        source = {'kind': 'census-end', 'record_sha256': hashlib.sha256(Path(record).read_bytes()).hexdigest()}
    if letters is not None:
        word = json.loads(Path(letters).read_text(encoding='utf-8'))
        menu = census.menu()
        if (not isinstance(word, list)
                or any(not isinstance(pair, list) or len(pair) != 2
                       or type(pair[0]) is not int or not 0 <= pair[0] < 600
                       or type(pair[1]) is not int or not 0 <= pair[1] < len(menu.items)
                       for pair in word)):
            raise ValueError('--letters must contain a JSON list of [grip, item] integer pairs in the menu')
        twists = {}
        for index, (cap, item) in enumerate(word):
            if cap not in twists:
                twists[cap] = menu.for_cap(cap)
            outcome = state.apply(twists[cap][item])
            if not outcome.applied:
                raise ValueError(f'letter {index} is {outcome.status}')
        source.update(letters=len(word), letters_sha256=hashlib.sha256(Path(letters).read_bytes()).hexdigest())
    return state, source


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--scramble', nargs=2, type=int, metavar=('K', 'SEED'))
    modes.add_argument('--record', type=Path)
    parser.add_argument('--letters', type=Path)
    parser.add_argument('--expect')
    parser.add_argument('--caps', nargs='+', type=int)
    args = parser.parse_args(argv)
    try:
        # Refuse bad destinations before rebuilding potentially expensive inputs.
        output_directory(args.out, ('x2-caps.jsonl', 'x2-summary.json'))
        caps = selected_caps(args.caps)
        state, source = build_state(scramble=args.scramble, record=args.record, letters=args.letters)
        _, summary = run(state, args.out, caps, expect=args.expect, source=source)
    except ValueError as error:
        parser.error(str(error))
    return 0 if summary['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
