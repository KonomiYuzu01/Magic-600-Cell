"""Single-process, temporary-directory acceptance tests for both run formats.

Run from the repository root with python3 -I -B. Fixtures are built independently
of runner.py, which need not exist. No output, cache or table belongs in the repo.
"""

from collections import Counter
import copy
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
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


# Private trees that hooks and other calls write to while the suite runs.
SKIP = ('work/codex-sandbox-home', 'work/loop-memory', 'work/reviews', 'work/worktrees')


def status():
    return subprocess.check_output(['git', '--no-optional-locks', 'status', '--porcelain', '--ignored',
        '--untracked-files=all', '--', '.'] + [':(exclude)' + p for p in SKIP], cwd=REPO)


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


def version2(rec):
    rec = copy.deepcopy(rec)
    rec['format'], rec['error'] = 'runner-run/2', None
    rec['config'].update(literal_limit=10000000, seeds='atlas', budgets={'cert_seconds': 3600},
                         source={'commit': None, 'status_sha256': None, 'dirty': None})
    return rec


def word_sha(word):
    return hashlib.sha256(json.dumps(word).encode('ascii')).hexdigest()


class CheckRunTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = Path(tempfile.mkdtemp(prefix='tutorial-check-tests-'))
        cls.addClassCleanup(shutil.rmtree, cls.tmp)
        cache_env = patch.dict(os.environ, {'JUMBLING_HOME_CACHE': str(cls.tmp / 'home-centroids.npy')})
        cache_env.start()
        cls.addClassCleanup(cache_env.stop)
        cls.worktree_before = status()
        cls.source_before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in
                             (HERE / 'check_run.py', HERE / 'test_check_run.py')}
        cls.menu = restore.menu('I_a')
        # The explicitly permitted exporter/loader is used only by fixture
        # construction. The checker itself never imports this dependency tree.
        import buffer
        cls.export = staticmethod(buffer.export_pose_table)
        cls.set_table = staticmethod(buffer.O.set_pose_table)
        for label, build in (('bulk', cls.build_bulk), ('tail', cls.build_tail),
                             ('short', cls.build_short)):
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
    def build_short(cls):
        # Two antipodal grips do not interact. Landing the first while twisting
        # the second again changes another off piece's pose, without collateral.
        item = next(i for i in restore.jumble_items(cls.menu)
                    if restore.to_tuple(restore.matmul(cls.menu.items[i][1], cls.menu.items[i][1]))
                    not in {restore.to_tuple(restore.ctx.kplus.matrix(k)) for k in restore.ctx.kplus.stab0})
        inverse = next(i for i, (_, m) in enumerate(cls.menu.items)
                       if restore.to_tuple(m) == restore.to_tuple(restore.transpose(cls.menu.items[item][1])))
        opposite = int(np.argmin(restore._NF @ restore._NF[0]))
        for label, move_other in (('short', False), ('pose_change', True)):
            base = cls.tmp / label
            st = sim.State(restore.ctx, menu=cls.menu)
            apply(st, cls.menu, [[0, item]])
            unit_piece = int(np.flatnonzero(off(st))[0])
            if move_other:
                apply(st, cls.menu, [[opposite, item]])
            else:
                blocked = next(c for c in restore.NEAR[0] if st.classify(int(c)).status == 'blocked')
                cls.blocked_letter = [int(blocked), item]
            before, off_before = state_record(st), off(st)
            base.mkdir()
            cls.export(st, base / 'start-table.json')
            (base / 'tables').mkdir()
            cls.export(st, base / 'tables/bulk-end.json')
            word = [[0, inverse]] + ([[opposite, item]] if move_other else [])
            apply(st, cls.menu, word)
            after = state_record(st)
            assert not off(st)[unit_piece]
            assert not np.any(~off_before & off(st))
            assert (after['off'] > 0) == move_other
            write(base / 'units/one.json', {'pieces': [unit_piece]})
            write(base / 'words/00000.json', {'word': word, 'input_digest': before['digest'],
                                             'output_digest': after['digest']})
            rec = blank_record({'kind': 'table', 'file': 'start-table.json', **before})
            rec['phases'] = {'bulk': {'result': 'stuck', 'steps': 0, 'twists': 0,
                                      'end': {**before, 'table': 'tables/bulk-end.json'}},
                             'tail': {'result': 'budget', 'steps': 1}}
            rec['steps'] = [{'index': 0, 'phase': 'tail', 'rule': 'pure', 'unit': 'units/one.json',
                             'unit_size': 1, 'word': 'words/00000.json', 'letters': len(word),
                             'contract': 'strict_prime', 'before': before, 'after': after,
                             'landed': 1, 'others_landed': int((off_before & ~off(st)).sum()) - 1}]
            rec['totals']['tail_twists'] = len(word)
            write(base / 'run.json', rec)
            setattr(cls, label, base)

    @classmethod
    def build_lattice(cls):
        base = cls.tmp / 'lattice_v2'
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
        shutil.copyfile(base / 'start-table.json', base / 'tables/lattice-input.json')
        out = base / 'lattice'
        out.mkdir()
        library = out / 'library-merged.json'
        full_library = read(J / 'restoration/09-general-method/data/library-merged.json.gz')
        # This retained-only input has nu = 0 and uses no corrections. Keep the
        # real mandatory w9 entry, rather than spending the acceptance budget
        # replaying all 36 unused library macros during restorer initialization.
        assert full_library['macros'][0]['family'] == 'w9'
        write(library, {k: full_library[k] for k in ('format', 'menu')} |
              {'macros': full_library['macros'][:1]})
        scripts = J / 'restoration/09-general-method/scripts/lattice/scripts'
        spec = 'table:tables/lattice-input.json'
        argv = [sys.executable, '-I', '-B', str(scripts / 'restore_lattice_late.py'), str(scripts), str(REPO),
                str(J), library.relative_to(base).as_posix(), out.relative_to(base).as_posix(), spec]
        with (cls.tmp / 'restore-lattice.log').open('w') as log:
            proc = subprocess.run(argv, cwd=base, stdout=log, stderr=subprocess.STDOUT, timeout=3600, check=False)
        if proc.returncode:
            raise AssertionError('lattice fixture restorer failed: ' + (cls.tmp / 'restore-lattice.log').read_text()[-3000:])
        summary = read(out / 'restore-summary.json')
        run, = summary['runs']
        assert run['status'] == 'restored' and run['input_digest'] == before['digest']
        assert run['corrections'] == []
        twists = run['retained_length'] + run.get('correction_twists', 0)
        rec = version2(blank_record({'kind': 'table', 'file': 'start-table.json', **before}))
        rec['phases'] = {'bulk': {'result': 'lattice', 'steps': 0, 'twists': 0,
                                  'end': {**before, 'table': 'tables/bulk-end.json'}}}
        rec['lattice'] = {'input_digest': before['digest'], 'library': 'lattice/library-merged.json',
                          'library_sha256': hashlib.sha256(library.read_bytes()).hexdigest(), 'out': 'lattice/',
                          'restore_rc': 0, 'verify_rc': 0, 'verify_last_line': 'PASS 0', 'chain': False,
                          'input': spec, 'twists': twists, 'seconds': 0}
        rec['result'], rec['totals']['lattice_twists'] = 'solved', twists
        write(base / 'run.json', rec)
        cls.lattice_v2 = base
        # Retain the legacy fixture and its absolute summary spec without a
        # second expensive restorer run. Only spec-derived lookup names change.
        legacy = cls.tmp / 'lattice'
        shutil.copytree(base, legacy)
        legacy_spec = 'table:' + str(legacy / 'tables/lattice-input.json')
        old_tag = spec.replace(':', '_').replace('/', '_')[-60:]
        new_tag = legacy_spec.replace(':', '_').replace('/', '_')[-60:]
        summary['runs'][0]['input'] = legacy_spec
        write(legacy / 'lattice/restore-summary.json', summary)
        solutions = read(legacy / 'lattice/solutions.json')
        solutions['solutions'][new_tag] = solutions['solutions'].pop(old_tag)
        write(legacy / 'lattice/solutions.json', solutions)
        (legacy / ('lattice/start-labels-%s.npy' % old_tag)).rename(
            legacy / ('lattice/start-labels-%s.npy' % new_tag))
        rec['format'], rec['config'] = 'runner-run/1', {'literal_only': True, 'scripts': {}}
        del rec['error'], rec['lattice']['input']
        write(legacy / 'run.json', rec)
        cls.lattice = legacy

    def case(self, fixture='bulk', label=None):
        if fixture in ('lattice', 'lattice_v2') and not hasattr(type(self), 'lattice'):
            started = time.monotonic()
            self.build_lattice()
            print('fixture lattice: %.2fs' % (time.monotonic() - started), flush=True)
        base = self.tmp / (self._testMethodName + ('-' + label if label else ''))
        shutil.copytree(getattr(self, fixture), base)
        return base, read(base / 'run.json')

    def v2_case(self, fixture='bulk', label=None):
        base, rec = self.case(fixture, label)
        rec = version2(rec)
        if fixture == 'tail':
            word = read(base / rec['steps'][0]['word'])['word']
            rec['steps'][0].update(gate='stepchk', V_source='tclose2', V_letters=339,
                                   V_sha256=word_sha(word[288:627]),
                                   parts=[['W', 288], ['V', 339], ['middle', 288], ['V_inverse', 339]])
        for step in rec['steps']:
            if step['phase'] in ('tail', 'fallback'):
                step.setdefault('gate', 'stepchk')
        return base, rec

    def check(self, base, rec, failure=None, report_parent=None, **kwargs):
        write(base / 'run.json', rec)
        rows = []
        final = C.check_run(base, rows.append, report_parent=report_parent, **kwargs)
        self.assertEqual(len(rows), len(rec['steps']) + 1, rows)
        self.assertIs(final['pass'], failure is None, rows)
        if failure is not None:
            self.assertIn(failure, {f for row in rows for f in row['fails']}, rows)
            self.assertIn(failure, final['fails'])
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
        verifier = base / 'vacuous.py'
        verifier.write_text("import json, pathlib, sys\n"
                            "pathlib.Path(sys.argv[5]).write_text(json.dumps({'pass': True, 'runs': []}))\n"
                            "print('PASS 0')\n")
        with patch.object(C, 'VERIFY_LATE', verifier):
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

    def fallback_step(self, base, rec):
        before = {k: rec['start'][k] for k in ('digest', 'off', 'heights')}
        write(base / 'units/one.json', {'pieces': [0]})
        write(base / 'words/00000.json', {'word': [], 'input_digest': before['digest'],
                                         'output_digest': before['digest']})
        rec['steps'] = [{'index': 0, 'phase': 'fallback', 'rule': 'fallback', 'unit': 'units/one.json',
                         'unit_size': 1, 'word': 'words/00000.json', 'letters': 0, 'contract': 'fallback',
                         'before': copy.deepcopy(before), 'after': copy.deepcopy(before),
                         'landed': 0, 'others_landed': 0}]
        if rec['format'] == 'runner-run/2':
            rec['steps'][0]['gate'] = 'stepchk'
        rec['phases']['tail'] = {'result': 'lattice', 'steps': 1}
        rec['lattice']['chain'] = rec['format'] == 'runner-run/1'

    def test_v2_bulk(self):
        base, rec = self.v2_case()
        self.assertEqual(self.check(base, rec)['end_digest'], C.SOLVED)

    def test_v2_tail(self):
        base, rec = self.v2_case('tail')
        self.assertEqual(self.check(base, rec)['end_digest'], rec['steps'][0]['after']['digest'])

    def test_v2_lattice(self):
        base, rec = self.v2_case('lattice_v2')
        # The copy has a different directory from the one used by the restorer.
        # Its run-relative input must still bind to this replay, including --out.
        parent = self.tmp / 'v2-check-output'
        parent.mkdir()
        final = self.check(base, rec, report_parent=parent)
        self.assertTrue(final['lattice_checked'])
        self.assertEqual(final['end_digest'], C.SOLVED)
        report, = (parent / 'check').glob('*/report.json')
        self.assertTrue(report.with_suffix('.stdout.log').read_text().splitlines()[-1].startswith('PASS '))
        self.assertTrue(report.with_suffix('.stderr.log').is_file())
        self.assertFalse((base / 'check').exists())

    def test_v2_fallback(self):
        base, rec = self.v2_case('lattice_v2')
        self.fallback_step(base, rec)
        self.assertTrue(self.check(base, rec)['lattice_checked'])

    def test_v2_bulk_close(self):
        base, rec = self.v2_case()
        step = rec['steps'][0]
        step['rule'] = 'close'
        self.check(base, rec)
        step.update(V_source='tclose2', V_letters=0, V_sha256=word_sha([]),
                    parts=[['V', 0], ['middle', step['letters']], ['V_inverse', 0]])
        self.check(base, rec, 'closure_keys')

    def test_v2_close_shape(self):
        base, rec = self.v2_case('tail')
        rec['steps'][0].update(rule='close', V_letters=0, V_sha256=word_sha([]),
                               parts=[['V', 0], ['middle', 1254], ['V_inverse', 0]])
        self.check(base, rec)

    def test_v2_w_only_shape(self):
        base, rec = self.v2_case('tail')
        step = rec['steps'][0]
        for key in ('V_source', 'V_letters', 'V_sha256'):
            del step[key]
        step['parts'] = [['W', 1254]]
        self.check(base, rec)

    def test_v2_parts_sum(self):
        base, rec = self.v2_case('tail')
        rec['steps'][0]['parts'][1][1] -= 1
        self.check(base, rec, 'parts')

    def test_v2_parts_shape(self):
        base, original = self.v2_case('tail')
        for parts in (None, {}, [['W', True]], [['W', -1]], [['W', 1254]],
                      [['W', 288], ['middle', 339], ['V', 288], ['V_inverse', 339]]):
            with self.subTest(parts=parts):
                rec = copy.deepcopy(original)
                rec['steps'][0]['parts'] = parts
                self.check(base, rec, 'parts')

    def test_v2_V_length(self):
        base, rec = self.v2_case('tail')
        rec['steps'][0]['parts'][1][1] -= 1
        rec['steps'][0]['parts'][2][1] += 1
        self.check(base, rec, 'V_sha256')

    def test_v2_V_sha256(self):
        base, original = self.v2_case('tail')
        for sha in ('0' * 64, 'z' * 64, None):
            with self.subTest(sha=sha):
                rec = copy.deepcopy(original)
                rec['steps'][0]['V_sha256'] = sha
                self.check(base, rec, 'V_sha256')

    def test_v2_V_inverse(self):
        base, rec = self.v2_case('tail')
        path = base / rec['steps'][0]['word']
        word = read(path)
        word['word'][915][1] = (word['word'][915][1] + 1) % 60
        write(path, word)
        self.check(base, rec, 'V_inverse')

    def test_v2_middle(self):
        base, rec = self.v2_case('tail')
        path = base / rec['steps'][0]['word']
        word = read(path)
        word['word'][627][1] = (word['word'][627][1] + 1) % 60
        write(path, word)
        self.check(base, rec, 'middle')

    def test_v2_gate(self):
        base, original = self.v2_case('tail')
        for gate, source in (('slp_apply', 'tclose2'), ('stepchk', 'certificate'), (None, 'tclose2'),
                             ('unknown', 'tclose2')):
            with self.subTest(gate=gate, source=source):
                rec = copy.deepcopy(original)
                rec['steps'][0].update(gate=gate, V_source=source)
                self.check(base, rec, 'gate')

    def test_v2_certificate(self):
        base, rec = self.v2_case('tail')
        rec['steps'][0].update(V_source='certificate', gate='slp_apply', T_size=1,
                               T_digest='a' * 64, N1=True, N2=True)
        # T is deliberately not rebuilt: only the literal word is replayed.
        self.check(base, rec)

    def test_v2_certificate_keys(self):
        base, original = self.v2_case('tail')
        original['steps'][0].update(V_source='certificate', gate='slp_apply', T_size=1,
                                    T_digest='a' * 64, N1=True, N2=True)
        for key, value in (('T_digest', None), ('T_size', 0), ('T_size', True), ('T_digest', 'x' * 64),
                           ('N1', 1), ('N2', False)):
            with self.subTest(key=key, value=value):
                rec = copy.deepcopy(original)
                if value is None:
                    del rec['steps'][0][key]
                else:
                    rec['steps'][0][key] = value
                self.check(base, rec, 'certificate_keys')

    def test_v2_closure_keys(self):
        base, original = self.v2_case('tail')
        for rule in ('pure', 'zero_bad'):
            with self.subTest(rule=rule):
                rec = copy.deepcopy(original)
                rec['steps'][0]['rule'] = rule
                self.check(base, rec, 'closure_keys')
        for rule in ('close', 't1prime'):
            for key in ('parts', 'V_source', 'V_letters', 'V_sha256'):
                with self.subTest(rule=rule, key=key):
                    rec = copy.deepcopy(original)
                    rec['steps'][0]['rule'] = rule
                    del rec['steps'][0][key]
                    self.check(base, rec, 'closure_keys')
        rec = copy.deepcopy(original)
        rec['steps'][0]['V_source'] = 'unknown'
        self.check(base, rec, 'closure_keys')
        rec = copy.deepcopy(original)
        for key in ('V_source', 'V_letters', 'V_sha256'):
            del rec['steps'][0][key]
        self.check(base, rec, 'closure_keys')

    def test_v2_config(self):
        base, original = self.v2_case()
        changes = [('literal_limit', v) for v in (-1, True, 1.5, None)]
        changes += [('seeds', v) for v in ('unknown', None)]
        changes += [('budgets', {'cert_seconds': v}) for v in (0, -1, True, '1', float('inf'), float('nan'))]
        changes += [('budgets', {}), ('source', {})]
        changes += [('source', {'commit': commit, 'status_sha256': sha, 'dirty': dirty})
                    for commit, sha, dirty in (('a' * 39, None, False), ('x' * 40, None, False),
                                               (None, 'x' * 64, False), (None, None, 1))]
        for key, value in changes:
            with self.subTest(key=key, value=value):
                rec = copy.deepcopy(original)
                rec['config'][key] = value
                self.check(base, rec, 'config')

    def test_v2_config_zero_and_provenance(self):
        base, rec = self.v2_case()
        rec['config'].update(literal_limit=0, seeds='default', budgets={'cert_seconds': 0.5},
                             source={'commit': 'a' * 40, 'status_sha256': 'b' * 64, 'dirty': False})
        self.check(base, rec)

    def test_v2_error(self):
        base, original = self.v2_case()
        for error in (42, 'signal /tmp/file', 'logs/child.log', 'C:\\private\\log', '~user', 'signal 15'):
            with self.subTest(error=error):
                rec = copy.deepcopy(original)
                rec['error'] = error
                self.check(base, rec, 'error')
        rec = copy.deepcopy(original)
        rec.update(result='error', error='signal 15')
        self.check(base, rec)
        rec['error'] = 'logs/child.log'
        self.check(base, rec, 'error')

    def test_v2_lattice_chain(self):
        base, rec = self.v2_case('lattice_v2')
        rec['lattice']['chain'] = True
        self.check(base, rec, 'lattice_chain')

    def test_v2_lattice_input_missing(self):
        base, rec = self.v2_case('lattice_v2')
        (base / rec['lattice']['input'][6:]).unlink()
        self.check(base, rec, 'lattice_input_missing')

    def test_v2_lattice_input_path(self):
        base, original = self.v2_case('lattice_v2')
        for spec in ('table:../input.json', 'table:/tmp/input.json', 'table:C:/input.json',
                     'table:tables\\input.json', 'word:1:1', 'table:', None):
            with self.subTest(spec=spec):
                rec = copy.deepcopy(original)
                rec['lattice']['input'] = spec
                self.check(base, rec, 'path')

    def test_v2_lattice_summary_run(self):
        base, rec = self.v2_case('lattice_v2')
        path = base / 'lattice/restore-summary.json'
        summary = read(path)
        summary['runs'][0]['input'] = 'table:start-table.json'
        write(path, summary)
        self.check(base, rec, 'lattice_summary_run')

    def test_v2_lattice_input_table(self):
        base, rec = self.v2_case('lattice_v2')
        shutil.copyfile(self.bulk / 'tables/bulk-end.json', base / rec['lattice']['input'][6:])
        self.check(base, rec, 'lattice_input_table')

    def test_gzip_words_by_content(self):
        for version in (1, 2):
            with self.subTest(version=version):
                base, rec = self.case(label=str(version))
                if version == 2:
                    rec = version2(rec)
                for step in rec['steps']:
                    old = base / step['word']
                    step['word'] += '.data'
                    (base / step['word']).write_bytes(gzip.compress(old.read_bytes(), mtime=0))
                    old.unlink()
                self.check(base, rec)

    def test_short_strict_prime(self):
        base, rec = self.case('short')
        self.check(base, rec)

    def test_strict_prime_pose(self):
        base, rec = self.case('pose_change')
        self.check(base, rec, 'strict_prime_pose')

    def test_unit_off_before(self):
        base, rec = self.case('short')
        unit = read(base / rec['steps'][0]['unit'])
        table = read(base / 'start-table.json')
        unit['pieces'].append(table['pose_id'].index(0))
        write(base / rec['steps'][0]['unit'], unit)
        rec['steps'][0].update(unit_size=2, landed=2)
        self.check(base, rec, 'unit_off_before')

    def test_phase_order(self):
        base, rec = self.case('short')
        before = {**rec['steps'][0]['after'], 'N': 0}
        write(base / 'words/00001.json', {'word': [], 'input_digest': before['digest'],
                                         'output_digest': before['digest']})
        rec['steps'].append({'index': 1, 'phase': 'bulk', 'rule': 'seam', 'word': 'words/00001.json',
                             'letters': 0, 'before': before, 'after': before})
        rec['phases']['bulk']['steps'] = 1
        self.check(base, rec, 'phase_order')

    def test_admissible(self):
        base, rec = self.case('short')
        path = base / rec['steps'][0]['word']
        word = read(path)
        word['word'] = [self.blocked_letter]
        write(path, word)
        self.check(base, rec, 'admissible')

    def test_bulk_table_digest(self):
        base, rec = self.case()
        shutil.copyfile(self.short / 'tables/bulk-end.json', base / 'tables/bulk-end.json')
        self.check(base, rec, 'bulk_table_digest')

    def test_contract(self):
        base, rec = self.case('short')
        rec['steps'][0]['contract'] = 'fallback'
        self.check(base, rec, 'contract')

    def test_others_landed(self):
        base, rec = self.case('short')
        rec['steps'][0]['others_landed'] += 1
        self.check(base, rec, 'others_landed')

    def test_lattice_chain(self):
        base, rec = self.case('lattice')
        rec['lattice']['chain'] = True
        self.check(base, rec, 'lattice_chain')

    def test_lattice_status(self):
        base, rec = self.case('lattice')
        path = base / 'lattice/restore-summary.json'
        summary = read(path)
        summary['runs'][0]['status'] = 'not restored'
        write(path, summary)
        self.check(base, rec, 'lattice_status')

    def test_lattice_verify_fail_line(self):
        base, rec = self.case('lattice')
        labels, = (base / 'lattice').glob('start-labels-*.npy')
        data = np.load(labels)
        data[0], data[1] = data[1], data[0]
        np.save(labels, data)
        self.check(base, rec, 'lattice_verify')
        report, = (base / 'check').glob('*/report.json')
        self.assertTrue(report.with_suffix('.stdout.log').read_text().splitlines()[-1].startswith('FAIL '))

    def test_lattice_gzip_chain_word(self):
        base, rec = self.case('lattice')
        self.fallback_step(base, rec)
        path = base / rec['steps'][0]['word']
        path.write_bytes(gzip.compress(path.read_bytes(), mtime=0))
        self.assertTrue(self.check(base, rec)['lattice_checked'])

    def test_lattice_verify_timeout(self):
        for version, fixture in ((1, 'lattice'), (2, 'lattice_v2')):
            with self.subTest(version=version):
                base, rec = self.case(fixture, str(version))
                write(base / 'run.json', rec)
                verifier = base / 'sleeper.py'
                verifier.write_text(
                    "import json, os, pathlib, signal, subprocess, sys, time\n"
                    "report = pathlib.Path(sys.argv[5])\n"
                    "child = subprocess.Popen([sys.executable, '-I', '-B', '-c', 'import time; time.sleep(60)'])\n"
                    "def stop(sig, frame):\n"
                    "    child.wait(timeout=2)\n"
                    "    raise SystemExit(0)\n"
                    "signal.signal(signal.SIGTERM, stop)\n"
                    "report.with_suffix('.pids.json').write_text(json.dumps([os.getpid(), child.pid]))\n"
                    "print('verifier started', flush=True)\n"
                    "print('verifier stderr', file=sys.stderr, flush=True)\n"
                    "time.sleep(60)\n")
                output = base / 'result.jsonl'
                argv = ['check_run.py', str(base), '--out', str(output), '--verify-seconds', '0.5']
                started = time.monotonic()
                with patch.object(C, 'VERIFY_LATE', verifier), patch.object(sys, 'argv', argv), \
                        patch.object(sys, 'stdout', io.StringIO()):
                    self.assertEqual(C.main(), 1)
                self.assertLess(time.monotonic() - started, 10)
                final = json.loads(output.read_text().splitlines()[-1])
                self.assertIn('lattice_verify_timeout', final['fails'])
                pids, = (base / 'check').glob('*/report.pids.json')
                for pid in read(pids):
                    with self.assertRaises(ProcessLookupError, msg='verifier or child survived timeout'):
                        os.kill(pid, 0)
                self.assertEqual(pids.with_name('report.stdout.log').read_text(), 'verifier started\n')
                self.assertEqual(pids.with_name('report.stderr.log').read_text(), 'verifier stderr\n')


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        super().stopTest(test)
        self.stream.writeln('  %.2fs' % (time.monotonic() - self.started))


if __name__ == '__main__':
    unittest.main(verbosity=2, testRunner=unittest.TextTestRunner(verbosity=2, resultclass=TimedResult))
