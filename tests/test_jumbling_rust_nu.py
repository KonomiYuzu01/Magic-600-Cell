"""LP2: ν (A5) over KN3.2's F2 and F5 against the ν reference (`rust/c600-jumble/src/nu.rs` behind the adapter's
`nu` and `nu_constants`; `research/jumbling/differential/references/nu.py`, over restoration stage 03's `nucoords.py`).

EH-10 (ENG-0J acceptance item 8, for the candidate engine): ν equals `nucoords.py` on the research lattice states (the
tables E3, I1 and I2 of restoration stages 01 and 02, with their TABLE-NU rows and restore-summary inputs) and is 0
after retained words from solved. Off the lattice ν is undefined, never zeros or an old value (KN2.4, LB3.1).

The other comparisons: the coordinates, the fixed coordinates under I_a, the C5 convention, the certificate program's
SHA-256, the pinned I_a menu identity and order digest, and each orbit's frame group with its C5 generator, against
the reference; a piece of each orbit with a nontrivial frame group turned in its own chamber by every element of the
chamber's stabiliser, as raw tables both engines read; two corrections of the pinned I_a library, whose ν equals their
rows (EH-11's check for these two; the catalogue and the other 34 come with `correction_set_check`); KN2.4's row on a
witnessed state, a derived value and an import with an open claim; and the adapter's input errors.

Not run: a witnessed I_a value with a nonzero fixed coordinate, which stage 04's theorems exclude, so the refusal
`KN2:consistency.fixed_nu_nonzero` is checked by the unit tests of `nu.rs` only; the language rows (LB3.1, LP3).

The adapter is built with `cargo build --release --locked --offline` in rust/. A missing Rust toolchain or NumPy fails
the check: a test that did not run is no pass (PHS.63). A pass is source and fixture evidence, not Windows/DirectX,
input, long-session or performance evidence.
"""

import sys

sys.dont_write_bytecode = True

import base64
import gzip
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / 'research' / 'jumbling'
FIXTURE_DIR = JUMBLING / 'fixtures'
CENSUS = ROOT / 'assets' / 'census.json'

NU_LEN = 45
ZERO = (0,) * NU_LEN
# Fixed retained words of signed 1-based primitive ids, as in the M8 comparison.
WORDS = (
    [1, 3, -5],
    [2, 2, 2, -7, 600, -1199],
    [(37 * i + 11) % 1200 + 1 if i % 3 else -((37 * i + 11) % 1200 + 1) for i in range(30)],
)
# Library corrections: 0 states its moved counts, 2 does not.
CORRECTIONS = (0, 2)
# The orbits whose frame groups are not trivial: C2 (0), D5 (6), C5 (17) and A5 (34).
TURNED_ORBITS = (0, 6, 17, 34)
# The pinned I_a fixture is off the lattice at this prefix.
OFF_PREFIX = 8
# The Cayley parameter omega = (2, 0, 0) of CT-KN1-06: exact [a, b, d] triples.
OMEGA_2 = [[2, 0, 1], [0, 0, 1], [0, 0, 1]]
OPEN = {'kind': 'open', 'evidence': None}
# The state contract stamps: the engine stamps the pin JI.3 names; the ν reference stamps A1 to A5.
NU_CONTRACT_REVISION = 'state-contract 2026-10-22 A1-A5'
ENGINE_CONTRACT_REVISION = 'state-contract 2026-11-15 A1-A6'

_ENGINES = {}


def setUpModule():
    global np, H, R, J1Engine, J1SessionEngine, nu, nu_refs, nucoords, C, census, sim, NP
    try:
        import numpy as np
    except ImportError as error:
        raise RuntimeError('NumPy is required: the ν comparison did not run (PHS.63)') from error
    sys.path.insert(0, str(JUMBLING))
    # The census module first: it moves the research centroid cache out of the repository before any theory import.
    from differential.leads import census
    from differential import harness as H
    from differential.candidate import rust_jumble as R
    from differential.j1_adapter import J1Engine
    from differential.references import corrections as C, nu, nu_refs
    from differential.references.session_engine import J1SessionEngine
    import nucoords
    import sim
    from sim.model import NP
    if Path(census.seams.CACHE).resolve().is_relative_to(ROOT):
        raise AssertionError('the centroid cache is inside the repository')
    R.build()
    _ENGINES['candidate'] = R.RustJumbleEngine(step_result=H.StepResult)
    _ENGINES['j1'] = J1Engine()
    _ENGINES['session'] = J1SessionEngine('I_a')


def tearDownModule():
    engine = _ENGINES.pop('candidate', None)
    if engine is not None:
        engine.close()


def candidate():
    return _ENGINES['candidate']


def j1():
    return _ENGINES['j1']


def session():
    return _ENGINES['session']


def fixture(name):
    return json.loads((FIXTURE_DIR / f'wj-{name}.json').read_text(encoding='utf-8'))


def record_step(record):
    return {'kind': 'record', 'record': record}


def matrix_record(record):
    """A J1 journal record as the candidate reads it. The candidate reads the families menu, matrix and cayley; a
    retained record becomes the family matrix, whose exact matrix is authoritative in both engines."""
    if record.get('family') != 'retained':
        return record
    return dict(record, family='matrix', params={})


def primitive_records(word):
    return [matrix_record(sim.primitive(session().ctx, mid).record()) for mid in word]


def raw_record(table, ids):
    raw = np.asarray(ids, dtype='<u4').tobytes()
    return {'pose_table': table,
            'pose_ids': {'encoding': 'u32le-base64', 'count': len(ids), 'sha256': hashlib.sha256(raw).hexdigest(),
                         'data': base64.b64encode(raw).decode('ascii')}}


def values(answer):
    """The 45 values of a total answer."""
    if answer.get('result') != 'total':
        raise AssertionError(f'ν is not total: {answer}')
    if len(answer['nu']) != NU_LEN:
        raise AssertionError(f'ν has {len(answer["nu"])} values')
    return tuple(answer['nu'])


class Constants(unittest.TestCase):
    """The coordinates, the pinned convention and digests, and the frame groups against the ν reference."""

    def test_coordinates_and_pins_follow_the_reference(self):
        got = candidate().nu_constants()
        self.assertEqual(tuple(got['names']), nu.NAMES)
        self.assertEqual(tuple(got['moduli']), nu.MODULI)
        self.assertEqual(tuple(got['i_a_fixed']), nu.I_A_FIXED)
        self.assertEqual((got['frame_bit_orbits'], got['c5_orbits']), (nucoords.C2F, nucoords.C5F))
        self.assertEqual(got['c5_convention'], nu.C5_CONVENTION)
        self.assertEqual(got['certificate_program_sha256'], nu.certificate_program_sha256())
        self.assertEqual((got['contract_revision'], nu.CONTRACT_REVISION),
                         (ENGINE_CONTRACT_REVISION, NU_CONTRACT_REVISION))
        candidate().start(fixture('I_a')['menu'])
        identity = candidate().identity()
        self.assertEqual((got['i_a_menu_identity'], got['i_a_order_digest']),
                         (identity['menu_id'], identity['menu_order']))
        self.assertEqual(got['i_a_order_digest'], census.I_A_ORDER_DIGEST)

    def test_frame_groups_follow_nucoords_and_the_census(self):
        coords = nu.coords()
        rows = json.loads(CENSUS.read_text(encoding='utf-8'))['orbits']
        groups = candidate().nu_constants()['groups']
        self.assertEqual([group['orbit'] for group in groups], list(range(35)))
        for group in groups:
            orbit = group['orbit']
            with self.subTest(orbit=orbit):
                self.assertEqual(group['kind'], nucoords.HG.get(orbit, 'trivial'))
                self.assertEqual(group['slots'], coords.k[orbit])
                self.assertEqual(group['order'], rows[orbit]['orientation_order'])
                table, _ = coords.ab[orbit]
                if group['kind'] == 'C5':
                    r0 = next(h for h, i in table.items() if i == 1)
                    self.assertEqual(group['generator'], list(r0))
                else:
                    self.assertIsNone(group['generator'])
                    self.assertEqual(group['order'], 1 if table is None else len(table))


class Values(unittest.TestCase):
    """EH-10: ν is 0 at solved and after retained words, equals nucoords.py on the research lattice states, and is
    undefined off the lattice."""

    @classmethod
    def setUpClass(cls):
        cls.menu = fixture('I_a')['menu']

    def test_solved_and_retained_words_give_zero(self):
        candidate().start(self.menu)
        for journal in (None, []):
            answer = candidate().nu(journal=journal)
            self.assertEqual((values(answer), answer['nonzero']), (ZERO, {}))
        for word in WORDS:
            with self.subTest(word=word[:6]):
                candidate().start(self.menu)
                step = candidate().retained_word(word)
                self.assertEqual((step['status'], step['applied']), ('admissible', len(word)))
                state = session().replay_retained(word)
                self.assertEqual(step['digest'], state.digest())
                self.assertEqual(nu.nu(state), ZERO)
                self.assertEqual(values(candidate().nu()), ZERO)
                row = candidate().nu(journal=primitive_records(word))
                self.assertEqual((values(row), row['state']), (ZERO, state.digest()))

    def test_research_lattice_states_follow_nucoords(self):
        rows = {row.sha256: row.row for row in nu_refs.table_nu_rows()}
        summary = nu_refs.restore_nu_rows()
        for name, path in nu_refs.TABLES.items():
            with self.subTest(table=name):
                state, record = nu_refs.table_state(path)
                reference = nu.nu(state)
                self.assertEqual(reference, rows[hashlib.sha256(gzip.decompress(path.read_bytes())).hexdigest()])
                self.assertEqual(reference, summary[record['digest']])
                nu.check_fixed(reference)
                self.assertNotEqual(reference, ZERO, 'a research input with ν = 0 checks no coordinate')
                candidate().start(self.menu)
                state_record = session().state_record(state)
                self.assertEqual(candidate().load(state_record).digest, state.digest())
                answer = candidate().nu()
                self.assertEqual((values(answer), answer['state']), (reference, state.digest()))
                self.assertEqual(answer['nonzero'], nu.nonzero(reference))
                # As an import with an open claim, KN2.4's row gives the value: A5 bounds no open claim.
                imported = candidate().import_table(f'nu/{name}', state_record, candidate().identity(), OPEN)
                self.assertEqual(imported['result'], 'derived', imported)
                self.assertEqual(values(candidate().nu(derived=f'nu/{name}')), reference)

    def test_off_the_lattice_nu_is_undefined(self):
        doc = fixture('I_a')
        records = doc['journal']['records'][:OFF_PREFIX]
        self.assertEqual(candidate().start(doc['menu']), j1().start(doc['menu']))
        for record in records:
            step = record_step(record)
            self.assertEqual(candidate().apply(step), j1().apply(step))
        state = j1().state
        self.assertGreater(state.off_lattice_count(), 0)
        with self.assertRaises(nu.NuUndefined):
            nu.nu(state)
        for journal in (None, records):
            answer = candidate().nu(journal=journal)
            self.assertEqual((answer['result'], answer['reason'], answer['state']),
                             ('undefined', 'off_lattice', state.digest()))
            self.assertNotIn('nu', answer)
        # The Cayley rotation of CT-KN1-06, in free mode, witnessed by its one-letter journal.
        record = candidate().cayley(0, OMEGA_2)
        candidate().start(None)
        candidate().apply(record_step(record))
        answer = candidate().nu(journal=[record])
        self.assertEqual((answer['result'], answer['reason']), ('undefined', 'off_lattice'))


class TurnedPieces(unittest.TestCase):
    """A piece of each orbit with a nontrivial frame group, turned in its own chamber by each element of the chamber's
    stabiliser in K⁺: raw tables both engines read under I_a, whose ν comes from that piece's frame alone."""

    def test_turned_pieces_follow_nucoords(self):
        ctx = session().ctx
        kp, reg, data = ctx.kplus, ctx.regions, ctx.data
        menu = fixture('I_a')['menu']
        names = list(nu.NAMES)
        for orbit in TURNED_ORBITS:
            piece = int(np.flatnonzero(data.orbit_id == orbit)[0])
            ks = np.arange(len(kp.perms), dtype=np.int64)
            transport = np.full(len(ks), reg.transport[piece], dtype=np.int64)
            chambers = reg.piece_of[reg.orbit_of[piece], kp.compose_vec(ks, transport)]
            stabiliser = [int(k) for k in ks[chambers == piece] if k]
            self.assertTrue(stabiliser, f'orbit {orbit}')
            frame = {f'f{orbit}', f'C5_{orbit}'} & set(names)
            seen = set()
            for k in stabiliser:
                with self.subTest(orbit=orbit, k=k):
                    ids = [0] * NP
                    ids[piece] = 1
                    record = raw_record([{'k': 0}, {'k': k}], ids)
                    candidate().start(menu)
                    loaded = candidate().load(record)
                    state = session().state_from_record(dict(record, digest=loaded.digest))
                    reference = nu.nu(state)
                    self.assertEqual(values(candidate().nu()), reference)
                    # Only the orbit's frame value can change: its piece stays in its chamber.
                    self.assertLessEqual(set(nu.nonzero(reference)), frame)
                    seen.add(tuple(reference[names.index(n)] for n in sorted(frame)))
            # The frame values the stabiliser reaches: C2 and D5 give 1 for a flip, C5 every value modulo 5 with the
            # identity's 0, and A5, which is perfect, gives nothing.
            expected = {0: {(1,)}, 6: {(0,), (1,)}, 17: {(1,), (2,), (3,), (4,)}, 34: {()}}[orbit]
            self.assertLessEqual(expected, seen, f'orbit {orbit}')


class Row(unittest.TestCase):
    """KN2.4's row: two corrections of the pinned I_a library from solved give their rows as the bare value, under
    their witnessing journals and as derived values, and J1's ν agrees; malformed requests are input errors."""

    def test_corrections_give_their_rows(self):
        library = C.load_library()
        menu = fixture('I_a')['menu']
        for cid in CORRECTIONS:
            correction = library[cid]
            with self.subTest(correction=cid):
                records = [matrix_record(record) for record in correction.records]
                self.assertEqual(candidate().start(menu), j1().start(menu))
                candidate().keep('nu/solved')
                for original, converted in zip(correction.records, records):
                    self.assertEqual(candidate().apply(record_step(converted)), j1().apply(record_step(original)))
                state = j1().state
                row = tuple(int(x) for x in correction.row)
                self.assertEqual(nu.nu(state), row)
                nu.check_fixed(row)
                self.assertNotEqual(row, ZERO)
                for journal in (None, records):
                    answer = candidate().nu(journal=journal)
                    self.assertEqual((values(answer), answer['state']), (row, state.digest()))
                    self.assertEqual(answer['nonzero'], nu.nonzero(row))
                name = f'nu/correction-{cid}'
                base = {'state': 'nu/solved', 'journal': []}
                derived = candidate().derive(name, candidate().identity(), base, records=records)
                self.assertEqual((derived['result'], derived['reachability']['kind']), ('derived', 'witnessed'))
                answer = candidate().nu(derived=name)
                self.assertEqual((values(answer), answer['state']), (row, state.digest()))
                candidate().release('nu/solved')

    def test_malformed_requests_are_input_errors(self):
        candidate().start(fixture('I_a')['menu'])
        for fields, needle in (
                ({'journal': 'x'}, 'journal'),
                ({'derived': 'nu/none'}, 'no derived value'),
                ({'state': 'nu/none'}, 'no stored state'),
                ({'derived': 'nu/none', 'journal': []}, 'names no state and no journal'),
                ({'journal': primitive_records([1])}, 'does not witness'),
        ):
            with self.subTest(fields=fields):
                with self.assertRaises(R.CandidateError) as error:
                    candidate().call('nu', **fields)
                self.assertEqual(error.exception.kind, 'input')
                self.assertIn(needle, error.exception.message)
        self.assertEqual(values(candidate().nu()), ZERO, 'an input error changes nothing')


if __name__ == '__main__':
    unittest.main()
