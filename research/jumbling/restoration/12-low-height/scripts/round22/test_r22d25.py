"""Acceptance of r22d25-cat/1; all scratch and caches live outside the worktree.

Run from the repository root: python3 -I -B <this file>
R22D25_SLOW=1 additionally rebuilds the 3000-twist stop (not a coverage test).
"""
import os
import sys
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ.setdefault(_name, '1')
sys.dont_write_bytecode = True

import ast
import hashlib
import importlib.util
import json
import shutil
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[5]
sys.path.insert(0, str(HERE))
import r22d25_search as S
K = S.K
np = K.np
CENSUS = ROOT / 'research/jumbling/restoration/11-scramble-census/runs'
CHECKER = ROOT / 'research/jumbling/restoration/13-tutorial-runner/scripts/check_r22d25.py'
CHECKER_TEST = CHECKER.with_name('test_check_r22d25.py')
Z7102 = '4ca440085d743d680163a35d735ff6a7c35796a57cba720ab4037a49887734db'
END16 = 'dceb9359b1b549c32afa36c2f88f6cd2cb6ef75feacef1b675f5c99e6b20a17b'
END43 = 'f0290f93bea0295ecc00c2a42f725abfd59d3c37732834bb82f5da41c38fa793'
Z36 = '8733a0aafd8e24ea1da990ebab0347074c20b11cf61d9ed04d34d25ef6f67261'
END36 = 'd79c03679e3b065164219ebd747cc2de89bc5759c081f7b95e9554cbb8c11ea1'
WORD16 = [[61, 57], [287, 3]] + [[110, 54], [149, 3], [110, 24], [149, 9]] * 6 + [[287, 9], [61, 57]]
WORD43 = [[61, 57]] + [[110, 3], [285, 58], [110, 9], [285, 26]] * 9 + [[61, 57]]
WORD36 = [[0, 12], [74, 8], [188, 4], [74, 4], [188, 8], [188, 4], [188, 4],
          [74, 8], [188, 8], [74, 4], [188, 8], [178, 1], [188, 4], [74, 8],
          [188, 4], [74, 4], [188, 8], [188, 8], [188, 4], [74, 8], [188, 8],
          [74, 4], [178, 1], [0, 45], [14, 12], [12, 12]]
STRADDLERS = [920, 921, 922, 1039, 1040, 1041, 1043, 1068, 1069, 7691, 7692, 7693, 7694, 7695]
FIXTURE = None


def env(cache):
    return dict(os.environ, OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1', MKL_NUM_THREADS='1',
                JUMBLING_HOME_CACHE=str(Path(cache) / 'home_centroids.npy'))


def stub(directory, kind, word=WORD16):
    path = Path(directory) / ('stub-' + kind + ('-short' if len(word) != 28 else '') + '.py')
    payload = {'word': word, 'input_digest': Z7102, 'output_digest': END16}
    script = """import argparse, json
from pathlib import Path
p=argparse.ArgumentParser()
for n in ('table','out','cache','seconds'): p.add_argument('--'+n, required=True)
a=p.parse_args(); out=Path(a.out); out.mkdir(parents=True,exist_ok=True)
report={'result':RESULT}
if RESULT=='step':
 report['step']={'unit':391}
 (out/'word.json').write_text(json.dumps(PAYLOAD))
(out/'report.json').write_text(json.dumps(report))
""".replace('RESULT', repr(kind)).replace('PAYLOAD', repr(payload))
    path.write_text(script)
    return path


def fixture():
    global FIXTURE
    if FIXTURE is None:
        temp = Path(tempfile.mkdtemp(prefix='r22d25-tests-'))
        K.set_cache(temp / 'cache')
        t = time.monotonic()
        st = S.load_start('record', CENSUS / 'round2/ia-k100-s7102.json')
        snap = K.snapshot(st)
        table = temp / 'seed7102.json'
        K.B.export_pose_table(st, table)
        FIXTURE = {'temp': temp, 'state': st, 'snapshot': snap, 'table': table,
                   'stub': stub(temp, 'step'), 'budget_stub': stub(temp, 'budget'),
                   'stuck_stub': stub(temp, 'stuck'), 'short_stub': stub(temp, 'step', WORD16[:-1])}
        print(f'fixture seed7102: {time.monotonic()-t:.2f}s', flush=True)
    return FIXTURE


def checked_report(test, path):
    """Validate every produced record against packet section 7."""
    doc = json.loads(Path(path).read_text())
    test.assertEqual(doc['format'], 'r22d25-run/1')
    test.assertEqual(doc['catalogue'], 'r22d25-cat/1')
    test.assertEqual(doc['limits'], S.DEFAULT_LIMITS)
    test.assertIn(doc['result'], ('lattice', 'stuck', 'budget', 'unsupported', 'error'))
    test.assertGreaterEqual(doc['seconds'], 0)
    for key in ('start', 'last'):
        row = doc[key]
        test.assertEqual(len(row['digest']), 64)
        test.assertEqual(row['off'], sum(row['heights'].values()))
    for i, row in enumerate(doc['steps']):
        test.assertEqual(row['index'], i)
        test.assertIn(row['kind'], ('landing', 'lowering'))
        test.assertIsInstance(row['high_exact'], bool)
        test.assertEqual(row['target'], sorted(set(row['target'])))
        test.assertTrue(row['target'])
        word = json.loads((Path(path).parent / row['word']).read_text())
        test.assertEqual(set(word), {'word', 'input_digest', 'output_digest'})
        test.assertEqual(row['letters'], len(word['word']))
        test.assertEqual(row['input_digest'], word['input_digest'])
        test.assertEqual(row['output_digest'], word['output_digest'])
        test.assertEqual(row['off_after'], sum(row['heights_after'].values()))
        test.assertGreaterEqual(row['seconds'], 0)
    test.assertTrue(set(S.FAMILIES) <= set(doc['ledger']))
    for row in doc['ledger'].values():
        test.assertIn(row['status'], ('exhausted', 'budget', 'unsupported', 'not_applicable'))
        test.assertGreaterEqual(row['tried'], 0)
        test.assertIsInstance(row['pruned'], dict)
        test.assertIn('cursor', row)
    test.assertEqual(doc['scripts'], S.script_digests())
    # Public fields (keys included) contain no absolute path.
    def relative(v):
        if isinstance(v, dict):
            return all(relative(k) and relative(x) for k, x in v.items())
        if isinstance(v, list):
            return all(relative(x) for x in v)
        return not isinstance(v, str) or not v.startswith(('/', '\\'))
    test.assertTrue(relative(doc))
    return doc


def loop_fixture(test):
    f = fixture()
    if 'run' not in f:
        out = f['temp'] / 'run'
        out.mkdir()
        shutil.copyfile(CENSUS / 'round2/ia-k100-s7102.json', out / 'start-record.json')
        t = time.monotonic()
        doc = S.run(f['state'], {'kind': 'record', 'file': 'start-record.json'}, out,
                    S.Limits(d19=f['stub'], cache=f['temp'] / 'cache'), S.Budget(600), max_steps=1)
        f['run'], f['run_doc'] = out, doc
        print(f'loop fixture (one literal d19 step): {time.monotonic()-t:.2f}s', flush=True)
    checked_report(test, f['run'] / 'run.json')
    return f


class InputTests(unittest.TestCase):
    def test_pose_table_keeps_every_piece(self):
        # Identity pieces do not contribute to State.digest(). A shortened
        # identity table can therefore have a matching digest and still omit
        # a labelled piece. The loader must check the full model separately.
        temp = Path(tempfile.mkdtemp(prefix='r22d25-input-'))
        try:
            st = K.E.sim.State(K.E.sim.get_context(), menu=K.E.R.menu('I_a'))
            path = temp / 'table.json'
            table = K.B.export_pose_table(st, path)
            table['pose_id'].pop()
            path.write_text(json.dumps(table))
            with self.assertRaisesRegex(ValueError, 'full_model'):
                S.load_start('table', path)
            st.pose_id = st.pose_id[:-1]
            with self.assertRaisesRegex(ValueError, 'full_model'):
                K.snapshot(st)
        finally:
            shutil.rmtree(temp)


class KernelTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.f = fixture()

    def test_a_snapshot7102(self):
        s = self.f['snapshot']
        self.assertEqual(s.digest, Z7102)
        self.assertEqual(s.off_count, 31479)
        self.assertEqual(s.heights_census, {'2': 31355, '4': 124})
        self.assertEqual([u['first'] for u in s.high_units], [270, 391, 408, 409, 6780, 32115, 53620, 53629, 61683])
        for u in s.high_units:
            self.assertEqual([c for c in u['caps_whole'] if c in s.clean and s.cap_branches[c] == u['branch']], [61])
        for c in range(600):
            np.testing.assert_array_equal(s.straddlers[c], K.E.straddler_set(s.state, c), err_msg=f'cap {c}')
        self.assertEqual(len(s.holes), 31479)
        self.assertEqual(len(s.branch_to_class), 5)
        self.assertEqual(s.branch_to_class[1], 3)
        for P, alphabet in s.free_alphabet.items():
            self.assertTrue(all(c in s.clean and 1 <= j < 60 for c, j in alphabet))
            caps = sorted({c for c, _ in alphabet})
            self.assertEqual(list(alphabet), [(c, j) for c in caps for j in range(1, 60)])
        print('confirmed section 4: seed7102 digest, off/heights, nine units, cap61, all 600 straddler sets', flush=True)

    def test_b_gates7102(self):
        s = self.f['snapshot']
        gate = K.strict_step(s, [391], {'word': WORD16, 'input_digest': Z7102, 'output_digest': END16})
        self.assertTrue(gate['ok'], gate['reason'])
        self.assertEqual((gate['letters'], gate['collateral'], gate['others_exact'], gate['others_landed']), (28, 0, 31437, 41))
        self.assertEqual((gate['off_before'], gate['off_after'], gate['output_digest']), (31479, 31437, END16))
        self.f['after16'] = gate['state']
        padded = K.strict_step(s, [391], [[61, 0]] + WORD16)
        self.assertTrue(padded['ok'], padded['reason'])
        self.assertEqual((padded['letters'], padded['output_digest']), (29, END16))
        high = K.strict_step(s, [391], WORD16, True)
        self.assertFalse(high['ok'])
        self.assertEqual(high['reason'], 'high_exact')
        self.assertEqual(len(high['high_pieces_changed']), 41)
        self.assertFalse(K.strict_step(s, [391], WORD16[:-1])['ok'])
        self.assertFalse(K.strict_step(s, [391], [])['ok'])
        blocked = next(c for c in range(600) if len(s.straddlers[c]))
        bad = K.strict_step(s, [391], [[blocked, 1]])
        self.assertFalse(bad['ok'])
        self.assertTrue(bad['reason'].startswith('inadmissible_prefix'))
        self.assertFalse(K.strict_step(s, [], WORD16)['ok'])
        self.assertFalse(K.strict_step(s, [391], {'word': WORD16, 'input_digest': '0' * 64})['ok'])
        self.assertEqual(s.state.digest(), Z7102)
        print('confirmed section 4: R22D-16 literal strict gate, collateral and exact/landed counts', flush=True)

    def test_c_d19_adapter(self):
        s = self.f['snapshot']
        limits = S.Limits(d19=self.f['stub'], cache=self.f['temp'] / 'cache')
        result = S.search_step(s, limits, S.Budget(300))
        self.assertEqual(result['status'], 'candidate', result)
        self.assertEqual((result['family'], result['target'], result['word']['output_digest']), ('d19', [391], END16))
        limits.d19 = self.f['short_stub']
        short = S.search_step(s, limits, S.Budget(300))
        self.assertEqual(short['status'], 'error')
        self.assertEqual(short['ledger']['d19']['tried'], 1)
        self.assertEqual(sum(short['ledger']['d19']['pruned'].values()), 1)
        limits.d19 = self.f['budget_stub']
        result = S.search_step(s, limits, S.Budget(300))
        self.assertEqual(result['status'], 'budget')
        self.assertIsNotNone(result['ledger']['d19']['cursor'])
        limits.d19 = self.f['temp'] / 'absent.py'
        self.assertEqual(S.search_step(s, limits, S.Budget(300))['status'], 'unsupported')

    def test_d_loop(self):
        f = loop_fixture(self)
        doc = f['run_doc']
        self.assertEqual(doc['result'], 'budget')
        self.assertEqual(len(doc['steps']), 1)
        self.assertEqual(doc['steps'][0]['letters'], 28)
        self.assertEqual(doc['last'], {'digest': END16, 'off': 31437, 'heights': {'2': 31355, '4': 82}})
        self.assertTrue(any(r['cursor'] for r in doc['ledger'].values()))
        end = K.play(f['state'], WORD16)
        refreshed = K.refresh_after_accept(end, END16)
        self.assertEqual(refreshed.digest, END16)
        self.assertEqual(refreshed.off_count, 31437)
        self.assertEqual(sum(u['pieces'] for u in refreshed.high_units), 82)
        self.assertFalse(np.shares_memory(refreshed.labels, f['snapshot'].labels))

    def test_e_one_second_cli(self):
        out = self.f['temp'] / 'one-second'
        child = subprocess.run([sys.executable, '-I', '-B', str(HERE / 'r22d25_search.py'), 'run',
                                '--table', str(self.f['table']), '--out', str(out), '--cache',
                                str(self.f['temp'] / 'cache'), '--seconds', '1'],
                               cwd=ROOT, env=env(self.f['temp'] / 'cache'), capture_output=True, text=True, timeout=600)
        self.assertEqual(child.returncode, 0, child.stderr)
        doc = checked_report(self, out / 'run.json')
        self.assertEqual(doc['result'], 'budget')
        self.assertTrue(any(row['status'] == 'budget' and row['cursor'] for row in doc['ledger'].values()))

    def test_f_cap61_not_protected(self):
        after16 = self.f.get('after16') or K.play(self.f['state'], WORD16)
        after43 = K.play(after16, WORD43)
        self.assertEqual(after43.digest(), END43)
        U = np.flatnonzero(K.heights(after43) == 4).tolist()
        self.assertEqual(len(U), 70)
        gate = K.protected_lowering(after43, U, [[61, 15]], 4)
        self.assertFalse(gate['ok'])
        self.assertEqual(gate['reason'], 'lattice_pieces_off')
        self.assertEqual(gate['collateral'], 2361)
        self.assertEqual(len(gate['lattice_pieces_off']), 2361)
        self.assertTrue((K.heights(gate['state'])[U] == 2).all())
        print('confirmed section 4: f0290f93 cap61 lowers 70 but takes 2361 lattice pieces off', flush=True)


class Height6Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.f = fixture()
        t = time.monotonic()
        cls.st = K.play(K.E.sim.State(K.E.sim.get_context(), menu=K.E.R.menu('I_a')), WORD36)
        cls.s = K.snapshot(cls.st)
        print(f'fixture R22D-36: {time.monotonic()-t:.2f}s', flush=True)

    def test_a_positive_lowering(self):
        s = self.s
        self.assertEqual(s.digest, Z36)
        self.assertEqual((s.off_count, s.heights_census), (5800, {'2': 5799, '6': 1}))
        self.assertEqual(np.flatnonzero(s.heights == 6).tolist(), [58388])
        self.assertEqual(K.E.R.Restorer(s.state, K.E.R.menu('I_a')).misalignment(), 2934)
        gate = K.protected_lowering(s, [58388], [[12, 45]], 6)
        self.assertTrue(gate['ok'], gate['reason'])
        self.assertEqual((gate['output_digest'], gate['off_after'], gate['heights_after']), (END36, 3098, {'2': 3097, '4': 1}))
        self.assertEqual(K.E.R.Restorer(gate['state'], K.E.R.menu('I_a')).misalignment(), 1574)
        result = S.search_step(s, S.Limits(cache=self.f['temp'] / 'cache'), S.Budget(300))
        self.assertEqual(result['status'], 'candidate', result)
        self.assertEqual(result['family'], 'F1')
        self.assertLessEqual(result['word']['word'][0], [12, 45])
        table, word = self.f['temp'] / 'height6.json', self.f['temp'] / 'height6-word.json'
        K.B.export_pose_table(s.state, table)
        word.write_text(json.dumps(result['word']))
        # A fresh process has its own stable-pose key registry and pose table.
        code = """import sys,json
from pathlib import Path
sys.path.insert(0,sys.argv[1])
import r22d25_search as S
S.K.set_cache(sys.argv[4])
st=S.load_start('table',sys.argv[2])
g=S.K.protected_lowering(st,[58388],json.loads(Path(sys.argv[3]).read_text()),6)
print(json.dumps({'ok':g['ok'],'reason':g['reason'],'output_digest':g.get('output_digest')}))
assert g['ok']
"""
        child = subprocess.run([sys.executable, '-I', '-B', '-c', code, str(HERE), str(table), str(word), str(self.f['temp'] / 'cache')],
                               cwd=ROOT, env=env(self.f['temp'] / 'cache'), capture_output=True, text=True, timeout=300)
        self.assertEqual(child.returncode, 0, child.stderr)
        self.assertTrue(json.loads(child.stdout.splitlines()[-1])['ok'])
        print('confirmed section 4: R22D-36 input/output digests, 6->4, off/heights and both N values', flush=True)

    def test_b_above6_unsupported(self):
        s = self.s
        for c in s.clean:
            if s.labels[58388, c] != 1:
                continue
            for j in K.nonretained_items():
                q = K.moved_pose(s.state, s.state.pose_id[58388], c, j)
                if s.state.ctx.kplus.index_of_matrix(q) < 0 and K.pose_info(q)[0] > 6:
                    high = K.snapshot(K.play(s.state, [[c, j]]))
                    result = S.search_step(high, S.Limits(), S.Budget(100))
                    self.assertEqual(result['status'], 'unsupported')
                    self.assertGreater(high.largest_height, 6)
                    self.assertIn(str(high.largest_height), result['heights'])
                    print(f'above-height-6 reachable fixture: R22D-36 plus {(c,j)}, H={high.largest_height}', flush=True)
                    return
        self.skipTest('No clean cap holding 58388 has a one-letter move above height 6; two-letter construction was not attempted')


class BlockerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.f = fixture()
        t = time.monotonic()
        cls.st = S.load_start('record', CENSUS / 'round3/ia-k400-s7109.json')
        cls.s = K.snapshot(cls.st)
        print(f'fixture seed7109: {time.monotonic()-t:.2f}s', flush=True)

    def test_a_step5(self):
        s = self.s
        self.assertEqual(s.digest, '7f8203dd30c35fc7b3b12f0e334ad4dc5bd38886e0ce14f6c19faccc2454f5c8')
        self.assertEqual(len(s.clean), 58)
        units = {u['first']: u for u in s.high_units}
        for first, size in ((3427, 6), (3428, 3), (11229, 1)):
            self.assertEqual(units[first]['pieces'], size)
            self.assertFalse(set(units[first]['caps_whole']) & set(s.clean))
        eligible, order = K.blocker_order(s)
        for first in (3427, 3428, 11229):
            self.assertIn(252, eligible[first])
        self.assertEqual(s.straddlers[252].tolist(), STRADDLERS)
        self.assertTrue((s.heights[STRADDLERS] == 2).all())
        self.assertEqual([c for c in s.clean if (s.labels[STRADDLERS, c] == 1).all()], [158])
        blockers = [u for u in s.height2_units if set(STRADDLERS) <= set(u['piece_ids'])]
        self.assertEqual(len(blockers), 1)
        self.assertEqual(blockers[0]['pieces'], 61)
        self.assertEqual(order[0]['first'], blockers[0]['first'])
        rows = S.ledger()
        result = S.t_family(s, order[0], True, S.Limits(), S.Budget(300), (), rows)
        self.assertIsNone(result)
        self.assertEqual(rows['T1']['status'], 'exhausted')
        self.assertEqual(rows['T1']['tried'] + sum(rows['T1']['pruned'].values()), 35400)
        print('confirmed section 4: seed7109 digest, three blocked units, cap252 fourteen straddlers, one 61-piece unit, 58 clean caps and T1 exhaustion', flush=True)

    def test_b_provider_budget_is_not_exhaustion(self):
        _, order = K.blocker_order(self.s)
        def limited(snapshot, unit, high_exact, limits, budget):
            self.assertEqual(unit['height'], 2)
            self.assertEqual(unit['pieces'], 61)
            self.assertNotEqual(unit['branch'], -1)
            self.assertEqual(unit['caps_whole'],
                             np.flatnonzero((snapshot.labels[unit['piece_ids']] == 1).all(axis=0)).tolist())
            if False:
                yield []
            return 'budget'
        rows = S.ledger((limited,))
        # Step 4 passes the original target through an S lowering. Its cached
        # height/branch/caps belong to Z, whereas a provider is searching at Y.
        original_target = dict(order[0], height=4, branch=-1, caps_whole=[])
        with self.assertRaises(S.BudgetStop) as caught:
            S.t_family(self.s, original_target, True, S.Limits(), S.Budget(300), (limited,), rows)
        self.assertEqual(caught.exception.family, 'limited')
        self.assertEqual(rows['limited']['status'], 'budget')


class SlowTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('R22D25_SLOW') == '1', 'set R22D25_SLOW=1 for the 3000-twist stop')
    def test_seed7302(self):
        f = fixture()
        st = S.load_start('record', CENSUS / 'heights/ia-k3000-s7302.json')
        s = K.snapshot(st)
        self.assertEqual(s.digest, '015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f')
        self.assertTrue((s.lattice_inside > 0).all())
        # This proves F1 exhausted for every height-6 unit, independently of
        # how far the enormous F2 enumeration reaches under its time budget.
        for u in s.high_units:
            if u['height'] == 6:
                self.assertTrue(all(K.single_letter_prune(s, u['piece_ids'], c, j, lowering=True)
                                    for c in range(600) for j in range(1, 60)))
        out = f['temp'] / 'slow-run'
        out.mkdir()
        K.B.export_pose_table(st, out / 'start-table.json')
        doc = S.run(st, {'kind': 'table', 'file': 'start-table.json'}, out,
                    S.Limits(cache=f['temp'] / 'cache'), S.Budget(1))
        checked_report(self, out / 'run.json')
        self.assertIn(doc['result'], ('budget', 'unsupported'))
        print('slow ledger', json.dumps(doc['ledger']), flush=True)


# The directories these tests could write to. Other trees (Git metadata, work/
# logs and worktrees) change under concurrent sessions and are not checked here;
# the implement wrapper checks the whole worktree itself.
TREE_PATHS = ('research/jumbling/restoration/12-low-height/scripts',
              'research/jumbling/restoration/13-tutorial-runner/scripts', 'work/jumbling-cache')


def worktree_fingerprint():
    """Includes ignored files and empty directories, not merely Git's index."""
    result = {}
    for top in TREE_PATHS:
        for directory, dirs, files in os.walk(ROOT / top, followlinks=False):
            dirs.sort()
            base = Path(directory)
            result[base.relative_to(ROOT).as_posix() + '/'] = ('dir',)
            for name in sorted(files):
                p = base / name
                try:
                    result[p.relative_to(ROOT).as_posix()] = (p.stat().st_mode, hashlib.sha256(p.read_bytes()).hexdigest())
                except FileNotFoundError:
                    continue
    return result


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        self.stream.writeln(f'  duration {test.id()}: {time.monotonic()-self.started:.2f}s')
        super().stopTest(test)


def main():
    started = time.monotonic()
    before = worktree_fingerprint()
    loader = unittest.TestLoader()
    suite = unittest.TestSuite(loader.loadTestsFromTestCase(cls) for cls in (InputTests, KernelTests, Height6Tests, BlockerTests, SlowTests))
    spec = importlib.util.spec_from_file_location('independent_checker_tests', CHECKER_TEST)
    tests = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(tests)
    # The checker tests load captured state independently; they share only the
    # temporary fixture location, never a kernel gate or its result.
    tests.fixture_factory = lambda: loop_fixture(unittest.TestCase())
    suite.addTests(loader.loadTestsFromModule(tests))
    result = unittest.TextTestRunner(verbosity=2, resultclass=TimedResult).run(suite)
    unchanged = before == worktree_fingerprint()
    elapsed = time.monotonic() - started
    # The elapsed time is reported, not judged: it depends on the machine's load.
    print(json.dumps({'test_paths_unchanged_including_ignored': unchanged, 'seconds': round(elapsed, 2)}), flush=True)
    if FIXTURE:
        shutil.rmtree(FIXTURE['temp'])
    return int(not (result.wasSuccessful() and unchanged))


if __name__ == '__main__':
    sys.exit(main())
