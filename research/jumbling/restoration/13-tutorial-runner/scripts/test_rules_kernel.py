"""P2 runner-side acceptance; exact synthetic geometry, headless research only.

Run at the repository root with python3 -I -B. KERNEL_SLOW=1 additionally runs
the real D19 at the state after the F1 step (not part of default acceptance).
All generated inputs, outputs and caches live in temporary storage.
"""
import os
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[name] = '1'
TEMP = Path(tempfile.mkdtemp(prefix='tutorial-kernel-tests-'))
CACHE = TEMP / 'cache'
os.environ['JUMBLING_HOME_CACHE'] = str(CACHE / 'home_centroids.npy')

import ast
import copy
import hashlib
import importlib.util
import json
import math
import shutil
import subprocess
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location('p2_tutorial_runner', HERE / 'runner.py')
R = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = R
spec.loader.exec_module(R)
import rules_kernel as K
import runner_api as api

KERNEL = Path('research/jumbling/restoration/12-low-height/scripts/round22/r22d25_search.py')
D19 = KERNEL.with_name('r22d19_rule.py')
CATALOGUE = 'r22d25-cat/1'
LIMITS = {'direct': 244, 'S': 244, 'T': 720, 'ST': 964}
FAMILIES = ('F1', 'F2', 'F3', 'd19', 'S', 'ST', 'T1')
COPIED = ('family', 'kind', 'target', 'params', 'high_exact', 'off_before', 'off_after', 'heights_after')
ATTEMPT_KEYS = {'rule', 'catalogue', 'outcome', 'reason', 'timed_out', 'seconds',
                'kernel_seconds', 'ledger_status', 'stop_family'}
Z36 = '8733a0aafd8e24ea1da990ebab0347074c20b11cf61d9ed04d34d25ef6f67261'


def setUpModule():
    # These acceptance-mode cases use test-only options (no bulk step, no lattice). The frozen
    # acceptance configuration (plan revision 3e, E3) is tested in test_runner.py.
    frozen = patch.object(R, 'acceptance_config', lambda args: None)
    frozen.start()
    unittest.addModuleCleanup(frozen.stop)


END36 = 'd79c03679e3b065164219ebd747cc2de89bc5759c081f7b95e9554cbb8c11ea1'
WORD36 = [[0, 12], [74, 8], [188, 4], [74, 4], [188, 8], [188, 4], [188, 4],
          [74, 8], [188, 8], [74, 4], [188, 8], [178, 1], [188, 4], [74, 8],
          [188, 4], [74, 4], [188, 8], [188, 8], [188, 4], [74, 8], [188, 8],
          [74, 4], [178, 1], [0, 45], [14, 12], [12, 12]]


class FakeState:
    """Off pieces with heights; the empty state has the runner's solved digest."""
    def __init__(self, off=(1, 2), heights=None):
        self.off = set(off)
        self.height = {p: h for p, h in (heights or {}).items() if p in self.off and h != 2}
        self.journal = []

    def digest(self):
        if not self.off:
            return R.SOLVED
        return hashlib.sha256(json.dumps([sorted(self.off), sorted(self.height.items())]).encode()).hexdigest()

    def off_lattice_count(self):
        return len(self.off)


class FakeMechanics:
    def __init__(self, start):
        self.start = start

    def scramble(self, k, seed):
        return FakeState(self.start.off, self.start.height)

    def load(self, path):
        doc = R.read_json(path)
        return FakeState(doc['off_pieces'], {int(p): h for p, h in doc['heights'].items()})

    def export(self, state, path):
        R.atomic_json(path, {'digest': state.digest(), 'off_pieces': sorted(state.off),
                             'heights': {str(p): h for p, h in sorted(state.height.items())}})

    def heights(self, state):
        histogram = {}
        for p in state.off:
            h = str(state.height.get(p, 2))
            histogram[h] = histogram.get(h, 0) + 1
        return {h: histogram[h] for h in sorted(histogram)}

    def snapshot(self, state):
        return {'digest': state.digest(), 'off': len(state.off), 'heights': self.heights(state)}

    def play(self, state, word):
        for p, item in word:
            if item == 0:
                continue
            if p not in state.off or item not in (1, 2, 4, 6):
                raise ValueError('inadmissible fake letter')
            if item == 1:
                state.off.remove(p)
            if item in (1, 2):
                state.height.pop(p, None)
            else:
                state.height[p] = item

    def apply_word(self, state, doc):
        doc = R.literal(doc, state.digest())
        self.play(state, doc['word'])
        if state.digest() != doc['output_digest']:
            raise ValueError('word output digest mismatch on own replay')


def unexpected_lattice(ctx):
    raise AssertionError('no test should call the lattice plugin')


def plugins():
    return {'height': SimpleNamespace(STUB=True, SCRIPTS=[],
                                     next_step=lambda ctx: api.End('not_applicable', 'stub')),
            'high': K, 'lattice': SimpleNamespace(SCRIPTS=[], play=unexpected_lattice)}


class TailRecorder(R.Runner):
    """Never runs a real tail rule. Selected fake tests make this tail succeed."""
    def __init__(self, *args, tail_lands=False, **kwargs):
        super().__init__(*args, **kwargs)
        self.tail_calls = 0
        self.tail_lands = tail_lands
        self.proposals = []
        self.answers = []

    def record_step(self, proposal, phase, sub):
        self.proposals.append(proposal)
        return super().record_step(proposal, phase, sub)

    def tail(self):
        self.tail_calls += 1
        if self.tail_lands:
            self.m.apply_word(self.state, {'word': [[p, 1] for p in sorted(self.state.off)],
                                          'output_digest': R.SOLVED})
        data = self.record['phases']['tail']
        data['result'] = 'stuck' if self.state.off_lattice_count() else 'lattice'
        self.record['result'] = 'stuck' if self.state.off_lattice_count() else 'budget'
        self.save()


def ledger(status='exhausted'):
    return {f: {'status': status, 'cursor': None} for f in FAMILIES}


def document(start=None, result='stuck', rows=None, reason=None):
    start = start if start is not None else FakeState(heights={1: 6})
    snap = FakeMechanics(start).snapshot(start)
    doc = {'format': 'r22d25-run/1', 'catalogue': CATALOGUE, 'limits': LIMITS.copy(),
           'start': dict(kind='table', file='start-table.json', **snap), 'steps': [],
           'result': result, 'ledger': ledger() if rows is None else rows,
           'last': snap, 'seconds': 0.125}
    if reason is not None:
        doc['reason'] = reason
    return doc


def step_document(start=None, word=None, result='lattice', family='F1', kind='landing', params=None):
    start = start if start is not None else FakeState(heights={1: 6})
    word = word if word is not None else [[1, 1], [2, 1]]
    after = FakeState(start.off, start.height)
    m = FakeMechanics(start)
    m.play(after, word)
    doc = document(start, result)
    doc['last'] = m.snapshot(after)
    row = {'index': 0, 'word': 'words/00000.json', 'family': family, 'kind': kind,
           'target': [1], 'params': params if params is not None else {'height': 6},
           'high_exact': True, 'letters': len(word), 'input_digest': start.digest(),
           'output_digest': after.digest(), 'off_before': len(start.off), 'off_after': len(after.off),
           'heights_after': m.heights(after), 'seconds': 0.0625}
    doc['steps'] = [row]
    if result == 'budget':
        doc['ledger']['T1'] = {'status': 'budget', 'cursor': {'max_steps': 1, 'step': 1}}
    payload = {'word': word, 'input_digest': row['input_digest'], 'output_digest': row['output_digest']}
    return doc, payload


STUB_SOURCE = '''import argparse, json, sys, tempfile, time
from pathlib import Path
p = argparse.ArgumentParser()
p.add_argument('command', choices=['run'])
for n in ('table', 'out', 'cache', 'seconds', 'd19', 'max-steps'):
    p.add_argument('--' + n, required=True)
a = p.parse_args()
spec = json.loads(Path(SPEC_PATH).read_text())
out = Path(a.out)
Path(CAPTURE_PATH).write_text(json.dumps({'argv': sys.argv[1:], 'out_exists': out.is_dir(),
                                        'out_entries': sorted(x.name for x in out.iterdir())}))
if spec.get('scratch'):
    scratch = Path(tempfile.mkdtemp(prefix='r22d25-d19-', dir=a.cache))
    (scratch / 'table.json').write_text('{}')
if 'raw_record' in spec:
    (out / 'run.json').write_text(spec['raw_record'])
elif spec.get('record') is not None:
    (out / 'run.json').write_text(json.dumps(spec['record']))
if 'raw_word' in spec or spec.get('word') is not None:
    (out / 'words').mkdir(exist_ok=True)
    (out / 'words/00000.json').write_text(spec['raw_word'] if 'raw_word' in spec else json.dumps(spec['word']))
time.sleep(spec.get('sleep', 0))
sys.exit(spec.get('exit', 0))
'''


def case(doc=None, word=None, start=None, options=(), tail_lands=False, **settings):
    base = Path(tempfile.mkdtemp(prefix='case-', dir=TEMP))
    source, capture, spec_path = base / 'kernel.py', base / 'capture.json', base / 'spec.json'
    spec_path.write_text(json.dumps({'record': doc, 'word': word, **settings}))
    source.write_text(STUB_SOURCE.replace('SPEC_PATH', repr(str(spec_path)))
                      .replace('CAPTURE_PATH', repr(str(capture))))
    start = start if start is not None else FakeState(heights={1: 6})
    args = R.parse_args(['--scramble', '0', '7', '--out', str(base / 'run'), '--cache', str(CACHE),
                         '--bulk-steps', '0', *options])
    r = TailRecorder(args, mechanics=FakeMechanics(start), plugins=plugins(), tail_lands=tail_lands)
    return SimpleNamespace(base=base, runner=r, source=source, capture=capture, spec=spec_path)


def observed(r, test):
    """Check that proposing or ending never changes the live state."""
    original = K.next_step

    def call(ctx):
        before = ctx.snapshot()
        answer = original(ctx)
        test.assertEqual(ctx.snapshot(), before)
        test.assertIsInstance(answer, (api.Proposal, api.End))
        r.answers.append(answer)
        return answer
    return call


def run_case(c, test):
    with patch.object(K, 'KERNEL', c.source), patch.object(K, 'next_step', observed(c.runner, test)):
        return c.runner.run()


def public_metadata(test, value):
    """Diagnostics and provenance cannot carry cursor paths or forbidden keys."""
    json.dumps(value, allow_nan=False)
    if isinstance(value, dict):
        test.assertFalse(set(value) & (api.FORBIDDEN_KEYS | {'unit', 'file', 'word', 'table', 'library', 'out'}))
        for child in value.values():
            public_metadata(test, child)
    elif isinstance(value, list):
        for child in value:
            public_metadata(test, child)
    elif isinstance(value, str):
        test.assertFalse(Path(value).is_absolute())


class BMappingTests(unittest.TestCase):
    def assert_end(self, c, result, reason, timed_out=False, stop=None, record_read=True):
        high = c.runner.record['phases']['high']
        self.assertEqual((high['result'], high['reason']), (result, reason))
        self.assertEqual(len(high['attempts']), 1)
        attempt = high['attempts'][0]
        self.assertEqual(set(attempt), ATTEMPT_KEYS)
        self.assertEqual((attempt['rule'], attempt['catalogue']), ('r22d25', CATALOGUE))
        self.assertEqual((attempt['outcome'], attempt['reason']), (result, reason))
        self.assertIs(attempt['timed_out'], timed_out)
        self.assertEqual(attempt['stop_family'], stop)
        self.assertGreaterEqual(attempt['seconds'], 0)
        if not record_read:
            self.assertIsNone(attempt['kernel_seconds'])
            self.assertIsNone(attempt['ledger_status'])
            self.assertIsNone(high['details']['record_sha256'])
        else:
            path = c.runner.out / high['details']['dir'] / 'run.json'
            self.assertEqual(high['details']['record_sha256'], hashlib.sha256(path.read_bytes()).hexdigest())
        public_metadata(self, high['details'])
        public_metadata(self, high['attempts'])
        return high

    def assert_error(self, doc, word=None, reason='kernel_record_invalid', stop=None, **settings):
        c = case(doc, word, **settings)
        rec = run_case(c, self)
        self.assertEqual(rec['result'], 'error')
        self.assertIn(reason, rec['error'])
        self.assertEqual(c.runner.tail_calls, 0)
        self.assert_end(c, 'error', reason, stop=stop, record_read=doc is not None or 'raw_record' in settings)
        return c

    def test_b01_lattice_proposal_and_verbatim_fields(self):
        for family in ('F1', 'F2', 'F3', 'd19', 'T1'):
            with self.subTest(family=family):
                params = {'unit': 1, 'high_exact_pieces': [2]} if family in ('F3', 'd19') else {'height': 6}
                doc, word = step_document(family=family, params=params)
                c = case(doc, word)
                rec = run_case(c, self)
                self.assertEqual(rec['result'], 'solved')
                self.assertEqual(rec['phases']['high']['result'], 'lattice')
                self.assertEqual(len(rec['steps']), 1)
                proposal, row = c.runner.proposals[0], rec['steps'][0]
                self.assertIsInstance(proposal, api.Proposal)
                self.assertEqual(proposal.word, word)
                self.assertEqual((row['phase'], row['subphase'], row['rule']), ('high', None, 'r22d25/' + family))
                self.assertEqual((row['standing'], row['gate'], row['contract']), ('rule', 'r22d25', 'kernel_landing'))
                self.assertEqual(row['attempts'], [])
                self.assertNotIn('height_from', row)
                self.assertEqual({k: row[k] for k in COPIED}, {k: doc['steps'][0][k] for k in COPIED})
                tree, sha = api.read_tree(c.runner.out / row['tree']['file'])
                self.assertEqual(tree, api.single('r22d25/' + family, len(word['word'])))
                self.assertEqual(sha, row['tree']['sha256'])
                kernel = row['kernel']
                self.assertEqual(set(kernel), {'catalogue', 'limits', 'result', 'seconds', 'max_steps',
                                              'd19_sha256', 'record_sha256', 'dir', 'step_seconds', 'call_seconds'})
                self.assertEqual((kernel['catalogue'], kernel['limits'], kernel['result']), (CATALOGUE, LIMITS, 'lattice'))
                self.assertEqual((kernel['seconds'], kernel['max_steps'], kernel['step_seconds']), (3600, 1, 0.0625))
                self.assertEqual(kernel['d19_sha256'], hashlib.sha256((ROOT / D19).read_bytes()).hexdigest())
                self.assertEqual(kernel['record_sha256'], hashlib.sha256(
                    (c.runner.out / kernel['dir'] / 'run.json').read_bytes()).hexdigest())
                self.assertEqual(kernel['dir'], 'attempts/00000-high/kernel')
                self.assertGreater(kernel['call_seconds'], 0)
                public_metadata(self, kernel)

    def test_b02_max_steps_proposal_then_handoff(self):
        doc, word = step_document(word=[[1, 1]], result='budget')
        c = case(doc, word)
        rec = run_case(c, self)
        self.assertEqual(len(rec['steps']), 1)
        self.assertEqual(rec['phases']['high']['result'], 'handoff')
        self.assertEqual(rec['steps'][0]['after']['heights'], {'2': 1})
        self.assertEqual(rec['steps'][0]['kernel']['result'], 'budget')
        self.assertEqual(c.runner.tail_calls, 1)

    def test_b03_st_parts_and_split_validation(self):
        params = {'split': 1, 'S_length': 1, 'T_length': 2, 'middle_digest': 'a' * 64,
                  'T_family': 'T1', 'high_exact_pieces': []}
        doc, word = step_document(word=[[0, 0], [1, 1], [2, 1]], family='ST', params=params)
        c = case(doc, word)
        rec = run_case(c, self)
        self.assertEqual(rec['result'], 'solved')
        row = rec['steps'][0]
        tree, _ = api.read_tree(c.runner.out / row['tree']['file'])
        self.assertEqual(tree, api.flat('r22d25/ST', [('S', 1), ('T', 2)]))
        self.assertEqual([(p['name'], p['span']) for p in tree['parts']], [('S', [0, 1]), ('T', [1, 3])])
        for s, t in ((1, 1), (0, 3), (3, 0), (True, 2), (1, None)):
            with self.subTest(S=s, T=t):
                bad = copy.deepcopy(doc)
                bad['steps'][0]['params'].update(S_length=s, T_length=t)
                self.assert_error(bad, word)

    def test_b04_all_clock_cursors_in_ledger_order(self):
        shapes = [('F1', {'unit': 1, 'grip': 12, 'item': 45}),
                  ('F2', {'unit': 1, 'c': 1, 'x': 2, 'd': 3, 'y': 4, 'z': 5}),
                  ('F3', {'unit': 1, 'a': 2, 'W_length': 1, 'W': [[1, 1]]}),
                  ('d19', {'adapter': 'd19', 'state': 'a' * 64}),
                  ('S', {'unit': 1, 'grip': 2, 'item': 3}),
                  ('T1', {'unit': 1, 'grip': 2, 'item': 3}),
                  ('provider', {'unit': 1, 'proposal': 3})]
        for family, cursor in shapes:
            with self.subTest(family=family):
                rows = ledger('budget')
                for i, row in enumerate(rows.values()):
                    row['cursor'] = ({'phase': 'not_reached', 'state': 'a' * 64},
                                     {'next_unit': 2}, {'phase': 'remaining_targets', 'unit': 1})[i % 3]
                rows[family] = {'status': 'budget', 'cursor': cursor}
                if family in ('S', 'T1'):
                    rows['ST'] = {'status': 'budget', 'cursor': {'unit': 1, 'grip': 1, 'item': 1,
                                                               'middle_digest': 'b' * 64}}
                if family == 'T1':
                    rows['S'] = {'status': 'budget', 'cursor': {'unit': 1, 'grip': 1, 'item': 1}}
                if family == 'provider':
                    rows['T1'] = {'status': 'exhausted', 'cursor': None}
                c = case(document(result='budget', rows=rows))
                rec = run_case(c, self)
                high = self.assert_end(c, 'budget', 'kernel_clock', timed_out=True, stop=family)
                self.assertEqual(rec['result'], 'budget')
                self.assertEqual(c.runner.tail_calls, 0)
                self.assertGreaterEqual(rec['acceptance']['timed_out_attempts'], 1)
                self.assertEqual(high['attempts'][0]['kernel_seconds'], 0.125)
                self.assertEqual(high['attempts'][0]['ledger_status'], {f: r['status'] for f, r in rows.items()})

    def test_b05_inconsistent_budget_and_lattice_shapes(self):
        good, word = step_document(result='budget')
        variants = []
        no_cursor = copy.deepcopy(good)
        no_cursor['ledger'] = ledger()
        variants.append(('step_without_max_steps', no_cursor, word))
        no_step_max = copy.deepcopy(good)
        no_step_max['steps'] = []
        variants.append(('max_steps_without_step', no_step_max, None))
        variants.append(('only_placeholders', document(result='budget', rows={
            f: {'status': 'budget', 'cursor': {'phase': 'not_reached'}} for f in FAMILIES}), None))
        start = document(result='budget')
        start['ledger']['F1'] = {'status': 'budget', 'cursor': {'step': 0, 'phase': 'start'}}
        variants.append(('start_placeholder', start, None))
        variants.append(('no_clock', document(result='budget'), None))
        variants.append(('lattice_without_step', document(result='lattice'), None))
        nonzero, nonzero_word = step_document(word=[[1, 1]])
        variants.append(('lattice_with_off', nonzero, nonzero_word))
        for result in ('budget', 'lattice'):
            two = copy.deepcopy(good)
            two['result'] = result
            two['steps'] *= 2
            variants.append((result + '_two_steps', two, word))
        for name, doc, payload in variants:
            with self.subTest(shape=name):
                self.assert_error(doc, payload)

    def test_b06_exhaustion_requires_the_whole_catalogue(self):
        rows = ledger()
        rows['provider'] = {'status': 'exhausted', 'cursor': None}
        c = case(document(rows=rows))
        rec = run_case(c, self)
        high = self.assert_end(c, 'stuck', 'kernel_exhausted')
        self.assertEqual(rec['result'], 'stuck')
        self.assertEqual(high['details']['catalogue'], CATALOGUE)
        self.assertEqual(rec['acceptance']['timed_out_attempts'], 0)
        for name in ('unfinished', 'with_step', 'missing_ST', 'empty', 'extra_unfinished'):
            with self.subTest(shape=name):
                doc, word = document(), None
                if name == 'unfinished':
                    doc['ledger']['S']['status'] = 'budget'
                elif name == 'with_step':
                    doc, word = step_document(result='stuck')
                elif name == 'missing_ST':
                    del doc['ledger']['ST']
                elif name == 'empty':
                    doc['ledger'] = {}
                else:
                    doc['ledger']['provider'] = {'status': 'budget', 'cursor': {'unit': 1, 'proposal': 0}}
                self.assert_error(doc, word, stop='provider' if name == 'extra_unfinished' else None)

    def test_b07_unsupported_is_error(self):
        for kernel_reason, reason in (('d19_missing', 'd19_missing'), ('height_above_6', 'invariant_height_above_6'),
                                      ('other_reason', 'kernel_unsupported')):
            with self.subTest(reason=kernel_reason):
                c = self.assert_error(document(result='unsupported', reason=kernel_reason), reason=reason)
                self.assertEqual(c.runner.record['phases']['high']['details']['kernel_reason'], kernel_reason)

    def test_b08_exit_missing_and_unreadable_records(self):
        c = self.assert_error(document(result='error', reason='d19_exit'), reason='kernel_error', exit=1)
        self.assertEqual(c.runner.record['phases']['high']['details']['kernel_reason'], 'd19_exit')
        self.assert_error(None, reason='kernel_exit', exit=7)
        start = document(result='budget')
        start['ledger']['F1'] = {'status': 'budget', 'cursor': {'step': 0, 'phase': 'start'}}
        self.assert_error(start, reason='kernel_exit', exit=1)
        self.assert_error(None, reason='kernel_record_missing')
        self.assert_error(None, reason='kernel_record_missing', raw_record='{"format":')
        self.assert_error(None, reason='kernel_exit', raw_record='{', exit=1)

    def test_b08_record_integrity(self):
        variants = [('result', 'unknown'), ('result', 'error'), ('format', 'wrong'), ('catalogue', 'wrong'),
                    ('steps', {}), ('ledger', []), ('seconds', float('nan'))]
        for key, value in variants:
            with self.subTest(key=key, value=value):
                doc = document()
                doc[key] = value
                self.assert_error(doc)
        for name in ('start_digest', 'start_off', 'limits', 'ledger_status', 'ledger_row', 'not_object'):
            with self.subTest(shape=name):
                doc = document()
                if name == 'start_digest':
                    doc['start']['digest'] = 'a' * 64
                elif name == 'start_off':
                    doc['start']['off'] = 123
                elif name == 'limits':
                    doc['limits']['ST'] = 963
                elif name == 'ledger_status':
                    doc['ledger']['F1']['status'] = None
                elif name == 'ledger_row':
                    doc['ledger']['F1'] = 'exhausted'
                else:
                    doc = []
                self.assert_error(doc)

    def test_b08_word_integrity(self):
        doc, word = step_document()
        for name in ('input_digest', 'output_digest', 'length', 'extra_key', 'missing', 'malformed', 'bad_letter'):
            with self.subTest(shape=name):
                bad = copy.deepcopy(word)
                settings = {}
                if name in ('input_digest', 'output_digest'):
                    bad[name] = 'a' * 64
                elif name == 'length':
                    bad['word'].pop()
                elif name == 'extra_key':
                    bad['extra'] = True
                elif name == 'missing':
                    bad = None
                elif name == 'malformed':
                    bad = None
                    settings['raw_word'] = '{'
                else:
                    bad['word'][0] = [600, 1]
                self.assert_error(doc, bad, **settings)

    def test_b09_end_digest_is_checked_by_runner_replay(self):
        doc, word = step_document()
        doc['steps'][0]['output_digest'] = word['output_digest'] = 'a' * 64
        c = case(doc, word)
        rec = run_case(c, self)
        self.assertEqual(rec['result'], 'error')
        self.assertEqual(rec['steps'], [])
        self.assertEqual(len(c.runner.proposals), 1)
        self.assertIsNone(rec['phases']['high']['result'])
        self.assertIsNotNone(rec['phases']['high']['start'])
        self.assertIn('word output digest mismatch on own replay', rec['error'])
        self.assertEqual(c.runner.tail_calls, 0)

    def test_b10_stale_files_cannot_supply_a_step(self):
        c = case()
        r = c.runner
        r.initialize()
        work = r.context('high', None).work_dir
        doc, word = step_document()
        for name in ('kernel', 'kernel-1'):
            old = work / name
            (old / 'words').mkdir(parents=True)
            R.atomic_json(old / 'run.json', doc)
            R.atomic_json(old / 'words/00000.json', word)
        with patch.object(K, 'KERNEL', c.source):
            with self.assertRaisesRegex(RuntimeError, 'kernel_record_missing'):
                r.plugin_phase('high')
        high = self.assert_end(c, 'error', 'kernel_record_missing', record_read=False)
        capture = json.loads(c.capture.read_text())
        self.assertEqual(capture['out_entries'], ['cache'])
        self.assertTrue(capture['out_exists'])
        self.assertEqual(Path(option(capture['argv'], '--out')).name, 'kernel-2')
        self.assertEqual(high['details']['dir'], 'attempts/00000-high/kernel-2')
        self.assertTrue((r.out / 'logs/00000-high-kernel-2.log').exists())
        self.assertEqual(r.record['steps'], [])

    def test_b11_wall_kill_ignores_all_child_files(self):
        for writes in (True, False):
            with self.subTest(start_placeholder=writes):
                doc = document(result='budget') if writes else None
                if writes:
                    doc['ledger']['F1'] = {'status': 'budget', 'cursor': {'step': 0, 'phase': 'start'}}
                c = case(doc, options=('--kernel-seconds', '1', '--kernel-wall-margin', '1'), sleep=600)
                started = time.monotonic()
                rec = run_case(c, self)
                self.assertLess(time.monotonic() - started, 15)
                self.assertEqual(rec['result'], 'budget')
                self.assert_end(c, 'timeout', 'kernel_wall_clock', timed_out=True, record_read=False)
                self.assertGreaterEqual(rec['acceptance']['timed_out_attempts'], 1)
                self.assertEqual(c.runner.tail_calls, 0)

    def test_b12_development_and_acceptance_arguments(self):
        for mode in ('development', 'acceptance'):
            with self.subTest(mode=mode):
                doc, word = step_document()
                c = case(doc, word, options=('--mode', mode, '--kernel-seconds', '3', '--kernel-wall-margin', '2'))
                calls = []
                execute = c.runner.execute

                def capture(argv, tag, seconds, **kwargs):
                    calls.append(([str(x) for x in argv], tag, seconds))
                    self.assertEqual(sorted(p.name for p in Path(option(argv, '--out')).iterdir()), ['cache'])
                    return execute(argv, tag, seconds, **kwargs)

                c.runner.execute = capture
                self.assertEqual(run_case(c, self)['result'], 'solved')
                self.assertEqual(len(calls), 1)
                argv, tag, wall = calls[0]
                self.assertEqual(argv[:5], [sys.executable, '-I', '-B', str(c.source), 'run'])
                self.assertEqual(float(option(argv, '--seconds')), api.CLOCK_SENTINEL if mode == 'acceptance' else 3)
                self.assertEqual(wall, None if mode == 'acceptance' else 5)
                self.assertEqual(option(argv, '--max-steps'), '1')
                self.assertEqual(Path(option(argv, '--d19')), ROOT / D19)
                self.assertEqual(Path(option(argv, '--table')), c.runner.out / 'tables/bulk-end.json')
                self.assertEqual(Path(option(argv, '--cache')), Path(option(argv, '--out')) / 'cache')
                self.assertEqual(tag, '00000-high-kernel')
                saved = json.loads(c.capture.read_text())
                self.assertEqual(saved['argv'], argv[4:])
                self.assertEqual(saved['out_entries'], ['cache'])

    def test_b13_missing_dependency_stops_before_successful_tail(self):
        c = case(document(result='unsupported', reason='d19_missing'), tail_lands=True)
        rec = run_case(c, self)
        self.assertEqual(rec['result'], 'error')
        self.assertIn('high phase error: d19_missing', rec['error'])
        self.assert_end(c, 'error', 'd19_missing')
        self.assertEqual(c.runner.tail_calls, 0)

    def test_b13_genuine_exhaustion_can_continue_to_successful_tail(self):
        c = case(document(), tail_lands=True)
        rec = run_case(c, self)
        self.assertEqual(rec['result'], 'solved')
        self.assert_end(c, 'stuck', 'kernel_exhausted')
        self.assertEqual(c.runner.tail_calls, 1)

    def test_b14_cancel_does_not_read_a_valid_exhaustion_record(self):
        c = case()
        r = c.runner
        r.initialize()

        def cancel(argv, tag, seconds, **kwargs):
            out = Path(option(argv, '--out'))
            R.atomic_json(out / 'run.json', document())
            return R.Child(-1, 0.0, r.out / 'logs' / (tag + '.log'), cancelled=True)

        r.execute = cancel
        with patch.object(K, 'KERNEL', c.source):
            with self.assertRaisesRegex(RuntimeError, 'cancelled'):
                r.plugin_phase('high')
        self.assert_end(c, 'error', 'cancelled', record_read=False)
        self.assertEqual(r.record['steps'], [])

    def test_b15_proposal_metadata_validation(self):
        variants = [('index', 1), ('word', '../words/00000.json'), ('family', 'S'), ('kind', 'unknown'),
                    ('target', []), ('target', [2, 1]), ('target', [True]), ('high_exact', 1),
                    ('off_before', 3), ('input_digest', 'a' * 64), ('off_after', 2),
                    ('letters', 3), ('params', None), ('heights_after', []), ('seconds', float('inf'))]
        for key, value in variants:
            with self.subTest(key=key, value=value):
                doc, word = step_document()
                doc['steps'][0][key] = value
                self.assert_error(doc, word)
        doc, word = step_document(word=[[1, 4]], result='budget', kind='lowering')
        c = case(doc, word, options=('--high-steps', '1'))
        rec = run_case(c, self)
        self.assertEqual(rec['steps'][0]['contract'], 'kernel_lowering')
        self.assertEqual(rec['phases']['high']['reason'], 'high_steps')
        doc['steps'][0]['heights_after']['6'] = 1
        self.assert_error(doc, word)

    def test_b16_private_kernel_reason_is_not_copied(self):
        for reason in ('/private/child/file', 'x' * 65, 'two words', 'bad\\path'):
            with self.subTest(reason=reason):
                c = self.assert_error(document(result='error', reason=reason), reason='kernel_error', exit=1)
                self.assertIsNone(c.runner.record['phases']['high']['details']['kernel_reason'])

    def test_b17_missing_d19_has_null_provenance(self):
        doc, word = step_document()
        c = case(doc, word)
        with patch.object(K, 'D19', c.base / 'missing-d19.py'):
            rec = run_case(c, self)
        self.assertEqual(rec['result'], 'solved')
        self.assertIsNone(rec['steps'][0]['kernel']['d19_sha256'])
        # Outside the repository and missing: neither its path nor a digest is recorded.
        self.assertEqual(rec['config']['kernel']['d19'], {'file': None, 'sha256': None})

    def test_b18_cache_and_interrupted_scratch_are_call_local(self):
        for copy_fallback in (False, True):
            with self.subTest(copy_fallback=copy_fallback):
                c = case(document(), options=('--kernel-seconds', '0.2', '--kernel-wall-margin', '0.2'),
                         scratch=True, sleep=5)
                shared = c.base / 'shared-cache'
                shared.mkdir()
                home = shared / 'home_centroids.npy'
                home.write_bytes(b'fixture cache seed')
                c.runner.cache = shared
                with patch.object(K.os, 'link', side_effect=OSError('fixture link failure')) if copy_fallback else \
                        patch.object(K.os, 'link', wraps=os.link):
                    run_case(c, self)
                self.assert_end(c, 'timeout', 'kernel_wall_clock', timed_out=True, record_read=False)
                argv = json.loads(c.capture.read_text())['argv']
                cache = Path(option(argv, '--cache'))
                self.assertEqual(cache, Path(option(argv, '--out')) / 'cache')
                self.assertTrue(cache.is_relative_to(c.runner.out / 'attempts'))
                self.assertTrue((cache / home.name).is_file())
                self.assertEqual((cache / home.name).read_bytes(), home.read_bytes())
                self.assertEqual(sorted(p.name for p in shared.iterdir()), [home.name])
                self.assertTrue(list(cache.glob('r22d25-d19-*/table.json')))

    def test_b19_main_cleans_temporary_tree_on_success_and_exception(self):
        module = sys.modules[__name__]
        for keep in ('0', '1'):
            for fail in (False, True):
                directory = Path(tempfile.mkdtemp(dir=TEMP, prefix='cleanup-'))
                with self.subTest(keep=keep, fail=fail), \
                        patch.object(module, 'TEMP', directory), \
                        patch.object(module, 'status', return_value=b''), \
                        patch.object(module, 'inventory', return_value={}), \
                        patch.dict(os.environ, KEEP_TEMP=keep), \
                        patch.object(unittest.TextTestRunner, 'run',
                                     side_effect=RuntimeError('fixture run failure') if fail else None,
                                     return_value=SimpleNamespace(wasSuccessful=lambda: True)):
                    if fail:
                        with self.assertRaisesRegex(RuntimeError, 'fixture run failure'):
                            main()
                    else:
                        self.assertEqual(main(), 0)
                    self.assertEqual(directory.exists(), keep == '1')
                shutil.rmtree(directory, ignore_errors=True)


def option(argv, name):
    return argv[argv.index(name) + 1]


class AModuleTests(unittest.TestCase):
    def test_a01_module_is_geometry_free(self):
        self.assertFalse(getattr(K, 'STUB', False))
        code = '''import sys
sys.path.insert(0, sys.argv[1])
import rules_kernel, runner_api
assert not any(n in sys.modules for n in ('numpy', 'sim', 'r22d25_state', 'r22d25_search'))
assert rules_kernel.api is runner_api
'''
        subprocess.run([sys.executable, '-I', '-B', '-c', code, str(HERE)], cwd=ROOT, check=True, timeout=30)
        for node in ast.walk(ast.parse((HERE / 'rules_kernel.py').read_text())):
            if isinstance(node, ast.Import):
                names = [n.name.split('.')[0] for n in node.names]
            elif isinstance(node, ast.ImportFrom):
                names = [(node.module or '').split('.')[0]]
            else:
                continue
            self.assertTrue(set(names) <= sys.stdlib_module_names | {'runner_api'}, names)

    def test_a02_script_provenance_is_read_by_initialize(self):
        required = {HERE.joinpath('rules_kernel.py').relative_to(ROOT), KERNEL, D19,
                    KERNEL.with_name('r22d25_state.py'), KERNEL.parent.parent / 'eb6.py',
                    KERNEL.parent.parent / 'screen.py', *[Path('research/jumbling/theory') / p
                        for p in ('blocks.py', 'classes.py', 'e3_class.py', 'rim.py')]}
        self.assertTrue(required <= set(K.SCRIPTS))
        self.assertEqual(K.KERNEL, KERNEL)
        self.assertEqual(K.D19, D19)
        for path in K.SCRIPTS:
            self.assertFalse(path.is_absolute())
            self.assertTrue((ROOT / path).is_file())
            (ROOT / path).resolve().relative_to(ROOT)
        c = case()
        c.runner.initialize()
        scripts = c.runner.record['config']['scripts']
        for path in K.SCRIPTS:
            self.assertEqual(scripts[path.as_posix()], hashlib.sha256((ROOT / path).read_bytes()).hexdigest())


class Geometry:
    """One real Mechanics and cache, independent constructions of Z36 and END36."""
    def __init__(self):
        started = time.monotonic()
        self.m = R.Mechanics(CACHE)
        self.ztable, self.endtable = TEMP / 'r22d36-table.json', TEMP / 'end36-table.json'
        z = self.m.sim.State(self.m.restore.ctx, menu=self.m.menu)
        self.m.apply_word(z, {'word': WORD36, 'output_digest': Z36})
        actual = self.m.snapshot(z)
        expected = {'digest': Z36, 'off': 5800, 'heights': {'2': 5799, '6': 1}}
        if actual != expected:
            raise AssertionError(('packet section 4.7 disagrees at R22D-36', actual))
        six_poses = [i for i, p in enumerate(z._poses) if p.kidx < 0
                     and self.m.trees.dist(self.m.trees.factors(z._pose_matrix(i))[0]) == 6]
        high = self.m.np.flatnonzero(self.m.np.isin(z.pose_id, six_poses)).tolist()
        if high != [58388]:
            raise AssertionError(('packet section 4.7 height-6 pieces disagree', high))
        self.m.export(z, self.ztable)
        end = self.m.sim.State(self.m.restore.ctx, menu=self.m.menu)
        self.m.apply_word(end, {'word': WORD36, 'output_digest': Z36})
        self.m.apply_word(end, {'word': [[12, 45]], 'output_digest': END36})
        actual = self.m.snapshot(end)
        expected = {'digest': END36, 'off': 3098, 'heights': {'2': 3097, '4': 1}}
        if actual != expected:
            raise AssertionError(('packet section 4.7 disagrees at END36', actual))
        self.m.export(end, self.endtable)
        self.after_d = None
        print(f'Independent R22D-36 and END36 fixtures: {time.monotonic() - started:.3f}s', flush=True)


_geometry = None


def geometry():
    global _geometry
    if _geometry is None:
        _geometry = Geometry()
    return _geometry


def real_runner(g, table, *options):
    base = Path(tempfile.mkdtemp(prefix='real-', dir=TEMP))
    args = R.parse_args(['--table', str(table), '--out', str(base / 'run'), '--cache', str(CACHE),
                         '--bulk-steps', '0', '--no-lattice', *options])
    return TailRecorder(args, mechanics=g.m, plugins=plugins())


class CRealKernelTests(unittest.TestCase):
    def test_c01_finite_budget_sentinel(self):
        spec = importlib.util.spec_from_file_location('p2_budget_oracle', ROOT / KERNEL)
        search = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = search
        spec.loader.exec_module(search)
        budget = search.Budget(api.CLOCK_SENTINEL)
        self.assertTrue(math.isfinite(budget.remaining()))
        self.assertGreater(budget.remaining(), 1e8)
        budget.check('F1', {'unit': 1, 'grip': 0, 'item': 1})
        with self.assertRaises(ValueError):
            search.Budget(float('inf'))

    def test_c02_nested_d19_acceptance_clock_at_end36(self):
        g = geometry()
        base = Path(tempfile.mkdtemp(prefix='d19-clock-', dir=TEMP))
        capture, stub = base / 'argv.json', base / 'd19.py'
        stub.write_text('''import argparse, json, sys
from pathlib import Path
p = argparse.ArgumentParser()
for n in ('table', 'out', 'cache', 'seconds'): p.add_argument('--' + n, required=True)
a = p.parse_args()
Path(CAPTURE).write_text(json.dumps(sys.argv[1:]))
(Path(a.out) / 'report.json').write_text('{"result":"budget"}')
'''.replace('CAPTURE', repr(str(capture))))
        r = real_runner(g, g.endtable, '--mode', 'acceptance')
        execute, calls = r.execute, []

        def guarded(argv, tag, seconds, **kwargs):
            calls.append(([str(x) for x in argv], seconds))
            # Observe the plugin's unlimited wall clock, then bound this test alone.
            return execute(argv, tag, 600, **kwargs)

        r.execute = guarded
        with patch.object(K, 'D19', stub), patch.object(K, 'next_step', observed(r, self)):
            rec = r.run()
        self.assertEqual(rec['result'], 'budget', rec['error'])
        high = rec['phases']['high']
        self.assertEqual((high['result'], high['reason']), ('budget', 'kernel_clock'))
        self.assertEqual(len(high['attempts']), 1)
        attempt = high['attempts'][0]
        self.assertEqual(attempt['stop_family'], 'd19')
        self.assertIs(attempt['timed_out'], True)
        self.assertGreaterEqual(rec['acceptance']['timed_out_attempts'], 1)
        self.assertEqual(len(calls), 1)
        self.assertIsNone(calls[0][1])
        self.assertEqual(float(option(calls[0][0], '--seconds')), api.CLOCK_SENTINEL)
        nested = json.loads(capture.read_text())
        seconds = float(option(nested, '--seconds'))
        self.assertTrue(math.isfinite(seconds))
        self.assertGreater(seconds, 1e8)
        self.assertEqual(r.tail_calls, 0)
        self.assertEqual(rec['steps'], [])
        self.assertEqual(g.m.snapshot(r.state), {'digest': END36, 'off': 3098, 'heights': {'2': 3097, '4': 1}})
        public_metadata(self, high['details'])
        public_metadata(self, high['attempts'])
        print(f'END36 real kernel with D19 budget stub: {attempt["seconds"]:.3f}s; '
              f'nested --seconds={seconds:.3f}', flush=True)

    def test_d_real_f1_at_r22d36(self):
        g = geometry()
        r = real_runner(g, g.ztable, '--high-steps', '1', '--kernel-seconds', '600', '--kernel-wall-margin', '300')
        with patch.object(K, 'next_step', observed(r, self)):
            rec = r.run()
        self.assertNotEqual(rec['result'], 'error', rec['error'])
        self.assertEqual(len(rec['steps']), 1)
        row = rec['steps'][0]
        self.assertEqual((row['phase'], row['rule'], row['family']), ('high', 'r22d25/F1', 'F1'))
        self.assertEqual(row['before']['digest'], Z36)
        self.assertEqual((row['standing'], row['gate'], row['contract']), ('rule', 'r22d25', 'kernel_' + row['kind']))
        word = R.read_json(r.out / row['word'])['word']
        self.assertTrue(word)
        self.assertLessEqual(word[0], [12, 45])
        self.assertEqual(row['off_after'], row['after']['off'])
        self.assertEqual(row['heights_after'], row['after']['heights'])
        self.assertEqual(row['after'], g.m.snapshot(r.state))
        self.assertTrue({p.as_posix() for p in K.SCRIPTS} <= rec['config']['scripts'].keys())
        self.assertEqual(rec['config']['scripts'][D19.as_posix()], hashlib.sha256((ROOT / D19).read_bytes()).hexdigest())
        high = rec['phases']['high']
        if row['kind'] == 'lowering':
            self.assertEqual(row['after']['heights'].get('4', 0), 1)
            self.assertEqual((high['result'], high['reason']), ('budget', 'high_steps'))
            self.assertEqual(rec['result'], 'budget')
            self.assertEqual(r.tail_calls, 0)
        else:
            self.assertEqual(row['kind'], 'landing')
            self.assertEqual(sum(row['after']['heights'].get(h, 0) for h in ('4', '6')), 0)
            self.assertEqual(high['result'], 'handoff')
            self.assertEqual(r.tail_calls, 1)
        if word[0] == [12, 45]:
            self.assertEqual(row['after'], {'digest': END36, 'off': 3098, 'heights': {'2': 3097, '4': 1}})
        g.after_d = (r.table, row['after'])
        print(f'R22D-36 F1 letter={word[0]}, after={row["after"]["digest"]}, '
              f'kernel call={row["kernel"]["call_seconds"]:.3f}s', flush=True)

    @unittest.skipUnless(os.environ.get('KERNEL_SLOW') == '1', 'set KERNEL_SLOW=1 for the real D19 search')
    def test_e_slow_real_d19(self):
        g = geometry()
        table = g.endtable
        if g.after_d and any(g.after_d[1]['heights'].get(h, 0) for h in ('4', '6')):
            table = g.after_d[0]
        r = real_runner(g, table, '--kernel-seconds', '3600', '--kernel-wall-margin', '600', '--high-steps', '1')
        with patch.object(K, 'next_step', observed(r, self)):
            rec = r.run()
        self.assertEqual(len(r.answers), 1)
        self.assertIsInstance(r.answers[0], (api.Proposal, api.End))
        self.assertIsNotNone(rec['phases']['high']['result'], rec['error'])
        high = rec['phases']['high']
        print(f'Slow real D19: high={high["result"]}, reason={high.get("reason")}, '
              f'steps={[(s["rule"], s["letters"]) for s in rec["steps"]]}', flush=True)


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        elapsed = time.monotonic() - self.started
        super().stopTest(test)
        self.stream.writeln(f'  {test.id()}: {elapsed:.3f}s')


# Same exclusions as test_runner.py: Git metadata and concurrent private outputs.
SKIP = ('.git', 'work/codex-sandbox-home', 'work/loop-memory', 'work/reviews', 'work/worktrees')


def inventory():
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


def main(names=()):
    try:
        started = time.monotonic()
        before_status, before_files = status(), inventory()
        module, loader = sys.modules[__name__], unittest.defaultTestLoader
        suite = loader.loadTestsFromNames(names, module) if names else loader.loadTestsFromModule(module)
        result = unittest.TextTestRunner(verbosity=2, resultclass=TimedResult).run(suite)
        after_status, after_files = status(), inventory()
        unchanged = before_status == after_status and before_files == after_files
        if unchanged:
            print('Worktree unchanged, including ignored files.', file=sys.stderr)
        else:
            changed = sorted(p for p in before_files.keys() | after_files.keys() if before_files.get(p) != after_files.get(p))
            print('FAIL: acceptance changed the worktree (including ignored files):', changed, file=sys.stderr)
        elapsed = time.monotonic() - started
        print(f'Acceptance elapsed: {elapsed:.3f}s.', file=sys.stderr)
        if elapsed > 20 * 60 and os.environ.get('KERNEL_SLOW') != '1':
            print('Note: default acceptance took over 20 minutes; see each test duration above.', file=sys.stderr)
        return 0 if result.wasSuccessful() and unchanged else 1
    finally:
        if os.environ.get('KEEP_TEMP') != '1':
            shutil.rmtree(TEMP, ignore_errors=True)


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
