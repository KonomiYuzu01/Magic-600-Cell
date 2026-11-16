"""LP2, M5: the Rust jumbling candidate against J1 (LP2 plan, section 11, acceptance items 3 to 11).

CT-KN2-01 (part: the J1 harness; the 600-cell-Full oracle is M4's, in tests/test_kernel_oracle.py): `harness.compare`
runs J1 and the candidate (`rust/c600-jumble` behind `differential/candidate/rust_jumble.py`) step by step on the
first 60 records of each W-J fixture, with surveys at steps 0 and 30 and the straddler sets of named caps at step 30
(every candidate certificate must pass J1's independent check), on the first 600 letters of H6030 and on refusals; a
planted difference fails at its step. With JUMBLING_FULL_REPLAY=1 every record of the three fixtures and all 3,156
letters of H6030 run. Every compared step compares the pose digest: CT-KN1-05 (part: Digest<poses>; Digest<labels>
is M4's, in rust/c600-kernel/tests/kernel.rs).

The other cases, with PHS.62's markers: CT-KN1-02 (whole: a word recorded under the reversed I_a order is refused);
CT-KN2-02 (part: heights and domain IDs against trees.py and the library's histograms on H6030's compared states and
on u1, and the free-mode Cayley rotation; not run: the cases b20, b06-14, b14 and b03, for M8); CT-KN2-04 (part: a
blocked, an invalid and an uncertain letter of the jumbled puzzle; 600-cell-Full's blocked and invalid recipes are
M4's); CT-KN2-05 (part: no float type in c600-kernel and c600-jumble; M4's scan adds c600-digest); CT-KN3-02 (whole:
pose entries beyond 2^64 and a 6,775-digit integer through the pose table); and CT-JI-03 (seam counts, JI.16's vector
and a cancelled batch). CT-KN2-03 (whole) runs in `cargo test` (`rust/c600-jumble/tests/crossing.rs`).

The adapter is built with `cargo build --release --locked --offline` in rust/ (into the ignored rust/target). A
missing Rust toolchain or NumPy fails the check: a test that did not run is no pass (PHS.63). A pass is source and
fixture evidence, not Windows/DirectX, input, long-session or performance evidence; the printed seconds are no
performance claim.
"""

import sys

sys.dont_write_bytecode = True

from contextlib import contextmanager
from dataclasses import fields as dataclass_fields, replace
import json
import os
from pathlib import Path
import re
from time import perf_counter
import unittest

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / 'research' / 'jumbling'
FIXTURE_DIR = JUMBLING / 'fixtures'
H6030 = JUMBLING / 'restoration/10-x1-unbounded/witnesses/H6030.json'
U1 = JUMBLING / 'restoration/12-low-height/runs/lc13-k40/steps/u1.json'
FULL = os.environ.get('JUMBLING_FULL_REPLAY') == '1'

FIXTURES = ('S4', 'I_a', 'I_b')
RECORDS = 60
SURVEYED = {0, 30}
H6030_LETTERS = 600
H6030_OBSERVED = {200, 400, 600} | ({1200, 2400, 3156} if FULL else set())
# Named caps at step 30: these two and the first four blocked grips there.
FIXED_CAPS = (0, 599)
BLOCKED_CAPS = 4
PLANTED_STEP = 7
# The Cayley parameter omega = (2, 0, 0): exact [a, b, d] triples.
OMEGA_2 = [[2, 0, 1], [0, 0, 1], [0, 0, 1]]
# JI.16: the supplied round-trip word from solved in the pinned I_a menu.
JI16_WORD = [[0, 12], [0, 45]]
# Batches (selection 5; JI.54): a seam scan of every cell in one chunk, and JI.16's prefix observations one letter
# per chunk. The budgets bound the batch and are larger than any batch here, except where a test cuts one off.
SEAM_CHUNK = 1 << 16
SEAM_BUDGET = 1 << 24
PREFIX_CHUNK = 1
IDENTITY_MISMATCH = 'KN1:input.identity_mismatch'
FLOAT_TYPE = re.compile(r'(?<![A-Za-z_.])f(?:32|64)\b')

_SECONDS = {}
_ENGINES = {}


def setUpModule():
    global np, H, M, R, WC, obs_refs, census, O, sim, api, trees, rim, J1Engine
    try:
        import numpy as np
    except ImportError as error:
        raise RuntimeError('NumPy is required: the jumbling comparison did not run (PHS.63)') from error
    sys.path.insert(0, str(JUMBLING))
    # The census module first: it moves the research centroid cache out of the repository before any theory import.
    from differential.leads import census
    from differential import harness as H, menu_order as M
    from differential.candidate import rust_jumble as R
    from differential.j1_adapter import J1Engine
    from differential.references import obs_refs, observations as O, word_check as WC
    import rim
    import sim
    from sim import api
    import trees
    if Path(census.seams.CACHE).resolve().is_relative_to(ROOT):
        raise AssertionError('the centroid cache is inside the repository')
    started = perf_counter()
    R.build()
    _SECONDS['build'] = perf_counter() - started
    started = perf_counter()
    _ENGINES['candidate'] = R.RustJumbleEngine(step_result=H.StepResult)
    _ENGINES['j1'] = J1Engine()
    _SECONDS['load'] = perf_counter() - started


def tearDownModule():
    engine = _ENGINES.pop('candidate', None)
    if engine is not None:
        engine.close()
    if _SECONDS:
        print('seconds: ' + ', '.join(f'{k} {v:.1f}' for k, v in _SECONDS.items()), file=sys.stderr)


def candidate():
    return _ENGINES['candidate']


def j1():
    return _ENGINES['j1']


def fixture(name):
    return json.loads((FIXTURE_DIR / f'wj-{name}.json').read_text(encoding='utf-8'))


def manifest_model_id():
    return json.loads((ROOT / 'assets/manifest.json').read_text(encoding='utf-8'))['model_id']


def record_step(record):
    return {'kind': 'record', 'record': record}


def letter_step(grip, item):
    return {'kind': 'letter', 'grip': grip, 'item': item}


@contextmanager
def unlimited_integer_digits():
    """Python refuses int/str conversions above 4,300 digits by default; CT-KN3-02 needs 6,775."""
    limit = sys.get_int_max_str_digits()
    sys.set_int_max_str_digits(0)
    try:
        yield
    finally:
        sys.set_int_max_str_digits(limit)


def j1_heights(state):
    """J1's per-pose heights, branches and domain ID (trees.py and the library's exact coset key), by exact pose key."""
    entries = api.pose_entries(state)
    rows = {}
    for pid, count in zip(*np.unique(state.pose_id, return_counts=True)):
        pid = int(pid)
        matrix = api.pose_matrix(state, pid)
        if entries[pid].kidx >= 0:
            heights, branches, domain = [0, 0], [-1, -1], O.LATTICE_KEY
        else:
            dl, dr, bl, br = trees.pose_tree(matrix)
            heights, branches, domain = [dl, dr], [bl, br], O.domain_key(O.to_tuple(matrix))
        rows[O.pose_key(matrix)] = {'domain': domain, 'heights': heights, 'branches': branches, 'count': int(count)}
    return rows


def max_left_height(summary):
    return max(int(key.split(',')[0]) for key in summary['pieces_by_heights'])


class TimedCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._started = perf_counter()

    @classmethod
    def tearDownClass(cls):
        _SECONDS[cls.__name__] = perf_counter() - cls._started

    def assert_heights(self, label, state, recorded=None):
        """CT-KN2-02: the candidate's heights and domain IDs equal trees.py per pose class, and its histograms and X5
        counts equal the library's (and the recorded trees-fixtures row where one exists)."""
        got = candidate().heights()
        expected = j1_heights(state)
        self.assertEqual(got['lattice_key'], O.LATTICE_KEY, label)
        self.assertEqual(set(got['poses']), set(expected), f'{label}: occupied pose classes')
        for key, row in expected.items():
            self.assertEqual(got['poses'][key], row, f'{label}: pose {key[:48]}')
        summary = obs_refs._tree_summary(state)
        for field in ('pieces_by_heights', 'pose_classes_by_heights'):
            self.assertEqual(got['histograms'][field], summary[field], f'{label}: {field}')
            if recorded is not None:
                self.assertEqual(got['histograms'][field], recorded[field], f'{label}: recorded {field}')
        self.assertEqual(got['height_counts'], summary['height_counts'], f'{label}: X5 counts')
        self.assertEqual(got['max_left_height'], max_left_height(summary), label)
        self.assertEqual(got['off'], int(state.off_lattice_count()), label)
        return got


class Observed:
    """The candidate behind compare, with a check after chosen steps once its pose digest equals J1's (compare applies
    the reference first, so J1 is at the same step; a differing digest is left for compare to report)."""

    def __init__(self, engine, reference, at, check):
        self.name = engine.name
        self._engine, self._reference, self._at, self._check = engine, reference, set(at), check
        self._step = 0
        self.checked = []
        self.results = []

    def start(self, menu):
        self._step = 0
        return self._engine.start(menu)

    def apply(self, step):
        result = self._engine.apply(step)
        self._step += 1
        self.results.append(result)
        if self._step in self._at and result.digest == self._reference.state.digest():
            self._check(self._step)
            self.checked.append(self._step)
        return result

    def survey(self, grips):
        return self._engine.survey(grips)

    def straddlers(self, caps):
        return self._engine.straddlers(caps)


class Planted(Observed):
    """The candidate with its pose digest changed at one step."""

    def __init__(self, engine, at):
        super().__init__(engine, None, (), None)
        self._planted = at

    def apply(self, step):
        result = self._engine.apply(step)
        self._step += 1
        if self._step == self._planted:
            flipped = '0' if result.digest[-1] != '0' else '1'
            result = replace(result, digest=result.digest[:-1] + flipped)
        return result


def blocked_and_admissible(engine, menu, records):
    """The candidate's survey after the given records: the sorted blocked and admissible grips."""
    engine.start(menu)
    for record in records:
        engine.apply(record_step(record))
    grips = engine.survey(range(600))
    return (sorted(g for g, v in grips.items() if v == 'b'), sorted(g for g, v in grips.items() if v == 'a'))


class SourceScanTests(TimedCase):
    def test_no_float_type_in_the_kernel_crates(self):
        """CT-KN2-05 (part: the sources of c600-kernel and c600-jumble; M4's scan adds c600-digest): no f32 or f64
        in them (a file name such as cell_normals.f32 is no type). INV-4 names no kernel site where a float may stand.
        The INV-4 half of the item (no display plan, A2 filter or float request in these crates) is not scanned: their
        modules are listed in rust/README.md, and a record's float request (params.requested) is an adapter error
        (selection 2, RefusalTests)."""
        sources = sorted((ROOT / 'rust/c600-kernel').rglob('*.rs')) + sorted((ROOT / 'rust/c600-jumble').rglob('*.rs'))
        self.assertTrue(sources)
        found = []
        for path in sources:
            for number, line in enumerate(path.read_text(encoding='utf-8').splitlines(), 1):
                if FLOAT_TYPE.search(line):
                    found.append(f'{path.relative_to(ROOT).as_posix()}:{number}')
        self.assertEqual(found, [])

    def test_the_scan_finds_a_float_type(self):
        for line in ('let x: f64 = 1.0;', 'let y = 2f32;', 'fn f(v: Vec<f64>)', 'x as f32'):
            self.assertTrue(FLOAT_TYPE.search(line), line)
        for line in ('assets/cell_normals.f32', 'let buf64 = 0;', 'u64'):
            self.assertFalse(FLOAT_TYPE.search(line), line)


class FixtureComparisonTests(TimedCase):
    """CT-KN2-01 and CT-KN1-05 on the W-J fixtures, with CT-KN2-02's heights at the recorded prefixes."""

    def compare_fixture(self, name):
        doc = fixture(name)
        steps = H.load_steps(doc)
        count = len(steps.steps) if FULL else RECORDS
        steps = replace(steps, steps=steps.steps[:count])
        blocked, _ = blocked_and_admissible(candidate(), steps.menu, doc['journal']['records'][:30])
        self.assertTrue(blocked, f'{name}@30 has a blocked grip to name')
        caps = sorted(set(FIXED_CAPS) | set(blocked[:BLOCKED_CAPS]))
        rows = {row['t']: row for row in json.loads(
            (JUMBLING / 'theory/results/trees-fixtures.json').read_text(encoding='utf-8'))[name]}
        observed = {t for t in rows if t <= count}

        def check(t):
            self.assert_heights(f'{name}@{t}', j1().state, rows[t])

        engine = Observed(candidate(), j1(), observed, check)
        report = H.compare(j1(), engine, steps, survey=SURVEYED, straddlers={30}, caps=caps, certify=j1().certify)
        self.assertTrue(report.passed, report.as_dict())
        self.assertEqual(report.steps_compared, count + 1)
        self.assertEqual(report.surveys_compared, len(SURVEYED))
        self.assertEqual(report.straddlers_compared, 1)
        self.assertEqual(engine.checked, sorted(observed))

    def test_S4(self):
        self.compare_fixture('S4')

    def test_I_a(self):
        self.compare_fixture('I_a')

    def test_I_b(self):
        self.compare_fixture('I_b')

    def test_certificates_at_step_30_pass_J1s_check(self):
        """Every candidate certificate at the named caps of I_a@30 passes J1's independent check, and a certificate
        moved to another piece fails it."""
        doc = fixture('I_a')
        records = doc['journal']['records'][:30]
        blocked, _ = blocked_and_admissible(candidate(), doc['menu'], records)
        j1().start(doc['menu'])
        for record in records:
            j1().apply(record_step(record))
        self.assertEqual(candidate().record()['digest'], j1().state.digest())
        rows = candidate().straddlers(blocked[:BLOCKED_CAPS])
        checked = 0
        for cap, row in rows.items():
            self.assertEqual(row['status'], 'blocked')
            self.assertEqual(sorted(row['certificates']), row['pieces'])
            for piece in row['pieces']:
                certificate = row['certificates'][piece]
                self.assertTrue(j1().certify(cap, piece, certificate), (cap, piece))
                self.assertTrue(candidate().certify(cap, piece, certificate), (cap, piece))
                checked += 1
            other = next(p for p in range(len(j1().state.pose_id)) if p not in row['pieces'])
            moved = dict(row['certificates'][row['pieces'][0]], piece=other)
            self.assertFalse(j1().certify(cap, other, moved))
            self.assertFalse(candidate().certify(cap, other, moved))
        self.assertGreater(checked, 0)


class H6030Tests(TimedCase):
    def test_h6030_letters_and_heights(self):
        """CT-KN2-01 on H6030's letters (the first 600; all 3,156 in full mode) and CT-KN2-02's heights and domain IDs
        on its compared states."""
        menu = fixture('I_a')['menu']
        steps = H.load_steps(H6030, menu)
        self.assertEqual(len(steps.steps), 3156)
        count = len(steps.steps) if FULL else H6030_LETTERS
        steps = replace(steps, steps=steps.steps[:count])
        observed = {t for t in H6030_OBSERVED if t <= count}
        engine = Observed(candidate(), j1(), observed, lambda t: self.assert_heights(f'H6030@{t}', j1().state))
        report = H.compare(j1(), engine, steps)
        self.assertTrue(report.passed, report.as_dict())
        self.assertEqual(report.steps_compared, count + 1)
        self.assertEqual(engine.checked, sorted(observed))


class RefusalTests(TimedCase):
    def setUp(self):
        self.doc = fixture('I_a')
        self.records = self.doc['journal']['records']
        self.blocked, self.admissible = blocked_and_admissible(candidate(), self.doc['menu'], self.records[:30])
        self.assertTrue(self.blocked and self.admissible)

    def refusals(self):
        """A blocked twist, a twist outside the menu (an exact Cayley rotation at an admissible grip) and a malformed
        record (every entry doubled, so no rotation)."""
        off_menu = candidate().cayley(self.admissible[0], OMEGA_2)
        record = self.records[30]
        malformed = dict(record, matrix=[[[2 * a, 2 * b, d] for a, b, d in row] for row in record['matrix']])
        return [letter_step(self.blocked[0], 0), record_step(off_menu), record_step(malformed)]

    def test_refusals_compare_and_change_nothing(self):
        """CT-KN2-01's refusals: J1 and the candidate refuse the same steps alike, nothing changes (compare checks
        it for both), and the next record still applies."""
        steps = H.load_steps(self.doc)
        steps = replace(steps, steps=steps.steps[:30] + tuple(self.refusals()) + (record_step(self.records[30]),))
        engine = Observed(candidate(), j1(), (), None)
        report = H.compare(j1(), engine, steps, survey={30, 33}, straddlers={33}, caps=self.blocked[:2],
                           certify=j1().certify)
        self.assertTrue(report.passed, report.as_dict())
        self.assertEqual([r.status for r in engine.results[30:]], ['blocked', 'invalid', 'invalid', 'admissible'])

    def test_letter_item_outside_the_menu_is_an_input_error(self):
        """A letter whose item lies outside the menu base names no twist, so it is no step to compare: J1's adapter
        and the candidate both reject it as malformed input, and nothing changes. A twist outside the menu is a
        refusal and runs in the comparison above."""
        item = len(self.doc['menu']['base'])
        before = candidate().record()
        with self.assertRaises(R.CandidateError) as caught:
            candidate().apply(letter_step(self.admissible[0], item))
        self.assertEqual(caught.exception.kind, 'input')
        self.assertEqual(candidate().record(), before)
        j1().start(self.doc['menu'])
        with self.assertRaises(H.HarnessInputError):
            j1().apply(letter_step(self.admissible[0], item))

    def test_records_the_candidate_does_not_read_are_input_errors(self):
        """Selection 2: a record of another family or with a float request is an adapter error, never a refusal,
        even when its grip or matrix is malformed too; nothing changes."""
        engine = candidate()
        engine.start(self.doc['menu'])
        before = engine.record()
        float_request = dict(self.records[0], params={'requested': {'degrees': 72.0}})
        for record in ({'grip': 0, 'family': 'x', 'matrix': [[[1, 0, 1]]]}, {'grip': 700, 'family': 'x'},
                       dict(float_request, grip=700)):
            with self.subTest(record=record):
                with self.assertRaises(R.CandidateError) as caught:
                    engine.apply(record_step(record))
                self.assertEqual(caught.exception.kind, 'input')
                self.assertEqual(engine.record(), before)

    def test_a_word_is_read_whole_before_any_letter_is_applied(self):
        """A word whose later letter names no grip, no menu item or no pair is an input error, and the state stays
        the one before the word, though its first letter alone is admissible."""
        engine = candidate()
        engine.start(self.doc['menu'])
        before = engine.record()
        identity = engine.identity()
        for bad in ([600, 0], [0, len(self.doc['menu']['base'])], [0]):
            with self.subTest(letter=bad):
                with self.assertRaises(R.CandidateError) as caught:
                    engine.word(identity, [[0, 12], bad])
                self.assertEqual(caught.exception.kind, 'input')
                self.assertEqual(engine.record(), before)
        moved = engine.word(identity, [[0, 12]])
        self.assertEqual(moved['status'], 'admissible')
        self.assertNotEqual(moved['digest'], before['digest'])

    def test_a_step_before_start_is_an_input_error(self):
        """A record step needs no menu but does need a started state: before start it is an input error, and the
        adapter keeps answering."""
        with R.RustJumbleEngine(step_result=H.StepResult) as fresh:
            with self.assertRaises(R.CandidateError) as caught:
                fresh.apply(record_step({'grip': 0}))
            self.assertEqual(caught.exception.kind, 'input')
            self.assertEqual(fresh.start(None).status, 'start')

    def test_planted_difference_fails_at_its_step(self):
        steps = H.load_steps(self.doc)
        steps = replace(steps, steps=steps.steps[:10])
        report = H.compare(j1(), Planted(candidate(), PLANTED_STEP), steps)
        self.assertFalse(report.passed)
        self.assertEqual((report.first_mismatch.step, report.first_mismatch.step_index_in_input,
                          report.first_mismatch.field), (PLANTED_STEP, PLANTED_STEP - 1, 'digest'))
        self.assertEqual(report.steps_compared, PLANTED_STEP + 1)

    def test_refused_letters_leave_the_state_unchanged(self):
        """CT-KN2-04 (part: the jumbling half): a blocked, an invalid and an uncertain (budget) letter each leave the
        pose digest, the lattice digest and the pose table unchanged; the same budget gives the same outcome again."""
        engine = candidate()
        engine.start(self.doc['menu'])
        for record in self.records[:30]:
            engine.apply(record_step(record))
        before = engine.record()
        lattice = engine.outcome(letter_step(self.blocked[0], 0))['lattice']
        self.assertEqual(engine.record(), before)
        blocked, off_menu, malformed = self.refusals()
        for step, status in ((blocked, 'blocked'), (off_menu, 'invalid'), (malformed, 'invalid')):
            outcome = engine.outcome(step)
            self.assertEqual(outcome['status'], status)
            self.assertEqual((outcome['digest'], outcome['lattice'], outcome['moved']), (before['digest'], lattice, 0))
            self.assertEqual(engine.record(), before)
        engine.keep('ct-kn2-04')
        found = None
        for grip in self.admissible:
            outcome = engine.outcome(letter_step(grip, 0))
            engine.use('ct-kn2-04')
            if outcome['status'] == 'admissible' and outcome['exact_evaluations'] > 0:
                found = grip, outcome['exact_evaluations']
                break
        self.assertIsNotNone(found, 'an admissible letter at I_a@30 that needs an exact evaluation')
        grip, evaluations = found
        outcomes = [engine.outcome(letter_step(grip, 0), budget=evaluations - 1) for _ in range(2)]
        self.assertEqual(outcomes[0], outcomes[1])
        self.assertEqual(outcomes[0]['status'], 'uncertain')
        self.assertEqual((outcomes[0]['digest'], outcomes[0]['lattice'], outcomes[0]['moved']),
                         (before['digest'], lattice, 0))
        self.assertEqual(engine.record(), before)
        # The cap's seam facts under the same budget claim nothing (JI.14); under the full budget they are complete.
        facts = engine.seam_facts(grip, chunk=SEAM_CHUNK, budget=SEAM_BUDGET, eval_budget=evaluations - 1)
        self.assertEqual((facts['status'], facts['complete'], facts['totals']), ('uncertain', False, None))
        facts = engine.seam_facts(grip, chunk=SEAM_CHUNK, budget=SEAM_BUDGET, eval_budget=evaluations)
        self.assertEqual((facts['status'], facts['complete']), ('admissible', True))
        self.assertEqual(engine.record(), before)
        self.assertEqual(engine.outcome(letter_step(grip, 0), budget=evaluations)['status'], 'admissible')
        self.assertTrue(engine.release('ct-kn2-04'))


class IdentityTests(TimedCase):
    def test_fixture_identity_blocks(self):
        """The candidate's identity block of each fixture menu (JI.54 records S4's and I_b's): the manifest's model,
        the menu identity and the order digest."""
        for name in FIXTURES:
            with self.subTest(fixture=name):
                menu = fixture(name)['menu']
                candidate().start(menu)
                block = candidate().identity()
                self.assertEqual(block['puzzle'], 'jumbling')
                self.assertEqual(block['model_id'], manifest_model_id())
                self.assertEqual(block['menu_id'], menu['identity'])
                self.assertEqual(block['menu_order'], M.order_digest(menu))
                self.assertTrue(block['contract_rev'])
        candidate().start(None)
        self.assertEqual((candidate().identity()['menu_id'], candidate().identity()['menu_order']), (None, None))

    def test_word_under_the_reversed_order_is_refused(self):
        """CT-KN1-02 (whole): a word recorded under the reversed I_a order has the same menu identity and another
        order digest, and it is refused whole, naming menu_order; nothing changes. The same word under the loaded order
        applies."""
        menu = fixture('I_a')['menu']
        reversed_menu = dict(menu, base=menu['base'][::-1])
        self.assertEqual(M.menu_identity(reversed_menu), M.I_A_IDENTITY)
        self.assertNotEqual(M.order_digest(reversed_menu), M.I_A_ORDER_DIGEST)
        for loaded, recorded in ((menu, reversed_menu), (reversed_menu, menu)):
            start = candidate().start(loaded)
            own = candidate().identity()
            block = dict(own, menu_order=M.order_digest(recorded))
            self.assertEqual(block['menu_id'], own['menu_id'])
            refused = candidate().word(block, JI16_WORD)
            self.assertEqual((refused['status'], refused['code'], refused['applied']), ('refused', IDENTITY_MISMATCH, 0))
            self.assertIn('menu_order', refused['message'])
            self.assertEqual((refused['digest'], refused['lattice']), (start.digest, start.lattice))
        candidate().start(menu)
        applied = candidate().word(candidate().identity(), JI16_WORD)
        self.assertEqual((applied['status'], applied['applied'], applied['refused_at']), ('admissible', 2, None))
        self.assertEqual(applied['digest'], start.digest)


class FreeModeTests(TimedCase):
    def test_cayley_rotation_outside_kplus(self):
        """CT-KN2-02 (part: the free-mode case): the Cayley rotation omega = (2,0,0) at cap 0 reads heights (0,0), lies
        outside K+, and its domain ID (the exact coset key) differs from the identity's; J1 agrees."""
        record = candidate().cayley(0, OMEGA_2)
        steps = H.Steps((record_step(record),))
        report = H.compare(j1(), Observed(candidate(), j1(), (), None), steps)
        self.assertTrue(report.passed, report.as_dict())
        heights = self.assert_heights('free mode', j1().state)
        off_lattice = [row for row in heights['poses'].values() if row['domain'] != heights['lattice_key']]
        self.assertEqual(len(off_lattice), 1)
        self.assertEqual(off_lattice[0]['heights'], [0, 0])
        entries = api.pose_entries(j1().state)
        self.assertEqual(sum(entry.kidx < 0 for entry in entries), 1)
        table = candidate().record()['pose_table']
        self.assertEqual([sorted(entry) for entry in table], [['k'], ['m']])

    def test_big_integers_round_trip_through_the_pose_table(self):
        """CT-KN3-02 (whole): pose entries beyond 2^64 and a 6,775-digit integer round-trip exactly through the
        candidate's pose-table output and input; J1's digest agrees on the first."""
        with unlimited_integer_digits():
            for omega, digits in (([[2 ** 40, 0, 1], [0, 0, 1], [0, 0, 1]], 25),
                                  ([[10 ** 3387, 0, 1], [0, 0, 1], [0, 0, 1]], 6775)):
                with self.subTest(digits=digits):
                    candidate().start(None)
                    step = candidate().apply(record_step(candidate().cayley(0, omega)))
                    self.assertEqual(step.status, 'admissible')
                    written = candidate().record()
                    entries = [abs(n) for entry in written['pose_table'] if 'm' in entry
                               for row in entry['m'] for triple in row for n in triple]
                    self.assertGreater(max(entries), 2 ** 64)
                    self.assertEqual(len(str(max(entries))), digits)
                    loaded = candidate().load(json.loads(json.dumps(written)))
                    self.assertEqual(loaded.digest, written['digest'])
                    self.assertEqual(candidate().record(), written)
                    if digits < 100:
                        j1().start(None)
                        self.assertEqual(j1().apply(record_step(candidate().cayley(0, omega))).digest, step.digest)


class U1Tests(TimedCase):
    def test_u1_heights(self):
        """CT-KN2-02 on ENG-0J item 8's in-repository state u1 (census seed 7101 before step 98)."""
        path, record = census.load(7101)
        bounds = [0]
        for step in record['steps']:
            bounds.append(bounds[-1] + step['twists'])
        letters = [list(m) for m in record['history']] + [list(m) for m in record['moves'][:bounds[97]]]
        state = census.state_at(7101, 98).st
        expected = json.loads(U1.read_text(encoding='utf-8'))['input_digest']
        self.assertEqual(state.digest(), expected)
        candidate().start(census.menu().record())
        for grip, item in letters:
            self.assertEqual(candidate().apply(letter_step(grip, item)).status, 'admissible')
        self.assertEqual(candidate().record()['digest'], expected)
        self.assert_heights('u1', state)


class SeamTests(TimedCase):
    def test_all_scope_counts_off_grip_cells(self):
        """CT-JI-03: seam_facts(all) counts off-grip cells. At the recorded I_a@40 stop (62 restorer letters), total N
        is the recorded 1,192 = 1,057 on-grip + 135 off-grip mismatches (the recorded off-grip cell count)."""
        reference = obs_refs.stop40_reference()
        doc = fixture('I_a')
        candidate().start(doc['menu'])
        for record in doc['journal']['records'][:40]:
            candidate().apply(record_step(record))
        for grip, item in reference['moves']:
            self.assertEqual(candidate().apply(letter_step(grip, item)).status, 'admissible')
        self.assertEqual(candidate().record()['digest'], reference['digest'])
        facts = candidate().seam_facts('all', chunk=SEAM_CHUNK, budget=SEAM_BUDGET)
        self.assertTrue(facts['complete'])
        totals = facts['totals']
        self.assertEqual((totals['total_N'], totals['on_grip_mismatches'], totals['off_grip_mismatches']),
                         (reference['n_total'], reference['grip_n'], reference['prose']['cells']))
        self.assertEqual(totals['total_N'], totals['on_grip_mismatches'] + totals['off_grip_mismatches'])
        cut = candidate().seam_facts('all', chunk=SEAM_CHUNK, budget=SEAM_CHUNK)
        self.assertEqual((cut['complete'], cut['totals'], cut['coverage'], cut['cut_off']),
                         (False, None, [0, SEAM_CHUNK], True))

    def test_supplied_table(self):
        """CT-JI-03: a supplied table with one on-grip and two off-grip mismatches gives total N 3."""
        cells = [{'occupancy': 'lower', 'grip': 'on'}, {'occupancy': 'upper', 'grip': 'off'},
                 {'occupancy': 'lower', 'grip': 'off'}, {'occupancy': 'both', 'grip': 'on'}]
        self.assertEqual(candidate().seam_totals(cells), {'total_N': 3, 'on_grip_mismatches': 1,
                                                          'off_grip_mismatches': 2, 'aligned': 1})


class JI16Tests(TimedCase):
    """CT-JI-03: JI.16's contract vector against exact replay through J1."""

    def setUp(self):
        self.menu = fixture('I_a')['menu']
        self.assertEqual(M.inverse_index(self.menu)[12], 45)

    def test_vector(self):
        engine = candidate()
        solved = engine.start(self.menu).digest
        j1().start(self.menu)
        # Step 2: total N 0, Phi 0 and every piece at height zero.
        facts = engine.seam_facts('all', chunk=SEAM_CHUNK, budget=SEAM_BUDGET)
        self.assertEqual((facts['totals']['total_N'], facts['totals']['on_grip_mismatches'],
                          facts['totals']['off_grip_mismatches']), (0, 0, 0))
        self.assertEqual(rim.Domains(j1().state).total_misalignment(), 0)
        heights = engine.heights()
        self.assertEqual((heights['phi'], heights['max_left_height']), (0, 0))
        self.assertEqual({tuple(row['heights']) for row in heights['poses'].values()}, {(0, 0)})
        # Step 3: a completed empty table.
        self.assertEqual(engine.units(), {'status': 'ok', 'units': [], 'complete': True})
        # Step 4: two admissible rows with F, D and delta-Phi zero; peak 2, excursion 0, tame, end digest solved.
        result = engine.prefix_observations(JI16_WORD, chunk=PREFIX_CHUNK, budget=len(JI16_WORD),
                                            fields=['N', 'heights', 'fd'], keep=True)
        rows = result['rows']
        self.assertEqual([row['status'] for row in rows], ['admissible', 'admissible'])
        self.assertEqual([(row['F'], row['D'], row['delta_phi']) for row in rows], [(0, 0, 0), (0, 0, 0)])
        self.assertEqual(result['final'], {'end_digest': solved, 'peak_left_height': 2, 'excursion': 0,
                                           'tame': True})
        self.assertEqual((result['complete'], result['coverage'], result['chunks']), (True, [0, 2], 2))
        self.assertEqual(engine.record()['digest'], solved, 'the batch reads a snapshot; the state stays at r')
        after_first = None
        for row, (grip, item) in zip(rows, JI16_WORD):
            reference = j1().apply(letter_step(grip, item))
            self.assertEqual((row['after'], row['moved'], row['moved_set']),
                             (reference.digest, reference.moved, reference.moved_set))
            self.assertEqual(row['N_after'], rim.Domains(j1().state).total_misalignment())
            self.assertEqual(row['max_left_height'], max_left_height(obs_refs._tree_summary(j1().state)))
            if after_first is None:
                after_first = WC.copy_state(j1().state)
        self.assertEqual((rows[0]['N_before'], rows[1]['N_after']), (0, 0))
        # Step 5: one exact-pose unit, the pieces item 12 moved, with no preferred target.
        engine.use('prefix/0')
        units = engine.units()['units']
        self.assertEqual(len(units), 1)
        unit = units[0]['members']
        self.assertEqual(H.moved_set_hash(unit), rows[0]['moved_set'])
        self.assertEqual(len(unit), rows[0]['moved'])
        self.assertNotIn('target', units[0])
        # Step 6: the word check of (0,45) on that unit at cap 0 equals J1's word check.
        check = engine.word_check([[0, 45]], unit=unit, cap=0)
        reference = WC.word_check(after_first, [[0, 45]], unit, cap=0)
        for field in dataclass_fields(reference):
            value = getattr(reference, field.name)
            self.assertEqual(check[field.name], list(value) if isinstance(value, tuple) else value, field.name)
        self.assertEqual((check['unit_landed'], check['lattice_pieces_lifted_off'], check['others'],
                          check['step_flag'], check['end_digest']), (True, 0, 0, True, solved))
        # Step 7: the final prefix state has no mismatch.
        engine.use('prefix/1')
        facts = engine.seam_facts('all', chunk=SEAM_CHUNK, budget=SEAM_BUDGET)
        self.assertEqual((facts['totals']['total_N'], facts['totals']['on_grip_mismatches'],
                          facts['totals']['off_grip_mismatches']), (0, 0, 0))
        # Step 8: the derived states are released.
        self.assertEqual([engine.release(f'prefix/{i}') for i in range(2)], [True, True])

    def test_cancelled_and_cut_off_batches_make_no_final_claim(self):
        """A batch cancelled after row 0 covers [0,1): no row 1 and no final solved claim; a budget of one letter cuts
        it off the same way."""
        candidate().start(self.menu)
        cancelled = candidate().prefix_observations(JI16_WORD, chunk=PREFIX_CHUNK, budget=len(JI16_WORD),
                                                    cancel_after=1)
        cut = candidate().prefix_observations(JI16_WORD, chunk=PREFIX_CHUNK, budget=1)
        for result, flag in ((cancelled, 'cancelled'), (cut, 'cut_off')):
            with self.subTest(flag=flag):
                self.assertEqual([row['index'] for row in result['rows']], [0])
                self.assertEqual((result['coverage'], result['complete'], result['final'], result[flag]),
                                 ([0, 1], False, None, True))


if __name__ == '__main__':
    unittest.main()
