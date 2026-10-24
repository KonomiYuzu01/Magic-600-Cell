"""Focused HD2P/2 acceptance; hard 550 wall-second ceiling, all writes temporary.

python -I -B research/jumbling/restoration/13-tutorial-runner/scripts/test_height_descent_experiment.py

Reads fixed definition modules, the literal WORD36 source identity, and only the
named 7306 table. Never imports the old tests/census or scans run directories.
Exact synthetic geometry and pinned-table suffix evidence, not native evidence.
"""

import ast
import copy
import hashlib
import json
import os
from pathlib import Path
import resource
import signal
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

STARTED = time.monotonic()
sys.dont_write_bytecode = True
for _key in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS', 'NUMEXPR_NUM_THREADS'):
    os.environ[_key] = '1'

# The outer process enforces elapsed time even if a C extension ignores signals.
# The child reserves time for termination/reporting, so 550 s is a failed check,
# never a silent skip of expensive tests.
if __name__ == '__main__' and '--worker' not in sys.argv:
    try:
        proc = subprocess.run([sys.executable, '-I', '-B', str(Path(__file__).resolve()), '--worker'],
                              timeout=548, check=False)
        elapsed = time.monotonic() - STARTED
        print(f'Acceptance wall seconds: {elapsed:.3f} (ceiling 550)', flush=True)
        sys.exit(proc.returncode if elapsed < 550 else 1)
    except subprocess.TimeoutExpired:
        print('FAILED: acceptance exceeded its wall allowance', flush=True)
        sys.exit(1)


def timeout(*_):
    os.write(2, b'FAILED: focused acceptance time ceiling\n')
    os._exit(1)


if __name__ == '__main__':
    sys.argv.remove('--worker')
    signal.signal(signal.SIGALRM, timeout)
    signal.setitimer(signal.ITIMER_REAL, 546)
    resource.setrlimit(resource.RLIMIT_CPU, (545, 546))

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
TEMP = tempfile.TemporaryDirectory(prefix='height-descent-acceptance-')
OUT = Path(TEMP.name)
os.environ['JUMBLING_HOME_CACHE'] = str(OUT / 'home_centroids.npy')


def audit(event, args):
    """Fail the suite if it attempts any filesystem write outside its temp root."""
    paths = []
    if event == 'open':
        name, mode, flags = args
        if isinstance(name, (str, bytes, os.PathLike)) and (
                (isinstance(mode, str) and any(x in mode for x in 'wax+')) or
                flags & (os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC | os.O_APPEND)):
            paths = [name]
    elif event in ('os.mkdir', 'os.remove', 'os.rmdir'):
        # shutil's fd-relative cleanup is separately confined by TemporaryDirectory.
        if isinstance(args[0], (str, bytes, os.PathLike)) and Path(os.fsdecode(args[0])).is_absolute():
            paths = [args[0]]
    elif event == 'os.rename':
        paths = list(args[:2])
    for name in paths:
        p = Path(os.fsdecode(name)).resolve()
        if not p.is_relative_to(OUT):
            raise AssertionError('test attempted a write outside its temporary output: ' + str(p))


sys.addaudithook(audit)
import check_height_descent_experiment as C  # noqa: E402
import height_descent_experiment as E  # noqa: E402

# Copied literally from test_check_contracts.py:48-51. Do not import that module.
WORD36 = [[0, 12], [74, 8], [188, 4], [74, 4], [188, 8], [188, 4], [188, 4],
          [74, 8], [188, 8], [74, 4], [188, 8], [178, 1], [188, 4], [74, 8],
          [188, 4], [74, 4], [188, 8], [188, 8], [188, 4], [74, 8], [188, 8],
          [74, 4], [178, 1], [0, 45], [14, 12], [12, 12]]


class ToyState:
    def __init__(self, name):
        self.name, self.trial_marks = name, []

    def digest(self):
        return hashlib.sha256(self.name.encode()).hexdigest()


class Oracle:
    """Tiny explicit transition graph, with exact, declared sets and outcomes.

    Absent edges are identity. None denotes an uncertified/rejected letter.
    This has no geometrical claims; it isolates ordering and control flow.
    """
    caps = (0, 1, 2, 3)
    items = (1, 2, 12, 13, 14)
    jumble = (12, 13, 14)

    def __init__(self):
        self.nodes = {'s': {'hist': {'2': 10, '4': 5}, 'blocked': {2: {9, 10}, 3: {11}}}}
        self.edges, self.rejected, self.moves = {}, set(), []
        self.insides = {0: {9, 10, 11}, 1: {9}, 2: {1}, 3: {2}}
        self.meeting = {(0, 1), (0, 2), (1, 2)}
        self.full_sets = []

    def node(self, name, hist=None, blocked=None):
        self.nodes[name] = dict(hist=dict(self.nodes['s']['hist'] if hist is None else hist),
                                blocked=copy.deepcopy(self.nodes['s']['blocked'] if blocked is None else blocked))
        return name

    def edge(self, start, c, x, end):
        self.edges[start, c, x] = end

    def clone(self, state):
        return copy.deepcopy(state)

    def histogram(self, state):
        return dict(self.nodes[state.name]['hist'])

    def classify(self, state, cap):
        blockers = self.nodes[state.name]['blocked'].get(cap, set())
        return SimpleNamespace(blockers=set(blockers), inside=set(self.insides[cap]))

    @staticmethod
    def clean(cl):
        return not cl.blockers

    @staticmethod
    def certificate(cl):
        return min(cl.blockers) if cl.blockers else None

    @staticmethod
    def inside(cl):
        return set(cl.inside)

    def meets(self, c, d):
        return tuple(sorted((c, d))) in self.meeting

    def straddlers(self, state, d):
        self.full_sets.append((state.name, d))
        return set(self.nodes[state.name]['blocked'].get(d, set()))

    def predict(self, state, c, x, cl, hist):
        target = self.edges.get((state.name, c, x), state.name)
        return None if target is None else dict(self.nodes[target]['hist'])

    def move(self, state, c, x):
        out = self.clone(state)
        out.trial_marks.append((c, x))
        self.moves.append((state.name, c, x))
        target = self.edges.get((state.name, c, x), state.name)
        if target is None or (state.name, c, x) in self.rejected or not self.clean(self.classify(state, c)):
            return None
        out.name = target
        return out

    def blocked(self, state):
        return len(self.nodes[state.name]['blocked'])

    def off(self, state):
        return set(range(sum(self.histogram(state).values())))

    def measure(self, state, initial_lattice, ever):
        hist = self.histogram(state)
        loss = self.off(state) & initial_lattice
        ever = ever | loss
        bk, q = self.blocked(state), 177121 * hist.get('6', 0) + hist.get('4', 0)
        return (dict(digest=state.digest(), heights=hist, maximum=max(map(int, hist), default=0),
                     n2_n4_n6=[hist.get(str(h), 0) for h in (2, 4, 6)], Q=q, Bk=bk,
                     M=601 * q + bk, N=2 * sum(hist.values()), off=len(self.off(state)),
                     lattice_loss=len(loss), ever_lattice_loss=len(ever)), ever)

    snapshot = measure

    def apply(self, state, c, x):
        trial = self.move(state, c, x)
        C.require(trial is not None, 'inadmissible or uncertain letter')
        state.name, state.trial_marks = trial.name, trial.trial_marks


def reference(state, oracle):
    """Simple exhaustive reference: replay all candidates, no certificate prune,
    prediction, cached eligibility or search implementation helpers.
    """
    hist = oracle.histogram(state)
    height = lambda st: max(map(int, oracle.histogram(st)), default=0)
    score = lambda st: 177121 * oracle.histogram(st).get('6', 0) + oracle.histogram(st).get('4', 0)
    if set(hist) <= {'2'}:
        return 'HANDOFF', None
    bound, q = height(state), score(state)
    clean = [c for c in oracle.caps if oracle.clean(oracle.classify(state, c))]
    for c in clean:
        for x in oracle.jumble:
            end = oracle.move(state, c, x)
            if end is not None and height(end) <= bound and score(end) < q:
                return 'SINGLE', [[c, x]]
    for c in clean:
        for x in oracle.items:
            middle = oracle.move(state, c, x)
            if middle is None or height(middle) > bound:
                continue
            for d in oracle.caps:
                if d == c or not oracle.meets(c, d):
                    continue
                if not oracle.straddlers(state, d) <= oracle.inside(oracle.classify(state, c)):
                    continue
                if not oracle.clean(oracle.classify(middle, d)):
                    continue
                candidates = []
                for y in oracle.jumble:
                    end = oracle.move(middle, d, y)
                    if end is not None and height(end) <= bound:
                        candidates.append((score(end), y))
                if candidates and min(candidates)[0] < q:
                    return 'CL2', [[c, x], [d, min(candidates)[1]]]
            end = middle
            if end is None or height(end) > bound:
                continue
            eh = oracle.histogram(end)
            if all(eh.get(str(k), 0) == hist.get(str(k), 0) for k in (6, 4)) and \
                    oracle.blocked(end) < oracle.blocked(state):
                return 'PEEL', [[c, x]]
    return 'STUCK', None


def record(engine, state, word, family, initial_lattice=None):
    if initial_lattice is None:
        initial_lattice = set(range(C.P)) - engine.off(state)
    before, ever = engine.measure(state, initial_lattice, set())
    trial, states = state, []
    for c, x in word:
        trial = engine.move(trial, c, x)
        if trial is None:
            raise AssertionError('bad test word')
        states.append(trial)
    decision = dict(family=family, word=word, state=trial, states=states)
    row, after_ever = E.step_record(decision, engine, before, initial_lattice, ever, 1, 0,
                                   E.Ledger(), lambda: {'cpu_seconds': 0, 'wall_seconds': 0})
    return row, initial_lattice, ever, after_ever


class OrderingTests(unittest.TestCase):
    def compare(self, oracle, expected):
        st = ToyState('s')
        original = copy.deepcopy(st.__dict__)
        trace = []
        ledger = E.Ledger(sink=trace.append)
        answer = E.choose_step(st, oracle, ledger)
        actual = answer.get('family', answer['result']), answer.get('word')
        self.assertEqual(actual, expected)
        self.assertEqual(reference(ToyState('s'), oracle), actual)
        self.assertEqual(st.__dict__, original, 'rejected or accepted trials changed the input')
        return answer, ledger, trace

    def pair(self, c=0, x=1, d=1):
        o = Oracle()
        o.node('middle', blocked={3: {11}})
        o.node('low', {'2': 12, '4': 3})
        o.edge('s', c, x, 'middle')
        o.edge('middle', d, 13, 'low')
        return o

    def test_01_single_precedes_pair_and_peel(self):
        o = self.pair()
        o.node('single', {'2': 10, '4': 4})
        o.edge('s', 1, 14, 'single')
        _, ledger, _ = self.compare(o, ('SINGLE', [[1, 14]]))
        self.assertEqual(ledger.complete, [])

    def test_02_single_cap_then_item(self):
        o = Oracle()
        o.node('lower', {'2': 10, '4': 4})
        for c, x in ((0, 13), (0, 14), (1, 12)):
            o.edge('s', c, x, 'lower')
        self.compare(o, ('SINGLE', [[0, 13]]))

    def test_03_pair_retained_before_jumble(self):
        o = self.pair(x=2)
        o.edge('s', 0, 12, 'middle')
        _, ledger, _ = self.compare(o, ('CL2', [[0, 2], [1, 13]]))
        self.assertEqual(ledger.complete, ['SINGLE'])

    def test_04_pair_cap_before_opener_item(self):
        o = self.pair(x=14)
        o.edge('s', 1, 1, 'middle')
        self.compare(o, ('CL2', [[0, 14], [1, 13]]))

    def test_05_first_second_cap_then_best_y_and_tie(self):
        for tie in (False, True):
            with self.subTest(tie=tie):
                o = self.pair()
                o.node('better', {'2': 20, '4': 2})
                o.node('best', {'2': 20, '4': 1})
                o.edge('middle', 1, 14, 'low' if tie else 'better')
                o.edge('middle', 2, 12, 'best')
                self.compare(o, ('CL2', [[0, 1], [1, 13 if tie else 14]]))

    def test_06_complete_exhaustion_then_peel_and_stuck(self):
        o = Oracle()
        _, ledger, _ = self.compare(o, ('STUCK', None))
        self.assertEqual(ledger.complete, ['SINGLE', 'CL2', 'PEEL'])
        o.node('peel', blocked={3: {11}})
        o.edge('s', 0, 2, 'peel')
        _, ledger, _ = self.compare(o, ('PEEL', [[0, 2]]))
        self.assertEqual(ledger.complete, ['SINGLE'])

    def test_07_pending_best_y_budget_never_falls_through(self):
        o, trace = self.pair(), []
        st = ToyState('s')
        ledger = E.Ledger(sink=trace.append)
        def interrupt():
            at = ledger.frontier or {}
            if at.get('phase') == 'second_item' and at.get('y') == 14:
                raise C.Budget('forced unfinished second-item enumeration')
        ledger.tick = interrupt
        with self.assertRaises(C.Budget):
            E.choose_step(st, o, ledger)
        self.assertEqual(ledger.complete, ['SINGLE'])
        self.assertFalse(any(r['at']['family'] == 'PEEL' for r in trace))
        self.assertEqual(ledger.frontier['status'], 'pending')
        self.assertEqual((st.name, st.trial_marks), ('s', []))

    def test_08_blocker_certificate_full_containment_and_reclassification(self):
        o = Oracle()
        o.insides[0] = {9}  # Certificate moves, the other blocker does not.
        o.node('opened', blocked={3: {11}})
        o.node('low', {'2': 8, '4': 1})
        o.edge('s', 0, 1, 'opened')
        o.edge('opened', 2, 12, 'low')
        _, ledger, _ = self.compare(o, ('PEEL', [[0, 1]]))
        self.assertGreater(ledger.counts['CL2:blocker_containment'], 0)
        self.assertNotIn(('s', 3), o.full_sets, 'certificate must exclude before full S_d')
        o = Oracle()
        o.node('still_blocked')
        o.edge('s', 0, 1, 'still_blocked')
        _, ledger, _ = self.compare(o, ('STUCK', None))
        self.assertGreater(ledger.counts['CL2:intermediate_blocked'], 0)

    def test_09_prefix_height_and_height8_endpoints(self):
        o = self.pair()
        o.nodes['middle']['hist'] = {'2': 10, '6': 1}
        o.node('eight', {'2': 7, '8': 1})
        o.edge('s', 0, 12, 'eight')
        _, ledger, _ = self.compare(o, ('STUCK', None))
        self.assertGreater(ledger.counts['SINGLE:height_prune'], 0)
        self.assertGreater(ledger.counts['CL2:opener_prune'], 0)

    def test_10_peel_count_changes_and_global_net_blocking(self):
        for hist, blocked in (({'2': 10, '4': 6}, {3: {11}}),
                              ({'2': 10, '4': 5, '6': 1}, {3: {11}}),
                              ({'2': 10, '4': 5}, {2: {9}, 3: {11}}),
                              ({'2': 10, '4': 5}, {0: {8}, 1: {7}, 3: {11}})):
            with self.subTest(hist=hist, blocked=blocked):
                o = Oracle()
                o.meeting.clear()
                o.node('bad', hist, blocked)
                o.edge('s', 0, 1, 'bad')
                self.compare(o, ('STUCK', None))

    def test_11_rejected_opener_isolation(self):
        o = self.pair(x=2)
        o.edge('s', 0, 1, 'middle')
        o.rejected.add(('s', 0, 1))
        _, ledger, _ = self.compare(o, ('CL2', [[0, 2], [1, 13]]))
        self.assertEqual(ledger.counts['CL2:opener_inadmissible'], 1)

    def test_12_current_height_recomputed_and_full_handoff(self):
        o = Oracle()
        o.nodes['s']['hist'] = {'2': 10, '6': 1}
        o.node('four', {'2': 10, '4': 1})
        o.node('back_up', {'2': 10, '6': 1})
        o.edge('s', 0, 12, 'four')
        o.edge('four', 0, 1, 'back_up')
        answer, _, _ = self.compare(o, ('SINGLE', [[0, 12]]))
        later = E.Ledger()
        E.choose_step(answer['state'], o, later)
        self.assertGreater(later.counts['CL2:opener_prune'], 0)
        for hist in ({}, {'2': 7}):
            o.nodes['s']['hist'] = hist
            self.compare(o, ('HANDOFF', None))
        o.nodes['s']['hist'] = {'8': 1}
        with self.assertRaisesRegex(ValueError, 'maximum'):
            E.choose_step(ToyState('s'), o, E.Ledger())

    def test_13_differential_candidate_grid(self):
        # Every opener position, target and tie pattern in the tiny domain.
        for c in (0, 1):
            for x in Oracle.items:
                for d in (0, 1, 2):
                    if c == d:
                        continue
                    for y in Oracle.jumble:
                        o = Oracle()
                        o.insides[1] = {9, 10, 11}
                        o.node('mid', blocked={3: {11}})
                        o.node('end', {'2': 30, '4': 2})
                        o.edge('s', c, x, 'mid')
                        o.edge('mid', d, y, 'end')
                        expected = reference(ToyState('s'), o)
                        self.compare(o, expected)

    def test_14_same_opener_pair_precedes_peel_but_earlier_peel_precedes_later_pair(self):
        o = self.pair(x=2)
        self.compare(o, ('CL2', [[0, 2], [1, 13]]))
        o.node('earlier_peel', blocked={3: {11}})
        o.edge('s', 0, 1, 'earlier_peel')
        _, ledger, trace = self.compare(o, ('PEEL', [[0, 1]]))
        self.assertEqual(ledger.complete, ['SINGLE'])
        self.assertFalse(any(e['at'].get('phase') == 'opener' and e['at'].get('x') == 2 for e in trace))

    def test_15_all_48_literal_ties_and_height6_opener_count_increase(self):
        o = self.pair()
        o.items, o.jumble = tuple(range(1, 60)), tuple(range(12, 60))
        o.nodes['s']['hist'] = {'2': 10, '4': 5, '6': 2}
        o.nodes['middle']['hist'] = {'2': 10, '4': 9, '6': 2}
        o.nodes['low']['hist'] = {'2': 15, '4': 100, '6': 1}
        for y in o.jumble:
            o.edge('middle', 1, y, 'low')
        self.compare(o, ('CL2', [[0, 1], [1, 12]]))

    def test_16_height6_single_allows_n4_increase_at_cap_236(self):
        o = Oracle()
        o.caps, o.insides = (236,), {236: {9, 10}}
        o.nodes['s'] = dict(hist={'2': 99634, '4': 623, '6': 87}, blocked={})
        o.node('height_four', {'2': 99634, '4': 710}, blocked={})
        o.edge('s', 236, 12, 'height_four')
        self.compare(o, ('SINGLE', [[236, 12]]))


class RecordTests(unittest.TestCase):
    def setUp(self):
        self.o = Oracle()
        self.o.node('end', {'2': 20, '4': 2})
        self.o.edge('s', 0, 12, 'end')
        self.st = ToyState('s')
        self.row, self.lattice, self.ever, _ = record(self.o, self.st, [[0, 12]], 'SINGLE')

    def replay(self, row):
        return C.replay_step(self.o, self.st, row, self.lattice, self.ever, 1, 0)

    def test_01_complete_record_and_loss_measurements(self):
        state, ever = self.replay(self.row)
        self.assertEqual(state.name, 'end')
        self.assertEqual(len(ever), 7)
        self.o.node('mid', {'2': 30, '4': 5}, blocked={3: {11}})
        self.o.node('back', {'2': 10, '4': 1})
        self.o.edge('s', 0, 1, 'mid')
        self.o.edge('mid', 1, 12, 'back')
        row, lattice, ever, _ = record(self.o, self.st, [[0, 1], [1, 12]], 'CL2')
        end, seen = C.replay_step(self.o, self.st, row, lattice, ever, 1, 0)
        self.assertEqual(row['after']['lattice_loss'], 0)
        self.assertEqual(row['after']['ever_lattice_loss'], 20)
        self.assertEqual(len(seen), 20)

    def test_02_forged_digests_histograms_and_measurements(self):
        variants = []
        for key in ('input_digest', 'output_digest'):
            row = copy.deepcopy(self.row)
            row[key] = '0' * 64
            variants.append(row)
        row = copy.deepcopy(self.row)
        row['prefix_digests'][0] = '0' * 64
        variants.append(row)
        for key in ('heights', 'N', 'off', 'Q', 'Bk', 'M', 'lattice_loss', 'ever_lattice_loss'):
            row = copy.deepcopy(self.row)
            row['prefixes'][0][key] = {'2': 1} if key == 'heights' else -1
            variants.append(row)
        for row in variants:
            with self.subTest(row=row):
                with self.assertRaises(ValueError):
                    self.replay(row)

    def test_03_inadmissible_and_uncertain_letters(self):
        self.o.edges['s', 0, 12] = None
        with self.assertRaisesRegex(ValueError, 'inadmissible or uncertain'):
            self.replay(self.row)

    def test_04_contract_guards_include_all_heights(self):
        before = self.row['before']
        for hist in ({'2': 15, '6': 1}, {'2': 15, '8': 1}):
            after = dict(self.row['after'], heights=hist)
            with self.assertRaisesRegex(ValueError, 'prefix maximum'):
                C.contract(before, [after], [[0, 12]], 'SINGLE')
        middle = dict(before, heights={'6': 1})
        with self.assertRaisesRegex(ValueError, 'prefix maximum'):
            C.contract(before, [middle, self.row['after']], [[0, 1], [1, 12]], 'CL2')

    def test_05_checker_rejects_peel_counts_and_global_count(self):
        before = self.row['before']
        for hist, bk in (({'2': 10, '4': 4}, 1), ({'2': 10, '4': 5, '6': 1}, 1),
                         (before['heights'], 2), (before['heights'], 3)):
            with self.subTest(hist=hist, bk=bk):
                after = dict(before, heights=hist, Bk=bk)
                with self.assertRaises(ValueError):
                    C.contract(before, [after], [[0, 1]], 'PEEL')
        # Moving poses is allowed; only the two counts and the global Bk matter.
        C.contract(before, [dict(before, Bk=1, digest='different')], [[0, 1]], 'PEEL')

    def test_06_family_shapes(self):
        for family, word in (('SINGLE', [[0, 1]]), ('CL2', [[0, 1], [0, 12]]),
                             ('CL2', [[0, 1], [1, 2]]), ('PEEL', [[0, 0]]),
                             ('CL2', [[0, 1], [1, 12], [0, 2]]), ('PEEL', [[True, 1]])):
            with self.assertRaises(ValueError):
                C.word_shape(word, family)

    def test_07_truncated_and_unverified_records(self):
        path = OUT / 'records.jsonl'
        start, final = {'type': 'START'}, {'type': 'FINAL', 'verification': {'status': 'complete'}}
        for rows, newline in (([start], True), ([start, final], False),
                              ([start, dict(final, verification={'status': 'incomplete'})], True),
                              ([start, final, self.row], True)):
            path.write_text('\n'.join(C.canonical(r) for r in rows) + ('\n' if newline else ''))
            with self.assertRaises(ValueError):
                C.read_events(path)
        path.write_text(C.canonical(start) + '\n{"type":')
        with self.assertRaises(ValueError):
            C.read_events(path)

    def test_08_ledger_integrity_and_frontier(self):
        path = OUT / 'ledger.jsonl'
        with path.open('w') as f:
            ledger = E.Ledger(sink=lambda row: C.emit(f, row))
            E.choose_step(ToyState('s'), Oracle(), ledger)
        row = {'type': 'FINAL', 'search': ledger.snapshot()}
        C.check_ledger(path, [row])
        forged = copy.deepcopy(row)
        forged['search']['entries'] -= 1
        with self.assertRaisesRegex(ValueError, 'ledger summary'):
            C.check_ledger(path, [forged])

    def test_09_checker_has_no_search_import_or_old_contract(self):
        tree = ast.parse((HERE / 'check_height_descent_experiment.py').read_text())
        imports = []
        for n in ast.walk(tree):
            if isinstance(n, ast.Import):
                imports.extend(a.name for a in n.names)
            elif isinstance(n, ast.ImportFrom):
                imports.append(n.module or '')
        self.assertFalse(any(name in ('height_descent_experiment', 'check_contracts', 'runner') for name in imports))
        self.assertNotIn('seams', sys.modules)
        for key in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
            self.assertEqual(os.environ[key], '1')

    def test_10_interrupted_initialization_is_incomplete_budget(self):
        output = OUT / 'early-budget'
        output.mkdir()
        class TestLimits:
            def check(self, search=False):
                pass
            def times(self):
                return dict(cpu_seconds=0, wall_seconds=0)
        args = SimpleNamespace(case=7306, cpu_seconds=3300, search_cpu_seconds=2400, max_steps=0)
        with patch.object(C, 'Runtime', side_effect=C.Budget('forced import interruption')):
            self.assertEqual(E.run(args, TestLimits(), output), 0)
        final = C.read_events(output / 'events.jsonl', allow_pending=True)[-1]
        self.assertEqual(final['result'], 'BUDGET')
        self.assertEqual(final['verification']['status'], 'incomplete')
        self.assertIsNone(final['last'])
        self.assertIsNone(C.parse((output / 'manifest.json').read_bytes())['identity'])
        with self.assertRaises(ValueError):
            C.read_events(output / 'events.jsonl')


class ExactTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        started = time.monotonic()
        cls.source_hashes = {p: C.sha((C.ROOT / p).read_bytes()) for p in C.SOURCE_PATHS}
        cls.rt = C.Runtime(OUT)
        cls.engine = E.ExactEngine(cls.rt)
        cls.checker = C.ReplayEngine(cls.rt)
        st = cls.rt.sim.State(cls.rt.ctx, menu=cls.rt.menu, filtered=False)
        for c, x in WORD36:
            st = cls.engine.move(st, c, x)
            assert st is not None
        cls.z = st
        cls.a = cls.engine.move(cls.z, 12, 14)
        cls.b = cls.engine.move(cls.a, 35, 13)
        print(f'Exact fixture initialized in {time.monotonic() - started:.3f}s', flush=True)

    def test_01_literal_fixture_identity_and_height_transitions(self):
        # Confirm the copied literal without importing or evaluating that test module.
        source = ast.parse((HERE / 'test_check_contracts.py').read_text())
        literal = next(ast.literal_eval(n.value) for n in source.body if isinstance(n, ast.Assign)
                       and any(isinstance(t, ast.Name) and t.id == 'WORD36' for t in n.targets))
        self.assertEqual(literal, WORD36)
        for st, digest, hist in (
            (self.z, '8733a0aafd8e24ea1da990ebab0347074c20b11cf61d9ed04d34d25ef6f67261', {'2': 5799, '6': 1}),
            (self.a, '7cd868f6466953eb6ea1e0d466ed66857d34edaea7dd7c554c1e74975005541b', {'2': 3097, '4': 1}),
            (self.b, '3ab25d214b6c4ad759ac7158a0fc1cabae1584aa711d3f757af7b91c43971837', {'2': 5405})):
            self.assertEqual(st.digest(), digest)
            self.assertEqual(self.engine.histogram(st), hist)
            self.assertEqual(self.checker.histogram(st), hist)
        row, lattice, ever, _ = record(self.engine, self.z, [[12, 14]], 'SINGLE')
        replayed, _ = C.replay_step(self.checker, self.z, row, lattice, ever, 1, 0)
        self.assertEqual(replayed.digest(), self.a.digest())

    def test_02_complete_single_search_and_independent_replay(self):
        original = self.a.digest()
        ledger = E.Ledger()
        answer = E.choose_step(self.a, self.engine, ledger)
        self.assertEqual(answer['result'], 'STEP')
        self.assertEqual(answer['family'], 'SINGLE')
        self.assertGreater(ledger.counts['SINGLE:no_descent'], 0)
        self.assertEqual(ledger.complete, [])
        row, lattice, ever, _ = record(self.engine, self.a, answer['word'], 'SINGLE')
        replayed, _ = C.replay_step(self.checker, self.a, row, lattice, ever, 1, 0)
        self.assertTrue(C.handoff(self.checker.histogram(replayed)))
        self.assertEqual(self.a.digest(), original)

    def test_03_exact_predictions_all_items_and_compacted_pose_ids(self):
        # All 59 items on the small fixture: actual apply vs full predicted hist.
        # Lattice pieces enter height 2; accepting an off-only delta would fail.
        saw_lattice = False
        for state, cap in ((self.z, 12), (self.a, 35)):
            cl = self.engine.classify(state, cap)
            self.assertTrue(self.engine.clean(cl))
            hist = self.engine.histogram(state)
            saw_lattice |= any(state._poses[int(state.pose_id[p])].kidx >= 0 for p in cl.inside)
            for item in range(1, 60):
                predicted = self.engine.predict(state, cap, item, cl, hist)
                actual = self.engine.move(state, cap, item)
                self.assertIsNotNone(actual)
                self.assertEqual(predicted, self.checker.histogram(actual), (cap, item))
        self.assertTrue(saw_lattice, 'the differential checks must include lattice collateral')

    def test_04_two_distinct_7306_relaxation_witnesses(self):
        started = time.monotonic()
        state = C.load_case(7306, self.rt, self.engine)
        self.assertEqual(state.journal, [])
        # No trajectory search on this table. Each word starts at the pinned state.
        for word, n4, end_hist, digest in C.PROBES[7306][:2]:
            with self.subTest(word=word):
                C.word_shape(word, 'CL2')
                self.assertTrue(self.checker.meets(word[0][0], word[1][0]))
                trial, prefixes = self.checker.clone(state), []
                hist = self.checker.histogram(trial)
                initial_q = C.qvalue(hist)
                for c, x in word:
                    cl = self.engine.classify(trial, c)
                    self.assertTrue(self.engine.clean(cl))
                    predicted = self.engine.predict(trial, c, x, cl, hist)
                    self.checker.apply(trial, c, x)
                    hist = self.checker.histogram(trial)
                    self.assertEqual(predicted, hist)
                    self.assertLessEqual(C.maximum(hist), 4)
                    prefixes.append(hist)
                self.assertEqual(prefixes[0]['4'], n4)
                self.assertEqual(hist, end_hist)
                self.assertLess(C.qvalue(hist), initial_q)
                self.assertTrue(trial.digest().startswith(digest))
                self.assertEqual(state.digest(), C.CASES[7306][2])
        print(f'Two pinned 7306 diagnostic replays: {time.monotonic() - started:.3f}s', flush=True)

    def test_05_synthetic_end_to_end_files_and_independent_reload(self):
        output = OUT / 'synthetic-run'
        output.mkdir()
        table = OUT / 'synthetic-input.json'
        C.export_table(self.a, self.rt, table)
        pin = (str(table), C.sha(table.read_bytes()), self.a.digest(), (3097, 1, 0))
        class TestLimits:
            def check(self, search=False):
                pass
            def times(self):
                return dict(cpu_seconds=time.process_time(), wall_seconds=time.monotonic() - STARTED)
        args = SimpleNamespace(case=7306, cpu_seconds=3300, search_cpu_seconds=2400, max_steps=1)
        # Only the test's synthetic table is substituted. The production harness
        # has no arbitrary-input CLI option, and its six pins stay immutable.
        with patch.object(C, 'TABLE_ROOT', ''), patch.dict(C.CASES, {7306: pin}), \
                patch.object(C, 'PROBES', {}):
            self.assertEqual(E.run(args, TestLimits(), output), 0)
            final = C.read_events(output / 'events.jsonl')[-1]
            self.assertEqual(final['result'], 'HANDOFF')
            self.assertEqual(final['steps'], 1)
            report = C.verify_output(output, self.rt)
            self.assertTrue(report['passed'])
            self.assertTrue(report['handoff'])
            # A forged terminal histogram cannot manufacture a handoff.
            forged = copy.deepcopy(final)
            forged['last']['heights'] = {'8': 1}
            E.replace_final(output / 'events.jsonl', forged)
            with self.assertRaises(ValueError):
                C.verify_output(output, self.rt)

    def test_06_input_hash_schema_assignment_and_digest_rejections(self):
        for case in (7302, 7303, 7304, 7305, 7306, 7307):
            pin = C.input_identity(case)
            raw = (C.ROOT / pin['table']).read_bytes()
            self.assertEqual(C.sha(raw), pin['sha256'])
            table = C.parse(raw)
            self.assertEqual(len(table['pose_id']), C.P)
            self.assertEqual(table['digest'], pin['digest'])
        path = OUT / 'input-validation.json'
        C.export_table(self.a, self.rt, path)
        original = C.parse(path.read_bytes())
        with self.assertRaisesRegex(ValueError, 'file hash'):
            C.load_table(path, self.rt, expected_sha='0' * 64)
        for mutate in (lambda d: d['pose_id'].pop(), lambda d: d['pose_id'].__setitem__(0, True),
                       lambda d: d.__setitem__('digest', '0' * 64), lambda d: d.__setitem__('unknown', 1)):
            doc = copy.deepcopy(original)
            mutate(doc)
            C.write_json(path, doc)
            with self.assertRaises(ValueError):
                C.load_table(path, self.rt)

    def test_07_all_caps_meet_predicate_and_source_preservation(self):
        for c in (0, 12, 35, 208, 487):
            self.assertEqual(sum(self.engine.meets(c, d) for d in range(600) if d != c), 80)
            self.assertTrue(all(self.engine.meets(c, d) == self.checker.meets(c, d) for d in range(600)))
        self.assertEqual(self.source_hashes, {p: C.sha((C.ROOT / p).read_bytes()) for p in C.SOURCE_PATHS})
        self.assertFalse((OUT / 'home_centroids.npy').exists())


if __name__ == '__main__':
    try:
        suite = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        print(f'Focused tests: {result.testsRun}; failures: {len(result.failures)}; '
              f'errors: {len(result.errors)}; worker wall seconds: {time.monotonic() - STARTED:.3f}', flush=True)
    finally:
        TEMP.cleanup()
        signal.setitimer(signal.ITIMER_REAL, 0)
    sys.exit(0 if result.wasSuccessful() else 1)
