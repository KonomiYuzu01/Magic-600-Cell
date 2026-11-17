"""LP2: the shipped correction catalogue and the drafted correction-set check against the research library
(`rust/c600-jumble/src/corrections.rs` behind the adapter's `corrections`, `correction_lookup` and
`correction_set_check`; `research/jumbling/differential/references/corrections.py` and `catalogue.py`).

DT-J1 O6 (b) and O7 (a) (owner, 22 October 2026): the 36 corrections ship as constructions that the solver selects by
id, and the check adds the rows of a set that the solver drafts. Every id below is fixed in the test; nothing here
chooses, ranks or combines corrections.

- The shipped file `rust/c600-jumble/data/corrections-I_a.json` has the bytes that `catalogue.py` writes from the
  pinned library, and the adapter's catalogue equals the library: the binding, the source, the coordinate names and
  each correction's family, letters and row.
- Each correction's word as twists of the I_a menu has the grips and exact matrices of the library's records.
- EH-11: from solved, each of the 36 corrections, played letter by letter as its literal word, is admissible and
  changes ν by exactly its row. From the research lattice input E3 the same holds for the corrections in `E3_IDS`
  (rows of b bits, a frame bit and a C5 value), and with JUMBLING_FULL_REPLAY=1 for all 36.
- `correction_set_check` equals `corrections.drafted_nu` on the caller's ids from solved, on a witnessed state, on a
  stored state with its journal, on a derived value and on E3 as an import with an open claim; a multiplicity m
  counts as m mod 10 copies, since every coordinate lies in F2 or F5. An unknown id, free mode and the I_b menu are
  refused. I_a with its items reordered refuses the words, whose letters number I_a's recorded order, and passes the
  check, which uses rows. Off the lattice the check is undefined. Malformed requests are input errors.

Not run: I1 and I2 as starts (the references ran all three in J1; E3 is EH-11's lattice research input here), and a
witnessed I_a state with a nonzero fixed coordinate, which stage 04's theorems exclude (the unit tests of `nu.rs`
plant one).

The adapter is built with `cargo build --release --locked --offline` in rust/. A missing Rust toolchain or NumPy fails
the check: a test that did not run is no pass (PHS.63). A pass is source and fixture evidence, not Windows/DirectX,
input, long-session or performance evidence.
"""

import sys

sys.dont_write_bytecode = True

import hashlib
import json
from math import gcd
import os
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / 'research' / 'jumbling'
FIXTURE_DIR = JUMBLING / 'fixtures'
MANIFEST = ROOT / 'assets' / 'manifest.json'

FULL = os.environ.get('JUMBLING_FULL_REPLAY') == '1'
COUNT = 36
LETTERS = 23932
NU_LEN = 45
ZERO = (0,) * NU_LEN
# EH-11 at E3: the rows of b bits up to 85 letters (ids 0 to 19), the frame bit f8 (id 20, 128 letters) and the C5
# value C5_17 (id 27, 512 letters); all 36 with JUMBLING_FULL_REPLAY=1.
E3_IDS = tuple(range(COUNT)) if FULL else tuple(range(21)) + (27,)
# Drafted sets as the caller writes them: (id, m) pairs, m any integer, ids repeated in any order.
SETS = (
    (),
    ((0, 1),),
    ((0, 1), (2, 1)),
    ((2, 1), (0, 1)),
    ((3, 1), (3, -1)),
    ((27, 2), (30, -1), (20, 10), (5, -11), (35, 3), (27, 3)),
    tuple((cid, 1) for cid in range(COUNT)),
)
# The pinned I_a fixture is off the lattice at this prefix.
OFF_PREFIX = 8
OPEN = {'kind': 'open', 'evidence': None}
UNKNOWN_IDS = (COUNT, 1000, 2**32 - 1)
NAMED_OBJECT_INVALID = 'KN2:input.named_object_invalid'
MENU_BINDING_MISMATCH = 'KN2:input.menu_binding_mismatch'

_ENGINES = {}


def setUpModule():
    global H, R, C, catalogue, census, library, nu, nu_refs
    try:
        import numpy  # noqa: F401
    except ImportError as error:
        raise RuntimeError('NumPy is required: the correction comparison did not run (PHS.63)') from error
    sys.path.insert(0, str(JUMBLING))
    # The census module first: it moves the research centroid cache out of the repository before any theory import.
    from differential.leads import census
    from differential import harness as H
    from differential.candidate import rust_jumble as R
    from differential.references import catalogue, corrections as C, nu, nu_refs
    from differential.references.session_engine import J1SessionEngine
    if Path(census.seams.CACHE).resolve().is_relative_to(ROOT):
        raise AssertionError('the centroid cache is inside the repository')
    library = C.load_library()
    R.build()
    _ENGINES['candidate'] = R.RustJumbleEngine(step_result=H.StepResult)
    _ENGINES['session'] = J1SessionEngine('I_a')


def tearDownModule():
    engine = _ENGINES.pop('candidate', None)
    if engine is not None:
        engine.close()


def candidate():
    return _ENGINES['candidate']


def fixture(name):
    return json.loads((FIXTURE_DIR / f'wj-{name}.json').read_text(encoding='utf-8'))


def record_step(record):
    return {'kind': 'record', 'record': record}


def matrix_record(record):
    """A J1 journal record as the candidate reads it: a retained record becomes the family matrix, whose exact matrix
    is authoritative in both engines."""
    if record.get('family') != 'retained':
        return record
    return dict(record, family='matrix', params={})


def values(answer):
    """The 45 values of a total answer."""
    if answer.get('result') != 'total':
        raise AssertionError(f'ν is not total: {answer}')
    if len(answer['nu']) != NU_LEN:
        raise AssertionError(f'ν has {len(answer["nu"])} values')
    return tuple(answer['nu'])


def copies(pairs):
    """A drafted set as `drafted_nu` takes it: m copies of each id, m taken modulo 10, since every coordinate lies in
    F2 or F5."""
    return [cid for cid, m in pairs for _ in range(m % 10)]


def exact(number):
    """An exact [a, b, d] number (a + b√5)/d in lowest terms with d > 0."""
    a, b, d = (int(x) for x in number)
    g = gcd(gcd(a, b), d)
    g = -g if d < 0 else g
    return a // g, b // g, d // g


def matrix(rows):
    return tuple(tuple(exact(x) for x in row) for row in rows)


def e3():
    """E3's J1 state, the record the candidate loads and its ν by the reference, read once."""
    if 'e3' not in _ENGINES:
        state, _ = nu_refs.table_state(nu_refs.TABLES['E3'])
        _ENGINES['e3'] = (state, _ENGINES['session'].state_record(state), nu.nu(state))
    return _ENGINES['e3']


class Catalogue(unittest.TestCase):
    """The shipped file and the adapter's catalogue against the pinned library."""

    def test_the_shipped_file_is_the_library_written_out(self):
        shipped, written = catalogue.SHIPPED.read_bytes(), catalogue.catalogue_bytes()
        self.assertTrue(shipped == written,
                        f'{catalogue.SHIPPED.relative_to(ROOT).as_posix()} has SHA-256 '
                        f'{hashlib.sha256(shipped).hexdigest()}, the library written out '
                        f'{hashlib.sha256(written).hexdigest()}; regenerate it with catalogue.py --out')

    def test_the_adapter_catalogue_follows_the_library(self):
        got = candidate().corrections()
        candidate().start(fixture('I_a')['menu'])
        identity = candidate().identity()
        self.assertEqual(identity['menu_order'], census.I_A_ORDER_DIGEST)
        self.assertEqual(got['format'], catalogue.FORMAT)
        self.assertEqual(got['binding'], {
            'model_id': json.loads(MANIFEST.read_bytes())['model_id'],
            'menu_identity': identity['menu_id'],
            'menu_order': identity['menu_order'],
            'contract_revision': candidate().nu_constants()['contract_revision'],
        })
        self.assertEqual(got['source'], {'library': C.LIBRARY.relative_to(ROOT).as_posix(),
                                         'sha256': C.LIBRARY_SHA256})
        self.assertEqual(tuple(got['names']), nu.NAMES)
        self.assertEqual([row['id'] for row in got['corrections']], list(range(COUNT)))
        self.assertEqual(sum(len(row['letters']) for row in got['corrections']), LETTERS)
        for row, correction in zip(got['corrections'], library):
            with self.subTest(correction=correction.id):
                self.assertEqual(row['family'], correction.family)
                self.assertEqual(len(row['letters']), correction.length)
                self.assertEqual([tuple(letter) for letter in row['letters']], list(correction.letters))
                self.assertEqual(tuple(row['row']), correction.row)

    def test_each_word_under_i_a_is_the_library_records(self):
        candidate().start(fixture('I_a')['menu'])
        shipped = candidate().corrections()['corrections']
        for correction in library:
            with self.subTest(correction=correction.id):
                self.assertEqual(candidate().correction_lookup(correction.id),
                                 {'result': 'found', 'correction': shipped[correction.id]})
                answer = candidate().correction_lookup(correction.id, twists=True)
                self.assertEqual(answer['correction'], shipped[correction.id])
                word = answer['word']
                self.assertEqual((word['result'], len(word['twists'])), ('word', correction.length))
                for index, (twist, record) in enumerate(zip(word['twists'], correction.records)):
                    if twist['grip'] != record['grip'] or matrix(twist['matrix']) != matrix(record['matrix']):
                        self.fail(f'letter {index}: grip {twist["grip"]} is not the record grip {record["grip"]}, '
                                  'or the matrices differ')


class EH11(unittest.TestCase):
    """EH-11: each correction, played letter by letter as its literal word, changes ν by exactly its row."""

    def play(self, correction, identity):
        step = candidate().word(identity, correction.letters)
        self.assertEqual((step['status'], step['applied']), ('admissible', correction.length), step)

    def test_from_solved(self):
        menu = fixture('I_a')['menu']
        for correction in library:
            with self.subTest(correction=correction.id):
                candidate().start(menu)
                self.play(correction, candidate().identity())
                self.assertEqual(values(candidate().nu()), correction.row)

    def test_from_the_lattice_input_e3(self):
        state, record, before = e3()
        self.assertNotEqual(before, ZERO, 'a research input with ν = 0 checks no addition')
        candidate().start(fixture('I_a')['menu'])
        self.assertEqual(candidate().load(record).digest, state.digest())
        self.assertEqual(values(candidate().nu()), before)
        identity = candidate().identity()
        candidate().keep('corrections/E3')
        try:
            for cid in E3_IDS:
                correction = library[cid]
                with self.subTest(correction=cid):
                    candidate().use('corrections/E3')
                    self.play(correction, identity)
                    self.assertEqual(values(candidate().nu()), nu.add(before, correction.row))
        finally:
            candidate().release('corrections/E3')


class SetCheck(unittest.TestCase):
    """KN2.4's `correction_set_check`: ν of the base plus the rows of the drafted set, as `drafted_nu` adds them."""

    def assertSets(self, before, **base):
        state = candidate().nu(**base)['state']
        for pairs in SETS:
            with self.subTest(base=sorted(base), pairs=pairs):
                answer = candidate().correction_set_check(pairs, **base)
                expected = C.drafted_nu(before, copies(pairs))
                self.assertEqual((values(answer), answer['state']), (expected, state))
                self.assertEqual(answer['nonzero'], nu.nonzero(expected))

    def test_solved_witnessed_stored_and_derived_bases(self):
        menu = fixture('I_a')['menu']
        candidate().start(menu)
        self.assertSets(ZERO, journal=[])
        candidate().keep('corrections/solved')
        three, four = library[3], library[4]
        records = [matrix_record(record) for record in three.records]
        for record in records:
            self.assertEqual(candidate().apply(record_step(record)).status, 'admissible')
        try:
            self.assertSets(three.row, journal=records)
            candidate().keep('corrections/three')
            candidate().start(menu)
            self.assertSets(three.row, state='corrections/three', journal=records)
            base = {'state': 'corrections/solved', 'journal': []}
            derived = candidate().derive('corrections/four', candidate().identity(), base, letters=four.letters)
            self.assertEqual((derived['result'], derived['reachability']['kind']), ('derived', 'witnessed'))
            self.assertSets(four.row, derived='corrections/four')
        finally:
            for name in ('corrections/solved', 'corrections/three'):
                candidate().release(name)

    def test_e3_as_an_import_with_an_open_claim(self):
        state, record, before = e3()
        candidate().start(fixture('I_a')['menu'])
        imported = candidate().import_table('corrections/E3', record, candidate().identity(), OPEN)
        self.assertEqual(imported['result'], 'derived', imported)
        self.assertSets(before, derived='corrections/E3')
        self.assertEqual(candidate().nu(derived='corrections/E3')['state'], state.digest())


class Refusals(unittest.TestCase):
    """Unknown ids, free mode, other menus, off the lattice, and malformed requests."""

    def assertRefused(self, answer, code):
        self.assertEqual((answer['result'], answer['code']), ('refused', code), answer)
        self.assertNotIn('nu', answer)

    def test_unknown_ids_are_refused(self):
        candidate().start(fixture('I_a')['menu'])
        for cid in UNKNOWN_IDS:
            with self.subTest(id=cid):
                self.assertRefused(candidate().correction_lookup(cid), NAMED_OBJECT_INVALID)
                self.assertRefused(candidate().correction_lookup(cid, twists=True), NAMED_OBJECT_INVALID)
                self.assertRefused(candidate().correction_set_check([(0, 1), (cid, 1)], journal=[]),
                                   NAMED_OBJECT_INVALID)

    def test_free_mode_and_other_menus(self):
        found = candidate().correction_lookup(0)['correction']
        for name, menu in (('free', None), ('I_b', fixture('I_b')['menu'])):
            with self.subTest(menu=name):
                candidate().start(menu)
                answer = candidate().correction_lookup(0, twists=True)
                self.assertEqual((answer['result'], answer['correction']), ('found', found))
                self.assertRefused(answer['word'], MENU_BINDING_MISMATCH)
                self.assertRefused(candidate().correction_set_check([(0, 1)], journal=[]), MENU_BINDING_MISMATCH)
        # I_a's items in reverse order: the same menu by its order-free identity, numbered otherwise.
        menu = fixture('I_a')['menu']
        candidate().start(menu)
        pinned = candidate().identity()
        candidate().start(dict(menu, base=list(reversed(menu['base']))))
        identity = candidate().identity()
        self.assertEqual(identity['menu_id'], pinned['menu_id'])
        self.assertNotEqual(identity['menu_order'], pinned['menu_order'])
        self.assertRefused(candidate().correction_lookup(0, twists=True)['word'], MENU_BINDING_MISMATCH)
        self.assertEqual(values(candidate().correction_set_check([(0, 1)], journal=[])), library[0].row)

    def test_off_the_lattice_the_check_is_undefined(self):
        doc = fixture('I_a')
        records = doc['journal']['records'][:OFF_PREFIX]
        candidate().start(doc['menu'])
        for record in records:
            candidate().apply(record_step(record))
        for pairs in ((), ((0, 1),)):
            with self.subTest(pairs=pairs):
                answer = candidate().correction_set_check(pairs, journal=records)
                self.assertEqual((answer['result'], answer['reason']), ('undefined', 'off_lattice'))
                self.assertNotIn('nu', answer)

    def test_malformed_requests_are_input_errors(self):
        candidate().start(fixture('I_a')['menu'])
        stray = matrix_record(library[0].records[0])
        for op, fields, needle in (
                ('correction_lookup', {}, 'correction id'),
                ('correction_lookup', {'id': -1}, 'correction id'),
                ('correction_lookup', {'id': 2**32}, 'correction id'),
                ('correction_lookup', {'id': '0'}, 'correction id'),
                ('correction_lookup', {'id': 0, 'twists': 'yes'}, 'twists must be'),
                ('correction_set_check', {'journal': []}, 'list of [id, m]'),
                ('correction_set_check', {'corrections': [[0]], 'journal': []}, 'must be [id, m]'),
                ('correction_set_check', {'corrections': [[0, '1']], 'journal': []}, 'multiplicity'),
                ('correction_set_check', {'corrections': [[-1, 1]], 'journal': []}, 'correction id'),
                ('correction_set_check', {'corrections': [[0, 1]]}, 'journal that witnesses'),
                ('correction_set_check', {'corrections': [], 'derived': 'corrections/none', 'journal': []},
                 'names no state and no journal'),
                ('correction_set_check', {'corrections': [], 'derived': 'corrections/none'}, 'no derived value'),
                ('correction_set_check', {'corrections': [], 'state': 'corrections/none', 'journal': []},
                 'no stored state'),
                ('correction_set_check', {'corrections': [], 'journal': [stray]}, 'does not witness'),
        ):
            with self.subTest(op=op, fields=fields):
                with self.assertRaises(R.CandidateError) as error:
                    candidate().call(op, **fields)
                self.assertEqual(error.exception.kind, 'input')
                self.assertIn(needle, error.exception.message)
        self.assertEqual(values(candidate().nu()), ZERO, 'an input error changes nothing')


if __name__ == '__main__':
    unittest.main()
