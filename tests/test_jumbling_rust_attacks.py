"""LP2, M5: attack tests on the Rust jumbling candidate, from the Fable solver's leads of 16 November 2026.

Each test states a rule of the jumbling interfaces (`docs/progress/1.0/jumbling-interfaces.md`, JI) and runs it
against the candidate (`rust/c600-jumble` behind `differential/candidate/rust_jumble.py`). A lead is no finding until
a test confirms it. Where J1's reference differs from JI, the test pins JI; the J1 faults are recorded in
`J1ReferenceFaults` as expected failures, so a fixed reference shows up as an unexpected success, and they are reported,
never fixed, while the research is stopped. `J1ApiContracts` holds JI.52's public J1 API (`sim/api.py`) to the same
rule.

Conformance, with PHS.62's markers: CT-JI-03 (part: cancellation yields only covered prefixes, on a three-letter word
in chunks of two, on prefix and seam batches cancelled before their first chunk and on a seam batch cancelled after
one; the fixed core cell is listed in the identity domain; not run here: off-grip counts and JI.16's replay, which
`tests/test_jumbling_rust_candidate.py` runs, and holding caps, which M8 builds). CT-KN2-04 (part: a word check with a
blocked letter, a refused named unit and a refused cap leave the state record unchanged; not run here: the invalid and
uncertain letters, which `tests/test_jumbling_rust_candidate.py` runs). The other tests check JI clauses with no
conformance ID of their own: JI.9 and JI.45 (named units and caps), JI.3 and JI.45 (the dimension), JI.11 (a refused
prefix has no end-state assertion; exact poses, never pose-table indices), JI.10, JI.12 and JI.39 (the built rows
name only the supplied letters; a cancelled batch is no empty completed one), JI.5 (the identity letter), KN2.14 (the
state record reader), JI.6 (sparse height bins), JI.53 and LB3.1 (a free-mode pose), KN1.4 (contract stamps) and JI.52
(J1's public API: refusals, one classification, read-only pose entries and the fixture's prefix range).

Not run here, because the candidate does not build the call (M8, LP2 plan, section 11): `label_checkpoint`,
`retained_checkpoint` and `sticker_test` (lead L9), `repair_list` and `covers` (lead L15's ordering, rank and
missing-cap rows), `release_check` (lead L2's agreement) and snapshot handles (lead L16's `KN2:stale.revision_gone`).
Not applicable is no pass (PHS.63).

The adapter is built with `cargo build --release --locked --offline` in rust/. A pass is source and fixture evidence.
"""

import sys

sys.dont_write_bytecode = True

import base64
import hashlib
import json
from pathlib import Path
import struct
import types
import unittest
import unittest.mock

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / 'research' / 'jumbling'
FIXTURE_DIR = JUMBLING / 'fixtures'

NP = 177120
NAMED_OBJECT_INVALID = 'KN2:input.named_object_invalid'
IDENTITY_MISMATCH = 'KN1:input.identity_mismatch'
UNSUPPORTED_DIMENSION = 'KN1:input.unsupported_dimension'
# JI.16's round trip: item 45 is the inverse of item 12 in the pinned I_a menu.
JI16_WORD = [[0, 12], [0, 45]]
SEAM_CHUNK = 1 << 16
SEAM_BUDGET = 1 << 24
# Fields of a word check that read the end state (JI.11: none covers a refused prefix).
END_FIELDS = ('end_digest', 'unit_on_lattice', 'unit_landed', 'lattice_pieces_lifted_off', 'collateral_ids',
              'others_exact', 'others_landed', 'others_changed_ids', 'off_after', 'off_min', 'off_max',
              'off_pieces_landed', 'pose_changed', 'pose_changed_ids', 'delta_N', 'heights_after', 'delta_n_h',
              'u_after', 'delta_u', 'peak_left_height', 'excursion', 'tame')
# Keys no built row may carry (JI.39): no rank, target, preference or next cap.
FORBIDDEN_KEYS = {'rank', 'ranking', 'score', 'best', 'target', 'preferred', 'suggested', 'next_cap', 'next'}

_ENGINES = {}


def setUpModule():
    global R
    sys.path.insert(0, str(JUMBLING))
    from differential.candidate import rust_jumble as R
    R.build()
    _ENGINES['candidate'] = R.RustJumbleEngine()


def tearDownModule():
    for engine in _ENGINES.values():
        close = getattr(engine, 'close', None)
        if close is not None:
            close()
    _ENGINES.clear()


def candidate():
    return _ENGINES['candidate']


def fixture(name):
    return json.loads((FIXTURE_DIR / f'wj-{name}.json').read_text(encoding='utf-8'))


def letter_step(grip, item):
    return {'kind': 'letter', 'grip': grip, 'item': item}


def identity_item(menu):
    one, zero = [1, 0, 1], [0, 0, 1]
    identity = [[one if i == j else zero for j in range(4)] for i in range(4)]
    items = [i for i, entry in enumerate(menu['base']) if entry['matrix'] == identity]
    if len(items) != 1:
        raise AssertionError(f'the menu has {len(items)} identity items')
    return items[0]


def pose_ids(record, pieces):
    raw = base64.b64decode(record['pose_ids']['data'])
    return [struct.unpack_from('<I', raw, 4 * p)[0] for p in pieces]


def with_pose_ids(record, table, remap):
    """A copy of a state record with another pose table and every pose ID mapped through `remap`."""
    raw = base64.b64decode(record['pose_ids']['data'])
    ids = struct.unpack(f'<{NP}I', raw)
    raw = struct.pack(f'<{NP}I', *(remap.get(i, i) for i in ids))
    return raw_record(record, table, raw)


def raw_record(record, table, raw, count=NP):
    return {'pose_table': table, 'digest': record['digest'],
            'pose_ids': {'encoding': 'u32le-base64', 'count': count, 'sha256': hashlib.sha256(raw).hexdigest(),
                         'data': base64.b64encode(raw).decode('ascii')}}


def keys_anywhere(value):
    if isinstance(value, dict):
        for key, item in value.items():
            yield key
            yield from keys_anywhere(item)
    elif isinstance(value, list):
        for item in value:
            yield from keys_anywhere(item)


class Fixture:
    """The pinned I_a menu, the pieces item 12 moves at cap 0 (unit A) and at a cap c2 whose inside set is disjoint
    from cap 0's and whose letter stays admissible after (0, 12) and before it (unit B); the lowest grip that
    (0, 12) blocks; and a piece that no letter here moves."""

    _cached = None

    @classmethod
    def get(cls):
        if cls._cached is None:
            cls._cached = cls()
        return cls._cached

    def __init__(self):
        engine = candidate()
        self.menu = fixture('I_a')['menu']
        self.identity = identity_item(self.menu)
        self.solved = engine.start(self.menu).digest
        self.a = self.unit_of(0)
        a_set = set(self.a)
        self.c2 = None
        for c2 in range(599, 0, -1):
            b = self.unit_of(c2)
            if a_set.isdisjoint(b) and self.admissible_after(0, c2) and self.admissible_after(c2, 0):
                self.c2, self.b = c2, b
                break
        if self.c2 is None:
            raise AssertionError('no cap with an inside set disjoint from cap 0 and an admissible letter 12')
        moved = a_set | set(self.b)
        self.lattice = next(p for p in range(NP) if p not in moved)
        survey = self.at((0, 12)).survey(range(600))
        blocked = [grip for grip, letter in survey.items() if letter == 'b']
        if not blocked:
            raise AssertionError('(0, 12) blocks no grip')
        self.blocked = min(blocked)

    def unit_of(self, cap):
        engine = candidate()
        engine.start(self.menu)
        if engine.apply(letter_step(cap, 12)).status != 'admissible':
            return []
        units = engine.units()['units']
        if len(units) != 1:
            raise AssertionError(f'one letter at cap {cap} gave {len(units)} units')
        return units[0]['members']

    def admissible_after(self, first, second):
        engine = candidate()
        engine.start(self.menu)
        return (engine.apply(letter_step(first, 12)).status == 'admissible'
                and engine.apply(letter_step(second, 12)).status == 'admissible')

    def at(self, *letters):
        """Start at solved and apply the letters; every one must be admissible."""
        engine = candidate()
        engine.start(self.menu)
        for grip, item in letters:
            status = engine.apply(letter_step(grip, item)).status
            if status != 'admissible':
                raise AssertionError(f'letter ({grip}, {item}) is {status}')
        return engine


class NamedObjectTests(unittest.TestCase):
    """Leads L1, L5 and L16 (caps): JI.9 and JI.45."""

    def setUp(self):
        self.f = Fixture.get()

    def assert_invalid(self, answer, argument, supplied):
        self.assertEqual((answer['status'], answer['code'], answer['argument'], answer['supplied']),
                         ('refused', NAMED_OBJECT_INVALID, argument, supplied))
        self.assertTrue(answer['condition'])

    def test_word_check_without_a_unit_leaves_the_step_flag_undefined(self):
        """JI.11: without a named unit the step flag is undefined(unit_not_named); no unit is inferred from the moved
        set, and no unit-size or landed field is defined."""
        engine = self.f.at((0, 12))
        check = engine.word_check([[0, 45]])
        self.assertEqual((check['step_flag'], check['step_flag_undefined']), (None, 'unit_not_named'))
        self.assertEqual((check['unit'], check['unit_size'], check['unit_on_lattice'], check['unit_landed']),
                         (None, None, None, None))
        self.assertEqual((check['others'], check['others_landed'], check['end_digest']),
                         (len(self.f.a), len(self.f.a), self.f.solved))

    def test_word_check_refuses_an_invalid_unit(self):
        """JI.9 and JI.45: an empty unit, a lattice piece, a unit spanning two exact poses and a piece ID outside the
        model are refused with KN2:input.named_object_invalid, naming the argument, the supplied IDs and the failed
        condition; nothing changes."""
        engine = self.f.at((0, 12), (self.f.c2, 12))
        before = engine.record()
        lattice = self.f.lattice
        for unit in ([], [lattice], [self.f.a[0], lattice], [self.f.a[0], self.f.b[0]], [NP], [-1],
                     [self.f.a[0], NP]):
            with self.subTest(unit=unit):
                self.assert_invalid(engine.word_check([[0, 45]], unit=unit), 'unit', unit)
                self.assertEqual(engine.record(), before)

    def test_units_refuses_an_invalid_named_set(self):
        """JI.9: the named-set argument of units obeys the same conditions; the empty all-off table is a completed
        answer, not a refusal."""
        engine = self.f.at((0, 12), (self.f.c2, 12))
        for pieces in ([], [self.f.a[0], self.f.b[0]], [NP], [-5]):
            with self.subTest(pieces=pieces):
                self.assert_invalid(engine.units(pieces), 'pieces', pieces)
        candidate().start(self.f.menu)
        self.assertEqual(candidate().units(), {'status': 'ok', 'units': [], 'complete': True})

    def test_caps_outside_the_model_are_refused(self):
        """JI.45: a cap outside 0..599 in word_check, prefix_observations or seam_facts is refused with
        KN2:input.named_object_invalid, never an adapter error, an empty result or another cap."""
        engine = self.f.at((0, 12))
        before = engine.record()
        for cap in (600, -1):
            with self.subTest(cap=cap):
                self.assert_invalid(engine.word_check([[0, 45]], cap=cap), 'cap', [cap])
                self.assert_invalid(engine.prefix_observations([[0, 45]], chunk=1, budget=1, cap=cap), 'cap', [cap])
                self.assert_invalid(engine.seam_facts(cap, chunk=SEAM_CHUNK, budget=SEAM_BUDGET), 'scope', [cap])
                self.assertEqual(engine.record(), before)

    def test_units_order_and_named_subsets(self):
        """JI.9 (lead L5): units lists every exact-pose group in canonical piece-ID order (by least member), members
        ascending; a named subset is read as a set, so an unsorted list with repeats names the sorted subset."""
        engine = self.f.at((0, 12), (self.f.c2, 12))
        units = engine.units()
        self.assertTrue(units['complete'])
        members = [row['members'] for row in units['units']]
        self.assertEqual(sorted(map(tuple, members)), sorted([tuple(self.f.a), tuple(self.f.b)]))
        self.assertEqual([m[0] for m in members], sorted(m[0] for m in members))
        self.assertTrue(all(m == sorted(m) for m in members))
        subset = [self.f.a[2], self.f.a[0], self.f.a[2], self.f.a[1]]
        named = engine.units(subset)['units']
        self.assertEqual([row['members'] for row in named], [sorted(set(subset))])
        self.assertEqual(engine.word_check([[0, 45]], unit=subset)['unit'], sorted(set(subset)))


class RefusedPrefixTests(unittest.TestCase):
    """Lead L2 (and L14's identity letter): JI.11."""

    def setUp(self):
        self.f = Fixture.get()
        self.after_first = self.f.at((0, 12)).record()['digest']
        self.blocked = self.f.blocked
        self.word = [[0, 12], [self.blocked, self.f.identity]]

    def test_identity_letter_at_a_blocked_grip_is_blocked(self):
        """JI.5 and JI.10: the identity item is classified like any letter; at a blocked grip it is blocked with a
        straddle certificate, and nothing changes."""
        engine = self.f.at((0, 12))
        before = engine.record()
        outcome = engine.outcome(letter_step(self.blocked, self.f.identity))
        self.assertEqual(outcome['status'], 'blocked')
        self.assertTrue(outcome['certificate']['points_checked_against_all_constraints'])
        self.assertEqual(engine.record(), before)

    def test_blocked_prefix_has_no_end_assertion(self):
        """JI.11: a blocked letter ends the word check, which reports its index, the processed range and the start
        facts; no end-state assertion covers the word (end digest, landing partition, off counts after, delta-N,
        height counts after and their delta are undefined), the step flag of a named unit is false (LB3.8 checks
        every prefix's admissibility), and the start is unchanged."""
        engine = candidate()
        engine.start(self.f.menu)
        before = engine.record()
        for unit in (None, self.f.b[:3]):
            with self.subTest(unit=unit):
                if unit is not None:
                    # The B pieces are on the lattice at solved, so start from (c2, 12) for a named unit.
                    engine = self.f.at((self.f.c2, 12))
                    before = engine.record()
                check = engine.word_check(self.word, unit=unit, cap=0)
                self.assertEqual((check['status'], check['admissible'], check['failing_letter_index'],
                                  check['failing_status'], check['applied_letters'], check['processed']),
                                 ('ok', False, 1, 'blocked', 1, [0, 2]))
                self.assertEqual(check['end_state_undefined'], 'refused_prefix')
                for field in END_FIELDS:
                    self.assertIsNone(check[field], field)
                self.assertEqual(check['start_digest'], before['digest'])
                self.assertEqual(check['step_flag'], None if unit is None else False)
                self.assertIsNotNone(check['heights_before'])
                self.assertIsNotNone(check['u_before'])
                self.assertEqual(engine.record(), before)

    def test_prefix_observations_stop_at_the_blocked_row(self):
        """JI.10 and JI.14: the batch observes row 0, reports row 1 as blocked with its certificate and stops; it
        makes no final claim, and row 1 applies nothing."""
        engine = candidate()
        engine.start(self.f.menu)
        result = engine.prefix_observations(self.word, chunk=1, budget=len(self.word))
        rows = result['rows']
        self.assertEqual([(row['index'], row['status']) for row in rows], [(0, 'admissible'), (1, 'blocked')])
        self.assertIsNotNone(rows[1]['certificate'])
        self.assertEqual((rows[1]['before'], rows[1]['after'], rows[1]['moved']),
                         (self.after_first, self.after_first, 0))
        self.assertEqual((result['complete'], result['first_refused'], result['final']), (False, 1, None))


class ExactPoseTests(unittest.TestCase):
    """Leads L4 and L5: JI.11 compares exact keys, never pose-table indices across revisions."""

    def setUp(self):
        self.f = Fixture.get()

    def test_exact_pose_unchanged_survives_renumbering(self):
        """Every starting-off piece of the other cap keeps its exact pose in both orders, though in at least one order
        its pose ID changes (the canonical table renumbers); both orders end at one configuration; and an untouched
        protected piece passes Net protection."""
        renumbered = []
        ends = []
        for first, second, kept in ((0, self.f.c2, self.f.a), (self.f.c2, 0, self.f.b)):
            with self.subTest(first=first):
                engine = self.f.at((first, 12))
                start = engine.record()
                check = engine.word_check([[second, 12]])
                self.assertTrue(check['admissible'])
                self.assertEqual((check['others'], check['others_exact'], check['others_landed'],
                                  check['others_changed_ids'], check['pose_changed']), (len(kept), len(kept), 0, [], 0))
                protected = engine.outcome(letter_step(second, 12), protect=[kept[0], kept[-1]])
                self.assertEqual(protected['status'], 'admissible')
                end = engine.record()
                self.assertEqual(end['digest'], check['end_digest'])
                renumbered.append(pose_ids(start, kept[:1]) != pose_ids(end, kept[:1]))
                ends.append(end)
        self.assertTrue(any(renumbered), 'neither order renumbers the kept group: the test has no teeth')
        self.assertEqual(ends[0], ends[1])


class BatchTests(unittest.TestCase):
    """Leads L3 and L15 (the built rows): CT-JI-03's cancellation, JI.12 and JI.39."""

    def setUp(self):
        self.f = Fixture.get()

    def test_cancel_yields_only_covered_prefixes(self):
        """CT-JI-03 (part): a three-letter word in chunks of two, cancelled after the first chunk, covers [0,2) with
        rows 0 and 1 only and makes no end, tameness, excursion or peak claim, in the batch or in a row."""
        engine = candidate()
        engine.start(self.f.menu)
        word = [[0, 12], [0, 45], [0, 12]]
        result = engine.prefix_observations(word, chunk=2, budget=len(word), cancel_after=1)
        self.assertEqual((result['coverage'], result['complete'], result['cancelled'], result['final']),
                         ([0, 2], False, True, None))
        self.assertEqual([row['index'] for row in result['rows']], [0, 1])
        for row in result['rows']:
            self.assertEqual(set(row) & {'end_digest', 'tame', 'excursion', 'peak_left_height', 'final'}, set())

    def test_cancelled_seam_batch_has_no_total(self):
        """JI.8 and JI.14: seam_facts(all) cancelled after one chunk covers that chunk and has no total N."""
        engine = self.f.at((0, 12))
        complete = engine.seam_facts('all', chunk=SEAM_CHUNK, budget=SEAM_BUDGET)
        self.assertTrue(complete['complete'])
        chunk = max(1, complete['cells'] // 4)
        cancelled = engine.seam_facts('all', chunk=chunk, budget=SEAM_BUDGET, cancel_after=1)
        self.assertEqual((cancelled['complete'], cancelled['cancelled'], cancelled['totals'], cancelled['coverage']),
                         (False, True, None, [0, chunk]))

    def test_cancelled_list_is_not_an_empty_completed_list(self):
        """JI.12 and JI.14: a batch cancelled before its first chunk has no rows, as an empty completed answer has
        none, but it is marked incomplete and cancelled, with empty coverage and no claim."""
        engine = candidate()
        engine.start(self.f.menu)
        prefix = engine.prefix_observations(JI16_WORD, chunk=1, budget=2, cancel_after=0)
        self.assertEqual((prefix['rows'], prefix['complete'], prefix['cancelled'], prefix['coverage'],
                          prefix['final']), ([], False, True, [0, 0], None))
        seams = engine.seam_facts('all', chunk=SEAM_CHUNK, budget=SEAM_BUDGET, cancel_after=0)
        self.assertEqual((seams['complete'], seams['cancelled'], seams['coverage'], seams['totals']),
                         (False, True, [0, 0], None))
        self.assertEqual(engine.units(), {'status': 'ok', 'units': [], 'complete': True})

    def test_rows_name_only_the_supplied_letters(self):
        """JI.10 and JI.39: a complete batch has exactly one row per supplied letter, in order, naming that letter;
        no built row ranks, targets or proposes anything."""
        engine = candidate()
        engine.start(self.f.menu)
        word = [[0, 12], [self.f.c2, 12], [0, 45], [self.f.c2, 45]]
        result = engine.prefix_observations(word, chunk=3, budget=len(word), fields=['N', 'heights', 'fd', 'sets'])
        self.assertTrue(result['complete'])
        self.assertEqual([(row['index'], row['letter']) for row in result['rows']],
                         [(i, letter) for i, letter in enumerate(word)])
        self.assertEqual(result['final']['end_digest'], self.f.solved)
        engine = self.f.at((0, 12), (self.f.c2, 12))
        answers = [result, engine.units(), engine.word_check([[0, 45]], unit=self.f.a, cap=0), engine.heights(),
                   engine.seam_facts('all', chunk=SEAM_CHUNK, budget=SEAM_BUDGET, rows='mismatched'),
                   engine.seam_facts(0, chunk=SEAM_CHUNK, budget=SEAM_BUDGET, rows='mismatched')]
        for answer in answers:
            self.assertEqual(set(keys_anywhere(answer)) & FORBIDDEN_KEYS, set())


class SeamTests(unittest.TestCase):
    """Lead L6: JI.8."""

    def setUp(self):
        self.f = Fixture.get()

    def test_core_cell_and_named_cap_scope(self):
        """JI.8: after (0, 12) the identity domain holds the core and no longer the cell of cap 0's centre, so its core
        cell at pole 0 is a listed mismatch; the named-cap scope reports N_c with total N undefined; the all-scope
        total is its on-grip plus its off-grip part."""
        engine = self.f.at((0, 12))
        lattice = engine.heights()['lattice_key']
        facts = engine.seam_facts('all', chunk=SEAM_CHUNK, budget=SEAM_BUDGET, rows='mismatched')
        core = [row for row in facts['rows'] if row['domain'] == lattice and 'core' in (row['lower'], row['upper'])]
        self.assertTrue(core, 'no mismatched core cell in the identity domain')
        self.assertIn(0, {row['pole'] for row in core})
        totals = facts['totals']
        self.assertEqual(totals['total_N'], totals['on_grip_mismatches'] + totals['off_grip_mismatches'])
        self.assertEqual(totals['total_N'], len(facts['rows']))
        named = engine.seam_facts(0, chunk=SEAM_CHUNK, budget=SEAM_BUDGET)
        self.assertTrue(named['complete'])
        self.assertIsNone(named['totals']['total_N'])
        self.assertIsInstance(named['totals']['N_c'], int)


class RecordReaderTests(unittest.TestCase):
    """Lead L8 on the candidate's state record reader (KN2.14, JI.5): form is validated, poses are canonical."""

    def setUp(self):
        self.f = Fixture.get()
        self.engine = self.f.at((0, 12), (self.f.c2, 12))
        self.record = self.engine.record()

    def assert_refused(self, record):
        before = self.engine.record()
        with self.assertRaises(R.CandidateError) as caught:
            self.engine.load(record)
        self.assertEqual(caught.exception.kind, 'input')
        self.assertEqual(self.engine.record(), before)

    def test_shuffled_table_reads_back_canonically(self):
        """A table whose off-lattice entries come in another order, with the pose IDs remapped, is the same
        configuration: equal digest and the same canonical record."""
        table = self.record['pose_table']
        self.assertEqual(len(table), 3)
        shuffled = with_pose_ids(self.record, [table[0], table[2], table[1]], {1: 2, 2: 1})
        self.assertEqual(self.engine.load(shuffled).digest, self.record['digest'])
        self.assertEqual(self.engine.record(), self.record)

    def test_malformed_tables_are_refused(self):
        """A K+ pose written as a matrix (the identity), a repeated pose, a pose ID past the table, the ID 2^32 - 1
        and a wrong count are refused before any state is made; none is read as an off-lattice pose."""
        table = self.record['pose_table']
        identity = {'m': [[[1, 0, 1] if i == j else [0, 0, 1] for j in range(4)] for i in range(4)]}
        raw = base64.b64decode(self.record['pose_ids']['data'])
        past = struct.pack('<I', len(table)) + raw[4:]
        minus_one = struct.pack('<I', 0xFFFFFFFF) + raw[4:]
        cases = {
            'identity as a matrix first': raw_record(self.record, [identity] + table[1:], raw),
            'identity as a matrix entry': raw_record(self.record, table + [identity], raw),
            'repeated pose': raw_record(self.record, table + [table[1]], raw),
            'pose ID past the table': raw_record(self.record, table, past),
            'pose ID 2^32 - 1': raw_record(self.record, table, minus_one),
            'wrong count': raw_record(self.record, table, raw[:-4], count=NP - 1),
        }
        for name, record in cases.items():
            with self.subTest(case=name):
                self.assert_refused(record)


class HeightCountTests(unittest.TestCase):
    """Lead L17: JI.6's sparse histogram counts pieces; a zero above-6 count has no maximum."""

    def test_s4_word_check_keeps_odd_bins(self):
        """A word check on an S4 state completes and keeps its odd bins: one S4 letter from solved that leaves the
        lattice moves its pieces from height 0 to height 1; at solved the above-6 maximum is undefined."""
        menu = fixture('S4')['menu']
        engine = candidate()
        engine.start(menu)
        checks = [engine.word_check([[0, item]]) for item in range(len(menu['base']))]
        item = next(i for i, check in enumerate(checks) if check['off_after'] > 0)
        check = checks[item]
        self.assertTrue(check['admissible'])
        moved = engine.outcome(letter_step(0, item))['moved']
        self.assertEqual(check['off_after'], moved)
        self.assertEqual((check['heights_before']['n_above_6'], check['heights_before']['max_above_6']), (0, None))
        self.assertEqual((check['delta_n_h'].get('n_0'), check['delta_n_h'].get('n_1')), (-moved, moved))
        self.assertEqual(sum(check['delta_n_h'].values()), 0)
        self.assertEqual(check['peak_left_height'], 1)

    def test_ia_letter_counts_pieces_at_height_two(self):
        """After (0, 12) the n_2 count is the moved count: multiplicities of pieces, not distinct poses."""
        engine = candidate()
        engine.start(fixture('I_a')['menu'])
        result = engine.prefix_observations([[0, 12]], chunk=1, budget=1, fields=['heights'])
        row = result['rows'][0]
        self.assertEqual(row['delta_n_h']['n_2'], row['moved'])
        self.assertEqual(result['final']['peak_left_height'], 2)


class FreePoseTests(unittest.TestCase):
    """Lead L10 (part): JI.53 and LB3.1 on a raw free-mode table. The domain ID of the rotation against `trees.py` is
    CT-KN2-02's free-mode case in `tests/test_jumbling_rust_candidate.py`; the coset side (id(g k) = id(g) for k in
    K+) is not run here."""

    def test_one_piece_at_a_cayley_rotation(self):
        """A raw table with one piece at the exact Cayley rotation R(omega = (2, 0, 0)) at cap 0: one piece off the
        lattice, every piece at height 0, and one unit, that piece, in a domain other than the lattice's."""
        engine = candidate()
        engine.start(None)
        matrix = engine.cayley(0, [[2, 0, 1], [0, 0, 1], [0, 0, 1]])['matrix']
        piece = 1000
        ids = [0] * NP
        ids[piece] = 1
        raw = struct.pack(f'<{NP}I', *ids)
        record = {'pose_table': [{'k': 0}, {'m': matrix}],
                  'pose_ids': {'encoding': 'u32le-base64', 'count': NP, 'sha256': hashlib.sha256(raw).hexdigest(),
                               'data': base64.b64encode(raw).decode('ascii')}}
        engine.load(record)
        heights = engine.heights()
        self.assertEqual(heights['off'], 1)
        self.assertEqual(heights['height_counts']['n_0'], NP)
        self.assertEqual((heights['phi'], heights['max_left_height']), (0, 0))
        units = engine.units()
        self.assertEqual([row['members'] for row in units['units']], [[piece]])
        self.assertEqual(units['units'][0]['heights'], [0, 0])
        self.assertNotEqual(units['units'][0]['domain'], heights['lattice_key'])


class StampTests(unittest.TestCase):
    """Lead L13 on the candidate: KN1.4 refuses another contract stamp, naming contract_rev."""

    def test_other_stamps_are_refused(self):
        engine = candidate()
        start = engine.start(fixture('I_a')['menu'])
        own = engine.identity()
        for stamp in ('state-contract 2026-10-22 A1-A5', 'state-contract 2099-01-01 A9', None):
            with self.subTest(stamp=stamp):
                refused = engine.word(dict(own, contract_rev=stamp), JI16_WORD)
                self.assertEqual((refused['status'], refused['code'], refused['applied']),
                                 ('refused', IDENTITY_MISMATCH, 0))
                self.assertIn('contract_rev', refused['message'])
                self.assertEqual(refused['digest'], start.digest)


class DimensionTests(unittest.TestCase):
    """Lead L16 (dimension): JI.3 and JI.45. The adapter's draft reads a missing dimension as 4."""

    def setUp(self):
        self.f = Fixture.get()
        self.engine = self.f.at((0, 12))
        self.before = self.engine.record()

    def assert_refused(self, answer, argument, supplied):
        self.assertEqual((answer['status'], answer['code'], answer['argument'], answer['supplied'],
                          answer['supported']), ('refused', UNSUPPORTED_DIMENSION, argument, supplied, [4]))
        self.assertEqual(self.engine.record(), self.before)

    def test_other_dimensions_are_refused_before_the_arrays(self):
        """A request naming dimension 3 or 5 is refused with KN1:input.unsupported_dimension, with the supplied and
        the supported dimension, before its malformed arrays are read, and nothing changes."""
        for dimension in (3, 5):
            with self.subTest(dimension=dimension):
                for op, fields in (('units', {'pieces': 'not a list'}), ('word_check', {'letters': 'not a list'}),
                                   ('prefix_observations', {'letters': [[0, 'x']], 'chunk': 0}),
                                   ('seam_facts', {'scope': [], 'chunk': 0}), ('heights', {}),
                                   ('apply', {'step': {'kind': 'letter', 'grip': 0, 'item': 45}}),
                                   ('start', {'menu': 'not a menu'})):
                    with self.subTest(op=op):
                        self.assert_refused(self.engine.call(op, dimension=dimension, **fields), 'dimension',
                                            dimension)

    def test_record_header_dimension_is_read_first(self):
        """A state record whose header names dimension 3 is refused before its pose table is read."""
        record = {'dimension': 3, 'pose_table': 'not a table', 'pose_ids': None}
        self.assert_refused(self.engine.call('load', record=record), 'record.dimension', 3)

    def test_dimension_four_is_read(self):
        """Dimension 4, given or not, reads the request as before."""
        self.assertEqual(self.engine.call('units', dimension=4), self.engine.units())
        self.assertEqual(self.engine.call('load', record=dict(self.before, dimension=4))['digest'],
                         self.before['digest'])


class J1ReferenceFaults(unittest.TestCase):
    """J1's word-check reference (`differential/references/word_check.py`) against JI. Two tests check the
    preconditions; each other test states a JI rule and is an expected failure while the reference differs: a J1
    fault, reported and never fixed in J1."""

    @classmethod
    def setUpClass(cls):
        try:
            import numpy  # noqa: F401
        except ImportError as error:
            raise RuntimeError('NumPy is required: the J1 reference checks did not run (PHS.63)') from error
        # The census module first: it moves the research centroid cache out of the repository.
        from differential.leads import census  # noqa: F401
        from differential.j1_adapter import J1Engine
        from differential.references import word_check as WC
        cls.WC = WC
        cls.f = Fixture.get()
        cls.j1 = J1Engine()
        cls.j1.start(cls.f.menu)
        cls.solved = WC.copy_state(cls.j1.state)
        if cls.j1.apply(letter_step(0, 12)).status != 'admissible':
            raise AssertionError('J1 refuses the letter (0, 12)')
        cls.after_first = WC.copy_state(cls.j1.state)
        if cls.j1.apply(letter_step(cls.f.c2, 12)).status != 'admissible':
            raise AssertionError(f'J1 refuses the letter ({cls.f.c2}, 12) after (0, 12)')
        cls.after_two = WC.copy_state(cls.j1.state)
        cls.refused_word = [[0, 12], [cls.f.blocked, cls.f.identity]]

    def test_reference_refuses_the_same_letter(self):
        """The precondition of the faults below: the reference refuses the identity letter at the grip that (0, 12)
        blocks, as the candidate does, and A is the set it moves; the end-state values are what differ."""
        check = self.WC.word_check(self.solved, self.refused_word, ())
        self.assertEqual((check.admissible, check.failing_letter_index, check.failing_status, check.applied_letters),
                         (False, 1, 'blocked', 1))
        self.assertEqual(sorted(check.collateral_ids), self.f.a)

    def test_the_mixed_set_holds_two_exact_poses(self):
        """The precondition of the mixed-set fault below: in J1's state after (0, 12) and (c2, 12), a piece of A and a
        piece of B are both off the lattice, in two different exact poses."""
        a, b = self.f.a[0], self.f.b[0]
        flags = self.after_two.lattice_flags()
        self.assertFalse(bool(flags[a]) or bool(flags[b]))
        self.assertNotEqual(self.after_two.pose(a), self.after_two.pose(b))

    @unittest.expectedFailure
    def test_refused_prefix_has_no_end_digest(self):
        """JI.11: no end-state assertion covers a refused prefix. The reference reports the played prefix's end digest,
        delta-N and landing partition as the word's."""
        check = self.WC.word_check(self.solved, self.refused_word, ())
        self.assertIsNone(check.end_digest)

    @unittest.expectedFailure
    def test_no_unit_leaves_the_step_flag_undefined(self):
        """JI.11: without a named unit the step flag is undefined(unit_not_named). The reference takes unit=() and
        returns a boolean."""
        check = self.WC.word_check(self.after_first, [[0, 45]], ())
        self.assertIsNone(check.step_flag)

    @unittest.expectedFailure
    def test_lattice_piece_in_a_unit_is_refused(self):
        """JI.9: a lattice piece in the unit argument is refused (KN2:input.named_object_invalid). The reference
        accepts it and counts the piece as a landed unit member."""
        with self.assertRaises(ValueError):
            self.WC.word_check(self.after_first, [[0, 45]], (self.f.a[0], self.f.lattice))

    @unittest.expectedFailure
    def test_mixed_pose_set_in_a_unit_is_refused(self):
        """JI.9: the pieces of a named unit share one exact pose, and a set that mixes two is refused
        (KN2:input.named_object_invalid). The reference checks only the ID range and accepts it."""
        with self.assertRaises(ValueError):
            self.WC.word_check(self.after_two, [[0, 45]], (self.f.a[0], self.f.b[0]))


class J1ApiContracts(unittest.TestCase):
    """Leads L7 and L11 on J1's public API (`research/jumbling/sim/api.py`, JI.52), through which the references reach
    J1. `tests/test_jumbling_api.py` calls each function once; these add the refusal kinds, the single classification,
    the read-only pose entries and the fixture's prefix range. A failure here is a fault of that module, which this
    branch does not edit; it is an expected failure and reported."""

    @classmethod
    def setUpClass(cls):
        try:
            import numpy
        except ImportError as error:
            raise RuntimeError('NumPy is required: the J1 API checks did not run (PHS.63)') from error
        # The census module first: it moves the research centroid cache out of the repository.
        from differential.leads import census  # noqa: F401
        import sim
        from sim import api
        from sim.twists import TwistMenu, UnrepresentableTwist
        cls.np, cls.sim, cls.api, cls.unrepresentable = numpy, sim, api, UnrepresentableTwist
        cls.f = Fixture.get()
        cls.ctx = sim.get_context()
        cls.menu = TwistMenu.from_record(cls.ctx, cls.f.menu)
        cls.solved = sim.State(cls.ctx, menu=cls.menu)
        cls.after_first = api.copy_state(cls.solved)
        if not api.apply_with_inside(cls.after_first, cls.twist(0, 12))[0].applied:
            raise AssertionError('J1 refuses the letter (0, 12)')

    @classmethod
    def twist(cls, grip, item):
        return cls.menu.for_cap(grip)[item]

    def assert_refused_unchanged(self, state, twist, status, budget=None):
        before = (state.digest(), state.snapshot())
        outcome, inside = self.api.apply_with_inside(state, twist, budget)
        self.assertEqual((outcome.status, bool(outcome.applied), int(inside.size)), (status, False, 0))
        self.assertEqual((state.digest(), state.snapshot()), before)
        self.assertNotIn('_commit', state.__dict__)

    def stub_apply(self):
        """J1's apply admitting every twist at once: the fixture replay's bookkeeping without the classification."""
        admitted = types.SimpleNamespace(applied=True)
        return unittest.mock.patch.object(self.sim.State, 'apply', autospec=True, return_value=admitted)

    def test_refusals_return_the_empty_set_and_change_nothing(self):
        """JI.52 (L7): a refusal returns the empty set and changes nothing. After (0, 12): the identity letter at a
        grip it blocks, the letter (0, 45) with an evaluation budget of 0, and a requested rotation with no exact
        representation. `tests/test_jumbling_api.py` runs an off-menu twist."""
        state = self.api.copy_state(self.after_first)
        self.assert_refused_unchanged(state, self.twist(self.f.blocked, self.f.identity), 'blocked')
        self.assert_refused_unchanged(state, self.twist(0, 45), 'uncertain', budget=0)
        self.assert_refused_unchanged(state, self.unrepresentable(0, self.np.eye(4)), 'uncertain')

    def test_an_admitted_twist_is_classified_once(self):
        """JI.52 (L7): the inside set returned is the one J1 committed, with no second classification."""
        state = self.api.copy_state(self.solved)
        classify = self.sim.State.classify
        with unittest.mock.patch.object(self.sim.State, 'classify', autospec=True, side_effect=classify) as spy:
            outcome, inside = self.api.apply_with_inside(state, self.twist(0, 12))
        self.assertTrue(outcome.applied)
        self.assertEqual(spy.call_count, 1)
        self.assertEqual(sorted(inside.tolist()), self.f.a)

    def test_pose_entries_are_immutable(self):
        """JI.52 (L11): pose_entries is a read-only copy; its entries and their exact matrices cannot be changed, so a
        caller cannot reach the state's pose table through them."""
        entries = self.api.pose_entries(self.after_first)
        self.assertTrue(any(entry.matrix is not None for entry in entries))
        for entry in entries:
            self.assertIsInstance(entry, tuple)
            if entry.matrix is not None:
                self.assertIsInstance(entry.matrix, tuple)
                self.assertTrue(all(isinstance(row, tuple) for row in entry.matrix))

    def test_fixture_state_replays_the_first_t_records(self):
        """The precondition of the range fault below: with J1's apply stubbed to admit, fixture_state replays t
        records for a t inside the fixture and empties the journal."""
        with self.stub_apply() as apply:
            state, _ = self.api.fixture_state(self.ctx, 'I_a', 3)
        self.assertEqual((apply.call_count, state.journal), (3, []))

    @unittest.expectedFailure
    def test_fixture_state_refuses_a_prefix_outside_the_fixture(self):
        """JI.52 (L11): fixture_state gives the state after the first t records, which exists only for t from 0 to the
        record count. The module slices the records instead, so t = -1 gives all but the last record and a t past the
        end the whole fixture, both silently."""
        count = len(fixture('I_a')['journal']['records'])
        with self.stub_apply():
            for t in (-1, count + 1):
                with self.assertRaises((ValueError, IndexError)):
                    self.api.fixture_state(self.ctx, 'I_a', t)


if __name__ == '__main__':
    unittest.main()
