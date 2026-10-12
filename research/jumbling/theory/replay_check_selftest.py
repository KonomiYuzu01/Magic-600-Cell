"""Exercise replay_check's CLI on the handed-over restoration words and forged claims.

All generated inputs, reports and pose tables live in a system temporary directory.
The known-good words and exact simulator supply expectations independently of the checker.
The positive CLI run saves its real orient.start result for reuse by the negative
tests, avoiding a second replay of the 17,670 setup twists.
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
CHECKER = HERE / 'replay_check.py'
T0 = time.time()

# Observe the real setup call in the CLI process. The original start function
# still checks every setup twist and digest, and returns the same Restorer.
CAPTURE_START = """
import json, runpy, sys
from pathlib import Path
table, checker = Path(sys.argv[1]), Path(sys.argv[2])
sys.argv = sys.argv[2:]
here = checker.resolve().parent
sys.path.insert(0, str(here))
sys.path.insert(0, str(here.parent))
sys.path.insert(0, str(here.parent / 'fixtures'))
import orient as O
from sim.kplus import matrix_json
original_start = O.start
def capture(*args, **kwargs):
    R, M = original_start(*args, **kwargs)
    st = R.st
    doc = {'poses': [matrix_json(st._pose_matrix(pid)) for pid in range(len(st._poses))],
           'pose_id': st.pose_id.tolist(), 'digest': st.digest()}
    table.write_text(json.dumps(doc) + '\\n')
    return R, M
O.start = capture
runpy.run_path(str(checker), run_name='__main__')
"""


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def read(path):
    return json.loads(Path(path).read_text())


def write(path, value):
    Path(path).write_text(json.dumps(value) + '\n')
    return path


def run(tmp, label, args, expected_code, start_export=None):
    out = tmp / f'{label}-report.json'
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1', OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1')
    command = [sys.executable, str(CHECKER), *map(str, args), '--out', str(out)]
    if start_export is not None:
        command = [sys.executable, '-c', CAPTURE_START, str(start_export), *command[1:]]
    log(test=label, expected_exit=expected_code)
    result = subprocess.run(command, env=env, text=True, stdout=subprocess.PIPE,
                            timeout=1200)
    details = out.read_text() if result.returncode != expected_code and out.is_file() else ''
    require(result.returncode == expected_code,
            f'{label}: exit {result.returncode}, expected {expected_code}\n{result.stdout}\n{details}')
    require(out.is_file(), f'{label}: no JSON report')
    report = read(out)
    require(report['pass'] is (expected_code == 0), f'{label}: pass disagrees with exit code')
    log(test=label, exit=result.returncode, seconds=report['seconds'])
    return report


def simulator_cases(table, first_word):
    """Find real admissible substitutions, collateral and a straddled cut at the 277 state."""
    import numpy as np

    sys.path.insert(0, str(HERE))
    sys.path.insert(0, str(HERE.parent))
    sys.path.insert(0, str(HERE.parent / 'fixtures'))
    import classes as CL
    import orient as O
    import restore

    members, cls = CL.classes()
    R, M = O.start(table)
    st = R.st
    start_doc = read(table)
    start_digest = st.digest()
    require(st.off_lattice_count() == 277, 'negative tests did not start at 277')

    forged = read(first_word)
    # Change the last pair, so that the entire remaining word is still replayable.
    for pair in forged['word'][:-1]:
        R.apply(*pair)
    c, original = forged['word'][-1]
    replacement = None
    for i in range(len(M.items)):
        if i == original:
            continue
        try:
            R.apply(c, i)
        except RuntimeError:
            continue
        try:
            if st.digest() != forged['output_digest']:
                replacement = i
                break
        finally:
            R.undo()
    require(replacement is not None, 'could not find an admissible forged menu index')
    forged['word'][-1][1] = replacement

    O.set_pose_table(st, start_doc)
    R = restore.Restorer(st, M)  # discard caches belonging to the forged prefix
    off0 = st._kpose()[st.pose_id] < 0
    poses0 = {p: st.pose(p) for p in np.flatnonzero(off0).tolist()}
    collateral_case = None
    for c in range(600):
        cut = st.classify(c, need_inside=True, stop_at_straddle=False)
        if cut.straddle is not None or cut.uncertain or not (~off0)[cut.inside].any():
            continue
        for i in restore.jumble_items(M):
            try:
                R.apply(c, i)
            except RuntimeError:
                continue
            try:
                off1 = st._kpose()[st.pose_id] < 0
                ids = np.flatnonzero(~off0 & off1).tolist()
                D = R.domains()
                depth_one = all(len(CL.hub(D.reps[d], members, cls)) == 1
                                for d in np.unique(D.dom[off1]).tolist())
                if ids and depth_one:
                    changed = [p for p in np.flatnonzero(off0 & off1).tolist() if st.pose(p) != poses0[p]]
                    require(len(ids) == int((~off0)[cut.inside].sum()),
                            'jumble did not take precisely the cap lattice pieces off')
                    collateral_case = {'word': [[c, i]], 'collateral_ids': ids,
                                       'restored': int((off0 & ~off1).sum()),
                                       'pose_changed_ids': changed, 'off_end': int(off1.sum()),
                                       'end_digest': st.digest()}
                    break
            finally:
                R.undo()
        if collateral_case is not None:
            break
    require(collateral_case is not None, 'could not find depth-one collateral at 277')
    require(st.digest() == start_digest, 'candidate trials changed the start state')

    refused = None
    for c in range(600):
        if st.classify(c, need_inside=True, stop_at_straddle=False).straddle is not None:
            i = restore.jumble_items(M)[0]
            try:
                R.apply(c, i)
            except RuntimeError:
                refused = [[c, i]]
            require(refused is not None, 'a straddled cut accepted a twist')
            break
    require(refused is not None, 'could not find an inadmissible twist at 277')
    require(st.digest() == start_digest, 'refused twist changed the start state')
    log(found_forged=[forged['word'][-1][0], replacement],
        found_collateral=collateral_case['word'], collateral=len(collateral_case['collateral_ids']),
        found_inadmissible=refused)
    return forged, collateral_case, refused


def exercise(tmp):
    require(CHECKER.is_file(), 'replay_check.py has not been implemented')
    results = HERE / 'results'
    setup = sorted((results / 'e-chain').glob('*.json'))
    require(len(setup) == 6, 'expected six e-chain word files')
    tail = results / 'e-chain-104'
    setup.append(tail / '07-word-277-preparation.json')
    words = [tail / name for name in ('08-word-residue-3-to-2.json', '09-word-residue-2-to-1.json',
                                      '10-word-residue-1-to-0.json', '11-word-restore-86.json')]
    require(sum(len(read(p)['word']) for p in setup) == 17670, 'setup fixture twist count changed')
    require(sum(len(read(p)['word']) for p in words) == 2437, 'checked fixture twist count changed')
    expected_digest = read(words[-1])['output_digest']
    require(expected_digest.startswith('7244ef1c11f4'), 'end fixture identity changed')
    claims = {'end_digest': expected_digest, 'restored': 86, 'off_at_end': 191,
              'collateral': 0, 'pose_changed': 0}
    good_claims = write(tmp / 'good-claims.json', claims)
    end_table = tmp / 'end-table.json'
    start_table = tmp / 'start-table.json'
    positive = run(tmp, 'positive', ['--start-words', *setup, '--words', *words,
                                    '--depth-every', '1', '--claims', good_claims, '--export', end_table], 0,
                   start_export=start_table)
    require(positive['start_digest'] == read(words[0])['input_digest'], 'wrong checked start digest')
    require(positive['start_off'] == 277, 'positive start off count')
    require(len(positive['files']) == 4, 'setup words were reported as checked files')
    for row, path in zip(positive['files'], words):
        n = len(read(path)['word'])
        require(row['twists'] == n, f'{path.name}: twist count')
        require(row['input_digest_ok'] is True and row['output_digest_ok'] is True,
                f'{path.name}: digest check')
        require(row['end_digest'] == read(path)['output_digest'], f'{path.name}: end digest')
        require(row['depth_tested'] == n and row['depth_failures'] == 0
                and row['first_depth_failure'] is None, f'{path.name}: prefix depth check')
        require(row['inadmissible'] is None, f'{path.name}: refused twist')
        require(row['off_min'] <= row['off_end'] <= row['off_max'], f'{path.name}: off count bounds')
    total = positive['total']
    require(total['twists'] == 2437 and total['depth_tested'] == 2437, 'positive total prefix count')
    require(total['input_digest_ok'] is True and total['output_digest_ok'] is True, 'total digests')
    require(total['depth_failures'] == 0 and total['first_depth_failure'] is None, 'total depth failures')
    for key, expected in (('restored', 86), ('collateral', 0), ('pose_changed', 0), ('off_end', 191)):
        require(total[key] == expected, f'positive total {key}: {total[key]} != {expected}')
        require(positive['files'][-1][key] == expected, f'last file {key} was compared to the wrong start')
    require(total['end_digest'] == expected_digest, 'positive end digest')
    require(total['collateral_ids'] == [] and total['pose_changed_ids'] == [], 'positive endpoint IDs')
    for key, expected in claims.items():
        claim = positive['claims'][key]
        require(claim == {'expected': expected, 'actual': expected, 'ok': True}, f'positive claim {key}')

    round_trip = run(tmp, 'round-trip', ['--table', end_table], 0)
    require(round_trip['start_digest'] == expected_digest, 'exported table did not round trip')
    require(round_trip['start_off'] == 191 and round_trip['total']['twists'] == 0, 'round-trip state')
    require(round_trip['files'] == [] and round_trip['total']['restored'] == 0, 'empty run endpoints')

    require(read(start_table)['digest'] == positive['start_digest'], 'captured setup state has the wrong digest')
    forged, collateral, refused = simulator_cases(start_table, words[0])

    forged_path = write(tmp / 'forged-word.json', forged)
    negative_a = run(tmp, 'forged-word', ['--table', start_table, '--words', forged_path], 1)
    row = negative_a['files'][0]
    require(row['input_digest_ok'] is True and row['output_digest_ok'] is False, 'forged digest not rejected')
    require(row['twists'] == len(forged['word']) and row['inadmissible'] is None, 'forged word was not replayed')

    collateral_path = write(tmp / 'collateral-word.json', collateral['word'])  # bare word format
    negative_b = run(tmp, 'collateral-observed', ['--table', start_table, '--words', collateral_path,
                                                '--depth-every', '100'], 0)
    for row in (negative_b['files'][0], negative_b['total']):
        require(row['collateral'] == len(collateral['collateral_ids']), 'collateral count disagrees with simulator')
        require(row['collateral_ids'] == collateral['collateral_ids'][:200], 'collateral IDs or truncation')
        require(row['pose_changed'] == len(collateral['pose_changed_ids']), 'exact changed-pose count')
        require(row['pose_changed_ids'] == collateral['pose_changed_ids'][:200], 'exact changed-pose IDs')
        require(row['restored'] == collateral['restored'] and row['off_end'] == collateral['off_end'],
                'collateral endpoint counts')
        require(row['end_digest'] == collateral['end_digest'], 'collateral endpoint digest')
        require(row['depth_tested'] == 1 and row['depth_failures'] == 0, 'last prefix was not depth tested')
        require(row['input_digest_ok'] is None and row['output_digest_ok'] is None, 'absent digests were invented')
    bad_claims = write(tmp / 'bad-claims.json', {'collateral': 0})
    claimed_b = run(tmp, 'collateral-claimed-zero', ['--table', start_table, '--words', collateral_path,
                                                 '--claims', bad_claims], 1)
    require(claimed_b['claims']['collateral'] == {'expected': 0, 'actual': len(collateral['collateral_ids']),
                                               'ok': False}, 'false protection claim not rejected')

    refused_path = write(tmp / 'refused-word.json', refused)
    empty = write(tmp / 'empty-word.json', [])
    negative_c = run(tmp, 'inadmissible', ['--table', start_table, '--words', refused_path, empty], 1)
    require(len(negative_c['files']) == 1, 'replay continued after an inadmissible twist')
    for row in (negative_c['files'][0], negative_c['total']):
        failure = row['inadmissible']
        require(failure is not None and failure['prefix_index'] == 0, 'refused prefix index missing')
        require(failure['file'] == str(refused_path), 'refused file missing')
        require([failure['grip'], failure['menu_index']] == refused[0], 'refused twist missing')
        require(row['twists'] == 0 and row['end_digest'] == positive['start_digest'], 'refused twist changed state')

    # Non-replayable digest failures must not prevent later files being checked.
    wrong = write(tmp / 'wrong-digests.json', {'word': [], 'input_digest': 'forged', 'output_digest': 'forged'})
    continued = run(tmp, 'digest-continuation', ['--table', start_table, '--words', wrong, empty], 1)
    require(len(continued['files']) == 2, 'a digest failure stopped replay')
    require(continued['files'][0]['input_digest_ok'] is False
            and continued['files'][0]['output_digest_ok'] is False, 'wrong digests accepted')
    require(continued['total']['input_digest_ok'] is False
            and continued['total']['output_digest_ok'] is False, 'total lost a digest failure')

    require(time.time() - T0 < 1200, 'self-test exceeded the 20-minute limit')
    return {'pass': True, 'twists': 2437, 'start_off': 277, 'restored': 86,
            'collateral': 0, 'pose_changed': 0, 'off_end': 191, 'end_digest': expected_digest,
            'seconds': round(time.time() - T0, 3)}


def main():
    tmp = Path(tempfile.mkdtemp(prefix='magic600-replay-check-'))
    try:
        print(json.dumps(exercise(tmp)), flush=True)
        return 0
    except (AssertionError, OSError, subprocess.TimeoutExpired) as exc:
        print(f'replay_check self-test failed: {exc}', file=sys.stderr, flush=True)
        return 1
    finally:
        shutil.rmtree(tmp)


if __name__ == '__main__':
    sys.exit(main())
