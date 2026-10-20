"""Headless acceptance for packet D19; all generated files stay in temporary directories.

Run from the repository root:
    python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/test_r22d19_rule.py
Set R22D19_SLOW=1 to additionally try the full, two-hour enumeration.
The census stop is rebuilt once per test class. Children receive its pose table.
"""
import os
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ.setdefault(_name, '1')

import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[5]
sys.path.insert(0, str(HERE))
import r22d19_rule as D  # noqa: E402

RECORD = REPO / 'research/jumbling/restoration/11-scramble-census/runs/round2/ia-k100-s7102.json'
SOLVED = REPO / 'research/jumbling/restoration/11-scramble-census/runs/round1/ia-k10-s7001.json'
INPUT = '4ca440085d743d680163a35d735ff6a7c35796a57cba720ab4037a49887734db'
OUTPUT = 'dceb9359b1b549c32afa36c2f88f6cd2cb6ef75feacef1b675f5c99e6b20a17b'
CASE = {'unit': 391, 'cap': 61, 'item': 57, 'x': [110, 54], 'y': [149, 3], 'e': 6, 'V': [[287, 3]]}
WORD = [[61, 57], [287, 3]] + [[110, 54], [149, 3], [110, 24], [149, 9]] * 6 + [[287, 9], [61, 57]]
NUMBERS = {'unit_size': 1, 'unit_on_lattice': 1, 'collateral': 0, 'others': 31478,
           'others_exact': 31437, 'others_landed': 41, 'off_before': 31479, 'off_after': 31437}


# Other sessions' state changes on its own while this test runs; this test writes none of it.
VOLATILE = ('.git', 'work/worktrees', 'work/reviews', 'work/loop-memory')


def status():
    excludes = [f':(exclude){p}' for p in VOLATILE[1:]]
    return subprocess.check_output(['git', '--no-optional-locks', 'status', '--porcelain=v1', '--ignored',
                                    '--untracked-files=all', '--', '.'] + excludes, cwd=REPO, text=True)


def worktree_fingerprint():
    """Include ignored files and empty directories, not just Git's tracked diff."""
    result = {}
    for root, dirs, files in os.walk(REPO, followlinks=False):
        relative = Path(root).relative_to(REPO).as_posix()
        dirs[:] = sorted(d for d in dirs if Path(relative, d).as_posix().removeprefix('./') not in VOLATILE)
        result[relative + '/'] = tuple(dirs)
        for name in sorted(files):
            path = Path(root) / name
            if path == REPO / '.git':
                continue
            try:
                kind, content = ('link', os.readlink(path).encode()) if path.is_symlink() else ('file', path.read_bytes())
            except FileNotFoundError:
                continue
            result[path.relative_to(REPO).as_posix()] = (kind, hashlib.sha256(content).hexdigest())
    return result


class RuleAcceptance(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        started = time.monotonic()
        cls.tmp = Path(tempfile.mkdtemp(prefix='r22d19-acceptance-'))
        cls.addClassCleanup(shutil.rmtree, cls.tmp)
        cls.cache = cls.tmp / 'cache'
        D.configure_cache(cls.cache)
        cls.st, cls.menu = D.load_current('record', RECORD)
        if cls.st.digest() != INPUT or cls.st.off_lattice_count() != 31479:
            raise AssertionError(f'section 4 disagrees: digest={cls.st.digest()}, off={cls.st.off_lattice_count()}')
        cls.rule = D.Rule(cls.st, cls.menu)
        cls.units = cls.rule.discover(D.Deadline())
        cls.table = cls.tmp / 'state.json'
        D.B.export_pose_table(cls.st, str(cls.table))
        print(f'\nsetup: rebuilt seed 7102 once in {time.monotonic() - started:.3f}s', flush=True)

    def setUp(self):
        self.started = time.monotonic()
        self.out = Path(tempfile.mkdtemp(prefix=self._testMethodName + '-', dir=self.tmp))

    def tearDown(self):
        self.assertEqual(self.rule.Z.digest(), INPUT)
        self.assertEqual(self.rule.scratch.digest(), INPUT)
        self.assertEqual(self.st.digest(), INPUT)
        print(f'{self._testMethodName}: {time.monotonic() - self.started:.3f}s', flush=True)

    def child(self, args, timeout=1200):
        env = dict(os.environ, JUMBLING_HOME_CACHE=str(self.cache / 'home_centroids.npy'))
        for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
            env[name] = '1'
        proc = subprocess.run([sys.executable, '-I', '-B'] + [str(a) for a in args], cwd=REPO,
                              env=env, text=True, capture_output=True, timeout=timeout)
        self.assertEqual(proc.returncode, 0, proc.stdout[-4000:] + '\n' + proc.stderr[-4000:])
        return proc

    def cli(self, case=None, seconds=1200):
        args = [HERE / 'r22d19_rule.py', '--table', self.table, '--out', self.out,
                '--cache', self.cache, '--seconds', str(seconds)]
        if case is not None:
            args += ['--case', json.dumps(case)]
        self.child(args, timeout=seconds + 180)
        return self.read_report()

    def read_report(self, directory=None):
        directory = self.out if directory is None else directory
        report = json.loads((directory / 'report.json').read_text())
        self.validate_report(report)
        return report

    def validate_report(self, report):
        self.assertEqual(set(report), {'format', 'rule', 'input', 'result', 'units', 'cases', 'rejected',
                                       'step', 'reached', 'seconds', 'scripts'})
        self.assertEqual(report['format'], 'r22d19-run/1')
        self.assertEqual(report['rule'], 'R22D-19')
        self.assertIn(report['result'], ('step', 'stuck', 'budget'))

        def integer(value):
            self.assertIs(type(value), int)
            self.assertGreaterEqual(value, 0)

        def digest(value):
            self.assertIsInstance(value, str)
            self.assertRegex(value, r'^[0-9a-f]{64}$')

        def duration(value):
            self.assertIsInstance(value, (int, float))
            self.assertGreaterEqual(value, 0)
            self.assertTrue(D.math.isfinite(value))

        def relative(value):
            self.assertIsInstance(value, str)
            self.assertFalse(Path(value).is_absolute())
            self.assertNotIn('..', Path(value).parts)
            self.assertFalse(re.match(r'^[A-Za-z]:', value))
            self.assertNotIn('\\', value)

        self.assertEqual(set(report['input']), {'kind', 'name', 'digest', 'off', 'heights'})
        self.assertIn(report['input']['kind'], ('record', 'table'))
        relative(report['input']['name'])
        self.assertEqual(Path(report['input']['name']).name, report['input']['name'])
        digest(report['input']['digest'])
        integer(report['input']['off'])
        self.assertIsInstance(report['input']['heights'], dict)
        for height, count in report['input']['heights'].items():
            self.assertRegex(height, r'^\d+$')
            integer(count)
        self.assertEqual(sum(report['input']['heights'].values()), report['input']['off'])
        self.assertIsInstance(report['units'], list)
        for unit in report['units']:
            self.assertEqual(set(unit), {'unit', 'pieces', 'branch', 'caps'})
            for k in ('unit', 'pieces', 'branch'):
                integer(unit[k])
            self.assertIsInstance(unit['caps'], list)
            self.assertEqual(unit['caps'], sorted(set(unit['caps'])))
            for cap in unit['caps']:
                integer(cap)
                self.assertLess(cap, 600)
        self.assertEqual([u['unit'] for u in report['units']], sorted(u['unit'] for u in report['units']))
        self.assertIsInstance(report['cases'], list)
        for case in report['cases']:
            self.assertEqual(set(case), {'unit', 'cap', 'item', 'outcome', 'pairs', 'commutators',
                                         'landing_powers', 'vacated', 'replays', 'seconds'})
            for k in ('unit', 'cap', 'item', 'pairs', 'commutators', 'landing_powers', 'vacated', 'replays'):
                integer(case[k])
            self.assertIn(case['outcome'], set(D.REASONS) | {'step', 'budget'})
            duration(case['seconds'])
        self.assertEqual(set(report['rejected']), set(D.REASONS))
        for value in report['rejected'].values():
            integer(value)
        duration(report['seconds'])
        self.assertIsInstance(report['scripts'], dict)
        self.assertTrue(report['scripts'])
        for path, sha in report['scripts'].items():
            relative(path)
            digest(sha)
            self.assertEqual(hashlib.sha256((REPO / path).read_bytes()).hexdigest(), sha)
        if report['result'] == 'budget':
            self.assertIsInstance(report['reached'], dict)
            self.assertTrue({'phase', 'unit', 'cap', 'item'} <= set(report['reached']))
            self.assertIsInstance(report['reached']['phase'], str)
            for k in ('unit', 'cap', 'item'):
                if report['reached'][k] is not None:
                    integer(report['reached'][k])
        else:
            self.assertIsNone(report['reached'])
        if report['result'] != 'step':
            self.assertIsNone(report['step'])
        else:
            step = report['step']
            self.assertEqual(set(step), {'unit', 'cap', 'item', 'x', 'y', 'e', 'V', 'letters', 'bad', 'word',
                                         'input_digest', 'output_digest', 'check'})
            D.validate_case({k: step[k] for k in CASE})
            for k in ('letters', 'bad'):
                integer(step[k])
            self.assertEqual(step['letters'], 2 + 4 * step['e'] + 2 * len(step['V']))
            self.assertLessEqual(step['letters'], 244)
            self.assertLess(step['bad'], 3097)
            self.assertEqual(step['word'], 'word.json')
            relative(step['word'])
            digest(step['input_digest'])
            digest(step['output_digest'])
            self.assertEqual(step['input_digest'], report['input']['digest'])
            self.assertEqual(set(step['check']), set(D.CHECK_FIELDS))
            self.assertIs(step['check']['strict_prime'], True)
            for k in D.CHECK_FIELDS[1:]:
                integer(step['check'][k])
        # No output string may reveal an absolute filesystem path.
        def strings(value):
            if isinstance(value, str):
                self.assertFalse(value.startswith('/'), value)
                self.assertFalse(re.match(r'^[A-Za-z]:[\\/]', value), value)
            elif isinstance(value, dict):
                for k, v in value.items():
                    strings(k)
                    strings(v)
            elif isinstance(value, list):
                for v in value:
                    strings(v)
        strings(report)

    def independent_check(self, directory, unit=391):
        # The real stepchk CLI in another process loads a fresh full state and
        # compares exact matrices. It does not call check_step5 or stable_pose.
        ufile, checkfile = directory / 'unit.json', directory / 'independent.json'
        pieces = next(u['pieces'].tolist() for u in self.units if u['unit'] == unit)
        ufile.write_text(json.dumps({'pieces': pieces}))
        self.child([HERE / 'stepchk.py', self.table, ufile, directory / 'word.json', checkfile,
                    '--require-strict'])
        check = json.loads(checkfile.read_text())
        self.assertIs(check['strict_prime'], True)
        self.assertIs(check['output_digest_matches'], True)
        return check

    def test_01_filter(self):
        # Fail if the filter calls step 3, even if its eventual answer is right.
        with mock.patch.object(D, 'PointModel', side_effect=AssertionError('filter entered step 3')):
            with mock.patch.object(D, 'search_case', side_effect=AssertionError('filter searched')):
                cases, rejected = self.rule.filters()
        self.assertEqual(len(self.units), 9, 'section 4 unit census disagreement')
        self.assertTrue(all(u['caps'] == [61] for u in self.units))
        self.assertEqual([u['unit'] for u in self.units],
                         [270, 391, 408, 409, 6780, 32115, 53620, 53629, 61683])
        self.assertEqual([len(u['pieces']) for u in self.units], [2, 1, 47, 30, 1, 31, 5, 3, 4])
        self.assertEqual(self.rule.heights, {'2': 31355, '4': 124})
        self.assertEqual(len(cases), 108)
        self.assertEqual([(c['unit'], c['cap'], c['item']) for c in cases if c['outcome'] == 'keep'],
                         [(391, 61, 55), (391, 61, 57), (391, 61, 59)])
        expected = dict.fromkeys(D.REASONS, 0)
        expected.update(split_cells=60, no_free_grip=42, fixed_occupied=3)
        self.assertEqual(rejected, expected, 'section 4 filter disagreement')
        for c in cases:
            if c['unit'] in (270, 408, 409, 32115, 53620):
                self.assertEqual(c['outcome'], 'split_cells')
            elif c['unit'] in (53629, 61683) or c['item'] not in (55, 57, 59):
                self.assertEqual(c['outcome'], 'no_free_grip')
            elif c['unit'] == 6780:
                self.assertEqual(c['outcome'], 'fixed_occupied')
            else:
                self.assertEqual(c['outcome'], 'keep')
        self.assertFalse(any(y._models or y._safe_alphabet for y in self.rule._lowered.values()))
        print(f'filter: 9 units, 108 cases, 3 survivors; rejected={rejected}', flush=True)

    def test_02_literal_step(self):
        report = self.cli(CASE)
        self.assertEqual(report['result'], 'step', report)
        self.assertEqual(report['input']['digest'], INPUT)
        self.assertEqual(report['step']['output_digest'], OUTPUT)
        self.assertEqual({k: report['step'][k] for k in CASE}, CASE)
        self.assertEqual(report['step']['letters'], 28)
        self.assertEqual(report['step']['bad'], 111)
        word = json.loads((self.out / 'word.json').read_text())
        self.assertEqual(set(word), {'word', 'input_digest', 'output_digest'})
        self.assertEqual(word, {'word': WORD, 'input_digest': INPUT, 'output_digest': OUTPUT})
        check = self.independent_check(self.out)
        self.assertEqual(check['start_digest'], INPUT)
        self.assertEqual(check['end_digest'], OUTPUT)
        self.assertEqual(check['letters'], 28)
        self.assertEqual({k: check[k] for k in NUMBERS}, NUMBERS)
        self.assertEqual(report['step']['check'], {k: check[k] for k in D.CHECK_FIELDS})

        unit = next(u for u in self.units if u['unit'] == 391)
        reason, context = self.rule.filter_case(unit, 61, 57)
        self.assertIsNone(reason)
        y, branch, target = context
        model = y.model(branch, D.Deadline())
        x, yy = model.index[110, 54], model.index[149, 3]
        action = model.array(model.comm(x, yy, 6))
        bad, cells = model.bad(action)
        self.assertEqual(cells, [[149, 169, 325]], 'section 4 Bad-cell disagreement')
        self.assertEqual(len(bad), 111)
        self.assertEqual(model.NC, 2641)
        self.assertTrue(all(s in model.cid for s in model.geom.home))
        self.assertEqual(len(model.contents[target]), 42)
        self.assertEqual(model.caps(target[0]), [18, 84, 110])
        self.assertNotIn(target[0], model.geom.home)
        self.assertTrue(model.is_home[action[model.cid[target[0]] * 5 + target[1]]])
        protected = model.protected(target)
        self.assertTrue((action[protected] == protected).all())
        alphabet = y.safe_alphabet(D.Deadline())
        self.assertTrue(alphabet.occupied[bad].all())
        inverse_v = int(alphabet.inv[alphabet.index[287, 3]])
        self.assertFalse(alphabet.occupied[alphabet.images([inverse_v], bad)].any())
        self.assertEqual(model.adjacent(set(model.caps(target[0]))),
                         sorted(model.adjacent(set(model.caps(target[0])))))
        print(f'step: 28 letters, Bad=111 in {cells}, strict_prime=true, counts={NUMBERS}', flush=True)

    def test_03_empty_vacator(self):
        report = self.cli(dict(CASE, V=[]))
        self.assertEqual(report['result'], 'stuck')
        self.assertEqual(len(report['cases']), 1)
        case = report['cases'][0]
        self.assertEqual(case['outcome'], 'no_vacator')
        self.assertEqual(case['landing_powers'], 1)
        self.assertEqual(case['vacated'], 0)
        self.assertEqual(case['replays'], 0)
        self.assertEqual(report['rejected']['no_vacator'], 1)
        self.assertFalse((self.out / 'word.json').exists())

    def test_04_early_rejections(self):
        for unit, item, reason in ((270, 57, 'split_cells'), (391, 15, 'no_free_grip')):
            directory = self.out / f'{unit}-{item}'
            with mock.patch.object(D, 'search_case', side_effect=AssertionError('early rejection entered step 3')):
                D.run_rule(self.rule, 'table', self.table.name, directory,
                           selection={'unit': unit, 'cap': 61, 'item': item})
            report = self.read_report(directory)
            self.assertEqual(report['result'], 'stuck')
            self.assertEqual(report['cases'][0]['outcome'], reason)
            for k in ('pairs', 'commutators', 'landing_powers', 'vacated', 'replays'):
                self.assertEqual(report['cases'][0][k], 0)
            self.assertEqual(report['rejected'][reason], 1)
            self.assertFalse((directory / 'word.json').exists())

    def test_05_corrupted_word(self):
        check = D.check_step5(self.rule, [391], WORD[:-1])
        self.assertFalse(check['strict_prime'], check)
        self.assertNotEqual(check['end_digest'], OUTPUT)

    def test_06_no_high_units_record(self):
        D.main(['--record', str(SOLVED), '--out', str(self.out), '--cache', str(self.cache)])
        report = self.read_report()
        self.assertLess(report['seconds'], time.monotonic() - self.started + 1)
        self.assertEqual(report['input']['kind'], 'record')
        self.assertEqual(report['input']['name'], SOLVED.name)
        self.assertEqual(report['input']['off'], 0)
        self.assertEqual(report['result'], 'stuck')
        self.assertEqual(report['units'], [])
        self.assertEqual(report['cases'], [])
        self.assertFalse((self.out / 'word.json').exists())

    def test_07_budget(self):
        report = self.cli(seconds=1)
        self.assertEqual(report['result'], 'budget')
        self.assertEqual(report['input']['digest'], INPUT)
        self.assertIsNotNone(report['reached'])
        self.assertFalse((self.out / 'word.json').exists())

    def test_07b_budget_inside_case(self):
        # The units are cached, so the clock first runs out at the case's step-1 tick.
        report = D.run_rule(self.rule, 'table', self.table.name, self.out, seconds=1,
                            selection={'unit': 391, 'cap': 61, 'item': 57}, started=time.monotonic() - 2)
        self.assertEqual(self.read_report()['reached'], report['reached'])
        self.assertEqual(report['result'], 'budget')
        self.assertEqual({k: report['reached'][k] for k in ('unit', 'cap', 'item')},
                         {'unit': 391, 'cap': 61, 'item': 57})
        self.assertEqual(report['cases'][-1]['outcome'], 'budget')
        self.assertEqual([c['unit'] for c in report['cases']], [391])
        self.assertFalse((self.out / 'word.json').exists())

    def test_07c_unmatched_case(self):
        with self.assertRaises(ValueError):
            D.run_rule(self.rule, 'table', self.table.name, self.out, selection={'unit': 391, 'cap': 0, 'item': 1})

    @unittest.skipUnless(os.environ.get('R22D19_SLOW') == '1', 'set R22D19_SLOW=1 for the full enumeration')
    def test_08_full_enumeration(self):
        report = self.cli(seconds=7200)
        self.assertEqual(report['result'], 'step', report)
        check = self.independent_check(self.out, report['step']['unit'])
        self.assertIs(check['strict_prime'], True)
        print('full enumeration word:', (self.out / 'word.json').read_text(), flush=True)


if __name__ == '__main__':
    started = time.monotonic()
    before_status, before_tree = status(), worktree_fingerprint()
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(RuleAcceptance)
    result = unittest.TextTestRunner(verbosity=2, failfast=True).run(suite)
    after_status, after_tree = status(), worktree_fingerprint()
    unchanged = before_status == after_status and before_tree == after_tree
    if not unchanged:
        changed = sorted(k for k in before_tree.keys() | after_tree.keys()
                         if before_tree.get(k) != after_tree.get(k))
        print('ERROR: acceptance changed the worktree (ignored files included):', changed, file=sys.stderr)
    print(f'acceptance: {time.monotonic() - started:.3f}s; worktree unchanged={unchanged}', flush=True)
    sys.exit(0 if result.wasSuccessful() and unchanged else 1)
