"""RUN-1 acceptance. All generated evidence and caches live in temporary storage.

python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/test_runner.py
Set RUNNER_SLOW=1 to include the depth-20 run through the lattice phase.
"""
import hashlib
import importlib.util
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
for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[name] = '1'
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
spec = importlib.util.spec_from_file_location('tutorial_runner', HERE / 'runner.py')
R = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = R
spec.loader.exec_module(R)


def check_record(test, out, record):
    """Check runner-run/1, including checkpoints while the run is in progress."""
    test.assertEqual(set(record), {'format', 'menu', 'start', 'config', 'phases', 'steps',
                                  'stuck', 'lattice', 'result', 'totals'})
    test.assertEqual(record['format'], 'runner-run/1')
    test.assertEqual(record['menu'], 'I_a')
    test.assertIn(record['result'], ('solved', 'stuck', 'budget', 'error'))
    test.assertIs(record['config']['literal_only'], True)
    test.assertEqual(record['config']['bulk_order'], R.BULK_ORDER)
    test.assertEqual(set(record['config']['budgets']), {'bulk_steps', 'bulk_step_seconds', 'bulk_seconds',
        'tail_steps', 'rule_seconds', 'greedy_seconds', 'fallback_max_length', 'par'})

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
        R.literal(word)
        current = word['output_digest']
        test.assertGreaterEqual(step['seconds'], 0)
        if step['phase'] == 'bulk':
            test.assertLess(step['after']['N'], step['before']['N'])
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
            if 'parts' in step:
                test.assertEqual(sum(n for _, n in step['parts']), step['letters'])
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
            'restore_rc', 'verify_rc', 'verify_last_line', 'chain', 'twists', 'seconds'})
        digest(lattice['input_digest'])
        relative(lattice['library'])
        test.assertEqual(hashlib.sha256((out / lattice['library']).read_bytes()).hexdigest(),
                         lattice['library_sha256'])


class TemporaryTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix='tutorial-runner-test-'))
        self.addCleanup(shutil.rmtree, self.tmp)

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
                    spec = 'table:' + str(runner.table)
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

    def test_lattice_chain_uses_bulk_end_and_tail_words(self):
        runner = StubRunner(self.args('run', '--tail-steps', '1'))
        runner.run()
        runner.state.off.clear()
        runner.state.digest = lambda: 'a' * 64
        runner.args.no_lattice = False
        commands = []

        def script(path, argv, tag, **kwargs):
            commands.append((path, argv))
            log = runner.out / 'logs' / (tag + '.log')
            log.write_text('FAIL 0.0\n')
            return R.Child(1, 0.0, log)

        with patch.object(runner, 'script', side_effect=script):
            runner.lattice()
        checker = commands[-1][1]
        self.assertEqual(checker[5], '--chain')
        self.assertEqual(checker[6], runner.out / 'tables/bulk-end.json')
        self.assertEqual(checker[7:], [runner.out / runner.record['steps'][0]['word']])
        self.assertTrue(runner.record['lattice']['chain'])

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


class ExactTests(TemporaryTest):
    @classmethod
    def setUpClass(cls):
        cls.shared = Path(tempfile.mkdtemp(prefix='tutorial-exact-cache-'))
        cls.addClassCleanup(shutil.rmtree, cls.shared)
        cls.mechanics = R.Mechanics(cls.shared)

    def invoke(self, *arguments):
        out = self.tmp / 'run'
        command = [sys.executable, '-I', '-B', HERE / 'runner.py', '--out', out,
                   '--cache', self.shared, *arguments]
        log = self.tmp / 'runner-cli.log'
        with log.open('w') as f:
            child = subprocess.run([str(x) for x in command], cwd=ROOT,
                stdout=f, stderr=subprocess.STDOUT, timeout=1400)
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
        out, record = self.invoke('--table', str(table))
        self.assertEqual(record['result'], 'solved')
        self.assertEqual(record['steps'], [])
        self.assertFalse(record['lattice']['chain'])
        self.assertEqual(record['lattice']['restore_rc'], 0)
        self.assertEqual(record['lattice']['verify_rc'], 0)
        self.assertTrue(record['lattice']['verify_last_line'].startswith('PASS '))
        report = R.read_json(out / 'lattice/report.json')
        self.assertTrue(report['pass'])
        self.assertEqual(len(report['runs']), 1)
        self.assertTrue(report['runs'][0]['restored_to_identity'])
        summary = R.read_json(out / 'lattice/restore-summary.json')['runs'][0]
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
        self.assertEqual(candidate.parts, [['W', 288], ['closure', 966]])
        self.assertEqual(set(candidate.word), {'word', 'input_digest', 'output_digest'})
        self.assertEqual(candidate.check['off_after'], 392)
        self.assertEqual(R.read_json(runner.out / 'attempts/t1/inverse/closure/result.json')['V_source'], 'certificate')

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


def inventory():
    """Include ignored files, and compare contents as well as Git's status text."""
    result = {}
    for directory, dirs, files in os.walk(ROOT):
        dirs[:] = sorted(d for d in dirs if d != '.git')
        for name in sorted(files):
            path = Path(directory) / name
            if path == ROOT / '.git':
                continue
            value = os.readlink(path) if path.is_symlink() else hashlib.sha256(path.read_bytes()).hexdigest()
            result[path.relative_to(ROOT).as_posix()] = value
    return result


def status():
    return subprocess.check_output(['git', 'status', '--porcelain', '--ignored', '--untracked-files=all'], cwd=ROOT)


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
    in_budget = elapsed <= 25 * 60 or os.environ.get('RUNNER_SLOW') == '1'
    print(f'Acceptance elapsed: {elapsed:.3f}s.', file=sys.stderr)
    if not in_budget:
        print('FAIL: default acceptance exceeded 25 minutes.', file=sys.stderr)
    return 0 if result.wasSuccessful() and unchanged and in_budget else 1


if __name__ == '__main__':
    sys.exit(main())
