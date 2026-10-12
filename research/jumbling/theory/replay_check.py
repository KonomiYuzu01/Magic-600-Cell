"""Independently replay native restoration words on the exact jumbling simulator.

Run from the repository root:
    python research/jumbling/theory/replay_check.py --start-words <setup.json> ... \
        --words <word.json> ... --claims <claims.json> --export <table.json> --out <report.json>

Prefix indices in failures are zero based; depth sampling counts completed twists
from one within each file. Off-count bounds include the empty prefix. Endpoint
counts always compare against the start of --words, including at file boundaries.
Missing word digests are reported as null. Collateral and changed poses fail only
when they disagree with a claim. An inadmissible twist stops the entire replay.

Evidence kind: exact simulator replay and synthetic geometry, not native-host evidence.
"""
import argparse
import json
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import classes as CL  # noqa: E402
import orient as O  # noqa: E402
from sim.kplus import matrix_json, to_tuple  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def read_word(name, n_items):
    doc = json.loads(Path(name).read_text())
    if isinstance(doc, list):
        doc = {'word': doc}
    if not isinstance(doc, dict) or not isinstance(doc.get('word'), list):
        raise ValueError('word file must be a list of pairs or an object with a word list')
    for key in ('input_digest', 'output_digest'):
        if key in doc and not isinstance(doc[key], str):
            raise ValueError(f'{key} must be a string')
    for index, pair in enumerate(doc['word']):
        if (not isinstance(pair, list) or len(pair) != 2 or any(type(x) is not int for x in pair)
                or not 0 <= pair[0] < 600 or not 0 <= pair[1] < n_items):
            raise ValueError(f'prefix {index}: expected integer (grip, menu_index) in the native menu')
    return doc


def row(name, digest, off):
    return {'name': str(name), 'twists': 0, 'input_digest_ok': None, 'output_digest_ok': None,
            'end_digest': digest, 'off_min': off, 'off_max': off, 'off_end': off,
            'depth_tested': 0, 'depth_failures': 0, 'first_depth_failure': None,
            'restored': 0, 'collateral': 0, 'collateral_ids': [],
            'pose_changed': 0, 'pose_changed_ids': [], 'inadmissible': None}


def endpoint(st, off0, poses0):
    off = st._kpose()[st.pose_id] < 0
    collateral = np.flatnonzero(~off0 & off).tolist()
    still_off = np.flatnonzero(off0 & off).tolist()
    # Pose IDs are local to the current compacted table; compare exact matrices.
    current = {int(pid): matrix_json(st._pose_matrix(int(pid)))
               for pid in np.unique(st.pose_id[still_off]).tolist()}
    changed = [p for p in still_off if current[int(st.pose_id[p])] != poses0[p]]
    return {'end_digest': st.digest(), 'off_end': int(off.sum()),
            'restored': int((off0 & ~off).sum()), 'collateral': len(collateral),
            'collateral_ids': collateral[:200], 'pose_changed': len(changed),
            'pose_changed_ids': changed[:200]}


def depth_failure(R, off, members, cls, hubs):
    if not off.any():
        return []
    D = R.domains()
    bad = []
    for d in np.unique(np.asarray(D.dom)[off]).tolist():
        rep = to_tuple(D.reps[d])
        # A hub depends only on this exact matrix and the fixed pole classes.
        # Cache by matrix, never by the domain index (which can change each twist).
        if rep not in hubs:
            hubs[rep] = len(CL.hub(rep, members, cls))
        if hubs[rep] != 1:
            bad.append(int(d))
    return bad


def test_depth(result, R, off, index, members, cls, hubs):
    result['depth_tested'] += 1
    bad = depth_failure(R, off, members, cls, hubs)
    if bad:
        result['depth_failures'] += 1
        if result['first_depth_failure'] is None:
            result['first_depth_failure'] = {'file': result['name'], 'prefix_index': index, 'domains': bad}


def combined_digest(files, key):
    checked = [r[key] for r in files if r[key] is not None]
    return all(checked) if checked else None


def check(a):
    R, M = O.start(a.table, a.start_words)
    st = R.st
    off0 = st._kpose()[st.pose_id] < 0
    start_digest, start_off = st.digest(), int(off0.sum())
    off_pieces = np.flatnonzero(off0).tolist()
    matrices = {int(pid): matrix_json(st._pose_matrix(int(pid)))
                for pid in np.unique(st.pose_id[off_pieces]).tolist()}
    poses0 = {p: matrices[int(st.pose_id[p])] for p in off_pieces}
    files = []
    total = row('total', start_digest, start_off)
    members, cls = CL.classes() if a.words else (None, None)
    hubs = {}
    for name in a.words:
        before = st.digest()
        result = row(name, before, st.off_lattice_count())
        files.append(result)
        try:
            doc = read_word(name, len(M.items))
        except (OSError, ValueError) as exc:
            result['error'] = str(exc)
            result.update(endpoint(st, off0, poses0))
            break
        if 'input_digest' in doc:
            result['input_digest_ok'] = before == doc['input_digest']
        last_tested = 0
        for index, (c, i) in enumerate(doc['word']):
            try:
                R.apply(c, i)
            except RuntimeError as exc:
                result['inadmissible'] = {'file': str(name), 'prefix_index': index,
                                          'grip': c, 'menu_index': i, 'error': str(exc)}
                break
            result['twists'] += 1
            off = st._kpose()[st.pose_id] < 0
            count = int(off.sum())
            result['off_min'] = min(result['off_min'], count)
            result['off_max'] = max(result['off_max'], count)
            if result['twists'] % a.depth_every == 0 or index == len(doc['word']) - 1:
                test_depth(result, R, off, index, members, cls, hubs)
                last_tested = result['twists']
            if result['twists'] % 250 == 0:
                log(file=Path(name).name, twists=result['twists'], off=count,
                    depth_failures=result['depth_failures'])
        # Also check the last successful prefix when a file stops on a refused twist.
        if result['twists'] > last_tested:
            test_depth(result, R, st._kpose()[st.pose_id] < 0, result['twists'] - 1, members, cls, hubs)
        result.update(endpoint(st, off0, poses0))
        if 'output_digest' in doc:
            result['output_digest_ok'] = result['end_digest'] == doc['output_digest']
        log(file=Path(name).name, twists=result['twists'], off=result['off_end'],
            input_digest_ok=result['input_digest_ok'], output_digest_ok=result['output_digest_ok'],
            depth_failures=result['depth_failures'], inadmissible=result['inadmissible'])
        if result['inadmissible'] is not None:
            break

    total.update(endpoint(st, off0, poses0))
    for key in ('twists', 'depth_tested', 'depth_failures'):
        total[key] = sum(r[key] for r in files)
    total['off_min'] = min([start_off] + [r['off_min'] for r in files])
    total['off_max'] = max([start_off] + [r['off_max'] for r in files])
    for key in ('input_digest_ok', 'output_digest_ok'):
        total[key] = combined_digest(files, key)
    for key in ('first_depth_failure', 'inadmissible'):
        total[key] = next((r[key] for r in files if r[key] is not None), None)
    report = {'start_digest': start_digest, 'start_off': start_off, 'files': files,
              'total': total, 'claims': {}, 'pass': all(
                  r['input_digest_ok'] is not False and r['output_digest_ok'] is not False
                  and r['depth_failures'] == 0 and r['inadmissible'] is None and 'error' not in r
                  for r in files)}
    if a.claims:
        try:
            claims = json.loads(Path(a.claims).read_text())
            if not isinstance(claims, dict):
                raise ValueError('claims must be a JSON object')
            fields = {'end_digest': 'end_digest', 'restored': 'restored', 'off_at_end': 'off_end',
                      'collateral': 'collateral', 'pose_changed': 'pose_changed'}
            for key, expected in claims.items():
                actual = total[fields[key]] if key in fields else None
                ok = key in fields and type(expected) is type(actual) and expected == actual
                report['claims'][key] = {'expected': expected, 'actual': actual, 'ok': ok}
                report['pass'] = report['pass'] and ok
        except (OSError, ValueError) as exc:
            report['error'] = f'claims: {exc}'
            report['pass'] = False
    if a.export:
        table = {'poses': [matrix_json(st._pose_matrix(pid)) for pid in range(len(st._poses))],
                 'pose_id': st.pose_id.tolist(), 'digest': total['end_digest']}
        Path(a.export).write_text(json.dumps(table) + '\n')
    return report


def positive_int(value):
    n = int(value)
    if n < 1:
        raise argparse.ArgumentTypeError('depth interval must be positive')
    return n


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--table', help='current-only pose table (default: the E2 end)')
    ap.add_argument('--start-words', nargs='*', default=[], help='setup word files checked by orient.start')
    ap.add_argument('--words', nargs='*', default=[], help='native word files to check, in chronological order')
    ap.add_argument('--depth-every', type=positive_int, default=1, help='test depth every n twists and at file ends')
    ap.add_argument('--claims', help='JSON endpoint claims for all checked words together')
    ap.add_argument('--export', help='write a current-only exact end pose table')
    ap.add_argument('--out', required=True, help='write the JSON report')
    a = ap.parse_args(argv)
    began = time.time()
    try:
        report = check(a)
    except (AssertionError, OSError, ValueError, RuntimeError, KeyError, IndexError, TypeError) as exc:
        # A rejected table or setup word cannot supply a validated checked start.
        report = {'start_digest': None, 'start_off': None, 'files': [],
                  'total': row('total', None, None), 'claims': {}, 'pass': False,
                  'error': str(exc)}
    report['seconds'] = round(time.time() - began, 3)
    try:
        Path(a.out).write_text(json.dumps(report, indent=1) + '\n')
    except OSError as exc:
        log(error=str(exc), passed=False)
        return 1
    log(passed=report['pass'], twists=report['total']['twists'], end_digest=report['total']['end_digest'])
    return 0 if report['pass'] else 1


if __name__ == '__main__':
    sys.exit(main())
