"""Temporary-directory, single-process tests for the literal lattice player.

python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/test_literal_lattice.py
Set LATTICE_SLOW=1 to include rebuilding the committed depth-20 handoff.
These are headless exact research checks; timings are cloud timings only.
"""

import gzip
import hashlib
import itertools
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
for _variable in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_variable] = '1'
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
J = ROOT / 'research/jumbling'
SCRIPTS = J / 'restoration/09-general-method/scripts/lattice/scripts'
CHECKER = SCRIPTS.parent / 'checker/verify_late.py'
sys.path[:0] = [str(HERE), str(J / 'theory'), str(J), str(J / 'fixtures')]

import numpy as np  # noqa: E402
import literal_lattice as P  # noqa: E402


def read(path):
    raw = Path(path).read_bytes()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def write(path, doc):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    raw = (json.dumps(doc) + '\n').encode()
    path.write_bytes(gzip.compress(raw) if path.suffix == '.gz' else raw)


# Git metadata and the private trees other processes write to while the suite runs.
SKIP = ('.git', 'work/codex-sandbox-home', 'work/loop-memory', 'work/reviews', 'work/worktrees')


def status():
    return subprocess.check_output(['git', 'status', '--porcelain', '--ignored', '--untracked-files=all',
                                    '--', '.'] + [':(exclude)' + p for p in SKIP[1:]],
                                   cwd=ROOT, env={**os.environ, 'GIT_OPTIONAL_LOCKS': '0'})


def snapshot():
    # Status alone cannot detect edits to an already modified/ignored file.
    # Include all file bytes, plus symlink targets without following links.
    result = {}
    for directory, dirs, files in os.walk(ROOT):
        here = Path(directory).relative_to(ROOT)
        dirs[:] = [d for d in dirs if (here / d).as_posix() not in SKIP]
        for name in dirs + files:
            path = Path(directory) / name
            try:
                if path.is_symlink():
                    result[path.relative_to(ROOT).as_posix()] = ('link', os.readlink(path))
                elif path.is_file():
                    with path.open('rb') as stream:
                        result[path.relative_to(ROOT).as_posix()] = hashlib.file_digest(stream, 'sha256').hexdigest()
            except FileNotFoundError:
                continue
    return result


def setUpModule():
    global BEFORE_STATUS, BEFORE_FILES
    BEFORE_STATUS, BEFORE_FILES = status(), snapshot()


def tearDownModule():
    if status() != BEFORE_STATUS or snapshot() != BEFORE_FILES:
        raise AssertionError('acceptance changed the worktree, including ignored files')


class ReportChecks:
    def report(self, path):
        doc = read(path)
        self.assertEqual(set(doc), {'format', 'restore_dir', 'run', 'input', 'order', 'scheduled', 'played',
                                    'junction', 'final', 'result', 'failure', 'position', 'seconds',
                                    'model_files_checked', 'scripts'})
        self.assertEqual(doc['format'], 'literal-lattice/1')
        self.assertIs(type(doc['run']), int)
        for name in (doc['restore_dir'], doc['input']['table']):
            self.assertEqual(Path(name).name, name)
        self.assertEqual(set(doc['input']), {'table', 'digest', 'labels_sha256'})
        self.assertEqual(set(doc['scheduled']), {'retained', 'corrections'})
        self.assertEqual(set(doc['played']), {'retained', 'corrections', 'total'})
        self.assertEqual(set(doc['junction']), {'T_digest', 'labels_equal'})
        self.assertEqual(set(doc['final']), {'digest', 'labels_identity'})
        self.assertEqual(set(doc['seconds']), {'retained', 'corrections', 'total'})
        for value in (*doc['scheduled'].values(), *doc['played'].values()):
            self.assertIs(type(value), int)
            self.assertGreaterEqual(value, 0)
        self.assertEqual(doc['played']['total'], doc['played']['retained'] + doc['played']['corrections'])
        for phase in ('retained', 'corrections'):
            self.assertLessEqual(doc['played'][phase], doc['scheduled'][phase])
        for value in doc['seconds'].values():
            self.assertIsInstance(value, (int, float))
            self.assertGreaterEqual(value, 0)
        self.assertGreaterEqual(doc['seconds']['total'], sum(doc['seconds'][k] for k in ('retained', 'corrections')))
        self.assertIn(doc['order'], ('retained-first', 'corrections-first'))
        self.assertIs(type(doc['model_files_checked']), bool)
        self.assertIs(type(doc['final']['labels_identity']), bool)
        self.assertIn(doc['junction']['labels_equal'], (None, False, True))
        for digest in (doc['input']['digest'], doc['input']['labels_sha256'],
                       doc['junction']['T_digest'], doc['final']['digest']):
            if digest is not None:
                self.assertRegex(digest, r'^[0-9a-f]{64}$')
        self.assertIn(HERE.relative_to(ROOT).as_posix() + '/literal_lattice.py', doc['scripts'])
        for path, digest in doc['scripts'].items():
            self.assertFalse(Path(path).is_absolute())
            self.assertNotIn('..', Path(path).parts)
            self.assertEqual(P.sha(ROOT / path), digest)
        # Check every string recursively, including failures and provenance.
        def strings(value):
            if isinstance(value, str):
                yield value
            elif isinstance(value, list):
                for item in value:
                    yield from strings(item)
            elif isinstance(value, dict):
                for key, item in value.items():
                    yield key
                    yield from strings(item)
        for value in strings(doc):
            self.assertFalse(value.startswith('/'), value)
            self.assertNotRegex(value, r'^[A-Za-z]:[\\/]')
        self.assertIn(doc['result'], ('solved', 'failed', 'budget'))
        if doc['result'] == 'failed':
            self.assertIsInstance(doc['failure'], str)
            self.assertTrue(doc['failure'])
            self.assertIsNone(doc['position'])
        else:
            self.assertIsNone(doc['failure'])
        if doc['result'] == 'budget':
            self.assertIsInstance(doc['position'], dict)
            self.assertEqual(doc['position']['twists_played'], doc['played']['total'])
            self.assertIn(doc['position']['phase'], ('retained', 'corrections', 'target', 'complete'))
            if doc['position']['phase'] in ('retained', 'complete'):
                for field in ('factor_index', 'twist_within_factor'):
                    self.assertIs(type(doc['position'][field]), int)
                    self.assertGreaterEqual(doc['position'][field], 0)
        else:
            self.assertIsNone(doc['position'])
        if doc['result'] == 'solved':
            self.assertTrue(doc['model_files_checked'])
            self.assertTrue(doc['final']['labels_identity'])
            self.assertIsNotNone(doc['final']['digest'])
            for phase in ('retained', 'corrections'):
                self.assertEqual(doc['played'][phase], doc['scheduled'][phase])
        return doc


def oracle(nodes, trees, size, move):
    """Independent net composition, following the checker's source/dest algebra.

    Nothing from Program.expand is used to calculate these expected nets.
    In particular conjugation maps the support through u, rather than expanding
    u^-1 x u, so the test does not merely mirror the player.
    """
    identity = np.arange(size)
    nets, lengths = [], []
    for node in nodes:
        op = node['op']
        if op in ('word', 'path'):
            if op == 'word':
                word = node['w']
            else:
                tree, index, word = trees[node['orbit']], node['node'], []
                while tree['parent'][index] >= 0:
                    word.insert(0, tree['move'][index])
                    index = tree['parent'][index]
            labels = identity.copy()
            for letter in word:
                src, dst = move(letter)
                labels[dst] = labels[src]
            net = np.argsort(labels)
            length = len(word)
        elif op == 'conj':
            x, u = nets[node['x']], nets[node['u']]
            u = u if node['us'] > 0 else np.argsort(u)
            net = identity.copy()
            net[u] = u[x]
            length = lengths[node['x']] + 2 * lengths[node['u']]
        else:
            net = identity.copy()
            for index, sign in node['f']:
                right = nets[index] if sign > 0 else np.argsort(nets[index])
                net = right[net]
            length = sum(lengths[index] for index, _ in node['f'])
        nets.append(net)
        lengths.append(length)
    return nets, lengths


class ExpansionTests(unittest.TestCase):
    def test_all_nodes_signs_and_lengths(self):
        size = 17
        permutations = {1: np.roll(np.arange(size), 1),
                        2: np.array([2, 0, 1] + list(range(3, size))),
                        3: np.arange(size)[::-1]}
        def move(letter):
            net = permutations[abs(letter)]
            return (np.arange(size), net) if letter > 0 else (net, np.arange(size))
        trees = [{'parent': [-1, 0, 1, 1], 'move': [0, 1, -2, 3]}]
        nodes = [{'op': 'word', 'w': [1, 2, -1]},
                 {'op': 'word', 'w': [2, 1, -3]},
                 {'op': 'path', 'orbit': 0, 'node': 2},
                 {'op': 'conj', 'x': 0, 'u': 1, 'us': 1},
                 {'op': 'conj', 'x': 0, 'u': 1, 'us': -1},
                 {'op': 'prod', 'f': [[3, 1], [4, -1], [2, -1], [1, 1]]},
                 {'op': 'conj', 'x': 4, 'u': 5, 'us': 1},
                 {'op': 'conj', 'x': 3, 'u': 5, 'us': -1},
                 {'op': 'prod', 'f': [[6, -1], [7, 1], [0, -1], [5, 1]]},
                 {'op': 'word', 'w': []}, {'op': 'prod', 'f': []}]
        nets, lengths = oracle(nodes, trees, size, move)
        program = P.Program(nodes, trees)
        self.assertEqual(program.lengths, lengths)
        for node, sign in itertools.product(range(len(nodes)), (1, -1)):
            with self.subTest(node=node, sign=sign):
                labels, count = np.arange(size), 0
                for letter in program.expand(node, sign):
                    src, dst = move(letter)
                    labels[dst] = labels[src]
                    count += 1
                self.assertEqual(count, lengths[node])
                expected = nets[node] if sign > 0 else np.argsort(nets[node])
                self.assertTrue(np.array_equal(labels, np.argsort(expected)))

    def test_lazy_deep_word_and_reject_forward_reference(self):
        nodes = [{'op': 'word', 'w': [1]}]
        for i in range(1, 1600):
            nodes.append({'op': 'prod', 'f': [[i - 1, 1], [i - 1, -1]]})
        program = P.Program(nodes, [])
        self.assertEqual(program.lengths[-1], 2 ** 1599)
        self.assertEqual(len(list(itertools.islice(program.expand(1599, -1), 20))), 20)
        with self.assertRaisesRegex(P.Failed, 'factor_node'):
            P.Program([{'op': 'prod', 'f': [[0, 1]]}], [])


class InputFailureTests(ReportChecks, unittest.TestCase):
    def test_truncated_gzip_cli_writes_failed_report(self):
        temporary = Path(tempfile.mkdtemp(prefix='literal-lattice-invalid-'))
        self.addCleanup(shutil.rmtree, temporary)
        directory = temporary / 'restore'
        write(directory / 'restore-summary.json', {'runs': [{'status': 'restored'}]})
        (directory / 'solutions.json.gz').write_bytes(gzip.compress(b'{}')[:-4])
        output = temporary / 'report.json'
        proc = subprocess.run([sys.executable, '-I', '-B', str(HERE / 'literal_lattice.py'),
                               str(directory), str(output), '--start-table', str(temporary / 'table.json')],
                              cwd=ROOT, capture_output=True, text=True, timeout=60, check=False)
        self.assertEqual(proc.returncode, 1, proc.stdout + proc.stderr)
        self.assertEqual(proc.stderr, '')
        doc = self.report(output)
        self.assertEqual(doc['result'], 'failed')
        self.assertEqual(doc['failure'], 'input_format')
        self.assertEqual(doc['played']['total'], 0)
        self.assertEqual(sorted(p.name for p in temporary.iterdir()), ['report.json', 'restore'])


class PlayerTests(ReportChecks, unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = Path(tempfile.mkdtemp(prefix='literal-lattice-tests-'))
        cls.addClassCleanup(shutil.rmtree, cls.tmp)
        cls.sim, loader, cls.ctx, cls.menu = P.simulator()
        cls.loader = staticmethod(loader)
        import buffer
        import restore
        cls.export = staticmethod(buffer.export_pose_table)
        cls.restore = restore
        cls.full_library = cls.tmp / 'library-merged.json'
        cls.full_library.write_bytes(gzip.decompress(P.DEFAULT_LIBRARY.read_bytes()))
        cls.lib = read(cls.full_library)
        # These two inputs need only w9's nonzero nu. Keep its committed exact
        # records, and let the restorer validate this small library itself.
        # Revalidating all 23,932 unrelated source-library twists dominates
        # fixture setup on this machine. This is not a library coverage claim.
        cls.lib = {**cls.lib, 'macros': cls.lib['macros'][:1], 'rank2': 1, 'rank5': 0}
        cls.library = cls.tmp / 'library-w9.json'
        write(cls.library, cls.lib)
        cls.fixtures = {}
        cls.build_fixture('plain')
        # Macro 0 (w9, nine menu twists) has nonzero nu, so it forces
        # corrections without needing a search or a long fixture word.
        cls.correction_macro = 0
        cls.build_fixture('corrected', macro=cls.correction_macro)
        # A small, genuinely restored seed state makes the mutations cheap;
        # the two positive cases still play their full three-letter schedules.
        base = cls.tmp / 'seed'
        base.mkdir()
        state = cls.sim.State(cls.ctx, menu=cls.menu)
        seed = read(ROOT / 'assets/execution_trees.json.gz')[23]['seed']
        for letter in seed:
            assert state.apply(cls.sim.primitive(cls.ctx, letter), journal=False).applied
        assert state.is_lattice()
        table = base / 'start-table.json'
        cls.export(state, table)
        cls.fixtures['seed'] = (base / 'restore', table)
        # Batch all inputs through one restorer, with the authentic w9 word.
        t0 = time.monotonic()
        combined = cls.tmp / 'restored'
        names = ('plain', 'corrected', 'seed')
        tables = [cls.fixtures[name][1] for name in names]
        cls.command([SCRIPTS / 'restore_lattice_late.py', SCRIPTS, ROOT, J, cls.library, combined,
                     *('table:' + str(table) for table in tables)], cls.tmp / 'restore.log')
        print('shared fixture restore: %.2fs' % (time.monotonic() - t0), flush=True)
        summary = read(combined / 'restore-summary.json')
        for index, name in enumerate(names):
            out, _ = cls.fixtures[name]
            shutil.copytree(combined, out)
            run = summary['runs'][index]
            assert run['status'] == 'restored'
            if name == 'corrected':
                assert run['order'] == 'retained-first' and run['corrections']
            else:
                assert not run['corrections']
            write(out / 'restore-summary.json', {**summary, 'runs': [run]})
            t0 = time.monotonic()
            cls.command([CHECKER, ROOT, J, cls.library, out, out.parent / 'verification.json'], out.parent / 'verify.log')
            assert read(out.parent / 'verification.json')['pass']
            print('fixture %s independent verify: %.2fs; retained %d; corrections %d' %
                  (name, time.monotonic() - t0, run['retained_length'], run.get('correction_twists', 0)), flush=True)
        print('corrections fixture: macro %d (%s)' %
              (cls.correction_macro, cls.lib['macros'][cls.correction_macro]['family']), flush=True)

    @classmethod
    def build_fixture(cls, name, macro=None):
        base = cls.tmp / name
        base.mkdir()
        state = cls.sim.State(cls.ctx, menu=cls.menu)
        if macro is not None:
            for record in cls.lib['macros'][macro]['records']:
                twist = cls.sim.Twist.from_record(cls.ctx, record)
                assert cls.menu.contains(twist) and state.apply(twist, journal=False).applied
            assert state.is_lattice()
        retained = [i for i in range(len(cls.menu.items)) if i not in cls.restore.jumble_items(cls.menu)
                    and cls.restore.element(cls.menu, 0, i) != cls.restore.I4]
        for cap, index in ((0, retained[0]), (17, retained[1]), (379, retained[0])):
            twist = cls.sim.Twist(cls.ctx, cap, cls.restore.element(cls.menu, cap, index),
                                  'menu', {'menu': cls.menu.name})
            assert state.apply(twist, journal=False).applied
        assert state.is_lattice()
        table, out = base / 'start-table.json', base / 'restore'
        cls.export(state, table)
        cls.fixtures[name] = (out, table)

    @staticmethod
    def command(arguments, log, timeout=1800):
        with Path(log).open('w') as stream:
            result = subprocess.run([sys.executable, '-I', '-B', *map(str, arguments)], cwd=ROOT,
                                    stdout=stream, stderr=subprocess.STDOUT, timeout=timeout, check=False)
        if result.returncode:
            raise AssertionError('fixture command failed: ' + Path(log).read_text()[-3000:])

    def case(self, fixture='plain'):
        base = self.tmp / self._testMethodName
        # subTest cases each get their own unmodified restorer output.
        base = Path(tempfile.mkdtemp(prefix=base.name + '-', dir=self.tmp))
        out, table = self.fixtures[fixture]
        shutil.copytree(out, base / 'restore')
        shutil.copyfile(table, base / 'start-table.json')
        return base / 'restore', base / 'start-table.json'

    def run_player(self, directory, table, *, cli=False, **kwargs):
        output = directory.parent / 'played.json'
        if cli:
            argv = [HERE / 'literal_lattice.py', directory, output, '--start-table', table]
            library = kwargs.pop('library', self.library)
            if library is not None:
                argv.extend(['--library', library])
            for key, value in kwargs.items():
                argv.extend(['--' + key.replace('_', '-'), value])
            self.command(argv, directory.parent / 'player.log')
        else:
            doc = P.play(directory, table, library=kwargs.pop('library', self.library), **kwargs)
            P.atomic_json(output, doc)
        return self.report(output)

    def positive(self, fixture, compressed=False):
        directory, table = self.case(fixture)
        if compressed:
            write(directory / 'solutions.json.gz', read(directory / 'solutions.json'))
            (directory / 'solutions.json').unlink()
        doc = self.run_player(directory, table, cli=True)
        self.assertEqual(doc['result'], 'solved', doc)
        self.assertEqual(doc['input']['digest'], read(table)['digest'])
        run = read(directory / 'restore-summary.json')['runs'][0]
        self.assertEqual(doc['played']['retained'], run['retained_length'])
        self.assertEqual(doc['played']['corrections'], run.get('correction_twists', 0))
        if fixture == 'corrected':
            self.assertTrue(doc['junction']['labels_equal'])
            self.assertEqual(doc['junction']['T_digest'], read(directory / 'restore-summary.json')['runs'][0]['T_digest'])
            # final.digest is now the played Y.R.C, not the stand-in T.C. Labels do not fix exact poses
            # (the plain fixture's played end has identity labels and another digest), so the played end
            # is checked by its labels; a simulator replay of 16 million twists is too slow for this suite.
            self.assertTrue(doc['final']['labels_identity'])
        seconds = doc['seconds']['retained']
        print('cloud timing of research code, %s: %d retained twists / %.3fs = %.1f twists/s' %
              (fixture, doc['played']['retained'], seconds, doc['played']['retained'] / seconds), flush=True)

    def test_plain_cli(self):
        self.positive('plain')

    def test_corrections_cli_compressed_solution(self):
        self.positive('corrected', compressed=True)

    def test_tampered_outputs(self):
        for corruption, expected in (
                ('word', 'final_labels_identity'), ('factor', 'final_labels_identity'),
                ('correction', 'correction_twists'), ('times', 'correction_twists'),
                ('input_digest', 'input_digest'), ('model', 'model_file_sha256')):
            with self.subTest(corruption=corruption):
                fixture = ('corrected' if corruption in ('correction', 'times') else
                           'seed' if corruption in ('word', 'factor') else 'plain')
                directory, table = self.case(fixture)
                path = directory / ('solutions.json' if corruption in ('word', 'factor', 'model') else 'restore-summary.json')
                doc = read(path)
                if corruption == 'word':
                    run = read(directory / 'restore-summary.json')['runs'][0]
                    tag = run['input'].replace(':', '_').replace('/', '_')[-60:]
                    used, todo = set(), [i for i, _ in doc['solutions'][tag]]
                    while todo:
                        index = todo.pop()
                        if index in used:
                            continue
                        used.add(index)
                        node = doc['nodes'][index]
                        if node['op'] == 'conj':
                            todo.extend((node['x'], node['u']))
                        elif node['op'] == 'prod':
                            todo.extend(i for i, _ in node['f'])
                    # Half turns are their own inverses. Negate a letter of a
                    # reachable word whose exact K+ rotation really changes.
                    kp = self.ctx.kplus
                    node, at = next((doc['nodes'][i], at)
                                    for i in sorted(used) if doc['nodes'][i]['op'] == 'word'
                                    for at, letter in enumerate(doc['nodes'][i]['w'])
                                    if kp.inv[kp.gen_idx[abs(letter) - 1]] != kp.gen_idx[abs(letter) - 1])
                    node['w'][at] *= -1
                elif corruption == 'factor':
                    run = read(directory / 'restore-summary.json')['runs'][0]
                    tag = run['input'].replace(':', '_').replace('/', '_')[-60:]
                    factors = doc['solutions'][tag]
                    factors[0][1] *= -1
                elif corruption == 'model':
                    doc['model_files']['assets/primitives.npz'] = '0' * 64
                elif corruption == 'correction':
                    doc['runs'][0]['corrections'].pop()
                elif corruption == 'times':
                    doc['runs'][0]['corrections'][0]['times'] += 1
                else:
                    doc['runs'][0]['input_digest'] = '0' * 64
                write(path, doc)
                report = self.run_player(directory, table)
                self.assertEqual(report['result'], 'failed', report)
                self.assertEqual(report['failure'], expected, report)

    def test_wrong_start_and_library(self):
        for corruption, expected in (('start', 'input_digest'), ('library', 'library_sha256'),
                                     ('labels', 'input_labels_sha256')):
            with self.subTest(corruption=corruption):
                directory, table = self.case()
                library = self.library
                if corruption == 'start':
                    shutil.copyfile(self.fixtures['corrected'][1], table)
                elif corruption == 'library':
                    library = directory.parent / 'changed-library.json'
                    library.write_bytes(self.library.read_bytes() + b'\n')
                else:
                    path = directory / 'restore-summary.json'
                    summary = read(path)
                    summary['runs'][0]['input_labels_sha256'] = '0' * 64
                    write(path, summary)
                report = self.run_player(directory, table, library=library)
                self.assertEqual(report['result'], 'failed', report)
                self.assertEqual(report['failure'], expected, report)

    def test_budget_inside_factor_and_exact_stop_digest(self):
        directory, table = self.case()
        # The clock is controlled only for deadline checks; actual play still
        # performs the first eleven primitive gathers and pose transports.
        original = P.LatticePlay.play
        ticks = [0]
        def played(lattice, letter):
            original(lattice, letter)
            ticks[0] += 1
        real_clock = time.monotonic
        origin = real_clock()
        def clock():
            return origin + (2 if ticks[0] >= 11 else 0)
        with patch.object(P.LatticePlay, 'play', played), patch.object(P.time, 'monotonic', clock):
            doc = self.run_player(directory, table, seconds=1)
        self.assertEqual(doc['result'], 'budget', doc)
        self.assertEqual(doc['played']['retained'], 11)
        self.assertEqual(doc['position']['phase'], 'retained')
        self.assertGreater(doc['position']['twist_within_factor'], 0)
        self.assertLess(doc['played']['retained'], doc['scheduled']['retained'])
        # Independently replay precisely the prefix on the exact simulator.
        state = self.sim.State(self.ctx, menu=self.menu)
        self.loader(state, read(table))
        solution = read(directory / 'solutions.json')
        program = P.Program(solution['nodes'], read(ROOT / 'assets/execution_trees.json.gz'))
        factors = next(iter(solution['solutions'].values()))
        prefix = itertools.chain.from_iterable(program.expand(node, sign) for node, sign in factors)
        for letter in itertools.islice(prefix, 11):
            self.assertTrue(state.apply(self.sim.primitive(self.ctx, letter), journal=False).applied)
        self.assertEqual(doc['final']['digest'], state.digest())

    def test_cli_tiny_budget(self):
        directory, table = self.case()
        doc = self.run_player(directory, table, cli=True, seconds=0)
        self.assertEqual(doc['result'], 'budget')
        self.assertEqual(doc['played']['total'], 0)
        self.assertEqual(doc['position']['factor_index'], 0)
        self.assertEqual(doc['position']['twist_within_factor'], 0)

    def test_cli_budget_during_retained(self):
        directory, table = self.case()
        # Preparing the primitive tables takes about two seconds on this
        # machine; five seconds reaches actual play but cannot finish it.
        doc = self.run_player(directory, table, cli=True, seconds=5)
        self.assertEqual(doc['result'], 'budget', doc)
        self.assertEqual(doc['position']['phase'], 'retained')
        self.assertGreater(doc['played']['retained'], 0)
        self.assertLess(doc['played']['retained'], doc['scheduled']['retained'])
        solution = read(directory / 'solutions.json')
        run = read(directory / 'restore-summary.json')['runs'][0]
        tag = run['input'].replace(':', '_').replace('/', '_')[-60:]
        program = P.Program(solution['nodes'], read(ROOT / 'assets/execution_trees.json.gz'))
        index = doc['position']['factor_index']
        self.assertEqual(sum(program.lengths[node] for node, _ in solution['solutions'][tag][:index]) +
                         doc['position']['twist_within_factor'], doc['played']['retained'])

    def test_pose_transport_matches_simulator(self):
        state = self.sim.State(self.ctx, menu=self.menu)
        player = P.LatticePlay(state)
        for letter in (1, -1, 35, 760, -35, -1200, 15, 17, -760):
            player.play(letter)
            self.assertTrue(state.apply(self.sim.primitive(self.ctx, letter), journal=False).applied)
            recovered = player.state()
            self.assertEqual(recovered.digest(), state.digest())
            self.assertTrue(np.array_equal(player.labels, state.lattice_stickers()['labels']))

    def test_gz_library_and_multiple_runs(self):
        directory, table = self.case()
        library = directory.parent / 'library.json.gz'
        library.write_bytes(P.DEFAULT_LIBRARY.read_bytes())
        path = directory / 'restore-summary.json'
        summary = read(path)
        summary['library']['sha256'] = P.sha(library)
        summary['runs'].insert(0, {'status': 'not restored'})
        write(path, summary)
        doc = self.run_player(directory, table, cli=True, run=1, library=library, seconds=0)
        self.assertEqual(doc['result'], 'budget', doc)
        self.assertTrue(doc['model_files_checked'])
        self.assertEqual(doc['run'], 1)

    def test_default_compressed_library(self):
        directory, table = self.case()
        # Bind this nu=0 run to the full library: no --library, summary hashes
        # plain JSON, default is its exact compressed archival copy. Stop after
        # validation to avoid replaying the same solution an extra time.
        path = directory / 'restore-summary.json'
        summary = read(path)
        summary['library']['sha256'] = P.sha(self.full_library)
        write(path, summary)
        doc = self.run_player(directory, table, cli=True, library=None, seconds=0)
        self.assertEqual(doc['result'], 'budget', doc)
        self.assertTrue(doc['model_files_checked'])

    def test_corrections_first(self):
        directory, table = self.case()
        state = self.sim.State(self.ctx, menu=self.menu)
        for record in self.lib['macros'][0]['records']:
            self.assertTrue(state.apply(self.sim.Twist.from_record(self.ctx, record), journal=False).applied)
        self.export(state, table)
        path = directory / 'restore-summary.json'
        summary = read(path)
        run = summary['runs'][0]
        run.update(input_digest=state.digest(), input_labels_sha256=P.labels_sha(state.lattice_stickers()['labels']),
                   corrections=[{'macro': 0, 'times': 1, 'inverse': True}], retained_length=0, factors=0,
                   correction_twists=9, post_correction_digest=self.sim.State(self.ctx, menu=self.menu).digest())
        run.pop('order', None)
        write(path, summary)
        path = directory / 'solutions.json'
        solution = read(path)
        tag = run['input'].replace(':', '_').replace('/', '_')[-60:]
        solution['solutions'][tag] = []
        write(path, solution)
        doc = self.run_player(directory, table)
        self.assertEqual(doc['result'], 'solved', doc)
        self.assertEqual(doc['order'], 'corrections-first')
        self.assertEqual(doc['played'], {'retained': 0, 'corrections': 9, 'total': 9})

    @unittest.skipUnless(os.environ.get('LATTICE_SLOW') == '1', 'set LATTICE_SLOW=1 for the committed depth-20 run')
    def test_slow_committed_depth20(self):
        work = Path(tempfile.mkdtemp(prefix='slow-', dir=self.tmp))
        self.command([HERE / 'runner.py', '--scramble', 20, 7106, '--out', work / 'run', '--cache', work / 'cache'],
                     work / 'runner.log', timeout=1800)
        directory = J / 'restoration/13-tutorial-runner/runs/depth20-seed7106/lattice'
        table = work / 'run/tables/bulk-end.json'
        # Output is in tmp even though the committed restoration is read-only.
        output = work / 'played.json'
        self.command([HERE / 'literal_lattice.py', directory, output, '--start-table', table,
                      '--library', self.full_library], work / 'player.log', timeout=1800)
        doc = self.report(output)
        self.assertEqual(doc['result'], 'solved', doc)
        self.assertEqual(doc['played']['retained'], 8701310)
        print('depth20 cloud research play: %.3fs, %.1f twists/s' %
              (doc['seconds']['retained'], 8701310 / doc['seconds']['retained']), flush=True)


class CommittedRecordTests(unittest.TestCase):
    def test_committed_depth20_solution_tag(self):
        directory = J / 'restoration/13-tutorial-runner/runs/depth20-seed7106/lattice'
        run = read(directory / 'restore-summary.json')['runs'][0]
        factors = P.solution_factors(read(directory / 'solutions.json.gz'), run['input'])
        self.assertEqual(len(factors), run['factors'])
        with self.assertRaisesRegex(P.Failed, 'solution_tag'):
            P.solution_factors({'solutions': {}}, run['input'])


class InvisibleRotationTests(unittest.TestCase):
    def test_retained_first_final_digest_is_the_played_state(self):
        """Y = T with one piece rotated invisibly to the labels; no retained factors, C = w9.

        The player's final.digest must be the digest of C played on Y, which is not the
        solved digest; the earlier stand-in T.C would report the solved digest.
        """
        from sim.kplus import matrix_json
        temporary = Path(tempfile.mkdtemp(prefix='literal-lattice-invisible-'))
        self.addCleanup(shutil.rmtree, temporary)
        sim, load_table, ctx, menu = P.simulator()
        kp, reg, data = ctx.kplus, ctx.regions, ctx.data
        full = read(P.DEFAULT_LIBRARY)
        lib = {**full, 'macros': full['macros'][:1], 'rank2': 1, 'rank5': 0}
        word = [sim.Twist.from_record(ctx, r) for r in lib['macros'][0]['records']]
        solved = sim.State(ctx, menu=menu).digest()
        target = sim.State(ctx, menu=menu)
        for twist in reversed(word):
            self.assertTrue(target.apply(twist.inverse(), journal=False).applied)
        labels = P.state_labels(target)
        counts = np.bincount(data.slot_piece, minlength=len(target.pose_id))
        start = None
        k = np.arange(len(kp.perms))
        for piece in np.flatnonzero((target.pose_id == 0) & (counts > 0))[::-1][:60]:
            piece = int(piece)
            destination = reg.piece_of[np.full(len(k), reg.orbit_of[piece]),
                                       kp.compose_vec(k, np.full(len(k), reg.transport[piece]))]
            for h in k[(destination == piece) & (k != 0)][:4]:
                state = sim.State(ctx, menu=menu)
                state.pose_id, state._poses, state._lookup = target.pose_id.copy(), list(target._poses), dict(target._lookup)
                state.pose_id[piece] = state._intern_k(int(h))
                state._compact()
                try:
                    if state.is_lattice() and np.array_equal(P.state_labels(state), labels):
                        start = state
                        break
                except P.Failed:
                    pass
            if start is not None:
                break
        self.assertIsNotNone(start, 'no rotation invisible to the labels found')
        self.assertNotEqual(start.digest(), target.digest())
        table = temporary / 'start.json'
        table.write_text(json.dumps({'poses': [matrix_json(start._pose_matrix(i)) for i in range(len(start._poses))],
                                     'pose_id': start.pose_id.tolist(), 'digest': start.digest(), 'journal': []}) + '\n')
        directory = temporary / 'restore'
        library = temporary / 'library-w9.json'
        write(library, lib)
        spec = 'table:' + str(table)
        write(directory / 'solutions.json', {'format': 'lattice-restoration-v1', 'conventions': '', 'nodes': [],
            'model_files': {name: P.sha(ROOT / name) for name in P.MODEL_FILES},
            'solutions': {spec.replace(':', '_').replace('/', '_')[-60:]: []}})
        write(directory / 'restore-summary.json', {'library': {'sha256': P.sha(library)}, 'runs': [{
            'input': spec, 'status': 'restored', 'factors': 0, 'retained_length': 0,
            'input_digest': start.digest(), 'input_labels_sha256': P.labels_sha(labels), 'order': 'retained-first',
            'T_digest': target.digest(), 'correction_twists': len(word),
            'corrections': [{'macro': 0, 'times': 1, 'inverse': False}]}]})
        doc = P.play(directory, table, library=library)
        self.assertEqual(doc['result'], 'solved', doc)
        self.assertTrue(doc['junction']['labels_equal'])
        replay = sim.State(ctx, menu=menu)
        load_table(replay, read(table))
        for twist in word:
            self.assertTrue(replay.apply(twist, journal=False).applied)
        self.assertEqual(doc['final']['digest'], replay.digest())
        self.assertNotEqual(doc['final']['digest'], solved)


class PoseTransportTests(unittest.TestCase):
    def test_deferred_leaf_poses_and_prefix_match_simulator(self):
        sim, _, ctx, menu = P.simulator()
        state = sim.State(ctx, menu=menu)
        lattice = P.LatticePlay(state)
        word = (1, 4, -38, 15, -1200, 4, 1)
        for sign, count in ((1, len(word)), (1, len(word)), (-1, len(word)), (-1, 3), (1, 0)):
            with self.subTest(sign=sign, count=count):
                lattice.defer_poses = True
                letters = iter(word) if sign > 0 else (-m for m in reversed(word))
                for letter in itertools.islice(letters, count):
                    lattice.play(letter)
                    self.assertTrue(state.apply(sim.primitive(ctx, letter), journal=False).applied)
                lattice.transport_word(word, sign, count)
                lattice.defer_poses = False
                self.assertEqual(lattice.state().digest(), state.digest())
                self.assertTrue(np.array_equal(lattice.labels, state.lattice_stickers()['labels']))


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        super().stopTest(test)
        self.stream.writeln('  %.2fs' % (time.monotonic() - self.started))


if __name__ == '__main__':
    unittest.main(verbosity=2, testRunner=unittest.TextTestRunner(verbosity=2, resultclass=TimedResult))
