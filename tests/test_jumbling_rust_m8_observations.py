"""LP2, M8: the Rust jumbling candidate's JI.6 and JI.7 observations against J1 (LP2 plan, section 11).

Each observation that M5 left to M8 is served by the adapter (`rust/c600-jumble/src/bin/adapter.rs`), called through
the driver (`differential/candidate/rust_jumble.py`) and compared with J1's reference on the states J1's own tests use,
undefined fields and refusals included:

- `rim_summary` against `references/observations.observe` on the recorded I_a@40 stop and the I_a prefixes 10, 20
  and 60 (J1's `tests/test_jumbling_observations.py`). CT-KN2-01 (part: the J1 harness through the kernel API for
  KN2.4's `rim_summary`; not run: `release_check`, `covers`, `repair_list`, `nu` and `correction_set_check`).

The Rust unit tests of the refusals are in `rust/c600-jumble/tests/observations.rs`. The adapter is built with
`cargo build --release --locked --offline` in rust/. A missing Rust toolchain or NumPy fails the check: a test that did
not run is no pass (PHS.63). A pass is source and fixture evidence, not Windows/DirectX, input, long-session or
performance evidence; the printed seconds are no performance claim.
"""

import sys

sys.dont_write_bytecode = True

from functools import lru_cache
import json
from pathlib import Path
from time import perf_counter
import unittest

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / 'research' / 'jumbling'
FIXTURE_DIR = JUMBLING / 'fixtures'
NAMED_OBJECT_INVALID = 'KN2:input.named_object_invalid'
RIM_FIELDS = ('N_c', 'In_c', 'Out_c', 'A_c', 'F_c', 'at_floor')

_SECONDS = {}
_ENGINES = {}


def setUpModule():
    global np, census, R, obs_refs, O
    try:
        import numpy as np
    except ImportError as error:
        raise RuntimeError('NumPy is required: the M8 comparison did not run (PHS.63)') from error
    sys.path.insert(0, str(JUMBLING))
    # The census module first: it moves the research centroid cache out of the repository before any theory import.
    from differential.leads import census
    from differential.candidate import rust_jumble as R
    from differential.references import obs_refs, observations as O
    if Path(census.seams.CACHE).resolve().is_relative_to(ROOT):
        raise AssertionError('the centroid cache is inside the repository')
    started = perf_counter()
    R.build()
    _SECONDS['build'] = perf_counter() - started
    _ENGINES['candidate'] = R.RustJumbleEngine()


def tearDownModule():
    engine = _ENGINES.pop('candidate', None)
    if engine is not None:
        engine.close()
    if _SECONDS:
        print('seconds: ' + ', '.join(f'{k} {v:.1f}' for k, v in _SECONDS.items()), file=sys.stderr)


def candidate():
    return _ENGINES['candidate']


def fixture(name):
    return json.loads((FIXTURE_DIR / f'wj-{name}.json').read_text(encoding='utf-8'))


def record_step(record):
    return {'kind': 'record', 'record': record}


def letter_step(grip, item):
    return {'kind': 'letter', 'grip': grip, 'item': item}


def timed(name, call, *args, **kwargs):
    started = perf_counter()
    result = call(*args, **kwargs)
    _SECONDS[name] = _SECONDS.get(name, 0.0) + perf_counter() - started
    return result


@lru_cache(maxsize=1)
def _stop40():
    """J1's recorded I_a@40 stop (I_a@40 and the 62 recorded restorer letters) with J1's observation of it."""
    state = timed('j1 stop40 replay', obs_refs.stop40_state)
    return state, timed('j1 observe', O.observe, state)


def candidate_at_stop40():
    reference = obs_refs.stop40_reference()
    doc = fixture('I_a')
    candidate().start(doc['menu'])
    for record in doc['journal']['records'][:40]:
        candidate().apply(record_step(record))
    for grip, item in reference['moves']:
        if candidate().apply(letter_step(grip, item)).status != 'admissible':
            raise AssertionError(f'the recorded restorer letter ({grip}, {item}) is refused')
    return reference


def candidate_at_prefix(t):
    doc = fixture('I_a')
    candidate().start(doc['menu'])
    for record in doc['journal']['records'][:t]:
        candidate().apply(record_step(record))


class TimedCase(unittest.TestCase):
    def assert_refused(self, answer, argument, supplied):
        self.assertEqual(answer['status'], 'refused')
        self.assertEqual(answer['code'], NAMED_OBJECT_INVALID)
        self.assertEqual((answer['argument'], answer['supplied']), (argument, supplied))
        self.assertTrue(answer['condition'])


class RimSummaryTests(TimedCase):
    def assert_rims(self, answer, observation, label):
        """Every cap's status, blocked flag and rim fields equal J1's, undefined (null) where J1's are None."""
        self.assertEqual(answer['status'], 'ok', label)
        self.assertTrue(answer['complete'], label)
        self.assertEqual(answer['digest'], observation.digest, label)
        rows = {row['cap']: row for row in answer['rows']}
        self.assertEqual(sorted(rows), list(range(600)), label)
        self.assertEqual(set(observation.rim), set(range(600)), label)
        for c, reference in observation.rim.items():
            row = rows[c]
            self.assertEqual(row['status'], reference.status, (label, c))
            self.assertEqual(row['blocked'], reference.blocked, (label, c))
            for field in RIM_FIELDS:
                self.assertEqual(row[field], getattr(reference, field), (label, c, field))
            self.assertEqual(row['undefined'], None if reference.status == 'admissible' else 'cap_not_admissible',
                             (label, c))
        self.assertEqual((answer['grip_n'], answer['grip_n_undefined']), (observation.grip_n, None), label)

    def test_stop40_matches_j1(self):
        """The recorded I_a@40 stop: every cap's row and grip N equal J1's; 15 caps are misaligned, eight of them at
        their floor, and grip N is the recorded 1,057."""
        state, observation = _stop40()
        candidate_at_stop40()
        self.assertEqual(candidate().record()['digest'], state.digest())
        answer = timed('candidate rim_summary', candidate().rim_summary)
        self.assert_rims(answer, observation, 'stop40')
        self.assertEqual(answer['grip_n'], 1057)
        misaligned = [row['cap'] for row in answer['rows'] if (row['N_c'] or 0) > 0]
        self.assertEqual(len(misaligned), 15)
        floor = [row['cap'] for row in answer['rows'] if (row['N_c'] or 0) > 0 and row['at_floor']]
        self.assertEqual(floor, [17, 34, 50, 162, 187, 237, 315, 390])

    def test_I_a_prefixes_match_j1(self):
        for t in (10, 20, 60):
            with self.subTest(prefix=t):
                state = timed('j1 prefix replay', obs_refs.fixture_state, 'I_a', t)
                observation = timed('j1 observe', O.observe, state)
                candidate_at_prefix(t)
                self.assertEqual(candidate().record()['digest'], state.digest())
                self.assert_rims(timed('candidate rim_summary', candidate().rim_summary), observation, f'I_a@{t}')

    def test_named_caps_refusals_and_an_exhausted_budget(self):
        """Through the adapter: named caps answer only for themselves, with no grip N; a cap outside 0..599 or an
        empty selection is refused with KN2:input.named_object_invalid and changes nothing; a budget of zero exact
        evaluations leaves every cap uncertain with no rim and no blocked flag."""
        candidate_at_prefix(10)
        before = candidate().record()['digest']
        named = candidate().rim_summary([390, 17, 17])
        self.assertEqual([row['cap'] for row in named['rows']], [17, 390])
        self.assertEqual((named['grip_n'], named['grip_n_undefined']), (None, 'caps_selected'))
        for caps in ([600], [-1], [0, 600], []):
            with self.subTest(caps=caps):
                self.assert_refused(candidate().rim_summary(caps), 'caps', caps)
        uncertain = candidate().rim_summary(eval_budget=0)
        self.assertEqual((uncertain['complete'], uncertain['uncertain_caps']), (False, 600))
        self.assertEqual((uncertain['grip_n'], uncertain['grip_n_undefined']), (None, 'uncertain_caps'))
        for row in uncertain['rows']:
            self.assertEqual((row['status'], row['blocked'], row['undefined']),
                             ('uncertain', None, 'uncertain_under_budget'))
            self.assertTrue(all(row[field] is None for field in RIM_FIELDS))
        self.assertEqual(candidate().record()['digest'], before)


if __name__ == '__main__':
    unittest.main()
