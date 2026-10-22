"""Single-process, temporary-directory acceptance tests for the run formats 1, 2 and 3.

Run from the repository root with python3 -I -B. Fixtures are built independently
of runner.py, which need not exist; RunnerRecordTests also checks format 3 records
that runner.py writes, and is skipped without it. No output, cache or table belongs
in the repo.
"""

import ast
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


# Format 3 (plan runner-run/3, sections 0 and 1). The fixtures spell out the clocks,
# the orders and the entry tests themselves, independently of the checker's constants.
SUBPHASES = ('r22d75', 'r22d81', 'h4')
CLOCKS = ('bulk_step_seconds', 'bulk_seconds', 'rule_seconds', 'greedy_seconds', 'cert_seconds',
          'height_seconds', 'height_cpu', 'kernel_seconds')
# From solved to Z36, heights {2: 5799, 6: 1} (12-low-height/scripts/round22/test_r22d25.py:39).
WORD36 = [[0, 12], [74, 8], [188, 4], [74, 4], [188, 8], [188, 4], [188, 4],
          [74, 8], [188, 8], [74, 4], [188, 8], [178, 1], [188, 4], [74, 8],
          [188, 4], [74, 4], [188, 8], [188, 8], [188, 4], [74, 8], [188, 8],
          [74, 4], [178, 1], [0, 45], [14, 12], [12, 12]]
# The tail fixture's t1prime step: W, then the closure V·middle·V⁻¹, where middle is W
# inverted and V⁻¹ is V inverted (test_v2_middle, test_v2_V_inverse).
CLOSURE_TREE = {'name': 't1prime', 'letters': 1254, 'parts': [
    {'name': 'W', 'letters': 288, 'span': [0, 288]},
    {'name': 'closure', 'letters': 966, 'parts': [
        {'name': 'V', 'letters': 339, 'span': [288, 627]},
        {'name': 'middle', 'letters': 288, 'span': [627, 915], 'inverse_of': [0]},
        {'name': 'V⁻¹', 'letters': 339, 'span': [915, 1254], 'inverse_of': [1, 0]}]}]}


def snap(doc):
    return {key: doc[key] for key in ('digest', 'off', 'heights')}


def holds(name, doc):
    """An entry test of section 0, from a fixture's own snapshot document."""
    hist = {int(h): n for h, n in doc['heights'].items()}
    top, n4, n6 = max(hist, default=0), hist.get(4, 0), hist.get(6, 0)
    return {'r22d75': top <= 6 and n4 + n6 > 0, 'r22d81': top <= 6 and n6 > 0,
            'h4': top <= 4 and n4 > 0, 'high': top <= 6 and n4 + n6 > 0}[name]


def inverse_items(menu):
    lookup = {restore.to_tuple(m): i for i, (_, m) in enumerate(menu.items)}
    return [lookup[restore.to_tuple(restore.transpose(m))] for _, m in menu.items]


def tree_parts(root):
    return 1 + sum(tree_parts(p) for p in root.get('parts', ()))


def write_tree(base, index, root, size=None):
    data = (json.dumps(root, separators=(',', ':')) + '\n').encode('utf-8')
    name = 'step-trees/%05d.json.gz' % index
    (base / name).parent.mkdir(parents=True, exist_ok=True)
    (base / name).write_bytes(gzip.compress(data, mtime=0))
    return {'file': name, 'sha256': hashlib.sha256(data).hexdigest(),
            'size': tree_parts(root) if size is None else size}


def write_report(base, restorer, start, end, total, labels=True, edit=None):
    """The parts of the lattice plugin's report that the checker reads (plan revision 3e, E2; I1R-01), as
    the record's {file, sha256}. ``edit`` changes the document before it is written."""
    doc = {'restorer': restorer,
           'player': {'input': {'digest': start}, 'scheduled': {'retained': total, 'corrections': 0},
                      'played': {'retained': total, 'corrections': 0, 'total': total},
                      'final': {'digest': end, 'labels_identity': labels}, 'result': 'solved', 'failure': None}}
    if edit is not None:
        edit(doc)
    write(base / 'lattice/report.json', doc)
    return {'file': 'lattice/report.json',
            'sha256': hashlib.sha256((base / 'lattice/report.json').read_bytes()).hexdigest()}


def write_ia16(base, index, word):
    data = np.array([60 * c + i for c, i in word], '<u2').tobytes()
    name = 'words/%05d.ia16.gz' % index
    (base / name).parent.mkdir(parents=True, exist_ok=True)
    (base / name).write_bytes(gzip.compress(data, mtime=0))
    return name, hashlib.sha256(data).hexdigest()


# The committed acceptance options and the kernel's D19 rule, as the runner records them (plan revision 3e).
ACCEPTANCE_CONFIG = HERE.parent / 'acceptance-config.json'
D19 = 'research/jumbling/restoration/12-low-height/scripts/round22/r22d19_rule.py'


def acceptance_records():
    raw = ACCEPTANCE_CONFIG.read_bytes()
    frozen = {'file': ACCEPTANCE_CONFIG.relative_to(REPO).as_posix(), 'sha256': hashlib.sha256(raw).hexdigest(),
              'options': json.loads(raw)['options']}
    return frozen, {'file': D19, 'sha256': hashlib.sha256((REPO / D19).read_bytes()).hexdigest()}


def blank3(start, mode='development'):
    acceptance = mode == 'acceptance'
    frozen, d19 = acceptance_records() if acceptance else (None, None)
    budgets = {name: None if acceptance else 3600 for name in CLOCKS}
    budgets.update(bulk_steps=None, tail_steps=None, high_steps=None, pair_shards=4,
                   fallback_max_length=1, par=1)
    return {'format': 'runner-run/3', 'menu': 'I_a', 'start': start,
            'config': {'literal_only': True, 'mode': mode, 'component': None,
                       'bulk_order': ['seam', 'cover', 'close', 'conj', 'lock', 'improve', 'shift'],
                       'phase_order': ['bulk', 'height', 'high', 'tail', 'lattice'],
                       'subphase_order': {'height': list(SUBPHASES)},
                       'tail_rules': ['zero_bad', 'pure', 'close', 't1prime'], 'budgets': budgets,
                       'clock_sentinel': 1e9 if acceptance else None,
                       'kernel': {'seconds': None if acceptance else 3600, 'wall_margin': 600, 'd19': d19,
                                  'max_steps': 1},
                       'lattice': {'restorer': frozen['options']['lattice_restorer'] if acceptance else 'late'},
                       'literal_limit': 10000000, 'seeds': 'atlas', 'acceptance_config': frozen,
                       'source': {'commit': None, 'status_sha256': None, 'dirty': None}, 'scripts': {}},
            'steps': [], 'phases': {}, 'result': None, 'error': None, 'stuck': None, 'lattice': None,
            'acceptance': {'verdict': None, 'certificate_steps': 0, 'fallback_steps': 0, 'probe': None,
                           'timed_out_attempts': 0},
            'totals': {'bulk_twists': 0, 'height_twists': 0, 'high_twists': 0, 'tail_twists': 0,
                       'fallback_twists': 0, 'lattice_twists': None, 'seconds': 0}}


def step3(base, step, word, **fields):
    """A format 3 row: a format 1 row with the new labels and a one-span tree named by its rule."""
    row = dict(step, subphase=None, encoding='json', word_sha256=word_sha(word), standing='rule', seconds=0.0)
    if row['phase'] != 'bulk':
        row.setdefault('gate', 'stepchk')
        row.setdefault('attempts', [])
    if 'tree' not in fields:
        row['tree'] = write_tree(base, row['index'], {'name': row['rule'], 'letters': len(word),
                                                      'span': [0, len(word)]})
    row.update(fields)
    return row


def phases3(bulk, docs, results, counts=None):
    """The phases block. docs[k] is the snapshot after position k (0 bulk, 1 to 3 the height
    sub-phases, 4 high, 5 tail), results[k - 1] the result and counts[k] the (steps, twists)
    of position k."""
    counts = counts or {}

    def block(k):
        data = {'result': results[k - 1], 'steps': counts.get(k, (0, 0))[0]}
        if results[k - 1] is not None:
            data.update(start=snap(docs[k - 1]), end=snap(docs[k]))
        return data

    ran = [k for k in (1, 2, 3) if results[k - 1] is not None]
    subs = [results[k - 1] for k in ran]
    height = {'result': next((r for r in ('error', 'budget', 'timeout', 'lattice') if r in subs),
                             subs[-1] if subs else None),
              'steps': sum(counts.get(k, (0, 0))[0] for k in (1, 2, 3)),
              'twists': sum(counts.get(k, (0, 0))[1] for k in (1, 2, 3)),
              'start': snap(docs[0]) if ran else None, 'end': snap(docs[ran[-1]]) if ran else None,
              'sub': {sub: block(k) for k, sub in enumerate(SUBPHASES, 1)}}
    return {'bulk': bulk, 'height': height, 'high': {**block(4), 'twists': counts.get(4, (0, 0))[1]},
            'tail': {'result': results[4], 'steps': counts.get(5, (0, 0))[0]},
            'lattice': {'result': None, 'steps': 0, 'twists': None}}


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
        elif fixture in ('height3', 'h46', 'played') and not hasattr(type(self), fixture):
            started = time.monotonic()
            getattr(type(self), 'build_' + fixture)()
            print('fixture %s: %.2fs' % (fixture, time.monotonic() - started), flush=True)
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

    # Format 3 (plan runner-run/3). Fixtures with height steps or a played lattice are
    # built on first use, like the lattice fixture.

    @classmethod
    def build_height3(cls):
        base = cls.tmp / 'height3'
        st = sim.State(restore.ctx, menu=cls.menu)
        apply(st, cls.menu, WORD36)
        start = state_record(st)
        assert start['digest'].startswith('8733a0aa') and start['heights'] == {'2': 5799, '6': 1}
        base.mkdir()
        cls.export(st, base / 'start-table.json')
        (base / 'tables').mkdir()
        shutil.copyfile(base / 'start-table.json', base / 'tables/bulk-end.json')
        rec = blank3({'kind': 'table', 'file': 'start-table.json', **start})
        # Two single letters at Z36 (integrator experiment): (n6, n4, n2) falls from
        # (1, 0, 5799) to (0, 1, 3097), tier 0, and then to (0, 0, 5405), tier 1, which ends
        # r22d75 (HEIGHT2). The rows carry the fields the dispatcher (P1) writes for r22d75.
        docs = [start]
        for index, (rule, letter) in enumerate((('r22d75/T0', [12, 14]), ('r22d75/T1', [35, 13]))):
            apply(st, cls.menu, [letter])
            docs.append(state_record(st))
            name = 'words/%05d.json' % index
            write(base / name, {'word': [letter], 'input_digest': docs[-2]['digest'],
                                'output_digest': docs[-1]['digest']})
            rec['steps'].append(step3(base, {'index': index, 'phase': 'height', 'rule': rule, 'word': name,
                                             'letters': 1, 'contract': rule, 'gate': 'r22d75',
                                             'before': docs[-2], 'after': docs[-1]}, [letter], subphase='r22d75',
                                      tau_before=None, tau_after=None, u_before=None, u_after=None, u_mid=None))
        assert docs[1]['heights'] == {'2': 3097, '4': 1} and docs[2]['heights'] == {'2': 5405}
        bulk = {'result': 'stuck', 'steps': 0, 'twists': 0, 'end': {**start, 'table': 'tables/bulk-end.json'}}
        rec['phases'] = phases3(bulk, [start] + [docs[2]] * 5,
                                ['handoff', 'not_applicable', 'not_applicable', 'not_applicable', 'stuck'], {1: (2, 2)})
        rec['stuck'] = {**docs[2], 'phase': 'tail', 'reason': 'no_rule', 'units': []}
        rec['result'] = rec['acceptance']['verdict'] = 'stuck'
        rec['totals']['height_twists'] = 2
        write(base / 'run.json', rec)
        cls.height3 = base

    @classmethod
    def h46_state(cls):
        # Not reachable, and the checker does not ask: Z36 with one more off piece, posed as the
        # height-4 piece after WORD36's first 25 letters. Its maximum is 6 and n4 is 1.
        st = sim.State(restore.ctx, menu=cls.menu)
        apply(st, cls.menu, WORD36[:25])
        matrix = next(st._pose_matrix(int(p)) for p in np.unique(st.pose_id) if st._poses[int(p)].kidx < 0
                      and trees.dist(trees.factors(st._pose_matrix(int(p)))[0]) == 4)
        st = sim.State(restore.ctx, menu=cls.menu)
        apply(st, cls.menu, WORD36)
        st.pose_id[int(np.flatnonzero(~off(st))[0])] = st._intern_matrix(matrix)
        st._compact()
        st._anchor_cache = {}
        return st

    @classmethod
    def build_h46(cls):
        st = cls.h46_state()
        start = state_record(st)
        assert start['heights'] == {'2': 5799, '4': 1, '6': 1}
        base = cls.tmp / 'h46'
        base.mkdir()
        cls.export(st, base / 'start-table.json')
        (base / 'tables').mkdir()
        shutil.copyfile(base / 'start-table.json', base / 'tables/bulk-end.json')
        rec = blank3({'kind': 'table', 'file': 'start-table.json', **start})
        bulk = {'result': 'stuck', 'steps': 0, 'twists': 0, 'end': {**start, 'table': 'tables/bulk-end.json'}}
        rec['phases'] = phases3(bulk, [start] * 6, ['stuck', 'stuck', 'not_applicable', 'stuck', 'stuck'])
        rec['stuck'] = {**start, 'phase': 'tail', 'reason': 'no_rule', 'units': []}
        rec['result'] = rec['acceptance']['verdict'] = 'stuck'
        write(base / 'run.json', rec)
        cls.h46 = base

    @classmethod
    def build_played(cls):
        # A lattice configuration that is not solved: three retained letters, as in build_lattice.
        retained = [i for i in range(len(cls.menu.items)) if i not in restore.jumble_items(cls.menu)
                    and restore.element(cls.menu, 0, i) != restore.I4]
        letters = [[0, retained[0]], [17, retained[1]], [379, retained[0]]]
        st = sim.State(restore.ctx, menu=cls.menu)
        apply(st, cls.menu, letters)
        start = state_record(st)
        assert start['off'] == 0 and start['digest'] != C.SOLVED
        base = cls.tmp / 'played'
        base.mkdir()
        cls.export(st, base / 'start-table.json')
        (base / 'tables').mkdir()
        shutil.copyfile(base / 'start-table.json', base / 'tables/bulk-end.json')
        rec = blank3({'kind': 'table', 'file': 'start-table.json', **start})
        bulk = {'result': 'lattice', 'steps': 0, 'twists': 0, 'end': {**start, 'table': 'tables/bulk-end.json'}}
        rec['phases'] = phases3(bulk, [start] * 6, ['not_applicable'] * 5)
        write(base / 'run.json', rec)
        inverse = inverse_items(cls.menu)
        cls.played, cls.played_letters = base, letters
        cls.played_word = [[c, inverse[i]] for c, i in reversed(letters)]

    def v3_case(self, fixture='bulk', label=None, mode='development'):
        """A format 3 record: the format 1 fixtures converted, the others built as format 3."""
        base, old = self.case(fixture, label)
        if old['format'] == 'runner-run/3':
            if mode != old['config']['mode']:
                old['config'] = blank3(old['start'], mode)['config']
            return base, old
        rec = blank3(old['start'], mode)
        bulk = old['phases']['bulk']
        tail = [s for s in old['steps'] if s['phase'] == 'tail']
        for step in old['steps']:
            word = read(base / step['word'])['word']
            fields = {}
            if step['rule'] == 't1prime':
                fields = dict(V_source='tclose2', V_letters=339, V_sha256=word_sha(word[288:627]),
                              parts=[['W', 288], ['V', 339], ['middle', 288], ['V_inverse', 339]],
                              tree=write_tree(base, step['index'], CLOSURE_TREE))
            rec['steps'].append(step3(base, step, word, **fields))
        docs = [snap(bulk['end'])] * 5 + [snap(tail[-1]['after']) if tail else snap(bulk['end'])]
        results = ['stuck' if holds(name, docs[0]) else 'not_applicable' for name in SUBPHASES + ('high',)]
        results.append({'bulk': 'not_applicable', 'tail': 'budget', 'short': 'lattice'}[fixture])
        rec['phases'] = phases3(bulk, docs, results, {5: (len(tail), sum(s['letters'] for s in tail))})
        rec['result'] = rec['acceptance']['verdict'] = 'budget' if fixture == 'tail' else 'solved'
        for phase in ('bulk', 'tail'):
            rec['totals'][phase + '_twists'] = old['totals'][phase + '_twists']
        return base, rec

    def played_case(self, prefix=None, labels=True):
        base, rec = self.case('played')
        word = self.played_word[:prefix]
        st = sim.State(restore.ctx, menu=self.menu)
        apply(st, self.menu, self.played_letters)
        before = state_record(st)
        apply(st, self.menu, word)
        after = state_record(st)
        name, sha = write_ia16(base, 0, word)
        self.export(st, base / 'tables/00000.json')
        rec['steps'] = [{'index': 0, 'phase': 'lattice', 'subphase': None, 'rule': 'lattice/orbit-0', 'word': name,
                         'letters': len(word), 'encoding': 'ia16', 'word_sha256': sha,
                         'tree': write_tree(base, 0, {'name': 'orbit 0', 'letters': len(word),
                                                      'span': [0, len(word)]}),
                         'contract': 'lattice_literal', 'gate': 'player', 'standing': 'rule', 'attempts': [],
                         'before': before, 'after': after, 'seconds': 0.0, 'labels_identity': labels,
                         'table': 'tables/00000.json'}]
        report = write_report(base, rec['config']['lattice']['restorer'], before['digest'], after['digest'],
                              len(word))
        rec['phases']['lattice'] = {'result': 'solved', 'steps': 1, 'twists': len(word), 'report': report}
        rec['totals']['lattice_twists'] = len(word)
        rec['result'] = rec['acceptance']['verdict'] = 'solved'
        return base, rec, after

    def probe_case(self):
        """Acceptance mode at the short fixture's start: the tail is stuck and the fallback,
        run only as a probe, finds the one-letter word that lands the unit."""
        base, old = self.case('short')
        start = snap(old['start'])
        rec = blank3(old['start'], 'acceptance')
        results = ['stuck' if holds(name, start) else 'not_applicable' for name in SUBPHASES + ('high',)]
        rec['phases'] = phases3(old['phases']['bulk'], [start] * 6, results + ['stuck'])
        word = read(base / old['steps'][0]['word'])
        write(base / 'probe/word.json', word)
        write(base / 'probe/unit.json', read(base / 'units/one.json'))
        probe = {'unit': 'probe/unit.json', 'unit_key': 0, 'word': 'probe/word.json', 'letters': len(word['word']),
                 'input_digest': word['input_digest'], 'output_digest': word['output_digest']}
        rec['stuck'] = {**start, 'phase': 'tail', 'reason': 'no_rule',
                        'units': [{'unit_key': 0, 'size': 1, 'attempts': []}], 'probe': probe}
        rec['acceptance'].update(probe=True, verdict='coverage_failure')
        rec['result'] = 'stuck'
        return base, rec

    def test_v3_bulk(self):
        base, rec = self.v3_case()
        final = self.check(base, rec)
        self.assertEqual((final['verdict'], final['end_digest'], final['unchecked_contracts'], final['bulk_twists']),
                         ('solved', C.SOLVED, 0, 9))

    def test_v3_acceptance_config(self):
        """Acceptance mode lifts every clock: null in config.budgets, the sentinel for the children."""
        base, original = self.v3_case(mode='acceptance')
        self.assertEqual(self.check(base, original)['verdict'], 'solved')
        for label, edit in (('a clock kept', lambda c: c['budgets'].update(height_cpu=3600)),
                            ('a clock missing', lambda c: c['budgets'].pop('kernel_seconds')),
                            ('no sentinel', lambda c: c.update(clock_sentinel=None)),
                            ('kernel clock', lambda c: c['kernel'].update(seconds=3600))):
            with self.subTest(label=label):
                rec = copy.deepcopy(original)
                edit(rec['config'])
                self.check(base, rec, 'config')
        # The committed acceptance options and the kernel's D19 rule (plan revision 3e, E2 and E3).
        other = 'schedule11' if original['config']['lattice']['restorer'] == 'late' else 'late'
        for label, edit, failure in (
                ('no acceptance config', lambda c: c.update(acceptance_config=None), 'acceptance_config'),
                ('another option', lambda c: c['budgets'].update(par=2), 'acceptance_config'),
                ('a step budget', lambda c: c['budgets'].update(bulk_steps=5), 'acceptance_config'),
                ('another restorer', lambda c: c['lattice'].update(restorer=other), 'acceptance_config'),
                ('other tail rules', lambda c: c.update(tail_rules=['pure']), 'acceptance_config'),
                ('another file', lambda c: c['acceptance_config'].update(sha256='0' * 64), 'acceptance_config'),
                ('other recorded options', lambda c: c['acceptance_config']['options'].update(par=2),
                 'acceptance_config'),
                ('no d19', lambda c: c['kernel'].update(d19=None), 'kernel_d19'),
                ('another d19', lambda c: c['kernel']['d19'].update(sha256='0' * 64), 'kernel_d19')):
            with self.subTest(label=label):
                rec = copy.deepcopy(original)
                edit(rec['config'])
                self.check(base, rec, failure)
        # A development run records no acceptance options, and a D19 only if it is this checkout's.
        base, rec = self.v3_case(label='development')
        frozen, d19 = acceptance_records()
        self.assertEqual(self.check(base, copy.deepcopy(rec))['verdict'], 'solved')
        rec['config']['kernel']['d19'] = d19
        self.assertEqual(self.check(base, copy.deepcopy(rec))['verdict'], 'solved')
        rec['config']['acceptance_config'] = frozen
        self.check(base, rec, 'acceptance_config')

    def test_v3_config(self):
        base, original = self.v3_case()
        for path, value, failure in ((('mode',), 'unknown', 'config'), (('component',), 'a/b', 'config'),
                                     (('kernel', 'max_steps'), 2, 'config'), (('lattice', 'restorer'), 'x', 'config'),
                                     (('budgets', 'cert_seconds'), 0, 'config'), (('clock_sentinel',), 1e9, 'config'),
                                     (('phase_order',), ['bulk', 'tail', 'height', 'high', 'lattice'], 'phase_order'),
                                     (('subphase_order',), {'height': ['r22d81', 'r22d75', 'h4']}, 'phase_order')):
            with self.subTest(path=path, value=value):
                rec = copy.deepcopy(original)
                doc = rec['config']
                for key in path[:-1]:
                    doc = doc[key]
                doc[path[-1]] = value
                self.check(base, rec, failure)

    def test_v3_code_fingerprint(self):
        base, original = self.v3_case()
        rec = copy.deepcopy(original)
        rec['config']['code'] = {'sha256': 'a' * 64, 'files': 2}
        self.check(base, rec)
        for code in (None, {}, {'sha256': 'x' * 64, 'files': 2}, {'sha256': 'a' * 64, 'files': 0},
                     {'sha256': 'a' * 64, 'files': True}, {'sha256': 'a' * 64, 'files': 2, 'extra': 1}):
            with self.subTest(code=code):
                rec['config']['code'] = code
                self.check(base, rec, 'config_code')

    def test_v3_resumes(self):
        base, original = self.v3_case()
        source = {'commit': None, 'status_sha256': None, 'dirty': None}
        original['resumes'] = [{'at_step': 0, 'seconds': 0.0, 'source': source},
                               {'at_step': len(original['steps']), 'seconds': 1.0, 'source': source}]
        self.check(base, original)
        for rows in (None, {}, [None], [{'at_step': True, 'seconds': 0, 'source': source}],
                     [{'at_step': -1, 'seconds': 0, 'source': source}],
                     [{'at_step': len(original['steps']) + 1, 'seconds': 0, 'source': source}]):
            # Non-finite numbers are written by the mutation helper deliberately.
            with self.subTest(rows=rows):
                rec = copy.deepcopy(original)
                rec['resumes'] = rows
                self.check(base, rec, 'resumes')
        mutations = [dict(at_step=0, seconds=s, source=source) for s in (-1, True, float('nan'), float('inf'))]
        mutations += [dict(at_step=0, seconds=0, source=s) for s in (None, {}, {'dirty': 'yes'})]
        mutations += [dict(at_step=0, seconds=0, source=source, extra=1), dict(at_step=0, seconds=0)]
        for row in mutations:
            with self.subTest(row=row):
                rec = copy.deepcopy(original)
                rec['resumes'] = [row]
                self.check(base, rec, 'resumes')
        for rows in ([{'at_step': 1, 'seconds': 0, 'source': source},
                      {'at_step': 0, 'seconds': 1, 'source': source}],
                     [{'at_step': 0, 'seconds': 2, 'source': source},
                      {'at_step': 0, 'seconds': 1, 'source': source}]):
            rec = copy.deepcopy(original)
            rec['resumes'] = rows
            self.check(base, rec, 'resumes')
        for version in ('runner-run/1', 'runner-run/2'):
            fails = []
            C.check_resumes({'format': version, 'steps': [], 'resumes': []}, fails)
            self.assertEqual(fails, ['resumes'])

    def test_v3_bulk_order(self):
        """No reopen in format 3: it reads the solver's earlier twists, not the state (plan D2)."""
        base, original = self.v3_case()
        for change in ('config', 'step', 'repeat'):
            with self.subTest(change=change):
                rec = copy.deepcopy(original)
                if change == 'config':
                    rec['config']['bulk_order'].append('reopen')
                elif change == 'step':
                    rec['steps'][3]['rule'] = 'reopen'
                else:
                    rec['config']['bulk_order'] = ['seam', 'seam']
                self.check(base, rec, 'bulk_order')

    def test_v3_step_labels(self):
        base, original = self.v3_case()
        for key, value, failure in (('encoding', 'gzip', 'encoding'), ('encoding', 'ia16', 'word_file'),
                                    ('word_sha256', '0' * 64, 'word_sha256'), ('standing', 'fallback', 'standing'),
                                    ('standing', 'certificate', 'standing'), ('subphase', 'r22d75', 'subphase'),
                                    ('phase', 'kernel', 'phase'), ('seconds', -1, 'step_keys'),
                                    ('seconds', float('nan'), 'step_keys'), ('standing', None, 'step_keys'),
                                    ('labels_identity', True, 'labels_identity'), ('tree', None, 'tree_missing')):
            with self.subTest(key=key, value=value):
                rec = copy.deepcopy(original)
                if value is None:
                    del rec['steps'][0][key]
                else:
                    rec['steps'][0][key] = value
                self.check(base, rec, failure)

    def test_v3_closure_tree(self):
        """A closure tree whose V⁻¹ names V and whose middle names W passes the full replay."""
        base, rec = self.v3_case('tail')
        final = self.check(base, rec)
        self.assertEqual((final['verdict'], final['tail_twists']), ('budget', 1254))

    def test_v3_tree_failures(self):
        base, original = self.v3_case('tail')

        def changed(edit):
            root = copy.deepcopy(CLOSURE_TREE)
            edit(root)
            return root

        def closure(root):
            return root['parts'][1]['parts']

        deep = {'name': 'leaf', 'letters': 1254, 'span': [0, 1254]}
        for _ in range(64):
            deep = {'name': 'level', 'letters': 1254, 'parts': [deep]}
        trees_ = (
            ('part sum', changed(lambda r: r['parts'][1].update(letters=965)), None, 'tree_length'),
            ('gap', changed(lambda r: closure(r)[0].update(span=[289, 627], letters=338)), None, 'tree_expansion'),
            ('overlap', changed(lambda r: closure(r)[0].update(span=[287, 627], letters=340)), None, 'tree_expansion'),
            ('root shorter than the word', {'name': 't1prime', 'letters': 1253, 'span': [0, 1253]}, None,
             'tree_length'),
            ('wrong inverse range', changed(lambda r: closure(r).__setitem__(2, {
                'name': 'V⁻¹', 'letters': 339, 'parts': [
                    {'name': 'head', 'letters': 288, 'span': [915, 1203], 'inverse_of': [0]},
                    {'name': 'rest', 'letters': 51, 'span': [1203, 1254]}]})), None, 'tree_expansion'),
            ('inverse of a later part', changed(lambda r: closure(r)[1].update(inverse_of=[1, 2])), None,
             'tree_malformed'),
            ('inverse of a missing part', changed(lambda r: closure(r)[1].update(inverse_of=[5])), None,
             'tree_malformed'),
            ('same_as with another name', changed(lambda r: closure(r)[2].update(same_as=closure(r)[2].pop('inverse_of'))),
             None, 'tree_malformed'),
            ('span and parts', changed(lambda r: r['parts'][1].update(span=[288, 1254])), None, 'tree_malformed'),
            ('empty name', changed(lambda r: r['parts'][0].update(name='')), None, 'tree_malformed'),
            ('empty parts', {'name': 't1prime', 'letters': 1254, 'parts': []}, None, 'tree_malformed'),
            ('depth 65', deep, None, 'tree_malformed'),
            ('size', CLOSURE_TREE, 4, 'tree_malformed'),
            ('forbidden key', changed(lambda r: r['parts'][0].update(slp=1)), None, 'forbidden_key'),
        )
        for label, root, size, failure in trees_:
            with self.subTest(label=label):
                rec = copy.deepcopy(original)
                rec['steps'][0]['tree'] = write_tree(base, 0, root, size)
                self.check(base, rec, failure)
        path = base / original['steps'][0]['tree']['file']
        data = gzip.decompress(path.read_bytes())
        for label, raw, failure in (('wrong sha', None, 'tree_sha256'), ('missing file', b'', 'tree_missing'),
                                    ('not gzip', data, 'tree_malformed'),
                                    ('mtime', gzip.compress(data, mtime=1), 'tree_malformed')):
            with self.subTest(label=label):
                rec = copy.deepcopy(original)
                write_tree(base, 0, CLOSURE_TREE)
                if raw is None:
                    rec['steps'][0]['tree']['sha256'] = '0' * 64
                elif not raw:
                    path.unlink()
                else:
                    path.write_bytes(raw)
                self.check(base, rec, failure)

    def test_v3_tree_same_as(self):
        """same_as names an earlier part with the same name and the same letters (plan section 1)."""
        base, rec = self.v3_case('tail')
        step = rec['steps'][0]
        word = read(base / step['word'])['word']
        n = len(word)
        i, j = next((i, j) for i in range(1, n) for j in range(i + 2, n - 1) if word[i] == word[j])
        k = next(k for k in range(i + 2, n - 1) if word[k] != word[i])

        def flat(name, at):
            ends = [(None, 0), ('x', i), ('letter', i + 1), ('y', at), (name, at + 1), ('z', n)]
            parts = [{'name': part, 'letters': end - ends[m][1], 'span': [ends[m][1], end]}
                     for m, (part, end) in enumerate(ends[1:])]
            parts[3]['same_as'] = [1]
            return {'name': 't1prime', 'letters': n, 'parts': parts}

        inverses = inverse_items(self.menu)
        for label, root, expected in (('same letter', flat('letter', j), []),
                                      ('other name', flat('other', j), ['tree_malformed']),
                                      ('other letter', flat('letter', k), ['tree_expansion'])):
            with self.subTest(label=label):
                fails = []
                C.check_tree(base, {**step, 'tree': write_tree(base, 0, root)}, word, inverses, fails)
                self.assertEqual(fails, expected)

    def test_v3_height_contracts(self):
        """The height rows pass check_contracts with every contract checked; a hook that leaves one
        unchecked is counted and fails the record unless the flag allows it."""
        base, rec = self.v3_case('height3')
        rows = []
        final = C.check_run(base, rows.append)
        self.assertTrue(final['pass'], rows)
        self.assertEqual((final['verdict'], final['height_twists'], final['unchecked_contracts']), ('stuck', 2, 0))
        self.assertTrue(all(row['pass'] and not row['unchecked'] and row['subphase'] == 'r22d75'
                            for row in rows[:-1]), rows)
        proc = subprocess.run([sys.executable, '-I', '-B', str(HERE / 'check_run.py'), str(base)],
                              cwd=REPO, capture_output=True, text=True, timeout=600, check=False)
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertEqual(json.loads(proc.stdout.splitlines()[-1])['unchecked_contracts'], 0)
        for fields, failure in (({'contract': 'height_order'}, 'height_rule'),
                                ({'rule': 'r22d75/T2', 'contract': 'r22d75/T2'}, 'height_tier')):
            with self.subTest(fields=fields):
                bad = copy.deepcopy(rec)
                bad['steps'][0].update(fields)
                self.check(base, bad, failure)
        with patch.object(C.check_contracts, 'finish', lambda *a: [C.UNCHECKED]):
            self.assertEqual(self.check(base, rec, 'unchecked_contract')['unchecked_contracts'], 2)
            final = self.check(base, rec, allow_unchecked=True)
            self.assertEqual(final['unchecked_contracts'], 2)

    def test_v3_contract_hooks(self):
        """prepare sees the state before a step, on_letter each letter, finish the state after (M6)."""
        base, rec = self.v3_case('height3')
        digests = [rec['steps'][0]['before']['digest']] + [s['after']['digest'] for s in rec['steps']]
        seen = []

        def prepare(row, st):
            seen.append(('prepare', row['index'], st.digest()))
            return row['index']

        def on_letter(ctx, k, st):
            seen.append(('letter', ctx, k, st.digest()))

        def finish(ctx, row, st):
            seen.append(('finish', ctx, st.digest()))
            return []

        hooks = C.check_contracts
        with patch.object(hooks, 'prepare', prepare), patch.object(hooks, 'on_letter', on_letter), \
                patch.object(hooks, 'finish', finish):
            final = self.check(base, rec)
        self.assertEqual(final['unchecked_contracts'], 0)
        self.assertEqual(seen, [('prepare', 0, digests[0]), ('letter', 0, 0, digests[1]), ('finish', 0, digests[1]),
                                ('prepare', 1, digests[1]), ('letter', 1, 0, digests[2]), ('finish', 1, digests[2])])

        def crash(*args):
            raise RuntimeError('hook crashed')

        for name, hook, failure in (('prepare', crash, 'contract'), ('on_letter', crash, 'contract'),
                                    ('finish', crash, 'contract'), ('finish', lambda *a: 'none', 'contract'),
                                    ('finish', lambda *a: ['n6_rise'], 'n6_rise')):
            with self.subTest(name=name, failure=failure), patch.object(hooks, name, hook):
                self.check(base, rec, failure, allow_unchecked=True)

    def test_v3_entry_tests(self):
        """Every not_applicable, handoff and lattice claim is recomputed from the checker's snapshots."""
        base, original = self.v3_case('height3')

        def sub(name, result):
            return lambda rec: rec['phases']['height']['sub'][name].update(result=result)

        for label, edit in (('r22d75 stuck after HEIGHT2', sub('r22d75', 'stuck')),
                            ('r22d75 lattice with pieces off', sub('r22d75', 'lattice')),
                            ('r22d81 stuck without height 6', sub('r22d81', 'stuck')),
                            ('high stuck without height 4 or 6', lambda rec: rec['phases']['high'].update(result='stuck')),
                            ('tail not applicable with pieces off',
                             lambda rec: rec['phases']['tail'].update(result='not_applicable'))):
            with self.subTest(label=label):
                rec = copy.deepcopy(original)
                edit(rec)
                self.check(base, rec, 'entry_test')

    def test_v3_phase_order(self):
        base, original = self.v3_case('height3')
        rec = copy.deepcopy(original)
        rec['steps'][0]['subphase'] = 'r22d81'          # r22d75 entered again after r22d81
        self.check(base, rec, 'phase_order')
        rec = copy.deepcopy(original)
        h4 = rec['phases']['height']['sub']['h4']       # high ran although h4 never did
        h4['result'] = None
        del h4['start'], h4['end']
        self.check(base, rec, 'phase_order')

    def test_v3_omitted_phases(self):
        """A phase or sub-phase is left out only after a result that ends the run, or in an
        error record cut short by an exception (plan section 0, phase results)."""
        base, original = self.v3_case('h46')
        start = snap(original['start'])
        results = ['stuck', 'stuck', 'not_applicable', 'stuck', 'stuck']

        def cut(kept, result, **fields):
            rec = copy.deepcopy(original)
            rec['phases'] = phases3(rec['phases']['bulk'], [start] * 6, kept + [None] * (5 - len(kept)))
            rec['result'] = rec['acceptance']['verdict'] = result
            rec['stuck'] = rec['stuck'] if result == 'stuck' else None
            rec.update(fields)
            return rec

        # A continuing result, then nothing, claimed as budget. With nothing kept this is the
        # review's counterexample (I0-03): a stuck bulk and no later phase.
        for n in range(5):
            with self.subTest(kept=results[:n]):
                self.check(base, cut(results[:n], 'budget'), 'phase_result')
        for kept in (['budget'], ['stuck', 'timeout'], results[:4] + ['budget']):
            with self.subTest(kept=kept):
                self.assertEqual(self.check(base, cut(kept, 'budget'))['verdict'], 'budget')
        # An exception in r22d81 ends the run as error; the runner leaves the height result null.
        rec = cut(['stuck'], 'error', error='r22d81 crashed')
        rec['phases']['height']['result'] = None
        rec['phases']['height']['sub']['r22d81']['start'] = start
        self.assertEqual(self.check(base, rec)['verdict'], 'error')
        # An exception after a tail step ends the run as error; the step stands and the tail
        # result stays null. The same record with any other run result fails.
        base, rec = self.v3_case('tail', 'error-after-step')
        self.assertTrue(rec['phases']['tail']['steps'])
        rec['phases']['tail']['result'] = None
        rec.update(result='error', error='stepchk digest mismatch', stuck=None)
        rec['acceptance']['verdict'] = 'error'
        self.assertEqual(self.check(base, rec)['verdict'], 'error')
        rec.update(result='budget', error=None)
        rec['acceptance']['verdict'] = 'budget'
        self.check(base, rec, 'phase_result')
        # With every piece on the lattice, --no-lattice ends the run budget with no lattice result.
        base, rec = self.v3_case('short', 'no-lattice')
        rec['result'] = rec['acceptance']['verdict'] = 'budget'
        self.assertEqual(self.check(base, rec)['verdict'], 'budget')
        rec['result'] = rec['acceptance']['verdict'] = 'stuck'
        rec['stuck'] = {**snap(rec['steps'][-1]['after']), 'phase': 'tail', 'reason': 'no_rule', 'units': []}
        self.check(base, rec, 'phase_result')

    def test_v3_interrupted_phase_entry(self):
        """A phase cut off by an exception after its steps keeps its start and its entry test
        (scoped verification, I0-V1-01)."""
        # r22d75 applies at Z36: its two steps stand when an exception follows them.
        base, rec = self.v3_case('height3', 'interrupted')
        rec = copy.deepcopy(rec)
        sub = rec['phases']['height']['sub']
        sub['r22d75']['result'] = None
        del sub['r22d75']['end']
        sub['r22d81'] = {'result': None, 'steps': 0}
        sub['h4'] = {'result': None, 'steps': 0}
        rec['phases']['height'].update(result=None, end=None)
        rec['phases']['high'] = {'result': None, 'steps': 0, 'twists': 0}
        rec['phases']['tail'] = {'result': None, 'steps': 0}
        rec.update(result='error', error='r22d75 crashed', stuck=None)
        rec['acceptance']['verdict'] = 'error'
        self.assertEqual(self.check(base, rec)['verdict'], 'error')
        # The same shape in the tail, a step recorded at a lattice configuration, already fails the
        # tail's own step contract (its unit must be off and land) before the phase rule is reached.
        base, rec, _ = self.played_case()
        rec['steps'][0].update(phase='tail', rule='pure', contract='strict_prime', gate='stepchk')
        rec['phases']['tail'] = {'result': None, 'steps': 1}
        rec['phases']['lattice'] = {'result': None, 'steps': 0, 'twists': None}
        rec['totals'].update(lattice_twists=None, tail_twists=rec['steps'][0]['letters'])
        rec.update(result='error', error='stepchk crashed')
        rec['acceptance']['verdict'] = 'error'
        self.check(base, rec, 'landed')

    def test_interrupted_plugin_phase_rules(self):
        """check_plugin_phase on a phase that an exception cut off, with stand-in snapshots."""
        class Snap:
            def __init__(self, heights, digest):
                self.hist, self.count, self.digest = heights, sum(heights.values()), digest

            def heights(self):
                return self.hist

            def check(self, doc, prefix, fails, heights=False):
                if not isinstance(doc, dict) or doc.get('digest') != self.digest:
                    C.add(fails, prefix)

        solved, z36 = Snap({}, 's' * 64), Snap({'2': 5799, '6': 1}, 'z' * 64)
        for name, before, start, count, run, expected in (
                ('high', solved, 's', 1, 'error', ['entry_test']),       # the review's counterexample
                ('high', z36, 'z', 1, 'error', []),
                ('h4', z36, 'z', 0, 'error', []),                        # cut off before its entry test mattered
                ('h4', z36, 'z', 1, 'error', ['entry_test']),
                ('r22d81', z36, 'x', 1, 'error', ['r22d81_start']),
                ('high', z36, None, 1, 'error', ['phase_result']),       # steps without a start
                ('high', z36, 'z', 1, 'budget', ['phase_result'])):
            with self.subTest(name=name, start=start, count=count, run=run):
                data = {'result': None, 'steps': count}
                if start:
                    data['start'] = {'digest': start * 64}
                fails = []
                self.assertIsNone(C.check_plugin_phase(name, data, count, before, before, run, fails))
                self.assertEqual(fails, expected)

    def test_v3_h4_entry_needs_max_4(self):
        """h4's entry test is max <= 4 and n4 > 0: with height 6 present, h4 is not applicable."""
        base, original = self.v3_case('h46')
        self.assertEqual(self.check(base, original)['verdict'], 'stuck')
        rec = copy.deepcopy(original)
        rec['phases']['height']['sub']['h4']['result'] = 'stuck'
        self.check(base, rec, 'entry_test')

    def test_v3_verdict_timed_out_attempt(self):
        """A stop with an attempt at the final state that timed out is budget, not stuck (R22D-55)."""
        base, original = self.v3_case('h46')
        rec = copy.deepcopy(original)
        rec['phases']['height']['sub']['r22d75'].update(reason='stub', details={}, attempts=[{'timed_out': True}])
        rec['acceptance']['timed_out_attempts'] = 1
        self.check(base, rec, 'verdict')
        rec['acceptance']['verdict'] = 'budget'
        self.assertEqual(self.check(base, rec)['verdict'], 'budget')
        rec['acceptance']['timed_out_attempts'] = 0
        self.check(base, rec, 'acceptance')
        rec = copy.deepcopy(original)
        rec['config']['component'] = 'height_stub'
        rec['acceptance']['verdict'] = 'component'
        self.assertEqual(self.check(base, rec)['verdict'], 'component')
        rec['acceptance']['verdict'] = 'stuck'
        self.check(base, rec, 'verdict')

    def test_v3_verdict_earlier_timeout(self):
        """A timed-out attempt before a step is counted, but the final state's stop stays stuck."""
        base, rec = self.v3_case('height3')
        rec['steps'][0]['attempts'] = [{'timed_out': True}]
        rec['acceptance']['timed_out_attempts'] = 1
        self.assertEqual(self.check(base, rec)['verdict'], 'stuck')

    def test_v3_verdict_order(self):
        """The verdict is the first match in the order of plan section 0."""
        final, earlier = SimpleNamespace(digest='f' * 64), SimpleNamespace(digest='e' * 64)

        def case(result='stuck', component=None, standings=(), probe=False, unit=False, sub=None, step=False):
            phases = {'height': {'sub': {s: {'result': None} for s in SUBPHASES}}, 'high': {'result': None}}
            ends = [earlier] * 7
            if sub is not None:                 # a timed-out r22d75 attempt, at the final state or not
                phases['height']['sub']['r22d75'] = {'result': 'stuck', 'attempts': [{'timed_out': True}]}
                ends[1] = final if sub else earlier
            steps = [{'phase': 'tail', 'standing': s, 'attempts': [{'timed_out': step}]} for s in standings]
            stuck = {'units': [{'attempts': [{'timed_out': unit}]}]}
            if probe:
                stuck['probe'] = {}
            timed = sum(a['timed_out'] for s in steps for a in s['attempts']) + unit + (sub is not None)
            record = {'result': result, 'config': {'component': component, 'mode': 'acceptance'}, 'phases': phases,
                      'stuck': stuck, 'acceptance': {'certificate_steps': standings.count('certificate'),
                                                     'fallback_steps': standings.count('fallback'),
                                                     'probe': probe, 'timed_out_attempts': timed}}
            return record, steps, ends

        for kwargs, verdict in (
                (dict(result='error', component='x', standings=('certificate',)), 'error'),
                (dict(component='bulk_budget', standings=('certificate', 'fallback')), 'component'),
                (dict(standings=('certificate', 'fallback'), probe=True), 'stuck'),
                (dict(standings=('fallback',)), 'coverage_failure'),
                (dict(probe=True, unit=True), 'coverage_failure'),
                (dict(result='budget'), 'budget'),
                (dict(unit=True), 'budget'),
                (dict(sub=True), 'budget'),
                (dict(sub=False), 'stuck'),
                (dict(standings=('rule',), step=True), 'stuck'),
                (dict(), 'stuck'),
                (dict(result='solved', standings=('rule',)), 'solved')):
            with self.subTest(**kwargs):
                record, steps, ends = case(**kwargs)
                record['acceptance']['verdict'] = verdict
                fails = []
                self.assertEqual(C.check_acceptance(record, steps, final, ends, fails), verdict)
                self.assertEqual(fails, [])
                record['acceptance']['verdict'] = 'other'
                C.check_acceptance(record, steps, final, ends, fails)
                self.assertEqual(fails, ['verdict'])

    def test_v3_entry_function(self):
        """The six entry tests of plan section 0, at their boundaries."""
        def snapshot(off, **heights):
            return SimpleNamespace(count=off, heights=lambda: {h[1:]: n for h, n in heights.items()})

        for name, snap_, expected in (
                ('r22d75', snapshot(2, h2=1, h4=1), True), ('r22d75', snapshot(2, h2=2), False),
                ('r22d75', snapshot(2, h8=1, h6=1), False), ('r22d81', snapshot(2, h4=2), False),
                ('r22d81', snapshot(2, h6=1), True), ('h4', snapshot(2, h4=1, h6=1), False),
                ('h4', snapshot(2, h4=1, h2=1), True), ('h4', snapshot(1, h2=1), False),
                ('high', snapshot(1, h6=1), True), ('high', snapshot(1, h8=1, h4=1), False),
                ('tail', snapshot(1, h2=1), True), ('tail', snapshot(0), False),
                ('lattice', snapshot(0), True), ('lattice', snapshot(1, h2=1), False)):
            with self.subTest(name=name, heights=snap_.heights()):
                self.assertIs(C.entry(name, snap_), expected)

    def test_v3_fallback_step(self):
        """Development mode plays a fallback step, and the verdict is coverage_failure."""
        base, rec = self.v3_case('short')
        rec['steps'][0].update(phase='fallback', rule='fallback', standing='fallback', contract='fallback')
        rec['totals'].update(tail_twists=0, fallback_twists=1)
        rec['acceptance'].update(fallback_steps=1, verdict='coverage_failure')
        final = self.check(base, rec)
        self.assertEqual((final['verdict'], final['fallback_twists']), ('coverage_failure', 1))
        rec['config'] = blank3(rec['start'], 'acceptance')['config']
        self.check(base, rec, 'standing')

    def test_v3_probe(self):
        """The probe word is replayed on a copy of the end state under the fallback contract:
        a non-empty word lands a unit that is off at the end state."""
        base, original = self.probe_case()
        final = self.check(base, original)
        self.assertEqual((final['verdict'], final['end_digest']), ('coverage_failure', original['start']['digest']))
        word = read(base / 'probe/word.json')
        write(base / 'probe/empty.json', {'word': [], 'input_digest': word['input_digest'],
                                          'output_digest': word['input_digest']})
        table = read(base / 'start-table.json')
        write(base / 'probe/lattice-unit.json', {'pieces': [table['pose_id'].index(0)]})
        for label, edit, failure in (
                ('output digest', lambda p, rec: p.update(output_digest='0' * 64), 'probe'),
                ('unit not landed', lambda p, rec: p.update(word='probe/empty.json', letters=0,
                                                             output_digest=word['input_digest']), 'probe'),
                ('unit on the lattice', lambda p, rec: p.update(unit='probe/lattice-unit.json'), 'probe'),
                # The review's counterexample (I0-04): an on-lattice unit and the empty word.
                ('empty word, unit on the lattice',
                 lambda p, rec: p.update(unit='probe/lattice-unit.json', word='probe/empty.json', letters=0,
                                         output_digest=word['input_digest']), 'probe'),
                ('letters', lambda p, rec: p.update(letters=2), 'probe'),
                ('missing unit', lambda p, rec: p.update(unit='probe/none.json'), 'probe'),
                ('development mode', lambda p, rec: rec.update(config=blank3(rec['start'])['config']), 'probe'),
                ('not counted', lambda p, rec: rec['acceptance'].update(probe=False), 'acceptance')):
            with self.subTest(label=label):
                rec = copy.deepcopy(original)
                edit(rec['stuck']['probe'], rec)
                self.check(base, rec, failure)

    def test_v3_played_lattice(self):
        base, rec, after = self.played_case()
        self.assertEqual(after['digest'], C.SOLVED)
        final = self.check(base, rec)
        self.assertEqual((final['end_digest'], final['lattice_twists'], final['lattice_checked'], final['verdict'],
                          final['unchecked_contracts']), (C.SOLVED, 3, False, 'solved', 0))
        original = rec
        for label, edit, failure in (
                ('labels not identity', lambda s: s.update(labels_identity=False), 'labels_identity'),
                ('labels missing', lambda s: s.pop('labels_identity'), 'labels_identity'),
                ('json encoding', lambda s: s.update(encoding='json'), 'encoding'),
                ('word sha256', lambda s: s.update(word_sha256='0' * 64), 'word_sha256'),
                ('contract', lambda s: s.update(contract='strict_prime'), 'contract')):
            with self.subTest(label=label):
                rec = copy.deepcopy(original)
                edit(rec['steps'][0])
                self.check(base, rec, failure)
        with patch.object(C.check_lattice, 'replay_lattice_step', lambda *a: 1 / 0):
            self.check(base, original, 'lattice_step')
        # The plugin's report must agree with the replay and the frozen configuration, field by field,
        # with its hash refreshed each time (I1R-01).
        start, total = original['steps'][0]['before']['digest'], original['phases']['lattice']['twists']
        restorer = original['config']['lattice']['restorer']
        report = base / 'lattice/report.json'
        saved = report.read_bytes()

        def field(*keys, value):
            def edit(doc):
                for key in keys[:-1]:
                    doc = doc[key]
                doc[keys[-1]] = value
            return edit

        for label, edit in (('restorer', field('restorer', value='schedule11' if restorer != 'schedule11' else 'late')),
                            ('input digest', field('player', 'input', 'digest', value=C.SOLVED)),
                            ('final digest', field('player', 'final', 'digest', value=start)),
                            ('final labels', field('player', 'final', 'labels_identity', value=False)),
                            ('result', field('player', 'result', value='failed')),
                            ('failure', field('player', 'failure', value='x')),
                            ('played split', field('player', 'played', 'corrections', value=1)),
                            ('played total', field('player', 'played', 'total', value=total + 1)),
                            ('scheduled', field('player', 'scheduled', 'corrections', value=1))):
            with self.subTest(report=label):
                rec = copy.deepcopy(original)
                rec['phases']['lattice']['report'] = write_report(base, restorer, start, C.SOLVED, total, edit=edit)
                self.check(base, rec, 'lattice_report')
        report.write_bytes(saved)
        # An identical copy under another name has the same hash, but it is not the file the runner writes.
        shutil.copyfile(report, base / 'lattice/alternate.json')
        for label, edit in (('hash', lambda r: r.update(sha256='0' * 64)),
                            ('file', lambda r: r.update(file='lattice/none.json')),
                            ('alternate copy', lambda r: r.update(file='lattice/alternate.json')),
                            ('extra key', lambda r: r.update(extra=1)),
                            ('no report', None)):
            with self.subTest(report=label):
                rec = copy.deepcopy(original)
                if edit is None:
                    rec['phases']['lattice'].pop('report')
                else:
                    edit(rec['phases']['lattice']['report'])
                self.check(base, rec, 'lattice_report')
        (base / 'lattice/alternate.json').unlink()
        # The last lattice row's table must hold the replayed end state (I1R-02).
        table = base / 'tables/00000.json'
        saved = table.read_bytes()
        st = sim.State(restore.ctx, menu=self.menu)
        apply(st, self.menu, self.played_letters)
        self.export(st, base / 'tables/start.json')
        for label, edit in (('missing', lambda s: s.pop('table')),
                            ('another path', lambda s: s.update(table='tables/start.json'))):
            with self.subTest(table=label):
                rec = copy.deepcopy(original)
                edit(rec['steps'][0])
                self.check(base, rec, 'table')
        for label, content in (('another state', (base / 'tables/start.json').read_bytes()), ('malformed', b'{}'),
                               ('truncated', saved[:20]), ('deleted', None)):
            with self.subTest(table=label):
                if content is None:
                    table.unlink()
                else:
                    table.write_bytes(content)
                try:
                    self.check(base, original, 'table')
                finally:
                    table.write_bytes(saved)
        self.check(base, original)

    def test_v3_lattice_labels_not_digest(self):
        """A played lattice is solved by identity labels that the checker replays itself, and its end
        digest is the replayed one (R3-05). Here a lattice configuration one letter short: the row's
        claim of identity labels fails. The case where the digest differs from SOLVED by a rotation
        the labels do not see is test_runner.py's test_w9_invisible_rotation, through the real plugin."""
        base, rec, after = self.played_case(prefix=2)
        self.assertEqual(after['off'], 0)
        self.assertNotEqual(after['digest'], C.SOLVED)
        final = self.check(base, rec, 'labels_identity')
        self.assertEqual((final['end_digest'], final['unchecked_contracts']), (after['digest'], 0))

    def lattice_end_case(self, end, result, verdict, attempts=(), label=None):
        """A lattice End recorded at the played fixture's start: nothing played (revision 3e, E2)."""
        base, rec = self.case('played', label or '%s-%s-%d' % (end, verdict, len(attempts)))
        start = snap(rec['start'])
        rec['phases']['lattice'] = {'result': end, 'steps': 0, 'twists': None, 'reason': 'restorer_no_solution',
                                    'details': {}, 'attempts': [dict(a) for a in attempts]}
        if end == 'stuck':
            rec['stuck'] = {**start, 'phase': 'lattice', 'reason': 'restorer_no_solution', 'units': []}
        rec['result'] = result
        rec['error'] = 'lattice phase error: player_failed' if result == 'error' else None
        rec['acceptance'].update(verdict=verdict, timed_out_attempts=sum(a['timed_out'] for a in attempts))
        return base, rec

    def test_v3_lattice_ends(self):
        """A recorded lattice End passes with the run result it implies; the verdict counts its attempts."""
        clock = {'rule': 'lattice/late', 'outcome': 'timeout', 'reason': 'lattice_wall_clock', 'timed_out': True}
        for end, result, verdict, attempts in (('stuck', 'stuck', 'stuck', ()), ('stuck', 'stuck', 'budget', (clock,)),
                                               ('timeout', 'budget', 'budget', (clock,)),
                                               ('budget', 'budget', 'budget', ()), ('error', 'error', 'error', ())):
            with self.subTest(end=end, attempts=len(attempts)):
                base, rec = self.lattice_end_case(end, result, verdict, attempts)
                final = self.check(base, rec)
                self.assertEqual((final['verdict'], final['lattice_twists'], final['end_digest']),
                                 (verdict, None, rec['start']['digest']))
        base, original = self.lattice_end_case('stuck', 'stuck', 'stuck', label='mutations')
        for label, edit, failure in (
                ('run result', lambda r: r.update(result='budget'), 'result'),
                ('verdict without the clock', lambda r: (r['phases']['lattice']['attempts'].append(clock),
                                                         r['acceptance'].update(timed_out_attempts=1)), 'verdict'),
                ('twists', lambda r: r['phases']['lattice'].update(twists=0), 'phase_twists'),
                ('claimed lattice', lambda r: r['phases']['lattice'].update(result='lattice'), 'phase_result'),
                ('not on the lattice', lambda r: r['phases']['lattice'].update(result='handoff'), 'phase_result')):
            with self.subTest(label=label):
                rec = copy.deepcopy(original)
                edit(rec)
                self.check(base, rec, failure)
        # An End after played rows is not a recorded End: a played phase is solved (K1).
        base, rec, _ = self.played_case()
        rec['phases']['lattice'].update(result='stuck')
        rec['result'] = rec['acceptance']['verdict'] = 'stuck'
        rec['stuck'] = {**snap(rec['steps'][-1]['after']), 'phase': 'lattice', 'reason': 'x', 'units': []}
        self.check(base, rec, 'phase_result')

    def test_v3_lattice_with_no_step(self):
        """Nothing scheduled: solved only when the checker's own state has identity labels (revision 3e, E2)."""
        def no_step(base, rec, total=0, labels=True):
            digest = (rec['steps'][-1]['after'] if rec['steps'] else rec['start'])['digest']
            report = write_report(base, rec['config']['lattice']['restorer'], digest, digest, total, labels=labels)
            rec['phases']['lattice'] = {'result': 'solved', 'steps': 0, 'twists': 0, 'report': report}
            rec['totals']['lattice_twists'] = 0
            return rec

        base, rec = self.v3_case(label='no-step')
        final = self.check(base, no_step(base, rec))
        self.assertEqual((final['verdict'], final['lattice_twists'], final['end_digest']), ('solved', 0, C.SOLVED))
        for label, args, failure in (('scheduled', {'total': 1}, 'lattice_report'),
                                     ('report labels', {'labels': False}, 'lattice_report')):
            with self.subTest(label=label):
                base, rec = self.v3_case(label=label.replace(' ', '-'))
                self.check(base, no_step(base, rec, **args), failure)
        # An identical copy of the report under another name is not the file the runner writes (I1R-01).
        base, rec = self.v3_case(label='alternate-copy')
        rec = no_step(base, rec)
        shutil.copyfile(base / 'lattice/report.json', base / 'lattice/alternate.json')
        rec['phases']['lattice']['report']['file'] = 'lattice/alternate.json'
        self.check(base, rec, 'lattice_report')
        # At the played fixture's start the labels are not the identity.
        base, rec = self.case('played', 'labels')
        rec['result'] = rec['acceptance']['verdict'] = 'solved'
        self.check(base, no_step(base, rec), 'labels_identity')

    def test_v3_high_phase_bound(self):
        """The kernel's letter bound over the whole high phase, from the checker's own start (P4-falsepass-02)."""
        base, rec = self.case('h46', 'bound')
        st = self.h46_state()
        start = state_record(st)
        word = [[12, 14]] * 7
        apply(st, self.menu, word)
        after = state_record(st)
        write(base / 'words/00000.json', {'word': word, 'input_digest': start['digest'], 'output_digest': after['digest']})
        rec['steps'] = [step3(base, {'index': 0, 'phase': 'high', 'rule': 'r22d25/F1', 'word': 'words/00000.json',
                                     'letters': 7, 'contract': 'kernel_landing', 'gate': 'r22d25',
                                     'before': start, 'after': after}, word)]
        rec['phases'] = phases3(rec['phases']['bulk'], [start] * 4 + [after] * 2,
                                ['stuck', 'stuck', 'not_applicable', 'stuck', 'stuck'], {4: (1, 7)})
        rec['stuck'] = {**after, 'phase': 'tail', 'reason': 'no_rule', 'units': []}
        rec['totals']['high_twists'] = 7
        write(base / 'run.json', rec)
        # With ST_LIMIT 0 the bound is 6 n6 = 6 letters at h46's start: 7 is one over, 6 is within.
        for limit, over in ((0, True), (1, False)):
            with self.subTest(limit=limit), patch.object(C.check_contracts, 'ST_LIMIT', limit):
                final = C.check_run(base, lambda row: None)
                self.assertIs('kernel_letter_bound' in final['fails'], over, final['fails'])
        with patch.object(C.check_contracts, 'check_high_phase', lambda *a: 1 / 0):
            self.assertIn('contract', C.check_run(base, lambda row: None)['fails'])

    def test_v3_lattice_composition(self):
        """Until I1 the lattice is checked by composition: verify_late, a component, no end digest."""
        base, old = self.case('lattice_v2')
        start = snap(old['start'])
        rec = blank3(old['start'])
        rec['phases'] = phases3(old['phases']['bulk'], [start] * 6, ['not_applicable'] * 5)
        twists = old['lattice']['twists']
        rec['phases']['lattice'] = {'result': 'solved', 'steps': 0, 'twists': twists}
        rec['config']['component'] = 'lattice_composition'
        rec['lattice'], rec['result'] = old['lattice'], 'solved'
        rec['totals']['lattice_twists'] = twists
        rec['acceptance']['verdict'] = 'component'
        parent = self.tmp / 'v3-check-output'
        parent.mkdir()
        final = self.check(base, rec, report_parent=parent)
        self.assertEqual((final['lattice_checked'], final['end_digest'], final['verdict'], final['lattice_twists']),
                         (True, None, 'component', twists))
        rec['config']['component'] = None
        rec['acceptance']['verdict'] = 'solved'
        self.check(base, rec, 'component')


@unittest.skipUnless((HERE / 'runner.py').is_file(), 'runner.py is not present')
class RunnerRecordTests(unittest.TestCase):
    """Format 3 records written by the real runner.py and its plugins, checked as written."""
    RUNS = {'acceptance': ['--scramble', '10', '7001', '--mode', 'acceptance'],
            'development': ['--scramble', '10', '7001'],
            # Z36 (R22D-36) with no bulk step and no tail rule: the height dispatcher (P1) runs.
            'dispatcher': ['--table', 'z36.json', '--bulk-steps', '0', '--tail-rules', '', '--fallback-max-length', '0']}

    @classmethod
    def setUpClass(cls):
        cls.tmp = Path(tempfile.mkdtemp(prefix='tutorial-check-records-'))
        cls.addClassCleanup(shutil.rmtree, cls.tmp)
        cache_env = patch.dict(os.environ, {'JUMBLING_HOME_CACHE': str(cls.tmp / 'home-centroids.npy')})
        cache_env.start()
        cls.addClassCleanup(cache_env.stop)
        cls.worktree_before = status()
        menu = restore.menu('I_a')
        st = sim.State(restore.ctx, menu=menu)
        apply(st, menu, WORD36)
        import buffer                           # fixture construction only, as in CheckRunTests
        buffer.export_pose_table(st, cls.tmp / 'z36.json')
        started, children = time.monotonic(), {}
        for name, arguments in cls.RUNS.items():
            cache = cls.tmp / ('cache-' + name)
            arguments = [str(cls.tmp / a) if a == 'z36.json' else a for a in arguments]
            log = (cls.tmp / (name + '.log')).open('w')
            cls.addClassCleanup(log.close)
            children[name] = subprocess.Popen(
                [sys.executable, '-I', '-B', str(HERE / 'runner.py'), '--out', str(cls.tmp / name),
                 '--cache', str(cache), *arguments], cwd=REPO, stdout=log, stderr=subprocess.STDOUT,
                env={**os.environ, 'JUMBLING_HOME_CACHE': str(cache / 'home_centroids.npy')}, start_new_session=True)
        try:
            for child in children.values():
                child.wait(timeout=max(1, 1800 - (time.monotonic() - started)))
        finally:
            # The runner cancels its own child groups on SIGTERM (test_runner.py does the same).
            for child in children.values():
                if child.poll() is None:
                    child.terminate()
                    try:
                        child.wait(timeout=60)
                    except subprocess.TimeoutExpired:
                        child.kill()
                        child.wait()
        for name, child in children.items():
            if child.returncode:
                raise AssertionError('runner %s failed: %s' % (name, (cls.tmp / (name + '.log')).read_text()[-3000:]))
        print('runner records: %.2fs' % (time.monotonic() - started), flush=True)

    @classmethod
    def tearDownClass(cls):
        if status() != cls.worktree_before:
            raise AssertionError('test run changed the worktree, including ignored files')

    def run_copy(self, name):
        base = self.tmp / ('%s-%s' % (self._testMethodName, name))
        shutil.copytree(self.tmp / name, base)
        return base, read(base / 'run.json')

    def check(self, base, rec, failure=None, **kwargs):
        write(base / 'run.json', rec)
        rows = []
        final = C.check_run(base, rows.append, **kwargs)
        self.assertEqual(len(rows), len(rec['steps']) + 1, rows)
        self.assertIs(final['pass'], failure is None, rows)
        if failure is not None:
            self.assertIn(failure, final['fails'])
        return final

    def test_scramble_records(self):
        for name in ('acceptance', 'development'):
            with self.subTest(mode=name):
                base, rec = self.run_copy(name)
                self.assertEqual((rec['format'], rec['config']['mode'], rec['result']), ('runner-run/3', name, 'solved'))
                self.assertTrue(rec['steps'] and all(s['phase'] == 'bulk' for s in rec['steps']))
                final = self.check(base, rec)
                self.assertEqual((final['verdict'], final['end_digest'], final['unchecked_contracts']),
                                 ('solved', C.SOLVED, 0))
                self.assertEqual(final['bulk_twists'], rec['totals']['bulk_twists'])

    def test_scramble_record_cli(self):
        base, _ = self.run_copy('acceptance')
        output = self.tmp / 'acceptance-check.jsonl'
        proc = subprocess.run([sys.executable, '-I', '-B', str(HERE / 'check_run.py'), str(base), '--out', str(output)],
                              cwd=REPO, capture_output=True, text=True, timeout=600, check=False)
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertEqual(proc.stdout, output.read_text())
        self.assertEqual(json.loads(proc.stdout.splitlines()[-1])['verdict'], 'solved')

    def test_dispatcher_record(self):
        """At R22D-36 r22d75 plays (12, 14) and then (35, 13) to HEIGHT2 (3ab25d21...); r22d81, h4 and high
        are not applicable (n4 = n6 = 0); the tail has no rule and is stuck. The checker recomputes every
        entry test and checks the height contracts and the trees (plan revision 3d, section 4). With
        --bulk-steps 0 the bulk ends on its step budget, so the run is a component and its verdict says so."""
        base, original = self.run_copy('dispatcher')
        self.assertEqual((original['config']['component'], original['result']), ('bulk_budget', 'stuck'))
        sub = original['phases']['height']['sub']
        self.assertEqual([sub[k]['result'] for k in SUBPHASES], ['handoff', 'not_applicable', 'not_applicable'])
        self.assertEqual((original['phases']['high']['result'], original['phases']['tail']['result']),
                         ('not_applicable', 'stuck'))
        self.assertEqual([(s['rule'], read(base / s['word'])['word']) for s in original['steps']],
                         [('r22d75/T0', [[12, 14]]), ('r22d75/T1', [[35, 13]])])
        end = original['steps'][-1]['after']
        self.assertTrue(end['digest'].startswith('3ab25d21'))
        self.assertEqual((end['off'], end['heights']), (5405, {'2': 5405}))
        final = self.check(base, original)
        self.assertEqual((final['verdict'], final['end_digest'], final['unchecked_contracts'], final['height_twists']),
                         ('component', end['digest'], 0, 2))
        for label, edit, failure in (
                ('verdict', lambda rec: rec['acceptance'].update(verdict='solved'), 'verdict'),
                ('r22d81 entered at height 2', lambda rec: rec['phases']['height']['sub']['r22d81'].update(
                    result='stuck'), 'entry_test'),
                ('phase order', lambda rec: rec['config'].update(phase_order=['bulk', 'high', 'height', 'tail',
                                                                              'lattice']), 'phase_order'),
                ('tree', lambda rec: rec['steps'][0]['tree'].update(sha256='0' * 64), 'tree_sha256')):
            with self.subTest(label=label):
                rec = copy.deepcopy(original)
                edit(rec)
                self.check(base, rec, failure)

    def test_bulk_order_reopen(self):
        base, original = self.run_copy('development')
        for change in ('config', 'step'):
            with self.subTest(change=change):
                rec = copy.deepcopy(original)
                if change == 'config':
                    rec['config']['bulk_order'].insert(0, 'reopen')
                else:
                    rec['steps'][-1]['rule'] = 'reopen'
                self.check(base, rec, 'bulk_order')


class IndependenceTests(unittest.TestCase):
    """Plan runner-run/3, section 0: the checker side shares only the definition of the puzzle."""
    FILES = ('check_run.py', 'check_contracts.py', 'check_lattice.py')
    # Imports outside the standard library: numpy, the simulator (sim/), the I_a menu (restore),
    # the trees, N's cut geometry (rim) and the checker's own modules.
    ALLOWED = {'numpy', 'sim', 'restore', 'trees', 'rim', 'check_contracts', 'check_lattice'}
    # No runner, rule, gate, player or verifier code (verify_late.py runs only as a child process).
    FORBIDDEN = ('runner', 'rules_', 'lattice_phase', 'literal_lattice', 'eb6', 'seams', 'orient', 'buffer',
                 'r22d25_', 'verify_late')
    # restore's menu, element and scramble (the owner's list), and three names restore re-exports
    # from the simulator: ctx is rim.ctx = sim.get_context(), to_tuple is sim.kplus.to_tuple, and
    # transpose is exact.transpose, the exact arithmetic that sim/ itself imports.
    RESTORE = {'menu', 'element', 'scramble', 'ctx', 'to_tuple', 'transpose'}
    RIM = {'Domains'}                           # N = rim.Domains(state).total_misalignment()

    def scan(self, source):
        problems, bases = [], set()
        tree = ast.parse(source)
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                names = [alias.name for alias in node.names]
            elif isinstance(node, ast.ImportFrom):
                names = [node.module or '']
                if node.level:
                    problems.append('relative import')
                if names[0] in ('restore', 'rim'):
                    allowed = self.RESTORE if names[0] == 'restore' else self.RIM
                    problems += ['%s.%s' % (names[0], a.name) for a in node.names if a.name not in allowed]
            else:
                names = []
            for name in names:
                top = name.split('.')[0]
                if top.startswith(self.FORBIDDEN):
                    problems.append('import ' + name)
                elif top not in sys.stdlib_module_names and top not in self.ALLOWED:
                    problems.append('unlisted import ' + name)
            if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name):
                bases.add(id(node.value))
                allowed = {'restore': self.RESTORE, 'rim': self.RIM}.get(node.value.id)
                if allowed is not None and node.attr not in allowed:
                    problems.append('%s.%s' % (node.value.id, node.attr))
            if isinstance(node, ast.Name) and node.id in ('__import__', 'importlib', 'runpy', 'exec', 'eval'):
                problems.append('dynamic code: ' + node.id)
        # The modules themselves are never passed on, so no other attribute can be reached.
        problems += ['%s used as a value' % node.id for node in ast.walk(tree)
                     if isinstance(node, ast.Name) and node.id in ('restore', 'rim') and id(node) not in bases]
        return problems

    def test_checker_side_independent(self):
        for name in self.FILES:
            with self.subTest(file=name):
                self.assertEqual(self.scan((HERE / name).read_text()), [])

    def test_scan_finds_dependencies(self):
        for source in ('import runner_api', 'import runner', 'from seams import SeamRestorer', 'import buffer as b',
                       'from rules_height import STUB', 'import r22d25_search', 'import literal_lattice',
                       'import lattice_phase', 'import orient', 'import eb6r', 'import verify_late', 'import scipy',
                       'from . import check_run', 'x = restore.jumble_items(m)', 'f(restore)', 'm = rim.ctx',
                       'from restore import NEAR', 'importlib.import_module("runner")', '__import__("orient")'):
            with self.subTest(source=source):
                self.assertTrue(self.scan(source))


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        super().stopTest(test)
        self.stream.writeln('  %.2fs' % (time.monotonic() - self.started))


if __name__ == '__main__':
    unittest.main(verbosity=2, testRunner=unittest.TextTestRunner(verbosity=2, resultclass=TimedResult))
