"""LP2, M8: the Rust jumbling candidate's JI.6 and JI.7 observations against J1 (LP2 plan, section 11).

Each observation that M5 left to M8 is served by the adapter (`rust/c600-jumble/src/bin/adapter.rs`), called through
the driver (`differential/candidate/rust_jumble.py`) and compared with J1's reference on the states J1's own tests use,
undefined fields and refusals included:

- `rim_summary` against `references/observations.observe` on the recorded I_a@40 stop and the I_a prefixes 10, 20
  and 60 (J1's `tests/test_jumbling_observations.py`);
- `release_check` against `leads/x5_release.release_check` on census seed 7101 before steps 44, 70 and 97, each
  step's first letter, and a blocked letter and a bad cap before step 97 (J1's `tests/test_jumbling_leads_repair.py`);
- `covers` against `leads/covers.covers_query` on census seed 7101 before step 17 at the cap of the step's first
  letter, and at a blocked and a closed meeting cap, and before steps 30 and 33 at the repair's cap, where J1's
  recorded flags say that c covers d and that d covers c (J1's `tests/test_jumbling_leads_covers.py`);
- `repair_list` on census seed 7101 before step 97 at cap 458 (J1's `tests/test_jumbling_leads_repair.py`): the
  completed list against J1's recorded density list, every outcome and row through J1's own `compare_recorded`; and
  against `leads/repair_list.repair_list` run live, a 50-pair cut-off, a cancellation at the eleventh check, a zero
  pair budget and the errors at a blocked, a closed and a solved cap.

CT-KN2-01 (part: the J1 harness through the kernel API for KN2.4's `rim_summary`, `release_check`, `covers` and
`repair_list`; not run: `nu` and `correction_set_check`, and the 600-cell-Full oracle O01 to O07).
CT-KN2-04 (part: the release check of a blocked letter leaves the input digest unchanged at J1's state; not run here:
the off-menu and uncertain letters, which `rust/c600-jumble/tests/observations.rs` runs).
CT-BND-29 (part: the built-in repair list that a pick takes its entry from, equal to J1's at census seed 7101; not run
here: BND.84's producer check of the pick and its record, hand-entered words, imported 0.4 macros and retyping, which
`rust/c600-run/tests/conformance_bnd.rs` covers in part).

The Rust unit tests of the refusals are in `rust/c600-jumble/tests/observations.rs`. The adapter is built with
`cargo build --release --locked --offline` in rust/. A missing Rust toolchain or NumPy fails the check: a test that did
not run is no pass (PHS.63). A pass is source and fixture evidence, not Windows/DirectX, input, long-session or
performance evidence; the printed seconds are no performance claim.
"""

import sys

sys.dont_write_bytecode = True

import copy
import dataclasses
from functools import lru_cache
import json
from pathlib import Path
from time import perf_counter
import unittest

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / 'research' / 'jumbling'
FIXTURE_DIR = JUMBLING / 'fixtures'
NAMED_OBJECT_INVALID = 'KN2:input.named_object_invalid'
KERNEL_COST = 'KN7:budget.kernel_cost'
RIM_FIELDS = ('N_c', 'In_c', 'Out_c', 'A_c', 'F_c', 'at_floor')
CENSUS_SEED = 7101
CENSUS_STEPS = (17, 30, 33, 44, 70, 97)

_SECONDS = {}
_ENGINES = {}


def setUpModule():
    global np, census, R, obs_refs, O, X5, CV, RL, X1, sim, rim, X4
    try:
        import numpy as np
    except ImportError as error:
        raise RuntimeError('NumPy is required: the M8 comparison did not run (PHS.63)') from error
    sys.path.insert(0, str(JUMBLING))
    # The census module first: it moves the research centroid cache out of the repository before any theory import.
    from differential.leads import census
    from differential.candidate import rust_jumble as R
    from differential.references import obs_refs, observations as O
    from differential.leads import x5_release as X5
    from differential.leads import covers as CV
    from differential.leads import repair_list as RL, x1_walk as X1
    from differential.leads import x4_covers as X4
    import sim
    import rim
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


def _clone(S):
    """A copy of J1's restorer that J1's own repair-list tests make (`tests/test_jumbling_leads_repair.py`); the
    context, the immutable matrices and the pose entries are shared."""
    st = copy.copy(S.st)
    st.pose_id = S.st.pose_id.copy()
    st._poses = list(S.st._poses)
    st._lookup = dict(S.st._lookup)
    st.journal = list(S.st.journal)
    st._anchor_cache = {}
    result = census.seams.SeamRestorer(st, S.M)
    result.moves = list(S.moves)
    return result


@lru_cache(maxsize=1)
def _census_states():
    """{n: (word, S)}: J1's restorer before steps 17, 30, 33, 44, 70 and 97 of census seed 7101, in one walk, with each
    step's recorded word: the states of J1's covers, recorded-flags, release-check and repair-list tests."""
    started = perf_counter()
    states = {}
    for step in census.walk(CENSUS_SEED, last=max(CENSUS_STEPS)):
        if step.n in CENSUS_STEPS:
            states[step.n] = ([tuple(letter) for letter in step.word], _clone(step.S))
    _SECONDS['j1 census walk'] = perf_counter() - started
    return states


@lru_cache(maxsize=1)
def _candidate_census():
    """The candidate replays the census record's scramble history and restorer moves once and keeps the state before
    each of the steps (`keep`); a test selects one with `use`."""
    started = perf_counter()
    _, record = census.load(CENSUS_SEED)
    bounds = [0]
    for step in record['steps']:
        bounds.append(bounds[-1] + step['twists'])
    candidate().start(census.menu().record())
    letters = [tuple(m) for m in record['history']]
    done = 0
    for n in sorted(CENSUS_STEPS):
        letters += [tuple(m) for m in record['moves'][done:bounds[n - 1]]]
        done = bounds[n - 1]
        for grip, item in letters:
            if candidate().apply(letter_step(grip, item)).status != 'admissible':
                raise AssertionError(f'census letter ({grip}, {item}) is refused')
        letters = []
        candidate().keep(f'census-{n}')
    _SECONDS['candidate census replay'] = perf_counter() - started
    return True


def at_census(n):
    """J1's step word and restorer before step n of census seed 7101, with the candidate at the same state."""
    word, S = _census_states()[n]
    _candidate_census()
    if candidate().use(f'census-{n}').digest != S.st.digest():
        raise AssertionError(f'the candidate and J1 differ before census step {n}')
    return word, S


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


class ReleaseCheckTests(TimedCase):
    def test_census_releases_match_j1(self):
        """J1's three release checks (`tests/test_jumbling_leads_repair.py`: census seed 7101 before steps 44, 70 and
        97, each step's first letter): N_c before and after, the floor, delta N_c and the height counts equal
        `x5_release.release_check`, and the start digest is J1's input digest. Every field the one-letter word check
        has is the word check's (JI.11). A twist at an admissible cap changes N only on its cut, so the word check's
        total delta N is J1's delta N_c too."""
        for n in (44, 70, 97):
            with self.subTest(step=n):
                word, S = at_census(n)
                c, x = word[0]
                before = S.st.digest()
                reference = timed('j1 release_check', X5.release_check, S, c, x)
                answer = timed('candidate release_check', candidate().release_check, [c, x])
                self.assertEqual(answer['status'], 'ok')
                self.assertEqual(answer['start_digest'], reference['input_digest'])
                self.assertEqual((answer['admissible'], answer['letter_cap'], answer['rim_undefined']), (True, c, None))
                self.assertEqual(answer['rim_before']['N_c'], reference['N_c_now'])
                self.assertEqual(answer['rim_after']['N_c'], reference['N_c_after'])
                self.assertEqual(answer['floor'], reference['floor'])
                self.assertEqual(answer['rim_after']['F_c'], reference['floor'])
                self.assertEqual(answer['delta_N_c'], reference['delta_N'])
                self.assertEqual(answer['delta_N'], reference['delta_N'])
                for field in ('heights_before', 'heights_after', 'delta_n_h'):
                    self.assertEqual(answer[field], reference[field], field)
                word_check = candidate().word_check([[c, x]])
                for key, value in word_check.items():
                    self.assertEqual(answer[key], value, key)
                self.assertEqual(candidate().record()['digest'], before)
                self.assertEqual(S.st.digest(), before)

    def test_a_blocked_letter_and_a_bad_cap_are_refused_by_both(self):
        """Before census step 97, cap 6 is blocked (J1's repair-list tests): J1's release check raises, and the
        candidate's answer is the refused one-letter word check with every rim field undefined and the start digest
        unchanged (CT-KN2-04 for the release check). A letter's grip outside 0..599 is refused by both (J1 with a
        ValueError, the adapter as input), and the candidate's optional delta-u cap outside 0..599 with
        KN2:input.named_object_invalid."""
        word, S = at_census(97)
        before = S.st.digest()
        with self.assertRaisesRegex(ValueError, 'cap 6 is blocked or uncertain'):
            X5.release_check(S, 6, 1)
        answer = candidate().release_check([6, 1])
        self.assertEqual((answer['admissible'], answer['failing_status'], answer['rim_undefined']),
                         (False, 'blocked', 'cap_not_admissible'))
        for field in ('rim_before', 'rim_after', 'floor', 'delta_N_c', 'end_digest', 'delta_N', 'heights_after'):
            self.assertIsNone(answer[field], field)
        self.assertEqual(answer['start_digest'], before)
        with self.assertRaisesRegex(ValueError, 'cap must be an integer in 0..599'):
            X5.release_check(S, 600, 1)
        with self.assertRaises(R.CandidateError) as raised:
            candidate().release_check([600, 1])
        self.assertEqual(raised.exception.kind, 'input')
        self.assert_refused(candidate().release_check(word[0], cap=600), 'cap', [600])
        self.assertEqual(candidate().record()['digest'], before)
        self.assertEqual(S.st.digest(), before)


class CoversTests(TimedCase):
    def assert_same(self, answer, reference, label):
        """The candidate's cover table equals J1's `as_dict` in every field but J1's wall-clock seconds; `by`, which
        only a cut-off job names, is null."""
        document = json.loads(json.dumps(reference.as_dict()))
        self.assertEqual(set(answer) - {'by'}, set(document) - {'seconds'}, label)
        for key in sorted(set(document) - {'seconds'}):
            self.assertEqual(answer[key], document[key], (label, key))
        self.assertIsNone(answer['by'], label)

    def test_census_step_17_matches_j1(self):
        """J1's covers test (`tests/test_jumbling_leads_covers.py`): census seed 7101 before step 17, at the cap of the
        step's first letter. The completed table equals `covers_query`: the 80 meeting caps in cap-ID order, the
        parameters (threshold 10, the pinned menu identity and order digest), and both directions of every answer,
        with the face counts, the face pieces that straddle a blocked meeting cap and each not-applicable reason. A
        blocked meeting cap and a closed one, each named in turn, are errors with no answers for both. Nothing
        changes the state."""
        word, S = at_census(17)
        cap = word[0][0]
        before = S.st.digest()
        reference = timed('j1 covers', CV.covers_query, S, cap)
        self.assertEqual(reference.status, 'completed', reference.message)
        answer = timed('candidate covers', candidate().covers, cap)
        self.assert_same(answer, reference, f'cap {cap}')
        self.assertEqual(answer['parameters']['menu_order_digest'], census.I_A_ORDER_DIGEST)
        reasons = {row['cap']: row['c_covers_d'].get('reason') for row in answer['answers']}
        blocked = [d for d, reason in reasons.items() if reason == 'blocked']
        closed = [d for d, reason in reasons.items() if reason == 'closed']
        self.assertTrue(blocked, 'the state must have a blocked meeting cap')
        self.assertTrue(any(row['d_covers_c']['straddlers'] for row in answer['answers']))
        for d in blocked[:1] + closed[:1]:
            with self.subTest(named=d):
                error = candidate().covers(d)
                self.assertEqual((error['status'], error['answers']), ('error', []))
                self.assert_same(error, timed('j1 covers', CV.covers_query, S, d), f'cap {d}')
        self.assertEqual(candidate().record()['digest'], before)
        self.assertEqual(S.st.digest(), before)

    def test_census_steps_30_and_33_match_j1_at_the_recorded_flags(self):
        """J1's recorded-flags test (`tests/test_jumbling_leads_covers.py`): census seed 7101 before steps 30 and 33,
        the first repairs (c, x) (d, y) (c, z) whose recorded flags say that c covers d and that d covers c. At the
        repair's cap c the completed table equals `covers_query`, and the row of d holds in both directions the exact
        answer of J1's `x4_covers.compare_recorded_flags`, which also checks that J1's float test reproduces both
        recorded flags; so a true answer in each direction is compared. Nothing changes the state."""
        flags = X4.recorded_flags()
        for n, flag in ((30, 'c_covers_d'), (33, 'd_covers_c')):
            with self.subTest(step=n):
                word, S = at_census(n)
                (c, _), (d, _), _ = word
                before = S.st.digest()
                reference = timed('j1 covers', CV.covers_query, S, c)
                self.assertEqual(reference.status, 'completed', reference.message)
                answer = timed('candidate covers', candidate().covers, c)
                self.assert_same(answer, reference, f'step {n}, cap {c}')
                row = next(row for row in answer['answers'] if row['cap'] == d)
                step = census.Step(n, 'conj', word, None, S)
                pairs = timed('j1 recorded flags', X4.compare_recorded_flags, step, flags[n])
                self.assertEqual(sorted(pair['recorded_flag'] for pair in pairs), ['c_covers_d', 'd_covers_c'])
                for pair in pairs:
                    name = pair['recorded_flag']
                    self.assertEqual(row[name], json.loads(json.dumps(pair['exact'])), f'step {n}: {name}')
                self.assertIs(row[flag]['covers'], True, f'step {n}: {flag}')
                self.assertEqual(candidate().record()['digest'], before)
                self.assertEqual(S.st.digest(), before)

    def test_a_bad_cap_and_an_exhausted_budget(self):
        """Through the adapter: a cap outside 0..599 is refused with KN2:input.named_object_invalid (J1 raises), a
        request without a cap is an input error, and a budget of zero exact evaluations cuts the job off with
        KN7:budget.kernel_cost and no answers, so it makes no claim about any meeting cap (KN7.4)."""
        word, S = at_census(17)
        before = S.st.digest()
        for cap in (600, -1):
            with self.subTest(cap=cap):
                with self.assertRaisesRegex(ValueError, 'cap must be an integer in 0..599'):
                    CV.covers_query(S, cap)
                self.assert_refused(candidate().covers(cap), 'cap', [cap])
        with self.assertRaises(R.CandidateError) as raised:
            candidate().call('covers')
        self.assertEqual(raised.exception.kind, 'input')
        cut = candidate().covers(word[0][0], eval_budget=0)
        self.assertEqual((cut['status'], cut['by'], cut['answers']), ('cut_off', 'KN7:budget.kernel_cost', []))
        self.assertEqual(cut['domain'], list(CV.meeting_caps(word[0][0])))
        self.assertEqual(candidate().record()['digest'], before)


def as_repair_result(answer):
    """The candidate's repair list as J1's typed result, so that J1's own `compare_recorded` reads it. The candidate
    keeps no wall-clock timings, so they are empty."""
    p, coverage = answer['parameters'], answer['coverage']
    outcomes = tuple((x, d, delta, tuple(info) if isinstance(info, list) else info)
                     for x, d, delta, info in answer['outcomes'])
    return RL.RepairListResult(
        answer['status'], answer['input_digest'],
        RL.RepairParameters(p['cap'], p['menu_identity'], p['order_digest'], p['tie_rule'], p['contract_revision']),
        RL.RepairCoverage(coverage['pairs_tried'], tuple(coverage['items_done']), tuple(coverage['meeting_caps'])),
        None if answer['rows'] is None else tuple(tuple(row) for row in answer['rows']),
        outcomes, RL.RepairTimings({}, 0.0), answer['message'])


class RepairListTests(TimedCase):
    def assert_same(self, answer, reference, label):
        """The candidate's repair list equals J1's result in every field but J1's timings: status, input digest,
        parameters, coverage, rows (null unless completed), every outcome with its undefined delta and reason, and the
        message. `by` names KN7:budget.kernel_cost for a cut-off job and is null otherwise."""
        document = json.loads(json.dumps({k: v for k, v in dataclasses.asdict(reference).items() if k != 'timings'}))
        self.assertEqual(set(answer) - {'by'}, set(document), label)
        for key in sorted(document):
            self.assertEqual(answer[key], document[key], (label, key))
        self.assertEqual(answer['by'], KERNEL_COST if answer['status'] == 'cut_off' else None, label)

    def test_census_step_97_cap_458_matches_the_recorded_list(self):
        """J1's recorded repair list (`tests/test_jumbling_leads_repair.py`): census seed 7101 before step 97 at cap
        458. The completed list passes J1's own `compare_recorded` against the 472 recorded outcomes, so every pair
        (x, d) in pinned x order then cap d order has J1's delta, or J1's reason when it has none, and the one listed
        row is J1's (6, 513, 7, 0, -2). The coverage, the parameters and the input digest are J1's. Nothing changes the
        state."""
        word, S = at_census(97)
        before = S.st.digest()
        answer = timed('candidate repair_list', candidate().repair_list, 458)
        self.assertEqual((answer['status'], answer['message'], answer['by']), ('completed', None, None))
        density = X1.density_records()[(97, 458)]
        self.assertEqual(len(density), 472)
        RL.compare_recorded(as_repair_result(answer), density, step=97)
        self.assertEqual(answer['rows'], [[6, 513, 7, 0, -2]])
        self.assertEqual(answer['coverage'], {'pairs_tried': 472, 'items_done': list(range(1, 60)),
                                              'meeting_caps': [349, 399, 416, 469, 484, 513, 523, 571]})
        self.assertEqual(answer['input_digest'], before)
        self.assertEqual(answer['parameters'], {
            'cap': 458, 'menu_identity': RL.I_A_IDENTITY, 'order_digest': RL.I_A_ORDER_DIGEST,
            'tie_rule': RL.TIE_RULE, 'contract_revision': RL.contract_revision()})
        self.assertEqual(candidate().record()['digest'], before)

    def test_a_pair_budget_and_a_cancellation_match_j1(self):
        """J1's cut-off and cancellation (`tests/test_jumbling_leads_repair.py`) run live on both: a budget of 50
        pairs stops after items 1 to 6, a cancellation observed at the eleventh check after 10 pairs, and a budget of
        zero pairs before any trial twist. Each is equal to J1's, outcomes as coverage and no rows, and changes
        nothing."""
        word, S = at_census(97)
        before = S.st.digest()
        reference = timed('j1 repair_list', RL.repair_list, S, 458, max_pairs=50)
        answer = timed('candidate repair_list', candidate().repair_list, 458, max_pairs=50)
        self.assert_same(answer, reference, 'max_pairs 50')
        self.assertEqual((answer['status'], answer['rows'], answer['coverage']['pairs_tried']), ('cut_off', None, 50))
        self.assertEqual(answer['coverage']['items_done'], list(range(1, 7)))
        calls = 0

        def cancel():
            nonlocal calls
            calls += 1
            return calls > 10

        reference = timed('j1 repair_list', RL.repair_list, S, 458, cancel=cancel)
        answer = timed('candidate repair_list', candidate().repair_list, 458, cancel_after=10)
        self.assert_same(answer, reference, 'cancelled at the eleventh check')
        self.assertEqual((answer['status'], answer['rows'], answer['coverage']['pairs_tried']), ('cancelled', None, 10))
        reference = RL.repair_list(S, 458, max_pairs=0)
        answer = candidate().repair_list(458, max_pairs=0)
        self.assert_same(answer, reference, 'max_pairs 0')
        self.assertEqual((answer['status'], answer['outcomes'], answer['coverage']['items_done']), ('cut_off', [], []))
        self.assertEqual(candidate().record()['digest'], before)
        self.assertEqual(S.st.digest(), before)

    def test_blocked_closed_and_solved_caps_are_errors_in_both(self):
        """Before census step 97, cap 6 is blocked and cap 0 is closed, and at the solved start cap 458 is closed
        (J1's repair-list tests): each is an error with J1's message, no outcomes, no meeting caps and no rows, and
        changes nothing."""
        word, S = at_census(97)
        before = S.st.digest()
        for cap in (6, 0):
            with self.subTest(cap=cap):
                answer = candidate().repair_list(cap)
                self.assert_same(answer, RL.repair_list(S, cap), f'cap {cap}')
                self.assertEqual((answer['status'], answer['coverage']['pairs_tried']), ('error', 0))
                self.assertTrue(answer['message'])
        self.assertEqual(candidate().record()['digest'], before)
        self.assertEqual(S.st.digest(), before)
        solved = census.seams.SeamRestorer(sim.State(rim.ctx, menu=census.menu()), census.menu())
        candidate().start(census.menu().record())
        self.assertEqual(candidate().record()['digest'], solved.st.digest())
        answer = candidate().repair_list(458)
        self.assert_same(answer, RL.repair_list(solved, 458), 'solved')
        self.assertEqual(answer['message'], 'cap 458 is closed (N_c = 0), not misaligned')

    def test_a_bad_cap_a_missing_cap_and_an_exhausted_budget(self):
        """Through the adapter: a cap outside 0..599 is refused with KN2:input.named_object_invalid (J1 answers an
        error with no outcomes), a request without a cap and a negative pair budget or cancellation point are input
        errors (J1 raises for a negative pair budget), and a budget of zero exact evaluations cuts the job off with
        KN7:budget.kernel_cost before any meeting cap is read, so it makes no claim about any pair (KN7.4)."""
        word, S = at_census(97)
        before = S.st.digest()
        for cap in (600, -1):
            with self.subTest(cap=cap):
                reference = RL.repair_list(S, cap)
                self.assertEqual((reference.status, reference.outcomes, reference.message),
                                 ('error', (), 'cap must be an integer in 0..599'))
                self.assert_refused(candidate().repair_list(cap), 'cap', [cap])
        with self.assertRaises(R.CandidateError) as raised:
            candidate().call('repair_list')
        self.assertEqual(raised.exception.kind, 'input')
        with self.assertRaisesRegex(ValueError, 'max_pairs must be a non-negative integer or None'):
            RL.repair_list(S, 458, max_pairs=-1)
        for field in ('max_pairs', 'cancel_after'):
            with self.subTest(field=field):
                with self.assertRaises(R.CandidateError) as raised:
                    candidate().call('repair_list', cap=458, **{field: -1})
                self.assertEqual(raised.exception.kind, 'input')
        cut = candidate().repair_list(458, eval_budget=0)
        self.assertEqual((cut['status'], cut['by'], cut['rows'], cut['outcomes']), ('cut_off', KERNEL_COST, None, []))
        self.assertEqual(cut['coverage'], {'pairs_tried': 0, 'items_done': [], 'meeting_caps': []})
        self.assertTrue(cut['message'])
        self.assertEqual(candidate().record()['digest'], before)


if __name__ == '__main__':
    unittest.main()
