"""P1 acceptance: fixtures, exact geometry, clocks and child failure semantics.

python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/test_rules_height.py
All generated files, including caches and snapshots, live in temporary storage.
"""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import pickle
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

sys.dont_write_bytecode = True
for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[name] = '1'
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
spec = importlib.util.spec_from_file_location('tutorial_runner', HERE / 'runner.py')
R = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = R
spec.loader.exec_module(R)
api = R.api
import rules_height as H
import numpy as np

OUTPUTS = ROOT / 'research/jumbling/restoration/12-low-height/runs/round22'
HEIGHT_OUTPUTS = OUTPUTS / 'cloud/height4-descent-bbef1c8e'
D, E = 'a' * 64, 'b' * 64
Z36 = '8733a0aafd8e24ea1da990ebab0347074c20b11cf61d9ed04d34d25ef6f67261'
S2 = '7cd868f6466953eb6ea1e0d466ed66857d34edaea7dd7c554c1e74975005541b'
S3 = '3ab25d214b6c4ad759ac7158a0fc1cabae1584aa711d3f757af7b91c43971837'
WORD36 = [[0, 12], [74, 8], [188, 4], [74, 4], [188, 8], [188, 4], [188, 4],
          [74, 8], [188, 8], [74, 4], [188, 8], [178, 1], [188, 4], [74, 8],
          [188, 4], [74, 4], [188, 8], [188, 8], [188, 4], [74, 8], [188, 8],
          [74, 4], [178, 1], [0, 45], [14, 12], [12, 12]]
CHECKER = None
REAL_RUNS = []


def lines(path):
    return path.read_text(encoding='utf-8').splitlines(keepends=True)


def event(tag, **payload):
    return tag + ' ' + json.dumps(payload) + '\n'


def start(name, **changes):
    payload = {'digest': D}
    if name == 'r22d75':
        payload['n6_n4_n2'] = [2, 3, 8]
    else:
        payload.update(tau=311, u=2, targets=[[311, 1]])
    payload.update(changes)
    return event('START', **payload)


def step(name, family=None, **changes):
    payload = dict(step=1, digest=E, input_digest=D)
    if name == 'r22d75':
        payload.update(word=[[12, 14]], length=1, n6_n4_n2=[1, 4, 9])
    else:
        payload.update(family=family or 'U', tau=311, u=1)
        if name == 'r22d81_pairs':
            pair = payload['family'] == 'U2'
            payload.update(letters=[[12, 14], [35, 13]] if pair else [[12, 14]],
                           length=2 if pair else 1, u_mid=3 if pair else None)
        else:
            payload['letter'] = [12, 14]
            if name == 'r22d81':
                payload['length'] = 1
    payload.update(changes)
    return event('STEP', **payload)


def end(tag, **changes):
    payload = dict(steps=0, digest=D, tau=311, u=2, targets=[[311, 1]])
    payload.update(changes)
    return event(tag, **payload)


def shard_outputs(n=2, choice=None, by='freeing', caps=(1, 2), class_a=(1,)):
    order = [[c, j] for c in class_a for j in range(1, 60)] + [
             [c, j] for c in caps if c not in class_a for j in range(1, 60)]
    outputs = []
    for shard in range(n):
        text = event('CONTEXT', digest=D, tau=311, u=2, first_letter_caps=list(caps),
                     classA=list(class_a), first_letters=len(order), shard=shard, nshards=n)
        count = 0
        for index, letter in enumerate(order):
            if index % n != shard:
                continue
            payload = dict(index=index, first=letter, cls='A' if letter[0] in class_a else 'B', valid=False)
            if index == choice:
                payload.update(valid=True, u_t=3, freed=[3] if by == 'freeing' else [],
                               successes=[[3, 1, 0], [3, 2, 1]])
                if payload['cls'] == 'B':
                    payload['freed'] = None
            text += event('FIRST', **payload)
            count += 1
        text += event('DONE', first_letters_done=count)
        outputs.append(text)
    return outputs


def checker():
    global CHECKER
    if CHECKER is None:
        spec = importlib.util.spec_from_file_location('height_format_oracle', HERE / 'check_run.py')
        CHECKER = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(CHECKER)
    return CHECKER


def hygiene(test, value):
    test.assertIsInstance(value, (api.Proposal, api.End))
    for attempt in value.attempts:
        test.assertIs(type(attempt['timed_out']), bool)
        test.assertEqual(set(attempt), {'script', 'tag', 'shard', 'outcome', 'reason', 'rc',
                                       'seconds', 'stopped', 'timed_out'})
    c = checker()

    def visit(doc):
        if isinstance(doc, dict):
            test.assertFalse((c.FORBIDDEN | c.PATH_KEYS).intersection(doc))
            for item in doc.values():
                visit(item)
        elif isinstance(doc, (list, tuple)):
            for item in doc:
                visit(item)
        elif isinstance(doc, str):
            test.assertFalse(c.private_path(doc), doc)

    visit(value.attempts)
    if isinstance(value, api.End):
        visit(value.details)
    else:
        visit(value.fields)
        test.assertFalse({'construction', 'labels_identity'}.intersection(value.fields))
        producer = value.attempts[-1]['script']
        test.assertEqual(value.fields['rule_script'], H.RULE_SOURCES[producer].as_posix())
        test.assertEqual(value.gate, H.RULE_SOURCES[producer].stem)
        test.assertEqual(value.contract, value.rule)
        test.assertEqual(value.phase, 'height')
        test.assertEqual(value.standing, 'rule')
        api.check_tree(value.tree or api.single(value.rule, len(value.word['word'])), len(value.word['word']))


class CommittedTests(unittest.TestCase):
    def parsed(self, dialect, path):
        result = H.parse_log(dialect, lines(path))
        self.assertEqual(result['status'], 'end', (path.name, result))
        previous = result['start']['digest']
        for row in result['steps']:
            self.assertEqual(row['input_digest'], previous)
            previous = row['output_digest']
        self.assertEqual(result['end']['payload']['digest'], previous)
        self.assertEqual(result['end']['payload']['steps'], len(result['steps']))
        return result

    def test_01_r22d81_and_pairs(self):
        parsed = self.parsed('r22d81', OUTPUTS / 'r22d81_from_stop.txt')
        self.assertEqual((parsed['start']['tau'], parsed['start']['u'], parsed['start']['digest']),
                         (274, 3, '015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f'))
        expected = [('H6', [64, 15], 274, 3), ('U', [124, 2], 274, 0),
                    ('H6', [274, 13], 104, 30), ('U', [46, 1], 104, 3),
                    ('U', [46, 3], 104, 0), ('H6', [104, 14], None, 0)]
        tau, u = 274, 3
        for row, (family, letter, after_tau, after_u) in zip(parsed['steps'], expected, strict=True):
            self.assertEqual((row['rule'], row['letters']), ('r22d81/' + family, [letter]))
            self.assertEqual([row['fields'][k] for k in ('tau_before', 'tau_after', 'u_before', 'u_after')],
                             [tau, after_tau, u, after_u])
            tau, u = after_tau, after_u
        self.assertEqual(parsed['steps'][3]['input_digest'], parsed['steps'][2]['output_digest'])
        self.assertEqual(parsed['end']['tag'], 'HANDOFF')
        self.assertEqual(parsed['end']['payload']['digest'],
                         'e108778ecb2533df175d8f7c1cf3b35396d4589f4dde86c017fc2688040fe150')
        self.assertEqual(parsed['end']['payload']['n2_n4_n6'], [99701, 1015, 0])
        self.assertEqual(parsed['end']['payload']['off'], 100716)
        for cap, initial_u, first_word, last, final_u in (
                (311, 182, [[185, 1]], 'UNLOCKED', 0), (290, 38, [[136, 1], [301, 1]], 'STUCK', 4)):
            parsed = self.parsed('r22d81_pairs', OUTPUTS / f'r22d81_pairs_cap{cap}.txt')
            self.assertEqual((parsed['start']['tau'], parsed['start']['u']), (cap, initial_u))
            self.assertEqual(parsed['steps'][0]['letters'], first_word)
            self.assertEqual(parsed['end']['tag'], last)
            self.assertEqual(parsed['steps'][-1]['fields']['u_after'], final_u)
            if cap == 290:
                self.assertEqual(parsed['end']['payload']['u'], final_u)
            previous_u = initial_u
            for row in parsed['steps']:
                f = row['fields']
                self.assertEqual((f['tau_before'], f['tau_after'], f['u_before']), (cap, cap, previous_u))
                self.assertEqual(row['rule'], 'r22d88/' + ('U2' if len(row['letters']) == 2 else 'U'))
                previous_u = f['u_after']
            if cap == 311:
                self.assertTrue(parsed['start']['digest'].startswith('1ff0061f'))
                row = parsed['steps'][3]
                self.assertEqual((row['rule'], row['letters'], row['fields']['u_mid'], row['fields']['u_after']),
                                 ('r22d88/U2', [[364, 3], [263, 6]], 8, 2))
                self.assertEqual(parsed['steps'][0]['fields']['u_after'], 152)
            else:
                self.assertEqual((parsed['steps'][0]['fields']['u_mid'], parsed['steps'][0]['fields']['u_after']),
                                 (61, 37))

    def test_02_census_tiers(self):
        expected = [
            ('7304', 'r22d75_census_k3000.txt', '7e2304b2', 'ac127dc3',
             [(170, 12, 1), (395, 12, 1), (500, 12, 1), (540, 12, 1), (217, 15, 2), (148, 15, 2)],
             [0, 797, 95674], 96471),
            ('7305', 'r22d75.txt', '273eab69', 'cc5ef7ec',
             [(10, 12, 1), (10, 15, 1), (67, 14, 1), (134, 13, 1), (195, 14, 1), (233, 15, 1),
              (421, 12, 1), (421, 12, 1), (484, 15, 1), (122, 12, 2), (175, 14, 2)],
             [0, 1419, 108453], 109872),
            ('7306', 'r22d75_census_k3000.txt', '9ebdc490', '16c4aefe',
             [(208, 12, 0), (76, 13, 1), (90, 12, 1), (109, 12, 1), (233, 15, 1), (387, 15, 1),
              (485, 13, 1), (403, 13, 2), (512, 15, 2)], [0, 2591, 108641], 111232),
            ('7307', 'r22d75_census_k3000.txt', 'ba5371ae', '895b45de',
             [(275, 14, 0), (516, 13, 0), (50, 15, 1), (474, 13, 1), (494, 15, 1), (147, 14, 2)],
             [0, 1585, 98984], 100569),
        ]
        for seed, filename, initial, final, moves, counts, off in expected:
            with self.subTest(seed=seed):
                parsed = self.parsed('r22d75', OUTPUTS / f'cloud/census-k3000-s{seed}' / filename)
                self.assertTrue(parsed['start']['digest'].startswith(initial))
                for row, (c, i, tier) in zip(parsed['steps'], moves, strict=True):
                    self.assertEqual((row['rule'], row['letters']), (f'r22d75/T{tier}', [[c, i]]))
                    self.assertTrue(all(row['fields'][k] is None for k in
                                        ('tau_before', 'tau_after', 'u_before', 'u_after')))
                payload = parsed['end']['payload']
                self.assertEqual(parsed['end']['tag'], 'STUCK')
                self.assertTrue(payload['digest'].startswith(final))
                self.assertEqual((payload['n6_n4_n2'], payload['off']), (counts, off))
                if seed == '7304':
                    self.assertEqual(parsed['steps'][4]['rule'], 'r22d75/T2')
                if seed == '7306':
                    self.assertEqual(parsed['steps'][1]['rule'], 'r22d75/T1')

    def test_03_h4_and_crashes(self):
        for number, initial, tau, u, moves, afters, final, target_count in (
                (1, '0ad859c2', 482, 80, [('U', [357, 1]), ('U', [357, 3]), ('U', [593, 3]), ('U', [593, 43])],
                 [(482, 11), (482, 8), (482, 2), (482, 1)], '6999b921', 18),
                (2, '391490f7', 482, 0, [('L4', [482, 15])], [(597, 41)], 'c312a2fc', 17),
                (3, '6c7a79d9', 597, 21, [('U', [535, 6])], [(597, 10)], 'b2651c24', 17),
                (4, '2430f4cc', 597, 8, [], [], '2430f4cc', 17)):
            parsed = self.parsed('h4', HEIGHT_OUTPUTS / f'h4_unlock_run{number}.txt')
            self.assertTrue(parsed['start']['digest'].startswith(initial))
            self.assertEqual((parsed['start']['tau'], parsed['start']['u']), (tau, u))
            for row, (family, letter), (new_tau, new_u) in zip(parsed['steps'], moves, afters, strict=True):
                self.assertEqual((row['rule'], row['letters']), ('h4/' + family, [letter]))
                self.assertEqual([row['fields'][k] for k in ('tau_before', 'tau_after', 'u_before', 'u_after')],
                                 [tau, new_tau, u, new_u])
                tau, u = new_tau, new_u
            payload = parsed['end']['payload']
            self.assertEqual((parsed['end']['tag'], payload['tau'], payload['u']), ('STUCK', tau, u))
            self.assertTrue(payload['digest'].startswith(final))
            self.assertEqual(len(payload['targets']), target_count)
        for filename in ('h4_unlock_run0_prediction_assert.txt', 'h4_unlock_run2_expect_misuse.txt'):
            self.assertEqual(H.parse_log('h4', lines(HEIGHT_OUTPUTS / filename))['status'], 'crash')

    def rounds(self, number):
        pattern = 'two_letter_u_shard*_part*.txt' if number == 1 else f'two_letter_u_round{number}_shard*.txt'
        return [(p, lines(p)) for p in sorted(HEIGHT_OUTPUTS.glob(pattern))]

    def test_04_pair_rounds(self):
        for number, first, second, before_u, after_u, mid, by, total in (
                (1, [541, 2], [593, 54], 1, 0, 1, 'first_success', 2065),
                (2, [553, 5], [535, 3], 41, 21, 41, 'freeing', 1947),
                (3, [363, 10], [535, 8], 10, 8, 10, 'first_success', 1947),
                (4, None, None, 8, None, None, None, 1947)):
            outputs = self.rounds(number)
            self.assertEqual(len(outputs), 9 if number == 1 else 3)
            chosen = H.choose_pair([output for _, output in outputs])
            self.assertEqual(chosen['result'], 'exhausted' if number == 4 else 'step', chosen)
            for (path, _), status in zip(outputs, chosen['outputs'], strict=True):
                self.assertEqual(status, 'done' if number == 4 or '_part3' in path.name else 'truncated')
            if number == 4:
                continue
            row = chosen['step']
            self.assertEqual(row['letters'], [first, second])
            self.assertEqual(row['rule'], 'h4/pair')
            f = row['fields']
            tau = 482 if number == 1 else 597
            self.assertEqual([f[k] for k in ('tau_before', 'tau_after', 'u_before', 'u_after', 'u_mid')],
                             [tau, tau, before_u, after_u, mid])
            self.assertEqual((f['pair_choice']['by'], f['pair_choice']['shards'], f['pair_choice']['first_letters']),
                             (by, 3, total))
            if number == 1:
                contexts = [json.loads(output[0].split(' ', 1)[1]) for _, output in outputs]
                self.assertEqual(contexts[0]['classA'], [357, 370, 541, 552, 590, 591, 593])
                indices = [json.loads(line[6:])['index'] for _, output in outputs
                           for line in output if line.startswith('FIRST ') and line.endswith('\n')]
                self.assertEqual(sorted(indices), list(range(2065)))
            if number == 2:
                context = json.loads(outputs[0][1][0].split(' ', 1)[1])
                self.assertEqual(context['classA'], [407, 471, 492, 523, 533, 553, 590, 591])

    def test_05_missing_and_conflicting_first_lines(self):
        for number, letter in ((1, [357, 1]), (1, [2, 1]), (2, [407, 1])):
            outputs = [output for _, output in self.rounds(number)]
            for output in outputs:
                for i, line in enumerate(output):
                    if line.startswith('FIRST ') and json.loads(line[6:])['first'] == letter:
                        del output[i]
                        break
            self.assertEqual(H.choose_pair(outputs)['result'], 'undecided')
        outputs = [output for _, output in self.rounds(1)]
        row = next(json.loads(line[6:]) for line in outputs[0] if line.startswith('FIRST '))
        row.update(valid=True, u_t=1, freed=[], successes=[[593, 1, 0]])
        outputs[0].append(event('FIRST', **row))
        self.assertEqual(H.choose_pair(outputs)['result'], 'error')
        outputs = [output for _, output in self.rounds(1)]
        i = next(i for i, line in enumerate(outputs[0]) if line.startswith('FIRST '))
        output = outputs[0][:i] + [outputs[0][i][:25]]
        outputs[0] = output
        chosen = H.choose_pair(outputs)
        self.assertEqual(chosen['outputs'][0], 'truncated')
        self.assertEqual(chosen['result'], 'undecided')
        for bad in ('STEP {broken}\n', 'START []\n', 'START {"digest": NaN}\n'):
            self.assertEqual(H.parse_log('r22d81', [bad])['status'], 'malformed')


class TemporaryTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix='height-rules-'))
        self.addCleanup(shutil.rmtree, self.tmp)
        env = patch.dict(os.environ, JUMBLING_HOME_CACHE=str(self.tmp / 'cache/home_centroids.npy'))
        env.start()
        self.addCleanup(env.stop)
        self.counter = 0

    def args(self, out, *extra):
        return R.parse_args(['--scramble', '1', '7', '--out', str(self.tmp / out),
                             '--cache', str(self.tmp / 'cache'), '--bulk-steps', '0',
                             '--pair-shards', '2', '--no-lattice', *extra])


class StubTests(TemporaryTest):
    def stub(self, text='', before='', after=''):
        self.counter += 1
        path = self.tmp / f'stub-{self.counter}.py'
        path.write_text('import sys, os, json, time, signal, resource\n' + before + '\n' +
                        f'sys.stdout.write({text!r}); sys.stdout.flush()\n' + after + '\n')
        return path

    def invoke(self, name, text='', *, before='', after='', overrides=None, extra=(),
               cancel=False, snapshot=None, execute=None, pair_digest=E):
        self.counter += 1
        runner = R.Runner(self.args(f'run-{self.counter}', *extra))
        calls = []
        paths = {
            'r22d75': self.stub(start('r22d75') + end('STUCK')),
            'r22d81': self.stub(start('r22d81') + end('STUCK')),
            'r22d81_pairs': self.stub(start('r22d81_pairs') + end('UNLOCKED')),
            'h4': self.stub(start('h4') + end('STUCK')),
            'two_letter_u': self.stub(before='outputs = ' + repr(shard_outputs()) +
                '\nsys.stdout.write(outputs[int(sys.argv[sys.argv.index("--shard")+1])]); sys.stdout.flush()'),
        }
        paths[name] = self.stub(text, before=before, after=after)
        paths.update(overrides or {})

        def record(argv, tag, seconds, **kwargs):
            calls.append((argv, tag, seconds, kwargs))
            if cancel and Path(argv[4]) == paths[name]:
                runner.cancel.set()
            return (execute or runner.execute)(argv, tag, seconds, **kwargs)

        sub = 'h4' if name == 'two_letter_u' else 'r22d81' if name == 'r22d81_pairs' else name
        state = SimpleNamespace(pose_id=np.array([0], dtype=np.int32),
                                _poses=[SimpleNamespace(kidx=-1, matrix=None, key=('fake',))],
                                digest=lambda: D)
        snap = snapshot or dict(digest=D, off=13, heights={'2': 8, '4': 3, '6': 2} if sub != 'h4'
                                else {'2': 8, '4': 3})
        work = runner.out / 'attempts' / ('00000-' + sub)
        work.mkdir()
        snapshot_fn = Mock(return_value=snap)
        ctx = api.Ctx(self.tmp / 'table.json', state, snapshot_fn, None, work, runner.cache,
                      record, {'height_cpu': runner.args.height_cpu, 'height_seconds': runner.args.height_seconds,
                               'pair_shards': runner.args.pair_shards}, runner.mode, sub)
        with patch.dict(H.RULE_SCRIPTS, paths), patch.object(H, 'pair_end_digest', return_value=pair_digest):
            result = H.next_step(ctx)
        snapshot_fn.assert_called_once_with()
        self.assertTrue(np.array_equal(state.pose_id, np.array([0], dtype=np.int32)))
        self.assertEqual(state.digest(), D)
        self.assertFalse((work / 'state.pkl').exists())
        self.assertFalse((work / 'state.pkl.tmp').exists())
        hygiene(self, result)
        self.last_ctx, self.last_runner, self.last_calls = ctx, runner, calls
        return result

    def assert_end(self, result, kind, reason=None):
        self.assertIsInstance(result, api.End)
        self.assertEqual(result.result, kind, result)
        if reason is not None:
            self.assertEqual(result.reason, reason)

    def test_01_r22d75(self):
        for tier, counts in enumerate(([1, 4, 9], [2, 2, 9], [2, 3, 7])):
            result = self.invoke('r22d75', start('r22d75') + step('r22d75', n6_n4_n2=counts))
            self.assertEqual(result.rule, f'r22d75/T{tier}')
            self.assertIsNone(result.tree)
        for tag, kind, reason in (('HEIGHT2', 'handoff', 'handoff'), ('STUCK', 'stuck', 'no_step'),
                                  ('BUDGET', 'budget', 'cpu_budget')):
            result = self.invoke('r22d75', start('r22d75') + end(tag))
            self.assert_end(result, kind, reason)
            self.assertEqual(result.attempts[0]['timed_out'], tag == 'BUDGET')

    def test_02_r22d81_families_and_ends(self):
        for family in ('H6', 'U', 'H4', 'H2'):
            tau = 12 if family == 'H6' else 311
            result = self.invoke('r22d81', start('r22d81') + step('r22d81', family, tau=tau))
            self.assertEqual(result.rule, 'r22d81/' + family)
            self.assertEqual([result.fields[k] for k in ('tau_before', 'tau_after', 'u_before', 'u_after')],
                             [311, tau, 2, 1])
        self.assert_end(self.invoke('r22d81', start('r22d81') + end('HANDOFF')), 'handoff')
        self.assert_end(self.invoke('r22d81', start('r22d81', tau=None, u=0, targets=[]) +
                                    end('STUCK', tau=None, u=0)), 'stuck', 'no_target')
        self.assertEqual(len(self.last_calls), 1)
        self.assert_end(self.invoke('r22d81', end('BUDGET')), 'budget', 'cpu_budget')
        pending = {'step': 1, 'family': 'H6', 'letter': [12, 14], 'length': 1, 'input_digest': D}
        result = self.invoke('r22d81', start('r22d81') + end('BUDGET', steps=1, digest=E,
                                                                           played_unreported=pending))
        self.assert_end(result, 'budget', 'cpu_budget')
        self.assertTrue(result.attempts[-1]['timed_out'])
        self.assert_end(self.invoke('r22d81', start('r22d81') + end('BUDGET', steps=1)), 'error', 'missing_step')

    def test_03_pairs_and_not_applicable(self):
        for family in ('U', 'U2'):
            result = self.invoke('r22d81_pairs', start('r22d81_pairs') + step('r22d81_pairs', family))
            self.assertEqual(result.rule, 'r22d88/' + family)
            self.assertEqual(len(result.attempts), 2)
            argv = self.last_calls[-1][0]
            self.assertEqual(argv[argv.index('--target') + 1], '311')
            if family == 'U2':
                self.assertEqual(result.fields['u_mid'], 3)
                self.assertEqual([p['name'] for p in result.tree['parts']], ['first letter', 'second letter'])
            else:
                self.assertNotIn('u_mid', result.fields)
                self.assertIsNone(result.tree)
        for tag, kind, reason in (('UNLOCKED', 'stuck', 'unlocked_without_step'),
                                  ('STUCK', 'stuck', 'no_step'), ('BUDGET', 'budget', 'cpu_budget')):
            self.assert_end(self.invoke('r22d81_pairs', start('r22d81_pairs') + end(tag)), kind, reason)
        for changes in ({'tau': 290}, {'u': 9}, {'digest': E}):
            result = self.invoke('r22d81_pairs', start('r22d81_pairs', **changes) + end('STUCK'))
            self.assert_end(result, 'error')
            self.assertIn(result.reason, ('inconsistent_output', 'digest_mismatch'))
        for name in ('r22d81', 'r22d81_pairs'):
            payload = {'max_height': 8, 'digest': E}
            result = self.invoke(name, event('NOT_APPLICABLE', **payload))
            self.assert_end(result, 'error', 'invariant_height_above_6')
            self.assertEqual(result.details, payload)
            self.assertEqual(len(result.attempts), 2 if name.endswith('pairs') else 1)
            self.assertEqual(result.attempts[-1]['outcome'], 'not_applicable')
            self.assertFalse(result.attempts[-1]['timed_out'])

    def test_04_h4_families_and_guards(self):
        for family in ('L4', 'U', 'L2'):
            tau = 12 if family == 'L4' else 311
            result = self.invoke('h4', start('h4') + step('h4', family, tau=tau))
            self.assertEqual(result.rule, 'h4/' + family)
            self.assertEqual([result.fields[k] for k in ('tau_before', 'tau_after', 'u_before', 'u_after')],
                             [311, tau, 2, 1])
        self.assert_end(self.invoke('h4', start('h4') + end('HANDOFF')), 'handoff')
        result = self.invoke('h4', start('h4', tau=None, u=0, targets=[]), after='time.sleep(5)')
        self.assert_end(result, 'stuck', 'no_target')
        self.assertTrue(result.attempts[0]['stopped'])
        self.assertEqual(result.details['note'], H.NO_TARGET_NOTE)
        self.assertEqual(len(self.last_calls), 1)
        self.assert_end(self.invoke('h4', start('h4') + end('STUCK', targets=[])), 'stuck', 'no_target')
        self.assertEqual(len(self.last_calls), 1)
        self.assert_end(self.invoke('h4', start('h4') + end('STUCK', tau=None, u=0)),
                        'error', 'inconsistent_output')
        self.assertEqual(len(self.last_calls), 1)
        self.assert_end(self.invoke('h4', end('BUDGET', tau=None, u=None)), 'budget', 'cpu_budget')
        result = self.invoke('h4', start('h4') + end('BUDGET', steps=1, digest=E, families={'L4': 1}))
        self.assert_end(result, 'budget', 'cpu_budget')
        self.assertTrue(result.attempts[-1]['timed_out'])
        for name, height, reason in (('r22d75', 8, 'invariant_height_above_6'),
                                     ('r22d81', 8, 'invariant_height_above_6'),
                                     ('h4', 6, 'invariant_height_above_4')):
            result = self.invoke(name, snapshot={'digest': D, 'off': 1, 'heights': {str(height): 1}})
            self.assert_end(result, 'error', reason)
            self.assertFalse(self.last_calls)

    def invoke_shards(self, outputs, **kwargs):
        before = 'outputs = ' + repr(outputs) + '\nshard = int(sys.argv[sys.argv.index("--shard")+1])\n'
        return self.invoke('two_letter_u', before=before + 'sys.stdout.write(outputs[shard]); sys.stdout.flush()',
                           **kwargs)

    def test_05_pair_steps_and_exhaustion(self):
        self.assert_end(self.invoke_shards(shard_outputs()), 'stuck', 'pair_exhausted')
        for by in ('freeing', 'first_success'):
            result = self.invoke_shards(shard_outputs(choice=0, by=by))
            self.assertEqual(result.rule, 'h4/pair')
            self.assertEqual(result.word, {'word': [[1, 1], [3, 1]], 'input_digest': D, 'output_digest': E})
            self.assertEqual([result.fields[k] for k in ('tau_before', 'tau_after', 'u_before', 'u_after', 'u_mid')],
                             [311, 311, 2, 0, 3])
            self.assertEqual(result.fields['pair_choice'], {'order_index': 0, 'class': 'A', 'freed': [3] if
                             by == 'freeing' else [], 'by': by, 'shards': 2, 'first_letters': 118})
            self.assertEqual(len(result.tree['parts']), 2)
            self.assertEqual([a['shard'] for a in result.attempts], [None, 0, 1])
        outputs = shard_outputs(choice=0)
        outputs[1] = outputs[1].splitlines(keepends=True)[0] + event('BUDGET', first_letters_done=0)
        result = self.invoke_shards(outputs)
        self.assertEqual(result.rule, 'h4/pair')
        self.assertTrue(result.attempts[-1]['timed_out'])
        outputs = [text.splitlines(keepends=True)[0] + event('BUDGET', first_letters_done=0)
                   for text in shard_outputs()]
        result = self.invoke_shards(outputs)
        self.assert_end(result, 'budget', 'cpu_budget')
        self.assertTrue(all(a['timed_out'] for a in result.attempts[1:]))

    def test_06_shard_errors_and_timeouts(self):
        for suffix, reason in (('', 'truncated'), ('raise RuntimeError("fixture crash")', 'crash')):
            outputs = shard_outputs(choice=0)
            outputs[1] = outputs[1].splitlines(keepends=True)[0]
            result = self.invoke_shards(outputs, after=suffix)
            self.assert_end(result, 'error', reason)
        outputs = shard_outputs(choice=0)
        context = json.loads(outputs[1].splitlines()[0][8:])
        context['first_letter_caps'] = [1, 3]
        outputs[1] = event('CONTEXT', **context) + event('BUDGET', first_letters_done=0)
        self.assert_end(self.invoke_shards(outputs), 'error', 'inconsistent_output')
        for missing_context in (False, True):
            outputs = shard_outputs()
            outputs[1] = '' if missing_context else outputs[1].splitlines(keepends=True)[0]
            result = self.invoke_shards(outputs, after='\nif shard == 1: time.sleep(5)',
                                        extra=('--height-seconds', '1'))
            self.assert_end(result, 'timeout', 'wall_clock')
            self.assertTrue(result.attempts[-1]['timed_out'])
        outputs = shard_outputs()
        outputs[0] = ''.join(outputs[0].splitlines(keepends=True)[:-1]) + event('DONE', first_letters_done=0)
        self.assert_end(self.invoke_shards(outputs), 'error', 'inconsistent_output')

    def test_07_common_clocks_and_signals(self):
        for name in H.RULE_SOURCES:
            with self.subTest(script=name, clock='wall'):
                result = self.invoke(name, after='time.sleep(5)', extra=('--height-seconds', '0.1'))
                self.assert_end(result, 'timeout', 'wall_clock')
                self.assertTrue(result.attempts[-1]['timed_out'])
            disable_core = 'resource.setrlimit(resource.RLIMIT_CORE, (0, 0))\n'
            for delay, cpu, kind, reason in ((1.05, '1', 'budget', 'cpu_rlimit'),
                                             (0, '3600', 'error', 'crash')):
                with self.subTest(script=name, cpu=cpu):
                    result = self.invoke(name, before=disable_core + f'time.sleep({delay})\n' +
                                         'os.kill(os.getpid(), signal.SIGXCPU)', extra=('--height-cpu', cpu))
                    self.assert_end(result, kind, reason)
                    self.assertEqual(result.attempts[-1]['timed_out'], kind == 'budget')

    def test_08_common_crash_truncation_and_cancel(self):
        for name in H.RULE_SOURCES:
            with self.subTest(script=name):
                self.assert_end(self.invoke(name, before='raise RuntimeError("fixture crash")'), 'error', 'crash')
                if name == 'two_letter_u':
                    before = 'outputs = ' + repr(shard_outputs()) + \
                        '\nsys.stdout.write(outputs[int(sys.argv[sys.argv.index("--shard")+1])]); sys.stdout.flush()'
                    result = self.invoke(name, before=before, after='print("Traceback after end", flush=True)')
                else:
                    result = self.invoke(name, start(name) + end('STUCK'),
                                         after='print("Traceback after end", flush=True)')
                self.assert_end(result, 'error', 'crash')
                self.assert_end(self.invoke(name), 'error', 'truncated')
                result = self.invoke(name, cancel=True)
                self.assert_end(result, 'error', 'cancelled')
                self.assertEqual(result.attempts[-1]['script'], name)
                self.assertFalse(list(self.last_runner.out.glob('logs/*' + name + '*.log')))

    def test_09_malformed_and_mismatched_steps(self):
        for name in ('r22d75', 'r22d81', 'r22d81_pairs', 'h4'):
            for bad in ('START {broken}\n', start(name) + 'STEP {broken}\n'):
                with self.subTest(script=name, bad=bad):
                    self.assert_end(self.invoke(name, bad, after='time.sleep(0.1)'), 'error', 'malformed_output')
            self.assert_end(self.invoke(name, start(name, digest=E) + step(name)), 'error', 'digest_mismatch')
            self.assert_end(self.invoke(name, start(name) + step(name, input_digest=E)), 'error', 'digest_mismatch')
            bad_letter_key = 'word' if name == 'r22d75' else 'letters' if name.endswith('pairs') else 'letter'
            invalid = [[0, 0]] if bad_letter_key in ('word', 'letters') else [0, 0]
            self.assert_end(self.invoke(name, start(name) + step(name, **{bad_letter_key: invalid})),
                            'error', 'malformed_output')
            # Ignore every line after the deciding STEP, even if poll() saw exit first.
            result = self.invoke(name, start(name) + step(name) + 'STEP {broken}\nTraceback later\n',
                                 after='sys.exit(1)')
            self.assertIsInstance(result, api.Proposal)
        for counts in ([2, 3, 8], [3, 0, 0]):
            self.assert_end(self.invoke('r22d75', start('r22d75') + step('r22d75', n6_n4_n2=counts)),
                            'error', 'malformed_output')
        for name in ('r22d81', 'h4', 'r22d81_pairs'):
            self.assert_end(self.invoke(name, start(name) + step(name, tau=12)), 'error', 'malformed_output')
        self.assert_end(self.invoke('two_letter_u', 'CONTEXT {broken}\n'), 'error', 'malformed_output')

    def test_10_missing_steps(self):
        for name, tag in (('r22d75', 'HEIGHT2'), ('r22d81', 'HANDOFF'),
                          ('r22d81_pairs', 'UNLOCKED'), ('h4', 'STUCK'),
                          ('r22d75', 'BUDGET'), ('r22d81_pairs', 'BUDGET')):
            with self.subTest(script=name, end=tag):
                self.assert_end(self.invoke(name, start(name) + end(tag, steps=1)), 'error', 'missing_step')
        for families in ({'U': 1}, {'L4': 2}, {'L4': 1, 'U': 1}):
            self.assert_end(self.invoke('h4', start('h4') + end('BUDGET', steps=1, families=families)),
                            'error', 'missing_step')
        self.assert_end(self.invoke('r22d75', start('r22d75') + step('r22d75').rstrip('\n')),
                        'error', 'truncated')

    def test_11_clocks_wiring(self):
        for mode in ('acceptance', 'development'):
            for name in H.RULE_SOURCES:
                def execute(argv, tag, seconds, **kwargs):
                    cpu = argv[argv.index('--cpu') + 1]
                    self.assertEqual(cpu, '1000000000.0' if mode == 'acceptance' else '7.0')
                    self.assertEqual(seconds, None if mode == 'acceptance' else 9.0)
                    self.assertEqual(argv[:4], [sys.executable, '-I', '-B', '-u'])
                    self.assertEqual(argv[argv.index('--expect') + 1], D)
                    self.assertTrue(Path(argv[argv.index('--snapshot') + 1]).exists())
                    log = self.tmp / (tag + '.log')
                    if name == 'r22d81_pairs' and tag.endswith('-r22d81'):
                        text = start('r22d81') + end('STUCK')
                    elif name == 'two_letter_u' and tag.endswith('-h4'):
                        text = start('h4') + end('STUCK')
                    elif name == 'two_letter_u':
                        shard = int(argv[argv.index('--shard') + 1])
                        text = shard_outputs()[shard]
                    else:
                        text = end('BUDGET')
                    log.write_text(text)
                    return R.Child(0, 0.01, log)
                self.invoke(name, execute=execute,
                            extra=('--mode', mode, '--height-cpu', '7', '--height-seconds', '9'))
                self.assertTrue(self.last_calls)

    def test_12_scripts_and_snapshot(self):
        self.assertFalse(getattr(H, 'STUB', False))
        self.assertEqual(set(H.RULE_SOURCES), set(H.RULE_SCRIPTS))
        self.assertEqual(len(H.SCRIPTS), 8)
        for path in H.SCRIPTS:
            self.assertIsInstance(path, Path)
            self.assertFalse(path.is_absolute())
            self.assertTrue((ROOT / path).is_file())
        state = SimpleNamespace(pose_id=np.array([0], dtype=np.int32),
                                _poses=[SimpleNamespace(kidx=-1, matrix=None, key=('fake',))])
        path = self.tmp / 'snapshot.pkl'
        H.write_snapshot(path, state, D, 1)
        self.assertEqual(path.read_bytes()[:2], b'\x80\x05')
        with path.open('rb') as f:
            value = pickle.load(f)  # only the snapshot this test wrote
        self.assertEqual({k: value[k] for k in ('record', 'history', 'moves', 'digest', 'off')},
                         {'record': 'runner', 'history': 0, 'moves': 0, 'digest': D, 'off': 1})
        self.assertEqual(value['poses'], [(-1, None, ('fake',))])
        self.assertTrue(np.array_equal(value['pose_id'], state.pose_id))
        with self.assertRaises(ValueError):
            H.next_step(SimpleNamespace(subphase='unknown'))

    def test_13_pair_order_and_validation(self):
        # Class A is enumerated first, but rule 2 compares actual (grip, item) letters across both classes.
        outputs = shard_outputs(choice=59, by='first_success', caps=(1, 2), class_a=(2,))
        result = H.choose_pair([text.splitlines(keepends=True) for text in outputs])
        self.assertEqual(result['step']['letters'], [[1, 1], [3, 1]])
        self.assertEqual(result['step']['fields']['pair_choice']['by'], 'first_success')
        # A freeing candidate wins even when a smaller B letter succeeds.
        parsed = [text.splitlines(keepends=True) for text in outputs]
        payload = dict(index=0, first=[2, 1], cls='A', valid=True, freed=[3], u_t=2, successes=[[4, 1, 0]])
        parsed[0][1] = event('FIRST', **payload)
        result = H.choose_pair(parsed)
        self.assertEqual(result['step']['letters'], [[2, 1], [4, 1]])
        for mutate in ('first', 'cls', 'success_order', 'digest', 'nshards', 'done_count'):
            outputs = [text.splitlines(keepends=True) for text in shard_outputs(choice=0)]
            if mutate in ('digest', 'nshards'):
                payload = json.loads(outputs[1][0][8:])
                payload[mutate] = E if mutate == 'digest' else 3
                outputs[1][0] = event('CONTEXT', **payload)
            elif mutate == 'done_count':
                outputs[0][-1] = event('DONE', first_letters_done=0)
            else:
                payload = json.loads(outputs[0][1][6:])
                if mutate == 'first':
                    payload['first'] = [True, 1]
                elif mutate == 'cls':
                    payload['cls'] = 'B'
                else:
                    payload['successes'].reverse()
                outputs[0][1] = event('FIRST', **payload)
            with self.subTest(mutate=mutate):
                self.assertEqual(H.choose_pair(outputs)['result'], 'error')
        outputs = [text.splitlines(keepends=True) for text in shard_outputs(choice=0)]
        # Identical duplicate evidence is permitted across resumed files.
        duplicate = [outputs[0][0], outputs[0][1]]
        self.assertEqual(H.choose_pair(outputs + [duplicate])['result'], 'step')
        self.assertEqual(H.choose_pair([[], ['DONE {"first_letters_done": 0}\n']])['result'], 'undecided')

    def test_14_parse_errors_survive_wall_kill(self):
        for text, reason in (('START {broken json\n', 'malformed_output'),
                             ('Traceback fixture crash\n', 'crash')):
            with self.subTest(reason=reason):
                result = self.invoke('r22d81', text, after='time.sleep(5)',
                                     extra=('--height-seconds', '0.3'))
                self.assert_end(result, 'error', reason)
                self.assertTrue(result.attempts[-1]['timed_out'])
        outputs = shard_outputs(choice=0)
        outputs[1] = outputs[1].splitlines(keepends=True)[0] + 'FIRST {broken json\n'
        result = self.invoke_shards(outputs, after='\nif shard == 1: time.sleep(5)',
                                    extra=('--height-seconds', '0.3'))
        self.assert_end(result, 'error', 'malformed_output')
        self.assertTrue(result.attempts[-1]['timed_out'])
        # A deciding STEP can coincide with the executor's timeout flag too.
        for payload, reason in ((step('r22d81', input_digest=E), 'digest_mismatch'),
                                (step('r22d81', step=2), 'missing_step')):
            def killed(argv, tag, seconds, **kwargs):
                log = self.tmp / (tag + '.log')
                log.write_text(start('r22d81') + payload)
                return R.Child(-signal.SIGKILL, 1, log, timeout=True)
            self.assert_end(self.invoke('r22d81', execute=killed), 'error', reason)

    def test_15_done_coverage_and_undecided_without_clock(self):
        for choice in (None, 0):
            outputs = [text.splitlines(keepends=True) for text in shard_outputs(choice=choice)]
            outputs[1] = [line for line in outputs[1] if not
                          (line.startswith('FIRST ') and json.loads(line[6:])['index'] == 5)]
            outputs[1][-1] = event('DONE', first_letters_done=len(outputs[1]) - 2)
            with self.subTest(choice=choice):
                self.assert_end(self.invoke_shards([''.join(out) for out in outputs]),
                                'error', 'inconsistent_output')
        self.assert_end(self.invoke_shards(shard_outputs()), 'stuck', 'pair_exhausted')
        # Exercise the defensive mapping when a chooser supplies no decision,
        # despite complete DONE evidence and no clock ending either attempt.
        with patch.object(H, '_pair_choice', return_value={'result': 'undecided'}):
            result = self.invoke_shards(shard_outputs())
        self.assert_end(result, 'error', 'inconsistent_output')
        self.assertTrue(all(a['outcome'] == 'done' and not a['timed_out'] for a in result.attempts[1:]))

    def test_16_end_lines_require_start(self):
        for name, tag in (('r22d75', 'HEIGHT2'), ('r22d75', 'STUCK'),
                          ('r22d81', 'HANDOFF'), ('r22d81', 'STUCK'),
                          ('r22d81_pairs', 'STUCK'), ('r22d81_pairs', 'UNLOCKED'),
                          ('h4', 'HANDOFF'), ('h4', 'STUCK')):
            with self.subTest(script=name, tag=tag):
                result = self.invoke(name, end(tag, tau=5, u=99))
                self.assert_end(result, 'error', 'truncated')
                self.assertNotEqual(self.invoke(name, start(name) + end(tag)).result, 'error')
        for name, fields in (('r22d81', {'played_unreported': {'step': 1, 'family': 'H6'}}),
                             ('h4', {'families': {'L4': 1}})):
            text = end('BUDGET', steps=1, digest=E, **fields)
            with self.subTest(script=name, tag='BUDGET'):
                self.assert_end(self.invoke(name, text), 'error', 'truncated')
                self.assert_end(self.invoke(name, start(name) + text), 'budget', 'cpu_budget')

    def test_17_zero_step_end_identity(self):
        for name, tags in (('r22d81', ('HANDOFF', 'STUCK', 'BUDGET')),
                           ('r22d81_pairs', ('UNLOCKED', 'STUCK', 'BUDGET')),
                           ('h4', ('HANDOFF', 'STUCK', 'BUDGET'))):
            for tag in tags:
                for fields in ({'tau': 5}, {'u': 99}):
                    with self.subTest(script=name, tag=tag, fields=fields):
                        self.assert_end(self.invoke(name, start(name) + end(tag, **fields)),
                                        'error', 'inconsistent_output')
        self.assert_end(self.invoke('r22d81', start('r22d81', tau=None, u=0) + end('STUCK')),
                        'error', 'inconsistent_output')
        self.assertEqual(len(self.last_calls), 1)
        self.assert_end(self.invoke('r22d81', start('r22d81', tau=None, u=0) +
                                    end('STUCK', tau=None, u=0)), 'stuck', 'no_target')
        self.assertEqual(len(self.last_calls), 1)

    def test_18_shard_crash_stops_sibling(self):
        for crashed in (0, 1):
            pid_dir = self.tmp / f'pids-{crashed}'
            pid_dir.mkdir()
            before = ('shard = int(sys.argv[sys.argv.index("--shard")+1])\n' +
                      f'pid_dir = {str(pid_dir)!r}\n' +
                      'with open(os.path.join(pid_dir, str(shard)), "w") as f: f.write(str(os.getpid()))\n' +
                      f'if shard == {crashed}:\n' +
                      '    while not os.path.exists(os.path.join(pid_dir, str(1-shard))): time.sleep(0.01)\n' +
                      '    raise RuntimeError("fixture crash")\n' +
                      'for _ in range(150):\n    print("heartbeat", flush=True)\n    time.sleep(0.2)\n')
            began = time.monotonic()
            result = self.invoke('two_letter_u', before=before, extra=('--mode', 'acceptance'))
            elapsed = time.monotonic() - began
            self.assert_end(result, 'error', 'crash')
            self.assertLess(elapsed, 5, 'a crashed shard must stop its sibling promptly')
            self.assertEqual(len(list(pid_dir.iterdir())), 2)
            for path in pid_dir.iterdir():
                with self.assertRaises(ProcessLookupError):
                    os.kill(int(path.read_text()), 0)
            sibling = next(a for a in result.attempts if a['shard'] == 1 - crashed)
            self.assertTrue(sibling['stopped'])

    def test_19_shard_wait_exception_stops_workers(self):
        stopped = []

        def worker(*args, stop_when, **kwargs):
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                if stop_when('heartbeat'):
                    stopped.append(True)
                    return
                time.sleep(0.01)

        ctx = SimpleNamespace(budgets={'pair_shards': 2})
        began = time.monotonic()
        with patch.object(H, '_call', side_effect=worker), \
                patch.object(H, 'as_completed', side_effect=KeyboardInterrupt):
            with self.assertRaises(KeyboardInterrupt):
                H._shards(ctx, {}, self.tmp / 'state.pkl', {'tau': 311, 'u': 2}, [])
        self.assertEqual(stopped, [True, True])
        self.assertLess(time.monotonic() - began, 1)


class FakeHigh:
    STUB = True

    @staticmethod
    def next_step(ctx):
        return api.End('not_applicable', 'stub')


class FakeLattice:
    @staticmethod
    def play(ctx):
        raise AssertionError('P1 never reaches lattice')


class ExactTests(TemporaryTest):
    @classmethod
    def setUpClass(cls):
        began = time.monotonic()
        cls.shared = Path(tempfile.mkdtemp(prefix='height-exact-'))
        cls.addClassCleanup(shutil.rmtree, cls.shared)
        env = patch.dict(os.environ, JUMBLING_HOME_CACHE=str(cls.shared / 'cache/home_centroids.npy'))
        env.start()
        cls.addClassCleanup(env.stop)
        cls.m = R.Mechanics(cls.shared / 'cache')
        cls.solved = cls.m.sim.State(cls.m.restore.ctx, menu=cls.m.menu)
        cls.z36 = cls.m.sim.State(cls.m.restore.ctx, menu=cls.m.menu)
        cls.m.apply_word(cls.z36, {'word': WORD36, 'input_digest': R.SOLVED, 'output_digest': Z36})
        cls.ztable = cls.shared / 'z36.json'
        cls.m.export(cls.z36, cls.ztable)
        cls.s2 = cls.m.load(cls.ztable)
        cls.m.apply_word(cls.s2, {'word': [[12, 14]], 'input_digest': Z36, 'output_digest': S2})
        cls.s2table = cls.shared / 's2.json'
        cls.m.export(cls.s2, cls.s2table)
        print(f'Geometry setup: {time.monotonic() - began:.3f}s; S2 = {cls.s2.digest()}', flush=True)

    def runner(self, name, table=None):
        args = R.parse_args(['--table', str(table or self.ztable), '--out', str(self.tmp / name),
                             '--cache', str(self.tmp / 'cache'), '--bulk-steps', '0',
                             '--mode', 'development', '--height-seconds', '600', '--no-lattice'])
        return R.Runner(args, self.m, {'height': H, 'high': FakeHigh, 'lattice': FakeLattice})

    def timed_execute(self, runner):
        original = runner.execute

        def execute(argv, tag, seconds, **kwargs):
            child = original(argv, tag, seconds, **kwargs)
            REAL_RUNS.append((tag, child.seconds))
            print(f'Real child {tag}: {child.seconds:.3f}s, rc={child.rc}', flush=True)
            return child
        return execute

    def test_01_r22d36_and_h4(self):
        self.assertEqual(self.m.snapshot(self.z36), {'digest': Z36, 'off': 5800, 'heights': {'2': 5799, '6': 1}})
        self.assertEqual(self.s2.digest(), S2)
        runner = self.runner('r22d36')
        original = H.next_step

        def checked(ctx):
            value = original(ctx)
            hygiene(self, value)
            return value

        with patch.object(runner, 'execute', side_effect=self.timed_execute(runner)), \
                patch.object(H, 'next_step', side_effect=checked):
            runner.initialize()
            runner.bulk()
            self.assertIsNone(runner.phases_between())
        rows = runner.record['steps']
        self.assertEqual(len(rows), 2)
        for row, rule, letter, digest, off, heights in zip(rows, ['r22d75/T0', 'r22d75/T1'],
                [[[12, 14]], [[35, 13]]], [S2, S3], [3098, 5405], [{'2': 3097, '4': 1}, {'2': 5405}], strict=True):
            self.assertEqual((row['rule'], R.read_json(runner.out / row['word'])['word']), (rule, letter))
            self.assertEqual(row['after'], {'digest': digest, 'off': off, 'heights': heights})
            self.assertEqual((row['contract'], row['gate']), (rule, 'r22d75_from_snapshot'))
            self.assertTrue(all(row[k] is None for k in ('tau_before', 'tau_after', 'u_before', 'u_after')))
        self.assertEqual([d['result'] for d in runner.record['phases']['height']['sub'].values()],
                         ['handoff', 'not_applicable', 'not_applicable'])
        self.assertEqual(runner.record['phases']['height']['result'], 'handoff')
        self.assertEqual(runner.record['phases']['high']['result'], 'not_applicable')
        second = self.runner('h4', runner.out / 'tables/00000.json')
        second.initialize()
        ctx = second.context('height', 'h4')
        before = second.state.digest()
        with patch.object(second, 'execute', side_effect=self.timed_execute(second)):
            ctx.execute = second.execute
            proposal = H.next_step(ctx)
        hygiene(self, proposal)
        self.assertIsInstance(proposal, api.Proposal, proposal)
        self.assertEqual(second.state.digest(), before)
        self.assertFalse((ctx.work_dir / 'state.pkl').exists())
        self.assertEqual((proposal.rule, proposal.word['word'], proposal.word['output_digest']),
                         ('h4/L4', [[35, 13]], S3))
        self.assertEqual([proposal.fields[k] for k in ('tau_before', 'tau_after', 'u_before', 'u_after')],
                         [35, None, 0, 0])
        second.record_step(proposal, 'height', 'h4')
        self.assertEqual(second.state.digest(), S3)
        print('H4 at S2:', proposal.rule, proposal.word, proposal.fields, flush=True)
        # The pair digest helper independently replays on a copy and leaves the live state intact.
        ctx = runner.context('height', 'r22d75')
        ctx.table, ctx.state = self.ztable, self.z36
        self.assertEqual(H.pair_end_digest(ctx, [[12, 14], [35, 13]]), S3)
        self.assertEqual(self.z36.digest(), Z36)
        ctx.table = self.s2table
        with self.assertRaises(H.OutputError) as caught:
            H.pair_end_digest(ctx, [[35, 13]])
        self.assertEqual(caught.exception.reason, 'digest_mismatch')

    def test_02_real_clock_sentinel(self):
        runner = self.runner('clocks')
        missing = self.tmp / 'absent.pkl'
        for name, script in H.RULE_SOURCES.items():
            argv = [sys.executable, '-I', '-B', '-u', str(ROOT / script), '--snapshot', str(missing), '--cpu', '1e9']
            if name == 'r22d81_pairs':
                argv += ['--target', '0']
            child = runner.execute(argv, 'missing-' + name, 600)
            REAL_RUNS.append(('missing-' + name, child.seconds))
            print(f'Real child missing-{name}: {child.seconds:.3f}s, rc={child.rc}', flush=True)
            text = child.log.read_text()
            self.assertEqual(child.rc, 1, text)
            self.assertIn('FileNotFoundError', text)
            self.assertNotIn('OverflowError', text)
            self.assertNotIn('ValueError', text)
        for name, state, extra, end_tag in (('r22d81', self.solved, [], 'HANDOFF'),
                ('r22d81_pairs', self.s2, ['--target', '35'], 'UNLOCKED')):
            path = self.tmp / (name + '.pkl')
            H.write_snapshot(path, state, state.digest(), int(state.off_lattice_count()))
            argv = [sys.executable, '-I', '-B', '-u', str(ROOT / H.RULE_SOURCES[name]),
                    '--snapshot', str(path), '--expect', state.digest(), '--cpu', '1e9', *extra]
            child = runner.execute(argv, 'loaded-' + name, 600)
            REAL_RUNS.append(('loaded-' + name, child.seconds))
            print(f'Real child loaded-{name}: {child.seconds:.3f}s, rc={child.rc}', flush=True)
            text = child.log.read_text()
            self.assertEqual(child.rc, 0, text)
            self.assertNotIn('OverflowError', text)
            self.assertNotIn('ValueError', text)
            parsed = H.parse_log(name, text.splitlines(keepends=True))
            self.assertEqual((parsed['status'], parsed['end']['tag'], parsed['end']['payload']['steps']),
                             ('end', end_tag, 0))

    def test_03_wrong_claim_records_runner_error(self):
        runner = self.runner('wrong-claim')
        script = self.tmp / 'wrong.py'
        text = start('r22d75', digest=Z36, n6_n4_n2=[1, 0, 5799]) + \
               step('r22d75', input_digest=Z36, digest='0' * 64, n6_n4_n2=[0, 1, 3097])
        script.write_text(f'print({text!r}, end="", flush=True)\n')
        original = H.next_step

        def checked(ctx):
            value = original(ctx)
            hygiene(self, value)
            self.assertIsInstance(value, api.Proposal)
            return value

        with patch.dict(H.RULE_SCRIPTS, r22d75=script), patch.object(H, 'next_step', side_effect=checked):
            runner.run()
        self.assertEqual(runner.record['result'], 'error')
        self.assertIn('word output digest mismatch', runner.record['error'])
        self.assertEqual(runner.record['steps'], [])
        self.assertIsNone(runner.record['phases']['height']['sub']['r22d75']['result'])
        self.assertIsNone(runner.record['phases']['height']['result'])


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        elapsed = time.monotonic() - self.started
        super().stopTest(test)
        self.stream.writeln(f'  {test.id()}: {elapsed:.3f}s')


# Same exclusions as test_runner.py: Git metadata and private concurrent writers.
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
    began = time.monotonic()
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
    print('Real child wall times:', [(tag, round(seconds, 3)) for tag, seconds in REAL_RUNS], file=sys.stderr)
    print(f'Acceptance elapsed: {time.monotonic() - began:.3f}s.', file=sys.stderr)
    return 0 if result.wasSuccessful() and unchanged else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
