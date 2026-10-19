"""Single-process, temporary-directory acceptance tests for the RUN-2 checker.

Run from the repository root with python3 -I -B. Fixtures are built independently
of runner.py, which need not exist. No output, cache or table belongs in the repo.
"""

from collections import Counter
import copy
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
for variable in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS', 'NUMEXPR_NUM_THREADS'):
    os.environ[variable] = '1'
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
J = REPO / 'research/jumbling'
sys.path[:0] = [str(HERE), str(J / 'theory'), str(J), str(J / 'fixtures')]

import check_run as C  # noqa: E402
import numpy as np  # noqa: E402
import restore  # noqa: E402
import rim  # noqa: E402
import sim  # noqa: E402
import trees  # noqa: E402


def read(path):
    data = Path(path).read_bytes()
    return json.loads(gzip.decompress(data) if data.startswith(b'\x1f\x8b') else data)


def write(path, doc):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(doc) + '\n')


def status():
    return subprocess.check_output(['git', 'status', '--porcelain', '--ignored', '--untracked-files=all'], cwd=REPO)


def off(st):
    return np.array([p.kidx < 0 for p in st._poses], bool)[st.pose_id]


def state_record(st, with_n=False):
    # Fixture measurements are computed here, not with the checker helpers.
    histogram = Counter()
    for pid, count in zip(*np.unique(st.pose_id, return_counts=True)):
        if st._poses[int(pid)].kidx < 0:
            height = trees.dist(trees.factors(st._pose_matrix(int(pid)))[0])
            histogram[str(height)] += int(count)
    doc = {'digest': st.digest(), 'off': int(st.off_lattice_count()), 'heights': dict(histogram)}
    if with_n:
        doc['N'] = int(rim.Domains(st).total_misalignment())
    return doc


def apply(st, menu, word):
    for c, i in word:
        outcome = st.apply(sim.Twist(restore.ctx, c, restore.element(menu, c, i), 'menu', {'menu': menu.name}),
                           journal=False)
        if not outcome.applied or outcome.uncertain:
            raise AssertionError(('fixture word not admissible', c, i))


def blank_record(start):
    return {'format': 'runner-run/1', 'menu': 'I_a', 'start': start,
            'config': {'literal_only': True, 'scripts': {}},
            'steps': [], 'phases': {}, 'result': 'budget', 'stuck': None, 'lattice': None,
            'totals': {'bulk_twists': 0, 'tail_twists': 0, 'fallback_twists': 0,
                       'lattice_twists': None, 'seconds': 0}}


class CheckRunTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.worktree_before = status()
        cls.source_before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in
                             (HERE / 'check_run.py', HERE / 'test_check_run.py')}
        cls.tmp = Path(tempfile.mkdtemp(prefix='tutorial-check-tests-'))
        cls.addClassCleanup(shutil.rmtree, cls.tmp)
        cls.menu = restore.menu('I_a')
        # The explicitly permitted exporter/loader is used only by fixture
        # construction. The checker itself never imports this dependency tree.
        import buffer
        cls.export = staticmethod(buffer.export_pose_table)
        cls.set_table = staticmethod(buffer.O.set_pose_table)
        for label, build in (('bulk', cls.build_bulk), ('tail', cls.build_tail), ('lattice', cls.build_lattice)):
            t0 = time.monotonic()
            build()
            print('fixture %s: %.2fs' % (label, time.monotonic() - t0), flush=True)

    @classmethod
    def tearDownClass(cls):
        if status() != cls.worktree_before:
            raise AssertionError('test run changed the worktree, including ignored files')
        for name, sha in cls.source_before.items():
            if hashlib.sha256((HERE / name).read_bytes()).hexdigest() != sha:
                raise AssertionError('test run changed its source')

    @classmethod
    def build_bulk(cls):
        base = cls.tmp / 'bulk'
        source = read(J / 'restoration/11-scramble-census/runs/round1/ia-k10-s7001.json')
        st, history = restore.scramble(cls.menu, 10, 7001)
        assert [list(h) for h in history] == source['history']
        rec = blank_record({'kind': 'scramble', 'k': 10, 'seed': 7001, **state_record(st)})
        pos = 0
        for index, recorded in enumerate(source['steps']):
            word = source['moves'][pos:pos + recorded['twists']]
            before = state_record(st, with_n=True)
            apply(st, cls.menu, word)
            after = state_record(st, with_n=True)
            assert (before['N'], after['N'], after['off']) == (
                recorded['N_before'], recorded['N_after'], recorded['off_after'])
            name = 'words/%05d.json' % index
            write(base / name, {'word': word, 'input_digest': before['digest'], 'output_digest': after['digest']})
            rec['steps'].append({'index': index, 'phase': 'bulk', 'rule': recorded['rule'], 'word': name,
                                 'letters': len(word), 'before': before, 'after': after})
            pos += len(word)
        assert st.digest() == C.SOLVED and pos == 9
        (base / 'tables').mkdir()
        cls.export(st, base / 'tables/bulk-end.json')
        end = {**state_record(st), 'table': 'tables/bulk-end.json'}
        rec['phases'] = {'bulk': {'result': 'lattice', 'steps': len(rec['steps']), 'twists': pos, 'end': end}}
        rec['result'], rec['totals']['bulk_twists'] = 'solved', pos
        write(base / 'run.json', rec)
        cls.bulk = base

    @classmethod
    def build_tail(cls):
        base = cls.tmp / 'tail'
        source = J / 'restoration/12-low-height/runs/round22/line-a'
        table = read(source / 'replay2/run1/z-b21.json.gz')
        write(base / 'start-table.json', table)
        write(base / 'tables/bulk-end.json', table)
        unit = read(source / 'units/eb5-k40-k00-b04.json')
        write(base / 'units/b04.json', unit)
        W = read(source / 'replay2/inputs/b04-cp4-W.json')['word']
        closure = read(source / 'replay2/run2/00-b04-c2/word-full.json')
        st = sim.State(restore.ctx, menu=cls.menu)
        cls.set_table(st, table)
        before, off_before = state_record(st), off(st)
        assert before['off'] == 437 and before['digest'].startswith('a3fef11c')
        apply(st, cls.menu, W)
        cls.w_after = state_record(st)
        assert cls.w_after['digest'] == closure['input_digest']
        cls.w_landed = int((off_before[unit['pieces']] & ~off(st)[unit['pieces']]).sum())
        cls.w_others = int((off_before & ~off(st)).sum()) - cls.w_landed
        assert int((~off_before & off(st)).sum()) == 86
        # The packet gives this exact endpoint. The positive checker test below
        # independently replays all 1,254 letters; fixture construction need not
        # play the closure a second time to manufacture its expected result.
        after = {'digest': closure['output_digest'], 'off': 392, 'heights': {'2': 392}}
        assert after['digest'] == 'fee07e63a8cf8672ae81cbdf3c1d13b9ed83e61de758a7f2e94b13d28dfe7167'
        assert len(W) == 288 and len(closure['word']) == 966
        word = W + closure['word']
        write(base / 'words/00000.json', {'word': word, 'input_digest': before['digest'],
                                         'output_digest': after['digest']})
        rec = blank_record({'kind': 'table', 'file': 'start-table.json', **before})
        rec['phases'] = {'bulk': {'result': 'stuck', 'steps': 0, 'twists': 0,
                                  'end': {**before, 'table': 'tables/bulk-end.json'}},
                         'tail': {'result': 'budget', 'steps': 1}}
        rec['steps'] = [{'index': 0, 'phase': 'tail', 'rule': 't1prime', 'unit': 'units/b04.json',
                         'unit_key': 2198, 'unit_size': 45, 'word': 'words/00000.json', 'letters': 1254,
                         'parts': [['W', 288], ['closure', 966]], 'contract': 'strict_prime',
                         'before': before, 'after': after, 'landed': 45, 'others_landed': 0}]
        rec['totals']['tail_twists'] = 1254
        write(base / 'run.json', rec)
        cls.tail, cls.W = base, W

    @classmethod
    def build_lattice(cls):
        base = cls.tmp / 'lattice'
        base.mkdir()
        retained = [i for i in range(len(cls.menu.items)) if i not in restore.jumble_items(cls.menu)
                    and restore.element(cls.menu, 0, i) != restore.I4]
        st = sim.State(restore.ctx, menu=cls.menu)
        apply(st, cls.menu, [[0, retained[0]], [17, retained[1]], [379, retained[0]]])
        before = state_record(st)
        assert before['off'] == 0 and before['digest'] != C.SOLVED
        cls.export(st, base / 'start-table.json')
        (base / 'tables').mkdir()
        shutil.copyfile(base / 'start-table.json', base / 'tables/bulk-end.json')
        out = base / 'lattice'
        out.mkdir()
        library = out / 'library-merged.json'
        library.write_bytes(gzip.decompress((J / 'restoration/09-general-method/data/library-merged.json.gz').read_bytes()))
        scripts = J / 'restoration/09-general-method/scripts/lattice/scripts'
        argv = [sys.executable, '-I', '-B', str(scripts / 'restore_lattice_late.py'), str(scripts), str(REPO),
                str(J), str(library), str(out), 'table:' + str(base / 'start-table.json')]
        with (cls.tmp / 'restore-lattice.log').open('w') as log:
            proc = subprocess.run(argv, cwd=REPO, stdout=log, stderr=subprocess.STDOUT, timeout=3600, check=False)
        if proc.returncode:
            raise AssertionError('lattice fixture restorer failed: ' + (cls.tmp / 'restore-lattice.log').read_text()[-3000:])
        summary = read(out / 'restore-summary.json')
        run, = summary['runs']
        assert run['status'] == 'restored' and run['input_digest'] == before['digest']
        twists = run['retained_length'] + run.get('correction_twists', 0)
        rec = blank_record({'kind': 'table', 'file': 'start-table.json', **before})
        rec['phases'] = {'bulk': {'result': 'lattice', 'steps': 0, 'twists': 0,
                                  'end': {**before, 'table': 'tables/bulk-end.json'}}}
        rec['lattice'] = {'input_digest': before['digest'], 'library': 'lattice/library-merged.json',
                          'library_sha256': hashlib.sha256(library.read_bytes()).hexdigest(), 'out': 'lattice/',
                          'restore_rc': 0, 'verify_rc': 0, 'verify_last_line': 'PASS 0', 'chain': False,
                          'twists': twists, 'seconds': 0}
        rec['result'], rec['totals']['lattice_twists'] = 'solved', twists
        write(base / 'run.json', rec)
        cls.lattice = base

    def case(self, fixture='bulk'):
        base = self.tmp / self._testMethodName
        shutil.copytree(getattr(self, fixture), base)
        return base, read(base / 'run.json')

    def check(self, base, rec, failure=None, report_parent=None):
        write(base / 'run.json', rec)
        rows = []
        final = C.check_run(base, rows.append, report_parent=report_parent)
        self.assertEqual(len(rows), len(rec['steps']) + 1, rows)
        self.assertIs(final['pass'], failure is None, rows)
        if failure is not None:
            self.assertIn(failure, {f for row in rows for f in row['fails']}, rows)
        return final

    def test_bulk_cli(self):
        base, _ = self.case()
        output = self.tmp / 'bulk-check.jsonl'
        proc = subprocess.run([sys.executable, '-I', '-B', str(HERE / 'check_run.py'), str(base), '--out', str(output)],
                              cwd=REPO, capture_output=True, text=True, timeout=180, check=False)
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertEqual(proc.stdout, output.read_text())
        rows = [json.loads(line) for line in proc.stdout.splitlines()]
        self.assertEqual(len(rows), 10)
        self.assertTrue(all(row['pass'] for row in rows), rows)
        self.assertEqual(rows[-1]['end_digest'], C.SOLVED)
        self.assertEqual(rows[-1]['bulk_twists'], 9)
        self.assertFalse((base / 'check').exists())

    def test_changed_letter(self):
        base, rec = self.case()
        path = base / rec['steps'][0]['word']
        doc = read(path)
        doc['word'][0][1] = (doc['word'][0][1] + 1) % 60
        write(path, doc)
        self.check(base, rec, 'output_digest')

    def test_word_segments(self):
        base, rec = self.case()
        path = base / rec['steps'][0]['word']
        doc = read(path)
        doc['segments'] = []
        write(path, doc)
        self.check(base, rec, 'forbidden_key')

    def test_step_slp(self):
        base, rec = self.case()
        rec['steps'][0]['slp'] = {}
        self.check(base, rec, 'forbidden_key')

    def test_start_digest(self):
        base, rec = self.case()
        rec['start']['digest'] = '0' * 64
        self.check(base, rec, 'start_digest')

    def test_absolute_path(self):
        base, rec = self.case()
        rec['steps'][0]['word'] = str(base / rec['steps'][0]['word'])
        self.check(base, rec, 'path')

    def test_parent_path(self):
        base, rec = self.case()
        rec['steps'][0]['word'] = 'words/../words/00000.json'
        self.check(base, rec, 'path')

    def test_cross_platform_paths(self):
        base, rec = self.case()
        for name in ('words\\00000.json', 'C:/words/00000.json'):
            with self.subTest(path=name):
                rec['steps'][0]['word'] = name
                self.check(base, rec, 'path')

    def test_word_shape(self):
        base, rec = self.case()
        path = base / rec['steps'][0]['word']
        original = read(path)
        for word in ([[600, 0]], [[0, 60]], [[True, 1]], [[0, 1.0]], [0, 1]):
            with self.subTest(word=word):
                doc = {**original, 'word': word}
                write(path, doc)
                self.check(base, rec, 'word_literal')
        write(path, {**original, 'input_digest': 'z' * 64})
        self.check(base, rec, 'word_digest')

    def test_record_shape(self):
        base, rec = self.case()
        for malformed in ([], None, 42, {**rec, 'totals': []}):
            with self.subTest(kind=type(malformed).__name__):
                write(base / 'run.json', malformed)
                rows = []
                final = C.check_run(base, rows.append)
                self.assertFalse(final['pass'])
                self.assertIn('record', final['fails'])
                self.assertIs(rows[-1], final)

    def test_missing_last_step(self):
        base, rec = self.case()
        rec['steps'].pop()
        self.check(base, rec, 'solved_digest')

    def test_n_must_fall(self):
        base, rec = self.case()
        rec['steps'][0]['after']['N'] = rec['steps'][0]['before']['N']
        self.check(base, rec, 'N_decrease')

    def test_index_gap(self):
        base, rec = self.case()
        rec['steps'][1]['index'] += 1
        self.check(base, rec, 'index')

    def test_letters(self):
        base, rec = self.case()
        rec['steps'][0]['letters'] += 1
        self.check(base, rec, 'letters')

    def test_heights(self):
        base, rec = self.case()
        rec['start']['heights']['2'] += 1
        self.check(base, rec, 'start_heights')

    def test_totals(self):
        base, rec = self.case()
        rec['totals']['bulk_twists'] += 1
        self.check(base, rec, 'bulk_twists')

    def test_error_and_stuck(self):
        base, rec = self.case()
        rec['result'] = 'error'
        self.check(base, rec)
        rec['result'] = 'stuck'
        rec['stuck'] = copy.deepcopy(rec['steps'][-1]['after'])
        rec['stuck']['units'] = []
        self.check(base, rec)
        rec['stuck']['digest'] = '0' * 64
        self.check(base, rec, 'stuck_digest')

    def test_bulk_table(self):
        base, rec = self.case()
        table = read(base / 'tables/bulk-end.json')
        table['digest'] = '0' * 64
        write(base / 'tables/bulk-end.json', table)
        self.check(base, rec, 'bulk_table')

    def test_tail_strict_prime(self):
        base, rec = self.case('tail')
        self.check(base, rec)

    def test_tail_w_has_collateral(self):
        base, rec = self.case('tail')
        step = rec['steps'][0]
        step.update(after=self.w_after, letters=288, parts=[['W', 288]],
                    landed=self.w_landed, others_landed=self.w_others)
        write(base / step['word'], {'word': self.W, 'input_digest': step['before']['digest'],
                                    'output_digest': self.w_after['digest']})
        rec['totals']['tail_twists'] = 288
        self.check(base, rec, 'collateral')

    def test_tail_landed_count(self):
        base, rec = self.case('tail')
        rec['steps'][0]['landed'] = 44
        self.check(base, rec, 'landed')

    def test_lattice(self):
        base, rec = self.case('lattice')
        report_parent = self.tmp / 'lattice-check-output'
        report_parent.mkdir()
        final = self.check(base, rec, report_parent=report_parent)
        self.assertTrue(final['lattice_checked'])
        self.assertEqual(final['end_digest'], C.SOLVED)
        self.assertEqual(len(list((report_parent / 'check').glob('*/report.json'))), 1)
        self.assertFalse((base / 'check').exists())

    def test_lattice_interrupted_error(self):
        base, rec = self.case('lattice')
        rec['result'] = 'error'
        rec['lattice'].update(twists=None, restore_rc=1, verify_rc=None, verify_last_line=None)
        rec['totals']['lattice_twists'] = None
        (base / 'lattice/restore-summary.json').unlink()
        self.assertFalse(self.check(base, rec)['lattice_checked'])

    def test_lattice_input_digest(self):
        base, rec = self.case('lattice')
        rec['lattice']['input_digest'] = '0' * 64
        self.check(base, rec, 'lattice_input_digest')

    def test_lattice_library_digest(self):
        base, rec = self.case('lattice')
        path = base / rec['lattice']['library']
        path.write_bytes(path.read_bytes() + b'\n')
        self.check(base, rec, 'library_sha256')

    def test_lattice_vacuous_report(self):
        base, rec = self.case('lattice')

        def fake_verifier(argv, **kwargs):
            write(Path(argv[8]), {'pass': True, 'runs': []})
            return SimpleNamespace(returncode=0, stdout='PASS 0\n')

        with patch.object(C.subprocess, 'run', side_effect=fake_verifier):
            self.check(base, rec, 'lattice_checked_run')

    def test_lattice_fallback_chain(self):
        base, rec = self.case('lattice')
        before = {k: rec['start'][k] for k in ('digest', 'off', 'heights')}
        # A fallback unit may already be on the lattice. An empty literal word
        # exercises --chain's required word argument without a search fixture.
        write(base / 'units/one.json', {'pieces': [0]})
        write(base / 'words/00000.json', {'word': [], 'input_digest': before['digest'],
                                         'output_digest': before['digest']})
        rec['steps'] = [{'index': 0, 'phase': 'fallback', 'rule': 'fallback', 'unit': 'units/one.json',
                         'unit_size': 1, 'word': 'words/00000.json', 'letters': 0, 'contract': 'fallback',
                         'before': copy.deepcopy(before), 'after': copy.deepcopy(before),
                         'landed': 0, 'others_landed': 0}]
        rec['phases']['tail'] = {'result': 'lattice', 'steps': 1}
        rec['lattice']['chain'] = True
        self.assertTrue(self.check(base, rec)['lattice_checked'])


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        super().stopTest(test)
        self.stream.writeln('  %.2fs' % (time.monotonic() - self.started))


if __name__ == '__main__':
    unittest.main(verbosity=2, testRunner=unittest.TextTestRunner(verbosity=2, resultclass=TimedResult))
