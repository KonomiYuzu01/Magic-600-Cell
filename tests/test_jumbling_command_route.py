"""KN5: the jumbling oracle route, M5's differential harness through the command layer with Jumble on.

LP3's plan asks, for PHS.23's acceptance item "Oracle equality at every committed step", that M5's oracle run through
the command layer with Jumble on (C8). `rust/c600-command/src/bin/c600-jumble-route.rs` serves M5's adapter protocol,
so `differential/candidate/rust_jumble.py` is its client, through `jumble.on` and `jumble.twist.menu`: every step is
one press of the menu item the harness names, at the current revision. `harness.compare` runs J1 and the route step
by step on the first 70 records of each W-J fixture (every record with JUMBLING_FULL_REPLAY=1) and compares the
status, the moved count, the moved set, the pose digest and the lattice digest at every step. Then every move is undone
with `session.undo` back to the entry and redone with `session.redo` to the end: each undo and redo reaches J1's pose
and lattice digests of its prefix and moves the pieces its move moved (JI.27). Seventy moves pass the layer's bound
of 64 chained pose moves, so the moves, the undos and the redos each cross a replay of the jumbling journal from
solved. Refusals: a blocked item, a record off the menu and a malformed record change nothing, and the next record
still moves.

Not compared: the identity item, which the command layer refuses with `KN2:input.recipe_shape` (a draft of
`jumble.twist.menu`) where J1 moves its inside pieces onto their own poses; the route answers it with an input error,
as it answers free mode, a budget and a protected piece: it presses items of the pinned menu, classified exactly, with
no protected piece. The fixtures' journals hold no identity record (their angles are 72 to 180 degrees); one would
stop the comparison with that input error, not pass it.

The route is built with `cargo build --release --locked --offline` in rust/ (into the ignored rust/target). A missing
Rust toolchain or NumPy fails the check: a test that did not run is no pass (PHS.63). A pass is source and fixture
evidence, not Windows/DirectX, input, long-session or performance evidence; the printed seconds are no performance
claim.
"""

import sys

sys.dont_write_bytecode = True

from dataclasses import replace
import json
import os
from pathlib import Path
import shutil
import subprocess
from time import perf_counter
import unittest

ROOT = Path(__file__).resolve().parents[1]
RUST = ROOT / 'rust'
JUMBLING = ROOT / 'research' / 'jumbling'
FIXTURE_DIR = JUMBLING / 'fixtures'
ROUTE = 'c600-jumble-route'
BINARY = RUST / 'target' / 'release' / (f'{ROUTE}.exe' if os.name == 'nt' else ROUTE)
BUILD_SECONDS = 1200
FULL = os.environ.get('JUMBLING_FULL_REPLAY') == '1'

# Past the layer's bound of 64 chained pose moves, so that the moves, the undos and the redos each cross a replay of
# the jumbling journal from solved.
RECORDS = 70
# The refusals follow this many I_a records, as M5's do.
REFUSED_AT = 30
RECIPE_SHAPE = 'KN2:input.recipe_shape'

_SECONDS = {}
_ENGINES = {}


def build():
    """Build the route in release and return its path."""
    cargo = shutil.which('cargo')
    if cargo is None:
        raise RuntimeError('cargo is not on PATH: the route was not built, so nothing was compared (PHS.63)')
    command = [cargo, 'build', '--release', '--locked', '--offline', '-p', 'c600-command', '--bin', ROUTE]
    done = subprocess.run(command, cwd=RUST, capture_output=True, text=True, timeout=BUILD_SECONDS)
    if done.returncode != 0:
        raise RuntimeError(f'cargo build exited with {done.returncode}:\n{done.stderr[-4000:]}')
    if not BINARY.is_file():
        raise RuntimeError(f'cargo build left no route at {BINARY.relative_to(ROOT)}')
    return BINARY


def setUpModule():
    global H, R
    try:
        import numpy  # noqa: F401
    except ImportError as error:
        raise RuntimeError('NumPy is required: the jumbling comparison did not run (PHS.63)') from error
    sys.path.insert(0, str(JUMBLING))
    from differential import harness as H
    from differential.candidate import rust_jumble as R
    from differential.j1_adapter import J1Engine
    started = perf_counter()
    binary = build()
    _SECONDS['build'] = perf_counter() - started
    started = perf_counter()
    _ENGINES['j1'] = J1Engine()
    _ENGINES['route'] = R.RustJumbleEngine(binary=binary, name='command-route', step_result=H.StepResult)
    _SECONDS['load'] = perf_counter() - started


def tearDownModule():
    engine = _ENGINES.pop('route', None)
    if engine is not None:
        engine.close()
    if _SECONDS:
        print('seconds: ' + ', '.join(f'{k} {v:.1f}' for k, v in _SECONDS.items()), file=sys.stderr)


def route():
    return _ENGINES['route']


def j1():
    return _ENGINES['j1']


def fixture(name):
    return json.loads((FIXTURE_DIR / f'wj-{name}.json').read_text(encoding='utf-8'))


def record_step(record):
    return {'kind': 'record', 'record': record}


def letter_step(grip, item):
    return {'kind': 'letter', 'grip': grip, 'item': item}


class TimedCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._started = perf_counter()

    @classmethod
    def tearDownClass(cls):
        _SECONDS[cls.__name__] = perf_counter() - cls._started


class Recorder:
    """The route behind compare, keeping the result of every step."""

    def __init__(self, engine):
        self.name = engine.name
        self._engine = engine
        self.results = []

    def start(self, menu):
        self.results = [self._engine.start(menu)]
        return self.results[0]

    def apply(self, step):
        result = self._engine.apply(step)
        self.results.append(result)
        return result


class FixtureComparisonTests(TimedCase):
    """J1 and the route step by step, then undo and redo through the command layer."""

    def compare_fixture(self, name):
        steps = H.load_steps(fixture(name))
        count = len(steps.steps) if FULL else RECORDS
        steps = replace(steps, steps=steps.steps[:count])
        engine = Recorder(route())
        report = H.compare(j1(), engine, steps)
        self.assertTrue(report.passed, report.as_dict())
        self.assertEqual(report.steps_compared, count + 1)
        # Every move's result, and the states the comparison found equal to J1's: the entry, then each move's.
        moves = [result for result in engine.results[1:] if result.status == 'admissible']
        self.assertTrue(moves, f'{name}: no record moved')
        states = [engine.results[0]] + moves
        for k in range(len(moves), 0, -1):
            undone = route().call('undo')
            self.assertEqual((undone['status'], undone['digest'], undone['lattice']),
                             ('undone', states[k - 1].digest, states[k - 1].lattice), f'{name}: the undo to {k - 1}')
            self.assertEqual((undone['moved'], undone['moved_set']), (moves[k - 1].moved, moves[k - 1].moved_set),
                             f'{name}: the undo to {k - 1} moves its move\'s pieces')
        with self.assertRaises(R.CandidateError) as caught:
            route().call('undo')
        self.assertEqual(caught.exception.kind, 'input')
        for k in range(1, len(moves) + 1):
            redone = route().call('redo')
            self.assertEqual((redone['status'], redone['digest'], redone['lattice']),
                             ('redone', states[k].digest, states[k].lattice), f'{name}: the redo to {k}')
            self.assertEqual((redone['moved'], redone['moved_set']), (moves[k - 1].moved, moves[k - 1].moved_set),
                             f'{name}: the redo to {k} moves its move\'s pieces')
        with self.assertRaises(R.CandidateError) as caught:
            route().call('redo')
        self.assertEqual(caught.exception.kind, 'input')

    def test_S4(self):
        self.compare_fixture('S4')

    def test_I_a(self):
        self.compare_fixture('I_a')

    def test_I_b(self):
        self.compare_fixture('I_b')


class RefusalTests(TimedCase):
    def test_refusals_compare_and_change_nothing(self):
        """After 30 I_a records: a non-identity item at a grip J1's survey finds blocked, a record of the S4 menu (a
        quarter turn, which I_a does not hold) at a grip J1 finds admissible, and a malformed record (every entry
        doubled, so no rotation) are refused alike by J1 and the route, and nothing changes (compare checks it for
        both); the next record still moves."""
        doc = fixture('I_a')
        records = doc['journal']['records']
        j1().start(doc['menu'])
        for record in records[:REFUSED_AT]:
            j1().apply(record_step(record))
        grips = j1().survey(range(600))
        blocked = sorted(grip for grip, letter in grips.items() if letter == 'b')
        admissible = {grip for grip, letter in grips.items() if letter == 'a'}
        self.assertTrue(blocked and admissible)
        quarter = next(record for record in fixture('S4')['journal']['records']
                       if record['angle_deg'] == 90.0 and record['grip'] in admissible)
        record = records[REFUSED_AT]
        malformed = dict(record, matrix=[[[2 * a, 2 * b, d] for a, b, d in row] for row in record['matrix']])
        refusals = (letter_step(blocked[0], 1), record_step(quarter), record_step(malformed))
        steps = H.load_steps(doc)
        steps = replace(steps, steps=steps.steps[:REFUSED_AT] + refusals + (record_step(record),))
        engine = Recorder(route())
        report = H.compare(j1(), engine, steps)
        self.assertTrue(report.passed, report.as_dict())
        self.assertEqual([r.status for r in engine.results[REFUSED_AT + 1:]],
                         ['blocked', 'invalid', 'invalid', 'admissible'])


class InputErrorTests(TimedCase):
    def test_inputs_the_route_does_not_read_change_nothing(self):
        """Free mode, the identity item, a budget, a protected piece, an item outside the menu, an undo at the entry, a
        redo with none to redo and a request the route does not answer are input errors. None changes anything, so
        the first two records then reach J1's results."""
        doc = fixture('I_a')
        records = doc['journal']['records']
        with self.assertRaises(R.CandidateError) as caught:
            route().start(None)
        self.assertEqual(caught.exception.kind, 'input')
        self.assertEqual(route().start(doc['menu']), j1().start(doc['menu']))
        cases = [
            ('apply', {'step': letter_step(0, 0)}, RECIPE_SHAPE),
            ('apply', {'step': record_step(records[0]), 'budget': 5}, 'budget'),
            ('apply', {'step': record_step(records[0]), 'protect': [1]}, 'protect'),
            ('apply', {'step': letter_step(0, len(doc['menu']['base']))}, 'letter item'),
            ('undo', {}, 'undo'),
            ('redo', {}, 'redo'),
            ('survey', {'grips': [0]}, 'survey'),
        ]
        for op, fields, needle in cases:
            with self.subTest(op=op, needle=needle):
                with self.assertRaises(R.CandidateError) as caught:
                    route().call(op, **fields)
                self.assertEqual(caught.exception.kind, 'input')
                self.assertIn(needle, caught.exception.message)
        for record in records[:2]:
            self.assertEqual(route().apply(record_step(record)), j1().apply(record_step(record)))


if __name__ == '__main__':
    unittest.main(verbosity=2)
