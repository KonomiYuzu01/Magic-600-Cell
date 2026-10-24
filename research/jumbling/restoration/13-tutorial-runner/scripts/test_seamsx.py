"""Exact headless bulk-rule comparisons; all generated caches are temporary.

Run from the repository root with python3 -I -B. Only the named D1 development
record and D3 fixture are read. No runner process or unrelated search is run.
"""
import atexit
import copy
import json
import os
from pathlib import Path
import random
import signal
import sys
import tempfile
import threading
import time
import traceback
import unittest
from unittest.mock import patch

STARTED = time.perf_counter()
LIMIT = 1500
sys.dont_write_bytecode = True
for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[name] = '1'
TEMPORARY = tempfile.TemporaryDirectory(prefix='seamsx-tests-')
atexit.register(TEMPORARY.cleanup)
CACHE = Path(TEMPORARY.name) / 'cache'
os.environ['JUMBLING_HOME_CACHE'] = str(CACHE / 'home_centroids.npy')

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
THEORY = ROOT / 'research/jumbling/theory'
sys.path[:0] = [str(HERE), str(THEORY), str(THEORY.parent), str(THEORY.parent / 'fixtures')]

import numpy as np  # noqa: E402
import restore  # noqa: E402
import rim  # noqa: E402
import runner  # noqa: E402
import seams  # noqa: E402
import seamsx  # noqa: E402
import sim  # noqa: E402

D1 = (ROOT / 'research/jumbling/restoration/13-tutorial-runner/runs/dev/'
      'd1-dev-s7101/run1-interrupted')
D3 = ROOT / 'research/jumbling/restoration/13-tutorial-runner/runs/dev/d3-s7102-fixture'
CONJ_STEPS = (29, 31, 32, 34, *range(36, 43))
RULES = ('seam', 'cover', 'level1', 'conj', 'level2', 'improve', 'shift')
M = restore.menu('I_a')


def clone(state):
    out = sim.State(state.ctx, filtered=state.filtered, menu=state.menu)
    out.pose_id = state.pose_id.copy()
    out._poses = list(state._poses)
    out._lookup = dict(state._lookup)
    out.journal = copy.deepcopy(state.journal)
    return out


def outcome(restorer, method):
    word = getattr(restorer, method)()
    return (word, list(restorer.moves), restorer.st.digest(), int(restorer.misalignment()),
            copy.deepcopy(restorer.st.journal))


class CacheTests(unittest.TestCase):
    def small(self, seed=7103):
        state, history = restore.scramble(M, 2, seed)
        return seamsx.SeamRestorer(state, M, workers=1), history

    def assert_fresh(self, restorer):
        fresh = seams.SeamRestorer(clone(restorer.st), M)
        for c, value in list(restorer._cls.items()):
            other = fresh.inside(c)
            if value is None:
                self.assertIsNone(other)
            else:
                np.testing.assert_array_equal(value, other)
        for c, value in list(restorer._sv.items()):
            self.assertEqual(value, fresh.scores(c), ('scores', c))
        for c, value in list(restorer._bal.items()):
            self.assertEqual(value, fresh.balance(c), ('balance', c))
        self.assertEqual(restorer.domains().total_misalignment(),
                         fresh.domains().total_misalignment())
        np.testing.assert_array_equal(restorer.domains().dom, fresh.domains().dom)
        np.testing.assert_array_equal(restorer.domains().cham, fresh.domains().cham)

    def test_domains_hook_and_worker_settings(self):
        class OtherDomains(rim.Domains):
            pass

        class OtherRestorer(seamsx.SeamRestorer):
            DOMAINS = OtherDomains

        state = sim.State(restore.ctx, menu=M)
        r = OtherRestorer(state, M, workers=1)
        self.assertIsInstance(r.domains(), OtherDomains)
        self.assertIs(r.domains(), r.domains())
        self.assertEqual(r.domains().total_misalignment(), 0)
        self.assertIs(seamsx.SeamRestorer.DOMAINS, rim.Domains)
        with patch.dict(os.environ, {}, clear=True):
            self.assertEqual(seamsx.SeamRestorer(state, M).workers, 1)
        for text in ('1', '3'):
            with patch.dict(os.environ, JUMBLING_BULK_WORKERS=text):
                self.assertEqual(seamsx.SeamRestorer(state, M).workers, int(text))
                self.assertEqual(seamsx.SeamRestorer(state, M, workers=2).workers, 2)
        for text in ('0', '-1', '', 'bad', '1.5'):
            with self.subTest(environment=text), patch.dict(os.environ, JUMBLING_BULK_WORKERS=text):
                with self.assertRaises(ValueError):
                    seamsx.SeamRestorer(state, M)
        for value in (0, -1, 1.5, '3', True):
            with self.subTest(workers=value), self.assertRaises(ValueError):
                seamsx.SeamRestorer(state, M, workers=value)

    def test_random_nested_apply_undo_restores_exact_entries(self):
        rng = random.Random(7105)
        r, _ = self.small()
        stack = []
        undos = 0

        def undo():
            nonlocal undos
            c, digest, domain, saved = stack.pop()
            r.undo()
            self.assertEqual(r.st.digest(), digest)
            self.assertIs(r._dom, domain)
            for cache, before in zip((r._cls, r._sv, r._bal), saved):
                for f in restore.NEAR[c]:
                    f = int(f)
                    self.assertEqual(f in cache, f in before)
                    if f in before:
                        self.assertIs(cache[f], before[f])
            self.assert_fresh(r)
            undos += 1

        for _ in range(18):
            if stack and (len(stack) == 3 or rng.random() < 0.45):
                undo()
                continue
            while True:
                c = rng.randrange(600)
                if r.inside(c) is not None:
                    break
            near = [int(x) for x in restore.NEAR[c]]
            far = next(f for f in range(600) if f not in near)
            for f in rng.sample(near, min(2, len(near))):
                r.scores(f)
                r.balance(f)
            # Leave an entry absent even if a prior nested trial populated it.
            missing = next(f for f in near if f != c)
            for cache in (r._cls, r._sv, r._bal):
                cache.pop(missing, None)
                cache.pop(far, None)
            domain = r._dom
            saved = tuple(dict(cache) for cache in (r._cls, r._sv, r._bal))
            stack.append((c, r.st.digest(), domain, saved))
            i = rng.choice([i for i in range(r.n_items) if r.g(c, i) != restore.I4])
            r.apply(c, i)
            r.scores(missing)
            r.balance(missing)
            # A newly computed entry outside NEAR[c] must survive undo too.
            r.scores(far)
            r.balance(far)
        while stack:
            undo()
        self.assertGreaterEqual(undos, 8)
        self.assertEqual(r.moves, [])
        self.assertEqual(r.st.journal, [])
        self.assertEqual(len(r._undo_cache), 0)

    def test_absent_domains_and_new_far_entries(self):
        r, history = self.small()
        c, i = history[-1]
        far = next(f for f in range(600) if f not in restore.NEAR[c])
        self.assertIsNone(r._dom)
        r.apply(c, i)
        r.scores(c)
        r.balance(c)
        r.scores(far)
        r.balance(far)
        entries = tuple(cache[far] for cache in (r._cls, r._sv, r._bal))
        r.undo()
        self.assertIsNone(r._dom)
        for cache, entry in zip((r._cls, r._sv, r._bal), entries):
            self.assertNotIn(c, cache)
            self.assertIs(cache[far], entry)
        self.assert_fresh(r)

    def test_external_state_edits_drop_snapshots(self):
        for mode in ('external_apply', 'external_undo', 'replace_record'):
            with self.subTest(mode=mode):
                r, history = self.small()
                c, i = history[-1]
                r.scores(c)
                r.balance(c)
                r.apply(c, i)
                twist = sim.Twist(restore.ctx, c, r.g(c, i), 'menu', {'menu': M.name})
                if mode == 'external_apply':
                    self.assertTrue(r.st.apply(twist).applied)
                elif mode == 'external_undo':
                    r.apply(c, i)
                    r.st.undo()
                else:
                    # Same grip, digest and record contents, but no longer the
                    # matching apply's journal record: the identity guard matters.
                    r.st.undo()
                    self.assertTrue(r.st.apply(twist).applied)
                r.scores(c)
                r.balance(c)
                r.undo()
                self.assertIsNone(r._dom)
                self.assertEqual(len(r._undo_cache), 0)
                for cache in (r._cls, r._sv, r._bal):
                    self.assertTrue(all(int(f) not in cache for f in restore.NEAR[c]))
                self.assert_fresh(r)

    def test_raising_apply_and_bounded_committed_history(self):
        r, history = self.small()
        parent = seams.SeamRestorer(clone(r.st), M)
        c, i = history[-1]
        for _ in range(48):
            r.scores(c)
            r.balance(c)
            r.apply(c, i)
            parent.apply(c, i)
            self.assertLessEqual(len(r._undo_cache), 3)
        self.assertEqual(len(r._undo_cache), 3)
        saved = list(r._undo_cache)
        before = r.st.snapshot()
        with patch.object(r.st, '_compact', side_effect=ValueError('transaction test')):
            with self.assertRaisesRegex(ValueError, 'transaction test'):
                r.apply(c, i)
        self.assertEqual(r.st.snapshot(), before)
        self.assertEqual(len(r._undo_cache), len(saved))
        for actual, expected in zip(r._undo_cache, saved):
            self.assertIs(actual, expected)
        with patch.object(r.st, 'apply', return_value=sim.Outcome('blocked', c)):
            with self.assertRaisesRegex(RuntimeError, 'not admissible'):
                r.apply(c, i)
        for actual, expected in zip(r._undo_cache, saved):
            self.assertIs(actual, expected)
        for _ in range(5):
            r.undo()
            parent.undo()
            self.assertEqual(r.st.snapshot(), parent.st.snapshot())
            self.assertEqual(r.moves, parent.moves)
            self.assert_fresh(r)


class ScriptedRestorer(seamsx.SeamRestorer):
    """Force completion races and worker faults through the real IPC scheduler."""
    def __init__(self, script):
        super().__init__(sim.State(restore.ctx, menu=M), M, workers=3)
        self.script = script
        self.parent_pid = os.getpid()
        self.parent_group = os.getpgrp()

    def survey(self, caps=range(600)):
        return {}

    def _conj_trials(self, sv):
        return [(0, i, (), 0) for i in range(len(self.script))]

    def _conj_trial(self, c, i, nb, d1):
        assert os.getpid() != self.parent_pid
        assert os.getpgrp() == self.parent_group
        delay, result = self.script[i]
        time.sleep(delay)
        if result == 'error':
            raise ValueError('intentional worker error')
        if result == 'exit':
            os._exit(7)
        return result


@unittest.skipUnless('fork' in seamsx.mp.get_all_start_methods(), 'fork not available')
class ParallelTests(unittest.TestCase):
    def assert_no_children(self):
        self.assertEqual(seamsx.mp.active_children(), [])

    def test_ordered_chunks_first_success_and_replay(self):
        first = [(0, 1), (0, 1), (0, 1)]
        later = [(0, 2), (0, 2), (0, 2)]
        # The earlier success is deliberately slower than a later one. The
        # next chunk contains errors that must never be evaluated.
        r = ScriptedRestorer([(0.05, None), (0.15, first), (0, later), (0, 'error')])
        expected = seams.SeamRestorer(clone(r.st), M)
        for c, i in first:
            expected.apply(c, i)
        self.assertEqual(r.conj(), first)
        self.assertEqual(r.st.snapshot(), expected.st.snapshot())
        self.assertEqual(r.moves, first)
        self.assert_no_children()
        r = ScriptedRestorer([(0, None)] * 3 + [(0.05, first), (0, later)])
        self.assertEqual(r.conj(), first)
        self.assert_no_children()

    def test_no_success_keeps_state_journal_and_moves(self):
        r = ScriptedRestorer([(0, None)] * 8)
        r.apply(0, 1)
        before, moves = r.st.snapshot(), list(r.moves)
        self.assertIsNone(r.conj())
        self.assertEqual(r.st.snapshot(), before)
        self.assertEqual(r.moves, moves)
        self.assert_no_children()
        r.script = []
        self.assertIsNone(r.conj())
        self.assert_no_children()

    def test_worker_exception_and_unexpected_exit_are_errors(self):
        for problem in ('error', 'exit'):
            with self.subTest(problem=problem):
                r = ScriptedRestorer([(0.1, None), (0, problem), (0.1, None)])
                before = r.st.snapshot()
                with self.assertRaisesRegex(RuntimeError, 'conj worker'):
                    r.conj()
                self.assertEqual(r.st.snapshot(), before)
                self.assertEqual(r.moves, [])
                self.assert_no_children()


class FallbackTests(unittest.TestCase):
    def test_single_worker_and_no_fork_call_parent(self):
        state = sim.State(restore.ctx, menu=M)
        for workers, methods in ((1, ['fork']), (3, ['spawn'])):
            r = seamsx.SeamRestorer(clone(state), M, workers=workers)
            with patch.object(seamsx.mp, 'get_all_start_methods', return_value=methods):
                with patch.object(seams.SeamRestorer, 'conj', return_value='parent') as parent:
                    self.assertEqual(r.conj(), 'parent')
                    parent.assert_called_once_with()


class RecordedTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.mechanics = runner.Mechanics(CACHE)
        cls.record = json.loads((D1 / 'run.json').read_text())
        cls.steps = {step['index']: step for step in cls.record['steps'] if step['index'] <= 42}
        cls.states, cls.words, cls.initial_caches, cls.elements = {}, {}, {}, {}
        state = cls.mechanics.load(D1 / 'start-table.json')
        # Rebuild exactly as bulk_child: start table, then each recorded word,
        # with Mechanics checking both digests and clearing each word's journal.
        for index in range(43):
            step = cls.steps[index]
            if index >= 29:
                cls.states[index] = clone(state)
                assert state.digest() == step['before']['digest']
            word = json.loads((D1 / step['word']).read_text())
            cls.words[index] = [tuple(x) for x in word['word']]
            cls.mechanics.apply_word(state, word)
        cls.fixture_jobs = {}
        cls.addClassCleanup(cls.stop_fixture_jobs)
        selected = [arg for arg in sys.argv[1:] if not arg.startswith('-')]
        includes_d3 = not selected or any(arg.endswith('RecordedTests') or 'test_z_d3' in arg
                                         for arg in selected)
        if includes_d3 and 'fork' in seamsx.mp.get_all_start_methods():
            cls.start_fixture_job('d3', check_d3_fixture)
        includes_rules = not selected or any(arg.endswith('RecordedTests') or 'test_y_every_rule' in arg
                                            for arg in selected)
        if includes_rules and 'fork' in seamsx.mp.get_all_start_methods():
            for index in (29, 31, 32):
                print(f'D1 preparing rule state={index}', flush=True)
                cls.compared_restorer(seams.SeamRestorer, index)
            seams.home_centroids()
            cls.start_fixture_job('rules', check_rule_suite)

    @classmethod
    def start_fixture_job(cls, name, check):
        ctx = seamsx.mp.get_context('fork')
        parent, child = ctx.Pipe(duplex=False)
        process = ctx.Process(target=fixture_child, args=(child, check, os.getpid()))
        cls.fixture_jobs[name] = process, parent
        process.start()
        child.close()
        tag, message = parent.recv()
        if tag != 'ready':
            raise AssertionError(message)

    @classmethod
    def fixture_result(cls, name, check):
        if name not in cls.fixture_jobs:
            return check()
        _, pipe = cls.fixture_jobs[name]
        tag, result = pipe.recv()
        if tag != 'ok':
            raise AssertionError(result)
        return result

    @classmethod
    def stop_fixture_jobs(cls):
        jobs = getattr(cls, 'fixture_jobs', {})
        cls.fixture_jobs = {}
        for process, pipe in jobs.values():
            if process.pid is not None:
                if process.is_alive() or process.exitcode != 0:
                    try:
                        os.killpg(process.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        if process.is_alive():
                            process.terminate()
                process.join(timeout=1)
                if process.is_alive():
                    os.killpg(process.pid, signal.SIGKILL)
                    process.join()
            process.close()
            pipe.close()

    @classmethod
    def compared_restorer(cls, kind, index, workers=None):
        """Independent fresh objects with parent-computed starting caches.

        A cold survey costs about 23 seconds on these states. Repeating it for
        every method/worker count would alone consume the acceptance budget.
        The parent computes each initial cache once; deep copies give every
        comparison its own dictionaries, arrays, domains and memo tables on
        canonical equal states. No cache produced by seamsx seeds a reference
        computation. Subsequent applies/undos run normally. The bulk replay
        and D3 checks below deliberately start with empty restorer caches.
        """
        names = ('_cls', '_sv', '_bal', '_dom')
        if index not in cls.initial_caches:
            reference = seams.SeamRestorer(clone(cls.states[index]), M)
            # These matrices depend on the menu and cap, never the state.
            reference._g = dict(cls.elements)
            sv = reference.survey()
            for c in sv:
                reference.balance(c)
            cls.elements.update(reference._g)
            cls.initial_caches[index] = tuple(getattr(reference, name) for name in names)
        options = {} if workers is None else {'workers': workers}
        restorer = kind(clone(cls.states[index]), M, **options)
        for name, value in zip(names, copy.deepcopy(cls.initial_caches[index])):
            setattr(restorer, name, value)
        # Matrix values are immutable; each restorer owns its dictionary.
        restorer._g = dict(cls.elements)
        return restorer

    def test_recorded_conj_parent_and_both_worker_counts(self):
        for index in CONJ_STEPS:
            step = self.steps[index]
            self.assertEqual(step['rule'], 'conj')
            baseline = outcome(self.compared_restorer(seams.SeamRestorer, index), 'conj')
            self.assertEqual(baseline[0], self.words[index])
            self.assertEqual(baseline[1], self.words[index])
            self.assertEqual(baseline[2], step['after']['digest'])
            self.assertEqual(baseline[3], step['after']['N'])
            for workers in (1, 3):
                with self.subTest(step=index, workers=workers):
                    r = self.compared_restorer(seamsx.SeamRestorer, index, workers=workers)
                    self.assertEqual(outcome(r, 'conj'), baseline)
        print(f'D1 conj comparisons: {len(CONJ_STEPS)} states, workers 1 and 3', flush=True)

    def test_y_every_rule_at_three_recorded_states(self):
        checked = self.fixture_result('rules', check_rule_suite)
        self.assertEqual(set(map(tuple, checked)), {(i, method) for i in (29, 31, 32) for method in RULES})
        print('D1 rule comparisons: 3 states, all 7 rules', flush=True)

    def test_full_bulk_rule_loop_29_through_42(self):
        order = tuple(self.record['config']['bulk_order'])
        self.assertEqual(list(order), runner.BULK_ORDER)
        r = seamsx.SeamRestorer(clone(self.states[29]), M, order=order, workers=3)
        methods = {'seam': r.seam, 'cover': r.cover, 'close': r.level1,
                   'conj': r.conj, 'lock': r.level2, 'improve': r.improve, 'shift': r.shift}
        for index in range(29, 43):
            with self.subTest(step=index):
                step = self.steps[index]
                self.assertEqual(r.st.digest(), step['before']['digest'])
                self.assertEqual(r.misalignment(), step['before']['N'])
                first = len(r.moves)
                for rule in r.order:
                    word = methods[rule]()
                    if word is not None:
                        break
                else:
                    self.fail(f'bulk loop stuck at step {index}')
                self.assertEqual(rule, step['rule'])
                self.assertEqual(word, self.words[index])
                self.assertEqual(r.moves[first:], word)
                self.assertEqual(r.st.digest(), step['after']['digest'])
                self.assertEqual(r.misalignment(), step['after']['N'])
                self.assertEqual([rec['grip'] for rec in r.st.journal], [c for c, _ in r.moves])
        print('D1 bulk loop: all 14 recorded words match', flush=True)

    def test_z_d3_fixture_conj(self):
        result = self.fixture_result('d3', check_d3_fixture)
        print(json.dumps(result), flush=True)


def check_d3_fixture():
    check = unittest.TestCase()
    mechanics = runner.Mechanics(CACHE)
    fixture = json.loads((D3 / 'steps-199-201.json').read_text())
    step = fixture['steps'][0]
    check.assertEqual(step['index'], 199)
    state = mechanics.load(D3 / 'bulk-198.json')
    check.assertEqual(state.digest(), step['before']['digest'])
    r = seamsx.SeamRestorer(state, M, workers=3)
    check.assertIs(r.DOMAINS, rim.Domains)
    check.assertEqual(r.misalignment(), step['before']['N'])
    started = time.perf_counter()
    trials = r._conj_trials(r.survey())
    word = r.conj()
    seconds = time.perf_counter() - started
    check.assertEqual(word, [tuple(x) for x in step['word']])
    check.assertEqual(word, [(315, 1), (79, 4), (315, 1)])
    check.assertEqual(r.moves, word)
    check.assertEqual(state.digest(), step['after']['digest'])
    check.assertEqual(r.misalignment(), step['after']['N'])
    position = next(i + 1 for i, trial in enumerate(trials) if trial[:2] == word[0])
    return {'fixture': 'd3-bulk-198', 'trial_count': len(trials),
            'winning_trial_position': position, 'wall_seconds': round(seconds, 6),
            'workers': r.workers}


def fixture_child(pipe, check, parent):
    """Overlap independent evidence; own and recover this test process group."""
    try:
        os.setsid()

        def parent_watch():
            while os.getppid() == parent:
                time.sleep(0.1)
            os.killpg(os.getpgrp(), signal.SIGKILL)

        threading.Thread(target=parent_watch, daemon=True).start()
        pipe.send(('ready', None))
        try:
            pipe.send(('ok', check()))
        except BaseException:
            pipe.send(('error', traceback.format_exc()))
    finally:
        pipe.close()


def rule_comparison(job):
    index, method = job
    print(f'D1 rule step={index} method={method} starting', flush=True)
    started = time.perf_counter()
    baseline = outcome(RecordedTests.compared_restorer(seams.SeamRestorer, index), method)
    parent_seconds = time.perf_counter() - started
    started = time.perf_counter()
    actual = outcome(RecordedTests.compared_restorer(seamsx.SeamRestorer, index, workers=3), method)
    new_seconds = time.perf_counter() - started
    unittest.TestCase().assertEqual(actual, baseline, f'step={index} rule={method}')
    print(f'D1 rule step={index} method={method} '
          f'parent_seconds={parent_seconds:.3f} new_seconds={new_seconds:.3f}', flush=True)
    return index, method, parent_seconds, new_seconds


def rule_branch(job):
    index, method, kind = job
    print(f'D1 rule step={index} method={method} branch={kind} starting', flush=True)
    started = time.perf_counter()
    restorer = (RecordedTests.compared_restorer(seams.SeamRestorer, index) if kind == 'parent'
                else RecordedTests.compared_restorer(seamsx.SeamRestorer, index, workers=3))
    result = outcome(restorer, method)
    seconds = time.perf_counter() - started
    print(f'D1 rule step={index} method={method} branch={kind} seconds={seconds:.3f}', flush=True)
    return index, method, kind, seconds, result


def rule_child(pipe, job):
    try:
        try:
            pipe.send(('ok', rule_branch(job)))
        except BaseException:
            pipe.send(('error', traceback.format_exc()))
    finally:
        pipe.close()


def parallel_rule_branches(jobs):
    """At most three isolated branches, each sending one small outcome."""
    if 'fork' not in seamsx.mp.get_all_start_methods():
        return [rule_branch(job) for job in jobs]
    ctx = seamsx.mp.get_context('fork')
    pending, results, next_job = {}, [], 0
    try:
        while pending or next_job < len(jobs):
            while next_job < len(jobs) and len(pending) < 3:
                parent, child = ctx.Pipe(duplex=False)
                process = ctx.Process(target=rule_child, args=(child, jobs[next_job]), daemon=True)
                process.start()
                child.close()
                pending[parent] = process
                next_job += 1
            ready = seamsx.wait(list(pending) + [p.sentinel for p in pending.values()])
            for pipe in list(pending):
                if pipe not in ready:
                    continue
                process = pending.pop(pipe)
                try:
                    message = pipe.recv()
                finally:
                    process.join()
                    process.close()
                    pipe.close()
                if message[0] == 'error':
                    raise AssertionError(message[1])
                results.append(message[1])
            for process in pending.values():
                if process.sentinel in ready:
                    raise AssertionError('rule-comparison child exited without a result')
        return results
    finally:
        for process in pending.values():
            if process.is_alive():
                process.terminate()
        for pipe, process in pending.items():
            process.join(timeout=1)
            if process.is_alive():
                process.kill()
                process.join()
            process.close()
            pipe.close()


def check_rule_suite():
    indexes = (29, 31, 32)
    for index in indexes:
        RecordedTests.compared_restorer(seams.SeamRestorer, index)
    seams.home_centroids()
    # Start the longest branches early; the old and new objects are independent.
    pairs = [(index, 'level2') for index in reversed(indexes)]
    pairs += [(index, method) for index in indexes for method in RULES
              if method not in ('level2', 'conj')]
    jobs = [(index, method, kind) for index, method in pairs for kind in ('parent', 'new')]
    results = parallel_rule_branches(jobs)
    check = unittest.TestCase()
    check.assertEqual({r[:3] for r in results}, set(jobs))
    outputs = {(index, method, kind): result for index, method, kind, _, result in results}
    for index, method in pairs:
        check.assertEqual(outputs[index, method, 'new'], outputs[index, method, 'parent'],
                          f'step={index} rule={method}')
    for index in indexes:
        rule_comparison((index, 'conj'))
    return [(index, method) for index in indexes for method in RULES]


if __name__ == '__main__':
    def timeout(signum, frame):
        # KeyboardInterrupt escapes unittest's subtest exception catcher, so
        # expiry stops the suite immediately and unwinds worker cleanup.
        raise KeyboardInterrupt('seamsx acceptance exceeded 1500 seconds')

    if hasattr(signal, 'SIGALRM'):
        signal.signal(signal.SIGALRM, timeout)
        signal.alarm(max(1, int(LIMIT - (time.perf_counter() - STARTED))))
    try:
        result = unittest.main(verbosity=2, exit=False, failfast=True).result
        passed = result.wasSuccessful()
    except KeyboardInterrupt as exc:
        print(f'Acceptance interrupted: {exc}', file=sys.stderr, flush=True)
        passed = False
    finally:
        RecordedTests.stop_fixture_jobs()
        if hasattr(signal, 'SIGALRM'):
            signal.alarm(0)
    elapsed = time.perf_counter() - STARTED
    print(f'Acceptance wall seconds: {elapsed:.3f} (limit {LIMIT})', flush=True)
    sys.exit(0 if passed and elapsed < LIMIT else 1)
