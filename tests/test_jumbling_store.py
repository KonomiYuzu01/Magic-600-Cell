"""EN2-B source/synthetic checks, with no NumPy or reference-engine imports.

Run: python tests/test_jumbling_store.py
All test writes are inside TemporaryDirectory; the standalone runner also
audits filesystem mutations and forbids any write outside its temporary root.
"""

import sys

# Namespace-package imports must not leave __pycache__ inside the worktree.
sys.dont_write_bytecode = True

import ast
import base64
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "research" / "jumbling"))

from differential.candidate import store


FIXTURE = json.loads((ROOT / "research/jumbling/fixtures/wj-I_a.json").read_text(encoding="utf-8"))
MENU = [item["matrix"] for item in FIXTURE["menu"]["base"]]
IDENTITIES = {
    "model": {"synthetic-model.json": "a" * 64},
    "menu": FIXTURE["menu"]["identity"],
    "menu_order": hashlib.sha256(json.dumps(MENU, separators=(",", ":")).encode()).hexdigest(),
    "contract_revision": "state-contract 2026-10-22 A1-A5",
}
CRASH_POINTS = ("after_temp_write", "before_replace", "after_replace")
_WRITE_ROOT = None


def synthetic_matrix(index):
    """Distinct exact matrices for encoding tests, not geometric rotations."""
    matrix = [[[int(i == j), 0, 1] for j in range(4)] for i in range(4)]
    matrix[0][0] = [index + 1, 0, 1]
    matrix[1][2] = [0, index, 1]
    return matrix


def state(table=None, ids=None):
    table = copy.deepcopy(table if table is not None else [{"k": 0}, {"k": 1}])
    ids = ids if ids is not None else [0, 1, 1]
    return {"revision": 0, "pose_table": table, "pose_ids": store.encode_ids(ids),
            "digest": store.canonical_digest(table, ids, synthetic_matrix)}


def session(*, section=False):
    doc = {"format": store.SESSION_FORMAT, "revision": 0,
           "retained": {"labels": "old", "preferences": {"theme": "夜", "scale": 1.25}}}
    if section:
        doc["jumbling"] = {"format": store.JUMBLING_FORMAT, "episodes": [],
                           "words": [], "scrambles": []}
    return doc


def opened():
    return store.cross_on(session(), {"kind": "word", "word": [1, -2, 1200]},
                          IDENTITIES, state())


def exit_state(doc):
    return {key: copy.deepcopy(value) for key, value in
            doc["jumbling"]["episodes"][-1]["state"].items() if key != "revision"}


def certificate(doc):
    return {"pose_digest": doc["jumbling"]["episodes"][-1]["state"]["digest"],
            "labels_digest": "b" * 64, "nu": [0] * 45,
            "c5_convention": "synthetic r0 convention",
            "certificate_program_sha256": "c" * 64,
            "contract_revision": IDENTITIES["contract_revision"]}


def word_record(letters, parts=None):
    result = {"name": "synthetic word", "menu_size": len(MENU),
              "letters": store.encode_ids(letters), "identities": copy.deepcopy(IDENTITIES),
              "recorded_at": "d" * 64}
    if parts is not None:
        result["parts"] = parts
    return result


def part_example():
    # Compute inverses from the *fixture's* exact menu, independently of store.
    keys = {json.dumps(matrix): i for i, matrix in enumerate(MENU)}
    inverse = [keys[json.dumps([list(row) for row in zip(*matrix)])] for matrix in MENU]
    item = next(i for i, inv in enumerate(inverse) if i != inv)
    forward = [7 * len(MENU) + item, 599 * len(MENU) + item, inverse[item]]
    backward = []
    for letter in reversed(forward):
        grip, index = divmod(letter, len(MENU))
        backward.append(grip * len(MENU) + inverse[index])
    letters = forward + backward + forward
    tree = {"name": "root", "letters": 9, "parts": [
        {"name": "V", "letters": 3, "parts": [
            {"name": "head", "letters": 1, "span": [0, 1]},
            {"name": "tail", "letters": 2, "span": [1, 3]},
        ]},
        {"name": "V inverse", "letters": 3, "span": [3, 6], "inverse_of": [0]},
        {"name": "V", "letters": 3, "span": [6, 9], "same_as": [0]},
    ]}
    return tree, letters, inverse


class StoreTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(dir=_WRITE_ROOT)
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.path = self.directory / "session.json"

    def roundtrip(self, doc):
        before = store.dumps(doc)
        store.write_session(self.path, doc)
        self.assertEqual(self.path.read_bytes(), before)
        restored = store.read_session(self.path)
        self.assertEqual(restored, doc)
        self.assertEqual(store.dumps(restored), before)
        self.assertEqual(list(self.directory.iterdir()), [self.path])
        return restored

    def assertBad(self, doc, path):
        with self.assertRaisesRegex(store.StoreFormatError, re.escape(path)):
            store.validate(doc)

    def test_candidate_imports_only_standard_library(self):
        forbidden = {"importlib", "__import__", "exec", "eval", "compile"}
        for filename in ("__init__.py", "store.py"):
            source = ROOT / "research/jumbling/differential/candidate" / filename
            for node in ast.walk(ast.parse(source.read_text(encoding="utf-8"))):
                if isinstance(node, (ast.Import, ast.ImportFrom)):
                    names = [alias.name for alias in node.names] if isinstance(node, ast.Import) else [node.module]
                    for name in names:
                        self.assertIsNotNone(name)
                        self.assertNotIn(name.split(".")[0], forbidden)
                        self.assertTrue(name.split(".")[0] in sys.stdlib_module_names
                                        or name.startswith("differential.candidate"), name)
                if isinstance(node, ast.Name):
                    self.assertNotIn(node.id, forbidden)
                if isinstance(node, ast.Attribute):
                    self.assertNotIn(node.attr, forbidden)
                    self.assertFalse(isinstance(node.value, ast.Name)
                                     and node.value.id == "sys" and node.attr == "path")
        self.assertNotIn("numpy", sys.modules)

    def test_q5_very_large_integers_roundtrip_exactly(self):
        matrix = synthetic_matrix(0)
        huge = 10 ** 450 + 7
        matrix[0][0] = [huge, -(10 ** 420 + 3), 1]
        matrix[0][1] = [1, -huge, 10 ** 460 + 9]
        store.check_q5(matrix[0][0])
        doc = store.cross_on(session(), {"kind": "word", "word": []}, IDENTITIES,
                             state([{"k": 0}, {"m": matrix}], [1, 0, 1]))
        restored = self.roundtrip(doc)
        self.assertEqual(restored["jumbling"]["episodes"][0]["state"]["pose_table"][1]["m"], matrix)
        self.assertIn(str(huge).encode(), self.path.read_bytes())

    def test_integers_beyond_python_decimal_guard(self):
        guard = sys.get_int_max_str_digits()
        matrix = synthetic_matrix(0)
        matrix[0][0] = [10 ** 6000 + 7, -(10 ** 5000 + 1), 1]
        matrix[0][1] = [1, 0, 10 ** 4500 + 3]
        doc = store.cross_on(session(), {"kind": "word", "word": []}, IDENTITIES,
                             state([{"k": 0}, {"m": matrix}], [1]))
        restored = self.roundtrip(doc)
        self.assertEqual(restored, doc)
        self.assertEqual(sys.get_int_max_str_digits(), guard)
        # This is a JSON number, not a quoted decimal string.
        self.assertIn(b"[1" + b"0" * 5999 + b"7,", self.path.read_bytes())
        restored_state = restored["jumbling"]["episodes"][0]["state"]
        self.assertEqual(store.canonical_digest(restored_state["pose_table"], [1], synthetic_matrix),
                         restored_state["digest"])

    def test_q5_invalid_values(self):
        invalid = ([1.0, 0, 1], [True, 0, 1], [1, False, 1], [1, 0, True],
                   ["1", 0, 1], [1, 0, 0], [1, 0, -1], [2, 4, 6],
                   [0, 0, 2], [1, 2], (1, 0, 1), None)
        for value in invalid:
            with self.subTest(value=value), self.assertRaises(store.StoreFormatError):
                store.check_q5(value)

    def test_matrix_dimensions_and_pose_table(self):
        for matrix in ([], synthetic_matrix(0)[:3], [[0] * 4] * 4):
            with self.subTest(matrix=matrix), self.assertRaises(store.StoreFormatError):
                store.check_matrix(matrix)
        for table in ([], [{"k": 1}], [{"k": False}], [{"k": 0}, {"k": 7200}],
                      [{"k": 0}, {"k": -1}], [{"k": 0}, {"k": 1}, {"k": 1}],
                      [{"k": 0}, {"m": synthetic_matrix(1)}, {"m": synthetic_matrix(1)}],
                      [{"k": 0}, {"k": 1, "m": synthetic_matrix(1)}]):
            with self.subTest(table=table), self.assertRaises(store.StoreFormatError):
                store.canonical_digest(table, [], synthetic_matrix)

    def test_full_size_ids_roundtrip(self):
        ids = [i % 321 for i in range(177_120)]
        encoded = store.encode_ids(ids)
        self.assertEqual(encoded["count"], 177_120)
        raw = base64.b64decode(encoded["data"])
        self.assertEqual(raw[:12], b"\x00\x00\x00\x00\x01\x00\x00\x00\x02\x00\x00\x00")
        self.assertEqual(encoded["sha256"], hashlib.sha256(raw).hexdigest())
        self.assertEqual(store.decode_ids(encoded, 321), ids)
        self.assertEqual(store.decode_ids(store.encode_ids([]), 0), [])

    def test_ids_reject_corrupt_count_checksum_and_range(self):
        encoded = store.encode_ids([0, 1, 2])
        for key, value in (("count", 4), ("count", True), ("sha256", "0" * 64),
                           ("sha256", "g" * 64), ("data", "!"), ("encoding", "other")):
            broken = dict(encoded, **{key: value})
            with self.subTest(key=key, value=value), self.assertRaises(store.StoreFormatError):
                store.decode_ids(broken, 3)
        with self.assertRaisesRegex(store.StoreFormatError, "outside the table"):
            store.decode_ids(encoded, 2)
        for value in (-1, True, 1.0, 2 ** 32):
            with self.subTest(value=value), self.assertRaises(store.StoreFormatError):
                store.encode_ids([value])

    def test_digest_three_pieces_hand_computed(self):
        first, second = synthetic_matrix(1), synthetic_matrix(2)
        # Step 1: Python's default JSON spacing, not the compact file encoding.
        first_key = json.dumps(first)
        second_key = json.dumps(second)
        # Step 2: sorted piece numbers for each pose, as signed int32 LE.
        first_ids = (0).to_bytes(4, "little", signed=True) + (2).to_bytes(4, "little", signed=True)
        second_ids = (1).to_bytes(4, "little", signed=True)
        # Step 3: sort (matrix key, member bytes), then hash each key and bytes.
        expected = hashlib.sha256()
        for key, members in sorted([(first_key, first_ids), (second_key, second_ids)]):
            expected.update(key.encode("utf-8"))
            expected.update(members)
        table = [{"k": 0}, {"m": first}, {"k": 2}]
        self.assertEqual(store.canonical_digest(table, [1, 2, 1], synthetic_matrix), expected.hexdigest())

    def test_digest_table_order_and_piece_changes(self):
        a, b = {"k": 4}, {"m": synthetic_matrix(8)}
        before = store.canonical_digest([{"k": 0}, a, b], [1, 2, 1], synthetic_matrix)
        reordered = store.canonical_digest([{"k": 0}, b, a], [2, 1, 2], synthetic_matrix)
        changed = store.canonical_digest([{"k": 0}, a, b], [0, 2, 1], synthetic_matrix)
        self.assertEqual(before, reordered)
        self.assertNotEqual(before, changed)
        self.assertEqual(store.canonical_digest([{"k": 0}], [0, 0, 0], synthetic_matrix),
                         hashlib.sha256(b"").hexdigest())

    def test_digest_only_resolves_used_nonzero_poses(self):
        calls = []

        def lookup(index):
            calls.append(index)
            return synthetic_matrix(index)

        store.canonical_digest([{"k": 0}, {"k": 1}, {"k": 2}], [0, 1, 1], lookup)
        self.assertEqual(calls, [1])

    def test_digest_rejects_repeated_or_identity_used_poses(self):
        # J1 interns each matrix once and keeps the identity at pose 0.
        repeated = [{"k": 0}, {"k": 3}, {"m": synthetic_matrix(3)}]
        with self.assertRaisesRegex(store.StoreFormatError, r"pose_table\[2\]"):
            store.canonical_digest(repeated, [1, 2], synthetic_matrix)
        identity = [{"k": 0}, {"m": synthetic_matrix(0)}]
        with self.assertRaisesRegex(store.StoreFormatError, r"pose_table\[1\]"):
            store.canonical_digest(identity, [0, 1], synthetic_matrix)
        # An unused repeat is not hashed; J1's table also keeps unused poses.
        store.canonical_digest(repeated, [1, 1], synthetic_matrix)

    def test_full_size_mixed_pose_state_roundtrip_digest(self):
        table = [{"k": 0}]
        for i in range(1, 321):
            if i % 3 == 0:
                table.append({"k": i})
            else:
                matrix = synthetic_matrix(i + 10_000)
                matrix[2][3] = [10 ** 450 + i, -i, 1]
                table.append({"m": matrix})
        ids = [(i * 17) % len(table) for i in range(177_120)]
        original = state(table, ids)
        doc = store.cross_on(session(), {"kind": "word", "word": []}, IDENTITIES, original)
        restored = self.roundtrip(doc)["jumbling"]["episodes"][0]["state"]
        decoded = store.decode_ids(restored["pose_ids"], len(restored["pose_table"]))
        self.assertEqual(decoded, ids)
        self.assertEqual(store.canonical_digest(restored["pose_table"], decoded, synthetic_matrix),
                         original["digest"])

    def test_all_fixture_journal_records_byte_exact_roundtrip(self):
        records = FIXTURE["journal"]["records"]
        self.assertGreater(len(records), 0)
        for record in records:
            store.check_record(record)
        doc = opened()
        doc["jumbling"]["episodes"][0]["journal"] = copy.deepcopy(records)
        restored = self.roundtrip(doc)["jumbling"]["episodes"][0]["journal"]
        for before, after in zip(records, restored):
            self.assertEqual(json.dumps(before, sort_keys=True, separators=(",", ":")).encode(),
                             json.dumps(after, sort_keys=True, separators=(",", ":")).encode())

    def test_journal_rejects_float_matrix_and_bad_fields(self):
        record = copy.deepcopy(FIXTURE["journal"]["records"][0])
        record["matrix"][0][0][0] = 1.0
        with self.assertRaisesRegex(store.StoreFormatError, r"matrix\[0\]\[0\]\[0\]"):
            store.check_record(record)
        for key, value in (("grip", True), ("grip", -1), ("grip", 600),
                           ("moved", -1), ("moved", True)):
            record = copy.deepcopy(FIXTURE["journal"]["records"][0])
            record[key] = value
            with self.subTest(key=key, value=value), self.assertRaises(store.StoreFormatError):
                store.check_record(record)

    def test_never_on_session_bytes_match_baseline_writer(self):
        doc = session()
        # Baseline writer knows exactly the retained-only session fields.
        baseline = {"format": doc["format"], "revision": doc["revision"], "retained": doc["retained"]}
        expected = (json.dumps(baseline, sort_keys=True, separators=(",", ":"),
                               ensure_ascii=False) + "\n").encode("utf-8")
        restored = self.roundtrip(doc)
        self.assertNotIn("jumbling", restored)
        self.assertEqual(self.path.read_bytes(), expected)
        self.assertTrue(expected.endswith(b"\n"))
        self.assertFalse(expected.endswith(b"\n\n"))

    def test_open_journal_and_witness_close(self):
        doc = opened()
        self.assertIsNone(self.roundtrip(doc)["jumbling"]["episodes"][0]["end"])
        doc["jumbling"]["episodes"][0]["journal"] = copy.deepcopy(FIXTURE["journal"]["records"][:2])
        doc["jumbling"]["episodes"][0]["end"] = {"kind": "witness", "word": [1, -2, 1200]}
        doc["revision"] += 1
        self.assertEqual(self.roundtrip(doc), doc)

    def test_word_label_exit_then_resume_roundtrip(self):
        doc = opened()
        exact_exit = exit_state(doc)
        closed = store.cross_off(doc, certificate(doc), exact_exit, {"labels": "new"})
        closed = self.roundtrip(closed)
        self.assertEqual(closed["jumbling"]["episodes"][0]["end"]["exit_state"], exact_exit)
        resumed = store.cross_on(closed, {"kind": "exit", "episode": 0, "retained_moves": [-1200, 2]},
                                 IDENTITIES, state())
        restored = self.roundtrip(resumed)
        self.assertEqual(restored["jumbling"]["episodes"][1]["start"]["episode"], 0)
        self.assertIsNone(restored["jumbling"]["episodes"][1]["end"])

    def test_exit_requires_an_earlier_label_end(self):
        doc = opened()
        doc["jumbling"]["episodes"][0]["end"] = {"kind": "witness", "word": []}
        with self.assertRaisesRegex(store.StoreFormatError, "start.episode"):
            store.cross_on(doc, {"kind": "exit", "episode": 0, "retained_moves": []}, IDENTITIES, state())
        closed = store.cross_off(opened(), certificate(opened()), exit_state(opened()), None)
        for index in (-1, 1, 100, True):
            with self.subTest(index=index), self.assertRaises(store.StoreFormatError):
                store.cross_on(closed, {"kind": "exit", "episode": index, "retained_moves": []},
                               IDENTITIES, state())

    def test_label_certificate_requires_all_zero_nu(self):
        doc = opened()
        for index in (0, 34, 35, 42, 43, 44):
            bad = certificate(doc)
            bad["nu"][index] = 1
            with self.subTest(index=index), self.assertRaisesRegex(store.StoreFormatError, "nu"):
                store.cross_off(doc, bad, exit_state(doc), None)
        for nu in ([0] * 44, [0] * 46, [False] + [0] * 44, [0] * 44 + [5]):
            bad = certificate(doc)
            bad["nu"] = nu
            with self.subTest(nu=nu), self.assertRaises(store.StoreFormatError):
                store.cross_off(doc, bad, exit_state(doc), None)

    def test_label_exit_digest_must_match_certificate_and_final_state(self):
        doc = opened()
        bad_exit = exit_state(doc)
        bad_exit["digest"] = "0" * 64
        with self.assertRaisesRegex(store.StoreFormatError, "exit_state.digest"):
            store.cross_off(doc, certificate(doc), bad_exit, None)
        bad_certificate = certificate(doc)
        bad_certificate["pose_digest"] = bad_exit["digest"]
        with self.assertRaisesRegex(store.StoreFormatError, "final state"):
            store.cross_off(doc, bad_certificate, bad_exit, None)

    def test_two_open_episodes_rejected(self):
        doc = opened()
        doc["jumbling"]["episodes"].append(copy.deepcopy(doc["jumbling"]["episodes"][0]))
        self.assertBad(doc, "$.jumbling.episodes[0].end")
        with self.assertRaisesRegex(store.StoreFormatError, "already open"):
            store.cross_on(opened(), {"kind": "word", "word": []}, IDENTITIES, state())

    def test_crossings_return_new_documents_without_aliases(self):
        original, start, poses = session(), {"kind": "word", "word": [1, -2]}, state()
        expected = copy.deepcopy(original)
        identities = copy.deepcopy(IDENTITIES)
        on = store.cross_on(original, start, identities, poses)
        self.assertEqual(original, expected)
        self.assertEqual(on["revision"], original["revision"] + 1)
        start["word"].append(3)
        poses["pose_table"].append({"k": 2})
        identities["model"].clear()
        self.assertEqual(on["jumbling"]["episodes"][0]["start"]["word"], [1, -2])
        self.assertEqual(len(on["jumbling"]["episodes"][0]["state"]["pose_table"]), 2)
        old_on = copy.deepcopy(on)
        cert, exited, labels = certificate(on), exit_state(on), {"labels": [1, 2]}
        off = store.cross_off(on, cert, exited, labels)
        self.assertEqual(on, old_on)
        self.assertEqual(off["revision"], on["revision"] + 1)
        cert["nu"][0] = 1
        exited["pose_table"].clear()
        labels["labels"].append(3)
        self.assertEqual(off["retained"], {"labels": [1, 2]})
        store.validate(off)
        with self.assertRaises(store.StoreFormatError):
            store.cross_off(off, certificate(on), exit_state(on), None)
        with self.assertRaises(store.StoreFormatError):
            store.cross_off(session(), certificate(on), exit_state(on), None)

    def test_word_672417_letters_roundtrip(self):
        letters = [i % (600 * len(MENU)) for i in range(672_417)]
        doc = session(section=True)
        doc["jumbling"]["words"].append(word_record(letters))
        restored = self.roundtrip(doc)["jumbling"]["words"][0]
        self.assertEqual(restored["letters"]["count"], 672_417)
        self.assertEqual(store.decode_ids(restored["letters"], 600 * len(MENU)), letters)
        self.assertEqual(restored["identities"], IDENTITIES)

    def test_parts_inverse_and_same_as_expand_from_ia_menu(self):
        root, letters, inverse = part_example()
        expanded = store.expand_parts(root, letters, inverse)
        self.assertEqual(expanded, letters)
        self.assertIsNot(expanded, letters)
        self.assertEqual(store.parts_length(root), len(expanded))
        doc = session(section=True)
        doc["jumbling"]["words"].append(word_record(letters, root))
        restored = self.roundtrip(doc)["jumbling"]["words"][0]
        self.assertEqual(store.expand_parts(restored["parts"],
                                           store.decode_ids(restored["letters"], 600 * len(MENU)),
                                           inverse), letters)

    def test_parts_bad_span_counts_inverse_same_as(self):
        root, letters, inverse = part_example()
        for mutate in (
            lambda r: r["parts"][0]["parts"][0].update(span=[1, 2]),
            lambda r: r["parts"][1].update(span=[2, 5]),
            lambda r: r["parts"][0].update(letters=4),
            lambda r: r["parts"][1].update(inverse_of=[2]),
            lambda r: r["parts"][1].update(inverse_of=[0, 1]),
            lambda r: r["parts"][2].update(name="another name"),
            lambda r: r["parts"][2].update(same_as=[True]),
            lambda r: r["parts"][2].update(inverse_of=[0]),
        ):
            broken = copy.deepcopy(root)
            mutate(broken)
            with self.subTest(broken=broken), self.assertRaises(store.StoreFormatError):
                store.expand_parts(broken, letters, inverse)
        for position, field in ((3, "inverse_of"), (8, "same_as")):
            broken = letters.copy()
            broken[position] = (broken[position] + 1) % (600 * len(MENU))
            with self.subTest(position=position), self.assertRaisesRegex(store.StoreFormatError, field):
                store.expand_parts(root, broken, inverse)

    def test_parts_nested_index_reference(self):
        root = {"name": "root", "letters": 2, "parts": [
            {"name": "group", "letters": 1, "parts": [
                {"name": "x", "letters": 1, "span": [0, 1]}]},
            {"name": "x", "letters": 1, "span": [1, 2], "same_as": [0, 0]},
        ]}
        self.assertEqual(store.expand_parts(root, [1, 1], [0, 1]), [1, 1])

    def test_parts_depth_64_allowed_65_rejected(self):
        tree = {"name": "leaf", "letters": 1, "span": [0, 1]}
        for _ in range(63):
            tree = {"name": "group", "letters": 1, "parts": [tree]}
        self.assertEqual(store.parts_length(tree), 1)
        self.assertEqual(store.expand_parts(tree, [0], [0]), [0])
        tree = {"name": "too deep", "letters": 1, "parts": [tree]}
        with self.assertRaisesRegex(store.StoreFormatError, "depth"):
            store.parts_length(tree)
        with self.assertRaisesRegex(store.StoreFormatError, "depth"):
            store.expand_parts(tree, [0], [0])
        doc = session(section=True)
        doc["jumbling"]["words"].append(word_record([0], tree))
        self.assertBad(doc, ".parts")

    def test_parts_length_without_expanding_unbounded_counts(self):
        count = 10 ** 450
        tree = {"name": "huge", "letters": count, "span": [0, count]}
        self.assertEqual(store.parts_length(tree), count)
        empty = {"name": "empty", "letters": 0, "parts": []}
        self.assertEqual(store.parts_length(empty), 0)
        self.assertEqual(store.expand_parts(empty, [], [0]), [])

    def test_word_validation_paths_and_same_as_content(self):
        root, letters, _ = part_example()
        letters[-1] = (letters[-1] + 1) % (600 * len(MENU))
        doc = session(section=True)
        doc["jumbling"]["words"].append(word_record(letters, root))
        self.assertBad(doc, ".same_as")
        doc["jumbling"]["words"][0] = word_record([600 * len(MENU)])
        self.assertBad(doc, "$.jumbling.words[0].letters.data[0]")
        doc["jumbling"]["words"][0] = word_record([0])
        doc["jumbling"]["words"][0]["menu_size"] = True
        self.assertBad(doc, "$.jumbling.words[0].menu_size")

    def test_scramble_separate_from_solver_journal(self):
        doc = opened()
        solver = copy.deepcopy(FIXTURE["journal"]["records"][:1])
        scramble = copy.deepcopy(FIXTURE["journal"]["records"][1:4])
        doc["jumbling"]["episodes"][0]["journal"] = solver
        doc["jumbling"]["scrambles"].append({"identities": copy.deepcopy(IDENTITIES),
                                             "journal": scramble, "seed": -7101})
        restored = self.roundtrip(doc)
        self.assertEqual(restored["jumbling"]["episodes"][0]["journal"], solver)
        self.assertEqual(restored["jumbling"]["scrambles"][0]["journal"], scramble)
        doc["jumbling"]["scrambles"][0].pop("seed")
        store.validate(doc)
        doc["jumbling"]["scrambles"][0]["seed"] = True
        self.assertBad(doc, "$.jumbling.scrambles[0].seed")

    def test_validate_field_paths_and_json_types(self):
        for key, value in (("revision", True), ("revision", -1), ("format", "future"),
                           ("jumbling", None), ("retained", {"bad": float("nan")}),
                           ("retained", (1, 2)), ("retained", {1: "bad"}),
                           ("retained", "\ud800")):
            doc = session()
            doc[key] = value
            with self.subTest(key=key, value=value):
                self.assertBad(doc, "$." + key)
        doc = opened()
        doc["jumbling"]["episodes"][0]["identities"]["menu_order"] = "not a digest"
        self.assertBad(doc, "$.jumbling.episodes[0].identities.menu_order")
        doc = opened()
        doc["jumbling"]["episodes"][0]["identities"]["menu"] = "A" * 64
        self.assertBad(doc, "$.jumbling.episodes[0].identities.menu")
        doc = opened()
        doc["jumbling"]["episodes"][0]["state"].pop("pose_ids")
        self.assertBad(doc, "$.jumbling.episodes[0].state.pose_ids")
        doc = session()
        doc["retained"] = doc
        self.assertBad(doc, "$.retained")

    def test_loads_rejects_malformed_duplicate_and_nonfinite_json(self):
        for data in (b'{"format":', b"\xff", b"[]", b'{"revision": 0, "revision": 1}',
                     b'{"format":"magic600-session-proto/1","revision":0,"retained":NaN}',
                     b'{"format":"magic600-session-proto/1","revision":0,"retained":1e999}'):
            with self.subTest(data=data), self.assertRaises(store.StoreFormatError):
                store.loads(data)

    def test_write_validates_before_any_mutation(self):
        previous = session()
        store.write_session(self.path, previous)
        before = self.path.read_bytes()
        broken = opened()
        broken["jumbling"]["episodes"][0]["journal"] = [{"grip": 0}]
        with self.assertRaises(store.StoreFormatError):
            store.write_session(self.path, broken)
        self.assertEqual(self.path.read_bytes(), before)
        self.assertEqual(list(self.directory.iterdir()), [self.path])

    def test_write_temp_location_and_flush_fsync_replace_order(self):
        events = []
        real_fsync, real_replace, real_mkstemp = os.fsync, os.replace, tempfile.mkstemp

        def synced(fd):
            events.append("fsync")
            return real_fsync(fd)

        def replaced(source, destination):
            self.assertEqual(Path(source).parent, self.path.parent)
            # Reading here proves buffered file bytes reached the temp file
            # before replacement; fsync must also already have been called.
            self.assertEqual(store.loads(Path(source).read_bytes()), session())
            self.assertIn("fsync", events)
            events.append("replace")
            return real_replace(source, destination)

        with mock.patch.object(store.os, "fsync", side_effect=synced), \
                mock.patch.object(store.os, "replace", side_effect=replaced), \
                mock.patch.object(store.tempfile, "mkstemp", wraps=real_mkstemp) as made:
            store.write_session(self.path, session())
        self.assertEqual(Path(made.call_args.kwargs["dir"]), self.directory)
        # Windows cannot open a directory, so the directory fsync is skipped there.
        self.assertEqual(events, ["fsync", "replace"] + ([] if os.name == "nt" else ["fsync"]))

    def test_crash_each_write_point_recovers_only_committed_revision(self):
        old = opened()
        new = copy.deepcopy(old)
        new["revision"] += 1
        new["retained"] = {"labels": "new"}
        for point in CRASH_POINTS:
            with self.subTest(point=point):
                store.write_session(self.path, old)
                with self.assertRaisesRegex(store.SimulatedCrash, point):
                    store.write_session(self.path, new, crash_at=point)
                if point != "after_replace":
                    self.assertEqual(len(list(self.directory.iterdir())), 2)
                restored = store.read_session(self.path)
                store.validate(restored)
                self.assertEqual(restored, new if point == "after_replace" else old)
                self.assertEqual(list(self.directory.iterdir()), [self.path])

    def test_recovery_ignores_truncated_invalid_and_valid_newer_temps(self):
        old = opened()
        store.write_session(self.path, old)
        prefix = f".{self.path.name}.session-"
        (self.directory / (prefix + "truncated.tmp")).write_bytes(b'{"format":')
        (self.directory / (prefix + "invalid.tmp")).write_bytes(b'{"format":"wrong"}')
        newer = copy.deepcopy(old)
        newer["revision"] += 1
        (self.directory / (prefix + "newer.tmp")).write_bytes(store.dumps(newer))
        other_path = self.directory / ".another.json.session-keep.tmp"
        other_path.write_bytes(b"unrelated")
        self.assertEqual(store.read_session(self.path), old)
        self.assertEqual(set(self.directory.iterdir()), {self.path, other_path})

    def test_recovery_never_promotes_temp_when_commit_is_missing_or_invalid(self):
        doc = opened()
        with self.assertRaises(store.SimulatedCrash):
            store.write_session(self.path, doc, crash_at="before_replace")
        with self.assertRaises(FileNotFoundError):
            store.read_session(self.path)
        self.assertEqual(list(self.directory.iterdir()), [])
        self.path.write_bytes(b"truncated committed document")
        temp = self.directory / f".{self.path.name}.session-complete.tmp"
        temp.write_bytes(store.dumps(doc))
        with self.assertRaises(store.StoreFormatError):
            store.read_session(self.path)
        self.assertEqual(list(self.directory.iterdir()), [self.path])

    def test_cross_off_crashes_are_atomic(self):
        on = opened()
        labels = {"labels": "after crossing", "preferences": on["retained"]["preferences"]}
        off = store.cross_off(on, certificate(on), exit_state(on), labels)
        for point in CRASH_POINTS:
            with self.subTest(point=point):
                store.write_session(self.path, on)
                with self.assertRaises(store.SimulatedCrash):
                    store.write_session(self.path, off, crash_at=point)
                restored = store.read_session(self.path)
                store.validate(restored)
                if restored["jumbling"]["episodes"][-1]["end"] is None:
                    self.assertEqual(restored, on)
                    self.assertEqual(restored["retained"], on["retained"])
                else:
                    self.assertEqual(restored, off)
                    self.assertEqual(restored["retained"], labels)
                self.assertEqual(list(self.directory.iterdir()), [self.path])

    def test_cross_on_crashes_keep_optional_section_atomic(self):
        before = session()
        after = store.cross_on(before, {"kind": "word", "word": []}, IDENTITIES, state())
        for point in CRASH_POINTS:
            with self.subTest(point=point):
                store.write_session(self.path, before)
                with self.assertRaises(store.SimulatedCrash):
                    store.write_session(self.path, after, crash_at=point)
                restored = store.read_session(self.path)
                self.assertEqual(restored, after if point == "after_replace" else before)
                if "jumbling" in restored:
                    self.assertIsNone(restored["jumbling"]["episodes"][-1]["end"])
                self.assertEqual(list(self.directory.iterdir()), [self.path])


def _audit_temp_writes(event, args):
    if _WRITE_ROOT is None:
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
            continue  # fdopen/fsync use a descriptor already opened under audit.
        path = Path(os.fsdecode(path))
        if not path.is_absolute() and directory_fd not in (None, -1):
            # shutil.rmtree uses fd-relative deletion on supported platforms.
            # Resolve that fd by inode using only directories under our root.
            status = os.fstat(directory_fd)
            root = Path(_WRITE_ROOT)
            for directory in [root] + [p for p in root.rglob("*") if p.is_dir()]:
                candidate = directory.stat()
                if (status.st_dev, status.st_ino) == (candidate.st_dev, candidate.st_ino):
                    path = directory / path
                    break
            else:
                raise AssertionError("test used a directory fd outside its TemporaryDirectory")
        if not path.resolve().is_relative_to(Path(_WRITE_ROOT).resolve()):
            raise AssertionError("test attempted a write outside its TemporaryDirectory")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory(prefix="jumbling-store-tests-") as temporary_root:
        _WRITE_ROOT = temporary_root
        sys.addaudithook(_audit_temp_writes)
        result = unittest.main(verbosity=2, exit=False).result
    _WRITE_ROOT = None
    raise SystemExit(not result.wasSuccessful())
