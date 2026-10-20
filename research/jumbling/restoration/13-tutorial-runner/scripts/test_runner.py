"""RUN-3 acceptance. All generated evidence and caches live in temporary storage.

python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/test_runner.py
Set RUNNER_SLOW=1 to include the depth-20 run through the lattice phase.
"""
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[name] = '1'
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
spec = importlib.util.spec_from_file_location('tutorial_runner', HERE / 'runner.py')
R = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = R
spec.loader.exec_module(R)


def check_record(test, out, record):
    """Check section 7 of RUN-3, including every committed checkpoint."""
    test.assertEqual(set(record), {'format', 'menu', 'start', 'config', 'phases', 'steps',
                                  'stuck', 'lattice', 'result', 'totals', 'error'})
    test.assertEqual(record['format'], 'runner-run/2')
    test.assertTrue(record['error'] is None or isinstance(record['error'], str))
    test.assertEqual(record['menu'], 'I_a')
    test.assertIn(record['result'], ('solved', 'stuck', 'budget', 'error'))
    test.assertIs(record['config']['literal_only'], True)
    test.assertEqual(record['config']['bulk_order'], R.BULK_ORDER)
    test.assertEqual(set(record['config']['budgets']), {'bulk_steps', 'bulk_step_seconds', 'bulk_seconds',
        'tail_steps', 'rule_seconds', 'greedy_seconds', 'cert_seconds', 'fallback_max_length', 'par'})
    test.assertGreater(record['config']['budgets']['cert_seconds'], 0)
    test.assertIs(type(record['config']['literal_limit']), int)
    test.assertGreaterEqual(record['config']['literal_limit'], 0)
    test.assertIn(record['config']['seeds'], ('atlas', 'default'))
    source = record['config']['source']
    test.assertEqual(set(source), {'commit', 'status_sha256', 'dirty'})
    if source['commit'] is not None:
        test.assertRegex(source['commit'], r'^[0-9a-f]{40}$')
    if source['status_sha256'] is not None:
        test.assertRegex(source['status_sha256'], r'^[0-9a-f]{64}$')
        test.assertIs(type(source['dirty']), bool)

    def digest(value):
        test.assertRegex(value, r'^[0-9a-f]{64}$')

    def snapshot(value):
        digest(value['digest'])
        test.assertGreaterEqual(value['off'], 0)
        test.assertEqual(sum(value['heights'].values()), value['off'])
        for key, count in value['heights'].items():
            test.assertRegex(key, r'^\d+$')
            test.assertGreater(count, 0)

    def relative(value):
        test.assertFalse(Path(value).is_absolute(), value)
        test.assertNotIn('..', Path(value).parts)
        test.assertNotIn('\\', value)
        test.assertTrue((out / value).is_file(), value)

    def no_program(value):
        if isinstance(value, dict):
            for key, child in value.items():
                test.assertNotIn(key, ('segments', 'slp', 'certificate'))
                no_program(child)
        elif isinstance(value, list):
            for child in value:
                no_program(child)
        elif isinstance(value, str):
            test.assertFalse(value.startswith('/'), value)

    no_program(record)
    snapshot(record['start'])
    if record['start']['kind'] == 'table':
        test.assertEqual(record['start']['file'], 'start-table.json')
        relative(record['start']['file'])
    else:
        test.assertEqual(record['start']['kind'], 'scramble')
        test.assertIsInstance(record['start']['k'], int)
        test.assertIsInstance(record['start']['seed'], int)
    for path, sha in record['config']['scripts'].items():
        test.assertFalse(Path(path).is_absolute())
        digest(sha)
        test.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), sha)
    snapshot(record['phases']['bulk']['end'])
    relative(record['phases']['bulk']['end']['table'])
    test.assertIn(record['phases']['bulk']['result'], ('lattice', 'stuck', 'timeout', 'budget'))
    test.assertIn(record['phases']['tail']['result'], ('lattice', 'stuck', 'budget'))
    current = record['start']['digest']
    for index, step in enumerate(record['steps']):
        test.assertEqual(step['index'], index)
        test.assertEqual(step['before']['digest'], current)
        digest(step['after']['digest'])
        relative(step['word'])
        word = R.read_json(out / step['word'])
        test.assertEqual(set(word), {'word', 'input_digest', 'output_digest'})
        test.assertEqual(word['input_digest'], current)
        test.assertEqual(word['output_digest'], step['after']['digest'])
        test.assertEqual(step['letters'], len(word['word']))
        test.assertEqual(step['word'].endswith('.json.gz'), step['letters'] > 100000)
        if step['letters'] > 100000:
            test.assertEqual((out / step['word']).read_bytes()[4:8], b'\0' * 4)
        R.literal(word)
        current = word['output_digest']
        test.assertGreaterEqual(step['seconds'], 0)
        if step['phase'] == 'bulk':
            test.assertLess(step['after']['N'], step['before']['N'])
            test.assertTrue({'gate', 'parts', 'V_source', 'V_letters', 'V_sha256', 'T_size',
                             'T_digest', 'N1', 'N2'}.isdisjoint(step))
        else:
            test.assertIn(step['phase'], ('tail', 'fallback'))
            relative(step['unit'])
            test.assertEqual(len(R.read_json(out / step['unit'])['pieces']), step['unit_size'])
            snapshot(step['before'])
            snapshot(step['after'])
            test.assertEqual(step['landed'], step['unit_size'])
            test.assertEqual(step['before']['off'] - step['after']['off'],
                             step['landed'] + step['others_landed'])
            test.assertEqual(step['contract'], 'fallback' if step['phase'] == 'fallback' else 'strict_prime')
            test.assertIn(step['gate'], ('stepchk', 'slp_apply'))
            test.assertEqual(step['gate'] == 'slp_apply', step.get('V_source') == 'certificate')
            if 'parts' in step:
                test.assertEqual(sum(n for _, n in step['parts']), step['letters'])
            closure = step['phase'] == 'tail' and (step['rule'] == 'close' or
                       (step['rule'] == 't1prime' and len(step['parts']) > 1))
            if closure:
                test.assertIn(step['V_source'], ('tclose2', 'certificate'))
                n = step['V_letters']
                test.assertGreaterEqual(n, 0)
                parts = step['parts']
                prefix = parts[0][1] if step['rule'] == 't1prime' else 0
                tail_parts = parts[1:] if step['rule'] == 't1prime' else parts
                test.assertEqual([name for name, _ in tail_parts], ['V', 'middle', 'V_inverse'])
                test.assertEqual(tail_parts[0][1], n)
                test.assertEqual(tail_parts[-1][1], n)
                v = word['word'][prefix:prefix + n]
                test.assertEqual(R.word_sha256(v), step['V_sha256'])
                if step['V_source'] == 'certificate':
                    test.assertIs(step['N1'], True)
                    test.assertIs(step['N2'], True)
                    test.assertGreaterEqual(step['T_size'], 0)
                    digest(step['T_digest'])
                else:
                    test.assertTrue({'T_size', 'T_digest', 'N1', 'N2'}.isdisjoint(step))
            else:
                test.assertTrue({'V_source', 'V_letters', 'V_sha256', 'T_size', 'T_digest',
                                 'N1', 'N2'}.isdisjoint(step))
                if step['rule'] == 't1prime':
                    test.assertEqual(step['parts'], [['W', step['letters']]])
                else:
                    test.assertNotIn('parts', step)
    for phase in ('bulk', 'tail', 'fallback'):
        steps = [s for s in record['steps'] if s['phase'] == phase]
        test.assertEqual(record['totals'][phase + '_twists'], sum(s['letters'] for s in steps))
    bulk = [s for s in record['steps'] if s['phase'] == 'bulk']
    test.assertEqual(record['phases']['bulk']['steps'], len(bulk))
    test.assertEqual(record['phases']['bulk']['twists'], sum(s['letters'] for s in bulk))
    test.assertEqual(record['phases']['tail']['steps'], len(record['steps']) - len(bulk))
    if record['stuck'] is not None:
        snapshot(record['stuck'])
        test.assertEqual(record['stuck']['digest'], current)
        for unit in record['stuck']['units']:
            test.assertEqual(set(unit), {'unit_key', 'size', 'attempts'})
            for attempt in unit['attempts']:
                test.assertEqual(attempt['unit_key'], unit['unit_key'])
                test.assertTrue({'rule', 'outcome', 'seconds'} <= set(attempt))
    if record['lattice'] is not None:
        lattice = record['lattice']
        test.assertEqual(set(lattice), {'input_digest', 'library', 'library_sha256', 'out',
            'restore_rc', 'verify_rc', 'verify_last_line', 'chain', 'input', 'twists', 'seconds'})
        test.assertIs(lattice['chain'], False)
        test.assertTrue(lattice['input'].startswith('table:'))
        relative(lattice['input'][6:])
        digest(lattice['input_digest'])
        relative(lattice['library'])
        test.assertEqual(hashlib.sha256((out / lattice['library']).read_bytes()).hexdigest(),
                         lattice['library_sha256'])


class TemporaryTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix='tutorial-runner-test-'))
        self.addCleanup(shutil.rmtree, self.tmp)
        environment = patch.dict(os.environ, JUMBLING_HOME_CACHE=str(self.tmp / 'cache/home_centroids.npy'))
        environment.start()
        self.addCleanup(environment.stop)

    def args(self, out='run', *extra):
        return R.parse_args(['--scramble', '1', '7', '--out', str(self.tmp / out),
            '--cache', str(self.tmp / 'cache'), '--bulk-steps', '0',
            '--fallback-max-length', '0', '--no-lattice', *extra])


class FakeState:
    def __init__(self, off=(1, 2)):
        self.off = set(off)
        self.journal = []

    def digest(self):
        return hashlib.sha256(json.dumps(sorted(self.off)).encode()).hexdigest() if self.off else R.SOLVED

    def off_lattice_count(self):
        return len(self.off)


class FakeMechanics:
    def inverse_word(self, word):
        return [list(pair) for pair in reversed(word)]

    def scramble(self, k, seed):
        return FakeState()

    def load(self, path):
        return FakeState(R.read_json(path)['off_pieces'])

    def export(self, state, path):
        R.atomic_json(path, {'digest': state.digest(), 'off_pieces': sorted(state.off)})

    def snapshot(self, state):
        return {'digest': state.digest(), 'off': len(state.off),
                'heights': {'2': len(state.off)} if state.off else {}}

    def off_set(self, state):
        return state.off.copy()

    def apply_word(self, state, doc):
        doc = R.literal(doc, state.digest())
        for c, i in doc['word']:
            if i != 1 or c not in state.off:
                raise ValueError('inadmissible fake letter')
            state.off.remove(c)
        if state.digest() != doc['output_digest']:
            raise ValueError('word output digest mismatch on own replay')


class StubRunner(R.Runner):
    def __init__(self, args):
        super().__init__(args, FakeMechanics())
        self.fail = set()
        self.delays = {}
        self.finished = []

    def units(self, table, work):
        work.mkdir(parents=True, exist_ok=True)
        units = []
        for key in sorted(self.state.off):
            path = work / f'u{key}.json'
            R.atomic_json(path, {'pieces': [key]})
            units.append(R.Unit(key, 1, path))
        return units

    def make_candidate(self, table, unit, work, rule='pure'):
        work.mkdir(parents=True, exist_ok=True)
        state = self.m.load(table)
        before = state.digest()
        off_before = len(state.off)
        state.off.remove(unit.key)
        word = {'word': [[unit.key, 1]], 'input_digest': before, 'output_digest': state.digest()}
        check = {'start_digest': before, 'end_digest': state.digest(), 'unit_size': 1,
                 'unit_on_lattice': 1, 'others_landed': 0, 'others_exact': len(state.off),
                 'others': len(state.off), 'off_before': off_before, 'off_after': len(state.off),
                 'collateral': 0, 'strict_prime': True}
        end = work / 'end.json'
        self.m.export(state, end)
        return R.Candidate(rule, word, check, end)

    def try_unit(self, table, unit, work):
        time.sleep(self.delays.get(unit.key, 0))
        self.finished.append(unit.key)
        if unit.key in self.fail:
            return R.Trial(attempts=[{'unit_key': unit.key, 'rule': 'pure', 'outcome': 'no word',
                                      'reason': 'stub exhausted', 'seconds': 0.0}])
        return R.Trial(self.make_candidate(table, unit, work))


class RecordTests(TemporaryTest):
    def test_source_identity_matches_worktree(self):
        runner = StubRunner(self.args())
        runner.initialize()
        source = runner.record['config']['source']
        self.assertEqual(source['commit'], subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT).decode().strip())
        status = subprocess.check_output(['git', '--no-optional-locks', 'status', '--porcelain=v1', '--untracked-files=all', '--',
                                          'research/jumbling'], cwd=ROOT)
        self.assertEqual(source['status_sha256'], hashlib.sha256(status).hexdigest())
        self.assertEqual(source['dirty'], bool(status))
        with patch.object(R.subprocess, 'check_output', side_effect=FileNotFoundError):
            self.assertEqual(R.source_identity(), {'commit': None, 'status_sha256': None, 'dirty': None})

    def test_out_with_json_anywhere_is_rejected_before_writes(self):
        for out in (self.tmp / 'x.json', self.tmp / 'a.json/run'):
            with self.subTest(out=out), patch('sys.stderr', new=io.StringIO()), self.assertRaises(SystemExit):
                R.parse_args(['--scramble', '1', '7', '--out', str(out)])
            self.assertFalse(out.exists())
        parent = self.tmp / 'cwd.json'
        parent.mkdir()
        argv = [sys.executable, '-I', '-B', HERE / 'runner.py', '--scramble', '1', '7', '--out', 'run']
        child = subprocess.run([str(a) for a in argv], cwd=parent, capture_output=True, text=True)
        self.assertEqual(child.returncode, 2)
        self.assertIn('out path must not contain .json', child.stderr)
        self.assertEqual(list(parent.iterdir()), [])
        self.assertEqual(sorted(p.name for p in self.tmp.iterdir()), ['cwd.json'])

    def test_checkpoints_and_word_files(self):
        runner = StubRunner(self.args())
        original = R.atomic_json
        checkpoints = []

        def write(path, doc):
            original(path, doc)
            if Path(path) == runner.out / 'run.json':
                check_record(self, runner.out, R.read_json(path))
                checkpoints.append(len(doc['steps']))

        with patch.object(R, 'atomic_json', side_effect=write):
            record = runner.run()
        self.assertEqual(record['result'], 'solved')
        self.assertTrue({0, 1, 2} <= set(checkpoints))
        self.assertEqual(record['totals']['tail_twists'], 2)

    def test_stuck_lists_every_unit_and_attempt(self):
        runner = StubRunner(self.args())
        runner.fail = {1, 2}
        record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual(record['result'], 'stuck')
        self.assertEqual([u['unit_key'] for u in record['stuck']['units']], [1, 2])
        self.assertTrue(all(u['attempts'] for u in record['stuck']['units']))

    def test_failed_prefix_is_recorded_on_step(self):
        runner = StubRunner(self.args('run', '--tail-steps', '1'))
        runner.fail = {1}
        record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual(record['result'], 'budget')
        self.assertEqual(record['steps'][0]['unit_key'], 2)
        self.assertEqual(record['steps'][0]['attempts'][0]['unit_key'], 1)

    def test_parallelism_preserves_first_success(self):
        words = []
        for par in (1, 2):
            runner = StubRunner(self.args(f'par{par}', '--par', str(par), '--tail-steps', '1'))
            runner.delays = {1: 0.15}
            record = runner.run()
            check_record(self, runner.out, record)
            self.assertEqual(record['steps'][0]['unit_key'], 1)
            words.append(R.read_json(runner.out / record['steps'][0]['word']))
            if par == 2:
                self.assertEqual(runner.finished[:2], [2, 1])
        self.assertEqual(words[0], words[1])

    def test_parallel_timeout_retries_earlier_units_alone(self):
        runner = StubRunner(self.args('run', '--par', '2', '--tail-steps', '1'))
        calls, original = [], runner.try_unit

        def trial(table, unit, work):
            calls.append((unit.key, work.name))
            if unit.key == 1 and not work.name.endswith('-serial'):
                return R.Trial(attempts=[{'unit_key': 1, 'rule': 'pure', 'outcome': 'timeout',
                    'reason': 'busy machine', 'seconds': 0.1}])
            if unit.key == 1:
                self.assertTrue(all(f[0] in (1, 2) for f in calls))
                candidate = original(table, unit, work).candidate
                return R.Trial(candidate, [{'unit_key': 1, 'rule': 'zero_bad', 'outcome': 'no word',
                                            'reason': 'solo retry', 'seconds': 0.0}])
            return original(table, unit, work)

        with patch.object(runner, 'try_unit', side_effect=trial):
            record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual(record['steps'][0]['unit_key'], 1)
        self.assertIn((1, 'u1-serial'), calls)
        self.assertEqual([a['reason'] for a in record['steps'][0]['attempts']], ['busy machine', 'solo retry'])

    def test_rule_timeout_is_a_failed_attempt(self):
        runner = R.Runner(self.args('run', '--tail-rules', 'pure', '--rule-seconds', '0.12'), FakeMechanics())
        runner.initialize()
        target = runner.out / 'units/test.json'
        R.atomic_json(target, {'pieces': [1]})

        def slow(script, argv, tag, **kwargs):
            return runner.execute([sys.executable, '-I', '-B', '-c',
                'import time; print("started", flush=True); time.sleep(10)'], tag, runner.args.rule_seconds)

        with patch.object(runner, 'script', side_effect=slow):
            started = time.monotonic()
            result = runner.try_unit(runner.table, R.Unit(1, 1, target), runner.out / 'attempts/timeout')
        self.assertLess(time.monotonic() - started, 3)
        self.assertIsNone(result.candidate)
        self.assertEqual(result.attempts[0]['outcome'], 'timeout')
        self.assertTrue(list((runner.out / 'logs').glob('*.log')))

    def test_lattice_pass_requires_exactly_the_right_run(self):
        runner = StubRunner(self.args())
        runner.initialize()
        runner.state.off.clear()
        # Keep a nonidentity lattice digest, without loading actual geometry.
        runner.state.digest = lambda: 'a' * 64
        runner.args.no_lattice = False

        for kind in ('zero', 'two', 'wrong input', 'not identity', 'fail line', 'good'):
            with self.subTest(kind=kind):
                def script(path, argv, tag, **kwargs):
                    log = runner.out / 'logs' / (tag + '.log')
                    out = runner.out / 'lattice'
                    spec = 'table:' + runner.rel(runner.table)
                    if path == R.CHECKER:
                        self.assertNotIn('--chain', argv)
                        checked = [{'input': spec, 'restored_to_identity': True}]
                        if kind == 'zero':
                            checked = []
                        elif kind == 'two':
                            checked *= 2
                        elif kind == 'wrong input':
                            checked[0]['input'] = 'table:wrong.json'
                        elif kind == 'not identity':
                            checked[0]['restored_to_identity'] = False
                        R.atomic_json(out / 'report.json', {'pass': True, 'runs': checked})
                        log.write_text('FAIL 0.0\n' if kind == 'fail line' else 'PASS 0.0\n')
                    else:
                        R.atomic_json(out / 'restore-summary.json', {'runs': [
                            {'input': spec, 'status': 'restored', 'retained_length': 3, 'correction_twists': 2}]})
                        log.write_text('restored\n')
                    return R.Child(0, 0.0, log)

                with patch.object(runner, 'script', side_effect=script):
                    runner.lattice()
                self.assertEqual(runner.record['result'], 'solved' if kind == 'good' else 'error')
                self.assertEqual(runner.record['lattice']['twists'], 5)

    def test_lattice_uses_relative_paths_without_chain_after_tail(self):
        runner = StubRunner(self.args('run', '--tail-steps', '1'))
        runner.run()
        runner.state.off.clear()
        runner.state.digest = lambda: 'a' * 64
        runner.args.no_lattice = False
        commands = []

        def script(path, argv, tag, **kwargs):
            commands.append((path, argv))
            self.assertEqual(kwargs['cwd'], runner.out)
            log = runner.out / 'logs' / (tag + '.log')
            log.write_text('FAIL 0.0\n')
            return R.Child(1, 0.0, log)

        with patch.object(runner, 'script', side_effect=script):
            runner.lattice()
        checker = commands[-1][1]
        self.assertNotIn('--chain', checker)
        self.assertEqual(checker[2:], ['lattice/library-merged.json', 'lattice', 'lattice/report.json'])
        self.assertFalse(runner.record['lattice']['chain'])
        self.assertEqual(runner.record['lattice']['input'], 'table:tables/00000.json')

    def test_fallback_is_recorded_separately(self):
        runner = StubRunner(self.args('run', '--fallback-max-length', '1', '--tail-steps', '1'))
        runner.fail = {1, 2}
        with patch.object(runner, 'fallback', side_effect=lambda unit, work:
                          runner.make_candidate(runner.table, unit, work, 'fallback')):
            record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual(record['result'], 'budget')
        self.assertEqual(record['steps'][0]['phase'], 'fallback')
        self.assertEqual(record['totals']['fallback_twists'], 1)
        self.assertEqual(record['totals']['tail_twists'], 0)

    def test_failed_fallback_is_in_stuck_attempts(self):
        runner = StubRunner(self.args('run', '--fallback-max-length', '1'))
        runner.fail = {1, 2}
        with patch.object(runner, 'fallback', side_effect=R.RuleFailure('bound exhausted', 'stub bound')):
            record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual(record['stuck']['units'][0]['attempts'][-1]['rule'], 'fallback')

    def test_closure_failure_preserves_vacancy_premises(self):
        runner = StubRunner(self.args())
        runner.initialize()
        unit = R.Unit(1, 1, runner.out / 'units/u.json')
        R.atomic_json(unit.target, {'pieces': [1]})

        def script(path, argv, tag, **kwargs):
            out = Path(argv[5])
            out.mkdir(parents=True, exist_ok=True)
            R.atomic_json(out / 'pre.json', {'vacancy': {'N1': False, 'N2': False, 'deficits': {'2': [2, 1]}}})
            R.atomic_json(out / 'result.json', {'result': 'no V'})
            log = runner.out / 'logs/failure.log'
            log.write_text('no V\n')
            return R.Child(0, 0.0, log)

        with patch.object(runner, 'script', side_effect=script):
            with self.assertRaises(R.RuleFailure) as raised:
                runner.closure(runner.table, unit, {'P': [], 'Q': [[1, 1]]}, runner.out / 'attempts/close')
        self.assertEqual(raised.exception.details, {'N1': False, 'N2': False, 'deficits': {'2': [2, 1]}})

    def test_rejects_programs_and_digest_mismatches(self):
        with self.assertRaises(ValueError):
            R.literal({'segments': []})
        with self.assertRaises(ValueError):
            R.literal({'word': [[True, 1]]})
        with self.assertRaises(ValueError):
            R.literal({'word': [], 'input_digest': 'a' * 64}, 'b' * 64)
        runner = StubRunner(self.args())
        runner.initialize()
        unit = runner.units(runner.table, runner.out / 'attempts/units')[0]
        candidate = runner.make_candidate(runner.table, unit, runner.out / 'attempts/bad')
        candidate.word['output_digest'] = 'a' * 64
        with self.assertRaises(ValueError):
            runner.advance(unit, candidate, [], 0.0)
        self.assertEqual(runner.record['steps'], [])

    def test_timeout_with_partial_premises_is_still_a_failed_attempt(self):
        runner = StubRunner(self.args())
        runner.initialize()
        unit = runner.units(runner.table, runner.out / 'attempts/units')[0]

        def script(path, argv, tag, **kwargs):
            out = Path(argv[5])
            out.mkdir(parents=True, exist_ok=True)
            (out / 'pre.json').write_text('{"vacancy":')
            log = runner.out / 'logs/partial.log'
            log.write_text('interrupted\n')
            return R.Child(-9, 0.1, log, timeout=True)

        with patch.object(runner, 'script', side_effect=script):
            with self.assertRaises(R.RuleFailure) as raised:
                runner.closure(runner.table, unit, {'P': [], 'Q': [[1, 1]]}, runner.out / 'attempts/partial')
        self.assertEqual(raised.exception.outcome, 'timeout')


class CertificateStub(R.Runner):
    """Exercise dispatch and gates without importing geometry or the searchers."""
    def __init__(self, args):
        super().__init__(args, FakeMechanics())
        self.calls = []
        self.fault = None
        self.n1 = self.n2 = True
        self.pre_vacancy = {}
        self.cert_format = 'r22b10/2'
        self.v_length = 2
        self.v = [[9, 0], [8, 0]]
        self.full = None

    def script(self, path, argv, tag, **kwargs):
        name = Path(path).name
        self.calls.append(name)
        log = self.out / 'logs' / (tag.replace('/', '-') + '.log')
        rows = []
        if name == 'cellpure5.py':
            out = Path(argv[3])
            R.atomic_json(out.with_name(out.stem + '-word.json'), {'word': [[1, 1]]})
        elif name == 'transport_census.py':
            R.atomic_json(argv[1], {'units': {Path(argv[2]).stem: {}}})
        elif name == 'tclose2.py':
            out = Path(argv[5])
            target = [10, 11]
            R.atomic_json(out / 'pre.json', {'ghost': {'T_on_lattice_at_end': 1, 'T_size': 1,
                'protected_exact_at_end': True, 'badW': target, 'badW_digest': R.word_sha256(target)},
                'vacancy': self.pre_vacancy})
            R.atomic_json(out / 'result.json', {'result': 'no V'})
            if self.full is not None:
                R.atomic_json(out / 'word-full.json', {'word': self.full})
        elif name == 'jordan_target.py':
            log.write_text('CONDITION ' + json.dumps({'fixed_vacant': self.n1, 'necessary': self.n2,
                                                     'necessary_deficits': {'2': [2, 1]}}) + '\n')
            return R.Child(124 if self.fault == 'jordan CPU limit' else 0, 0.0, log)
        elif name == 'r22b10_cert.py':
            self.assert_seed_args = argv[-2:]
            R.atomic_json(argv[3], {'format': self.cert_format, 'complete': self.fault != 'incomplete',
                'V_length': self.v_length,
                'state_digest': 'a' * 64 if self.fault == 'certificate digest' else R.read_json(argv[1])['digest']})
        elif name == 'check_r22b10.py':
            log.write_text('PASS extra\n' if self.fault == 'check' else 'checked PASS\n')
            return R.Child(0, 0.0, log)
        elif name == 'expand_slp.py':
            if self.fault == 'expansion over limit':
                log.write_text(json.dumps({'over_limit': True, 'length': str(self.v_length)}) + '\n')
                return R.Child(3, 0.0, log)
            R.atomic_json(argv[1], {'word': self.v[:-1] if self.fault == 'expanded word length' else self.v,
                                   'input_digest': R.read_json(argv[0])['state_digest']})
            rows = [{'letters': len(self.v) + (self.fault == 'expand'), 'word_sha256': R.word_sha256(self.v)}]
        elif name == 'slp_apply.py':
            state = self.m.load(argv[1])
            before = self.m.snapshot(state)
            pieces = R.read_json(argv[-1])['pieces']
            state.off.difference_update(pieces)
            self.m.export(state, argv[3])
            for i, segment in enumerate(R.read_json(argv[2])['segments']):
                kind, wf = next(iter(segment.items()))
                word = R.read_json(wf)['word']
                row = {'segment': i, 'kind': kind, 'length': len(word), 'digest': state.digest(), 'off': len(state.off)}
                if kind == 'safe_word_file':
                    inverse = Path(wf).stem == 'V_inverse'
                    if self.fault != 'missing hash' and not (inverse and self.fault == 'inverse missing hash'):
                        wrong_hash = self.fault == 'hash' or (inverse and self.fault == 'inverse hash')
                        row['word_sha256'] = 'a' * 64 if wrong_hash else R.word_sha256(word)
                rows.append(row)
            if self.fault == 'segment length':
                rows[0]['length'] += 1
            elif self.fault == 'segment count':
                rows.pop()
            elif self.fault == 'segment digest':
                rows[0]['digest'] = 'z' * 64
            elif self.fault == 'final segment digest':
                rows[-1]['digest'] = 'a' * 64
            rows.append({'start_digest': 'a' * 64 if self.fault == 'start digest' else before['digest'],
                'end_digest': 'a' * 64 if self.fault == 'end digest' else state.digest(),
                'strict_prime': self.fault != 'strict',
                'unit_size': len(pieces), 'unit_on_lattice': len(pieces), 'collateral': 0,
                'others': len(state.off), 'others_exact': len(state.off), 'others_landed': 0,
                'off_before': before['off'], 'off_after': len(state.off)})
        elif name == 'stepchk.py':
            # Used only for a valid literal closure in the contract tests.
            state = self.m.load(argv[0])
            before = self.m.snapshot(state)
            state.off.remove(1)
            self.m.export(state, argv[4])
            R.atomic_json(argv[3], {'start_digest': before['digest'], 'end_digest': state.digest(),
                'strict_prime': True, 'unit_size': 1, 'unit_on_lattice': 1, 'collateral': 0,
                'others': 1, 'others_exact': 1, 'others_landed': 0, 'off_before': 2, 'off_after': 1})
        else:
            raise AssertionError(name)
        log.write_text(''.join(json.dumps(row) + '\n' for row in rows))
        return R.Child(0, 0.0, log)


class CertificateTests(TemporaryTest):
    def setup_runner(self, *extra):
        runner = CertificateStub(self.args('run', '--tail-rules', 'close', *extra))
        runner.initialize()
        target = runner.out / 'units/u.json'
        R.atomic_json(target, {'pieces': [1]})
        return runner, R.Unit(1, 1, target)

    def trial(self, runner, unit):
        return runner.try_unit(runner.table, unit, runner.out / 'attempts/trial')

    def test_complete_certificate_formats_dispatch_and_advance(self):
        for fmt, seeds in (('r22b10/1', 'default'), ('r22b10/2', 'atlas')):
            with self.subTest(fmt=fmt):
                runner = CertificateStub(self.args(seeds, '--seeds', seeds))
                runner.cert_format = fmt
                runner.initialize()
                unit = R.Unit(1, 1, runner.out / 'units/u.json')
                R.atomic_json(unit.target, {'pieces': [1]})
                candidate = runner.close(runner.table, unit, {'P': [], 'Q': [[1, 1]]}, runner.out / 'attempts/close')
                self.assertEqual(candidate.gate, 'slp_apply')
                self.assertEqual(candidate.parts, [['V', 2], ['middle', 1], ['V_inverse', 2]])
                self.assertEqual(candidate.metadata['V_source'], 'certificate')
                self.assertEqual(runner.assert_seed_args, ['--seeds', seeds])
                with patch.object(runner.m, 'apply_word', side_effect=AssertionError('must load the gated end table')):
                    runner.advance(unit, candidate, [], 0.0)
                check_record(self, runner.out, runner.record)
                # The ordinary middle has no word_sha256 in the stub's segment row.
                rows = R.json_rows(runner.out / 'logs/attempts-close-whole.log')
                self.assertNotIn('word_sha256', rows[1])

    def test_strict_landing_t1prime_has_only_w_parts(self):
        runner, unit = self.setup_runner()
        wfile = runner.out / 'attempts/W.json'
        R.atomic_json(wfile, {'word': [[1, 1]]})
        candidate = runner.t1prime(runner.table, unit, wfile, runner.out / 'attempts/t1')
        self.assertEqual(candidate.parts, [['W', 1]])
        self.assertEqual(candidate.gate, 'stepchk')
        self.assertEqual(candidate.metadata, {})
        self.assertEqual(runner.calls, ['stepchk.py'])
        runner.advance(unit, candidate, [], 0.0)
        check_record(self, runner.out, runner.record)

    def test_literal_limit_zero_never_builds_certificate(self):
        runner, unit = self.setup_runner('--literal-limit', '0')
        trial = self.trial(runner, unit)
        self.assertIsNone(trial.candidate)
        self.assertEqual(trial.attempts[0]['outcome'], 'no literal closure')
        self.assertNotIn('r22b10_cert.py', runner.calls)

    def test_false_pre_vacancy_never_builds_certificate(self):
        runner, unit = self.setup_runner()
        for key in ('N1', 'N2'):
            with self.subTest(key=key):
                runner.calls.clear()
                runner.pre_vacancy = {key: False}
                trial = self.trial(runner, unit)
                self.assertIsNone(trial.candidate)
                self.assertIs(trial.attempts[0][key], False)
                self.assertNotIn('jordan_target.py', runner.calls)
                self.assertNotIn('r22b10_cert.py', runner.calls)

    def test_target_failing_n1_or_n2_is_recorded_without_certificate(self):
        runner, unit = self.setup_runner()
        for n1, n2 in ((False, True), (True, False)):
            with self.subTest(N1=n1, N2=n2):
                runner.calls.clear()
                runner.n1, runner.n2 = n1, n2
                trial = self.trial(runner, unit)
                self.assertIsNone(trial.candidate)
                self.assertEqual((trial.attempts[0]['N1'], trial.attempts[0]['N2']), (n1, n2))
                self.assertEqual(trial.attempts[0]['T_size'], 2)
                self.assertIn('deficits', trial.attempts[0])
                self.assertNotIn('r22b10_cert.py', runner.calls)

    def test_failed_certificate_expansion_and_gate_are_attempts(self):
        runner, unit = self.setup_runner()
        for fault in ('check', 'incomplete', 'certificate digest', 'expand', 'expanded word length',
                      'strict', 'hash', 'missing hash', 'inverse hash', 'inverse missing hash',
                      'segment length', 'segment count',
                      'segment digest', 'final segment digest', 'start digest', 'end digest'):
            with self.subTest(fault=fault):
                runner.fault = fault
                trial = self.trial(runner, unit)
                self.assertIsNone(trial.candidate)
                self.assertEqual(trial.attempts[0]['T_size'], 2)
                if fault == 'strict':
                    self.assertEqual(trial.attempts[0]['outcome'], 'not strict_prime')
                if fault in ('hash', 'missing hash', 'inverse hash', 'inverse missing hash',
                             'segment length', 'segment count', 'segment digest',
                             'final segment digest', 'start digest', 'end digest', 'certificate digest'):
                    self.assertEqual(trial.attempts[0]['outcome'], 'digest mismatch')
                self.assertEqual(runner.record['steps'], [])

    def test_jordan_condition_survives_its_later_cpu_limit(self):
        runner, unit = self.setup_runner()
        runner.fault = 'jordan CPU limit'
        trial = self.trial(runner, unit)
        self.assertIsNotNone(trial.candidate)
        self.assertEqual(trial.candidate.gate, 'slp_apply')

    def test_expansion_limit_exit_is_a_failed_attempt(self):
        runner, unit = self.setup_runner()
        runner.fault = 'expansion over limit'
        trial = self.trial(runner, unit)
        self.assertIsNone(trial.candidate)
        self.assertEqual(trial.attempts[0]['outcome'], 'over literal limit')
        self.assertEqual(trial.attempts[0]['V_length'], str(runner.v_length))
        self.assertNotIn('slp_apply.py', runner.calls)

    def test_over_limit_records_exact_decimal_without_expanding(self):
        runner, unit = self.setup_runner('--literal-limit', '5')
        runner.v_length = 10 ** 180
        trial = self.trial(runner, unit)
        self.assertIsNone(trial.candidate)
        self.assertEqual(trial.attempts[0]['outcome'], 'over literal limit')
        self.assertEqual(trial.attempts[0]['V_length'], str(10 ** 180))
        self.assertNotIn('expand_slp.py', runner.calls)
        # W contributes to the whole-step preflight, even though it is outside V W_inverse V_inverse.
        runner.v_length = 2
        with self.assertRaises(R.RuleFailure) as raised:
            runner.closure(runner.table, unit, {'P': [], 'Q': [[1, 1]]}, runner.out / 'attempts/prefix', prefix_length=1)
        self.assertEqual(raised.exception.outcome, 'over literal limit')
        self.assertNotIn('expand_slp.py', runner.calls)

    def test_tclose2_parts_are_checked_against_core_and_inverse(self):
        runner, unit = self.setup_runner()
        runner.full = [[9, 0], [1, 1], [9, 0]]
        candidate = runner.close(runner.table, unit, {'P': [], 'Q': [[1, 1]]}, runner.out / 'attempts/good')
        self.assertEqual(candidate.metadata['V_source'], 'tclose2')
        self.assertEqual(candidate.gate, 'stepchk')
        for word in ([[9, 0], [2, 1], [9, 0]], [[9, 0], [1, 1], [8, 0]], [[1, 1], [9, 0]]):
            with self.subTest(word=word):
                runner.full = word
                with self.assertRaises(R.RuleFailure) as raised:
                    runner.close(runner.table, unit, {'P': [], 'Q': [[1, 1]]}, runner.out / 'attempts/bad')
                self.assertEqual(raised.exception.outcome, 'contract failed')


def bulk_stub(out, cache):
    """Commit one step, corrupt only the uncommitted table, then block with a child."""
    runner = R.Runner(type('Args', (), {'out': out, 'cache': cache})(), FakeMechanics())
    runner.record = R.read_json(out / 'run.json')
    state = runner.m.load(out / 'start-table.json')
    before = dict(runner.m.snapshot(state), N=2)
    state.off.remove(1)
    after = dict(runner.m.snapshot(state), N=1)
    word = {'word': [[1, 1]], 'input_digest': before['digest'], 'output_digest': after['digest']}
    R.atomic_json(out / 'words/00000.json', word)
    runner.table = out / 'tables/bulk-end.json'
    runner.m.export(state, runner.table)
    runner.record['steps'].append({'index': 0, 'phase': 'bulk', 'rule': 'close',
        'word': 'words/00000.json', 'letters': 1, 'before': before, 'after': after, 'seconds': 0.0})
    runner.record['phases']['bulk'].update(steps=1, twists=1,
        end=dict(runner.m.snapshot(state), table='tables/bulk-end.json'))
    runner.save()
    # Recovery has to replay the checkpoint; loading this partial table would be wrong.
    runner.m.export(FakeState(()), runner.table)
    child = subprocess.Popen([sys.executable, '-I', '-B', '-c', 'import time; time.sleep(120)'])
    R.atomic_json(out / 'bulk-progress.json', {'step_started': time.monotonic()})
    R.atomic_json(out / 'stub-ready.json', {'bulk': os.getpid(), 'descendant': child.pid})
    time.sleep(120)


class BulkTests(TemporaryTest):
    def test_signal_cancels_rule_group_even_after_event_is_cleared(self):
        runner = StubRunner(self.args())
        runner.initialize()
        ready = self.tmp / 'rule-ready.json'
        code = ('import json, os, pathlib, subprocess, sys, time; '
                'child = subprocess.Popen([sys.executable, "-I", "-B", "-c", "import time; time.sleep(120)"]); '
                'pathlib.Path(sys.argv[1]).write_text(json.dumps([os.getpid(), child.pid])); time.sleep(120)')
        with R.ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(runner.execute, [sys.executable, '-I', '-B', '-c', code, ready], 'rule-signal', 10)
            deadline = time.monotonic() + 5
            while not ready.exists() and time.monotonic() < deadline:
                time.sleep(0.02)
            self.assertTrue(ready.exists())
            runner.on_signal(signal.SIGTERM, None)
            runner.cancel.clear()  # A scheduler reset cannot erase the signal.
            child = future.result(timeout=5)
        self.assertTrue(child.cancelled)
        self.assertFalse(child.timeout)
        self.assert_processes_exited(R.read_json(ready))
        with self.assertRaisesRegex(R.RunSignal, 'signal 15'):
            runner.require(child)

    def test_bulk_timeout_replays_last_completed_close_step(self):
        runner = StubRunner(self.args('run', '--bulk-steps', '2', '--bulk-step-seconds', '0.3'))
        original_save = runner.save

        def save():
            original_save()
            check_record(self, runner.out, runner.record)

        with patch.object(R, 'SELF', Path(__file__)), patch.object(runner, 'tail'), patch.object(runner, 'save', side_effect=save):
            record = runner.run()
        self.assertNotEqual(record['result'], 'error', record['error'])
        self.assertEqual(record['phases']['bulk']['result'], 'timeout')
        self.assertEqual(record['phases']['bulk']['steps'], 1)
        self.assertEqual(record['totals']['bulk_twists'], 1)
        self.assertEqual(runner.m.off_set(runner.state), {2})
        self.assertEqual(runner.m.load(runner.table).off, {2})
        self.assertEqual(record['steps'][0]['rule'], 'close')
        self.assertTrue({'V_source', 'V_letters', 'V_sha256', 'parts', 'gate'}.isdisjoint(record['steps'][0]))
        self.assert_processes_exited(R.read_json(runner.out / 'stub-ready.json').values())

    def assert_processes_exited(self, pids):
        for pid in pids:
            path = Path('/proc') / str(pid) / 'stat'
            # A zombie has exited; the environment's PID 1 owns its reaping.
            try:
                state = path.read_text().rsplit(')', 1)[1].split()[0]
            except FileNotFoundError:
                continue
            self.assertIn(state, ('Z', 'X'), f'process {pid} still running')

    def test_sigterm_and_sighup_preserve_child_checkpoint_and_kill_group(self):
        for number in (signal.SIGTERM, signal.SIGHUP):
            with self.subTest(signal=number):
                out = self.tmp / f'signal{number}'
                log = self.tmp / f'signal{number}.log'
                with log.open('w') as f:
                    proc = subprocess.Popen([sys.executable, '-I', '-B', str(Path(__file__)),
                        '--_signal-parent', str(out), str(self.tmp / 'cache')], cwd=ROOT,
                        stdout=f, stderr=subprocess.STDOUT, start_new_session=True)
                    try:
                        deadline = time.monotonic() + 15
                        while not (out / 'stub-ready.json').exists():
                            if proc.poll() is not None or time.monotonic() > deadline:
                                self.fail('stub did not reach committed checkpoint: ' + log.read_text())
                            time.sleep(0.02)
                        pids = R.read_json(out / 'stub-ready.json')
                        proc.send_signal(number)
                        self.assertEqual(proc.wait(timeout=10), 2, log.read_text())
                    finally:
                        R.kill_child(proc)
                record = R.read_json(out / 'run.json')
                check_record(self, out, record)
                self.assertEqual(record['result'], 'error')
                self.assertEqual(record['error'], f'signal {number}')
                self.assertEqual(len(record['steps']), 1)
                self.assertEqual(record['totals']['bulk_twists'], 1)
                self.assertEqual(record['phases']['bulk']['twists'], 1)
                self.assertEqual(record['phases']['bulk']['steps'], 1)
                table = FakeMechanics().load(out / 'tables/bulk-end.json')
                self.assertEqual(table.off, {2})
                self.assertEqual(table.digest(), record['steps'][0]['after']['digest'])
                self.assert_processes_exited(pids.values())


class ExactTests(TemporaryTest):
    @classmethod
    def setUpClass(cls):
        cls.shared = Path(tempfile.mkdtemp(prefix='tutorial-exact-cache-'))
        cls.addClassCleanup(shutil.rmtree, cls.shared)
        environment = patch.dict(os.environ, JUMBLING_HOME_CACHE=str(cls.shared / 'home_centroids.npy'))
        environment.start()
        cls.addClassCleanup(environment.stop)
        cls.mechanics = R.Mechanics(cls.shared)

    def invoke(self, *arguments):
        out = self.tmp / 'run'
        command = [sys.executable, '-I', '-B', HERE / 'runner.py', '--out', out,
                   '--cache', self.shared, *arguments]
        log = self.tmp / 'runner-cli.log'
        with log.open('w') as f:
            child = subprocess.Popen([str(x) for x in command], cwd=ROOT,
                stdout=f, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                child.wait(timeout=60 * 60)
            except subprocess.TimeoutExpired:
                # Let the runner cancel its separately owned child groups before
                # reaping it. The suite still enforces its total 60-minute budget.
                child.send_signal(signal.SIGTERM)
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    R.kill_child(child)
                raise
            finally:
                R.kill_child(child)
        detail = log.read_text()
        if (out / 'logs/runner.log').exists():
            detail += (out / 'logs/runner.log').read_text()
        self.assertEqual(child.returncode, 0, detail)
        record = R.read_json(out / 'run.json')
        check_record(self, out, record)
        return out, record

    def test_01_scramble_10_matches_census(self):
        out, record = self.invoke('--scramble', '10', '7001')
        self.assertEqual(record['result'], 'solved')
        self.assertEqual(record['totals']['bulk_twists'], 9)
        self.assertEqual(record['phases']['tail']['steps'], 0)
        self.assertIsNone(record['lattice'])
        census = R.read_json(ROOT / R.J / 'restoration/11-scramble-census/runs/round1/ia-k10-s7001.json')
        self.assertEqual([(s['rule'], s['letters']) for s in record['steps']],
                         [(s['rule'], s['twists']) for s in census['steps']])
        self.assertEqual(record['phases']['bulk']['end']['digest'], R.SOLVED)
        self.assertEqual(record['phases']['bulk']['end']['off'], 0)
        state = self.mechanics.load(out / 'start-table.json')
        for step in record['steps']:
            self.mechanics.apply_word(state, R.read_json(out / step['word']))
        self.assertEqual(state.digest(), R.SOLVED)

    def test_02_small_lattice_configuration(self):
        m = self.mechanics
        retained = sorted(set(range(len(m.menu.items))) - set(m.restore.jumble_items(m.menu)) - {0})
        self.assertTrue(retained)
        # A short nested commutator at three caps keeps this lattice fixture small.
        item = next(i for i in retained if m.restore.to_tuple(m.restore.matmul(
            m.restore.matmul(m.menu.items[i][1], m.menu.items[i][1]),
            m.menu.items[i][1])) == m.restore.I4)
        square = m.restore.to_tuple(m.restore.matmul(m.menu.items[item][1], m.menu.items[item][1]))
        inverse = next(i for i in retained if m.restore.to_tuple(m.menu.items[i][1]) == square)
        second = next(int(c) for c in m.restore.NEAR[0] if int(c) != 0)
        third = min((set(map(int, m.restore.NEAR[0])) & set(map(int, m.restore.NEAR[second]))) - {0, second})
        comm = [(0, item), (second, item), (0, inverse), (second, inverse)]
        comm_inverse = [(second, item), (0, item), (second, inverse), (0, inverse)]
        word = comm + [(third, item)] + comm_inverse + [(third, inverse)]
        state = m.sim.State(m.restore.ctx, menu=m.menu)
        for cap, item in word:
            outcome = state.apply(m.sim.Twist(m.restore.ctx, cap, m.restore.element(m.menu, cap, item),
                                             'menu', {'menu': m.menu.name}))
            self.assertTrue(outcome.applied)
            self.assertFalse(outcome.uncertain)
        self.assertEqual(state.off_lattice_count(), 0)
        self.assertNotEqual(state.digest(), R.SOLVED)
        self.assertLess(int(m.np.count_nonzero(state.pose_id)), 1000)
        table = self.tmp / 'lattice-input.json'
        m.export(state, table)
        # This same retained commutator has zero charge, so the existing w9
        # entry suffices for its exact restore and path checks. Replaying all
        # unused correction macros would spend the certificate tests' budget.
        # The opt-in depth-20 CLI test uses the complete production library.
        library = self.tmp / 'fixture-library.json.gz'
        w9 = R.read_json(ROOT / R.LIBRARY)['macros'][0]
        self.assertEqual(w9['family'], 'w9')
        R.atomic_json(library, {'macros': [w9]})
        args = R.parse_args(['--table', str(table), '--out', str(self.tmp / 'run'),
                             '--cache', str(self.shared)])
        runner = R.Runner(args, m)
        with patch.object(R, 'LIBRARY', library):
            record = runner.run()
        out = runner.out
        check_record(self, out, record)
        self.assertEqual(record['result'], 'solved', record['error'])
        self.assertEqual(record['steps'], [])
        self.assertFalse(record['lattice']['chain'])
        self.assertEqual(record['lattice']['input'], 'table:tables/bulk-end.json')
        self.assertEqual(record['lattice']['restore_rc'], 0)
        self.assertEqual(record['lattice']['verify_rc'], 0)
        self.assertTrue(record['lattice']['verify_last_line'].startswith('PASS '))
        report = R.read_json(out / 'lattice/report.json')
        self.assertTrue(report['pass'])
        self.assertEqual(len(report['runs']), 1)
        self.assertTrue(report['runs'][0]['restored_to_identity'])
        summary = R.read_json(out / 'lattice/restore-summary.json')['runs'][0]
        self.assertTrue(all(not any(value) for value in summary['nu_input'].values()))
        for path in (out / 'run.json', out / 'lattice/restore-summary.json', out / 'lattice/report.json'):
            def no_absolute(value):
                if isinstance(value, dict):
                    for key, child in value.items():
                        no_absolute(key)
                        no_absolute(child)
                elif isinstance(value, list):
                    for child in value:
                        no_absolute(child)
                elif isinstance(value, str):
                    self.assertFalse(value.startswith('/'), value)
                    self.assertNotIn(str(ROOT), value)
                    self.assertNotIn(str(self.tmp), value)
            no_absolute(R.read_json(path))
        print(f'  lattice retained solve: {summary["seconds"]}s; checker: {report["seconds"]}s',
              file=sys.stderr, flush=True)

    def test_03_stored_literal_t1prime(self):
        base = ROOT / R.J / 'restoration/12-low-height/runs/round22/line-a'
        table = self.tmp / 'input.json'
        R.atomic_json(table, R.read_json(base / 'replay2/run1/z-b21.json.gz'))
        self.assertEqual(R.read_json(table)['digest'],
                         'a3fef11ccce359a8a8456d0a0a1e82bdf26f296e08a717464b9c7b9d089f21df')
        self.assertEqual(self.mechanics.load(table).off_lattice_count(), 437)
        args = R.parse_args([
            '--table', str(table), '--out', str(self.tmp / 'run'), '--cache', str(self.shared),
            '--greedy-seconds', '0'])
        runner = R.Runner(args, self.mechanics)
        runner.initialize()
        unit_file = base / 'units/eb5-k40-k00-b04.json'
        self.assertEqual(len(R.read_json(unit_file)['pieces']), 45)
        unit = R.Unit(2198, 45, unit_file)
        wfile = base / 'replay2/inputs/b04-cp4-W.json'
        self.assertEqual(len(R.read_json(wfile)['word']), 288)
        candidate = runner.t1prime(table, unit, wfile, runner.out / 'attempts/t1')
        self.assertTrue(candidate.check['strict_prime'])
        expected = json.loads((base / 'replay2/run2/seq.jsonl').read_text().splitlines()[0])
        self.assertEqual(len(candidate.word['word']), int(expected['letters']))
        self.assertEqual(len(candidate.word['word']), 1254)
        self.assertEqual(candidate.word['output_digest'], expected['end_digest'])
        self.assertEqual(candidate.word['output_digest'],
                         'fee07e63a8cf8672ae81cbdf3c1d13b9ed83e61de758a7f2e94b13d28dfe7167')
        self.assertEqual(candidate.parts, [['W', 288], ['V', 339], ['middle', 288], ['V_inverse', 339]])
        self.assertEqual(candidate.metadata['V_source'], 'tclose2')
        self.assertEqual(candidate.gate, 'stepchk')
        self.assertEqual(set(candidate.word), {'word', 'input_digest', 'output_digest'})
        self.assertEqual(candidate.check['off_after'], 392)
        self.assertEqual(R.read_json(runner.out / 'attempts/t1/inverse/closure/result.json')['V_source'], 'certificate')
        runner.advance(unit, candidate, [], 0.0)
        check_record(self, runner.out, runner.record)
        self.assertEqual(runner.state.digest(), expected['end_digest'])
        self.assertEqual(runner.state.off_lattice_count(), 392)

    def certificate_runner(self, table_name):
        base = ROOT / R.J / 'restoration/12-low-height/runs/round22/line-a'
        table = self.tmp / 'input.json'
        R.atomic_json(table, R.read_json(base / table_name))
        args = R.parse_args(['--table', str(table), '--out', str(self.tmp / 'run'),
            '--cache', str(self.shared), '--greedy-seconds', '0'])
        runner = R.Runner(args, self.mechanics)
        runner.initialize()
        return base, table, runner

    def test_05_certificate_closure_at_current_state(self):
        base, table, runner = self.certificate_runner('literal/run1/z-02-b16.json.gz')
        self.assertEqual(runner.state.digest(), 'fde9e0fde60bf0eb9ea3ac8e2a34a9d420601bb02d5d2fbbb86bf096fd1b0403')
        self.assertEqual(runner.state.off_lattice_count(), 507)
        target = base / 'units/eb5-k40-k00-b20.json'
        unit = R.Unit(1201, len(R.read_json(target)['pieces']), target)
        self.assertEqual(unit.size, 60)
        core = R.read_json(base / 'replay2/inputs/cores/b20-core.json')
        candidate = runner.close(table, unit, core, runner.out / 'attempts/close')
        self.assertEqual(candidate.parts, [['V', 20232], ['middle', 36], ['V_inverse', 20232]])
        self.assertEqual(candidate.metadata, {'V_source': 'certificate', 'V_letters': 20232,
            'V_sha256': '2a5e1a1c186dc18f808f27d7e3e32f39d1716908c65298ec3bd0274ac2ece71d',
            'T_size': 111, 'T_digest': '395d1d2125581bbe75ca86099c65f9f286ad103047cda4b9ecb22fb7393a6e21',
            'N1': True, 'N2': True})
        self.assertEqual(candidate.gate, 'slp_apply')
        self.assertEqual(len(candidate.word['word']), 40500)
        self.assertEqual(R.word_sha256(candidate.word['word']),
                         '197d582c4c4326a7267dc389deda4900172841f459e4269bf0814250360dccb2')
        runner.advance(unit, candidate, [], 0.0)
        check_record(self, runner.out, runner.record)
        self.assertEqual(runner.state.digest(), 'aab10f250f0aedbbcc0fbf0eb655cb0e4434a839c22fc7957559a27437f46e7c')
        self.assertEqual(runner.state.off_lattice_count(), 447)
        self.assertEqual(runner.record['steps'][0]['before']['off'], 507)

    def test_06_certificate_t1prime_at_z1(self):
        base, table, runner = self.certificate_runner('literal/run2/z-02-b18.json.gz')
        self.assertEqual(runner.state.digest(), 'b1a6b4ebbeddbd05b125f9973b3c77a111a65d89963ac3478b5965fb43f47baa')
        self.assertEqual(runner.state.off_lattice_count(), 324)
        target = base / 'units/eb5-k40-k00-b07.json'
        unit = R.Unit(7, len(R.read_json(target)['pieces']), target)
        candidate = runner.t1prime(table, unit, base / 'replay2/inputs/b07-W.json', runner.out / 'attempts/t1')
        self.assertEqual(candidate.parts, [['W', 48], ['V', 139774], ['middle', 48], ['V_inverse', 139774]])
        self.assertEqual(candidate.metadata, {'V_source': 'certificate', 'V_letters': 139774,
            'V_sha256': '384e924e20f20607849d3ed78c34d1a7016c40e669c7fcd5ddaf9a2c5327d0a8',
            'T_size': 198, 'T_digest': 'c74506ed672f5e7f278c800f09264683dda8821ea4dcb28cf62ec39338917ba3',
            'N1': True, 'N2': True})
        self.assertEqual(candidate.gate, 'slp_apply')
        self.assertEqual(len(candidate.word['word']), 279644)
        runner.advance(unit, candidate, [], 0.0)
        check_record(self, runner.out, runner.record)
        self.assertEqual(runner.state.digest(), 'cc03b2f9b38a946389b0f3f0473dfd6ccb8693244cdf612a054dc0992c835049')
        self.assertEqual(runner.state.off_lattice_count(), 213)
        self.assertEqual(runner.record['steps'][0]['before']['off'], 324)

    @unittest.skipUnless(os.environ.get('RUNNER_SLOW') == '1', 'set RUNNER_SLOW=1 for depth 20')
    def test_04_slow_scramble_20(self):
        _, record = self.invoke('--scramble', '20', '7106')
        self.assertEqual(record['result'], 'solved')
        self.assertIsNotNone(record['lattice'])


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        elapsed = time.monotonic() - self.started
        super().stopTest(test)
        self.stream.writeln(f'  {test.id()}: {elapsed:.3f}s')


# Git metadata and the private trees other processes write to while the suite runs.
SKIP = ('.git', 'work/codex-sandbox-home', 'work/loop-memory', 'work/reviews', 'work/worktrees')


def inventory():
    """Include ignored files, and compare contents as well as Git's status text."""
    result = {}
    for directory, dirs, files in os.walk(ROOT):
        here = Path(directory).relative_to(ROOT)
        dirs[:] = sorted(d for d in dirs if (here / d).as_posix() not in SKIP)
        for name in sorted(files):
            path = Path(directory) / name
            if path == ROOT / '.git':
                continue
            try:
                value = os.readlink(path) if path.is_symlink() else hashlib.sha256(path.read_bytes()).hexdigest()
            except FileNotFoundError:
                continue
            result[path.relative_to(ROOT).as_posix()] = value
    return result


def status():
    return subprocess.check_output(['git', '--no-optional-locks', 'status', '--porcelain', '--ignored',
        '--untracked-files=all', '--', '.'] + [':(exclude)' + p for p in SKIP[1:]], cwd=ROOT)


def main():
    started = time.monotonic()
    before_status, before_files = status(), inventory()
    suite = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
    result = unittest.TextTestRunner(verbosity=2, resultclass=TimedResult).run(suite)
    after_status, after_files = status(), inventory()
    unchanged = before_status == after_status and before_files == after_files
    if not unchanged:
        changed = sorted(p for p in before_files.keys() | after_files.keys() if before_files.get(p) != after_files.get(p))
        print('FAIL: acceptance changed the worktree (including ignored files):', changed, file=sys.stderr)
    else:
        print('Worktree unchanged, including ignored files.', file=sys.stderr)
    elapsed = time.monotonic() - started
    print(f'Acceptance elapsed: {elapsed:.3f}s.', file=sys.stderr)
    if elapsed > 60 * 60 and os.environ.get('RUNNER_SLOW') != '1':
        print('Note: default acceptance took over 60 minutes.', file=sys.stderr)
    return 0 if result.wasSuccessful() and unchanged else 1


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == '--_bulk-child':
        bulk_stub(Path(sys.argv[2]), Path(sys.argv[4]))
        sys.exit(0)
    if len(sys.argv) > 1 and sys.argv[1] == '--_signal-parent':
        args = R.parse_args(['--scramble', '1', '7', '--out', sys.argv[2], '--cache', sys.argv[3],
                             '--bulk-steps', '2', '--no-lattice'])
        with patch.object(R, 'SELF', Path(__file__)):
            record = StubRunner(args).run()
        sys.exit(2 if record['result'] == 'error' else 0)
    sys.exit(main())
