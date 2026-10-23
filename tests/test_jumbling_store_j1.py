"""EN2-D: exact J1/store round trips; all runtime writes stay in tempfile.

Run directly, or through unittest alongside the NumPy-free harness checks.
The latter uses one isolated worker so those checks keep their import proof.
JUMBLING_FULL_REPLAY=1 also checks S4/I_b ends and correction 35's records.
"""

import sys

sys.dont_write_bytecode = True

import copy
from functools import lru_cache
import gzip
import json
import os
from pathlib import Path
import tempfile
from time import perf_counter
import unittest

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / "research" / "jumbling"
sys.path.insert(0, str(JUMBLING))

from differential import independence, menu_order
from differential.candidate import store


_FULL = os.environ.get("JUMBLING_FULL_REPLAY") == "1"
_IN_PROCESS = __name__ == "__main__"
_TEMP_ROOT = None
_CLASS_SECONDS = {}
_MEASUREMENTS = {}
_READ_BACK_JOURNALS = {}
_CRASH_POINTS = ("after_temp_write", "before_replace", "after_replace")

# J1 replay of library-merged.json.gz, macro id 35 (card5:MB2-5), from
# solved under wj-I_a.json's menu: 5,629 admissible records, ending on lattice.
# The default run replays the stored letters once against this measured reference;
# full mode additionally replays the original library records.
_CORRECTION_DIGEST = "80642dd327e7c86b085019db9fda41c87a2f540386d2bfeb90397bf1abf325d1"
# J1 prefix replays of the same fixture journal (CPython 3.11 / NumPy 2.3.5).
_IA_40_DIGEST = "b8918ba6c1d9d353923a85f14d80059c3922e53d21723f2a8723315cde1398fc"
_IA_60_DIGEST = "ec3acb21fe14da213680d2b2e83a04d2f74dc8f77fa6dffeac4fd495a60b6e0a"
# J1's first H6030 prefix with a component above 2**64 (204 letters, 66 bits).
_X1_204_DIGEST = "1c34e7c3623d63f553b85ec2564f117f5c0892aaee6e1ea7c084198dc7f2af6e"


def _audit_temp_writes(event, args):
    if _TEMP_ROOT is None:
        return
    paths = []
    if event == "open":
        path, mode, flags = args
        if flags & (os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC | os.O_APPEND):
            paths = [(path, None)]
    elif event in ("os.mkdir", "os.chmod"):
        paths = [(args[0], args[2])]
    elif event in ("os.remove", "os.rmdir"):
        paths = [(args[0], args[1])]
    elif event == "os.truncate":
        paths = [(args[0], None)]
    elif event == "os.rename":
        paths = [(args[0], args[2]), (args[1], args[3])]
    elif event in ("os.link", "os.symlink"):
        raise AssertionError("test attempted to create a link")
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
                raise AssertionError("directory fd is outside the test's tempfile root")
        if not path.resolve().is_relative_to(_TEMP_ROOT):
            raise AssertionError("test attempted a write outside its tempfile root")


def setUpModule():
    global _TEMP_ROOT, _CTX, np, sim, j1_state
    temporary = tempfile.TemporaryDirectory(prefix="jumbling-store-j1-")
    _TEMP_ROOT = Path(temporary.name).resolve()
    if _TEMP_ROOT.is_relative_to(ROOT):
        temporary.cleanup()
        _TEMP_ROOT = None
        raise AssertionError("tempfile directory must be outside the repository")

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
        raise unittest.SkipTest("NumPy is required for the J1 store round trips") from error
    import sim
    from differential import j1_state
    started = perf_counter()
    _CTX = sim.get_context()  # One shared J1 context for the entire module.
    _MEASUREMENTS["context_seconds"] = perf_counter() - started


@lru_cache(maxsize=None)
def _fixture(name):
    return json.loads((JUMBLING / "fixtures" / f"wj-{name}.json").read_text(encoding="utf-8"))


@lru_cache(maxsize=None)
def _menu(name):
    return sim.TwistMenu.from_record(_CTX, _fixture(name)["menu"])


def _identities(name):
    fixture = _fixture(name)
    return {"model": copy.deepcopy(fixture["journal"]["model_identity"]),
            "menu": menu_order.menu_identity(fixture["menu"]),
            "menu_order": menu_order.order_digest(fixture["menu"]),
            "contract_revision": fixture["journal"]["contract_revision"]}


def _opened(state, name="I_a", *, journal=None, revision=None):
    if journal is None:
        journal = state.journal
    if revision is None:
        revision = len(journal)
    record = dict(j1_state.state_record(state), revision=revision)
    doc = store.cross_on({"format": store.SESSION_FORMAT, "revision": 0, "retained": {}},
                         {"kind": "word", "word": []}, _identities(name), record)
    doc["revision"] = revision + 1
    doc["jumbling"]["episodes"][0]["journal"] = copy.deepcopy(journal)
    store.validate(doc)
    return doc


def _snapshot(state):
    # Poses and exact Q5 entries are immutable during J1 application. Copy only
    # the changing state containers, keeping the one shared context and menu.
    saved = copy.copy(state)
    saved.pose_id = state.pose_id.copy()
    saved._poses = list(state._poses)
    saved._lookup = dict(state._lookup)
    saved.journal = list(state.journal)
    saved._anchor_cache = {}
    return saved


def _apply_records(state, records):
    for index, record in enumerate(records):
        outcome = state.apply(sim.Twist.from_record(_CTX, record))
        if not outcome.applied or outcome.moved != record.get("moved", outcome.moved):
            raise AssertionError(f"record {index}: {outcome}")


def _apply_letters(state, letters, *, capture_at=None):
    captured = None
    for index, twist in enumerate(j1_state.twists_from_letters(state.menu, letters), 1):
        outcome = state.apply(twist, journal=False)
        if not outcome.applied:
            raise AssertionError(f"letter {index - 1}: {outcome}")
        if index == capture_at:
            captured = state.digest()
    return captured


@lru_cache(maxsize=None)
def _fixture_states(name):
    fixture = _fixture(name)
    stages = ("start", "mid", "end") if name == "I_a" or _FULL else ("start", "mid")
    counts = {fixture["stages"][stage]["records"] for stage in stages}
    if name == "I_a":
        counts.update((40, 60))
    records = fixture["journal"]["records"][:max(counts)]
    for record in records:
        store.check_record(record)
    # Journal transport is checked with a solved checkpoint at revision zero.
    # Each actual stage is then stored separately with its matching prefix.
    checkpoint = sim.State(_CTX, menu=_menu(name))
    with tempfile.TemporaryDirectory(dir=_TEMP_ROOT) as directory:
        path = Path(directory) / "journal.json"
        store.write_session(path, _opened(checkpoint, name, journal=records, revision=0))
        returned = store.read_session(path)["jumbling"]["episodes"][0]["journal"]
    if returned != records:
        raise AssertionError(f"{name}: stored journal differs from fixture records")
    _READ_BACK_JOURNALS[name] = returned
    saved = {}

    class CapturingState(sim.State):
        # Inherit J1's replay unchanged; observe apply only to save stage states.
        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)
            saved[0] = _snapshot(self)

        def apply(self, *args, **kwargs):
            outcome = super().apply(*args, **kwargs)
            if outcome.applied and len(self.journal) in counts:
                saved[len(self.journal)] = _snapshot(self)
            return outcome

    started = perf_counter()
    CapturingState.replay(dict(fixture["journal"], records=returned), _CTX, menu=_menu(name))
    _MEASUREMENTS[f"{name}_fixture_replay_seconds"] = perf_counter() - started
    if name == "I_a":
        _MEASUREMENTS["i_a_journal_replay_seconds"] = _MEASUREMENTS["I_a_fixture_replay_seconds"]
    return saved


@lru_cache(maxsize=1)
def _correction():
    path = JUMBLING / "restoration/09-general-method/data/library-merged.json.gz"
    with gzip.open(path, "rt", encoding="utf-8") as stream:
        library = json.load(stream)
    if library["format"] != "I_a-lattice-correction-library-v1":
        raise AssertionError("unexpected correction library format")
    macro = next(m for m in library["macros"] if m["id"] == 35)
    if macro["family"] != "card5:MB2-5" or macro["length"] != 5629 or len(macro["records"]) != 5629:
        raise AssertionError("unexpected correction 35 source")
    letters = j1_state.letters_from_records(_CTX, _menu("I_a"), macro["records"])
    return macro["records"], letters


def _word(letters, *, parts=None):
    word = {"name": "J1 word", "menu_size": 60, "identities": _identities("I_a"),
            "recorded_at": _fixture("I_a")["stages"]["start"]["digest"],
            "letters": store.encode_ids(grip * 60 + item for grip, item in letters)}
    if parts is not None:
        word["parts"] = parts
    return word


class _StoreCase(unittest.TestCase):
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
        self.path = self.directory / "session.json"

    def roundtrip(self, doc):
        store.validate(doc)
        store.write_session(self.path, doc)
        restored = store.read_session(self.path)
        self.assertEqual(restored, doc)
        self.assertEqual(store.dumps(restored), self.path.read_bytes())
        self.assertEqual(list(self.directory.iterdir()), [self.path])
        return restored

    def assertState(self, record, state, expected_digest=None):
        digest = state.digest()
        self.assertEqual(record["digest"], digest)
        if expected_digest is not None:
            self.assertEqual(digest, expected_digest)
        ids = store.decode_ids(record["pose_ids"], len(record["pose_table"]))
        self.assertEqual(ids, state.pose_id.tolist())
        self.assertEqual(record["pose_ids"], store.encode_ids(state.pose_id.tolist()))
        self.assertEqual(store.canonical_digest(record["pose_table"], ids,
                                               j1_state.kplus_matrix(_CTX)), digest)
        self.assertEqual(record["pose_table"][0], {"k": 0})
        self.assertEqual(len(record["pose_table"]), len(state._poses))
        for stored, pose in zip(record["pose_table"], state._poses):
            if pose.kidx >= 0:
                self.assertEqual(stored, {"k": pose.kidx})
                self.assertIs(type(stored["k"]), int)
            else:
                self.assertEqual(set(stored), {"m"})
                self.assertEqual(len(stored["m"]), 4)
                for row, exact_row in zip(stored["m"], pose.matrix):
                    self.assertEqual(len(row), 4)
                    for triple, exact in zip(row, exact_row):
                        self.assertEqual(triple, [exact.a, exact.b, exact.d])
                        self.assertTrue(all(type(value) is int for value in triple))


class FixtureStoreTests(_StoreCase):
    def test_fixture_states_roundtrip(self):
        for name in ("S4", "I_a", "I_b"):
            fixture = _fixture(name)
            self.assertEqual(_CTX.data.identity, fixture["journal"]["model_identity"])
            self.assertEqual(_menu(name).identity, fixture["journal"]["menu_identity"])
            for stage in ("start", "mid", "end"):
                if stage == "end" and name != "I_a" and not _FULL:
                    continue
                with self.subTest(menu=name, stage=stage):
                    reference = fixture["stages"][stage]
                    state = _fixture_states(name)[reference["records"]]
                    record = j1_state.state_record(state)
                    self.assertState(record, state, reference["digest"])
                    doc = _opened(state, name, journal=fixture["journal"]["records"][:reference["records"]])
                    restored = self.roundtrip(doc)["jumbling"]["episodes"][0]
                    self.assertEqual(restored["identities"], _identities(name))
                    self.assertState(restored["state"], state, reference["digest"])

    def test_i_a_crash_recovery_is_a_whole_revision(self):
        fixture = _fixture("I_a")
        states = _fixture_states("I_a")
        mid, end = (states[fixture["stages"][stage]["records"]] for stage in ("mid", "end"))
        old = _opened(mid, journal=fixture["journal"]["records"][:fixture["stages"]["mid"]["records"]])
        new = _opened(end, journal=fixture["journal"]["records"])
        new["revision"] = old["revision"] + 1
        new["jumbling"]["episodes"][0]["state"]["revision"] = old["jumbling"]["episodes"][0]["state"]["revision"] + 1
        for point in _CRASH_POINTS:
            with self.subTest(crash_at=point):
                store.write_session(self.path, old)
                with self.assertRaisesRegex(store.SimulatedCrash, point):
                    store.write_session(self.path, new, crash_at=point)
                expected, state = (new, end) if point == "after_replace" else (old, mid)
                if point != "after_replace":
                    self.assertEqual(len(list(self.directory.iterdir())), 2)
                recovered = store.read_session(self.path)
                self.assertEqual(recovered, expected)  # Includes journal, revision and poses.
                self.assertState(recovered["jumbling"]["episodes"][0]["state"], state)
                self.assertEqual(list(self.directory.iterdir()), [self.path])

    def test_i_a_journal_roundtrip_and_replay(self):
        fixture = _fixture("I_a")
        state = _fixture_states("I_a")[fixture["stages"]["end"]["records"]]
        for record in fixture["journal"]["records"]:
            store.check_record(record)
        restored = self.roundtrip(_opened(state, journal=fixture["journal"]["records"]))
        records = restored["jumbling"]["episodes"][0]["journal"]
        self.assertEqual(records, fixture["journal"]["records"])
        # _fixture_states inherited State.replay on these read-back records and
        # captured its end state. Reuse that replay, keeping the default under
        # budget; verify the final-state document returns the identical journal.
        self.assertEqual(records, _READ_BACK_JOURNALS["I_a"])
        self.assertState(restored["jumbling"]["episodes"][0]["state"], state,
                         fixture["stages"]["end"]["digest"])

    def test_scramble_separate_from_solver_journal(self):
        fixture = _fixture("I_a")
        state = _fixture_states("I_a")[60]
        scramble, solver = fixture["journal"]["records"][:40], fixture["journal"]["records"][40:60]
        doc = _opened(state, journal=solver, revision=60)
        doc["jumbling"]["scrambles"].append({"identities": _identities("I_a"),
                                             "journal": copy.deepcopy(scramble), "seed": 7101})
        restored = self.roundtrip(doc)["jumbling"]
        self.assertEqual(restored["scrambles"], doc["jumbling"]["scrambles"])
        self.assertEqual(restored["episodes"][0]["journal"], solver)
        replayed = sim.State.replay(restored["scrambles"][0]["journal"], _CTX, menu=_menu("I_a"))
        self.assertEqual(replayed.digest(), _IA_40_DIGEST)
        self.assertEqual(replayed.digest(), _fixture_states("I_a")[40].digest())
        _apply_records(replayed, restored["episodes"][0]["journal"])
        self.assertState(restored["episodes"][0]["state"], replayed, _IA_60_DIGEST)


class J1ExportTests(_StoreCase):
    def test_import_independence(self):
        self.assertEqual(independence.scan_paths([JUMBLING / "differential/j1_state.py"],
                                                 independence.HARNESS_POLICY), [])

    def test_record_not_in_menu_names_its_index(self):
        menu = _menu("I_a")
        records = [menu.for_cap(0)[0].record(), sim.plane(_CTX, 0, 13, degrees=10).record()]
        with self.assertRaisesRegex(ValueError, "record 1"):
            j1_state.letters_from_records(_CTX, menu, records)


class LabelAndIntegerStoreTests(_StoreCase):
    def test_label_exit_keeps_exact_poses(self):
        from exact import matmul, transpose

        menu = _menu("I_a")
        cap = menu.for_cap(0)
        g = next(t for t in cap if not t.retained)
        a = next(t for t in cap if t.retained and not t.is_identity)
        state = sim.State(_CTX, menu=menu)
        self.assertTrue(state.apply(g).applied)
        self.assertFalse(state.is_lattice())
        self.assertTrue(state.apply(sim.Twist(_CTX, 0, matmul(a.matrix, transpose(g.matrix)))).applied)
        self.assertTrue(state.is_lattice())
        self.assertGreater(state.moved_count(), 0)
        retained = sim.State(_CTX, menu=menu)
        self.assertTrue(retained.apply(a).applied)
        self.assertEqual(state.digest(), retained.digest())
        doc = _opened(state)
        # Synthetic certificate content: only its accepted shape, digest binding
        # and the exact exit poses are tested; this is not a nu/theorem check.
        certificate = {"pose_digest": state.digest(), "labels_digest": "b" * 64,
                       "nu": [0] * 45, "c5_convention": "synthetic convention",
                       "certificate_program_sha256": "c" * 64,
                       "contract_revision": "state-contract 2026-10-22 A1-A5"}
        doc["jumbling"]["episodes"][0]["identities"]["contract_revision"] = certificate["contract_revision"]
        closed = store.cross_off(doc, certificate, j1_state.state_record(state), {})
        restored = self.roundtrip(closed)["jumbling"]["episodes"][0]
        self.assertEqual(restored["end"]["kind"], "label")
        self.assertEqual(restored["end"]["certificate"], certificate)
        self.assertState(restored["state"], state)
        self.assertState(restored["end"]["exit_state"], state)

    def test_x1_pose_components_exceed_64_bits(self):
        witness = json.loads((JUMBLING / "restoration/10-x1-unbounded/witnesses/H6030.json").read_text(encoding="utf-8"))
        self.assertEqual(witness["from"], "solved")
        state = sim.State(_CTX, menu=_menu("I_a"))
        started = perf_counter()
        # A fixed prefix, so the case never depends on the machine's speed.
        for count, twist in enumerate(j1_state.twists_from_letters(state.menu, witness["word"][:204]), 1):
            self.assertTrue(state.apply(twist).applied, f"H6030 letter {count - 1}")
        largest = max((max(abs(q.a), abs(q.b), q.d)
                       for pose in state._poses if pose.kidx < 0
                       for row in pose.matrix for q in row), default=0)
        self.assertGreater(largest, 2 ** 64)
        self.assertEqual(state.digest(), _X1_204_DIGEST)
        _MEASUREMENTS["large_integer_case"] = {"prefix": count, "bits": largest.bit_length(),
                                               "seconds": perf_counter() - started}
        record = j1_state.state_record(state)
        self.assertState(record, state)
        restored = self.roundtrip(_opened(state))["jumbling"]["episodes"][0]["state"]
        self.assertState(restored, state)


class StoredWordTests(_StoreCase):
    def test_correction_35_letters_roundtrip_and_replay(self):
        records, letters = _correction()
        menu = _menu("I_a")
        self.assertEqual(len(menu), 60)
        self.assertEqual(menu.identity, menu_order.I_A_IDENTITY)
        self.assertEqual(menu_order.order_digest(_fixture("I_a")["menu"]), menu_order.I_A_ORDER_DIGEST)
        self.assertEqual(len(letters), 5629)
        doc = _opened(sim.State(_CTX, menu=menu))
        doc["jumbling"]["words"].append(_word(letters))
        word = self.roundtrip(doc)["jumbling"]["words"][0]
        self.assertEqual(word["identities"], _identities("I_a"))
        self.assertEqual(word["recorded_at"], sim.State(_CTX, menu=menu).digest())
        decoded = [list(divmod(code, 60)) for code in store.decode_ids(word["letters"], 600 * 60)]
        self.assertEqual(decoded, letters)
        self.assertEqual(len(decoded), 5629)
        # Independent direction: J1 transports the stored items back to grips.
        # Compare every letter with its exact source record without replaying it.
        matrices = {}
        for record, twist in zip(records, j1_state.twists_from_letters(menu, decoded)):
            if twist not in matrices:
                matrices[twist] = sim.kplus.matrix_json(twist.matrix)
            self.assertEqual(matrices[twist], record["matrix"])
        state = sim.State(_CTX, menu=menu)
        started = perf_counter()
        _apply_letters(state, decoded)
        _MEASUREMENTS["correction_35_letters_replay_seconds"] = perf_counter() - started
        self.assertEqual(state.digest(), _CORRECTION_DIGEST)
        self.assertTrue(state.is_lattice())
        if _FULL:
            started = perf_counter()
            source = sim.State.replay(records, _CTX, menu=menu)
            _MEASUREMENTS["correction_35_records_replay_seconds"] = perf_counter() - started
            self.assertEqual(source.digest(), state.digest())

    def test_inverse_and_same_as_parts_roundtrip(self):
        _, letters = _correction()
        a = letters[:40]
        inverse = menu_order.inverse_index(_fixture("I_a")["menu"])
        literal = a + [[grip, inverse[item]] for grip, item in reversed(a)] + a
        tree = {"name": "root", "letters": 120, "parts": [
            {"name": "A", "letters": 40, "span": [0, 40]},
            {"name": "A inverse", "letters": 40, "span": [40, 80], "inverse_of": [0]},
            {"name": "A", "letters": 40, "span": [80, 120], "same_as": [0]},
        ]}
        doc = _opened(sim.State(_CTX, menu=_menu("I_a")))
        doc["jumbling"]["words"].append(_word(literal, parts=tree))
        word = self.roundtrip(doc)["jumbling"]["words"][0]
        self.assertEqual(word["parts"], tree)
        codes = store.decode_ids(word["letters"], 600 * 60)
        expanded = store.expand_parts(word["parts"], codes, inverse)
        self.assertEqual(expanded, [grip * 60 + item for grip, item in literal])
        self.assertEqual(store.parts_length(word["parts"]), 120)
        self.assertEqual(len(expanded), 120)
        state = sim.State(_CTX, menu=_menu("I_a"))
        at_80 = _apply_letters(state, [list(divmod(code, 60)) for code in expanded], capture_at=80)
        self.assertEqual(at_80, word["recorded_at"])
        reference = sim.State(_CTX, menu=_menu("I_a"))
        _apply_letters(reference, literal)
        self.assertEqual(state.digest(), reference.digest())


def load_tests(loader, tests, pattern):
    if _IN_PROCESS:
        return tests
    # Keep NumPy/J1 out of the caller: the existing acceptance modules assert
    # their absence. One worker runs all actual tests and shares one context.
    from tests import isolated_numpy
    return isolated_numpy.suite("test_jumbling_store_j1", "J1 store round trips in an isolated NumPy process",
                                "NumPy is required for the J1 store round trips")


if __name__ == "__main__":
    unittest.main(verbosity=0)
