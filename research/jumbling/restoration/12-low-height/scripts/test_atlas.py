"""LIT-A: real-state atlas certificates and independent adversarial checks.

Run from the repository root: python3 -I -B <this file>.
LITERAL_SLOW=1 also builds the other eight E2 middle-state certificates.
All generated evidence, logs and the home-centroid cache stay in one temporary
directory, including report.md. This is headless research evidence only.
"""
import contextlib
import copy
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
WORK = Path(tempfile.mkdtemp(prefix='lit-a-'))
os.environ['JUMBLING_HOME_CACHE'] = str(WORK / 'home_centroids.npy')
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_name] = '1'
ENV = dict(os.environ)
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RUNS = HERE.parent / 'runs/round22/line-a/replay2'
sys.path.insert(0, str(HERE))
import r22b10_cert as C
import check_r22b10 as K
import numpy as np

MASTER = [25, 64, 67, 69, 16, 70, 17, 105, 81, 41, 31, 22, 116, 75, 82, 74,
          33, 23, 32, 115, 108, 106, 83, 90, 34, 66, 65, 14, 12]
LENGTHS = {25: 10, 64: 46, 67: 22, 69: 46, 16: 94, 70: 46, 17: 190,
           105: 9214, 81: 46, 41: 142, 31: 766, 22: 766, 116: 190, 75: 1534,
           82: 46, 74: 46, 33: 70, 23: 1534, 32: 766, 115: 46, 108: 94,
           106: 142, 83: 94, 90: 94, 34: 142, 66: 142, 65: 1534, 14: 286, 12: 562}
GROUP_A = {64, 67, 69, 70, 81, 41, 82, 74, 33, 34, 66, 65}
GROUP_B = {105, 75, 115, 108, 106, 83, 90}
GROUP_C = {25, 17, 31, 22, 23, 32, 14}
GROUP_D = {16, 12}
GROUP_E = {116}
CASES = [
    ('b14', 'run1/z-b12.json.gz', 'run1/b14-target.json', 19, GROUP_A | GROUP_B),
    ('b16', 'run1/z-b14.json.gz', 'run1/b16-target.json', 19, GROUP_A | GROUP_B),
    ('b20', 'run1/z-b16.json.gz', 'run1/b20-target.json', 19, GROUP_A | GROUP_B),
    ('b21', 'run1/z-b20.json.gz', 'run1/b21-target.json', 19, GROUP_A | GROUP_B),
    ('b07', 'run2/03-b07-z1-table.json.gz', 'run2/03-b07-v-target.json', 20, GROUP_A | GROUP_B | GROUP_E),
    ('b13', 'run2/04-b13-z1-table.json.gz', 'run2/04-b13-v-target.json', 20, GROUP_A | GROUP_B | GROUP_E),
    ('b19', 'run2/05-b19-z1-table.json.gz', 'run2/05-b19-v-target.json', 20, GROUP_A | GROUP_B | GROUP_E),
    ('b15', 'run2/09-b15-z1-table.json.gz', 'run2/09-b15-v-target.json', 19, GROUP_A | GROUP_C),
    ('b06+b05', 'b06-71/t1/b06-14-z1-table.json.gz', 'b06-71/t1/b06-14-v-target.json', 29,
     GROUP_A | GROUP_B | GROUP_C | GROUP_D | GROUP_E),
]
EVIDENCE = []


def build_case(case):
    name, source, target_path, _, _ = case
    table = WORK / f'{name}-table.json'
    with gzip.open(RUNS / source, 'rb') as stream:
        table.write_bytes(stream.read())
    target = json.loads((RUNS / target_path).read_text())
    start = time.monotonic()
    with (WORK / f'{name}-build.log').open('w') as log, contextlib.redirect_stdout(log):
        B = C.modules()
        import screen
        st, menu, _ = screen.load('table', str(table))
        G = B.Alphabet(SimpleNamespace(st=st), menu)
        cert = C.build(G, target, seconds=900, seeds='atlas')
    elapsed = time.monotonic() - start
    path = WORK / f'{name}-cert.json.gz'
    C.write_result(str(path), cert)
    fields = ('least', 'size', 'k', 'atlas_type', 'atlas_depth', 'sigma', 's', 'e', 'f')
    evidence = dict(state=name, orbit_order=cert['orbit_order'],
                    rows=[{key: row.get(key) for key in fields} for row in cert['orbits']],
                    complete=cert['complete'], missing=cert['missing'], V_length=cert['V_length'],
                    bound=cert['bound'], build_seconds=elapsed, check_seconds=None)
    EVIDENCE.append(evidence)
    return G, target, cert, table, path, evidence


def child(args, name, timeout=600, env=ENV):
    log = WORK / f'{name}.log'
    start = time.monotonic()
    with log.open('w') as stream:
        result = subprocess.run([sys.executable, '-I', '-B'] + list(map(str, args)),
                                cwd=REPO, env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=timeout)
    return result.returncode, log.read_text(), time.monotonic() - start


class AtlasCertificates(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.case = CASES[7]
        cls.G, cls.target, cls.cert, cls.table, cls.path, cls.evidence = build_case(cls.case)

    def catalogue(self, G, target, cert, case):
        _, _, _, count, expected = case
        orbit, sizes = G.orbits()
        actual = {int(np.flatnonzero(orbit == lab)[0]) for lab in set(orbit[target['T']].tolist()) if sizes[lab] > 1}
        self.assertEqual(actual, expected)
        self.assertEqual(len(actual), count)
        self.assertEqual(cert['format'], 'r22b10/2')
        self.assertEqual(cert['seeds'], 'atlas')
        self.assertEqual(cert['orbit_order'], [x for x in MASTER if x in actual])
        self.assertEqual([row['least'] for row in cert['orbits']], cert['orbit_order'])
        self.assertTrue(cert['complete'], cert['missing'])
        self.assertEqual(cert['missing'], [])
        self.assertLessEqual(cert['V_length'], cert['bound'])
        self.assertLessEqual(cert['bound'], 10**7)
        for row in cert['orbits']:
            with self.subTest(least=row['least']):
                self.assertEqual(row['route'], 'atlas')
                self.assertEqual(row['s'], LENGTHS[row['least']])
                self.assertEqual(len(row['seed_word']), row['s'])
                self.assertEqual(row['cycle'], row['anchors'])

    def reject(self, cert, reason):
        # A corruption must fail before even the first P or Q table is read.
        with patch.object(K, 'tree', side_effect=AssertionError('table checks reached')) as tree:
            with self.assertRaisesRegex(ValueError, reason):
                K.check(self.G, self.target, cert)
            tree.assert_not_called()

    def test_01_b15_catalogue_complete_bound_and_checker_cli(self):
        self.catalogue(self.G, self.target, self.cert, self.case)
        rc, output, elapsed = child([HERE / 'check_r22b10.py', 'table', self.table,
                                     RUNS / self.case[2], self.path], 'b15-check')
        self.evidence['check_seconds'] = elapsed
        self.assertEqual(rc, 0, output[-6000:])
        self.assertEqual(output.splitlines()[-1], 'PASS')

    def test_02_control_swapped_orbit_order(self):
        cert = copy.deepcopy(self.cert)
        cert['orbit_order'][:2] = cert['orbit_order'][1::-1]
        self.reject(cert, 'orbit order')

    def test_03_control_missing_orbit_order(self):
        cert = copy.deepcopy(self.cert)
        cert['orbit_order'].pop()
        self.reject(cert, 'orbit_order must contain every nontrivial target orbit exactly once')

    def test_04_control_changed_sigma(self):
        cert = copy.deepcopy(self.cert)
        cert['orbits'][0]['sigma'] = (cert['orbits'][0]['sigma'] + 1) % len(C.modules().CL.ctx.kplus.perms)
        self.reject(cert, 'atlas (seed_word|placement)')

    def test_05_control_non_safe_seed_word(self):
        cert = copy.deepcopy(self.cert)
        blocked = next(g for g in range(600) if g not in self.G.by_grip)
        cert['orbits'][0]['seed_word'][0] = [blocked, self.G.letters[0][1]]
        self.reject(cert, 'non-SAFE')

    def test_06_control_reversed_cycle(self):
        cert = copy.deepcopy(self.cert)
        cert['orbits'][0]['cycle'].reverse()
        self.reject(cert, 'atlas cycle')

    def test_07_control_legacy_format_with_atlas_rows(self):
        cert = copy.deepcopy(self.cert)
        cert['format'] = 'r22b10/1'
        self.reject(cert, 'orbit order|unknown seed route')

    def test_08_control_false_seed_node_expansion(self):
        cert = copy.deepcopy(self.cert)
        row = cert['orbits'][0]
        cert['nodes'].append(dict(op='inv', arg=row['seed'], length=row['s']))
        row['seed'] = len(cert['nodes']) - 1
        self.reject(cert, 'atlas seed node expansion differs')

    def test_09_control_missing_row_and_incomplete(self):
        cert = copy.deepcopy(self.cert)
        cert['orbits'].pop()
        self.reject(cert, 'target orbit list differs')
        cert = dict(self.cert, complete=False, missing=[dict(least=25, reason='control')])
        self.reject(cert, 'incomplete certificate')

    def test_10_control_steps_out_of_order(self):
        cert = copy.deepcopy(self.cert)
        cert['steps'].reverse()
        self.reject(cert, 'steps are not in orbit_order')

    def test_11_default_cli_preserves_legacy_keys_and_order(self):
        orbit, sizes = self.G.orbits()
        labels = sorted({int(orbit[x]) for x in self.target['T'] if sizes[orbit[x]] > 1},
                        key=lambda o: (-int(sizes[o]), int(np.flatnonzero(orbit == o)[0])))
        # Use the same real state with its two smallest target orbits. Giving
        # all nineteen orbits a tiny time share can leave no completed rows,
        # which cannot exercise the legacy ordering rule.
        labels = labels[-2:]
        T = [x for x in self.target['T'] if int(orbit[x]) in labels]
        target = dict(self.target, T=T, T_digest=hashlib.sha256(json.dumps(sorted(T)).encode()).hexdigest())
        target_path = WORK / 'legacy-target.json'
        target_path.write_text(json.dumps(target) + '\n')
        path = WORK / 'legacy-cert.json.gz'
        rc, output, _ = child([HERE / 'r22b10_cert.py', 'table', self.table,
                               target_path, path, '--seconds', 90], 'legacy-build', timeout=120)
        self.assertEqual(rc, 0, output[-6000:])
        with gzip.open(path, 'rt') as stream:
            cert = json.load(stream)
        with gzip.open(RUNS / 'run2/09-b15-v-cert.json.gz', 'rt') as stream:
            old = json.load(stream)
        self.assertEqual(cert['format'], 'r22b10/1')
        self.assertEqual(set(cert), set(old))
        order = [int(np.flatnonzero(orbit == o)[0]) for o in labels]
        written = [row['least'] for row in cert['orbits']]
        self.assertEqual(written, order)
        self.assertTrue(all(row['route'] in ('commutator', 'jordan') for row in cert['orbits']))

    def test_12_legacy_synthetic_certificates(self):
        rc, output, _ = child([HERE / 'test_r22b10.py'], 'legacy-tests', timeout=180)
        self.assertEqual(rc, 0, output[-6000:])

    def test_13_cache_override_default_and_empty_in_children(self):
        code = ('import sys; from pathlib import Path; '
                'sys.path.insert(0, str(Path("research/jumbling/theory").resolve())); '
                'import seams; print(str(seams.CACHE))')
        default = REPO / 'work/jumbling-cache/home_centroids.npy'
        for value, expected in ((str(WORK / 'child-cache.npy'), WORK / 'child-cache.npy'),
                                (None, default), ('', default)):
            with self.subTest(value=value):
                env = dict(ENV)
                if value is None:
                    env.pop('JUMBLING_HOME_CACHE', None)
                else:
                    env['JUMBLING_HOME_CACHE'] = value
                rc, output, _ = child(['-c', code], f'cache-{len(value or "")}', timeout=120, env=env)
                self.assertEqual(rc, 0, output[-6000:])
                self.assertEqual(output.splitlines()[-1], str(expected))

    @unittest.skipUnless(os.environ.get('LITERAL_SLOW') == '1', 'set LITERAL_SLOW=1 for all nine E2 states')
    def test_14_slow_all_nine_state_catalogues(self):
        for case in CASES:
            with self.subTest(state=case[0]):
                if case[0] == 'b15':
                    self.catalogue(self.G, self.target, self.cert, case)
                else:
                    G, target, cert, _, _, _ = build_case(case)
                    self.catalogue(G, target, cert, case)


class TimedResults(unittest.TextTestResult):
    def startTest(self, test):
        super().startTest(test)
        self.started = time.monotonic()

    def stopTest(self, test):
        if not hasattr(self, 'timings'):
            self.timings = []
        failed = any(t is test for t, _ in self.failures + self.errors)
        skipped = any(t is test for t, _ in self.skipped)
        self.timings.append((str(test), 'SKIP' if skipped else 'FAIL' if failed else 'PASS',
                             time.monotonic() - self.started))
        super().stopTest(test)


def main():
    before = subprocess.check_output(['git', 'status', '--porcelain', '--ignored'], cwd=REPO)
    start = time.monotonic()
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(AtlasCertificates)
    result = unittest.TextTestRunner(verbosity=2, resultclass=TimedResults).run(suite)
    elapsed = time.monotonic() - start
    after = subprocess.check_output(['git', 'status', '--porcelain', '--ignored'], cwd=REPO)
    unchanged = before == after
    report = ['# LIT-A implementation and acceptance report', '',
              '- r22b10_cert.py: opt-in deterministic atlas seeds and r22b10/2 orbit order; default remains r22b10/1.',
              '- check_r22b10.py: independent atlas reconstruction, exact seed leaves, oriented cycles and full prior domains before tables.',
              '- seams.py: JUMBLING_HOME_CACHE override, retaining the existing default.',
              '- test_atlas.py: b15, fast corruption controls, legacy and cache checks; optional all-nine catalogue tests.', '',
              'Command: `python3 -I -B research/jumbling/restoration/12-low-height/scripts/test_atlas.py`',
              f'Elapsed: {elapsed:.3f} seconds. Worktree status including ignored files unchanged: {unchanged}.', '',
              '| Test | Result | Seconds |', '| --- | --- | --- |']
    report.extend(f'| {name} | {outcome} | {seconds:.3f} |' for name, outcome, seconds in getattr(result, 'timings', []))
    for evidence in EVIDENCE:
        report.extend(['', f'## {evidence["state"]} certificate', '', '```json', json.dumps(evidence, indent=2), '```'])
    report.extend(['', 'Evidence: headless Linux research geometry; build/check seconds are acceptance timings, not performance claims.',
                   'Open: optional slow states unless enabled; LIT-B literal expansion/play and E2 replay belong to the integrator.',
                   'report.md is retained here in temporary storage because it is outside the four-file write contract.'])
    (WORK / 'report.md').write_text('\n'.join(report) + '\n')
    (WORK / 'evidence.json').write_text(json.dumps(EVIDENCE, indent=2) + '\n')
    print(f'Acceptance elapsed: {elapsed:.3f}s; worktree status including ignored files unchanged: {unchanged}', flush=True)
    print(f'Report: {WORK / "report.md"}', flush=True)
    return 0 if result.wasSuccessful() and unchanged and elapsed < 25 * 60 else 1


if __name__ == '__main__':
    sys.exit(main())
