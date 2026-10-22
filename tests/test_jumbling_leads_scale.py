"""EN2-H bounded scale leads: exact heights, exposed meshes and whole-word X3.

One shared J1 context; runtime writes are audited and confined outside the
repository. Shared unittest invocations use isolated_numpy so the differential
harness's import-independence checks remain NumPy-free. Measurements remain in
_CLASS_SECONDS and _MEASUREMENTS for an external reporting runner; success is
silent apart from unittest's summary.
"""

import sys

sys.dont_write_bytecode = True

from contextlib import redirect_stdout
from copy import copy
import io
import json
import os
from pathlib import Path
import tempfile
from time import perf_counter
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
_IN_PROCESS = __name__ == "__main__"
_TEMP_ROOT = None
_AUDIT_ACTIVE = False
_CLASS_SECONDS = {}
_MEASUREMENTS = {}
_BLOCKED = None


def _audit_writes(event, args):
    if not _AUDIT_ACTIVE:
        return
    paths = []
    if event == "open" and (args[2] or 0) & (os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC | os.O_APPEND):
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
        raise AssertionError("scale tests must not create links")
    for value, directory_fd in paths:
        if isinstance(value, int):
            continue
        path = Path(os.fsdecode(value))
        if not path.is_absolute() and directory_fd not in (None, -1):
            status = os.fstat(directory_fd)
            # tempfile's fd-based cleanup also goes through the audit hook.
            for directory in [_TEMP_ROOT] + [p for p in _TEMP_ROOT.rglob("*") if p.is_dir()]:
                candidate = directory.stat()
                if (status.st_dev, status.st_ino) == (candidate.st_dev, candidate.st_ino):
                    path = directory / path
                    break
            else:
                raise AssertionError("unscoped directory-fd write")
        if path.resolve().is_relative_to(ROOT):
            raise AssertionError("scale test attempted a runtime write inside the repository")


def setUpModule():
    global _TEMP_ROOT, _AUDIT_ACTIVE, CTX, np, sim, heights, meshes, x3, j1_state, census
    temporary = tempfile.TemporaryDirectory(prefix="jumbling-leads-scale-")
    _TEMP_ROOT = Path(temporary.name).resolve()
    if _TEMP_ROOT.is_relative_to(ROOT):
        temporary.cleanup()
        raise AssertionError("test tempfile root must be outside the repository")

    def cleanup():
        global _AUDIT_ACTIVE
        temporary.cleanup()
        _AUDIT_ACTIVE = False

    unittest.addModuleCleanup(cleanup)
    _AUDIT_ACTIVE = True
    sys.addaudithook(_audit_writes)
    try:
        import numpy as np
    except ImportError as error:
        raise unittest.SkipTest("NumPy is required for the J1 scale leads") from error
    sys.path.insert(0, str(ROOT / "research/jumbling"))
    import sim
    from differential import j1_state
    from differential.leads import census, heights, meshes, x3_previews as x3
    census.isolate_cache(_TEMP_ROOT)
    started = perf_counter()
    CTX = sim.get_context()
    _MEASUREMENTS["context_seconds"] = perf_counter() - started


def _snapshot(state):
    result = copy(state)
    result.pose_id = state.pose_id.copy()
    result._poses = list(state._poses)
    result._lookup = dict(state._lookup)
    result.journal = list(state.journal)
    result._anchor_cache = {}
    return result


def _independent_height(state):
    """Parse the public plain-data export; do not call the measured helper."""
    record = j1_state.state_record(state)
    entries = [triple for pose in record["pose_table"] if "m" in pose
               for row in pose["m"] for triple in row]
    return (max((max(abs(a), abs(b), d) for a, b, d in entries), default=0),
            max((d for _, _, d in entries), default=0))


def _blocked_word():
    """Find a blocked grip by exact trial classification in a one-letter state."""
    global _BLOCKED
    if _BLOCKED is None:
        menu = census.menu()
        item = next(i for i, t in enumerate(menu.for_cap(0)) if not t.retained)
        first = [0, item]
        state = sim.State(CTX, menu=menu)
        if not state.apply(j1_state.twists_from_letters(menu, [first])[0], journal=False).applied:
            raise AssertionError("the planted word's first letter must be admitted")
        for grip in range(600):
            cl = state.classify(grip, need_inside=False, stop_at_straddle=True)
            if cl.status == "blocked":
                cap = menu.for_cap(grip)
                blocked_item = next(i for i, twist in enumerate(cap) if not twist.is_identity)
                before = state.snapshot()
                outcome = state.apply(cap[blocked_item], journal=False)
                if outcome.status != "blocked" or state.snapshot() != before:
                    raise AssertionError("the planted letter was not a transactional refusal")
                _BLOCKED = ([first, [grip, blocked_item], first], _snapshot(state))
                break
        else:
            raise AssertionError("could not find a blocked letter in the one-letter state")
    return _BLOCKED


class _TimedCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._started = perf_counter()

    @classmethod
    def tearDownClass(cls):
        _CLASS_SECONDS[cls.__name__] = perf_counter() - cls._started


class HeightsTests(_TimedCase):
    def test_h6030_width_crossings_even_between_samples(self):
        source = heights.h6030(ctx=CTX).prefix(210)
        result = heights.run_heights(source.state, source, label=source.label, every=12)
        self.assertEqual(result["status"], "complete")
        self.assertEqual(result["steps"], 210)
        self.assertEqual(result["crossings"], {
            "31": {"step": 170, "bits": 32}, "53": {"step": 192, "bits": 54},
            "63": {"step": 202, "bits": 64}, "64": {"step": 203, "bits": 65}})
        self.assertEqual(next(r for r in result["rows"] if r["step"] == 204)["height_bits"], 66)
        self.assertEqual(result["rows"][-1]["step"], 210)
        self.assertEqual(result["rows"][0]["max_entry_height"], 0)
        self.assertGreater(result["seconds"]["replay"], 0)
        self.assertGreater(result["seconds"]["bookkeeping"], 0)
        self.assertTrue(result["checks_passed"])
        self.assertIsNone(result["stage_checks"][0]["passed"])  # Full end deliberately not run.
        _MEASUREMENTS["h6030_210"] = {k: result[k] for k in ("seconds", "crossings", "maxima")}

    def test_fixture_60_every_value_matches_independent_export(self):
        source = heights.fixture("wj-I_a", ctx=CTX).prefix(60)
        independent = []

        def observed_twists():
            independent.append(_independent_height(source.state))
            for twist in source:
                yield twist
                independent.append(_independent_height(source.state))

        result = heights.run_heights(source.state, observed_twists(), label=source.label)
        self.assertEqual(result["steps"], 60)
        self.assertEqual(len(result["rows"]), 61)
        self.assertEqual([(r["max_entry_height"], r["max_denominator"]) for r in result["rows"]], independent)
        self.assertEqual(independent[0], (0, 0))
        self.assertEqual(source.state.digest(), "ec3acb21fe14da213680d2b2e83a04d2f74dc8f77fa6dffeac4fd495a60b6e0a")

    def test_negative_control_all_digests_and_centre_heights(self):
        source = heights.journal("negative_control", ctx=CTX)
        result = heights.run_heights(source.state, source, label=source.label)
        self.assertEqual(result["steps"], 24)
        self.assertTrue(result["checks_passed"])
        self.assertEqual([r["digest"] for r in result["rows"]], source.digests)
        self.assertEqual([r["cap_0_pose_entry_height"] for r in result["rows"] if r["step"] and r["step"] % 2 == 0], source.cap0_heights)
        self.assertEqual(len(result["digest_checks"]), 25)
        self.assertEqual(len(result["cap0_height_checks"]), 12)
        self.assertTrue(all(r["passed"] for r in result["digest_checks"] + result["cap0_height_checks"]))

    def test_width_wrapper_restored_on_success_and_error(self):
        original = sim.kernel.sign5
        source = heights.h6030(ctx=CTX).prefix(6)
        result = heights.run_heights(source.state, source, label="widths", every=4, widths=True)
        self.assertIs(sim.kernel.sign5, original)
        self.assertEqual(len(result["operand_widths"]), 6)
        self.assertGreater(result["max_sign_operand_bits"], 0)
        self.assertEqual(result["max_sign_operand_bits"], max(r["max_operand_bits"] for r in result["operand_widths"]))
        self.assertTrue(all("sign_operand_bits" in row for row in result["rows"][1:]))
        failed = heights.h6030(ctx=CTX).prefix(3)
        apply = failed.state.apply
        calls = []

        def planted_error(*args, **kwargs):
            calls.append(1)
            if len(calls) == 2:
                self.assertIsNot(sim.kernel.sign5, original)
                raise RuntimeError("planted apply error")
            return apply(*args, **kwargs)

        with patch.object(failed.state, "apply", side_effect=planted_error):
            with self.assertRaisesRegex(RuntimeError, "planted apply error"):
                heights.run_heights(failed.state, failed, label="error", widths=True)
        self.assertEqual(len(calls), 2)
        self.assertIs(sim.kernel.sign5, original)

    def test_stage_mismatch_names_the_stage(self):
        source = heights.fixture("I_a", ctx=CTX).prefix(1)
        source.stages["start"]["max_entry_height"] += 1
        result = heights.run_heights(source.state, source, label="planted")
        self.assertFalse(result["checks_passed"])
        check = next(c for c in result["stage_checks"] if c["stage"] == "start")
        self.assertFalse(check["passed"])
        self.assertFalse(check["fields"]["max_entry_height"]["passed"])
        self.assertTrue(check["fields"]["max_denominator"]["passed"])
        empty = heights.fixture("I_a", ctx=CTX).prefix(0)
        result = heights.run_heights(empty.state, empty, label="start")
        self.assertEqual((result["rows"][0]["max_entry_height"], result["rows"][0]["max_denominator"]), (0, 0))
        self.assertTrue(next(c for c in result["stage_checks"] if c["stage"] == "start")["passed"])

    def test_other_inputs_and_first_refusal(self):
        for name in ("S4", "I_b"):
            with self.subTest(fixture=name):
                source = heights.fixture(name, ctx=CTX).prefix(1)
                result = heights.run_heights(source.state, source, label=name)
                self.assertTrue(result["checks_passed"])
                self.assertTrue(next(c for c in result["stage_checks"] if c["stage"] == "start")["passed"])
        for name in ("E2", "E3", "random_0", "random_1"):
            with self.subTest(journal=name):
                source = heights.journal(name, ctx=CTX)
                self.assertIsNone(source.state.menu)
                self.assertEqual(heights.run_heights(source.state, source, label=name)["steps"], len(source))
        letters, end = _blocked_word()
        state = sim.State(CTX, menu=census.menu())
        result = heights.run_heights(state, j1_state.twists_from_letters(state.menu, letters), label="blocked", every=5)
        self.assertEqual(result["refused"], {"index": 1, "step": 2, "status": "blocked"})
        self.assertEqual(result["steps"], 1)
        self.assertEqual(result["rows"][-1]["step"], 2)
        self.assertEqual(state.digest(), end.digest())


class MeshesTests(_TimedCase):
    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        cls.faces = meshes.orbit_faces(CTX)
        cls.report = meshes.summary(cls.faces, CTX.regions.report["orbit_sizes"])

    def test_exact_faces_euler_hosts_and_totals(self):
        self.assertEqual(len(self.faces), 36)
        self.assertEqual(sum(self.report["orbit_sizes"]), 177120)
        for row in self.faces:
            self.assertEqual(sorted(f["pole"] for f in row["host_faces"]), CTX.regions.rep_hosts[row["orbit"]])
            for face in row["cut_faces"] + row["host_faces"]:
                self.assertTrue(face["euler_ok"])
                self.assertEqual(face["V"] - face["E"] + face["F"], 2)
                self.assertEqual(face["E"] * 2, sum(f["V"] for f in face["faces2"]))
                self.assertEqual(face["triangles"], sum(f["V"] - 2 for f in face["faces2"]))
                self.assertEqual(face["triangles"], 2 * face["V"] - 4)
                self.assertEqual(len({tuple(f["vertex_ids"]) for f in face["faces2"]}), face["F"])
        full = self.report["full"]
        self.assertEqual(full["cut"]["faces3"], 939000)
        self.assertEqual(full["cut"]["triangles"], 4322400)
        self.assertEqual(full["host"]["faces3"], 259800)
        self.assertEqual(full["host"]["triangles"], 1437600)
        self.assertEqual(full["in"]["faces3"] + full["out"]["faces3"], full["cut"]["faces3"])
        self.assertEqual(self.report["retained_stickers"]["base_triangles"], 10160)
        self.assertEqual(self.report["retained_stickers"]["full_triangles"], 6096000)
        _MEASUREMENTS["meshes"] = {k: self.report[k] for k in ("base", "full", "base_mesh", "full_mesh")}

    def test_build_base_mesh_matches_the_exact_count(self):
        mesh = meshes.build_base_mesh(self.faces)
        self.assertEqual(mesh["triangle_count"], self.report["base"]["cut"]["triangles"])
        self.assertEqual(mesh["vertices"].shape, (mesh["triangle_count"] * 3, 4))
        self.assertEqual(mesh["vertices"].dtype, np.dtype("<f4"))
        self.assertEqual(mesh["bytes"], self.report["base_mesh"]["cut_bytes"])
        self.assertEqual(mesh["offsets"][-1], len(mesh["vertices"]))
        self.assertEqual(len(mesh["offsets"]), self.report["base"]["cut"]["faces3"] + 1)
        self.assertTrue(np.isfinite(mesh["vertices"]).all())
        for begin, end, face in zip(mesh["offsets"], mesh["offsets"][1:], mesh["faces"]):
            self.assertEqual((end - begin) % 3, 0)
            self.assertIn(face["side"], ("in", "out"))

    def test_all_433_stickers_explain_the_retained_difference(self):
        result = meshes.sticker_comparison(CTX)
        self.assertEqual(result["sticker_count"], 433)
        self.assertEqual([r["sticker"] for r in result["stickers"]], list(range(433)))
        self.assertEqual(result["totals"]["exact_triangles"] * 600, 1437600)
        self.assertEqual(result["totals"]["retained_triangles"] * 600, 6096000)
        self.assertGreater(result["differing_stickers"], 0)
        self.assertGreater(result["largest_ratio"], 1)
        self.assertGreater(result["totals"]["corners_not_float32_exact_vertices_1e9"], 0)
        self.assertGreater(result["totals"]["collapsed_triangles_float32"], 0)
        self.assertGreaterEqual(result["totals"]["corners_not_exact_vertices_1e9"], result["totals"]["corners_not_float32_exact_vertices_1e9"])
        _MEASUREMENTS["sticker_comparison"] = {k: result[k] for k in ("differing_stickers", "largest_ratio", "totals", "explanation")}


class X3PreviewTests(_TimedCase):
    def test_u1_from_checked_census_end_and_exact_effect(self):
        started = perf_counter()
        state = x3.start_state(("census", "7101"), ctx=CTX)
        _MEASUREMENTS["u1_start_seconds"] = perf_counter() - started
        result = x3.preview_word(state, x3.load_word(x3.U1_PATH), label="u1")
        self.assertEqual(result["status"], "complete")
        self.assertEqual(result["expanded_length"], 624)
        self.assertEqual(result["steps"], 624)
        self.assertEqual(len(result["letters"]), 624)
        self.assertEqual(result["start_digest"], "7e128820541abc82e45c683db92f38413acba6e282c00f3185585e507b4e6818")
        self.assertEqual(result["end_digest"], "ce7ddaaf5c29489d1fd604e1becbc80e0368f9fccb4925355ea26c5b8c16355c")
        self.assertEqual((result["off_before"], result["off_after"]), (1193, 1121))
        self.assertEqual(result["off_pieces_landed"], 72)
        self.assertEqual(result["lattice_pieces_lifted_off"], 0)
        self.assertEqual(result["other_off_exact_pose_unchanged"], 1121)
        self.assertEqual(result["other_off_exact_pose_changed"], 0)
        self.assertTrue(all(c["passed"] for c in result["checks"]))
        self.assertEqual(result["samples"][-1]["step"], 624)
        self.assertNotIn("classify", state.__dict__)
        _MEASUREMENTS["u1_preview"] = result["seconds"]

    def test_correction35_and_h6030_200_letter_previews(self):
        for source in (heights.correction35(ctx=CTX, limit=200), heights.h6030(ctx=CTX).prefix(200)):
            with self.subTest(word=source.label):
                result = x3.preview(source.state, source.letters, source.state.menu,
                                    label=source.label, sample_every=75)
                self.assertEqual(result["status"], "complete")
                self.assertEqual(result["steps"], 200)
                self.assertEqual(result["expanded_length"], 200)
                self.assertEqual([r["step"] for r in result["letters"]], list(range(1, 201)))
                self.assertEqual([r["step"] for r in result["samples"]], [0, 75, 150, 200])
                self.assertGreater(sum(r["exact_evaluations"] for r in result["letters"]), 0)
                self.assertTrue(all(r["filtered_evaluations"] == 0 for r in result["letters"]))
                self.assertGreater(result["seconds"]["median"], 0)
                self.assertLessEqual(result["seconds"]["p95"], result["seconds"]["largest"])

    def test_blocked_prefix_stops_before_following_letter(self):
        letters, expected = _blocked_word()
        state = sim.State(CTX, menu=census.menu())
        with patch.object(state, "apply", wraps=state.apply) as apply:
            result = x3.preview(state, letters, state.menu, label="blocked", sample_every=10)
        self.assertEqual(apply.call_count, 2)
        self.assertEqual(result["status"], "refused")
        self.assertEqual(result["refused"], {"index": 1, "step": 2, "status": "blocked"})
        self.assertEqual(result["expanded_length"], 3)
        self.assertEqual(result["steps"], 1)
        self.assertEqual(result["letters"][-1]["moved"], 0)
        self.assertEqual(result["end_digest"], expected.digest())
        self.assertEqual(state.snapshot(), expected.snapshot())

    def test_input_digest_mismatch_is_refused_before_apply(self):
        with tempfile.TemporaryDirectory(dir=_TEMP_ROOT) as directory:
            path = Path(directory) / "wrong-start.json"
            path.write_text(json.dumps({"word": [[0, 1]], "input_digest": "f" * 64}), encoding="utf-8")
            doc = x3.load_word(path)
            state = sim.State(CTX, menu=census.menu())
            before = state.snapshot()
            with patch.object(state, "apply", wraps=state.apply) as apply:
                result = x3.preview_word(state, doc, label="wrong-start")
            self.assertEqual(apply.call_count, 0)
            self.assertEqual(result["status"], "refused")
            self.assertEqual(result["refused"]["status"], "input_digest_mismatch")
            self.assertEqual(result["expanded_length"], 1)
            self.assertEqual(result["letters"], [])
            self.assertEqual(state.snapshot(), before)

    def test_external_record_then_and_start_digest(self):
        letters, expected = _blocked_word()
        with tempfile.TemporaryDirectory(dir=_TEMP_ROOT) as directory:
            path = Path(directory) / "census-record.json"
            path.write_text(json.dumps({"menu": "I_a", "k": 0, "seed": 17,
                                        "history": [], "moves": [], "steps": []}), encoding="utf-8")
            state = x3.start_state(("record", str(path)), then=letters[:1],
                                   start_digest=expected.digest(), ctx=CTX)
            self.assertEqual(state.digest(), expected.digest())
            self.assertEqual(x3.then_letters(json.dumps(letters[:1])), letters[:1])
            with self.assertRaisesRegex(ValueError, "start-digest"):
                x3.start_state(("solved",), start_digest="f" * 64, ctx=CTX)

    def test_filtered_counts_and_wrapper_error_cleanup(self):
        source = heights.h6030(ctx=CTX).prefix(6)
        source.state.filtered = True
        result = x3.preview(source.state, source.letters, source.state.menu, label="filtered", sample_every=2)
        self.assertGreater(sum(r["filtered_evaluations"] for r in result["letters"]), 0)
        self.assertNotIn("classify", source.state.__dict__)
        with patch.object(source.state, "apply", side_effect=RuntimeError("planted preview error")):
            with self.assertRaisesRegex(RuntimeError, "planted preview error"):
                x3.preview(source.state, source.letters[:1], source.state.menu, label="error")
        self.assertNotIn("classify", source.state.__dict__)


class CLISmokeTests(_TimedCase):
    def test_in_process_cli_outputs_and_external_paths(self):
        with tempfile.TemporaryDirectory(dir=_TEMP_ROOT) as directory:
            root = Path(directory)
            word = root / "two-letters.json"
            letters = heights.h6030(ctx=CTX).letters[:2]
            solved_digest = sim.State(CTX).digest()
            word.write_text(json.dumps({"word": letters, "input_digest": solved_digest}), encoding="utf-8")
            output = io.StringIO()
            with redirect_stdout(output):
                hr = heights.main(["--out", str(root / "heights"), "--word", str(word), "solved", "--every", "2", "--widths"])
                xr = x3.main(["--out", str(root / "x3"), "--word", str(word), "--start", "solved",
                              "--then", "[]", "--start-digest", solved_digest, "--label", "smoke", "--sample-every", "1"])
                mr = meshes.main(["--out", str(root / "meshes"), "--write-mesh"])
            self.assertEqual(output.getvalue(), "")
            for report in (hr, xr, mr):
                for key in ("cpu_count", "python", "numpy", "commit", "evidence_kind", "production_command"):
                    self.assertIn(key, report)
                self.assertEqual(report["numpy"], np.__version__)
                self.assertGreater(report["cpu_count"], 0)
                self.assertEqual(report["production_command"][:2], ["python", "-B"])
            for subdir, prefix in (("heights", "heights"), ("x3", "x3"), ("meshes", "meshes")):
                doc = json.loads((root / subdir / f"{prefix}-summary.json").read_text(encoding="utf-8"))
                self.assertIn("evidence_kind", doc)
            self.assertEqual(hr["inputs"][0]["steps"], 2)
            self.assertEqual(len(hr["inputs"][0]["operand_widths"]), 2)
            hrows = [json.loads(s) for s in (root / "heights/heights-steps.jsonl").read_text().splitlines()]
            self.assertEqual([r["step"] for r in hrows], [0, 2])
            self.assertIn("max_denominator", hrows[-1])
            self.assertEqual(xr["inputs"][0]["status"], "complete")
            self.assertEqual(xr["inputs"][0]["expanded_length"], 2)
            xrows = [json.loads(s) for s in (root / "x3/x3-letters.jsonl").read_text().splitlines()]
            self.assertEqual(len(xrows), 2)
            self.assertIn("exact_evaluations", xrows[0])
            self.assertIn("filtered_evaluations", xrows[0])
            mrows = (root / "meshes/meshes-orbits.jsonl").read_text().splitlines()
            self.assertEqual(len(mrows), 36)
            self.assertEqual((root / "meshes/exposed-mesh-vertices.f32").stat().st_size, mr["base_mesh"]["cut_bytes"])
            offsets = json.loads((root / "meshes/exposed-mesh-offsets.json").read_text())
            self.assertEqual(offsets["offsets"][-1] * 16, mr["base_mesh"]["cut_bytes"])

    def test_all_clis_refuse_output_inside_repository(self):
        for main in (heights.main, meshes.main, x3.main):
            with self.subTest(cli=main.__module__):
                with self.assertRaisesRegex(ValueError, "outside the repository"):
                    main(["--out", str(ROOT / "forbidden-scale-output")])


def load_tests(loader, tests, pattern):
    if _IN_PROCESS:
        return tests
    from tests import isolated_numpy
    return isolated_numpy.suite("test_jumbling_leads_scale", "J1 scale leads in an isolated NumPy process",
                                "NumPy is required for the J1 scale leads")


if __name__ == "__main__":
    unittest.main(verbosity=0)
