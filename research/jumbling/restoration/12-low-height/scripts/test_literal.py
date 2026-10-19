# LIT-B acceptance: exact synthetic-DAG controls at E2 b15's actual Z1.
# Headless research simulator evidence; no native or product performance claim.
# From the repository root:
#   python3 -I -B research/jumbling/restoration/12-low-height/scripts/test_literal.py
# Single-process, with sequential children. All outputs/caches live in TEMP.
import contextlib
import copy
import gzip
import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

T0 = time.monotonic()
sys.dont_write_bytecode = True
TEMP = Path(tempfile.mkdtemp(prefix='literal-tests-'))
os.environ['JUMBLING_HOME_CACHE'] = str(TEMP / 'home_centroids.npy')
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_name] = '1'
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
sys.path[:0] = [str(HERE), str(HERE / 'round22')]
import slp_apply as A
import expand_slp as E
import replay_slp as RS
import e2_b06_t1 as B06
import numpy as np

ROOT = HERE.parent
RUNS = ROOT / 'runs/round22/line-a'
REPLAY = RUNS / 'replay2'
Z1 = REPLAY / 'run2/09-b15-z1-table.json.gz'
Z1_DIGEST = '627958f3097848aeb615086fcd0ee9e71f77aabdb5b4b94260e51dec7b4356e9'


def write_json(path, doc):
    opener = gzip.open if str(path).endswith('.gz') else open
    with opener(path, 'wt', encoding='utf-8') as stream:
        json.dump(doc, stream)
        stream.write('\n')
    return str(path)


def certificate(nodes, digest=Z1_DIGEST, root=None, version='r22b10/1'):
    nodes = copy.deepcopy(nodes)
    for node in nodes:
        if node['op'] == 'letter':
            length = 1
        elif node['op'] == 'cat':
            length = sum(nodes[n]['length'] for n in node['args'])
        else:
            length = nodes[node['arg']]['length'] * (node['exp'] if node['op'] == 'pow' else 1)
        node['length'] = length
    root = len(nodes) - 1 if root is None else root
    return {'format': version, 'state_digest': digest, 'nodes': nodes,
            'V': root, 'V_length': nodes[root]['length']}


def clone(st):
    B, _ = A.modules()
    result = B.sim.State(st.ctx, menu=st.menu)
    result.pose_id = st.pose_id.copy()
    result._poses, result._lookup = list(st._poses), dict(st._lookup)
    return result


def status():
    result = subprocess.run(['git', 'status', '--porcelain', '--ignored', '--untracked-files=all'],
                            cwd=REPO, capture_output=True, text=True, check=True)
    # Status alone misses byte changes to a file already marked modified or
    # untracked. Include every worktree file, ignored files and empty folders;
    # the worktree's .git file is read, never followed into the shared Git dir.
    snapshot = {}
    for directory, dirs, names in os.walk(REPO, followlinks=False):
        dirs.sort()
        for name in dirs + names:
            path = Path(directory) / name
            key = str(path.relative_to(REPO))
            if path.is_symlink():
                snapshot[key] = ('link', os.readlink(path))
            elif path.is_file():
                digest = hashlib.sha256()
                with path.open('rb') as stream:
                    for chunk in iter(lambda: stream.read(1024 * 1024), b''):
                        digest.update(chunk)
                snapshot[key] = ('file', digest.hexdigest())
            else:
                snapshot[key] = ('directory',)
    return result.stdout, snapshot


class Literal(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.B, cls.screen = A.modules()
        # Before LIT-A lands, seams ignores the environment variable. None of
        # the default children needs centroids; guard in-process use as well.
        import seams
        seams.CACHE = Path(os.environ['JUMBLING_HOME_CACHE'])
        cls.st, cls.M, _ = cls.screen.load('table', str(Z1))
        if cls.st.digest() != Z1_DIGEST:
            raise AssertionError('b15 middle Z1 digest changed')
        cls.G = cls.B.Alphabet(SimpleNamespace(st=cls.st), cls.M)
        cls.rotations = A.Rotations(cls.st.ctx.kplus)
        G, mul = cls.G, cls.rotations.mul
        # Include a chamber-fixed rotation and overlapping, noncommuting caps.
        for n, (c, _, k, ys) in enumerate(G.letters):
            xs = G.capx[c]
            fixed = xs[(xs == ys) & G.occupied[xs]]
            if cls.rotations.orders[k] > 2 and len(fixed):
                cls.fixed_chamber, first = int(fixed[0]), n
                break
        else:
            raise AssertionError('no occupied chamber-fixed rotation found')
        chosen = [first]
        ca, _, ka, _ = G.letters[first]
        for n, (c, _, k, _) in enumerate(G.letters):
            if c in {G.letters[m][0] for m in chosen}:
                continue
            if G.home[G.capx[ca], c].any() and mul[ka, k] != mul[k, ka]:
                chosen.append(n)
                if len(chosen) == 3:
                    break
        if len(chosen) != 3:
            raise AssertionError('no overlapping noncommuting SAFE-ALL controls')
        cls.a, cls.b, cls.c = G.pairs(chosen)
        leaves = [{'op': 'letter', 'grip': c, 'item': i} for c, i in (cls.a, cls.b, cls.c)]
        cls.cert = certificate(leaves + [
            {'op': 'cat', 'args': [0, 1, 2, 0]},
            {'op': 'inv', 'arg': 3},
            {'op': 'pow', 'arg': 3, 'exp': 2},
            {'op': 'pow', 'arg': 4, 'exp': 3},
            {'op': 'cat', 'args': [5, 1, 6, 2, 3, 1]},
            {'op': 'pow', 'arg': 7, 'exp': 4},
            {'op': 'inv', 'arg': 8},
            {'op': 'cat', 'args': [8, 3, 9, 7]},
            {'op': 'pow', 'arg': 10, 'exp': 5},
            {'op': 'cat', 'args': [11, 8, 0, 11]},
        ])
        # Independent recursive expansion for this deliberately small control.
        def expand(ident):
            node = cls.cert['nodes'][ident]
            if node['op'] == 'letter':
                return [[node['grip'], node['item']]]
            if node['op'] == 'cat':
                return [pair for child in node['args'] for pair in expand(child)]
            if node['op'] == 'inv':
                return A.inverse_word(cls.M, expand(node['arg']))
            return expand(node['arg']) * node['exp']
        cls.word = expand(cls.cert['V'])
        if not 1000 < len(cls.word) < 5000:
            raise AssertionError('control word is outside the packet size range')
        cls.word_hash = hashlib.sha256(json.dumps(cls.word).encode()).hexdigest()
        cls.cert_file = write_json(TEMP / 'synthetic-cert.json.gz', cls.cert)
        cls.word_file = write_json(TEMP / 'synthetic-word.json.gz',
                                   {'word': cls.word, 'input_digest': Z1_DIGEST})
        # Use both supported certificate versions in labelled-action controls.
        cls.cert2_file = write_json(TEMP / 'synthetic-v2.json', dict(cls.cert, format='r22b10/2'))
        A.print_row({'control_letters': len(cls.word), 'controls': [cls.a, cls.b, cls.c],
                     'chamber_fixed_rotation_at': cls.fixed_chamber})

    def cli(self, args, expected=0):
        remaining = max(1, 1490 - (time.monotonic() - T0))
        result = subprocess.run([sys.executable, '-I', '-B', *map(str, args)], cwd=REPO,
                                env=dict(os.environ), capture_output=True, text=True, timeout=remaining)
        self.assertEqual(result.returncode, expected, result.stdout + '\n' + result.stderr)
        return result

    def test_01_expand_and_labelled_actions(self):
        out = TEMP / 'expanded.json.gz'
        result = self.cli([HERE / 'expand_slp.py', self.cert_file, out])
        info = json.loads(result.stdout.splitlines()[-1])
        self.assertEqual(A.read_json(out), {'word': self.word, 'input_digest': Z1_DIGEST})
        self.assertEqual(info, {'letters': len(self.word), 'word_sha256': self.word_hash})
        again = TEMP / 'expanded-again.json'
        result = self.cli([HERE / 'expand_slp.py', self.cert_file, again])
        self.assertEqual(json.loads(result.stdout.splitlines()[-1]), info)
        self.assertEqual(A.read_json(again)['word'], self.word)
        inverted = TEMP / 'expanded-inverse.json.gz'
        result = self.cli([HERE / 'expand_slp.py', self.cert2_file, inverted, '--inverse'])
        inverse_word = A.inverse_word(self.M, self.word)
        self.assertEqual(A.read_json(inverted)['word'], inverse_word)
        inverse_hash = hashlib.sha256(json.dumps(inverse_word).encode()).hexdigest()
        self.assertEqual(json.loads(result.stdout.splitlines()[-1])['word_sha256'], inverse_hash)

        literal, slp = clone(self.st), clone(self.st)
        action_seconds = []
        action = A.literal_action
        def timed_action(*args):
            start = time.monotonic()
            value = action(*args)
            action_seconds.append(time.monotonic() - start)
            return value
        start = time.monotonic()
        with patch.object(A, 'literal_action', timed_action):
            row, _ = A.apply_segment(literal, self.M, {'safe_word_file': str(out), 'inverse': False},
                                     rotations=self.rotations)
        segment_seconds = time.monotonic() - start
        self.assertEqual(row['kind'], 'safe_word_file')
        self.assertEqual(row['length'], len(self.word))
        self.assertEqual(row['word_sha256'], self.word_hash)
        A.apply_segment(slp, self.M, {'slp': self.cert2_file}, rotations=self.rotations)
        self.assertEqual(literal.digest(), slp.digest())
        self.assertNotEqual(literal.digest(), Z1_DIGEST)
        inverse_literal, inverse_slp = clone(self.st), clone(self.st)
        row, _ = A.apply_segment(inverse_literal, self.M, {'safe_word_file': str(out), 'inverse': True},
                                 rotations=self.rotations)
        A.apply_segment(inverse_slp, self.M, {'slp': self.cert_file, 'inverse': True}, rotations=self.rotations)
        self.assertEqual(inverse_literal.digest(), inverse_slp.digest())
        self.assertNotEqual(inverse_literal.digest(), literal.digest())
        self.assertEqual(row['word_sha256'], inverse_hash)
        A.apply_segment(literal, self.M, {'safe_word_file': str(out), 'inverse': True}, rotations=self.rotations)
        self.assertEqual(literal.digest(), Z1_DIGEST)
        A.print_row({'safe_word_file_letters_per_second': round(len(self.word) / segment_seconds, 1),
                     'segment_seconds': round(segment_seconds, 6),
                     'literal_action_letters_per_second': round(len(self.word) / action_seconds[0], 1),
                     'literal_action_seconds': round(action_seconds[0], 6), 'state': Z1_DIGEST})

    def test_02_first_300_match_simulator(self):
        file = write_json(TEMP / 'first-300.json', {'word': self.word[:300], 'input_digest': Z1_DIGEST})
        fast, simulator = clone(self.st), clone(self.st)
        A.apply_segment(fast, self.M, {'safe_word_file': file}, rotations=self.rotations)
        A.apply_segment(simulator, self.M, {'word_file': file})
        self.assertEqual(fast.digest(), simulator.digest())
        self.assertNotEqual(simulator.digest(), Z1_DIGEST)
        A.print_row({'simulator_letters': 300, 'end_digest': fast.digest()})

    def test_02b_chamber_fixed_rotations_are_retained(self):
        number = self.G.index[tuple(self.a)]
        actual = A.literal_action(self.G, self.rotations, [number])
        expected = A.Evaluator(self.G, self.rotations, self.cert, root=0).evaluate()
        for left, right in zip(actual, expected):
            np.testing.assert_array_equal(left, right)
        self.assertEqual(int(actual[0][self.fixed_chamber]), self.fixed_chamber)
        self.assertNotEqual(int(actual[1][self.fixed_chamber]), 0)

    def test_03_over_limit_writes_nothing(self):
        huge = REPLAY / 'run2/09-b15-v-cert.json.gz'
        length = str(A.read_json(huge)['V_length'])
        out = TEMP / 'refused.json.gz'
        for limit in ([], ['--limit', '0']):
            result = self.cli([HERE / 'expand_slp.py', huge, out, *limit], expected=3)
            self.assertEqual(json.loads(result.stdout), {'over_limit': True, 'length': length})
            self.assertFalse(out.exists())
        out.write_bytes(b'keep existing output')
        result = self.cli([HERE / 'expand_slp.py', self.cert_file, out, '--limit', '1000'], expected=3)
        self.assertEqual(json.loads(result.stdout), {'over_limit': True, 'length': str(len(self.word))})
        self.assertEqual(out.read_bytes(), b'keep existing output')

    def test_03b_dag_validation_and_deep_iterative_expansion(self):
        # Bad lengths/references are rejected even outside the chosen root.
        for change in ({'op': 'cat', 'args': [99], 'length': 0},
                       {'op': 'pow', 'arg': 0, 'exp': -1, 'length': -1},
                       {'op': 'letter', 'grip': 0, 'item': 0, 'length': 2}):
            doc = copy.deepcopy(self.cert)
            doc['nodes'].append(change)
            path = write_json(TEMP / 'bad-dag.json', doc)
            out = TEMP / 'bad-dag-word.json'
            with self.subTest(node=change), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(E.main([path, str(out)]), 1)
                self.assertFalse(out.exists())
        doc = copy.deepcopy(self.cert)
        doc['V_length'] += 1
        path = write_json(TEMP / 'bad-v-length.json', doc)
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(E.main([path, str(TEMP / 'bad-v-word.json'), '--node', '0']), 1)
        nodes = [dict(op='letter', grip=self.a[0], item=self.a[1])]
        nodes.extend({'op': 'inv', 'arg': n} for n in range(1500))
        doc = certificate(nodes)
        path = write_json(TEMP / 'deep-dag.json', doc)
        out = TEMP / 'deep-word.json'
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(E.main([path, str(out)]), 0)
        self.assertEqual(A.read_json(out)['word'], [self.a])
        # Zero-length powers must not loop exp times.
        doc = certificate([{'op': 'cat', 'args': []}, {'op': 'pow', 'arg': 0, 'exp': 10**100}])
        path = write_json(TEMP / 'empty-power.json', doc)
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(E.main([path, str(out)]), 0)
        self.assertEqual(A.read_json(out)['word'], [])

    def test_04_unsafe_and_wrong_digest_leave_poses_unchanged(self):
        state = clone(self.st)
        before = state.snapshot()
        unsafe = [min(self.G.blocked), self.a[1]]
        path = write_json(TEMP / 'unsafe.json.gz', {'word': [self.a, unsafe], 'input_digest': Z1_DIGEST})
        with self.assertRaisesRegex(ValueError, 'segment 7:.*not a SAFE-ALL letter'):
            A.apply_segment(state, self.M, {'safe_word_file': path}, 7, self.rotations)
        self.assertEqual(state.snapshot(), before)
        self.assertEqual(state.digest(), Z1_DIGEST)
        path = write_json(TEMP / 'wrong-digest.json', {'word': [self.a], 'input_digest': '0' * 64})
        with self.assertRaisesRegex(ValueError, 'input_digest mismatch'):
            A.apply_segment(state, self.M, {'safe_word_file': path, 'inverse': False}, rotations=self.rotations)
        self.assertEqual(state.snapshot(), before)

    def driver(self, kind, limit):
        """Real driver entry points; prepared builder/checker/search/apply evidence.

        Expansion itself runs in process, so these controls need no atlas and
        do not run any builder, checker, search or simulator child.
        """
        base = TEMP / f'driver-{kind}-{limit}'
        base.mkdir()
        wd = base / 'work'
        start = write_json(base / 'start.json', {'digest': 'a' * 64})
        unit = write_json(base / 'unit.json', {'pieces': [0]})
        target = write_json(base / 'target.json', {'T': [], 'T_digest': hashlib.sha256(b'[]').hexdigest()})
        core = write_json(base / 'core.json', {'P': [self.a], 'Q': [self.b]})
        wfile = write_json(base / 'W.json', {'word': [self.a, self.b]})
        calls = []

        def run(args, log, py=RS.PY):
            calls.append([str(a) for a in args])
            name, log = Path(args[0]).name, Path(log)
            log.write_text('')
            if name == 'stepchk.py':
                word = A.read_json(args[3])['word']
                write_json(args[4], {'unit_on_lattice': int(bool(word)), 'unit_size': 1, 'collateral': 1,
                                     'others': 100, 'others_exact': 100, 'others_landed': 0,
                                     'strict_prime': False, 'end_digest': Z1_DIGEST})
            elif name == 'prep_t1prime.py':
                tag, directory = args[3], Path(args[4])
                write_json(directory / f'{tag}-z1-table.json', {'digest': Z1_DIGEST})
                write_json(directory / f'{tag}-Q.json', {'pieces': []})
                word = A.read_json(args[2])['word']
                write_json(directory / f'{tag}-winv.json', {'P': [], 'Q': A.inverse_word(self.M, word)})
            elif name == 'tclose2.py':
                directory = Path(args[6])
                directory.mkdir()
                write_json(directory / 'pre.json', {'ghost': {'badW': [], 'badW_digest': hashlib.sha256(b'[]').hexdigest(),
                           'T_size': 0, 'T_on_lattice_at_end': 0, 'protected_exact_at_end': True, 'free_alive': True}})
            elif name == 'jordan_target.py':
                log.write_text('CONDITION ' + json.dumps({'fixed_vacant': True, 'necessary': True}) + '\n')
            elif name == 'r22b10_cert.py':
                self.assertEqual(args[-2:], ['--seeds', 'atlas'])
                write_json(args[4], self.cert)
            elif name == 'check_r22b10.py':
                log.write_text('PASS\n')
            elif name == 'expand_slp.py':
                with log.open('w') as stream, contextlib.redirect_stdout(stream):
                    return E.main([str(a) for a in args[1:]])
            elif name == 'slp_apply.py':
                program = A.read_json(args[3])
                rows = []
                for j, segment in enumerate(program['segments']):
                    length = len(self.word) if 'safe_word_file' in segment or 'slp' in segment else RS.segment_length(segment)
                    rows.append({'segment': j, 'length': length})
                rows.append({'strict_prime': True, 'off_before': 96, 'off_after': 71, 'end_digest': 'e' * 64})
                log.write_text(''.join(json.dumps(row) + '\n' for row in rows))
                write_json(args[4], {'digest': 'e' * 64})
            else:
                self.fail(f'unexpected child: {name}')
            return 0

        options = ['--seeds', 'atlas', '--literal-limit', str(limit)]
        with patch.object(RS, 'run', run), patch.object(B06, 'off_at', return_value=set()), \
                contextlib.redirect_stdout(io.StringIO()):
            if kind == 'b06':
                prep = base / 'prep'
                prep.mkdir()
                write_json(prep / 'x-word.json', {'word': [self.a, self.b, self.c],
                           'input_digest': 'a' * 64, 'output_digest': 'b' * 64})
                write_json(prep / 'zx.json', {'digest': 'b' * 64})
                write_json(prep / 'free.json', {'pieces': []})
                B06.main([*options, start, str(prep), str(wd), '14'])
                tag, overhead = 'b06-14', 5
            else:
                step = {'unit': 'b15', 'unit_file': unit}
                step.update({'core': core, 'target': target} if kind == 'closure' else {'t1prime': wfile})
                plan = write_json(base / 'plan.json', {'steps': [step]})
                RS.main([start, plan, str(wd), *options])
                tag, overhead = '00-b15', 3 if kind == 'closure' else 4
        rows = [json.loads(line) for line in (wd / 'seq.jsonl').read_text().splitlines()]
        row = rows[0]
        self.assertTrue(any(Path(call[0]).name == 'check_r22b10.py' for call in calls))
        pf = wd / f'{tag}-program.json'
        if limit and 2 * len(self.word) + overhead > limit:
            self.assertEqual(row['how'], 'none')
            self.assertEqual(row['reason'], 'V is over the literal limit')
            self.assertEqual(row['V_length'], str(len(self.word)))
            self.assertFalse(pf.exists())
            self.assertFalse(any(Path(call[0]).name == 'expand_slp.py' for call in calls))
        else:
            program = A.read_json(pf)
            safe = [segment for segment in program['segments'] if 'safe_word_file' in segment]
            self.assertEqual(len(safe), 2 if limit else 0)
            if limit:
                self.assertFalse(any('slp' in segment for segment in program['segments']))
                self.assertFalse(safe[0].get('inverse', False))
                self.assertIs(safe[1]['inverse'], True)
                self.assertEqual(safe[0]['safe_word_file'], safe[1]['safe_word_file'])
                positions = [j for j, s in enumerate(program['segments']) if 'safe_word_file' in s]
                self.assertEqual(positions[1] - positions[0], 2)
                self.assertIn('core_file', program['segments'][positions[0] + 1])
                self.assertEqual(row['V'], 'literal')
                self.assertEqual(row['V_letters'], len(self.word))
                self.assertEqual(row['V_sha256'], self.word_hash)
                self.assertEqual(row['letters'], 2 * len(self.word) + overhead)
                self.assertIs(type(row['letters']), int)
            else:
                self.assertEqual(sum('slp' in s for s in program['segments']), 2)
                self.assertIs(type(row['letters']), str)
        # The complete record, including certificate/end-table names, is path-free.
        self.assertNotIn('/', json.dumps(row))
        self.assertNotIn('\\', json.dumps(row))

    def test_05_driver_literal_branches(self):
        for kind in ('closure', 't1prime', 'b06'):
            for limit in (10000, len(self.word) - 1, 2 * len(self.word) + 2, 0):
                with self.subTest(driver=kind, limit=limit):
                    self.driver(kind, limit)

    def test_06_stepchk_require_strict(self):
        table = TEMP / 'z-b21.json'
        write_json(table, A.read_json(REPLAY / 'run1/z-b21.json.gz'))
        empty = write_json(TEMP / 'empty.json', {'word': []})
        for name, strict in (('b04', False), ('b14', True)):
            unit = RUNS / f'units/eb5-k40-k00-{name}.json'
            for required in (False, True):
                report = TEMP / f'strict-{name}-{required}.json'
                args = [HERE / 'round22/stepchk.py', table, unit, empty, report]
                if required:
                    args.append('--require-strict')
                self.cli(args, expected=int(required and not strict))
                self.assertIs(A.read_json(report)['strict_prime'], strict)
        mismatch = write_json(TEMP / 'wrong-output.json', {'word': [], 'output_digest': '0' * 64})
        for required in (False, True):
            report = TEMP / f'wrong-output-{required}.json'
            args = [HERE / 'round22/stepchk.py', table, RUNS / 'units/eb5-k40-k00-b14.json', mismatch, report]
            if required:
                args.append('--require-strict')
            self.cli(args, expected=int(required))
            self.assertIs(A.read_json(report)['strict_prime'], True)
            self.assertIs(A.read_json(report)['output_digest_matches'], False)

    @unittest.skipUnless(os.environ.get('LITERAL_E2E') == '1', 'requires LITERAL_E2E=1 and LIT-A atlas seeds')
    def test_07_optional_atlas_b15_end_to_end(self):
        table = TEMP / 'e2e-start.json'
        write_json(table, A.read_json(REPLAY / 'run2/z-05-b19.json.gz'))
        plan = write_json(TEMP / 'e2e-plan.json', {'steps': [{'unit': 'b15',
                 'unit_file': str(RUNS / 'units/eb5-k40-k00-b15.json'),
                 't1prime': str(REPLAY / 'run2/09-b15-W.json'), 'greedy_seconds': 1}]})
        wd = TEMP / 'e2e'
        self.cli([HERE / 'round22/replay_slp.py', table, plan, wd,
                  '--seeds', 'atlas', '--literal-limit', '10000000'])
        row = RS.json_lines(wd / 'seq.jsonl')[0]
        self.assertEqual(row['how'], 't1prime')
        self.assertEqual(row['V'], 'literal')
        self.assertIs(row['strict_prime'], True)
        self.assertEqual((row['off_before'], row['off_after']), (96, 71))


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        A.print_row({'test': test.id().rsplit('.', 1)[-1], 'seconds': round(time.monotonic() - self.started, 3)})
        super().stopTest(test)


def main():
    before = status()
    try:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(Literal)
        result = unittest.TextTestRunner(verbosity=2, resultclass=TimedResult).run(suite)
    finally:
        shutil.rmtree(TEMP)
    unchanged = status() == before
    seconds = time.monotonic() - T0
    passed = result.wasSuccessful() and seconds < 1500 and unchanged
    A.print_row({'acceptance_seconds': round(seconds, 3), 'worktree_status_unchanged_including_ignored': unchanged,
                 'passed': passed})
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
