"""Packet acceptance proof for buffer vacancy transport, using fresh temp data.

Run from the repository root: ``python research/jumbling/theory/buffer_selftest.py``.
The seven source words are replayed with digest/admissibility checks; no saved
X0 table or precomputed certificate supplies the result. Artifacts live only
under ``tempfile.mkdtemp()`` and are removed on exit. This is headless, exact
synthetic geometry evidence, not Windows/DirectX or performance evidence.
"""
import copy
import json
import random
import shutil
import sys
import tempfile
import time
from pathlib import Path
from types import SimpleNamespace

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import buffer as B  # noqa: E402
import numpy as np  # noqa: E402

X0 = '3c086301091a2dad43b5efd1d1702ce005976a5f54ecbc9d22b520e49aba106b'
FIXED = [119124, 119141, 119151, 128877, 128888]
INVENTORIES = [
    (1439, 2, 3), (2396, 2, 3), (3590, 3, 4), (3598, 3, 6), (3600, 1, 5),
    (7158, 1, 1), (7174, 3, 3), (7184, 6, 7), (7185, 8, 11), (7187, 4, 6),
    (7191, 8, 11), (7192, 6, 10), (7193, 8, 13), (7195, 4, 8), (7196, 8, 14),
    (7197, 6, 12), (7198, 4, 11), (7199, 2, 11), (7200, 2, 22),
]


def require(ok, message):
    if not ok:
        raise AssertionError(message)


def rejects(fn, fragment):
    try:
        fn()
    except ValueError as error:
        require(fragment in str(error), f'unexpected rejection: {error}')
    else:
        raise AssertionError(f'expected rejection containing {fragment!r}')


def check_weighted_edges():
    """A conjugate endpoint must also propagate through ordinary letter edges."""
    G = SimpleNamespace(inverse=lambda word: list(reversed(word)))
    tr = SimpleNamespace(G=G, NP=5, orbit=np.zeros(5, np.int64), edges=np.array([0, 4]),
                         src=np.array([1, 2, 2, 3]), dst=np.array([2, 1, 3, 2]), lid=np.array([0, 0, 1, 1]))
    free = np.ones(4, bool)
    before = B.weighted_tree(tr, 0, free, [])
    require(before[0].tolist() == [0, -1, -1, -1, -1], 'unexpected initial weighted graph')
    extra = [{'from': 1, 'to': 0, 'word': [2, 3, 2]}]
    tree = B.weighted_tree(tr, 0, free, [], extra)
    require(tree[0].tolist() == [0, 3, 4, 5, -1], 'new conjugate endpoint did not propagate letter routes')
    require(B.tree_path(tr, tree, [], 3, extra) == [2, 3, 2, 0, 1], 'conjugate route expansion differs')
    p = np.array([4, 1, 2, 3, 0])
    macro = ([0, 1], p, np.array([0, 4]))
    tree = B.weighted_tree(tr, 0, free, [macro], extra)
    require(tree[0][4] == 2 and B.tree_path(tr, tree, [macro], 4, extra) == [0, 1],
            'weighted macro route expansion differs')


def main():
    started = time.monotonic()
    check_weighted_edges()
    directory = Path(tempfile.mkdtemp(prefix='buffer-selftest-'))
    try:
        words = sorted((B.HERE / 'results/e-chain').glob('*.json'))
        require(len(words) == 6, 'expected six sorted e-chain source files')
        words.append(B.HERE / 'results/e-chain-104/07-word-277-preparation.json')
        B.log(selftest_stage='source_replay', word_files=len(words))
        R, M = B.O.start(words=[str(p) for p in words])
        require(R.st.digest() == X0 and R.st.off_lattice_count() == 277, 'X0 digest or off count differs')
        table = directory / 'x0.json'
        B.export_pose_table(R.st, table)
        A = B.Alphabet(R, M)
        members, _ = B.CL.classes()
        target = B.cell_chambers(A.home, members, 0, [369, 507])
        orbit, sizes = A.orbits()
        cond = B.condition(A, target)
        fixed = target[sizes[orbit[target]] == 1].tolist()
        require(len(A.safe) == 570 and len(A.letters) == 6270, 'SAFE-ALL alphabet counts differ')
        require(len(target) == 86 and len(cond['by_orbit']) == 24, 'target or orbit count differs')
        require(fixed == FIXED and cond['fixed_vacant'], 'fixed target chambers or vacancies differ')
        actual = [(r['size'], r['target'], r['holes']) for r in cond['by_orbit'] if r['size'] > 1]
        require(sorted(actual) == sorted(INVENTORIES), 'orbit inventory multiset differs')
        require(cond['necessary'], 'necessary clearing condition does not hold at X0')
        require(np.all(np.diff(A.orbit_least) > 0), 'orbits are not numbered by least chamber')
        rng = random.Random(19)
        w = [rng.randrange(len(A.letters)) for _ in range(25)]
        p = A.perm(w)
        points = np.array(rng.sample(range(A.NP), 50))
        require(np.array_equal(A.images(w, points), p[points]), 'point images differ from the full word permutation')
        require(np.array_equal(p[A.perm(A.inverse(w))], np.arange(A.NP)), 'inverse word differs from inverse permutation')
        B.log(selftest_stage='alphabet_and_inventories_passed')

        jc = B.jordan(A, target, seed=1)
        require(not jc['missing'] and len(jc['certificates']) == 19, 'Jordan search did not certify all 19 orbits')
        jordan_file = directory / 'jordan.json'
        jordan_file.write_text(json.dumps(jc) + '\n')
        saved_jordan = json.loads(jordan_file.read_text())
        for o, cert in saved_jordan['certificates'].items():
            checked = B.check_jordan(A, cert['word'], int(o), cert['prime'])
            require(checked['prime'] == cert['prime'] and checked['cycle_least_point'] == cert['cycle_least_point'],
                    'independent Jordan word verification differs')
        first = next(iter(saved_jordan['certificates'].values()))
        rejects(lambda: B.check_jordan(A, [], first['orbit']), 'no Jordan certificate')
        rejects(lambda: B.check_jordan(A, first['word'], first['orbit'], prime=2), 'no Jordan certificate')
        require(jc['uncertifiable_below_8'] == sorted(orbit[fixed].tolist()), 'small uncertifiable orbits differ')
        B.log(selftest_stage='jordan_words_independently_verified', certificates=19)

        cert = B.certificate(A, 0, [369, 507])
        require(cert['peeled'] == cert['of'] == 81, 'certificate does not peel all 81 movable targets')
        require(cert['all_reached'] and cert['thresholds_equal_targets'], 'routes or orbit thresholds are incomplete')
        require(cert['family_bound'] <= 1030, 'family bound exceeds 1030 letters')
        cert_file = directory / 'certificate.json'
        # The CLI output envelope, as well as standalone certificate files, is reusable.
        cert_file.write_text(json.dumps({'certificate': cert}) + '\n')
        reloaded = B.load_certificate(cert_file, A, 0, [369, 507])
        require(reloaded == cert, 'certificate round trip differs')
        cond = B.condition(A, target, reloaded)
        require(cond['necessary'] and cond['sufficient'], 'certificate state conditions fail')
        depleted = np.zeros(A.NP, bool)
        depleted[A.holes] = True
        depleted[FIXED[0]] = False
        failure = B.condition(A, target, reloaded, depleted)
        require(not failure['necessary'] and not failure['sufficient'] and failure['fixed_occupied'] == [FIXED[0]],
                'occupied fixed chamber was not reported')
        depleted[FIXED[0]] = True
        row = next(r for r in cond['by_orbit'] if r['size'] > 1)
        remove = np.flatnonzero(depleted & (A.orbit == row['orbit']))
        depleted[remove[:row['holes'] - row['target'] + 1]] = False
        failure = B.condition(A, target, reloaded, depleted)
        require(not failure['necessary'] and not failure['sufficient'] and
                any(r['orbit'] == row['orbit'] and r['deficit'] == 1 for r in failure['necessary_deficits']),
                'orbit inventory deficit was not reported')
        forged = copy.deepcopy(cert)
        forged['off_geometry_fingerprint'] = '0' * 64
        forged_file = directory / 'forged.json'
        forged_file.write_text(json.dumps(forged) + '\n')
        rejects(lambda: B.load_certificate(forged_file, A, 0, [369, 507]), 'fingerprint')
        wrong_target = copy.deepcopy(cert)
        wrong_target['target']['chambers'].pop()
        rejects(lambda: B.validate_certificate(A, wrong_target), 'target chambers')
        B.log(selftest_stage='certificate_verified', family_bound=cert['family_bound'])

        # Exercise the actual --certificate-in/--demo interface on exported X0.
        demo_file = directory / 'demo.json'
        require(B.main(['--table', str(table), '--cls', '0', '--cell', '369', '507',
                        '--certificate-in', str(cert_file), '--demo', '300', '1', '--out', str(demo_file)]) == 0,
                'CLI demo failed')
        result = json.loads(demo_file.read_text())
        d = result['demo']
        require(d['generator_length'] == 300 and len(d['generator_word']) == 300, 'demo generator length differs')
        require(d['certificate_reused'] and d['fingerprint_matches'] and d['constructor_journal_empty'],
                'demo did not reuse the certificate from a history-free table')
        require(d['condition']['necessary'] and d['condition']['sufficient'], 'demo source inventory condition fails')
        require(d['preparation_prefix_checks'] == len(d['preparation_word']) == d['preparation_length'] and
                d['off_prefixes_unchanged'], 'not every preparation prefix preserved exact off geometry')
        require(d['predicted_equals_actual'] and d['target_vacant_after_preparation'], 'demo did not clear the target')
        require(d['preparation_length'] <= cert['family_bound'] and d['off_after'] == 277 and d['restored'] == 0 and
                not d['pieces_taken_off'] and d['remaining_off_poses_equal_source'], 'demo off-piece or length proof failed')
        require(d['source_pose_table']['journal'] == [], 'exported source table contains a journal')
        require(len(d['predicted_holes']) == 277, 'demo predicted hole count differs')
        # Also refuse a forged fingerprint at the demo's exported, changed state.
        changed_table = directory / 'changed.json'
        changed_table.write_text(json.dumps(d['source_pose_table']) + '\n')
        fresh, menu = B._fresh_restorer(changed_table)
        changed = B.Alphabet(fresh, menu)
        rejects(lambda: B.load_certificate(forged_file, changed, 0, [369, 507]), 'fingerprint')
        rejects(lambda: B.main(['--table', str(changed_table), '--cls', '0', '--cell', '369', '507',
                                '--certificate-in', str(forged_file), '--out', str(directory / 'refused.json')]),
                'fingerprint')
        require(not (directory / 'refused.json').exists(), 'refused certificate wrote a successful output')
        summary = {'acceptance': 'passed', 'state_digest': X0, 'off_lattice': 277, 'safe_all_grips': 570,
                   'letters': 6270, 'target_chambers': 86, 'orbits_meeting_target': 24,
                   'jordan_certificates': 19, 'peeled': 81, 'family_bound': cert['family_bound'],
                   'demo_generator_length': 300, 'demo_seed': 1, 'demo_preparation_length': d['preparation_length'],
                   'demo_filled': d['filled'], 'demo_prefix_checks': d['preparation_prefix_checks'],
                   'demo_off_after': d['off_after'], 'demo_source_digest': d['source_digest'],
                   'demo_after_preparation_digest': d['after_preparation_digest'],
                   'seconds': round(time.monotonic() - started, 2)}
        print(json.dumps(summary), flush=True)
        return 0
    finally:
        shutil.rmtree(directory)


if __name__ == '__main__':
    sys.exit(main())
