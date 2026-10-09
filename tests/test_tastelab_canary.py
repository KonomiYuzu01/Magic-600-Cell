"""Mixed-tier feature canaries at every fitting boundary, using synthetic data only."""
from __future__ import annotations

import io
import shutil
import sys
import tempfile
import unittest
from collections import Counter
from contextlib import ExitStack
from pathlib import Path
from types import SimpleNamespace
from unittest import mock
from uuid import uuid4

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

try:
    import numpy as np
    from PIL import Image
    from threadpoolctl import threadpool_limits
    from tastelab import common, embed, fetch, learn, mapping, screen, seeds, sources, store
    HAVE_ENV = True
except ImportError:
    HAVE_ENV = False

MODEL = "fake-16"
DIM = 16


@unittest.skipUnless(HAVE_ENV, "needs the Taste Lab environment")
class CanaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        limits = threadpool_limits(limits=1)
        limits.__enter__()
        cls.addClassCleanup(limits.__exit__, None, None, None)

    def setUp(self):
        tmp = Path(tempfile.gettempdir()) / f"tastelab-test-{uuid4().hex}"
        tmp.mkdir()
        self.addCleanup(shutil.rmtree, tmp, ignore_errors=True)
        guard = mock.patch.object(common, "PRIVATE_BASE", tmp)
        guard.start()
        self.addCleanup(guard.stop)
        self.db = store.Store(tmp / "library")
        self.addCleanup(self.db.close)
        self.rng = np.random.default_rng(60)
        self.number = 0
        self.plan = seeds.parse({"version": 1, "categories": {
            "a_only": {"kind": "focus", "target": 0, "terms": ["known A"], "adjacent": ["adjacent A"]},
            "b_only": {"kind": "focus", "target": 0, "terms": ["known B"], "adjacent": ["canary B term"]},
        }})
        self.probes = seeds.parse_probes({"version": 1, "template": "{}", "axes": [["plain", "ornate"]],
                                          "terms": [f"term {i}" for i in range(12)]})
        self.texts = {"plain": -np.eye(DIM)[0], "ornate": np.eye(DIM)[0]}
        self.texts.update({t: embed.normalise([i - 6, 1] + [0] * (DIM - 2))
                           for i, t in enumerate(self.probes.terms)})
        self.db.put_text_embeddings(MODEL, DIM, [(t, embed.to_blob(v)) for t, v in self.texts.items()])

    def add(self, tier, verdict=None):
        self.number += 1
        sha = f"{self.number:064x}"
        v = self.rng.normal(0, 0.03, DIM)
        v[0] += -1 if verdict == "dislike" else 1
        v[-1] = 10 if tier == "B" else 0
        v = embed.normalise(v)
        out = io.BytesIO()
        Image.new("RGB", (64, 64), (200 if tier == "B" else 20, 50, 60)).save(out, format="JPEG")
        meta = store.ImageMeta(sha, "archive" if tier == "B" else "met",
                               "private-reference" if tier == "B" else "CC0-1.0",
                               "Synthetic fixture", tier, "b_only" if tier == "B" else "a_only", 64, 64)
        self.assertEqual(self.db.add_image(meta, out.getvalue(), embedding=(MODEL, DIM, embed.to_blob(v))), "stored")
        if verdict:
            self.db.add_rating(sha, verdict, "test")
        return sha

    def mixed(self):
        a = [self.add("A", verdict) for verdict in ("like", "dislike") for _ in range(30)]
        b = [self.add("B", verdict) for verdict in ("like", "dislike") for _ in range(8)]
        self.add("A")
        b.append(self.add("B"))
        return a, b

    def assert_no_canary(self, values, step):
        values = np.asarray(values)
        self.assertGreater(values.size, 0, step + " must actually run")
        self.assertTrue(np.all(np.abs(values[..., -1]) < 1e-7), step)

    def test_five_class_a_likes_and_five_class_b_dislikes_never_make_ready(self):
        for _ in range(5):
            self.add("A", "like")
            self.add("B", "dislike")
        model = learn.TasteModel(self.db, MODEL)
        data = model.snapshot()
        self.assert_no_canary(data.X, "classifier snapshot")
        state = model.refresh()
        self.assertEqual((state.ratings, state.likes, state.dislikes, state.ready), (5, 5, 0, False))
        self.assertEqual(self.db.model_runs(), [])

    def test_every_learning_and_map_fit_receives_only_class_a(self):
        a, b = self.mixed()
        calls = Counter()
        stage = "classifier"
        original_fit = learn.LogisticRegression.fit
        original_cluster = learn.KMeans.fit
        original_cv = learn.cross_val_predict

        def classifier(estimator, X, y, *args, **kwargs):
            calls["classifier"] += 1
            self.assert_no_canary(X, "classifier (including each CV fold)")
            return original_fit(estimator, X, y, *args, **kwargs)

        def clusters(estimator, X, *args, **kwargs):
            calls[stage + " clusters"] += 1
            self.assert_no_canary(X, stage + " clusters")
            return original_cluster(estimator, X, *args, **kwargs)

        def cv(estimator, X, y, *args, **kwargs):
            calls["cross-validation"] += 1
            self.assert_no_canary(X, "cross-validation")
            return original_cv(estimator, X, y, *args, **kwargs)

        def guard_X(name, original):
            def call(X, *args, **kwargs):
                calls[name] += 1
                self.assert_no_canary(X, name)
                return original(X, *args, **kwargs)
            return call

        def terms(direction, *args, **kwargs):
            calls["term ranking"] += 1
            self.assert_no_canary(direction, "term ranking direction")
            return original_terms(direction, *args, **kwargs)

        def directions(probes, vectors):
            calls["axis directions"] += 1
            for value in vectors.values():
                self.assert_no_canary(value, "axis text directions")
            return original_directions(probes, vectors)

        original_terms, original_directions = mapping.term_ranking, mapping.axis_directions
        with ExitStack() as patches:
            patches.enter_context(mock.patch.object(learn.LogisticRegression, "fit", classifier))
            patches.enter_context(mock.patch.object(learn.KMeans, "fit", clusters))
            patches.enter_context(mock.patch.object(learn, "cross_val_predict", cv))
            patches.enter_context(mock.patch.object(mapping, "rank_axes", guard_X("axis ranking", mapping.rank_axes)))
            patches.enter_context(mock.patch.object(mapping, "themes", guard_X("theme centroids", mapping.themes)))
            patches.enter_context(mock.patch.object(mapping, "term_ranking", terms))
            patches.enter_context(mock.patch.object(mapping, "axis_directions", directions))
            model = learn.TasteModel(self.db, MODEL, record=False)
            self.assertTrue(model.refresh().ready)
            stage = "selector"
            selector = learn.Selector(self.db, model, self.plan, seed=0)
            self.assertEqual(set(selector.next(100)), set(self.db.embedded(MODEL)) - self.db.rated())
            stage = "proposal"
            proposed = learn.propose_terms(self.db, model, self.plan, self.texts)
            self.assertNotIn("canary B term", [t for t, _, _ in proposed])
            stage = "map"
            report, _ = mapping.build_report(self.db, self.probes, MODEL, model, day="2026-10-09")
        for step in ("classifier", "cross-validation", "selector clusters", "proposal clusters", "map clusters",
                     "axis directions", "axis ranking", "theme centroids", "term ranking"):
            self.assertGreater(calls[step], 0, step)
        self.assertEqual((report["model"]["likes"], report["model"]["dislikes"]), (30, 30))
        self.assertEqual(set(model._fit.data.shas), set(a))
        self.assertEqual({r["sha256"] for r in report["placements"]}, set(b))

    def test_class_b_ratings_never_advance_proposal_cadence_or_rank_categories(self):
        for _ in range(100):
            self.add("B", "like")
        self.assertFalse(learn.proposals_due(self.db))
        model = learn.TasteModel(self.db, MODEL, record=False)
        proposed = learn.propose_terms(self.db, model, self.plan, self.texts)
        self.assertEqual(proposed, [])
        self.assertEqual(self.db.last_proposal_rating(), 0)

    def test_only_class_b_images_do_not_fit_selector_or_map(self):
        shas = [self.add("B", verdict) for verdict in ("like", "dislike") for _ in range(12)]
        model = learn.TasteModel(self.db, MODEL, record=False)
        with mock.patch.object(learn.KMeans, "fit", side_effect=AssertionError("class B fit")), \
                mock.patch.object(mapping, "rank_axes", side_effect=AssertionError("class B axes")):
            selector = learn.Selector(self.db, model, self.plan, seed=0)
            report, _ = mapping.build_report(self.db, self.probes, MODEL, model, day="2026-10-09")
        self.assertFalse(report["model"]["ready"])
        self.assertIsNotNone(report["insufficient"])
        self.assertEqual(selector.next(10), [])  # these references have already been rated
        self.assertEqual({p["sha256"] for p in report["placements"]}, set(shas))

    def test_class_b_placements_do_not_change_class_a_axes_terms_or_themes(self):
        for verdict in ("like", "dislike"):
            for _ in range(15):
                self.add("A", verdict)
        model = learn.TasteModel(self.db, MODEL, record=False)
        before, _ = mapping.build_report(self.db, self.probes, MODEL, model, day="2026-10-09")
        b = [self.add("B", verdict) for verdict in ("like", "dislike") for _ in range(8)]
        after, _ = mapping.build_report(self.db, self.probes, MODEL, model, day="2026-10-09")
        for field in ("axes", "explained", "favoured_terms", "disfavoured_terms", "ranked_terms",
                      "liked_themes", "anti_goal_themes", "metaphor_material", "anti_goals"):
            self.assertEqual(before[field], after[field], field)
        self.assertEqual({r["sha256"] for r in after["placements"]}, set(b))
        self.assertTrue(all(0 <= r["probability"] <= 1 and len(r["axes"]) == len(after["axes"])
                            for r in after["placements"]))

    def test_screen_calibration_rejects_class_b_canaries_at_every_step(self):
        vectors_seen, probes_seen, gates_seen, downloaded = [], [], [], []
        fake = embed.FakeEmbedder(texts={t: np.eye(DIM)[0] for t in screen.UNSAFE_PROBES + screen.SAFE_PROBES})
        original_probes, original_score, original_gate = screen.Screen._valid_probes, screen.Screen.unsafe_score, screen.gate

        def valid_probes(instance):
            probes_seen.append(instance._probes.copy())
            return original_probes(instance)

        def unsafe_score(instance, X):
            vectors_seen.append(np.array(X))
            self.assert_no_canary(X, "calibration scoring")
            return original_score(instance, X)

        def gate(groups, negatives, **kwargs):
            gates_seen.append((groups, negatives))
            return original_gate(groups, negatives, **kwargs)

        def search(client, query, cursor, **kwargs):
            rows = []
            for source, licence, ident in (("met", "CC0-1.0", "A"),
                                           ("archive", "private-reference", "B"),
                                           ("archive", "CC0-1.0", "B-forged-licence")):
                image_url = ("https://images.metmuseum.org/" if source == "met" else "https://archive.org/download/synthetic/") + ident + ".png"
                page_url = "https://www.metmuseum.org/art/collection/search/1" if source == "met" else "https://archive.org/details/synthetic"
                licence_url = sources.ADAPTERS["archive"].terms_url if licence == "private-reference" else sources.CC0_URL
                rows.append(sources.Candidate(source, query + ident, image_url, page_url, licence, licence_url,
                                               "Synthetic credit", title="adult woman nude"))
            return rows, None

        def get(url, **kwargs):
            downloaded.append(url)
            image = Image.new("RGB", (96, 96), (200 if "/B" in url else 20, 50, 60))
            out = io.BytesIO()
            image.save(out, format="PNG")
            return SimpleNamespace(body=out.getvalue())

        def image_vectors(images):
            return np.stack([np.eye(DIM)[-1 if im.getpixel((0, 0))[0] == 200 else 0] for im in images])

        with mock.patch.object(sources.ADAPTERS["met"], "search", side_effect=search), \
                mock.patch.object(screen, "CALIBRATION_GROUPS", {"nudity": {"queries": (("met", "positive"),), "match": "nude"}}), \
                mock.patch.object(screen, "NEGATIVE_QUERIES", (("met", "negative"),)), \
                mock.patch.multiple(screen, MIN_GROUP=1, GROUP_SIZE=2, NEGATIVE_SIZE=2), \
                mock.patch.object(fake, "embed_images", side_effect=image_vectors), \
                mock.patch.object(screen.Screen, "_valid_probes", valid_probes), \
                mock.patch.object(screen.Screen, "unsafe_score", unsafe_score), \
                mock.patch.object(screen, "gate", side_effect=gate):
            report = fetch.calibrate(fake, SimpleNamespace(get=get), log=lambda _: None)
        self.assertTrue(report["passed"])
        self.assertGreater(len(probes_seen), 0)
        self.assertEqual(len(vectors_seen), 2)
        self.assertEqual(len(gates_seen), 1)
        for X in [*probes_seen, *vectors_seen]:
            self.assert_no_canary(X, "calibration boundary")
        self.assertTrue(all("/A.png" in url for url in downloaded))
        self.assertEqual(report["groups"]["nudity"]["scorable"], 1)
        self.assertEqual(report["negatives"]["scorable"], 1)


if __name__ == "__main__":
    unittest.main()
