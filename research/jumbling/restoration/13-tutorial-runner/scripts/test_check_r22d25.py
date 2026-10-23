"""Independent checker tests; also loaded by round22/test_r22d25.py.

Run from the repository root with python3 -I -B. Every mutation gets its own
temporary run directory. The only shared input is a captured exact start state.
"""
import os
import sys
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ.setdefault(_name, '1')
sys.dont_write_bytecode = True

import ast
import copy
import hashlib
import importlib.util
import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
CHECKER = HERE / 'check_r22d25.py'
sys.path.insert(0, str(HERE))
import check_r22d25 as C

fixture_factory = None


class FormatTests(unittest.TestCase):
    """Malformed records must fail before they can select extra input files."""

    def test_source_scope(self):
        empty = {'digest': hashlib.sha256(b'').hexdigest(), 'off': 0, 'heights': {}}
        doc = {'format': 'r22d25-run/1', 'catalogue': 'r22d25-cat/1', 'limits': dict(C.LIMITS),
               'start': dict(empty, kind='table', file='start-table.json'), 'last': dict(empty),
               'steps': [], 'result': 'lattice', 'seconds': 0.,
               'ledger': {f: {'status': 'not_applicable', 'tried': 0, 'pruned': {}, 'cursor': None}
                          for f in C.FAMILIES},
               'scripts': {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in C.SOURCES}}
        C.validate_format(doc)
        extra = copy.deepcopy(doc)
        extra['scripts']['AGENTS.md'] = hashlib.sha256((ROOT / 'AGENTS.md').read_bytes()).hexdigest()
        with self.assertRaises(C.CheckFailure):
            C.validate_format(extra)


def fixture():
    if fixture_factory is not None:
        return fixture_factory()
    # Standalone execution builds the same fixture once, using only the test
    # harness to arrange it. The checker itself has no such dependency.
    path = ROOT / 'research/jumbling/restoration/12-low-height/scripts/round22/test_r22d25.py'
    spec = importlib.util.spec_from_file_location('kernel_test_fixtures', path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.loop_fixture(unittest.TestCase())


class CheckerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.f = fixture()
        cls.temp = Path(tempfile.mkdtemp(prefix='r22d25-checker-tests-'))
        cls.original = cls.f['run']
        cls.doc = json.loads((cls.original / 'run.json').read_text())
        C.validate_format(cls.doc)
        cls.counter = 0

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.temp)
        if fixture_factory is None:
            shutil.rmtree(cls.f['temp'])

    def fresh(self):
        type(self).counter += 1
        dest = self.temp / str(self.counter)
        shutil.copytree(self.original, dest)
        # Mutation checks use an independently loaded current-only capture to
        # avoid replaying the same 372 census letters for every mutation. The
        # unmodified CLI check below separately exercises the record constructor.
        shutil.copyfile(self.f['table'], dest / 'start-table.json')
        doc = json.loads((dest / 'run.json').read_text())
        doc['start'].update(kind='table', file='start-table.json')
        C.validate_format(doc)
        (dest / 'run.json').write_text(json.dumps(doc))
        return dest, doc

    def check(self, directory):
        lines = []
        code = C.verify(directory, self.temp / 'cache', emit=lines.append)
        for line in lines[:-1]:
            self.assertIn('check', json.loads(line))
        self.assertTrue(lines[-1] == 'PASS' or lines[-1].startswith('FAIL '))
        return code, lines

    def test_a_cli_pass_record(self):
        C.validate_format(self.doc)
        child = subprocess.run([sys.executable, '-I', '-B', str(CHECKER), str(self.original), '--cache', str(self.temp / 'cache')],
                               cwd=ROOT, env=dict(os.environ, OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1',
                                                  MKL_NUM_THREADS='1', JUMBLING_HOME_CACHE=str(self.temp / 'cache/home_centroids.npy')),
                               capture_output=True, text=True, timeout=600)
        self.assertEqual(child.returncode, 0, child.stdout + child.stderr)
        self.assertEqual(child.stdout.splitlines()[-1], 'PASS')
        rows = [json.loads(line) for line in child.stdout.splitlines()[:-1]]
        self.assertTrue(any(row['check'] == 'step.0.literal_gate' for row in rows))

    def test_b_table_pass(self):
        directory, doc = self.fresh()
        C.validate_format(doc)
        code, lines = self.check(directory)
        self.assertEqual(code, 0, lines)
        self.assertEqual(lines[-1], 'PASS')

    def test_c_mutations(self):
        cases = ('letter', 'input_digest', 'output_digest', 'word_input_digest', 'word_output_digest',
                 'played_length', 'off_count', 'high_exact', 'target_unit', 'stuck_budget',
                 'absolute_path', 'windows_path')
        for case in cases:
            with self.subTest(case=case):
                directory, doc = self.fresh()
                wf = directory / doc['steps'][0]['word']
                word = json.loads(wf.read_text())
                row = doc['steps'][0]
                if case == 'letter':
                    word['word'][0][1] = 56
                    expected = ('step.0.word', 'step.0.output_digest', 'step.0.gate')
                elif case == 'input_digest':
                    row['input_digest'] = '0' * 64
                    expected = ('step.0.input_digest',)
                elif case == 'output_digest':
                    row['output_digest'] = '0' * 64
                    expected = ('step.0.output_digest',)
                elif case == 'word_input_digest':
                    word['input_digest'] = '0' * 64
                    expected = ('step.0.input_digest',)
                elif case == 'word_output_digest':
                    word['output_digest'] = '0' * 64
                    expected = ('step.0.output_digest',)
                elif case == 'played_length':
                    row['letters'] += 1
                    expected = ('step.0.played_length',)
                elif case == 'off_count':
                    row['off_before'] += 1
                    expected = ('step.0.off_before',)
                elif case == 'high_exact':
                    row['high_exact'] = True
                    expected = ('step.0.gate',)
                elif case == 'target_unit':
                    row['target'] = [391, 408]
                    expected = ('step.0.family.unit',)
                elif case == 'stuck_budget':
                    doc['result'] = 'stuck'
                    for value in doc['ledger'].values():
                        value.update(status='exhausted', cursor=None)
                    doc['ledger']['d19'].update(status='budget', cursor={'unit': 270})
                    expected = ('stuck.ledger',)
                elif case == 'absolute_path':
                    row['params']['untrusted_file'] = '/tmp/forbidden.json'
                    expected = ('paths.relative',)
                else:
                    row['params']['untrusted_file'] = 'C:\\private\\forbidden.json'
                    expected = ('paths.relative',)
                (directory / 'run.json').write_text(json.dumps(doc))
                wf.write_text(json.dumps(word))
                code, lines = self.check(directory)
                self.assertEqual(code, 1, lines)
                name = lines[-1].removeprefix('FAIL ')
                self.assertTrue(name.startswith(expected), (case, lines))

    def test_d_import_independence(self):
        C.validate_format(self.doc)
        tree = ast.parse(CHECKER.read_text())
        names = []
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                names.extend(alias.name for alias in node.names)
            elif isinstance(node, ast.ImportFrom):
                names.append(node.module or '')
                names.extend(alias.name for alias in node.names)
        self.assertFalse([n for n in names if any(p.startswith('r22d25_') or p in ('eb6r', 'stepchk') for p in n.split('.'))])

    def test_d_identity_is_played_and_counted(self):
        directory, doc = self.fresh()
        wf = directory / doc['steps'][0]['word']
        word = json.loads(wf.read_text())
        word['word'].insert(0, [61, 0])
        doc['steps'][0]['letters'] = 29
        wf.write_text(json.dumps(word))
        (directory / 'run.json').write_text(json.dumps(doc))
        C.validate_format(doc)
        code, lines = self.check(directory)
        self.assertEqual(code, 0, lines)
        self.assertEqual(lines[-1], 'PASS')

    def test_e_stuck_single_letter_is_rederived(self):
        directory, doc = self.fresh()
        M = C.restore.menu('I_a')
        solved = C.sim.State(C.sim.get_context(), menu=M)
        st = C.replay(solved, [[0, 12]], 'test.start')
        from sim.kplus import matrix_json
        table = {'poses': [matrix_json(st._pose_matrix(i)) for i in range(len(st._poses))],
                 'pose_id': st.pose_id.tolist(), 'digest': st.digest(), 'journal': []}
        (directory / 'start-table.json').write_text(json.dumps(table))
        info = C.summary(st)
        doc.update(start=dict(info, kind='table', file='start-table.json'), steps=[], last=info, result='stuck')
        for row in doc['ledger'].values():
            row.update(status='exhausted', tried=0, pruned={}, cursor=None)
        C.validate_format(doc)
        (directory / 'run.json').write_text(json.dumps(doc))
        code, lines = self.check(directory)
        self.assertEqual(code, 1, lines)
        self.assertEqual(lines[-1], 'FAIL stuck.T1')


if __name__ == '__main__':
    unittest.main(verbosity=2)
