"""Packet P3 acceptance: exact synthetic geometry, headless research only.

python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/test_lattice_phase.py
All generated data, caches and child output stay in a temporary directory.
LATTICE_PHASE_SLOW=1 additionally rebuilds the development seed 7106 handoff.
"""
import array
import functools
import gzip
import hashlib
import importlib.util
import itertools
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import types
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
for _variable in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_variable] = '1'
TEMPORARY = Path(tempfile.mkdtemp(prefix='lattice-phase-tests-'))
os.environ['JUMBLING_HOME_CACHE'] = str(TEMPORARY / 'cache/home_centroids.npy')
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
J = ROOT / 'research/jumbling'
sys.path.insert(0, str(HERE))

import lattice_phase as L  # noqa: E402
import runner_api as api  # noqa: E402
import literal_lattice as P  # noqa: E402
import test_literal_lattice as tll  # noqa: E402

_spec = importlib.util.spec_from_file_location('lattice_test_runner', HERE / 'runner.py')
R = importlib.util.module_from_spec(_spec)
sys.modules[_spec.name] = R
_spec.loader.exec_module(R)
SOLVED = hashlib.sha256(b'').hexdigest()
PLAY_CHILDREN = 0


def read(path):
    return P.read_json(path)


def write(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, allow_nan=False) + '\n')


def signed(word, sign):
    return list(word) if sign > 0 else [-m for m in reversed(word)]


def independent_expansions(nodes, trees):
    """Small fixture oracle; deliberately does not call Program's iterators."""
    words = []
    for nd in nodes:
        if nd['op'] == 'word':
            word = list(nd['w'])
        elif nd['op'] == 'path':
            tree, j, word = trees[nd['orbit']], nd['node'], []
            while tree['parent'][j] >= 0:
                word.insert(0, tree['move'][j])
                j = tree['parent'][j]
        elif nd['op'] == 'conj':
            u = signed(words[nd['u']], nd['us'])
            word = signed(u, -1) + words[nd['x']] + u
        else:
            word = list(itertools.chain.from_iterable(signed(words[j], sign) for j, sign in nd['f']))
        words.append(word)
    return words


@functools.cache
def shared():
    s = types.SimpleNamespace()
    started = time.monotonic()
    s.sim, s.load, s.ctx, s.menu = P.simulator()
    import buffer
    import restore
    s.export, s.restore = buffer.export_pose_table, restore
    s.mechanics = R.Mechanics(TEMPORARY / 'cache')
    s.trees = read(ROOT / 'assets/execution_trees.json.gz')
    s.model_files = {name: P.sha(ROOT / name) for name in P.MODEL_FILES}
    full = read(P.DEFAULT_LIBRARY)
    s.lib = {**full, 'macros': full['macros'][:1], 'rank2': 1, 'rank5': 0}
    s.library = TEMPORARY / 'library-w9.json'
    write(s.library, s.lib)
    # Build the oracle through restore.element, rather than the player's
    # conjugation-to-base correspondence. Only A4 entries can be primitives.
    retained = [i for i, (_, matrix) in enumerate(s.menu.items) if s.ctx.kplus.index_of_matrix(matrix) >= 0]
    by_grip = {}
    for grip in range(600):
        items = {restore.element(s.menu, grip, i): i for i in retained}
        assert len(items) == len(retained)
        by_grip[grip] = items
    s.primitive_codes = {}
    for m in itertools.chain(range(1, 1201), range(-1200, 0)):
        twist = s.sim.primitive(s.ctx, m)
        s.primitive_codes[m] = 60 * twist.grip + by_grip[twist.grip][twist.matrix]
    lookup = {restore.to_tuple(matrix): i for i, (_, matrix) in enumerate(s.menu.items)}
    s.inverse_items = [lookup[restore.to_tuple(restore.transpose(matrix))] for _, matrix in s.menu.items]
    s.correction_codes = {}
    s.fixtures, s.cached = {}, None
    print(f'Shared exact fixture setup: {time.monotonic() - started:.3f}s', flush=True)
    return s


def code_for_twist(twist):
    s = shared()
    key = twist.grip, twist.matrix
    if key not in s.correction_codes:
        matches = [i for i in range(len(s.menu.items))
                   if s.restore.element(s.menu, twist.grip, i) == twist.matrix]
        assert len(matches) == 1
        s.correction_codes[key] = 60 * twist.grip + matches[0]
    return s.correction_codes[key]


def correction_word(sequence):
    s, word = shared(), []
    for c in sequence:
        twists = [s.sim.Twist.from_record(s.ctx, r) for r in s.lib['macros'][c['macro']]['records']]
        if c['inverse']:
            twists = [t.inverse() for t in reversed(twists)]
        word.extend(twists * c['times'])
    return word


def synthetic_nodes():
    s = shared()
    tree = s.trees[23]
    near = next(i for i, parent in enumerate(tree['parent'])
                if parent >= 0 and tree['parent'][parent] >= 0
                and tree['parent'][tree['parent'][parent]] < 0)
    return [{'op': 'word', 'w': [2, 4, -38]},
            {'op': 'path', 'orbit': 23, 'node': near},
            {'op': 'word', 'w': [1, -2]},
            {'op': 'conj', 'x': 0, 'u': 1, 'us': 1},
            {'op': 'conj', 'x': 2, 'u': 1, 'us': -1},
            {'op': 'prod', 'f': [[3, 1], [4, -1], [0, 1], [0, 1]]},
            {'op': 'word', 'w': []}, {'op': 'prod', 'f': []},
            {'op': 'prod', 'f': [[5, 1], [6, 1], [7, -1]]}]


def fixture(name='plain', *, nodes=None, factors=None, order='corrections-first', sequence=None,
            cascade=None, counts=None, invisible=False):
    s = shared()
    if name in s.fixtures:
        return s.fixtures[name]
    base = Path(tempfile.mkdtemp(prefix=name + '-', dir=TEMPORARY))
    nodes = synthetic_nodes() if nodes is None else nodes
    factors = [[8, 1], [2, -1], [0, 1]] if factors is None else factors
    sequence = [] if sequence is None else sequence
    cascade = [23, 17, 34] if cascade is None else cascade
    counts = [1, 0, len(factors) - 1] if counts is None else counts
    words = independent_expansions(nodes, s.trees)
    expansion = list(itertools.chain.from_iterable(signed(words[j], sign) for j, sign in factors))
    twists = correction_word(sequence)
    target = s.sim.State(s.ctx, menu=s.menu)
    for twist in reversed(twists):
        assert target.apply(twist.inverse(), journal=False).applied
    state = s.sim.State(s.ctx, menu=s.menu)
    if order == 'retained-first':
        s.load(state, table_doc(target))
    for m in signed(expansion, -1):
        assert state.apply(s.sim.primitive(s.ctx, m), journal=False).applied
    post_correction_digest = state.digest()
    if order == 'corrections-first':
        for twist in reversed(twists):
            assert state.apply(twist.inverse(), journal=False).applied
    if invisible:
        state = add_invisible_rotation(state)
    table = base / 'start-table.json'
    s.export(state, table)
    assert state.is_lattice()
    spec = 'table:start-table.json'
    run = {'input': spec, 'status': 'restored', 'input_digest': state.digest(),
        'input_labels_sha256': P.labels_sha(P.state_labels(state)), 'factors': len(factors),
        'factors_per_orbit': dict(zip(map(str, cascade), counts)), 'retained_length': len(expansion),
        'corrections': sequence, 'correction_twists': len(twists)}
    if order == 'retained-first':
        run.update(order=order, T_digest=target.digest())
    elif sequence:
        run['post_correction_digest'] = post_correction_digest
    summary = {'cascade_order': cascade, 'library': {'sha256': P.sha(s.library)}, 'runs': [run]}
    solution = {'format': 'lattice-restoration-v1', 'model_files': s.model_files,
                'nodes': nodes, 'solutions': {spec.replace(':', '_'): factors}}
    template = base / 'template'
    write(template / 'restore-summary.json', summary)
    write(template / 'solutions.json', solution)
    result = types.SimpleNamespace(base=base, template=template, table=table, summary=summary,
        solution=solution, nodes=nodes, factors=factors, sequence=sequence, expansion=expansion,
        before={'digest': state.digest(), 'off': 0, 'heights': {}})
    result.stub = make_stub(template)
    s.fixtures[name] = result
    return result


def table_doc(state):
    from sim.kplus import matrix_json
    return {'poses': [matrix_json(state._pose_matrix(i)) for i in range(len(state._poses))],
            'pose_id': state.pose_id.tolist(), 'digest': state.digest(), 'journal': []}


def add_invisible_rotation(target):
    """The LLP w9 fixture's exact search for a pose invisible to sticker labels."""
    s = shared()
    np, kp, reg = s.mechanics.np, s.ctx.kplus, s.ctx.regions
    labels = P.state_labels(target)
    counts = np.bincount(s.ctx.data.slot_piece, minlength=len(target.pose_id))
    k = np.arange(len(kp.perms))
    for piece in np.flatnonzero((target.pose_id == 0) & (counts > 0))[::-1][:60]:
        piece = int(piece)
        destination = reg.piece_of[np.full(len(k), reg.orbit_of[piece]),
                                  kp.compose_vec(k, np.full(len(k), reg.transport[piece]))]
        for h in k[(destination == piece) & (k != 0)][:4]:
            state = s.sim.State(s.ctx, menu=s.menu)
            state.pose_id = target.pose_id.copy()
            state._poses, state._lookup = list(target._poses), dict(target._lookup)
            state.pose_id[piece] = state._intern_k(int(h))
            state._compact()
            try:
                if state.is_lattice() and np.array_equal(P.state_labels(state), labels):
                    assert state.digest() != target.digest()
                    return state
            except P.Failed:
                pass
    raise AssertionError('no rotation invisible to labels found')


def make_stub(template, mode='copy', status=None):
    directory = Path(tempfile.mkdtemp(prefix='restorer-stub-', dir=TEMPORARY))
    script = directory / 'restorer-stub.py'
    script.write_text('''import hashlib, json, shutil, sys
from pathlib import Path
MODE, TEMPLATE, STATUS = ''' + repr((mode, str(template), status)) + '''
if MODE == 'exit':
    raise SystemExit(1)
if MODE == 'missing':
    raise SystemExit(0)
library, out, spec = sys.argv[-3:]
out = Path(out)
shutil.copytree(TEMPLATE, out, dirs_exist_ok=True)
summary_path = out / 'restore-summary.json'
summary = json.loads(summary_path.read_text())
run = summary['runs'][0]
run['input'] = spec
summary['library']['sha256'] = hashlib.sha256(Path(library).read_bytes()).hexdigest()
if STATUS is not None:
    run['status'] = STATUS
summary_path.write_text(json.dumps(summary))
solution_path = out / 'solutions.json'
solution = json.loads(solution_path.read_text())
factors = next(iter(solution['solutions'].values()))
solution['solutions'] = {spec.replace(':', '_').replace('/', '_')[-60:]: factors}
solution_path.write_text(json.dumps(solution))
''')
    return script


def new_context(f, *, initialized=False, restorer='late', wrapper=None, mode='development'):
    s = shared()
    base = Path(tempfile.mkdtemp(prefix='context-', dir=TEMPORARY))
    r = R.Runner(R.parse_args(['--table', str(f.table), '--out', str(base / 'run'),
        '--cache', str(TEMPORARY / 'cache'), '--mode', mode, '--lattice-restorer', restorer]), s.mechanics)
    if initialized:
        r.initialize()
        ctx = r.context('lattice', None)
    else:
        work = Path(tempfile.mkdtemp(prefix='lattice-', dir=r.out / 'attempts'))
        ctx = api.Ctx(f.table, object(), lambda: dict(f.before), s.mechanics, work,
                      r.cache, r.execute, {'lattice_restorer': restorer}, mode=mode)
    ctx.budgets['lattice_restorer'] = restorer
    execute = r.execute

    def counted(argv, tag, seconds, **kwargs):
        global PLAY_CHILDREN
        stage = 'play' if '--_play' in argv else 'restore'
        assert tag == f'lattice-{stage}-' + ctx.work_dir.name
        assert seconds is None and kwargs['cwd'] == ctx.work_dir
        if stage == 'play':
            PLAY_CHILDREN += 1
        return execute(argv, tag, seconds, **kwargs)

    ctx.execute = counted if wrapper is None else wrapper(counted, ctx)
    return ctx, r


def invoke(f, *, wrapper=None, seconds=None, initialized=False, stub=None, mode='development', restorer='late'):
    ctx, r = new_context(f, initialized=initialized, restorer=restorer, wrapper=wrapper, mode=mode)
    original_state = r.state.digest() if initialized else ctx.state
    with patch.object(L, 'LIBRARY', shared().library), patch.dict(L.RESTORERS, {restorer: stub or f.stub}), \
            patch.object(L, 'PLAY_SECONDS', seconds):
        result = L.play(ctx)
    assert (r.state.digest() == original_state) if initialized else ctx.state is original_state
    return result, ctx, r


def read_codes(path):
    raw = gzip.decompress(Path(path).read_bytes())
    values = array.array('H', raw)
    if sys.byteorder != 'little':
        values.byteswap()
    return list(values)


def code_bytes(codes):
    values = array.array('H', codes)
    if sys.byteorder != 'little':
        values.byteswap()
    return values.tobytes()


def inverse_codes(codes):
    inverse = shared().inverse_items
    return [60 * (code // 60) + inverse[code % 60] for code in reversed(codes)]


def expected_words(f):
    s = shared()
    expanded = independent_expansions(f.nodes, s.trees)
    words = []
    for orbit, first, n in L.orbit_partition(f.summary):
        if n:
            word = itertools.chain.from_iterable(signed(expanded[j], sg) for j, sg in f.factors[first:first+n])
            words.append((f'lattice/orbit-{orbit}', [s.primitive_codes[m] for m in word]))
    corrections = [(f'lattice/correction-{i}-{c["macro"]}', [code_for_twist(t) for t in correction_word([c])])
                   for i, c in enumerate(f.sequence)]
    return words + corrections if f.summary['runs'][0].get('order') == 'retained-first' else corrections + words


def verify_tree(test, tree, codes, program=None):
    """Independent ranges and reference oracle, including each named DAG part."""
    seen, at, count = {}, 0, 0
    expanded = independent_expansions(program.nodes, shared().trees) if program is not None else None

    def visit(part, path, depth):
        nonlocal at, count
        count += 1
        test.assertLessEqual(depth, 64)
        test.assertFalse(set(part) - L.TREE_KEYS)
        test.assertFalse(L.private_path(part['name']))
        start = at
        if 'span' in part:
            test.assertEqual(part['span'], [at, at + part['letters']])
            at += part['letters']
        else:
            test.assertTrue(part['parts'])
            for i, child in enumerate(part['parts']):
                visit(child, path + (i,), depth + 1)
            test.assertEqual(at - start, part['letters'])
        for key in ('same_as', 'inverse_of'):
            if key in part:
                previous, lo, hi = seen[tuple(part[key])]
                test.assertLessEqual(hi, start)
                expected = codes[lo:hi]
                if key == 'inverse_of':
                    expected = inverse_codes(expected)
                else:
                    test.assertEqual(part['name'], previous['name'])
                test.assertEqual(codes[start:at], expected)
        if expanded is not None and part['name'].split(' ')[0] in ('word', 'path', 'conj', 'prod'):
            _, number = part['name'].split(' ')
            sign = -1 if number.endswith('⁻¹') else 1
            j = int(number.removesuffix('⁻¹'))
            test.assertEqual(codes[start:at], [shared().primitive_codes[m] for m in signed(expanded[j], sign)])
        seen[path] = part, start, at

    visit(tree, (), 1)
    test.assertEqual(at, len(codes))
    test.assertEqual(api.check_tree(tree, len(codes)), count)
    return count


class A_ModuleTests(unittest.TestCase):
    def test_01_module_contract_and_child_start(self):
        self.assertFalse(getattr(L, 'STUB', False))
        self.assertNotIn('LatticeStep', vars(L))
        self.assertNotIn('LatticeResult', vars(L))
        for path in L.SCRIPTS:
            self.assertFalse(Path(path).is_absolute())
            self.assertTrue((ROOT / path).is_file())
            self.assertRegex(P.sha(ROOT / path), '^[a-f0-9]{64}$')
        source = '''import sys, time
sys.path.insert(0, ''' + repr(str(HERE)) + ''')
import lattice_phase
assert not {'numpy', 'sim', 'literal_lattice'}.intersection(sys.modules)
t0 = time.monotonic()
import literal_lattice
literal_lattice.simulator()
print('Child simulator start (cloud research): %.3fs' % (time.monotonic() - t0), flush=True)
'''
        proc = subprocess.run([sys.executable, '-I', '-B', '-c', source], cwd=ROOT,
                              capture_output=True, text=True, timeout=120)
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        print(proc.stdout.strip(), flush=True)


class Capture:
    def __init__(self):
        self.events, self.codes = [], []

    def begin(self, program, factors, corrections, order):
        self.events.append(('begin', order))

    def leaf(self, node, sign, codes):
        self.events.append(('leaf', node, sign, len(codes)))
        self.codes.extend(codes)

    def factor_end(self, index, state):
        self.events.append(('factor_end', index))
        assert callable(state)

    def correction_twist(self, code):
        self.events.append(('correction_twist',))
        self.codes.append(code)

    def correction_end(self, index, state):
        self.events.append(('correction_end', index))
        assert state().is_lattice()

    def final(self, state):
        self.events.append(('final', state.digest()))


class B_ObserverTests(unittest.TestCase):
    def test_01_literal_codes_reports_and_both_orders(self):
        corrected = [{'macro': 0, 'times': 1, 'inverse': False}]
        first = [{'macro': 0, 'times': 2, 'inverse': True}, {'macro': 0, 'times': 1, 'inverse': False}]
        fixtures = [fixture(), fixture('corrected', order='retained-first', sequence=corrected),
                    fixture('corrections-first', sequence=first)]
        for f in fixtures:
            with self.subTest(f=f.base.name):
                capture = Capture()
                baseline = P.play(f.template, f.table, library=shared().library)
                with patch.object(P.LatticePlay, 'transport_word', side_effect=AssertionError('deferred observed play')):
                    report = P.play(f.template, f.table, library=shared().library, observer=capture)
                self.assertEqual(report['result'], 'solved', report)
                self.assertEqual({k: v for k, v in report.items() if k not in ('seconds', 'scripts')},
                                 {k: v for k, v in baseline.items() if k not in ('seconds', 'scripts')})
                expected = list(itertools.chain.from_iterable(word for _, word in expected_words(f)))
                self.assertEqual(capture.codes, expected)
                self.assertEqual(hashlib.sha256(code_bytes(capture.codes)).hexdigest(),
                                 hashlib.sha256(code_bytes(expected)).hexdigest())
                self.assertEqual(len(capture.codes), report['played']['total'])
                events = capture.events
                self.assertEqual(sum(e[0] == 'begin' for e in events), 1)
                self.assertEqual(sum(e[0] == 'final' for e in events), 1)
                self.assertEqual(events[0], ('begin', report['order']))
                self.assertEqual(events[-1], ('final', SOLVED))
                boundaries = [e for e in events if e[0] in ('factor_end', 'correction_end')]
                factors = [('factor_end', i) for i in range(len(f.factors))]
                corrections = [('correction_end', i) for i in range(len(f.sequence))]
                self.assertEqual(boundaries, factors + corrections if report['order'] == 'retained-first'
                                 else corrections + factors)
                if f is fixtures[0]:
                    seconds = report['seconds']['retained']
                    print(f'Observer synthetic play (cloud research): {len(f.expansion)} twists, '
                          f'{seconds:.6f}s, {len(f.expansion)/seconds:.1f} twists/s', flush=True)

    def test_02_budget_prefix_and_observer_exception(self):
        f, capture = fixture(), Capture()
        original, ticks = P.LatticePlay.play, [0]
        def played(lattice, letter):
            original(lattice, letter)
            ticks[0] += 1
        with patch.object(P.LatticePlay, 'play', played), \
                patch.object(P.time, 'monotonic', lambda: 2 if ticks[0] >= 5 else 0):
            report = P.play(f.template, f.table, library=shared().library, seconds=1, observer=capture)
        self.assertEqual(report['result'], 'budget', report)
        self.assertEqual(report['played']['retained'], 5)
        self.assertEqual(capture.codes, [shared().primitive_codes[m] for m in f.expansion[:5]])
        state = shared().mechanics.load(f.table)
        for m in f.expansion[:5]:
            self.assertTrue(state.apply(shared().sim.primitive(shared().ctx, m), journal=False).applied)
        self.assertEqual(report['final']['digest'], state.digest())
        capture = Capture()
        def failed(state):
            raise ValueError('observer failure')
        capture.final = failed
        report = P.play(f.template, f.table, library=shared().library, observer=capture)
        self.assertEqual(report['result'], 'failed')

    def test_03_all_signed_primitive_encodings(self):
        s = shared()
        table = P.ObserverLetters(s.sim, s.ctx, s.menu).primitives
        self.assertEqual(len(table), 2400)
        self.assertEqual(table, s.primitive_codes)
        for move, code in table.items():
            twist = s.sim.primitive(s.ctx, move)
            self.assertEqual(code // 60, twist.grip)
            self.assertEqual(s.restore.element(s.menu, code // 60, code % 60), twist.matrix)
            self.assertEqual(table[-move], 60 * (code // 60) + s.inverse_items[code % 60])


class C_TreeTests(unittest.TestCase):
    def tree(self, nodes, factors, **patches):
        program = P.Program(nodes, shared().trees)
        with patch.dict(vars(L), patches):
            tree, folded, depth = L.orbit_tree(program, factors, 23)
        word = list(itertools.chain.from_iterable(program.expand(j, s) for j, s in factors))
        codes = [shared().primitive_codes[m] for m in word]
        verify_tree(self, tree, codes, program)
        return tree, folded, depth

    def test_01_four_conjugations(self):
        for us, sign in itertools.product((1, -1), repeat=2):
            with self.subTest(us=us, sign=sign):
                nodes = [{'op': 'word', 'w': [1]}, {'op': 'word', 'w': [2]},
                         {'op': 'conj', 'x': 0, 'u': 1, 'us': us}]
                tree, folded, depth = self.tree(nodes, [[2, sign]])
                trailing = tree['parts'][0]['parts'][0]['parts'][2]
                self.assertEqual(trailing['inverse_of'], [0, 0, 0])
                self.assertEqual((folded, depth), (0, 0))

    def test_02_repetition_including_trailing_conjugator(self):
        nodes = [{'op': 'word', 'w': [1]}, {'op': 'word', 'w': [2]},
                 {'op': 'conj', 'x': 0, 'u': 1, 'us': 1}]
        tree, _, _ = self.tree(nodes, [[2, 1], [1, 1], [2, 1]])
        self.assertEqual(tree['parts'][1]['parts'][0]['same_as'], [0, 0, 2])
        self.assertEqual(tree['parts'][2]['parts'][0]['same_as'], [0, 0])

    def test_03_empty_depth_and_size_folding(self):
        tree, folded, depth = self.tree([{'op': 'word', 'w': []}, {'op': 'prod', 'f': []}], [[0, 1], [1, -1]])
        self.assertTrue(all('span' in p['parts'][0] for p in tree['parts']))
        self.assertEqual((folded, depth), (0, 0))
        nodes = [{'op': 'word', 'w': [2]}]
        for i in range(70):
            nodes.append({'op': 'prod', 'f': [[i, 1]]})
        tree, folded, depth = self.tree(nodes, [[70, 1]])
        self.assertEqual((folded, depth), (0, 1))
        self.assertEqual(max(d for _, _, d in api.walk(tree)), 64)
        nodes = [{'op': 'word', 'w': [2]}, {'op': 'prod', 'f': [[0, 1], [0, -1]]},
                 {'op': 'prod', 'f': [[1, 1], [1, -1], [0, 1]]}]
        tree, folded, depth = self.tree(nodes, [[2, 1]], TREE_PART_LIMIT=6)
        self.assertEqual(folded, 2)
        self.assertEqual(depth, 0)
        self.assertLessEqual(api.tree_size(tree), 6)
        tree, folded, depth = self.tree(nodes, [[2, 1], [2, -1]], TREE_PART_LIMIT=4)
        self.assertEqual(tree['name'], 'orbit 23')
        self.assertIn('span', tree)
        self.assertEqual((folded, depth), (1, 0))
        self.assertLessEqual(api.tree_size(tree), 4)

    def test_04_correction_repetitions(self):
        correction = {'macro': 0, 'times': 3, 'inverse': True}
        tree = L.correction_tree(0, correction, shared().lib['macros'][0])
        codes = [code_for_twist(t) for t in correction_word([correction])]
        verify_tree(self, tree, codes)
        self.assertEqual(tree['parts'][1]['same_as'], [0])
        self.assertEqual(tree['parts'][2]['same_as'], [0])


class ResultChecks:
    def check_result(self, result, f, *, expected_digest=SOLVED):
        self.assertIsInstance(result, api.LatticeResult, result)
        s, expected = shared(), expected_words(f)
        self.assertEqual(len(result.steps), len(expected))
        state = s.mechanics.load(f.table)
        before = dict(f.before)
        program = P.Program(f.nodes, s.trees)
        for step, (rule, codes) in zip(result.steps, expected):
            self.assertIsInstance(step, api.LatticeStep)
            self.assertEqual(step.rule, rule)
            self.assertEqual((step.contract, step.gate, step.standing, step.encoding),
                             ('lattice_literal', 'player', 'rule', 'ia16'))
            self.assertEqual(step.before, before)
            self.assertTrue(step.word_file.is_absolute() and step.tree_file.is_absolute())
            self.assertEqual(read_codes(step.word_file), codes)
            expected_sha = hashlib.sha256(code_bytes(codes)).hexdigest()
            self.assertEqual(step.word_sha256, expected_sha)
            self.assertEqual(api.ia16_digest(step.word_file), {'sha256': expected_sha, 'letters': len(codes)})
            tree, tree_sha = api.read_tree(step.tree_file)
            self.assertEqual(tree_sha, step.tree_sha256)
            self.assertEqual(verify_tree(self, tree, codes, program if '/orbit-' in rule else None), step.tree_size)
            for code in codes:
                twist = s.sim.Twist(s.ctx, code // 60, s.restore.element(s.menu, code // 60, code % 60),
                                    'menu', {'menu': s.menu.name})
                outcome = state.apply(twist, journal=False)
                self.assertTrue(outcome.applied and not outcome.uncertain)
            before = {'digest': state.digest(), 'off': 0, 'heights': {}}
            self.assertEqual(step.after, before)
            self.assertTrue(state.is_lattice())
            self.assertTrue(L.safe_record(step.fields))
            self.assertFalse(L.FIELD_FORBIDDEN.intersection(step.fields))
        self.assertEqual(before['digest'], expected_digest)
        self.assertEqual(read(result.end_table)['digest'], expected_digest)
        self.assertEqual(result.report['player']['final'], {'digest': expected_digest, 'labels_identity': True})
        self.assertTrue(L.safe_record(result.report))
        self.assertEqual(result.report['skipped_orbits'], [o for o, _, n in L.orbit_partition(f.summary) if n == 0])
        self.assertEqual(result.report['step_counts'], {'orbit': sum(n > 0 for _, _, n in L.orbit_partition(f.summary)),
                                                       'correction': len(f.sequence)})
        self.assertEqual(sum('labels_identity' in step.fields for step in result.steps), bool(result.steps))
        if result.steps:
            self.assertIs(result.steps[-1].fields['labels_identity'], True)
        self.assertTrue(result.end_table.is_absolute())
        baseline = P.play(f.template, f.table, library=s.library)
        self.assertEqual(baseline['final']['digest'], expected_digest)
        return state

    def check_runner(self, r, result):
        end = result.steps[-1].after['digest']
        letters, count = sum(s.letters for s in result.steps), len(result.steps)
        r.played_lattice(result)
        self.assertEqual(r.record['result'], 'solved')
        # The runner keeps the plugin's report next to the record (plan revision 3e, E2).
        report = {'file': 'lattice/report.json',
                  'sha256': hashlib.sha256((r.out / 'lattice/report.json').read_bytes()).hexdigest()}
        self.assertEqual(r.record['phases']['lattice'], {'result': 'solved', 'steps': count, 'twists': letters,
                                                         'report': report})
        self.assertEqual(json.loads((r.out / 'lattice/report.json').read_text()), result.report)
        self.assertEqual(r.record['totals']['lattice_twists'], letters)
        self.assertIsNone(r.record['lattice'])
        self.assertIsNone(r.record['config']['component'])
        self.assertEqual(len(r.record['steps']), count)
        self.assertTrue(all(row['encoding'] == 'ia16' for row in r.record['steps']))
        self.assertEqual(r.state.digest(), end)


class D_PluginTests(ResultChecks, unittest.TestCase):
    def test_01_synthetic_program(self):
        f = fixture()
        result, ctx, _ = invoke(f)
        self.check_result(result, f)
        self.assertEqual([step.rule for step in result.steps], ['lattice/orbit-23', 'lattice/orbit-34'])
        self.assertEqual(result.steps[0].fields, {'orbit': 23, 'factor_first': 0, 'factor_count': 1,
                                                'tree_folded': 0, 'tree_folded_depth': 0})
        cache = TEMPORARY / 'cached-success'
        shutil.copytree(ctx.work_dir, cache)
        shared().cached = cache
        player = result.report['player']
        startup = result.report['play_seconds'] - player['seconds']['retained'] - player['seconds']['corrections']
        print(f'First --_play child: {result.report["play_seconds"]:.3f}s total; '
              f'{startup:.3f}s startup, validation and finalization overhead (cloud research)', flush=True)

    def test_02_corrected_and_runner(self):
        # Acceptance mode takes the frozen restorer, schedule11 (plan revision 3e, E5). The stub stands in for it,
        # with the realignment data's hash that a schedule11 summary must carry.
        f = fixture('corrected', order='retained-first', sequence=[{'macro': 0, 'times': 1, 'inverse': False}])
        template = Path(tempfile.mkdtemp(prefix='schedule11-', dir=TEMPORARY)) / 'template'
        shutil.copytree(f.template, template)
        summary = read(template / 'restore-summary.json')
        summary['realign_sha256'] = L.sha(ROOT / L.REALIGN)
        write(template / 'restore-summary.json', summary)
        result, _, r = invoke(f, initialized=True, mode='acceptance', restorer='schedule11', stub=make_stub(template))
        self.assertEqual(result.report['restorer'], 'schedule11')
        self.check_result(result, f)
        self.assertEqual([step.rule for step in result.steps],
                         ['lattice/orbit-23', 'lattice/orbit-34', 'lattice/correction-0-0'])
        self.assertEqual(result.steps[-1].fields, {'correction': 0, 'macro': 0, 'family': 'w9',
            'times': 1, 'inverse': False, 'macro_letters': 9, 'labels_identity': True})
        self.check_runner(r, result)

    def test_03_zero_steps(self):
        f = fixture('zero', nodes=[], factors=[], cascade=[0], counts=[0])
        # The real player and parent both accept the missing partition at zero factors.
        f.summary['runs'][0].pop('factors_per_orbit', None)
        write(f.template / 'restore-summary.json', f.summary)
        result, _, _ = invoke(f)
        self.check_result(result, f)
        self.assertEqual(result.steps, [])
        self.assertEqual(result.report['player']['played']['total'], 0)
        # I0 deliberately refuses a zero-step result; do not pass it to the runner.


class E_ConjugationPluginTests(ResultChecks, unittest.TestCase):
    def test_four_cases_in_ia16_and_json_letters(self):
        for us, sign in itertools.product((1, -1), repeat=2):
            with self.subTest(us=us, sign=sign):
                nodes = [{'op': 'word', 'w': [1]}, {'op': 'word', 'w': [2]},
                         {'op': 'conj', 'x': 0, 'u': 1, 'us': us}]
                f = fixture(f'conj-{us}-{sign}', nodes=nodes, factors=[[2, sign]], cascade=[0], counts=[1])
                result, _, _ = invoke(f)
                self.check_result(result, f)
                step = result.steps[0]
                program = P.Program(nodes, shared().trees)
                expected = [shared().primitive_codes[m] for m in program.expand(2, sign)]
                json_letters = json.loads(json.dumps([[c // 60, c % 60] for c in expected]))
                codes = read_codes(step.word_file)
                self.assertEqual(codes, [api.ia16_code(g, i) for g, i in json_letters])
                tree, _ = api.read_tree(step.tree_file)
                verify_tree(self, tree, expected, program)
                self.assertEqual(tree['parts'][0]['parts'][0]['parts'][2]['inverse_of'], [0, 0, 0])


class F_InvisibleRotationTests(ResultChecks, unittest.TestCase):
    def test_w9_played_digest_and_runner_round_trip(self):
        f = fixture('invisible', nodes=[], factors=[], order='retained-first',
            sequence=[{'macro': 0, 'times': 1, 'inverse': False}], cascade=[0], counts=[0], invisible=True)
        f.summary['runs'][0].pop('factors_per_orbit', None)
        write(f.template / 'restore-summary.json', f.summary)
        state = shared().mechanics.load(f.table)
        for twist in correction_word(f.sequence):
            self.assertTrue(state.apply(twist, journal=False).applied)
        expected = state.digest()
        self.assertNotEqual(expected, SOLVED)
        result, _, r = invoke(f, initialized=True)
        self.check_result(result, f, expected_digest=expected)
        self.assertEqual(len(result.steps), 1)
        self.assertEqual(result.steps[0].rule, 'lattice/correction-0-0')
        self.assertEqual(result.steps[0].letters, 9)
        self.check_runner(r, result)


class EndChecks:
    def check_end(self, result, outcome, reason, check=None, timed_out=None):
        self.assertIsInstance(result, api.End, result)
        self.assertEqual((result.result, result.reason), (outcome, reason))
        if check is not None:
            self.assertEqual(result.details['check'], check)
        for attempt in result.attempts:
            self.assertIs(type(attempt['timed_out']), bool)
            self.assertEqual(set(attempt), {'rule', 'outcome', 'reason', 'timed_out', 'seconds'})
            self.assertTrue(L.safe_record(attempt))
            self.assertFalse({'file', 'word', 'unit', 'table', 'library', 'out'}.intersection(attempt))
        self.assertTrue(L.safe_record(result.details))
        if timed_out is not None:
            self.assertEqual(len(result.attempts), 1)
            self.assertIs(result.attempts[0]['timed_out'], timed_out)
        return result


class G_FailureTests(EndChecks, unittest.TestCase):
    def test_01_player_budget(self):
        result, ctx, _ = invoke(fixture(), seconds=0)
        self.check_end(result, 'budget', 'player_clock', timed_out=True)
        self.assertIsInstance(result.details['position'], dict)
        self.assertEqual(result.details['played']['total'], 0)
        self.assertFalse((ctx.work_dir / 'play/words').exists())

    def test_02_bad_word_and_model_hash(self):
        f = fixture()
        for field, failure in (('word', 'final_labels_identity'), ('model', 'model_file_sha256')):
            with self.subTest(field=field):
                template = Path(tempfile.mkdtemp(prefix='tampered-', dir=TEMPORARY))
                shutil.copytree(f.template, template, dirs_exist_ok=True)
                solution = read(template / 'solutions.json')
                if field == 'word':
                    self.assertEqual(solution['nodes'][0]['w'][0], 2)
                    solution['nodes'][0]['w'][0] = -2
                else:
                    solution['model_files'][P.MODEL_FILES[0]] = '0' * 64
                write(template / 'solutions.json', solution)
                result, ctx, _ = invoke(f, stub=make_stub(template))
                self.check_end(result, 'error', 'player_failed')
                self.assertEqual(result.details['failure'], failure)
                self.assertFalse((ctx.work_dir / 'play/words').exists())

    def test_03_restorer_output_checks(self):
        def mutate(check):
            def wrapper(execute, ctx):
                def call(argv, tag, seconds, **kwargs):
                    child = execute(argv, tag, seconds, **kwargs)
                    self.assertTrue(tag.startswith('lattice-restore-'))
                    path = ctx.work_dir / 'restore/restore-summary.json'
                    summary = read(path)
                    run = summary['runs'][0]
                    if check == 'input_digest':
                        run['input_digest'] = '0' * 64
                    elif check == 'runs':
                        summary['runs'].append(dict(run))
                    elif check == 'input':
                        run['input'] = 'table:another.json'
                    elif check == 'orbit_partition':
                        run['factors_per_orbit']['23'] += 1
                    else:
                        summary['library']['sha256'] = '0' * 64
                    write(path, summary)
                    return child
                return call
            return wrapper
        for check in ('input_digest', 'runs', 'input', 'orbit_partition', 'library_sha256'):
            with self.subTest(check=check):
                result, _, _ = invoke(fixture(), wrapper=mutate(check))
                self.check_end(result, 'error', 'restorer_output_invalid', check=check)

    def test_04_restorer_outcomes_and_entry(self):
        f = fixture()
        for mode, status, outcome, reason in (
            ('exit', None, 'error', 'restorer_exit'), ('missing', None, 'error', 'restorer_output_missing'),
            ('copy', 'failure certificate: nu outside the span of the correction library', 'stuck', 'restorer_no_solution'),
            ('copy', 'not a lattice configuration', 'error', 'restorer_status')):
            with self.subTest(mode=mode, status=status):
                result, _, _ = invoke(f, stub=make_stub(f.template, mode, status))
                self.check_end(result, outcome, reason, timed_out=False if outcome == 'stuck' else None)
                if status is not None:
                    self.assertEqual(result.details['status'], status)
        for check in ('not_on_lattice', 'table_digest', 'restorer_unknown'):
            ctx, _ = new_context(f)
            if check == 'not_on_lattice':
                ctx.snapshot = lambda: {**f.before, 'off': 1}
            elif check == 'table_digest':
                ctx.snapshot = lambda: {**f.before, 'digest': '0' * 64}
            else:
                ctx.budgets.clear()
            self.check_end(L.play(ctx), 'error', check)

    def test_07_failure_certificate_identity(self):
        f = fixture()
        status = 'failure certificate: nu outside the span of the correction library'
        stub = make_stub(f.template, status=status)
        for restorer in ('late', 'schedule11'):
            checks = (None, 'runs', 'input', 'input_digest', 'library_sha256')
            if restorer == 'schedule11':
                checks += ('realign_sha256',)
            for check in checks:
                def wrapper(execute, ctx):
                    ctx.budgets['lattice_restorer'] = restorer

                    def call(argv, tag, seconds, **kwargs):
                        self.assertTrue(tag.startswith('lattice-restore-'), 'a certificate must not start the player')
                        child = execute(argv, tag, seconds, **kwargs)
                        path = ctx.work_dir / 'restore/restore-summary.json'
                        summary = read(path)
                        summary['realign_sha256'] = L.sha(ROOT / L.REALIGN)
                        run = summary['runs'][0]
                        if check == 'runs':
                            summary['runs'].append(dict(run))
                        elif check == 'input':
                            run['input'] = 'table:another.json'
                        elif check == 'input_digest':
                            run['input_digest'] = '0' * 64
                        elif check == 'library_sha256':
                            summary['library']['sha256'] = '0' * 64
                        elif check == 'realign_sha256':
                            summary['realign_sha256'] = '0' * 64
                        write(path, summary)
                        return child
                    return call

                with self.subTest(restorer=restorer, check=check), \
                        patch.dict(L.RESTORERS, {'schedule11': stub}):
                    result, _, _ = invoke(f, stub=stub, wrapper=wrapper)
                    if check is None:
                        self.check_end(result, 'stuck', 'restorer_no_solution', timed_out=False)
                    else:
                        self.check_end(result, 'error', 'restorer_output_invalid', check=check)

    def test_08_main_cleans_temporary_tree_on_success_and_exception(self):
        module = sys.modules[__name__]
        for keep in ('0', '1'):
            for fail in (False, True):
                directory = Path(tempfile.mkdtemp(dir=TEMPORARY, prefix='cleanup-'))
                with self.subTest(keep=keep, fail=fail), \
                        patch.object(module, 'TEMPORARY', directory), \
                        patch.object(module, 'status', return_value=b''), \
                        patch.object(module, 'inventory', return_value={}), \
                        patch.dict(os.environ, KEEP_TEMP=keep), \
                        patch.object(unittest.TextTestRunner, 'run',
                                     side_effect=RuntimeError('fixture run failure') if fail else None,
                                     return_value=types.SimpleNamespace(wasSuccessful=lambda: True)):
                    if fail:
                        with self.assertRaisesRegex(RuntimeError, 'fixture run failure'):
                            main()
                    else:
                        self.assertEqual(main(), 0)
                    self.assertEqual(directory.exists(), keep == '1')
                shutil.rmtree(directory, ignore_errors=True)

    def test_05_child_outcomes(self):
        for stage in ('restore', 'play'):
            for kind, outcome, reason in (('timeout', 'timeout', 'lattice_wall_clock'),
                ('cancelled', 'error', 'cancelled'), ('exit', 'error', 'restorer_exit' if stage == 'restore' else 'player_exit')):
                with self.subTest(stage=stage, kind=kind):
                    def wrapper(execute, ctx):
                        def call(argv, tag, seconds, **kwargs):
                            if tag == f'lattice-{stage}-' + ctx.work_dir.name:
                                self.assertIsNone(seconds)
                                self.assertEqual(kwargs['cwd'], ctx.work_dir)
                                return R.Child(1, 0.125, ctx.work_dir / 'child.log',
                                               timeout=kind == 'timeout', cancelled=kind == 'cancelled')
                            return execute(argv, tag, seconds, **kwargs)
                        return call
                    result, _, _ = invoke(fixture(), wrapper=wrapper)
                    # A timed-out or cancelled call keeps its attempt row and its time (I1R-04).
                    self.check_end(result, outcome, reason, timed_out={'timeout': True, 'cancelled': False}.get(kind))
                    if kind != 'exit':
                        self.assertEqual((result.attempts[0]['outcome'], result.attempts[0]['reason']),
                                         (outcome, reason))
                        self.assertGreaterEqual(result.attempts[0]['seconds'], 0.125)

    def test_06_play_output_tampering(self):
        s, f = shared(), fixture()
        if s.cached is None:
            result, ctx, _ = invoke(f)
            self.assertIsInstance(result, api.LatticeResult)
            s.cached = TEMPORARY / 'cached-success'
            shutil.copytree(ctx.work_dir, s.cached)
        for mutation, check in (('word', 'word_sha256'), ('word-gzip', 'word_sha256'),
                                ('tree', 'tree'), ('tree-format', 'tree'), ('end', 'end_table'),
                                ('json', 'manifest'), ('format', 'manifest'), ('path', 'manifest'),
                                ('report', 'manifest'), ('seconds', 'manifest'), ('fields', 'fields')):
            with self.subTest(mutation=mutation):
                def wrapper(execute, ctx):
                    def call(argv, tag, seconds, **kwargs):
                        self.assertIsNone(seconds)
                        self.assertEqual(kwargs['cwd'], ctx.work_dir)
                        if tag == 'lattice-restore-' + ctx.work_dir.name:
                            shutil.copytree(s.cached / 'restore', ctx.work_dir / 'restore', dirs_exist_ok=True)
                        else:
                            self.assertEqual(tag, 'lattice-play-' + ctx.work_dir.name)
                            output = ctx.work_dir / 'play'
                            shutil.copytree(s.cached / 'play', output, dirs_exist_ok=True)
                            path = output / 'play-result.json'
                            manifest = read(path)
                            if mutation == 'word':
                                word = output / manifest['steps'][0]['word_file']
                                codes = read_codes(word)
                                codes[0] = (codes[0] + 1) % 36000
                                word.write_bytes(gzip.compress(code_bytes(codes), mtime=0))
                            elif mutation == 'word-gzip':
                                word = output / manifest['steps'][0]['word_file']
                                raw = bytearray(word.read_bytes())
                                raw[10] ^= 0xff
                                word.write_bytes(raw)
                            elif mutation == 'tree':
                                tree_path = output / manifest['steps'][0]['tree_file']
                                tree, _ = api.read_tree(tree_path)
                                leaf = next(part for part, _, _ in api.walk(tree) if 'span' in part)
                                leaf['span'][1] += 1
                                tree_path.write_bytes(gzip.compress(json.dumps(tree).encode(), mtime=0))
                            elif mutation == 'tree-format':
                                tree_path = output / manifest['steps'][0]['tree_file']
                                raw = b'[]\n'
                                tree_path.write_bytes(gzip.compress(raw, mtime=0))
                                manifest['steps'][0]['tree_sha256'] = hashlib.sha256(raw).hexdigest()
                                write(path, manifest)
                            elif mutation == 'end':
                                end = read(output / manifest['end_table'])
                                end['digest'] = '0' * 64
                                write(output / manifest['end_table'], end)
                            elif mutation == 'json':
                                path.write_text('{')
                            elif mutation == 'format':
                                manifest['format'] = 'lattice-phase-play/0'
                                write(path, manifest)
                            elif mutation == 'report':
                                del manifest['report']['input']
                                write(path, manifest)
                            elif mutation == 'seconds':
                                manifest['steps'][0]['seconds'] = True
                                write(path, manifest)
                            elif mutation == 'fields':
                                manifest['steps'][0]['fields']['parts'] = []
                                write(path, manifest)
                            else:
                                manifest['steps'][0]['word_file'] = '../escape.gz'
                                write(path, manifest)
                        return R.Child(0, 0.01, ctx.work_dir / 'child.log')
                    return call
                result, ctx, _ = invoke(f, wrapper=wrapper)
                self.check_end(result, 'error', 'play_output_invalid', check=check)
                self.assertFalse((ctx.work_dir / 'play/words').exists())


class H_RealLateRestorerTests(EndChecks, unittest.TestCase):
    def test_real_late_restorer_budget(self):
        s = shared()
        state = s.sim.State(s.ctx, menu=s.menu)
        retained = [i for i in range(len(s.menu.items)) if i not in s.restore.jumble_items(s.menu)
                    and s.restore.element(s.menu, 0, i) != s.restore.I4]
        for cap, item in ((0, retained[0]), (17, retained[1]), (379, retained[0])):
            twist = s.sim.Twist(s.ctx, cap, s.restore.element(s.menu, cap, item), 'menu', {'menu': s.menu.name})
            self.assertTrue(state.apply(twist, journal=False).applied)
        base = Path(tempfile.mkdtemp(prefix='real-late-', dir=TEMPORARY))
        table = base / 'start-table.json'
        s.export(state, table)
        f = types.SimpleNamespace(table=table, before={'digest': state.digest(), 'off': 0, 'heights': {}})
        ctx, _ = new_context(f)
        times = []
        execute = ctx.execute
        def timed(argv, tag, seconds, **kwargs):
            child = execute(argv, tag, seconds, **kwargs)
            if tag.startswith('lattice-restore-'):
                times.append(child.seconds)
                print(f'Real late restorer (cloud research): {child.seconds:.3f}s', flush=True)
            return child
        ctx.execute = timed
        with patch.object(L, 'LIBRARY', s.library), patch.object(L, 'PLAY_SECONDS', 0):
            result = L.play(ctx)
        self.check_end(result, 'budget', 'player_clock', timed_out=True)
        summary = read(ctx.work_dir / 'restore/restore-summary.json')
        self.assertEqual(len(summary['runs']), 1)
        run = summary['runs'][0]
        self.assertEqual(run['status'], 'restored')
        self.assertEqual(run['input'], 'table:start-table.json')
        self.assertEqual(run['input_digest'], state.digest())
        self.assertEqual(list(run['factors_per_orbit']), list(map(str, summary['cascade_order'])))
        self.assertEqual(sum(run['factors_per_orbit'].values()), run['factors'])
        self.assertEqual(summary['library']['sha256'], P.sha(ctx.work_dir / 'library-merged.json'))
        self.assertEqual(run['retained_length'], 16184022)
        self.assertEqual(len(times), 1)



class I_ArgvTests(unittest.TestCase):
    def test_both_restorer_commands(self):
        f = fixture()
        for name in ('late', 'schedule11'):
            with self.subTest(restorer=name):
                calls = []
                def wrapper(execute, ctx):
                    def call(argv, tag, seconds, **kwargs):
                        calls.append((argv, tag, seconds, kwargs))
                        return R.Child(1, 0.0, ctx.work_dir / 'child.log')
                    return call
                ctx, _ = new_context(f, restorer=name, wrapper=wrapper)
                with patch.object(L, 'LIBRARY', shared().library):
                    result = L.play(ctx)
                self.assertEqual((result.result, result.reason), ('error', 'restorer_exit'))
                self.assertEqual(len(calls), 1)
                argv, tag, seconds, kwargs = calls[0]
                script = ROOT / L.RESTORERS[name]
                prefix = [sys.executable, '-I', '-B', script]
                prefix += [script.parent, ROOT, J] if name == 'late' else [ROOT, J, ROOT / L.REALIGN]
                self.assertEqual(argv, prefix + ['library-merged.json', 'restore', 'table:start-table.json'])
                self.assertEqual(tag, 'lattice-restore-' + ctx.work_dir.name)
                self.assertIsNone(seconds)
                self.assertEqual(kwargs, {'cwd': ctx.work_dir})


class J_LiteralSubsetTests(unittest.TestCase):
    def test_explicit_fast_subset(self):
        names = ['ExpansionTests.test_all_nodes_signs_and_lengths',
            'ExpansionTests.test_lazy_deep_word_and_reject_forward_reference',
            'PoseTransportTests.test_deferred_leaf_poses_and_prefix_match_simulator',
            'InvisibleRotationTests.test_retained_first_final_digest_is_the_played_state']
        if hasattr(tll, 'ObserverTests'):
            names.append('ObserverTests')
        loader = unittest.TestLoader()
        suite = unittest.TestSuite(loader.loadTestsFromName(name, tll) for name in names)
        def tests(suite):
            for test in suite:
                if isinstance(test, unittest.TestSuite):
                    yield from tests(test)
                else:
                    yield test
        self.assertTrue(all('PlayerTests' not in test.id() for test in tests(suite)))
        result = unittest.TextTestRunner(verbosity=2, resultclass=TimedResult).run(suite)
        self.assertTrue(result.wasSuccessful())


class K_SlowTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('LATTICE_PHASE_SLOW') == '1', 'set LATTICE_PHASE_SLOW=1 for schedule11')
    def test_schedule11_development_handoff(self):
        s = shared()
        base = Path(tempfile.mkdtemp(prefix='schedule11-', dir=TEMPORARY))
        out = base / 'handoff'
        proc = subprocess.run([sys.executable, '-I', '-B', str(HERE / 'runner.py'), '--scramble', '20', '7106',
            '--no-lattice', '--out', str(out), '--cache', str(base / 'cache')], cwd=ROOT,
            stdout=(base / 'handoff.log').open('w'), stderr=subprocess.STDOUT)
        self.assertEqual(proc.returncode, 0)
        record = read(out / 'run.json')
        table = next((out / row['table'] for row in reversed(record['steps']) if 'table' in row),
                     out / 'tables/bulk-end.json')
        state = s.mechanics.load(table)
        self.assertTrue(state.is_lattice(), f'{record["result"]}: {record["phases"]}')
        old = '1b263d3e9e0b9dcd2feece889acf0d20c8c96492e2a5e895034be3dd9c46578e'
        print(f'Rebuilt handoff {state.digest()}; committed handoff {old}', flush=True)
        f = types.SimpleNamespace(table=table, before={'digest': state.digest(), 'off': 0, 'heights': {}})
        ctx, _ = new_context(f, restorer='schedule11')
        result = L.play(ctx)
        self.assertIsInstance(result, api.LatticeResult, result)
        self.assertLessEqual(result.report['player']['played']['retained'], 20866844)
        self.assertTrue(result.report['player']['final']['labels_identity'])
        print(f'Schedule11: {len(result.steps)} steps; {sum(s.letters for s in result.steps)} twists; '
              f'restore {result.report["restore_seconds"]:.3f}s; play {result.report["play_seconds"]:.3f}s', flush=True)


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        super().stopTest(test)
        self.stream.writeln(f'  {test.id()}: {time.monotonic() - self.started:.3f}s')


SKIP = ('.git', 'work/codex-sandbox-home', 'work/loop-memory', 'work/reviews', 'work/worktrees')


def inventory():
    result = {}
    for directory, dirs, files in os.walk(ROOT):
        here = Path(directory).relative_to(ROOT)
        dirs[:] = [d for d in dirs if (here / d).as_posix() not in SKIP]
        for name in dirs + files:
            path = Path(directory) / name
            if path == ROOT / '.git':
                continue
            try:
                if path.is_symlink():
                    result[path.relative_to(ROOT).as_posix()] = ('link', os.readlink(path))
                elif path.is_file():
                    result[path.relative_to(ROOT).as_posix()] = P.sha(path)
            except FileNotFoundError:
                continue
    return result


def status():
    return subprocess.check_output(['git', '--no-optional-locks', 'status', '--porcelain', '--ignored',
        '--untracked-files=all', '--', '.'] + [':(exclude)' + p for p in SKIP[1:]], cwd=ROOT)


def main(names=()):
    try:
        started = time.monotonic()
        before_status, before_files = status(), inventory()
        loader = unittest.TestLoader()
        suite = loader.loadTestsFromNames(names, sys.modules[__name__]) if names else loader.loadTestsFromModule(sys.modules[__name__])
        try:
            result = unittest.TextTestRunner(verbosity=2, resultclass=TimedResult).run(suite)
        finally:
            after_status, after_files = status(), inventory()
        unchanged = before_status == after_status and before_files == after_files
        if unchanged:
            print('Worktree unchanged, including ignored files.', file=sys.stderr)
        else:
            changed = sorted(p for p in before_files.keys() | after_files.keys() if before_files.get(p) != after_files.get(p))
            print('FAIL: acceptance changed the worktree, including ignored files:', changed, file=sys.stderr)
        print(f'--_play children: {PLAY_CHILDREN}; test E kept in children; test H kept in default suite.', file=sys.stderr)
        print(f'Acceptance elapsed: {time.monotonic() - started:.3f}s.', file=sys.stderr)
        return 0 if result.wasSuccessful() and unchanged else 1
    finally:
        if os.environ.get('KEEP_TEMP') != '1':
            shutil.rmtree(TEMPORARY, ignore_errors=True)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
