"""Taste map: named axes, themes, like rates and the private report (tastelab environment).

  tools\\.venv\\tastelab\\Scripts\\python.exe tests\\test_tastelab_tastemap.py

Synthetic vectors and temporary stores only. Notebook cells run in process;
explore is tested through its command, without a server or browser.
"""
from __future__ import annotations

import io
import json
import os
import runpy
import shutil
import sys
import tempfile
import unittest
from uuid import uuid4
from dataclasses import dataclass
from unittest import mock
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools"))

try:
    import numpy as np
    from PIL import Image
    from sklearn.metrics import roc_auc_score

    from tastelab import common, embed, explore, mapping, seeds
    from tastelab.store import ImageMeta, Store
except ImportError:  # the tastelab environment is not installed here
    np = None

MODEL = "fake-16"
DIM = 16
NOTEBOOK = ROOT / "tools" / "tastelab" / "tastemap.py"


@dataclass
class State:
    ratings: int = 0
    likes: int = 0
    dislikes: int = 0
    ready: bool = True
    auc: float | None = 0.8
    auc_delta: float | None = 0.01
    stable: bool = False
    weak: bool = False


class FakeModel:
    def __init__(self, direction):
        self._direction = direction

    def refresh(self, *, force=False):
        return State()

    def direction(self):
        return self._direction


def unit(v):
    v = np.asarray(v, dtype=np.float32)
    return v / np.linalg.norm(v)


def axis_vector(i, scale=1.0):
    v = np.zeros(DIM, dtype=np.float32)
    v[i] = scale
    return v


def probes_for(axes, terms):
    return seeds.parse_probes({"version": 1, "template": "an image of {}", "axes": [list(a) for a in axes], "terms": terms})


@unittest.skipIf(np is None, "tastelab environment (numpy, scikit-learn, Pillow) not installed")
class AnalysisTests(unittest.TestCase):
    def test_dependent_axes_explain_zero_for_orthogonal_preference_in_512_dimensions(self):
        for dim in (3, 512):
            with self.subTest(dim=dim):
                e = np.eye(dim)
                axes = [e[0], e[1], (e[0] + e[1]) / np.sqrt(2)]
                self.assertEqual(mapping.explained_share(e[2], axes), 0)

    def test_dependent_axis_never_increases_explained_share(self):
        e = np.eye(512)
        preference = (e[0] + e[2]) / np.sqrt(2)
        share = mapping.explained_share(preference, [e[0], e[1]])
        self.assertEqual(share, 0.5)
        for dependent in (e[0], (e[0] + e[1]) / np.sqrt(2), np.zeros(512)):
            self.assertEqual(mapping.explained_share(preference, [e[0], e[1], dependent]), share)

    def test_long_term_rankings_never_overlap(self):
        terms = {f"term {i}": unit([i - 6, 1] + [0] * 14) for i in range(13)}
        liked, avoided = mapping.term_ranking(axis_vector(0), terms)
        self.assertEqual((len(liked), len(avoided)), (5, 5))
        self.assertTrue({t for t, _ in liked}.isdisjoint(t for t, _ in avoided))

    def test_liked_and_avoided_term_lists_never_share_a_term(self):
        liked, avoided = mapping.separate_terms(["warm", "ornate", "warm", "soft"], ["soft", "plain", "plain"])
        self.assertEqual((liked, avoided), (["warm", "ornate"], ["plain"]))

    def test_short_term_list_is_one_complete_ranking(self):
        terms = {f"term {i}": unit([i - 3, 1] + [0] * 14) for i in range(6)}
        ranked, avoided = mapping.term_ranking(axis_vector(0), terms)
        self.assertEqual(len(ranked), 6)
        self.assertEqual(avoided, [])
        self.assertEqual([t for t, _ in ranked], [f"term {i}" for i in reversed(range(6))])

    def test_auc_matches_scikit_learn_with_ties(self):
        rng = np.random.default_rng(1)
        scores = rng.integers(0, 5, 200).astype(float)
        y = rng.integers(0, 2, 200)
        self.assertAlmostEqual(mapping.auc(scores, y), roc_auc_score(y, scores), places=12)
        self.assertTrue(np.isnan(mapping.auc([1.0, 2.0], [1, 1])))

    def test_bootstrap_interval_brackets_the_auc(self):
        rng = np.random.default_rng(2)
        y = np.array([1] * 60 + [0] * 60)
        scores = y + rng.normal(0, 1.0, 120)
        lo, hi = mapping.bootstrap_interval(scores, y, rounds=100)
        self.assertLess(lo, mapping.auc(scores, y))
        self.assertGreater(hi, mapping.auc(scores, y))
        self.assertGreater(lo, 0.5)

    def test_quintile_rates_follow_the_projection(self):
        scores = np.arange(10, dtype=float)
        y = np.array([0, 0, 0, 0, 0, 1, 1, 1, 1, 1])
        self.assertEqual(mapping.quintile_rates(scores, y), [0.0, 0.0, 0.5, 1.0, 1.0])

    def test_axes_rank_strength_skip_redundant_and_mark_weak(self):
        rng = np.random.default_rng(3)
        y = np.array([1] * 80 + [0] * 80)
        noise = rng.normal(0, 1, (80, DIM))
        shift = axis_vector(0, 2.0) - axis_vector(2, 0.6)   # likes lie towards +e0 and a little towards -e2
        X = np.concatenate([noise + shift, noise - shift])   # identical elsewhere, so other axes carry nothing
        axes = [("plain", "ornate", axis_vector(0)),
                ("plain again", "ornate again", unit(axis_vector(0) + axis_vector(1, 0.05))),
                ("cool", "warm", axis_vector(2)),
                ("x1", "y1", axis_vector(5)), ("x2", "y2", axis_vector(6)), ("x3", "y3", axis_vector(7))]
        kept = mapping.rank_axes(X, y, axes)
        self.assertIn(kept[0]["preferred"], ("ornate", "ornate again"))
        self.assertEqual(len({"ornate", "ornate again"} & {a["positive"] for a in kept}), 1)   # the twin is redundant
        self.assertEqual(kept[1]["preferred"], "cool")
        self.assertEqual(kept[1]["avoided"], "warm")
        self.assertEqual(len(kept), mapping.MIN_AXES)          # weak axes only fill up to the minimum
        self.assertTrue(all(a["weak"] for a in kept[2:]))
        self.assertFalse(kept[0]["weak"])
        self.assertGreater(kept[0]["quintiles"][-1], kept[0]["quintiles"][0])

    def test_axis_directions_use_the_phrased_texts(self):
        probes = probes_for([("dark", "bright"), ("calm", "loud")], ["neon"])
        vectors = {"an image of dark": axis_vector(0), "an image of bright": axis_vector(1)}
        found = mapping.axis_directions(probes, vectors)
        self.assertEqual([(a, b) for a, b, _ in found], [("dark", "bright")])
        np.testing.assert_allclose(found[0][2], unit(axis_vector(1) - axis_vector(0)), atol=1e-6)

    def test_explained_share_and_term_ranking(self):
        self.assertAlmostEqual(mapping.explained_share(axis_vector(0), [axis_vector(0), axis_vector(1)]), 1.0)
        self.assertAlmostEqual(mapping.explained_share(unit([1, 1] + [0] * 14), [axis_vector(0)]), 0.5)
        self.assertIsNone(mapping.explained_share(None, [axis_vector(0)]))
        terms = {"glow": axis_vector(0), "rust": -axis_vector(0), "grid": axis_vector(1)}
        favoured, disfavoured = mapping.term_ranking(axis_vector(0), terms, k=2)
        self.assertEqual([t for t, _ in favoured], ["glow", "grid", "rust"])
        self.assertEqual(disfavoured, [])

    def test_themes_name_groups_by_their_nearest_terms(self):
        rng = np.random.default_rng(4)
        X = embed.normalise(np.concatenate([axis_vector(0) + rng.normal(0, 0.05, (30, DIM)),
                                            axis_vector(1) + rng.normal(0, 0.05, (20, DIM))]))
        shas = [f"{i:064x}" for i in range(50)]
        found = mapping.themes(X, shas, {"glow": axis_vector(0), "grid": axis_vector(1)}, k_max=2)
        self.assertEqual([t["size"] for t in found], [30, 20])
        self.assertEqual([t["terms"][0] for t in found], ["glow", "grid"])
        self.assertTrue(set(found[1]["examples"]) <= set(shas[30:]))
        self.assertEqual(mapping.themes(X[:3], shas[:3], {}), [])

    def test_category_rates_ignore_skips_in_the_rate(self):
        rows = [("y2k", "like"), ("y2k", "like"), ("y2k", "dislike"), ("y2k", "skip"), ("control", "skip")]
        rates = {r["category"]: r for r in mapping.category_rates(rows)}
        self.assertEqual(rates["y2k"]["like_rate"], 0.667)
        self.assertEqual(rates["y2k"]["skips"], 1)
        self.assertIsNone(rates["control"]["like_rate"])


@unittest.skipIf(np is None, "tastelab environment (numpy, scikit-learn, Pillow) not installed")
class ReportTests(unittest.TestCase):
    def test_short_terms_render_as_one_ranking_without_conflicting_labels(self):
        report, _ = mapping.build_report(self.store, self.probes, MODEL, FakeModel(axis_vector(0)), day="2026-10-02")
        text = mapping.markdown(report)
        self.assertIn("## Probe term ranking", text)
        self.assertNotIn("Probe terms along the preference direction", text)
        self.assertNotIn("Probe terms against the preference direction", text)
        self.assertEqual(report["favoured_terms"], [])
        self.assertEqual(report["disfavoured_terms"], [])
        self.assertEqual(len(report["ranked_terms"]), 3)

    def setUp(self):
        self.tmp = Path(tempfile.gettempdir()) / f"tastelab-test-{uuid4().hex}"
        self.tmp.mkdir()
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        guard = mock.patch.object(common, "PRIVATE_BASE", self.tmp)
        guard.start()
        self.addCleanup(guard.stop)
        self.root = self.tmp / "tastelab"
        self.store = Store(self.root)
        self.probes = probes_for([("plain", "ornate"), ("cool", "warm"), ("still", "busy"), ("dull", "shiny"),
                                  ("small", "large")], ["neon glow", "brass", "grid"])
        texts = {"plain": -axis_vector(0), "ornate": axis_vector(0), "cool": -axis_vector(2), "warm": axis_vector(2),
                 "still": -axis_vector(5), "busy": axis_vector(5), "dull": -axis_vector(6), "shiny": axis_vector(6),
                 "small": -axis_vector(7), "large": axis_vector(7),
                 "neon glow": axis_vector(0), "brass": -axis_vector(0), "grid": axis_vector(3)}
        self.store.put_text_embeddings(MODEL, DIM, [(self.probes.phrase(t), embed.to_blob(unit(v))) for t, v in texts.items()])
        rng = np.random.default_rng(5)
        for i in range(60):
            like = i % 2 == 0
            sha = f"{i + 1:064x}"
            colour = (200, 80, 40) if like else (40, 80, 200)
            out = io.BytesIO()
            Image.new("RGB", (64, 64), colour).save(out, format="JPEG")
            category = ("y2k", "gaming", "control")[i % 3]
            self.assertEqual(self.store.add_image(ImageMeta(sha, "wikimedia", "CC0-1.0", "Someone", "A", category, 64, 64),
                                                  out.getvalue()), "stored")
            v = rng.normal(0, 0.3, DIM)
            v[0] += 1.5 if like else -1.5
            self.store.put_embeddings(MODEL, DIM, [(sha, embed.to_blob(unit(v)))])
            self.store.add_rating(sha, "like" if like else "dislike", "s1", note=f"note {i}" if i < 4 else None)
        self.store.add_screen_counts("safebooru", 10, 2)
        self.store.add_model_run(MODEL, 60, 30, 30, 0.9, None, False)

    def tearDown(self):
        self.store.close()

    def test_report_names_axes_themes_and_writes_private_files(self):
        report, sheets = mapping.build_report(self.store, self.probes, MODEL, FakeModel(axis_vector(0)), day="2026-10-02")
        self.assertIsNone(report["insufficient"])
        self.assertEqual(report["axes"][0]["preferred"], "ornate")
        self.assertEqual(report["axes"][0]["sheet"], "axis-1")
        self.assertIn("ornate", report["metaphor_material"])
        self.assertIn("plain", report["anti_goals"])
        self.assertEqual(report["ranked_terms"][0][0], "neon glow")
        self.assertEqual(report["ranked_terms"][-1][0], "brass")
        self.assertEqual(report["favoured_terms"], [])
        self.assertEqual(report["disfavoured_terms"], [])
        self.assertEqual({r["category"] for r in report["categories"]}, {"y2k", "gaming", "control"})
        self.assertEqual(len(report["notes"]["like"]) + len(report["notes"]["dislike"]), 4)
        self.assertEqual(report["screen"], {"safebooru": {"checked": 10, "discarded": 2}})
        self.assertEqual(report["missing_probe_vectors"], 0)
        top, bottom = sheets["axis-1"]
        self.assertTrue(all(v == "like" for _, v in top))
        self.assertTrue(all(v == "dislike" for _, v in bottom))
        path = mapping.write_report(self.root / "reports", report, sheets, self.store.thumb_path)
        self.assertEqual(path, self.root / "reports" / "tastemap-2026-10-02.md")
        text = path.read_text(encoding="utf-8")
        self.assertIn(mapping.PRIVATE, text)
        self.assertIn("![axis 1](tastemap-2026-10-02-axis-1.jpg)", text)
        self.assertIn("never a model feature", text)
        data = json.loads((self.root / "reports" / "tastemap-2026-10-02.json").read_text(encoding="utf-8"))
        self.assertEqual(data["axes"][0]["preferred"], "ornate")
        for name in sheets:
            with Image.open(self.root / "reports" / f"tastemap-2026-10-02-{name}.jpg") as im:
                self.assertEqual(im.format, "JPEG")
        written = sorted(p.name for p in (self.root / "reports").iterdir())
        self.assertTrue(all(n.startswith("tastemap-2026-10-02") for n in written), written)

    def test_few_ratings_report_insufficient_without_axes(self):
        for r in self.store.ratings()[4:]:
            self.store.undo_last("s1")
        report, sheets = mapping.build_report(self.store, self.probes, MODEL, FakeModel(None), day="2026-10-02")
        self.assertIn("at least", report["insufficient"])
        self.assertEqual(report["axes"], [])
        self.assertNotIn("axis-1", sheets)
        self.assertIn("Axes need at least", mapping.markdown(report))


def make_data(root: Path, images: int, canary: str) -> None:
    """A data folder with synthetic images, vectors under the CLIP model id and one rating whose note is `canary`."""
    rng = np.random.default_rng(6)
    probes = seeds.load_probes(seeds.PROBES_DEFAULT)
    with Store(root) as store:
        store.put_text_embeddings(embed.MODEL_ID, 512, [(probes.phrase(t), embed.to_blob(unit(rng.normal(size=512))))
                                                       for t in probes.texts()])
        for i in range(images):
            like = i % 2 == 0
            sha = f"{i + 1:064x}"
            out = io.BytesIO()
            Image.new("RGB", (64, 64), (200, 80, 40) if like else (40, 80, 200)).save(out, format="JPEG")
            store.add_image(ImageMeta(sha, "wikimedia", "CC0-1.0", "Someone", "A", "y2k", 64, 64), out.getvalue())
            v = rng.normal(0, 0.3, 512)
            v[0] += 1.5 if like else -1.5
            store.put_embeddings(embed.MODEL_ID, 512, [(sha, embed.to_blob(unit(v)))])
            store.add_rating(sha, "like" if like else "dislike", "s1", note=canary if i == 0 else None)


@unittest.skipIf(np is None, "tastelab environment not installed")
class NotebookTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.gettempdir()) / f"tastelab-test-{uuid4().hex}"
        self.tmp.mkdir()
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        guard = mock.patch.object(common, "PRIVATE_BASE", self.tmp)
        guard.start()
        self.addCleanup(guard.stop)
        self.root = self.tmp / "tastelab"

    def notebook(self):
        return runpy.run_path(str(NOTEBOOK))["app"]

    def test_notebook_missing_data_writes_nothing(self):
        import marimo as mo
        from types import SimpleNamespace
        with mock.patch.object(mo, "cli_args", return_value={"data": str(self.root)}), \
                mock.patch.object(mo, "app_meta", return_value=SimpleNamespace(mode="script")):
            self.notebook().run()
        self.assertFalse(self.root.exists())

    def test_notebook_writes_private_report_without_server(self):
        import marimo as mo
        import socket
        from types import SimpleNamespace
        make_data(self.root, 6, "synthetic notebook note")
        with mock.patch.object(mo, "cli_args", return_value={"data": str(self.root)}), \
                mock.patch.object(mo, "app_meta", return_value=SimpleNamespace(mode="script")), \
                mock.patch.object(socket.socket, "bind", side_effect=AssertionError("no listeners")):
            _, definitions = self.notebook().run()
        self.addCleanup(definitions["store"].close)
        reports = list((self.root / "reports").glob("tastemap-*.md"))
        self.assertEqual(len(reports), 1)
        self.assertIn("synthetic notebook note", reports[0].read_text(encoding="utf-8"))
        self.assertFalse((NOTEBOOK.parent / "__marimo__").exists())

    def test_notebook_refuses_exposed_editor_cache(self):
        import marimo as mo
        from types import SimpleNamespace
        make_data(self.root, 6, "private synthetic note")
        with mock.patch.object(mo, "cli_args", return_value={"data": str(self.root)}), \
                mock.patch.object(mo, "app_meta", return_value=SimpleNamespace(mode="edit")), \
                mock.patch.object(sys, "pycache_prefix", None):
            _, definitions = self.notebook().run()
        self.assertNotIn("store", definitions)
        self.assertEqual(list((self.root / "reports").iterdir()), [])

    def test_cache_contained_needs_the_prefix_inside_data_folder(self):
        self.root.mkdir()
        for prefix, expected in ((None, False), (str(self.root / "marimo"), True), (str(self.root), True),
                                 (str(self.tmp), False), (str(self.tmp / "tastelab-other"), False)):
            with mock.patch.object(sys, "pycache_prefix", prefix):
                self.assertEqual(mapping.cache_contained(self.root), expected, prefix)

    def test_cache_contained_rejects_session_markers(self):
        self.root.mkdir()
        (self.root / "session.sqlite3").write_bytes(b"synthetic marker")
        with mock.patch.object(sys, "pycache_prefix", str(self.root / "marimo")):
            self.assertFalse(mapping.cache_contained(self.root))

    def test_explore_command_keeps_marimo_caches_private_offline(self):
        argv, env = explore.command(self.root)
        self.assertEqual(argv[1:5], ["-m", "marimo", "edit", "--skip-update-check"])
        self.assertEqual(argv[5:], [str(NOTEBOOK), "--", "--data", str(self.root)])
        self.assertTrue(Path(argv[5]).is_absolute())
        self.assertEqual(env["PYTHONPYCACHEPREFIX"], str(self.root / explore.CACHE_FOLDER))
        self.assertEqual(env["MARIMO_SKIP_UPDATE_CHECK"], "1")
        self.assertFalse(self.root.exists())

    def test_explore_refuses_data_outside_private_base_before_launch(self):
        with mock.patch.object(explore.subprocess, "call", side_effect=AssertionError("no launch")):
            with self.assertRaises(common.Refused):
                explore.main(["--data", str(self.tmp.parent / "other")])


if __name__ == "__main__":
    unittest.main()
