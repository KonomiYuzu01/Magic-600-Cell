"""Offline learning, selection and proposal checks with real temporary stores."""
from __future__ import annotations

import hashlib
import shutil
import sys
import tempfile
import unittest
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest import mock
from uuid import uuid4

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools"))

try:
    import numpy as np
    from PIL import Image
    from threadpoolctl import threadpool_limits
    from tastelab import common, embed, images, learn, seeds, store
    HAVE_ENV = True
except ImportError:
    HAVE_ENV = False


def plan(categories=("alpha", "beta"), adjacent=None):
    return seeds.parse({"version": 1, "categories": {
        name: {"kind": "focus", "target": 0, "terms": ["existing " + name],
               "adjacent": (adjacent or {}).get(name, [])} for name in categories}})


@unittest.skipUnless(HAVE_ENV, "needs the tastelab environment (numpy, sklearn, Pillow)")
class TempStore(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Small offline fixtures do not benefit from dozens of native worker threads.
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
        self.store = store.Store(tmp / "data")
        self.addCleanup(self.store.close)
        self.fake = embed.FakeEmbedder()
        self.rng = np.random.default_rng(123)
        self.number = 0
        self.thumb = images.thumbnail_jpeg(Image.new("RGB", (96, 96), (80, 130, 170)))

    def add(self, verdict=None, *, category="alpha", vector=None, embedded=True, model_id=None):
        self.number += 1
        sha = hashlib.sha256(str(self.number).encode()).hexdigest()
        meta = store.ImageMeta(sha, "wikimedia", "CC0-1.0", "Synthetic fixture", "A", category, 96, 96,
                               added=(datetime(2026, 1, 1, tzinfo=timezone.utc) + timedelta(seconds=self.number)).isoformat())
        self.store.add_image(meta, self.thumb)
        if embedded:
            if vector is None:
                vector = self.rng.normal(0, 0.05, 16)
                vector[0] += -1 if verdict == "dislike" else 1
            self.store.put_embeddings(model_id or self.fake.model_id, 16, [(sha, embed.to_blob(embed.normalise(vector)))])
        if verdict:
            self.store.add_rating(sha, verdict, "test")
        return sha

    def ready_model(self, *, record=True):
        for verdict in ("like", "dislike"):
            for _ in range(learn.MIN_PER_CLASS):
                self.add(verdict)
        model = learn.TasteModel(self.store, self.fake.model_id, record=record)
        model.refresh(force=True)
        return model


class ModelTests(TempStore):
    def test_threshold_crossing_trains_immediately_in_either_class(self):
        for last in ("like", "dislike"):
            with self.subTest(last=last):
                for row in self.store.images():
                    self.store.remove_image(row.sha256)
                model = learn.TasteModel(self.store, self.fake.model_id)
                for verdict in ("like", "dislike"):
                    for _ in range(4 if verdict == last else 5):
                        self.add(verdict)
                self.assertFalse(model.refresh().ready)
                self.add(last)
                self.assertTrue(model.due())
                self.assertTrue(model.refresh().ready)
                self.assertFalse(model.due())
                for row in self.store.images():
                    self.store.remove_image(row.sha256)

    def test_one_class_snapshot_retrains_when_second_class_arrives(self):
        for _ in range(6):
            self.add("like")
        model = learn.TasteModel(self.store, self.fake.model_id)
        self.assertFalse(model.refresh().ready)
        self.add("dislike")
        self.assertTrue(model.due())
        self.assertEqual(model.refresh().dislikes, 1)
        for _ in range(4):
            self.add("dislike")
        self.assertTrue(model.refresh().ready)

    def test_single_threshold_crossing_refreshes_unready_counts(self):
        for _ in range(4):
            self.add("like")
        model = learn.TasteModel(self.store, self.fake.model_id)
        self.assertEqual(model.refresh().likes, 4)
        self.add("like")
        self.assertTrue(model.due())
        self.assertEqual(model.refresh().likes, 5)

    def test_effective_labels_embeddings_skips_and_readiness(self):
        model = learn.TasteModel(self.store, self.fake.model_id)
        for verdict in ("like", "dislike"):
            for _ in range(5):
                self.add(verdict)
        self.add("skip")
        missing = self.add("like", embedded=False)
        self.add("dislike", model_id="other")
        before = model.state()
        self.assertEqual((before.ratings, before.likes, before.dislikes, before.ready), (13, 5, 5, False))
        data = model.snapshot()
        self.assertEqual(data.X.dtype, np.float32)
        self.assertEqual(data.shas, [r.sha256 for r in self.store.ratings()[:10]])
        state = model.refresh()
        self.assertTrue(state.ready)
        self.assertGreater(state.auc, 0.9)
        self.assertEqual((state.ratings, state.likes, state.dislikes), (13, 5, 5))
        probabilities = model.predict([data.shas[0], data.shas[5], missing, "f" * 64])
        self.assertEqual(probabilities.dtype, np.float64)
        self.assertGreater(probabilities[0], probabilities[1])
        np.testing.assert_array_equal(probabilities[2:], [0.5, 0.5])
        np.testing.assert_allclose(model.predict_vectors(data.X), model.predict(data.shas))
        self.assertAlmostEqual(np.linalg.norm(model.direction()), 1)

    def test_unready_is_neutral_and_records_nothing(self):
        shas = [self.add("like") for _ in range(6)]
        model = learn.TasteModel(self.store, self.fake.model_id)
        state = model.refresh()
        self.assertFalse(state.ready)
        self.assertIsNone(state.auc)
        self.assertIsNone(model.direction())
        np.testing.assert_array_equal(model.predict(shas), [0.5] * 6)
        self.assertEqual(model.predict_vectors(np.zeros((3, 16))).dtype, np.float64)
        self.assertEqual(model.predict_vectors(np.zeros((0, 16))).shape, (0,))
        self.assertEqual(self.store.model_runs(), [])

    def test_training_is_pure_deterministic_float64_and_cross_validated(self):
        model = self.ready_model(record=False)
        data = model.snapshot()
        with ThreadPoolExecutor(max_workers=1) as pool:
            first = pool.submit(learn.TasteModel.train, data).result()
        second = learn.TasteModel.train(data)
        self.assertEqual(first.auc, second.auc)
        np.testing.assert_array_equal(first.classifier.coef_, second.classifier.coef_)
        self.assertEqual(first.classifier.coef_.dtype, np.float64)
        self.assertEqual(first.classifier.class_weight, "balanced")
        self.assertEqual(first.classifier.max_iter, 2000)
        cv = learn.StratifiedKFold(n_splits=5, shuffle=True, random_state=0)
        probabilities = learn.cross_val_predict(first.classifier, data.X.astype(np.float64), data.y,
                                                cv=cv, method="predict_proba")[:, 1]
        self.assertEqual(first.auc, learn.roc_auc_score(data.y, probabilities))
        self.assertEqual(first.params, {"C": 1.0, "folds": 5, "labelled": 10,
                                        "sklearn_version": learn.sklearn.__version__})

    def test_due_tracks_new_labels_and_invalidated_pairs(self):
        model = learn.TasteModel(self.store, self.fake.model_id)
        self.assertTrue(model.due())
        model = self.ready_model()
        self.assertFalse(model.due())
        self.assertTrue(model.due(force=True))
        self.add("skip")
        self.add("like", embedded=False)
        for _ in range(learn.RETRAIN_EVERY - 1):
            self.add("like")
        self.assertFalse(model.due())
        self.add("dislike")
        self.assertTrue(model.due())
        model.refresh()
        sha = model._fit.data.shas[0]
        self.store.add_rating(sha, "dislike", "change")
        self.assertTrue(model.due())
        model.refresh()
        self.store.undo_last("change")
        self.assertTrue(model.due())
        model.refresh()
        self.store.remove_image(sha)
        self.assertTrue(model.due())

    def test_undo_recomputes_readiness_and_effective_verdict(self):
        model = self.ready_model()
        removed_label = self.store.undo_last("test")
        self.assertTrue(model.due())
        state = model.refresh()
        self.assertEqual((state.likes, state.dislikes, state.ready), (5, 4, False))
        self.assertEqual(model.predict([removed_label]).tolist(), [0.5])
        self.store.add_rating(removed_label, "dislike", "test")
        state = model.refresh(force=True)
        self.assertTrue(state.ready)
        self.store.add_rating(removed_label, "skip", "test")
        self.assertTrue(model.due())
        self.assertFalse(model.refresh().ready)

    def test_old_and_equal_worker_fits_are_ignored(self):
        model = self.ready_model()
        older = learn.TasteModel.train(model.snapshot())
        self.add("like")
        newer = learn.TasteModel.train(model.snapshot())
        state = model.apply(newer)
        before = self.store.db.total_changes
        self.assertIs(model.apply(older), state)
        self.assertIs(model.apply(newer), state)
        self.assertEqual(self.store.db.total_changes, before)

    def test_restart_suppression_and_record_false_write_nothing(self):
        model = self.ready_model()
        before = self.store.db.total_changes
        restarted = learn.TasteModel(self.store, self.fake.model_id)
        restarted.refresh(force=True)
        self.assertEqual(self.store.db.total_changes, before)
        self.add("like")
        before = self.store.db.total_changes
        readonly = learn.TasteModel(self.store, self.fake.model_id, record=False)
        with mock.patch.object(self.store, "add_model_run", side_effect=AssertionError("write")):
            readonly.refresh(force=True)
        self.assertEqual(self.store.db.total_changes, before)
        self.assertEqual(len(self.store.model_runs()), 1)
        self.assertTrue(model.state().ready)

    def plateau_fit(self, n, auc, serial=1):
        y = np.asarray([1] * (n // 2) + [0] * (n - n // 2))
        X = embed.normalise(self.rng.normal(size=(n, 16)))
        data = learn.TrainingSet([f"{i:064x}" for i in range(n)], X, y, n + 5, serial)
        return learn.Fit(data, object() if auc is not None else None, auc, {})

    def test_stability_uses_latest_eligible_run_of_the_same_model(self):
        mid = self.fake.model_id
        self.store.add_model_run(mid, 1000, 90, 100, 0.70, None, False)
        self.store.add_model_run(mid, 200, 100, 100, 0.72, None, False)
        self.store.add_model_run("other", 200, 100, 100, 0.1, None, False)
        self.store.add_model_run(mid, 201, 101, 100, 0.95, None, False)
        model = learn.TasteModel(self.store, mid)
        state = model.apply(self.plateau_fit(300, 0.73))
        self.assertAlmostEqual(state.auc_delta, 0.01)
        self.assertTrue(state.stable)
        self.assertFalse(state.weak)
        run = self.store.model_runs()[-1]
        self.assertEqual((run["ratings"], run["likes"], run["dislikes"], run["stable"]), (305, 150, 150, True))

    def test_weak_plateau_and_stability_limits(self):
        mid = self.fake.model_id
        self.store.add_model_run(mid, 200, 100, 100, 0.49, None, False)
        model = learn.TasteModel(self.store, mid, record=False)
        state = model.apply(self.plateau_fit(300, 0.50))
        self.assertTrue(state.weak)
        self.assertFalse(state.stable)
        state = model.apply(self.plateau_fit(300, 0.52, serial=2))
        self.assertFalse(state.weak)
        self.assertFalse(state.stable)
        state = model.apply(self.plateau_fit(299, 0.50, serial=3))
        self.assertFalse(state.weak)
        self.assertIsNone(state.auc_delta)
        empty = learn.TasteModel(self.store, "no-runs", record=False)
        self.assertIsNone(empty.apply(self.plateau_fit(300, 0.8)).auc_delta)


class SelectorTests(TempStore):
    def test_embedding_backfill_becomes_selectable_without_count_change(self):
        sha = self.add(embedded=False)
        selector = learn.Selector(self.store, self.cold_model(), plan(), seed=0)
        self.assertEqual(selector.next(), [])
        self.store.put_embeddings(self.fake.model_id, 16, [(sha, embed.to_blob(np.eye(16)[0]))])
        self.assertEqual(selector.next(), [sha])

    def test_replacement_with_same_count_never_selects_removed_hash(self):
        old = self.add()
        selector = learn.Selector(self.store, self.cold_model(), plan(), seed=0)
        self.store.remove_image(old)
        new = self.add()
        self.assertEqual(self.store.count_images(), 1)
        self.assertEqual(selector.next(10), [new])
        self.assertNotIn(old, selector._cold)

    def test_changed_embedding_rebuilds_diversity_and_representatives(self):
        basis = np.eye(16)
        reference = self.add(vector=basis[0])
        first = self.add(vector=-basis[0])
        second = self.add(vector=basis[1])
        with mock.patch.multiple(learn, COLD_START=1, QUOTA_UNTIL=0):
            selector = learn.Selector(self.store, self.cold_model(), plan(), seed=0)
            selector.shown(reference)
            self.store.put_embeddings(self.fake.model_id, 16, [(first, embed.to_blob(basis[0]))])
            self.assertEqual(selector.next(exclude=[reference]), [second])

    def test_new_rating_for_unembedded_image_cannot_crash_quota(self):
        sha = self.add()
        removed = self.add()
        selector = learn.Selector(self.store, self.cold_model(), plan(), seed=0)
        self.store.remove_image(removed)
        other = self.add("skip", embedded=False, category="beta")
        self.assertEqual(selector.next(), [sha])
        self.assertNotEqual(sha, other)

    def cold_model(self):
        model = learn.TasteModel(self.store, self.fake.model_id, record=False)
        model.refresh()
        return model

    def test_empty_store_and_images_without_matching_embeddings(self):
        model = self.cold_model()
        selector = learn.Selector(self.store, model, plan(), seed=0)
        self.assertEqual(selector.next(10), [])
        self.add(embedded=False)
        self.add(model_id="other")
        self.assertEqual(selector.next(10), [])
        valid = self.add()
        self.assertEqual(selector.next(10), [valid])

    def test_cold_representatives_are_nearest_centres_by_cluster_size(self):
        shas = [self.add(vector=self.rng.normal(size=16)) for _ in range(18)]
        X = embed.from_blobs(list(self.store.embeddings(self.fake.model_id, shas).values()), 16)
        kmeans = learn.KMeans(n_clusters=4, n_init=1, random_state=7).fit(X)
        sizes = np.bincount(kmeans.labels_, minlength=4)
        expected = [shas[np.argmin(np.sum((X - kmeans.cluster_centers_[i]) ** 2, axis=1))]
                    for i in np.argsort(-sizes, kind="stable")]
        with mock.patch.multiple(learn, COLD_START=4, QUOTA_UNTIL=0):
            selector = learn.Selector(self.store, self.cold_model(), plan(), seed=7)
            self.assertEqual(selector.next(4), expected)

    def test_reservations_exclusion_skips_and_undo(self):
        shas = [self.add(vector=self.rng.normal(size=16)) for _ in range(8)]
        self.store.add_rating(shas[0], "skip", "skip")
        with mock.patch.object(learn, "QUOTA_UNTIL", 0):
            selector = learn.Selector(self.store, self.cold_model(), plan(), seed=0)
            first = selector.next(1, exclude=shas[1:2])[0]
            second = selector.next(1, exclude=shas[1:2])[0]
            self.assertNotEqual(first, second)
            self.assertNotIn(first, shas[:2])
            selector.shown(first)
            self.assertEqual(selector.next(1, exclude=[s for s in shas if s != first]), [first])
            self.store.undo_last("skip")
            self.assertEqual(selector.next(1, exclude=shas[1:]), [shas[0]])

    def test_cache_rebuilds_on_eligible_image_changes(self):
        self.add()
        model = self.cold_model()
        with mock.patch.object(self.store, "embeddings", wraps=self.store.embeddings) as reads:
            with mock.patch.object(learn, "KMeans", wraps=learn.KMeans) as clusters:
                selector = learn.Selector(self.store, model, plan(), seed=0)
                selector.next()
                selector.next()
                self.assertEqual(reads.call_count, 3)
                added = self.add(vector=-np.eye(16)[0])
                self.assertEqual(selector.next(), [added])
                self.store.remove_image(added)
                self.assertEqual(selector.next(), [])
                self.assertEqual(clusters.call_count, 3)
        self.assertEqual(selector.next(0), [])

    def test_farthest_point_after_cold_start(self):
        basis = np.eye(16)
        a, b, c = [self.add(vector=v) for v in (basis[0], basis[1], -basis[0])]
        with mock.patch.multiple(learn, COLD_START=2, QUOTA_UNTIL=0):
            selector = learn.Selector(self.store, self.cold_model(), plan(), seed=0)
            selector.shown(a)
            selector.shown(b)
            self.assertEqual(selector.next(1, exclude=[a, b]), [c])

    def test_quota_uses_plan_order_and_sequential_batch_counts(self):
        alpha = [self.add(category="alpha") for _ in range(4)]
        beta = [self.add(category="beta") for _ in range(4)]
        selector = learn.Selector(self.store, self.cold_model(), plan(), seed=0)
        picks = selector.next(2)
        self.assertIn(picks[0], alpha)
        self.assertIn(picks[1], beta)
        selector.shown(picks[0])
        self.store.add_rating(picks[0], "skip", "quota")
        self.assertIn(selector.next()[0], beta)

    def test_quota_keeps_eight_percent_for_each_focus_style(self):
        focus = [f"style_{i}" for i in range(8)]
        for i in range(240):
            self.add(category=focus[i % 8], vector=self.rng.normal(size=16))
        selector = learn.Selector(self.store, self.cold_model(), plan(focus), seed=0)
        counts = dict.fromkeys(focus, 0)
        for _ in range(200):
            sha = selector.next()[0]
            selector.shown(sha)
            self.store.add_rating(sha, "skip", "quota")
            counts[self.store.image(sha).category] += 1
        self.assertTrue(all(count >= 16 for count in counts.values()), counts)

    def test_ready_uncertainty_times_diversity_and_seeded_exploration(self):
        model = self.ready_model(record=False)
        basis = np.eye(16)
        a, b, c = [self.add(vector=v) for v in (basis[0], basis[1], -basis[0])]
        probabilities = {a: 0.5, b: 0.5, c: 0.51}
        with mock.patch.object(learn, "QUOTA_UNTIL", 0):
            selector = learn.Selector(self.store, model, plan(), seed=0)
            selector.shown(a)
            with mock.patch.object(model, "predict", side_effect=lambda ss: np.array([probabilities[s] for s in ss])):
                self.assertEqual(selector.next(), [c])
            random = learn.Selector(self.store, model, plan(), seed=3)
            rng = np.random.default_rng(3)
            self.assertLess(rng.random(), learn.EXPLORE)
            with mock.patch.object(model, "predict", side_effect=AssertionError("exploration must be uniform")):
                self.assertEqual(random.next(), [[a, b, c][int(rng.integers(3))]])

    def test_ready_batches_match_sequential_display_and_no_history_is_zero_similarity(self):
        model = self.ready_model(record=False)
        shas = [self.add(vector=self.rng.normal(size=16)) for _ in range(6)]
        probabilities = dict(zip(shas, [0.01, 0.2, 0.49, 0.7, 0.8, 0.9]))
        with mock.patch.multiple(learn, QUOTA_UNTIL=0, EXPLORE=0):
            batch = learn.Selector(self.store, model, plan(), seed=0)
            sequential = learn.Selector(self.store, model, plan(), seed=0)
            with mock.patch.object(model, "predict", side_effect=lambda ss: np.array([probabilities[s] for s in ss])):
                picks = batch.next(3)
                self.assertEqual(picks[0], shas[2])
                expected = []
                for _ in range(3):
                    sha = sequential.next()[0]
                    sequential.shown(sha)
                    expected.append(sha)
                self.assertEqual(picks, expected)
                self.assertTrue(set(batch.next(10)).isdisjoint(picks))

    def test_diversity_uses_only_the_last_recent_displays(self):
        model = self.ready_model(record=False)
        basis = np.eye(16)
        a, b, reference = [self.add(vector=v) for v in (basis[0], -basis[0], basis[1])]
        with mock.patch.multiple(learn, QUOTA_UNTIL=0, EXPLORE=0):
            selector = learn.Selector(self.store, model, plan(), seed=0)
            selector.shown(a)
            for _ in range(learn.RECENT):
                selector.shown(reference)
            with mock.patch.object(model, "predict", side_effect=lambda ss: np.full(len(ss), 0.5)):
                self.assertEqual(selector.next(exclude=[reference]), [a])
        self.assertNotEqual(a, b)

    def test_quota_ends_at_200_effective_ratings(self):
        model = self.ready_model(record=False)
        for _ in range(190):
            self.add("skip", category="alpha")
        a, b = self.add(category="alpha"), self.add(category="beta")
        selector = learn.Selector(self.store, model, plan(), seed=0)
        with mock.patch.object(model, "predict", side_effect=lambda ss: np.array([0.5 if s == a else 0.99 for s in ss])):
            self.assertEqual(selector.next(), [a])
        self.assertNotEqual(a, b)


class ProposalTests(TempStore):
    def test_due_uses_effective_rating_multiples_including_skips(self):
        for _ in range(99):
            self.add("skip", embedded=False)
        self.assertFalse(learn.proposals_due(self.store))
        self.add("skip", embedded=False)
        self.assertTrue(learn.proposals_due(self.store))
        self.store.mark_proposal_round(150)
        self.assertFalse(learn.proposals_due(self.store))
        for _ in range(100):
            self.add("skip", embedded=False)
        self.assertTrue(learn.proposals_due(self.store))
        self.store.undo_last("test")
        self.assertFalse(learn.proposals_due(self.store))

    def test_proposals_alternate_rates_and_liked_clusters_and_deduplicate(self):
        basis = np.eye(16)
        for _ in range(6):
            self.add("like", category="alpha", vector=basis[0] + self.rng.normal(0, 0.001, 16))
            self.add("dislike", category="alpha")
        for _ in range(4):
            self.add("like", category="beta", vector=basis[1] + self.rng.normal(0, 0.001, 16))
        self.add("skip", category="gamma")
        terms = plan(("alpha", "beta", "gamma"), {
            "alpha": ["old accepted", "alpha new", "BETA NEW"],
            "beta": ["EXISTING ALPHA", "old rejected", "beta new", "beta second"],
            "gamma": ["never propose"]})
        self.store.add_proposals([("Old accepted", None, "probe"), ("Old rejected", None, "probe")], 0)
        self.store.set_proposal_status("Old accepted", "accepted")
        self.store.set_proposal_status("Old rejected", "rejected")
        model = learn.TasteModel(self.store, self.fake.model_id)
        result = learn.propose_terms(self.store, model, terms, {"probe one": basis[0], "probe two": basis[1]})
        self.assertEqual(result, [("beta new", "beta", "adjacent"), ("probe one", None, "probe"),
                                  ("beta second", "beta", "adjacent"), ("probe two", None, "probe"),
                                  ("alpha new", "alpha", "adjacent")])
        self.assertEqual(self.store.last_proposal_rating(), 17)
        self.assertEqual(learn.propose_terms(self.store, model, terms, {"probe one": basis[0]}), [])
        self.assertEqual(len(self.store.proposals()), 7)

    def test_round_limit_five_like_minimum_and_empty_round_mark(self):
        terms = plan(("alpha",), {"alpha": [f"term {i}" for i in range(20)]})
        model = learn.TasteModel(self.store, self.fake.model_id)
        for _ in range(4):
            self.add("like")
        result = learn.propose_terms(self.store, model, terms, {"probe": np.eye(16)[0]})
        self.assertEqual(len(result), learn.PROPOSE_MAX)
        self.assertTrue(all(origin == "adjacent" for _, _, origin in result))
        self.add("like", embedded=False)
        no_adjacent = plan(("alpha",))
        self.assertEqual(learn.propose_terms(self.store, model, no_adjacent, {"probe": np.eye(16)[0]}), [])
        self.assertEqual(self.store.last_proposal_rating(), 5)
        self.add("like")
        self.assertEqual(learn.propose_terms(self.store, model, no_adjacent, {"probe": np.eye(16)[0]}),
                         [("probe", None, "probe")])
        self.store.set_proposal_status("probe", "fetched")
        self.assertEqual(learn.propose_terms(self.store, model, no_adjacent, {"PROBE": np.eye(16)[0]}), [])


if __name__ == "__main__":
    unittest.main()
