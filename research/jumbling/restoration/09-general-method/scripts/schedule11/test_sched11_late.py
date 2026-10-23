"""SCH-L acceptance: exact headless fixtures, with all outputs in temporary storage.

Run from the repository root with python3 -I -B. No native or performance claim.
"""
import gzip
import hashlib
import json
import os
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import resource
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import unittest

sys.dont_write_bytecode = True
for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[name] = '1'
soft, hard = resource.getrlimit(resource.RLIMIT_AS)
limit = 7000000 * 1024
resource.setrlimit(resource.RLIMIT_AS, (limit if soft == resource.RLIM_INFINITY else min(soft, limit), hard))

import numpy as np

HERE = Path(__file__).resolve().parent
GM = HERE.parents[1]
J = GM.parents[1]
ROOT = J.parents[1]
S03 = GM.parent / '03-retained-group-and-lattice-restorer'
LATTICE = GM / 'scripts/lattice'
RESTORER = HERE / 'restore_schedule11_late.py'
REALIGN = GM / 'data/realign.json.gz'
CHECKER = LATTICE / 'checker/verify_late.py'
DEADLINE = time.monotonic() + 45 * 60
sys.path[:0] = [str(HERE), str(S03 / 'scripts')]
from rlib import Retained, NS
from kit import SLP
from sched11 import Schedule11


def read_json(path):
    return json.loads(path.read_text())


def tag(spec):
    return spec.replace(':', '_').replace('/', '_')[-60:]


def invoke(script, arguments, log):
    """Every child inherits the address-space limit and cannot write bytecode."""
    started = time.monotonic()
    with log.open('w') as output:
        child = subprocess.run([sys.executable, '-I', '-B', str(script), *map(str, arguments)],
                               cwd=ROOT, stdout=output, stderr=subprocess.STDOUT,
                               timeout=max(1, DEADLINE - started))
    return child.returncode, log.read_text(), time.monotonic() - started


# Git metadata and the private trees other processes write to while the test runs.
SKIP = {'.git', 'work/codex-sandbox-home', 'work/loop-memory', 'work/reviews', 'work/worktrees'}


def snapshot():
    """Include ignored files, empty directories and modes, without consulting Git."""
    result = {}
    for directory, dirs, files in os.walk(ROOT):
        here = Path(directory).relative_to(ROOT)
        dirs[:] = [d for d in dirs if (here / d).as_posix() not in SKIP]
        for name in dirs + files:
            path = Path(directory) / name
            try:
                mode = path.lstat().st_mode
                if stat.S_ISREG(mode):
                    value = hashlib.sha256(path.read_bytes()).hexdigest()
                elif stat.S_ISLNK(mode):
                    value = os.readlink(path)
                else:
                    value = None
            except FileNotFoundError:
                continue
            result[str(path.relative_to(ROOT))] = (mode, value)
    return result


class UsageTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix='sched11-late-usage-'))
        self.addCleanup(shutil.rmtree, self.tmp)

    def check_usage(self, realign, inputs):
        out = self.tmp / 'out'
        rc, text, _ = invoke(RESTORER, [ROOT, J, realign, self.tmp / 'library.json', out, *inputs],
                             self.tmp / 'usage.log')
        self.assertNotEqual(rc, 0, text)
        self.assertIn('usage:', text.lower())
        self.assertFalse(out.exists())
        self.assertEqual(sorted(p.name for p in self.tmp.iterdir()), ['usage.log'])

    def test_no_inputs_writes_nothing(self):
        self.check_usage(REALIGN, [])

    def test_missing_realign_writes_nothing(self):
        self.check_usage(self.tmp / 'missing.json.gz', ['word:11:3'])


class Schedule11LateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = Path(tempfile.mkdtemp(prefix='sched11-late-'))
        cls.addClassCleanup(shutil.rmtree, cls.tmp)
        print('  building shared schedule and simulator fixtures', flush=True)
        cls.library = cls.tmp / 'library.json'
        cls.library.write_bytes(gzip.decompress((GM / 'data/library-merged.json.gz').read_bytes()))
        # Derive the tariffs from the schedule itself, never from output records.
        R = Retained(ROOT)
        schedule = Schedule11(R, SLP(R), json.loads(gzip.decompress(REALIGN.read_bytes())))
        cls.bounds = schedule.orbit_bounds()
        cls.cascade = schedule.cascade
        del schedule, R

        sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures')]
        import restore as RS
        from sim.state import State
        from sim.twists import Twist
        from sim.kplus import matrix_json
        menu = RS.menu('I_a')
        cls.solved_digest = State(RS.ctx, menu=menu).digest()
        retained = sorted(set(range(len(menu.items))) - set(RS.jumble_items(menu)) - {0})
        item = next(i for i in retained if RS.to_tuple(RS.matmul(
            RS.matmul(menu.items[i][1], menu.items[i][1]), menu.items[i][1])) == RS.I4)
        square = RS.to_tuple(RS.matmul(menu.items[item][1], menu.items[item][1]))
        inverse = next(i for i in retained if RS.to_tuple(menu.items[i][1]) == square)
        second = next(int(c) for c in RS.NEAR[0] if int(c) != 0)
        third = min((set(map(int, RS.NEAR[0])) & set(map(int, RS.NEAR[second]))) - {0, second})
        comm = [(0, item), (second, item), (0, inverse), (second, inverse)]
        comm_inverse = [(second, item), (0, item), (second, inverse), (0, inverse)]
        word = comm + [(third, item)] + comm_inverse + [(third, inverse)]
        state = State(RS.ctx, menu=menu)
        for cap, index in word:
            outcome = state.apply(Twist(RS.ctx, cap, RS.element(menu, cap, index),
                                        'menu', {'menu': menu.name}))
            assert outcome.applied and not outcome.uncertain
        assert state.is_lattice() and state.digest() != cls.solved_digest
        assert 0 < np.count_nonzero(state.pose_id) < 1000

        def export(st, path):
            doc = {'poses': [matrix_json(st._pose_matrix(i)) for i in range(len(st._poses))],
                   'pose_id': st.pose_id.tolist(), 'digest': st.digest(), 'journal': []}
            path.write_text(json.dumps(doc))

        cls.table = cls.tmp / 'lattice-input.json'
        export(state, cls.table)
        cls.table_digest = state.digest()
        off = State(RS.ctx, menu=menu)
        index = RS.jumble_items(menu)[0]
        outcome = off.apply(Twist(RS.ctx, 0, RS.element(menu, 0, index), 'menu', {'menu': menu.name}))
        assert outcome.applied and not outcome.uncertain and not off.is_lattice()
        # One jumble twist moves the whole cap off the lattice, not a single piece.
        print('  off-lattice fixture: %d pieces off the lattice' % off.off_lattice_count(), flush=True)
        cls.off_table = cls.tmp / 'off-lattice.json'
        export(off, cls.off_table)

        cls.inputs = ['word:11:3', 'w9word:12:3', 'mix:13:3:2', 'table:' + str(cls.table)]
        cls.out = cls.tmp / 'valid'
        cls.template_out = cls.tmp / 'template'
        cls.off_out = cls.tmp / 'off-output'
        cls.off_spec = 'table:' + str(cls.off_table)
        cls.plain_realign = cls.tmp / 'realign.json'
        cls.plain_realign.write_bytes(gzip.decompress(REALIGN.read_bytes()))
        # Each invocation validates the whole library. They share only immutable
        # fixture inputs and own disjoint outputs; overlap these costly startups.
        cls.workers = ThreadPoolExecutor(max_workers=3)
        cls.addClassCleanup(cls.workers.shutdown, wait=True, cancel_futures=True)
        restored = cls.workers.submit(invoke, RESTORER,
            [ROOT, J, REALIGN, cls.library, cls.out, *cls.inputs], cls.tmp / 'restore.log')
        cls.template_result = cls.workers.submit(invoke, LATTICE / 'scripts/restore_lattice_late.py',
            [LATTICE / 'scripts', ROOT, J, cls.library, cls.template_out, 'w9word:12:3'],
            cls.tmp / 'template.log')
        cls.off_result = cls.workers.submit(invoke, RESTORER,
            [ROOT, J, cls.plain_realign, cls.library, cls.off_out, cls.off_spec], cls.tmp / 'off.log')
        print('  running schedule, template and off-lattice fixtures concurrently', flush=True)
        rc, cls.stdout, seconds = restored.result()
        if rc != 0:
            raise AssertionError(cls.stdout)
        print('  schedule restorer: %.1fs' % seconds, flush=True)
        cls.summary = read_json(cls.out / 'restore-summary.json')
        cls.runs = {r['input']: r for r in cls.summary['runs']}
        rc, text, seconds = invoke(CHECKER,
            [ROOT, J, cls.library, cls.out, cls.tmp / 'report.json'], cls.tmp / 'verify.log')
        if rc != 0 or not text.splitlines()[-1].startswith('PASS'):
            raise AssertionError(text)
        print('  unchanged checker: %s (%.1fs)' % (text.splitlines()[-1], seconds), flush=True)
        cls.report = read_json(cls.tmp / 'report.json')

    def test_01_restored_inputs_and_bounds(self):
        self.assertEqual(sum(self.bounds.values()), 20866844)
        self.assertEqual(self.summary['bound'], 20866844)
        self.assertEqual(self.summary['cascade_order'], self.cascade)
        self.assertEqual(self.summary['realign_sha256'], hashlib.sha256(REALIGN.read_bytes()).hexdigest())
        library = self.summary['library']
        self.assertEqual(library['sha256'], hashlib.sha256(self.library.read_bytes()).hexdigest())
        self.assertEqual(library['macros'], 36)
        self.assertEqual(library['rank2'], 34)
        self.assertEqual(library['rank5'], 2)
        self.assertTrue(self.summary['nu_w9'])
        self.assertEqual([r['input'] for r in self.summary['runs']], self.inputs)
        self.assertIs(self.report['pass'], True)
        self.assertEqual([r['input'] for r in self.report['runs']], self.inputs)
        for r in self.report['runs']:
            self.assertIs(r['restored_to_identity'], True)
            self.assertIs(r['length_matches_restorer'], True)
            self.assertIs(r['start_matches_restorer'], True)

        # Independently count the serialized words, then the per-orbit factor slices.
        doc = read_json(self.out / 'solutions.json')
        self.assertEqual(doc['format'], 'lattice-restoration-v1')
        self.assertEqual(set(doc['solutions']), {tag(s) for s in self.inputs})
        with gzip.open(ROOT / 'assets/execution_trees.json.gz', 'rt') as f:
            trees = json.load(f)
        lengths = []
        for node in doc['nodes']:
            if node['op'] == 'word':
                length = len(node['w'])
            elif node['op'] == 'path':
                length, n = 0, node['node']
                while trees[node['orbit']]['parent'][n] >= 0:
                    length += 1
                    n = trees[node['orbit']]['parent'][n]
            elif node['op'] == 'conj':
                length = lengths[node['x']] + 2 * lengths[node['u']]
            else:
                self.assertEqual(node['op'], 'prod')
                length = sum(lengths[i] for i, _ in node['f'])
            lengths.append(length)

        json_lines = [json.loads(line) for line in self.stdout.splitlines() if line.startswith('{')]
        self.assertEqual([r['input'] for r in json_lines], self.inputs)
        self.assertTrue(self.stdout.splitlines()[-1].startswith('done '))
        for rec in json_lines:
            self.assertFalse(any(key.endswith('_per_orbit') for key in rec))
        for spec in self.inputs:
            with self.subTest(input=spec.split(':')[0]):
                rec = self.runs[spec]
                self.assertEqual(rec['status'], 'restored')
                self.assertEqual(rec['bound'], 20866844)
                self.assertLessEqual(rec['retained_length'], 20866844)
                stored = np.load(self.out / ('start-labels-%s.npy' % tag(spec)), allow_pickle=False)
                self.assertEqual(stored.shape, (NS,))
                self.assertEqual(hashlib.sha256(stored.astype('<i8').tobytes()).hexdigest(),
                                 rec['input_labels_sha256'])
                for field in ('factors_per_orbit', 'length_per_orbit', 'method_per_orbit', 'counts_per_orbit'):
                    self.assertEqual(set(map(int, rec[field])), set(self.bounds))
                fs, offset = doc['solutions'][tag(spec)], 0
                for o in self.cascade:
                    count = rec['factors_per_orbit'][str(o)]
                    part = fs[offset:offset + count]
                    self.assertEqual(len(part), count)
                    self.assertEqual(sum(lengths[i] for i, _ in part), rec['length_per_orbit'][str(o)])
                    self.assertLessEqual(rec['length_per_orbit'][str(o)], self.bounds[o])
                    self.assertTrue(rec['method_per_orbit'][str(o)])
                    self.assertEqual(sum(rec['counts_per_orbit'][str(o)].values()), count)
                    offset += count
                self.assertEqual(offset, len(fs))
                self.assertEqual(len(fs), rec['factors'])
                self.assertEqual(sum(lengths[i] for i, _ in fs), rec['retained_length'])
                ratio, orbit = max((rec['length_per_orbit'][str(o)] / self.bounds[o], o) for o in self.bounds)
                label = 'table' if spec.startswith('table:') else spec
                print('  %-16s retained %d; largest length/bound %.6f (orbit %d)' %
                      (label, rec['retained_length'], ratio, orbit), flush=True)
        self.assertEqual(self.runs['word:11:3']['nu_input'], {})
        self.assertEqual(self.runs['word:11:3']['corrections'], [])
        self.assertEqual(self.runs[self.inputs[-1]]['input_digest'], self.table_digest)
        for spec in ('w9word:12:3', 'mix:13:3:2'):
            rec = self.runs[spec]
            self.assertTrue(rec['corrections'])
            self.assertTrue(rec['nu_input'])
            self.assertEqual(rec['order'], 'retained-first')
            self.assertRegex(rec['T_digest'], r'^[0-9a-f]{64}$')
            self.assertEqual(rec['final_digest'], self.solved_digest)

    def test_02_template_correction_choice(self):
        rc, text, seconds = self.template_result.result()
        self.assertEqual(rc, 0, text)
        old = read_json(self.template_out / 'restore-summary.json')['runs'][0]
        self.assertEqual(old['status'], 'restored')
        new = self.runs['w9word:12:3']
        fields = ('input_digest', 'input_labels_sha256', 'nu_input', 'corrections', 'T_digest')
        for field in fields:
            self.assertEqual(new[field], old[field], field)
        print('  template comparison: all five fields match; retained %d (%.1fs)' %
              (old['retained_length'], seconds), flush=True)

    def test_03_off_lattice_table_has_no_solution(self):
        # Also exercise the uncompressed realignment input.
        out = self.off_out
        rc, text, seconds = self.off_result.result()
        self.assertEqual(rc, 0, text)
        summary = read_json(out / 'restore-summary.json')
        self.assertEqual(summary['realign_sha256'], hashlib.sha256(self.plain_realign.read_bytes()).hexdigest())
        self.assertEqual(len(summary['runs']), 1)
        self.assertEqual(summary['runs'][0]['status'], 'not a lattice configuration')
        self.assertEqual(summary['runs'][0]['input'], self.off_spec)
        self.assertEqual(read_json(out / 'solutions.json')['solutions'], {})
        self.assertEqual(list(out.glob('start-labels-*.npy')), [])
        print('  off-lattice table: rejected with no solution (%.1fs)' % seconds, flush=True)

    def check_rejected(self, out, name):
        """The checker reports FAIL and exits 0; a crash is not a rejection."""
        report = self.tmp / (name + '-report.json')
        rc, text, _ = invoke(CHECKER, [ROOT, J, self.library, out, report], self.tmp / (name + '.log'))
        self.assertEqual(rc, 0, text)
        self.assertTrue(text.splitlines()[-1].startswith('FAIL'), text)
        print('  %s: rejected by unchanged checker' % name, flush=True)
        return {r['input']: r for r in read_json(report)['runs']}

    def test_04_checker_rejects_flipped_factor(self):
        out = self.tmp / 'flipped'
        shutil.copytree(self.out, out)
        path = out / 'solutions.json'
        doc = read_json(path)
        factors = doc['solutions'][tag('word:11:3')]
        self.assertTrue(factors)
        factors[0][1] *= -1
        path.write_text(json.dumps(doc))
        runs = self.check_rejected(out, 'flipped-factor')
        self.assertFalse(runs['word:11:3']['restored_to_identity'])
        for spec in self.inputs[1:]:
            self.assertTrue(runs[spec]['restored_to_identity'], spec)

    def test_05_checker_rejects_wrong_retained_length(self):
        out = self.tmp / 'wrong-length'
        shutil.copytree(self.out, out)
        path = out / 'restore-summary.json'
        doc = read_json(path)
        doc['runs'][0]['retained_length'] += 1
        path.write_text(json.dumps(doc))
        runs = self.check_rejected(out, 'wrong-length')
        first = runs[doc['runs'][0]['input']]
        self.assertFalse(first['length_matches_restorer'])
        self.assertTrue(first['restored_to_identity'])


class WorktreeTests(unittest.TestCase):
    def test_no_generated_worktree_files(self):
        after = snapshot()
        changed = sorted(p for p in set(BASELINE) | set(after) if BASELINE.get(p) != after.get(p))
        self.assertEqual(changed, [], 'worktree changed, including ignored files')


if __name__ == '__main__':
    BASELINE = snapshot()
    unittest.main(verbosity=2)
