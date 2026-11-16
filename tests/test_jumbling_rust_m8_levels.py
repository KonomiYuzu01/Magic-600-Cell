"""LP2, M8: the jumbled puzzle's level crossings, end tests, shapes and gauges against J1
(`rust/c600-jumble/src/levels.rs` behind the adapter's `project`, `sticker_test`, `retained_checkpoint`,
`label_checkpoint`, `retained_word`, `gauge` and `gauge_between`).

CT-PS1-14 (part: a retained checkpoint's own path, the supplied witness word or the journal's reduction, replays to its
pose digest as J1's `State.checkpoint` replays it; the certificate of a label checkpoint under the journal's witnessed
claim gives the pose digest of J1's `label_certificate` and `Digest<labels>` of J1's labels and of 600-cell-Full's label
permutation (`assets/primitives.npz`); retained words resumed from a kept checkpoint reproduce both digests; not run:
the session store's checkpoint records and their resume through the store (M6), and a certificate whose ν the ν row
computes).

CT-KN2-06 (whole: Theorem 1 of KN1.14 as a conformance test of the candidate against J1 on J1's fixtures, with the
gauges S, R and all (σ ≠ id) built from J1's regions, on the lattice at solved and at the end of a correction of the
pinned I_a library, and off it at prefix 8 of every fixture: J1's image X·γ, the surveys, each letter's status and
moved set, w(Z) = w(X)·γ in both engines, and the level each class keeps; Note N1 proves the statement, and this
checks the engine against it).

CT-KN1-06 (part: the fixture clause at prefixes 0, 1, 2, 5, 10, 20, 21 and 22 of every fixture: the off count read
from the regions and from the poses is J1's, equal shapes have equal off counts, and the equal shapes of I_a prefixes 21
and 22 come with a gauge with σ ≠ id; not run: the fixtures' other prefixes, and the raw tables (R, R, QR) and
(R, QR, QR), which rust/c600-jumble/tests/levels.rs runs).

The other comparisons: KN1.9's projections at fixture prefixes and after the Cayley rotation of CT-KN1-06 against J1's
off count, lattice flags, chambers and labels (the J1 check of CT-KN1-03, whose run is `rust/c600-jumble/tests/
levels.rs`); the retained checkpoint against `State.checkpoint` at every compared prefix; the label checkpoint of two
library corrections whose ν is not 0; a displaced centre and a turned centre (KN1.17) as raw tables both engines read;
and the adapter's refusals and input errors. Every word here is fixed in the test or read from a fixture; nothing
chooses a twist.

The adapter is built with `cargo build --release --locked --offline` in rust/. A missing Rust toolchain or NumPy fails
the check: a test that did not run is no pass (PHS.63). A pass is source and fixture evidence, not Windows/DirectX,
input, long-session or performance evidence.
"""

import sys

sys.dont_write_bytecode = True

import base64
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / 'research' / 'jumbling'
FIXTURE_DIR = JUMBLING / 'fixtures'

FIXTURES = ('S4', 'I_a', 'I_b')
PREFIX = 40
PROJECTED = {0, 1, 2, 5, 10, 20, 40}
BLOCKING_PREFIX = 30
LEVEL_NOT_HELD = 'KN1:input.level_not_held'
WORD_BLOCKED_PREFIX = 'KN2:input.word_blocked_prefix'
NU_LEN = 45
# The Cayley parameter omega = (2, 0, 0) of CT-KN1-06: exact [a, b, d] triples.
OMEGA_2 = [[2, 0, 1], [0, 0, 1], [0, 0, 1]]
# Fixed retained words of signed 1-based primitive ids, and a fixed continuation from a kept checkpoint.
WORDS = (
    [1, 3, -5],
    [2, 2, 2, -7, 600, -1199],
    [(37 * i + 11) % 1200 + 1 if i % 3 else -((37 * i + 11) % 1200 + 1) for i in range(30)],
)
RESUMED = [9, -9, 1024, 77, -400]
# Library corrections: 0 states its moved counts, 2 does not.
CORRECTIONS = (0, 2)

_ENGINES = {}


def setUpModule():
    global np, H, R, J1Engine, J1SessionEngine, nu, C, sim, api, NP
    try:
        import numpy as np
    except ImportError as error:
        raise RuntimeError('NumPy is required: the M8 level comparison did not run (PHS.63)') from error
    sys.path.insert(0, str(JUMBLING))
    # The census module first: it moves the research centroid cache out of the repository before any theory import.
    from differential.leads import census
    from differential import harness as H
    from differential.candidate import rust_jumble as R
    from differential.j1_adapter import J1Engine
    from differential.references import corrections as C, nu
    from differential.references.session_engine import J1SessionEngine
    import sim
    from sim import api
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


def int32(text):
    return np.frombuffer(base64.b64decode(text), dtype='<i4').astype(np.int64)


def uint32(text):
    return np.frombuffer(base64.b64decode(text), dtype='<u4').astype(np.int64)


def sha256_of(values, dtype):
    return hashlib.sha256(np.asarray(values).astype(dtype).tobytes()).hexdigest()


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


def j1_verdict(state, checkpoint):
    """J1's `State.checkpoint` as a verdict: yes with the source and word, no with the fact, or unknown."""
    if checkpoint['checkpoint']:
        return 'yes', checkpoint['source'], list(checkpoint['witness_word'])
    if not checkpoint['lattice']:
        return 'no', 'off_lattice', checkpoint['off_lattice']
    if not nu.centres_home(state):
        return 'no', 'centre_displaced', None
    return 'unknown', None, None


def candidate_verdict(answer):
    evidence = answer.get('evidence', {})
    if answer['verdict'] == 'yes':
        return 'yes', evidence['source'], evidence['witness_word']
    if answer['verdict'] == 'no':
        return 'no', evidence['fact'], evidence.get('off')
    return 'unknown', None, None


def j1_chambers(state):
    """Each lattice piece of a J1 state with the chamber it occupies, as `State.lattice_stickers` computes it."""
    k = api.piece_kplus(state)
    pieces = np.flatnonzero(k >= 0)
    reg, kp = state.ctx.regions, state.ctx.kplus
    return pieces, reg.piece_of[reg.orbit_of[pieces], kp.compose_vec(k[pieces], reg.transport[pieces])]


class RetainedCheckpoints(unittest.TestCase):
    """CT-PS1-14's engine half on fixed retained words from solved under the pinned I_a menu."""

    @classmethod
    def setUpClass(cls):
        cls.menu = fixture('I_a')['menu']

    def candidate_labels(self):
        answer = candidate().project('labels', values=True)
        self.assertEqual(answer['result'], 'total')
        return int32(answer['value']['labels']), answer['value']['digest']

    def check_digests(self, word, labels, labels_digest, pose_digest):
        """The candidate's pose digest, labels and `Digest<labels>` against J1's replay and Full's permutation."""
        state = session().replay_retained(word)
        self.assertEqual(pose_digest, state.digest())
        reference = nu.labels(state)
        np.testing.assert_array_equal(labels, reference)
        np.testing.assert_array_equal(labels, session().apply_generators(session().solved_labels(), word))
        self.assertEqual(labels_digest, sha256_of(reference, '<i4'))
        return state

    def test_retained_words_reproduce_poses_and_labels(self):
        for word in WORDS:
            with self.subTest(word=word[:6]):
                candidate().start(self.menu)
                step = candidate().retained_word(word)
                self.assertEqual((step['status'], step['applied'], step['refused_at']), ('admissible', len(word), None))
                labels, digest = self.candidate_labels()
                state = self.check_digests(word, labels, digest, step['digest'])
                # The supplied witness and the journal's reduction replay to the pose digest, as J1's checkpoint does.
                supplied = candidate().retained_checkpoint(witness=word)
                self.assertEqual(candidate_verdict(supplied), ('yes', 'supplied', word))
                self.assertEqual(supplied['evidence']['pose_digest'], state.digest())
                self.assertEqual(j1_verdict(state, state.checkpoint(witness=word)), ('yes', 'supplied', word))
                reduced = candidate().retained_checkpoint(journal=primitive_records(word))
                self.assertEqual(candidate_verdict(reduced), j1_verdict(state, state.checkpoint()))
                self.assertEqual(reduced['verdict'], 'yes')
                # A5's certificate under the journal's witnessed claim, against J1's label certificate.
                answer = candidate().label_checkpoint({'kind': 'witnessed', 'journal': primitive_records(word)})
                self.assertEqual(answer['verdict'], 'yes', answer)
                certificate = session().label_certificate(state)
                evidence = answer['evidence']
                self.assertEqual(evidence['pose_digest'], certificate['pose_digest'])
                self.assertEqual(evidence['labels_digest'], digest)
                self.assertEqual(certificate['labels_digest'], nu.labels_digest(labels))
                self.assertEqual(evidence['nu'], list(certificate['nu']))
                self.assertEqual((evidence['nu_source'], evidence['contract_revision']),
                                 ('retained_witness', certificate['contract_revision']))
                self.assertEqual((evidence['reachability']['kind'], evidence['reachability']['retained']),
                                 ('witnessed', True))
                # Open and promised claims answer unknown at ν = 0 (ONE.28).
                for kind in ('open', 'promised'):
                    claim = {'kind': kind, 'evidence': None}
                    self.assertEqual(candidate().label_checkpoint(claim, nu=[0] * NU_LEN)['verdict'], 'unknown')

    def test_resumed_retained_moves_reproduce_both_digests(self):
        word = WORDS[1]
        candidate().start(self.menu)
        candidate().retained_word(word)
        self.assertEqual(candidate().retained_checkpoint(witness=word)['verdict'], 'yes')
        candidate().keep('m8/checkpoint')
        try:
            kept = candidate().project('labels', state='m8/checkpoint')
            candidate().start(self.menu)
            self.assertEqual(candidate().project('labels', state='m8/checkpoint'), kept)
            candidate().use('m8/checkpoint')
            step = candidate().retained_word(RESUMED)
            labels, digest = self.candidate_labels()
            state = self.check_digests(word + RESUMED, labels, digest, step['digest'])
            answer = candidate().retained_checkpoint(witness=word + RESUMED)
            self.assertEqual(candidate_verdict(answer), ('yes', 'supplied', word + RESUMED))
            self.assertEqual(answer['evidence']['pose_digest'], state.digest())
        finally:
            candidate().release('m8/checkpoint')


class Projections(unittest.TestCase):
    """KN1.9's projections and the retained checkpoint against J1 at fixture prefixes and off the lattice."""

    def check_projections(self, state, where):
        off = state.off_lattice_count()
        position = candidate().project('position', values=True)
        labels = candidate().project('labels', values=off == 0)
        shape = candidate().project('shape')
        pieces, chambers = j1_chambers(state)
        np.testing.assert_array_equal(uint32(position['value']['chambers']), chambers, where)
        if off == 0:
            self.assertEqual((position['result'], labels['result']), ('total', 'total'), where)
            np.testing.assert_array_equal(int32(labels['value']['labels']), nu.labels(state), where)
        else:
            self.assertEqual((position['result'], labels['result']), ('partial', 'partial'), where)
            self.assertEqual(position['missing'], {'off': off, 'uncovered': off, 'collisions': 0}, where)
            self.assertEqual((labels['missing']['off'], labels['missing']['collisions']), (off, 0), where)
            np.testing.assert_array_equal(uint32(position['value']['piece_ids']), pieces, where)
        self.assertEqual((shape['result'], shape['value']['off'], shape['value']['off_poses']), ('total', off, off),
                         where)

    def test_fixture_prefixes_follow_j1(self):
        seen = set()
        for name in FIXTURES:
            doc = fixture(name)
            records = doc['journal']['records'][:PREFIX]
            with self.subTest(fixture=name):
                self.assertEqual(candidate().start(doc['menu']), j1().start(doc['menu']))
                for n in range(len(records) + 1):
                    if n:
                        step = record_step(records[n - 1])
                        self.assertEqual(candidate().apply(step), j1().apply(step), f'{name} step {n}')
                    state = j1().state
                    got = candidate_verdict(candidate().retained_checkpoint(journal=records[:n]))
                    self.assertEqual(got, j1_verdict(state, state.checkpoint()), f'{name} prefix {n}')
                    seen.add(got[:2])
                    if n in PROJECTED:
                        self.check_projections(state, f'{name} prefix {n}')
        # The prefixes reach both answers J1 gives here: yes at solved, and no off the lattice.
        self.assertLessEqual({('yes', 'journal'), ('no', 'off_lattice')}, seen)

    def test_the_cayley_rotation_of_ct_kn1_06(self):
        record = candidate().cayley(0, OMEGA_2)
        candidate().start(None)
        j1().start(None)
        self.assertEqual(candidate().apply(record_step(record)), j1().apply(record_step(record)))
        self.assertGreater(j1().state.off_lattice_count(), 0)
        self.check_projections(j1().state, 'the Cayley rotation at grip 0')
        for answer in (candidate().sticker_test(), candidate().retained_checkpoint(witness=[]),
                       candidate().label_checkpoint({'kind': 'open', 'evidence': None})):
            self.assertEqual(candidate_verdict(answer), ('no', 'off_lattice', j1().state.off_lattice_count()))


class Corrections(unittest.TestCase):
    """Two corrections of the pinned I_a library from solved end on the lattice with every centre home and ν ≠ 0: J1's
    label certificate refuses with nu_nonzero, and the candidate's label checkpoint answers no under ν from J1, unknown
    without ν, whatever the claim."""

    def test_label_checkpoint_of_a_correction(self):
        library = C.load_library()
        menu = fixture('I_a')['menu']
        for cid in CORRECTIONS:
            correction = library[cid]
            with self.subTest(correction=cid):
                records = [matrix_record(record) for record in correction.records]
                self.assertEqual(candidate().start(menu), j1().start(menu))
                for original, converted in zip(correction.records, records):
                    self.assertEqual(candidate().apply(record_step(converted)), j1().apply(record_step(original)))
                state = j1().state
                self.assertTrue(state.is_lattice() and nu.centres_home(state))
                value = [int(x) for x in nu.nu(state)]
                self.assertEqual(tuple(value), tuple(correction.row))
                with self.assertRaises(nu.LabelCheckpointRefused) as refused:
                    session().label_certificate(state)
                self.assertEqual(refused.exception.fact, 'nu_nonzero')
                witnessed = {'kind': 'witnessed', 'journal': records}
                answer = candidate().label_checkpoint(witnessed, nu=value)
                self.assertEqual((answer['verdict'], answer['evidence']['fact']), ('no', refused.exception.fact))
                self.assertEqual((answer['evidence']['nu'], answer['evidence']['nu_source']), (value, 'supplied'))
                self.assertEqual(candidate().label_checkpoint(witnessed)['verdict'], 'unknown')
                for kind in ('open', 'promised'):
                    answer = candidate().label_checkpoint({'kind': kind, 'evidence': None}, nu=value)
                    self.assertEqual((answer['verdict'], answer['evidence']['fact']), ('no', 'nu_nonzero'))
                sticker = candidate().sticker_test()
                self.assertEqual((sticker['verdict'], sticker['evidence']['fact']), ('no', 'labels_differ'))
                labels = candidate().project('labels', values=True)
                np.testing.assert_array_equal(int32(labels['value']['labels']), nu.labels(state))
                got = candidate_verdict(candidate().retained_checkpoint(journal=records))
                self.assertEqual(got, j1_verdict(state, state.checkpoint()))


class RawTables(unittest.TestCase):
    """Raw tables both engines read (form and identity only, KN2.14): a rotation of the whole puzzle that displaces
    centres, and a centre turned by a third-turn of A4₀, which passes the sticker test and is no checkpoint (KN1.17)."""

    @classmethod
    def setUpClass(cls):
        cls.menu = fixture('I_a')['menu']

    def load_both(self, table, ids):
        record = raw_record(table, ids)
        candidate().start(self.menu)
        loaded = candidate().load(record)
        return session().state_from_record(dict(record, digest=loaded.digest))

    def test_a_displaced_centre(self):
        k = int(sim.primitive(session().ctx, 1).kidx)
        state = self.load_both([{'k': 0}, {'k': k}], [1] * NP)
        self.assertTrue(state.is_lattice())
        self.assertFalse(nu.centres_home(state))
        got = candidate_verdict(candidate().retained_checkpoint(witness=[1]))
        self.assertEqual(got, j1_verdict(state, state.checkpoint(witness=[1])))
        self.assertEqual(got[:2], ('no', 'centre_displaced'))
        with self.assertRaises(nu.LabelCheckpointRefused) as refused:
            session().label_certificate(state)
        answer = candidate().label_checkpoint({'kind': 'open', 'evidence': None})
        self.assertEqual((answer['verdict'], answer['evidence']['fact']), ('no', refused.exception.fact))
        labels = candidate().project('labels', values=True)
        np.testing.assert_array_equal(int32(labels['value']['labels']), nu.labels(state))
        self.assertEqual(candidate().sticker_test()['evidence']['fact'], 'labels_differ')

    def test_a_turned_centre(self):
        ctx = session().ctx
        kp, data = ctx.kplus, ctx.data
        third = next(k for k in kp.stab0 if k and kp.compose(k, kp.compose(k, k)) == 0)
        centre = int(data.centre_pieces[list(int(p) for p in data.centre_poles).index(0)])
        ids = [0] * NP
        ids[centre] = 1
        state = self.load_both([{'k': 0}, {'k': int(third)}], ids)
        self.assertNotEqual(state.digest(), session().solved().digest())
        np.testing.assert_array_equal(nu.labels(state), session().solved_labels())
        sticker = candidate().sticker_test()
        self.assertEqual(sticker['verdict'], 'yes', sticker)
        # J1's certificate leaves reachability to its caller; its digests are the sticker test's evidence.
        certificate = session().label_certificate(state)
        self.assertEqual(sticker['evidence']['pose_digest'], certificate['pose_digest'])
        self.assertEqual(sticker['evidence']['labels_digest'], sha256_of(nu.labels(state), '<i4'))
        got = candidate_verdict(candidate().retained_checkpoint(journal=[]))
        self.assertEqual(got, j1_verdict(state, state.checkpoint()))
        self.assertEqual(got[0], 'unknown')
        for kind in ('open', 'promised', 'by_theorem', 'refuted'):
            answer = candidate().label_checkpoint({'kind': kind, 'evidence': None})
            self.assertEqual(answer['verdict'], 'unknown', kind)
        self.assertEqual(candidate().label_checkpoint({'kind': 'witnessed', 'journal': []})['verdict'], 'unknown')


class Refusals(unittest.TestCase):
    """The sticker class is not held; a retained word is refused whole at its first refused letter, with nothing
    changed, where J1 refuses that letter; a malformed request is an input error and changes nothing."""

    def test_the_sticker_class_is_not_held(self):
        candidate().start(None)
        answer = candidate().project('sticker_class')
        self.assertEqual((answer['result'], answer['code']), ('refused', LEVEL_NOT_HELD))
        self.assertNotIn('value', answer)

    def test_a_retained_word_is_refused_whole_at_a_blocked_letter(self):
        # The letters are fixed by the survey: the first admissible grip, then the first grip blocked after it.
        doc = fixture('I_a')
        ctx = session().ctx
        self.assertEqual(candidate().start(doc['menu']), j1().start(doc['menu']))
        for record in doc['journal']['records'][:BLOCKING_PREFIX]:
            self.assertEqual(candidate().apply(record_step(record)), j1().apply(record_step(record)))
        before = candidate().record()
        letters = j1().survey(range(600))
        self.assertEqual(candidate().survey(range(600)), letters)
        first = 2 * next(grip for grip in range(600) if letters[grip] == 'a') + 1
        self.assertEqual(j1().apply(record_step(sim.primitive(ctx, first).record())).status, 'admissible')
        letters = j1().survey(range(600))
        second = 2 * next(grip for grip in range(600) if letters[grip] == 'b') + 1
        refused = j1().apply(record_step(sim.primitive(ctx, second).record()))
        self.assertEqual(refused.status, 'blocked')
        answer = candidate().retained_word([first, second])
        self.assertEqual((answer['status'], answer['code'], answer['applied'], answer['refused_at']),
                         (refused.status, WORD_BLOCKED_PREFIX, 0, 1))
        self.assertEqual(answer['digest'], before['digest'])
        self.assertEqual(candidate().record(), before)

    def test_malformed_requests_are_input_errors(self):
        candidate().start(None)
        retained_record = sim.primitive(session().ctx, 1).record()
        bad = (
            ('project', {'level': 'chamber'}),
            ('project', {'level': 'labels', 'values': 1}),
            ('project', {'level': 'labels', 'state': 'no such state'}),
            ('sticker_test', {'state': 'no such state'}),
            ('label_checkpoint', {'reachability': {'kind': 'witnessed'}}),
            ('label_checkpoint', {'reachability': {'kind': 'open', 'journal': []}}),
            ('label_checkpoint', {'reachability': {'kind': 'maybe'}}),
            ('label_checkpoint', {'reachability': {'kind': 'open'}, 'nu': [2] * NU_LEN}),
            ('label_checkpoint', {'reachability': {'kind': 'open'}, 'nu': [0] * (NU_LEN - 1)}),
            ('retained_checkpoint', {}),
            ('retained_checkpoint', {'witness': [1], 'journal': []}),
            ('retained_checkpoint', {'witness': [0]}),
            ('retained_checkpoint', {'journal': [retained_record]}),
            ('retained_word', {'word': [1201]}),
            ('retained_word', {'word': [0]}),
        )
        for op, fields in bad:
            with self.subTest(op=op, fields=fields):
                before = candidate().record()
                with self.assertRaises(R.CandidateError) as raised:
                    candidate().call(op, **fields)
                self.assertEqual(raised.exception.kind, 'input')
                self.assertEqual(candidate().record(), before)


# ------------------------------------------------------------------------------------------------ shapes and gauges

SHAPE_PREFIX = 22
SHAPED = (0, 1, 2, 5, 10, 20, 21, 22)
GAUGE_PREFIX = 8
# A fixed word of signed 1-based primitive ids: id 3 is blocked at I_a prefix 8 and id 1 admissible there.
GAUGE_WORD = [3, 1, 7, 77]
SURVEYED = tuple(range(0, 600, 15))
LETTER = {'admissible': 'a', 'blocked': 'b', 'uncertain': 'u'}


def j1_stabiliser_picks(ctx):
    """Per orbit, from J1's regions: R of KN1.7 at the representative, the K⁺ elements that keep its home region, and
    S, those of R that keep the cell of each of its stickers; returns the first element of S other than id and the
    first element of R outside S, or None."""
    reg, kp = ctx.regions, ctx.kplus
    slot_piece = np.asarray(ctx.data.slot_piece)
    cell_slots = slot_piece.size // 600
    perms = np.asarray(kp.perms)
    picks = []
    for orbit, rep in enumerate(reg.reps):
        region = [int(k) for k in np.flatnonzero(reg.piece_of[orbit] == rep)]
        cells = np.flatnonzero(slot_piece == rep) // cell_slots
        sticker = [k for k in region if np.array_equal(perms[k][cells], cells)]
        picks.append((next((k for k in sticker if k), None), next((k for k in region if k not in sticker), None)))
    return picks


def j1_gauges(ctx):
    """(sigma, t) of the gauges S, R and all, built from J1's regions as the Rust test builds them: S and R turn every
    piece of an orbit by the orbit's pick carried by the piece's transport; all is the relabel by primitive 1's K⁺
    element g, with σA = g(A) and t_A = g."""
    reg, kp = ctx.regions, ctx.kplus
    orbit = np.asarray(reg.orbit_of, np.int64)
    transport = np.asarray(reg.transport, np.int64)
    inverse = np.asarray(kp.inv, np.int64)
    picks = j1_stabiliser_picks(ctx)
    gauges = {}
    for name, which in (('S', 0), ('R', 1)):
        chosen = np.array([-1 if pick[which] is None else pick[which] for pick in picks], np.int64)[orbit]
        has = chosen >= 0
        t = np.zeros(NP, np.int64)
        t[has] = kp.compose_vec(transport[has], kp.compose_vec(chosen[has], inverse[transport[has]]))
        gauges[name] = (np.arange(NP, dtype=np.int64), t)
    g = int(sim.primitive(ctx, 1).kidx)
    sigma = np.asarray(reg.piece_of[orbit, kp.compose_vec(np.full(NP, g), transport)], np.int64)
    gauges['all'] = (sigma, np.full(NP, g, np.int64))
    return gauges


def j1_act(state, sigma, t):
    """X·γ of KN1.7 on a J1 state, as a raw pose table J1 reads: piece A takes the pose X[σA]·t_A, an exact product,
    and the image keeps the menu."""
    from exact import matmul
    from sim.kplus import to_tuple
    kp = state.ctx.kplus
    order = len(kp.perms)
    entries = api.pose_entries(state)
    pairs = np.asarray(state.pose_id, np.int64)[sigma] * order + t
    unique, inverse = np.unique(pairs, return_inverse=True)
    table = []
    for pair in unique.tolist():
        pid, k = divmod(pair, order)
        entry = entries[pid]
        if entry.kidx >= 0:
            table.append((kp.compose(int(entry.kidx), k), None))
        else:
            table.append((-1, to_tuple(matmul(entry.matrix, kp.matrix(k)))))
    return api.from_pose_table(state.ctx, table, inverse, state.menu)


class Shapes(unittest.TestCase):
    """CT-KN1-06's fixture clause: at the shaped prefixes of every fixture the candidate's shape counts J1's off count
    twice, from the regions and from the poses, with no collision; equal shapes never come with unequal off counts; and
    equal shapes of two different configurations, at I_a prefixes 21 and 22, come with a gauge between them (KN1.7)."""

    def related(self, name, earlier, n):
        """The gauge from the current state, prefix n, to the stored prefix `earlier` of equal shape, applied."""
        between = candidate().gauge_between(f'm8/shape/{earlier}')
        self.assertTrue(between['shapes_equal'], f'{name} prefixes {earlier} and {n}')
        self.assertIsNotNone(between['gauge'], f'{name} prefixes {earlier} and {n}')
        candidate().keep('m8/shape/current')
        image = candidate().gauge(uint32(between['gauge']['sigma']), uint32(between['gauge']['t']))
        self.assertEqual(image['digest'], between['other'])
        candidate().use('m8/shape/current')
        return name, earlier, n, between['gauge']['class'], between['state'] != between['other']

    def test_equal_shapes_have_equal_off_counts(self):
        off_by_shape, pairs = {}, []
        for name in FIXTURES:
            doc = fixture(name)
            records = doc['journal']['records'][:SHAPE_PREFIX]
            first = {}
            try:
                self.assertEqual(candidate().start(doc['menu']), j1().start(doc['menu']))
                for n in range(len(records) + 1):
                    if n:
                        step = record_step(records[n - 1])
                        self.assertEqual(candidate().apply(step), j1().apply(step), f'{name} step {n}')
                    if n not in SHAPED:
                        continue
                    shape = candidate().project('shape')['value']
                    off = j1().state.off_lattice_count()
                    self.assertEqual((shape['off'], shape['off_poses'], shape['collisions']), (off, off, 0),
                                     f'{name} prefix {n}')
                    self.assertEqual(off_by_shape.setdefault(shape['digest'], off), off, f'{name} prefix {n}')
                    if shape['digest'] in first:
                        pairs.append(self.related(name, first[shape['digest']], n))
                    else:
                        first[shape['digest']] = n
                        candidate().keep(f'm8/shape/{n}')
            finally:
                for n in first.values():
                    candidate().release(f'm8/shape/{n}')
                candidate().release('m8/shape/current')
        self.assertIn(('I_a', 21, 22, 'all', True), pairs)


class Gauges(unittest.TestCase):
    """CT-KN2-06: Theorem 1 of KN1.14 on the candidate, against J1, for the gauges S, R and all of `j1_gauges`, on the
    lattice at solved under the I_a menu and at the end of a correction of the pinned I_a library, and off it at prefix
    8 of every fixture. For Z = X·γ: the candidate's Z is J1's; every grip has one status from X and from Z (a sample
    of grips in J1); each letter of a fixed word has one status from both, piece A moves from Z exactly when σA moves
    from X, and w(Z) = w(X)·γ in both engines. The class's level is kept, labels under S, posed regions under R and the
    shape under all, and the candidate finds γ between X and Z."""

    @classmethod
    def setUpClass(cls):
        cls.gauges = j1_gauges(session().ctx)

    def reach(self, menu, records, converted):
        """Both engines from solved under `menu` through the records; the candidate keeps its state as m8/x."""
        self.assertEqual(candidate().start(menu), j1().start(menu))
        for original, record in zip(records, converted):
            self.assertEqual(candidate().apply(record_step(record)), j1().apply(record_step(original)))
        candidate().keep('m8/x')
        return api.copy_state(j1().state)

    def run_word(self, name, state):
        """GAUGE_WORD letter by letter from the stored candidate state `name` and from a copy of the J1 state: per letter
        the two statuses, the candidate's moved set and J1's inside set; then the candidate's final digest and J1's
        final state."""
        candidate().use(name)
        trial = api.copy_state(state)
        rows, digest = [], None
        for mid in GAUGE_WORD:
            answer = candidate().retained_word([mid])
            outcome, inside = api.apply_with_inside(trial, sim.primitive(trial.ctx, mid))
            rows.append((answer['status'], outcome.status, answer['moved_set'], np.sort(np.asarray(inside, np.int64))))
            digest = answer['digest']
        return rows, digest, trial

    def project(self, level, name):
        return candidate().project(level, state=name)['value']['digest']

    def test_a_gauge_transfers_admissibility_moves_and_images(self):
        correction = C.load_library()[CORRECTIONS[0]]
        menu = fixture('I_a')['menu']
        bases = [('solved under the I_a menu', menu, [], [], True),
                 (f'correction {CORRECTIONS[0]} of the I_a library', menu, correction.records,
                  [matrix_record(r) for r in correction.records], True)]
        for name in FIXTURES:
            doc = fixture(name)
            prefix = doc['journal']['records'][:GAUGE_PREFIX]
            bases.append((f'{name} prefix {GAUGE_PREFIX}', doc['menu'], prefix, prefix, False))
        keeps = {'S': {'labels', 'region_class', 'shape'}, 'R': {'region_class', 'shape'}, 'all': {'shape'}}
        statuses = set()
        try:
            for where, menu, records, converted, lattice in bases:
                x = self.reach(menu, records, converted)
                off = x.off_lattice_count()
                self.assertEqual(off == 0, lattice, where)
                letters = candidate().survey(range(600))
                j1_letters = {g: LETTER[row['status']] for g, row in x.survey(SURVEYED).items()}
                self.assertEqual({g: letters[g] for g in SURVEYED}, j1_letters, where)
                x_rows, wx_digest, wx = self.run_word('m8/x', x)
                self.assertEqual(wx_digest, wx.digest(), where)
                candidate().keep('m8/wx')
                statuses.update(row[1] for row in x_rows)
                for name, (sigma, t) in self.gauges.items():
                    with self.subTest(where=where, gauge=name):
                        z = j1_act(x, sigma, t)
                        answer = candidate().gauge(sigma, t, state='m8/x')
                        self.assertEqual((answer['digest'], answer['source'], answer['gauge']['class']),
                                         (z.digest(), x.digest(), name))
                        self.assertNotEqual(z.digest(), x.digest())
                        candidate().keep('m8/z')
                        self.assertEqual(candidate().survey(range(600)), letters)
                        self.assertEqual({g: LETTER[row['status']] for g, row in z.survey(SURVEYED).items()},
                                         j1_letters)
                        z_rows, wz_digest, wz = self.run_word('m8/z', z)
                        for (cx, jx, mx, ix), (cz, jz, mz, iz) in zip(x_rows, z_rows):
                            self.assertEqual((cx, cz, jz), (jx, jx, jx))
                            self.assertEqual((mx, mz), (H.moved_set_hash(ix.tolist()), H.moved_set_hash(iz.tolist())))
                            np.testing.assert_array_equal(np.sort(sigma[iz]), ix)
                        self.assertEqual(wz_digest, wz.digest())
                        self.assertEqual(candidate().gauge(sigma, t, state='m8/wx')['digest'], wz_digest)
                        self.assertEqual(j1_act(wx, sigma, t).digest(), wz_digest)
                        kept = {level for level in ('labels', 'region_class', 'shape')
                                if self.project(level, 'm8/x') == self.project(level, 'm8/z')}
                        self.assertEqual(kept, keeps[name])
                        if off == 0:
                            self.assertEqual(np.array_equal(nu.labels(z), nu.labels(x)), name == 'S')
                        between = candidate().gauge_between('m8/z', state='m8/x')
                        self.assertEqual((between['shapes_equal'], between['gauge']['class']), (True, name))
                        np.testing.assert_array_equal(uint32(between['gauge']['sigma']), sigma)
                        np.testing.assert_array_equal(uint32(between['gauge']['t']), t)
                # X and w(X): equal shapes exactly when a gauge relates them; on the lattice they always do.
                between = candidate().gauge_between('m8/wx', state='m8/x')
                self.assertEqual(between['shapes_equal'], between['gauge'] is not None, where)
                if off == 0:
                    self.assertTrue(between['shapes_equal'], where)
                if between['gauge'] is not None:
                    image = candidate().gauge(uint32(between['gauge']['sigma']), uint32(between['gauge']['t']),
                                              state='m8/x')
                    self.assertEqual(image['digest'], wx.digest(), where)
        finally:
            for name in ('m8/x', 'm8/z', 'm8/wx'):
                candidate().release(name)
        self.assertLessEqual({'admissible', 'blocked'}, statuses)

    def test_an_invalid_gauge_is_an_input_error(self):
        candidate().start(None)
        before = candidate().record()
        identity = np.arange(NP)
        g = int(sim.primitive(session().ctx, 1).kidx)
        bad = (
            {'sigma': 'not base64!'},
            {'sigma': base64.b64encode(b'\0' * 8).decode('ascii')},
            {'sigma': R.RustJumbleEngine._u32(np.zeros(NP, np.int64))},
            {'t': R.RustJumbleEngine._u32(np.full(NP, 7200))},
            {'t': R.RustJumbleEngine._u32(np.full(NP, 70000))},
            {'sigma': R.RustJumbleEngine._u32(identity), 't': R.RustJumbleEngine._u32(np.full(NP, g))},
            {'sigma': 3},
        )
        for fields in bad:
            with self.subTest(fields=sorted(fields)):
                with self.assertRaises(R.CandidateError) as raised:
                    candidate().call('gauge', **fields)
                self.assertEqual(raised.exception.kind, 'input')
                self.assertEqual(candidate().record(), before)
        for fields in ({}, {'other': 'no such state'}, {'other': 1}):
            with self.subTest(op='gauge_between', fields=fields):
                with self.assertRaises(R.CandidateError) as raised:
                    candidate().call('gauge_between', **fields)
                self.assertEqual(raised.exception.kind, 'input')


if __name__ == '__main__':
    unittest.main()
