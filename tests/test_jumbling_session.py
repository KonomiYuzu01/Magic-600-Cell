"""EN2-L optional-state sessions: fresh synthetic data, source/fixture evidence.

NumPy and J1 load only in setUpModule, or in an isolated worker when combined
with the NumPy-free differential tests. Every runtime write is audited.
"""

import sys

sys.dont_write_bytecode = True

import copy
import gzip
import json
import os
from pathlib import Path
import random
import tempfile
from time import perf_counter
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / "research/jumbling"
sys.path.insert(0, str(JUMBLING))
_IN_PROCESS = __name__ == "__main__"
_TEMP_ROOT = None
_CLASS_SECONDS = {}
_MEASUREMENTS = {}
_CRASH_POINTS = ("after_temp_write", "before_replace", "after_replace")


def _audit_temp_writes(event, args):
    if _TEMP_ROOT is None:
        return
    paths = []
    if event == "open":
        if args[2] & (os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC | os.O_APPEND):
            paths = [(args[0], None)]
    elif event in ("os.mkdir", "os.chmod"):
        paths = [(args[0], args[2])]
    elif event in ("os.remove", "os.rmdir"):
        paths = [(args[0], args[1])]
    elif event == "os.truncate":
        paths = [(args[0], None)]
    elif event == "os.rename":
        paths = [(args[0], args[2]), (args[1], args[3])]
    elif event in ("os.link", "os.symlink"):
        raise AssertionError("session tests must not create links")
    for path, directory_fd in paths:
        if isinstance(path, int):
            continue
        path = Path(os.fsdecode(path))
        if not path.is_absolute() and directory_fd not in (None, -1):
            status = os.fstat(directory_fd)
            for directory in [_TEMP_ROOT] + [p for p in _TEMP_ROOT.rglob("*") if p.is_dir()]:
                candidate = directory.stat()
                if (status.st_dev, status.st_ino) == (candidate.st_dev, candidate.st_ino):
                    path = directory / path
                    break
            else:
                raise AssertionError("directory fd is outside the test's temporary root")
        if not path.resolve().is_relative_to(_TEMP_ROOT):
            raise AssertionError("session test attempted a write outside its temporary root")


def setUpModule():
    global _TEMP_ROOT, np, sim, S, store, nu, independence, j1_state
    global _ENGINE, _CORRECTION, _INVERSE, _IDENTITIES, _RETAINED_ONE
    temporary = tempfile.TemporaryDirectory(prefix="jumbling-session-")
    _TEMP_ROOT = Path(temporary.name).resolve()
    if _TEMP_ROOT.is_relative_to(ROOT):
        temporary.cleanup()
        _TEMP_ROOT = None
        raise AssertionError("session test data must be outside the repository")

    def cleanup():
        global _TEMP_ROOT
        try:
            temporary.cleanup()
        finally:
            _TEMP_ROOT = None

    unittest.addModuleCleanup(cleanup)
    sys.addaudithook(_audit_temp_writes)
    try:
        import numpy as np
    except ImportError as error:
        raise unittest.SkipTest("NumPy is required for the optional-state session tests") from error
    # census creates its isolated centroid cache at import; keep it under audit.
    with mock.patch.object(tempfile, "tempdir", str(_TEMP_ROOT)):
        from differential.leads import census  # noqa: F401
    import sim
    from differential import independence, j1_state, menu_order
    from differential.candidate import session as S, store
    from differential.references import nu
    from differential.references.session_engine import J1SessionEngine
    started = perf_counter()
    _ENGINE = J1SessionEngine()
    _IDENTITIES = copy.deepcopy(_ENGINE.identities)
    _INVERSE = menu_order.inverse_index(_ENGINE.menu.record())
    with gzip.open(JUMBLING / "restoration/09-general-method/data/library-merged.json.gz",
                   "rt", encoding="utf-8") as stream:
        library = json.load(stream)
    if library["format"] != "I_a-lattice-correction-library-v1":
        raise AssertionError("correction library format differs from the measured fixture")
    correction = next(m for m in library["macros"] if m["id"] == 2)
    if correction["length"] != 5 or len(correction["records"]) != 5:
        raise AssertionError("correction 2 length differs: expected five letters")
    _CORRECTION = j1_state.letters_from_records(_ENGINE.ctx, _ENGINE.menu, correction["records"])
    _RETAINED_ONE = j1_state.letters_from_records(
        _ENGINE.ctx, _ENGINE.menu, _ENGINE.replay_retained([1]).journal)
    _MEASUREMENTS["setup_seconds"] = perf_counter() - started


def load_tests(loader, tests, pattern):
    if _IN_PROCESS:
        return tests
    from tests.isolated_numpy import suite
    return suite("test_jumbling_session", "optional-state sessions (isolated worker)",
                 "NumPy is required for the optional-state session tests")


class _SessionCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._started = perf_counter()

    @classmethod
    def tearDownClass(cls):
        _CLASS_SECONDS[cls.__name__] = perf_counter() - cls._started

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(dir=_TEMP_ROOT)
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.count = 0

    def session(self):
        self.count += 1
        return S.Session(_ENGINE, self.directory / f"session-{self.count}.json",
                         identities=_IDENTITIES)

    def apply(self, session, letters):
        session.jumble_letters(session.revision, copy.deepcopy(letters))

    def crossing(self):
        # Correction 2 (row b34), retained generator 1 as a menu letter, correction 2 again:
        # ν = b34 + 0 + b34 = 0 at a lattice state that is not solved and has no known witness.
        session = self.session()
        session.jumble_on(session.revision)
        self.apply(session, _CORRECTION + _RETAINED_ONE + _CORRECTION)
        self.assertEqual(_ENGINE.nu(session.state), nu.ZERO, "correction 2, generator 1, correction 2: ν")
        self.assertNotEqual(session.state.digest(), _ENGINE.solved().digest(), "crossing pose_digest")
        self.assertFalse(np.array_equal(nu.labels(session.state), _ENGINE.solved_labels()), "crossing labels")
        return session

    def labels_equal(self, actual, expected):
        actual, expected = np.asarray(actual), np.asarray(expected)
        self.assertEqual(actual.shape, expected.shape, "labels.shape")
        difference = np.flatnonzero(actual != expected)
        if difference.size:
            slot = int(difference[0])
            self.fail(f"labels slot {slot}: session {actual[slot]}, reference {expected[slot]}")

    def refused(self, session, error, kind, message, action):
        before, revision, document = session.path.read_bytes(), session.revision, session.document
        with self.assertRaises(error) as caught:
            action()
        self.assertEqual(caught.exception.kind, kind, "refusal.kind")
        self.assertIn(message, str(caught.exception), "refusal.message")
        self.assertEqual(session.path.read_bytes(), before, "refusal changed stored bytes")
        self.assertEqual(session.revision, revision, "refusal changed in-memory revision")
        self.assertEqual(session.document, document, "refusal changed in-memory document")

    def assert_exit(self, session, labels):
        document = store.read_session(session.path)
        store.validate(document)
        self.assertFalse(session.jumbling)
        episode = document["jumbling"]["episodes"][-1]
        certificate = episode["end"]["certificate"]
        self.assertEqual(certificate["labels_digest"], _ENGINE.labels_digest(labels),
                         "exit certificate.labels_digest")
        self.assertEqual(certificate["labels_digest"], document["retained"]["labels_digest"],
                         "retained.labels_digest")
        self.assertEqual(certificate["pose_digest"], episode["state"]["digest"],
                         "exit certificate.pose_digest")
        self.assertEqual(episode["end"]["exit_state"]["digest"], episode["state"]["digest"],
                         "exit_state.digest")
        self.assertEqual(certificate["nu"], [0] * 45, "exit certificate.nu")
        self.assertEqual(certificate["contract_revision"], nu.CONTRACT_REVISION)
        self.assertEqual(certificate["c5_convention"], nu.C5_CONVENTION)
        self.assertEqual(certificate["certificate_program_sha256"], nu.certificate_program_sha256())
        self.labels_equal(session._retained_labels(), labels)

    def fault(self, session, mutate):
        # Deliberate corruption of synthetic data: validate representation, then
        # prove that a later command refuses to treat it as reachability evidence.
        mutate(session._doc["jumbling"]["episodes"][-1])
        store.write_session(session.path, session._doc)


class OptionalSessionTests(_SessionCase):
    def test_1_never_on_is_byte_identical_to_retained_only(self):
        with mock.patch.object(_ENGINE, "solved", side_effect=AssertionError("pose path used")):
            session = self.session()
            baseline = S.RetainedOnlySession(_ENGINE, self.directory / "retained.json")
            self.assertEqual(session.path.read_bytes(), baseline.path.read_bytes())
            commands = [("retained_moves", ([1, -6, 300],)), ("checkpoint", ()),
                        ("retained_command", ("macro",)), ("reset", ()),
                        ("import_labels", (_ENGINE.labels_digest(_ENGINE.solved_labels()),)),
                        ("restore", (0,)), ("retained_moves", ([-1200, 4],)), ("export", ())]
            for name, args in commands:
                with self.subTest(command=name):
                    getattr(session, name)(session.revision, *args)
                    getattr(baseline, name)(baseline.revision, *args)
                    self.assertEqual(session.document["retained"], baseline.document["retained"])
                    self.assertEqual(session.path.read_bytes(), baseline.path.read_bytes(),
                                     f"stored bytes after {name}")
                    self.assertNotIn("jumbling", store.read_session(session.path))
            _MEASUREMENTS["retained_layout"] = session.document["retained"]

    def test_2_enter_solved_and_thirty_seeded_retained_generators(self):
        rng = random.Random(7202)
        random_word = [rng.choice((-1, 1)) * rng.randrange(1, 1201) for _ in range(30)]
        for word in ([], random_word):
            with self.subTest(generators=len(word)):
                session = self.session()
                if word:
                    session.retained_moves(session.revision, word)
                session.jumble_on(session.revision, **({"word": word} if not word else {}))
                reference = sim.State(_ENGINE.ctx, menu=_ENGINE.menu)
                for generator in word:
                    self.assertTrue(reference.apply(sim.primitive(_ENGINE.ctx, generator)).applied)
                self.assertEqual(session.state.digest(), reference.digest(), "entry pose_digest")
                self.assertTrue(session.state.checkpoint(word)["checkpoint"])
                checkpoint = session.checkpoint(session.revision, witness=word)
                self.assertEqual(checkpoint["witness"], word)
                self.assertEqual(checkpoint["certificate"], nu.label_certificate(reference))
                self.labels_equal(nu.labels(session.state), _ENGINE.apply_generators(
                    _ENGINE.solved_labels(), word))
                self.assertEqual(session.document["jumbling"]["episodes"][0]["start"],
                                 {"kind": "word", "word": word})

    def test_3_witnessed_return_continues_the_retained_word(self):
        session = self.session()
        session.jumble_on(session.revision)
        # Measured I_a pair: y*x is the retained generator -236 at grip 117.
        pair, witness = [[117, 12], [117, 14]], [-236]
        self.apply(session, pair[:1])
        self.assertFalse(session.state.is_lattice())
        self.apply(session, pair[1:])
        checkpoint = session.checkpoint(session.revision, witness=witness)
        self.assertEqual(checkpoint["witness"], witness)
        expected, reference_checkpoint = session.state.export_retained(witness)
        self.assertTrue(reference_checkpoint["checkpoint"])
        labels = session.jumble_off(session.revision, witness=witness)
        self.labels_equal(labels, expected)
        self.assert_exit(session, labels)
        self.assertEqual(session.document["retained"]["checkpoint"],
                         {"kind": "word", "word": witness})
        tail = [1, -1200]
        session.retained_moves(session.revision, tail)
        self.assertEqual(session.document["retained"]["journal"], tail)
        self.labels_equal(session._retained_labels(), _ENGINE.apply_generators(
            _ENGINE.solved_labels(), witness + tail))

    def test_4_unwitnessed_label_crossing_stores_certificate_and_exact_exit(self):
        session = self.crossing()
        self.assertEqual(len(session.document["jumbling"]["episodes"][0]["journal"]), 10 + len(_RETAINED_ONE))
        expected, exact = nu.labels(session.state), _ENGINE.state_record(session.state)
        checkpoint = session.checkpoint(session.revision)
        self.assertIsNone(checkpoint["witness"])
        labels = session.jumble_off(session.revision)
        self.labels_equal(labels, expected)
        self.assert_exit(session, labels)
        document = session.document
        self.assertEqual(document["jumbling"]["episodes"][0]["end"]["exit_state"], exact)
        self.assertNotIn("witness", document["jumbling"]["episodes"][0]["end"])
        self.assertEqual(document["retained"]["checkpoint"]["kind"], "labels")
        self.assertEqual(document["retained"]["journal"], [])
        _MEASUREMENTS["crossing_pose_digest"] = exact["digest"]
        _MEASUREMENTS["crossing_labels_digest"] = document["retained"]["labels_digest"]

    def test_5_solved_end_exits_on_labels(self):
        """Non-solved poses with identity labels are not run: main has no cheap reachable fixture.

        The measured stage-13 case needs 8.7 million retained twists; this rule
        is exercised at solved reached through a jumble twist and its inverse.
        """
        session = self.session()
        session.jumble_on(session.revision)
        grip, item = _CORRECTION[0]
        self.apply(session, [[grip, item]])
        self.assertGreater(session.state.off_lattice_count(), 0)
        self.apply(session, [[grip, _INVERSE[item]]])
        self.labels_equal(nu.labels(session.state), _ENGINE.solved_labels())
        labels = session.jumble_off(session.revision)
        self.assert_exit(session, labels)
        self.labels_equal(labels, _ENGINE.solved_labels())

    def test_6_resume_exit_poses_then_break_chain_by_reset_import_restore(self):
        session = self.crossing()
        labels = session.jumble_off(session.revision)
        self.assert_exit(session, labels)
        exited = session.document["jumbling"]["episodes"][0]["end"]["exit_state"]
        moves = [1, -700, 1200]
        session.retained_moves(session.revision, moves)
        expected_labels = _ENGINE.apply_generators(labels, moves)
        self.labels_equal(session._retained_labels(), expected_labels)
        session = S.Session(_ENGINE, session.path, identities=_IDENTITIES)
        session.jumble_on(session.revision)
        reference = _ENGINE.state_from_record(exited)
        for generator in moves:
            self.assertTrue(reference.apply(sim.primitive(_ENGINE.ctx, generator)).applied)
        self.assertEqual(session.state.digest(), reference.digest(), "resumed pose_digest")
        self.labels_equal(nu.labels(session.state), expected_labels)
        self.assertEqual(session.document["jumbling"]["episodes"][-1]["start"],
                         {"kind": "exit", "episode": 0, "retained_moves": moves})
        self.assert_exit(session, session.jumble_off(session.revision))
        for action in ("reset", "import", "restore"):
            with self.subTest(chain_break=action):
                if action == "reset":
                    session.reset(session.revision)
                elif action == "import":
                    session.import_labels(session.revision, _ENGINE.labels_digest(_ENGINE.solved_labels()))
                else:
                    session.restore(session.revision, 0)  # The recovery checkpoint before the first exit.
                self.assertIsNone(session.document["jumbling"]["resume"])
                self.refused(session, S.WordRequired, "word_required", "labels alone",
                             lambda: session.jumble_on(session.revision))
                session.jumble_on(session.revision, word=[])
                self.assert_exit(session, session.jumble_off(session.revision, witness=[]))

    def test_7_refusals_preserve_bytes_revision_and_memory(self):
        for reason in ("off_lattice", "nu_nonzero", "centre_displaced", "journal_removed",
                       "journal_replaced", "labels_alone", "retained_workflow", "menu_word_off",
                       "menu_change_on"):
            with self.subTest(reason=reason):
                session = self.session()
                if reason == "labels_alone":
                    session.import_labels(session.revision, _ENGINE.labels_digest(_ENGINE.solved_labels()))
                    self.refused(session, S.WordRequired, "word_required", "retained word from solved",
                                 lambda: session.jumble_on(session.revision))
                    continue
                if reason == "menu_word_off":
                    self.refused(session, S.WrongMode, "wrong_mode", "Jumble is off",
                                 lambda: self.apply(session, _CORRECTION[:1]))
                    continue
                session.jumble_on(session.revision)
                if reason == "off_lattice":
                    self.apply(session, _CORRECTION[:1])
                    self.refused(session, S.OffLattice, "off_lattice", "twists that bring pieces back",
                                 lambda: session.jumble_off(session.revision))
                elif reason == "nu_nonzero":
                    self.apply(session, _CORRECTION)
                    self.assertEqual(nu.nonzero(_ENGINE.nu(session.state)), {"b34": 1})
                    self.refused(session, S.NuNonzero, "nu_nonzero", "ν is not 0; corrections",
                                 lambda: session.jumble_off(session.revision))
                elif reason == "centre_displaced":
                    synthetic = sim.State.from_global_rotation(_ENGINE.ctx, 2)
                    self.assertEqual(_ENGINE.ctx.kplus.compose(
                        _ENGINE.ctx.kplus.compose(2, 2), 2), 0, "global rotation order three")
                    self.assertEqual(nu.nu(synthetic), nu.ZERO)
                    self.assertFalse(nu.centres_home(synthetic))
                    with self.assertRaises(nu.LabelCheckpointRefused) as caught:
                        nu.label_certificate(synthetic)
                    self.assertEqual(caught.exception.fact, "centre_displaced")
                    self.fault(session, lambda e: e.update(state=dict(
                        _ENGINE.state_record(synthetic), revision=session.revision)))
                    self.refused(session, S.Unreachable, "unreachable", "centre is outside",
                                 lambda: session.jumble_off(session.revision))
                elif reason.startswith("journal_"):
                    self.apply(session, _CORRECTION * 2)
                    # Correction 2 has order two here. The named retained move
                    # gives a non-solved ν = 0 state, so deleting its journal
                    # cannot accidentally remain a proof from solved.
                    self.apply(session, _RETAINED_ONE)
                    replacement = ([] if reason == "journal_removed" else
                                   _ENGINE.replay_retained([2]).journal)
                    expected = session.state.digest()
                    replayed = _ENGINE.replay_journal(replacement).digest()
                    self.assertNotEqual(replayed, expected)
                    self.fault(session, lambda e: e.update(journal=copy.deepcopy(replacement)))
                    self.refused(session, S.Unreachable, "unreachable",
                                 f"episode 0 digest: journal {replayed}, poses {expected}",
                                 lambda: session.jumble_off(session.revision))
                elif reason == "retained_workflow":
                    for kind in ("macro", "block", "certificate", "setup_search"):
                        with self.subTest(kind=kind):
                            self.refused(session, S.WrongMode, "wrong_mode", f"retained {kind} action",
                                         lambda k=kind: session.retained_command(session.revision, k))
                else:
                    self.refused(session, S.WrongMode, "wrong_mode", "menu-change action",
                                 lambda: session.menu_change(session.revision, _IDENTITIES))


class RefusalAtomicityTests(_SessionCase):
    def test_stale_revision_at_every_command(self):
        session = self.session()
        session.jumble_on(session.revision)
        commands = [(session.retained_moves, ([1],)), (session.reset, ()),
                    (session.import_labels, ("0" * 64,)), (session.restore, (0,)),
                    (session.retained_command, ("macro",)), (session.jumble_on, ()),
                    (session.jumble_letters, ([],)), (session.menu_change, (_IDENTITIES,)),
                    (session.checkpoint, ()), (session.jumble_off, ()), (session.export, ())]
        for command, args in commands:
            with self.subTest(command=command.__name__):
                self.refused(session, S.StaleRevision, "stale_revision", "command revision 0",
                             lambda c=command, a=args: c(0, *a))

    def test_checkpoint_with_neither_witness_nor_label_certificate(self):
        session = self.session()
        session.import_labels(session.revision, _ENGINE.labels_digest(_ENGINE.solved_labels()))
        self.refused(session, S.CheckpointRequired, "checkpoint_required",
                     "neither a witness nor a label certificate", lambda: session.checkpoint(session.revision))
        session.jumble_on(session.revision, word=[])
        self.apply(session, _CORRECTION)
        self.refused(session, S.NuNonzero, "nu_nonzero", "corrections",
                     lambda: session.checkpoint(session.revision))

    def test_export_off_lattice(self):
        session = self.session()
        session.jumble_on(session.revision)
        self.apply(session, _CORRECTION[:1])
        self.refused(session, S.OffLattice, "off_lattice", "pieces are off the lattice",
                     lambda: session.export(session.revision))

    def test_mismatched_witness_and_entry_word(self):
        session = self.session()
        self.refused(session, S.WitnessMismatch, "witness_mismatch", "entry labels_digest",
                     lambda: session.jumble_on(session.revision, word=[1]))
        session.jumble_on(session.revision)
        for action in (session.checkpoint, session.jumble_off):
            with self.subTest(command=action.__name__):
                self.refused(session, S.WitnessMismatch, "witness_mismatch", "witness pose_digest",
                             lambda a=action: a(session.revision, witness=[1]))

    def test_invalid_later_letter_is_transactional(self):
        session = self.session()
        session.jumble_on(session.revision)
        self.refused(session, S.TwistRefused, "twist_refused", "fully admissible menu-word",
                     lambda: self.apply(session, [_CORRECTION[0], [600, 0]]))
        self.assertEqual(session.state.digest(), _ENGINE.solved().digest())

    def test_identity_and_input_errors_are_transactional(self):
        session = self.session()
        bad = copy.deepcopy(_IDENTITIES)
        bad["menu_order"] = "0" * 64
        self.refused(session, S.IdentityMismatch, "identity_mismatch", "menu_order:",
                     lambda: session.menu_change(session.revision, bad))
        self.refused(session, S.InvalidInput, "invalid_input", "signed generator ids",
                     lambda: session.retained_moves(session.revision, [True]))
        self.refused(session, S.InvalidInput, "invalid_input", "existing checkpoint index",
                     lambda: session.restore(session.revision, -1))
        session.import_labels(session.revision, "0" * 64)
        self.refused(session, S.LabelDataRequired, "label_data_required", "only by a digest",
                     lambda: session.retained_moves(session.revision, [1]))

    def test_candidate_policy_and_planted_forbidden_import(self):
        path = JUMBLING / "differential/candidate/session.py"
        self.assertEqual(independence.scan_paths([path], independence.CANDIDATE_POLICY), [])
        source = path.read_text(encoding="utf-8") + "\nimport sim\n"
        planted = independence.scan_source(source, independence.CANDIDATE_POLICY, path=str(path))
        self.assertEqual([(v.line, v.message) for v in planted],
                         [(len(source.splitlines()), "forbidden import: sim")])
        self.assertEqual(independence.scan_paths(independence.harness_files(),
                                                 independence.HARNESS_POLICY), [])


class RecoveryTests(_SessionCase):
    def test_nonidentity_exit_resume_refuses_a_planted_restart_from_solved(self):
        session = self.session()
        session.jumble_on(session.revision)
        self.apply(session, [[117, 12], [117, 14]])
        exited = _ENGINE.state_record(session.state)
        self.assertNotEqual(exited["digest"], _ENGINE.solved().digest())
        labels = session.jumble_off(session.revision)  # No supplied witness.
        self.assert_exit(session, labels)
        moves = [1, -700, 1200]
        session.retained_moves(session.revision, moves)
        expected_labels = _ENGINE.apply_generators(labels, moves)
        wrong = _ENGINE.labels_digest(_ENGINE.apply_generators(_ENGINE.solved_labels(), moves))
        expected = _ENGINE.labels_digest(expected_labels)
        self.assertNotEqual(wrong, expected)
        replay = _ENGINE.replay_retained
        with mock.patch.object(_ENGINE, "replay_retained",
                               side_effect=lambda generators, **kw: replay(generators)):
            self.refused(session, S.WitnessMismatch, "witness_mismatch",
                         f"entry labels_digest: replay {wrong}, retained {expected}",
                         lambda: session.jumble_on(session.revision))
        session.jumble_on(session.revision)
        reference = _ENGINE.state_from_record(exited)
        for generator in moves:
            self.assertTrue(reference.apply(sim.primitive(_ENGINE.ctx, generator)).applied)
        self.assertEqual(session.state.digest(), reference.digest(), "nonidentity resume pose_digest")
        self.labels_equal(nu.labels(session.state), expected_labels)

    def test_crashes_during_both_crossings_recover_one_whole_revision(self):
        for crossing in ("on", "off"):
            for point in _CRASH_POINTS:
                with self.subTest(crossing=crossing, crash_at=point):
                    session = self.session()
                    if crossing == "off":
                        session.jumble_on(session.revision)
                        self.apply(session, _CORRECTION * 2)
                    before, revision = session.document, session.revision
                    with mock.patch.object(store, "write_session", wraps=store.write_session) as write:
                        session.crash_at = point
                        action = session.jumble_on if crossing == "on" else session.jumble_off
                        with self.assertRaisesRegex(store.SimulatedCrash, point):
                            action(revision)
                    after = copy.deepcopy(write.call_args.args[1])
                    self.assertEqual(write.call_count, 1, "crossing must write once")
                    self.assertEqual(session.revision, revision, "interrupted in-memory revision")
                    recovered = store.read_session(session.path)
                    self.assertEqual(recovered, after if point == "after_replace" else before,
                                     "crash returned a partial crossing")
                    store.validate(recovered)
                    reloaded = S.Session(_ENGINE, session.path, identities=_IDENTITIES)
                    self.assertEqual(reloaded.document, recovered)
                    self.assertEqual(reloaded.jumbling,
                                     (crossing == "on") == (point == "after_replace"))
                    if crossing == "off" and point == "after_replace":
                        self.assert_exit(reloaded, nu.labels(_ENGINE.state_from_record(
                            recovered["jumbling"]["episodes"][-1]["end"]["exit_state"])))
                    self.assertEqual(list(session.path.parent.glob(f".{session.path.name}.session-*.tmp")), [])

    def test_resume_rejects_forged_certificate_and_load_is_atomic(self):
        session = self.crossing()
        self.assert_exit(session, session.jumble_off(session.revision))
        good = session.document
        bad_path = self.directory / "forged.json"
        bad = copy.deepcopy(good)
        bad["jumbling"]["episodes"][0]["end"]["certificate"]["certificate_program_sha256"] = "0" * 64
        store.write_session(bad_path, bad)
        self.refused(session, S.Unreachable, "unreachable", "certificate.certificate_program_sha256",
                     lambda: session.load(bad_path))
        self.assertEqual(store.read_session(bad_path), bad, "refused load rewrote its input")

    def test_malformed_recovery_checkpoints_are_refused_at_load_and_restore(self):
        session = self.session()
        session.retained_moves(session.revision, [1, -700])
        session.reset(session.revision)
        good = session.document
        self.assertEqual(len(good["retained"]["checkpoints"]), 1)
        snapshot = good["retained"]["checkpoints"][0]
        plants = {
            "generator outside the model": (S.InvalidInput, "signed generator ids",
                                            dict(snapshot, journal=[1201])),
            "journal that misses its labels": (S.WitnessMismatch, "retained labels_digest",
                                               dict(snapshot, journal=[1])),
            "extra key": (S.InvalidInput, "recovery checkpoint needs exactly",
                          dict(snapshot, checkpoints=[])),
            "other model": (S.IdentityMismatch, "model_identity",
                            dict(snapshot, model_identity={"planted": True})),
        }
        for index, (name, (error, message, planted)) in enumerate(plants.items()):
            with self.subTest(snapshot=name):
                bad = copy.deepcopy(good)
                bad["retained"]["checkpoints"][0] = planted
                bad_path = self.directory / f"planted-{index}.json"
                store.write_session(bad_path, bad)
                stored = bad_path.read_bytes()
                self.refused(session, error, error.kind, message, lambda: session.load(bad_path))
                self.assertEqual(bad_path.read_bytes(), stored, "refused load rewrote its input")
                # A snapshot changed after load is checked again by restore.
                session._doc = copy.deepcopy(bad)
                self.refused(session, error, error.kind, message,
                             lambda: session.restore(session.revision, 0))
                session._doc = copy.deepcopy(good)
        session.restore(session.revision, 0)
        self.labels_equal(session._retained_labels(),
                          _ENGINE.apply_generators(_ENGINE.solved_labels(), [1, -700]))


if __name__ == "__main__":
    unittest.main(verbosity=0)
