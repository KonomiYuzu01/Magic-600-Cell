"""The taste model and the choice of the next image (tastelab environment).

Model: logistic regression (scikit-learn, balanced class weights) on the
normalised class A image embeddings of liked (1) and disliked (0) images. Skips
never count. Class B may be scored locally but never fitted. The model is ready once there are at least MIN_PER_CLASS likes and
MIN_PER_CLASS dislikes; once ready it retrains after every RETRAIN_EVERY new like
or dislike ratings, and readiness is recomputed after an undo or a removal. The
AUC is a 5-fold stratified cross-validation score, computed only when ready and
otherwise None; nothing is invented. Every training is recorded as a model run.

Stable (the stop suggestion; the owner may continue): at least
STABLE_MIN_RATINGS like or dislike ratings, an AUC that changed by less than
STABLE_DELTA over the last STABLE_WINDOW of them, and an AUC of at least
STABLE_MIN_AUC. A plateau below that AUC is reported as weak: the model does not
separate likes from dislikes yet.

Next image:
- cold start, until the model is ready: the first COLD_START images are the
  class A images nearest the k-means centres of class A embeddings, then the images
  farthest from those shown recently;
- once ready: about 1 - EXPLORE of the picks by uncertainty (probability near
  0.5) times diversity (distance to the last 50 shown), about EXPLORE uniformly
  at random;
- during the first QUOTA_UNTIL ratings every focus category keeps at least QUOTA
  of the images shown; the category is a quota label, never a model feature;
- rated (including skipped) and removed images are not shown again.

Proposals: after every PROPOSE_EVERY ratings, up to PROPOSE_MAX new search terms
from the `adjacent` lists of the categories with the highest like rate and from
probe terms whose text vectors lie nearest the centroids of liked clusters;
terms already in the plan or already proposed are skipped.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Sequence

import numpy as np
import sklearn
from sklearn.cluster import KMeans
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import roc_auc_score
from sklearn.model_selection import StratifiedKFold, cross_val_predict

from tastelab import embed

MIN_PER_CLASS = 5
RETRAIN_EVERY = 10
CV_FOLDS = 5
COLD_START = 50
QUOTA_UNTIL = 200
QUOTA = 0.08
EXPLORE = 0.2
RECENT = 50
STABLE_MIN_RATINGS = 300
STABLE_WINDOW = 100
STABLE_DELTA = 0.02
STABLE_MIN_AUC = 0.6
PROPOSE_EVERY = 100
PROPOSE_MAX = 10


@dataclass
class ModelState:
    ratings: int            # effective class A like, dislike and skip ratings
    likes: int
    dislikes: int
    ready: bool
    auc: float | None
    auc_delta: float | None
    stable: bool
    weak: bool              # plateau reached with an AUC below STABLE_MIN_AUC


@dataclass
class TrainingSet:
    """The labelled data of one training, read from the store in the store's thread."""
    shas: list              # labelled images (like or dislike with an embedding), in rating order
    X: object               # (n, dim) float32 unit vectors
    y: object               # (n,) int: 1 like, 0 dislike
    ratings: int            # effective like, dislike and skip ratings
    serial: int             # increases with every snapshot of one TasteModel


@dataclass
class Fit:
    data: TrainingSet
    classifier: object | None   # fitted scikit-learn estimator; None while not ready
    auc: float | None
    params: dict


def _vectors(blobs):
    blobs = list(blobs)
    return embed.from_blobs(blobs, len(blobs[0]) // 4 if blobs else 0)


class TasteModel:
    """The owner's taste model for one embedding model id.

    A Store belongs to one thread, so training is split for the rating window:
    `snapshot` reads the store, `train` is pure and may run in a worker thread,
    `apply` adopts the result in the store's thread. `refresh` does all three
    when a training is due."""

    def __init__(self, store, model_id: str, *, record: bool = True):
        """`record=False` lets the taste map fit without writing a model run."""
        self.store = store
        self.model_id = model_id
        self.record = record
        self._serial = 0
        self._fit = None
        self._state = None

    def _examples(self):
        eligible = {m.sha256 for m in self.store.images(tier="A") if not self.store.is_blocked(m.sha256)}
        ratings = [r for r in self.store.ratings() if r.sha256 in eligible]
        labelled = [r for r in ratings if r.verdict in ("like", "dislike")]
        blobs = self.store.embeddings(self.model_id, [r.sha256 for r in labelled])
        pairs = [(r.sha256, int(r.verdict == "like")) for r in labelled if r.sha256 in blobs]
        return ratings, pairs, blobs

    def due(self, *, force: bool = False) -> bool:
        """True when forced, before the first training, after RETRAIN_EVERY new like or dislike ratings,
        at a class threshold or second class, or when an undo, removal or changed verdict invalidates the fit."""
        if force or self._fit is None:
            return True
        _, pairs, _ = self._examples()
        adopted = set(zip(self._fit.data.shas, np.asarray(self._fit.data.y).tolist()))
        old = [sum(label == c for _, label in adopted) for c in (0, 1)]
        current = [sum(label == c for _, label in pairs) for c in (0, 1)]
        crossed = any(before < MIN_PER_CLASS <= after for before, after in zip(old, current))
        second_class = min(old) == 0 and min(current) > 0
        return (crossed or second_class or len(pairs) - len(adopted) >= RETRAIN_EVERY
                or not adopted.issubset(set(pairs)))

    def snapshot(self) -> TrainingSet:
        ratings, pairs, blobs = self._examples()
        self._serial += 1
        shas = [sha for sha, _ in pairs]
        return TrainingSet(shas, _vectors(blobs[sha] for sha in shas),
                           np.asarray([label for _, label in pairs], dtype=int), len(ratings), self._serial)

    @staticmethod
    def train(data: TrainingSet) -> Fit:
        """Fit and cross-validate when ready; touches no store and no shared state."""
        y = np.asarray(data.y, dtype=int)
        params = {"C": 1.0, "folds": CV_FOLDS, "labelled": len(y), "sklearn_version": sklearn.__version__}
        if np.count_nonzero(y == 1) < MIN_PER_CLASS or np.count_nonzero(y == 0) < MIN_PER_CLASS:
            return Fit(data, None, None, params)
        X = np.asarray(data.X, dtype=np.float64)
        classifier = LogisticRegression(class_weight="balanced", C=1.0, max_iter=2000)
        cv = StratifiedKFold(n_splits=CV_FOLDS, shuffle=True, random_state=0)
        probabilities = cross_val_predict(classifier, X, y, cv=cv, method="predict_proba")[:, 1]
        auc = float(roc_auc_score(y, probabilities))
        classifier.fit(X, y)
        return Fit(data, classifier, auc, params)

    def apply(self, fit: Fit) -> ModelState:
        """Adopt a fit (ignored when an older snapshot than the adopted one), record the model run, return the state."""
        if self._fit is not None and fit.data.serial <= self._fit.data.serial:
            return self.state()
        likes = int(np.count_nonzero(np.asarray(fit.data.y) == 1))
        dislikes = len(fit.data.y) - likes
        runs = [r for r in self.store.model_runs() if r["model"] == self.model_id]
        previous = next((r for r in reversed(runs) if r["auc"] is not None
                         and r["likes"] + r["dislikes"] <= likes + dislikes - STABLE_WINDOW), None)
        delta = fit.auc - previous["auc"] if fit.auc is not None and previous is not None else None
        plateau = likes + dislikes >= STABLE_MIN_RATINGS and delta is not None and abs(delta) < STABLE_DELTA
        ready = fit.classifier is not None
        self._state = ModelState(fit.data.ratings, likes, dislikes, ready, fit.auc, delta,
                                 bool(plateau and fit.auc >= STABLE_MIN_AUC),
                                 bool(plateau and fit.auc < STABLE_MIN_AUC))
        self._fit = fit
        latest = runs[-1] if runs else None
        if self.record and ready and (latest is None or (latest["likes"], latest["dislikes"]) != (likes, dislikes)):
            self.store.add_model_run(self.model_id, fit.data.ratings, likes, dislikes, fit.auc, delta,
                                     self._state.stable, fit.params)
        return self._state

    def refresh(self, *, force: bool = False) -> ModelState:
        """apply(train(snapshot())) when due; returns the current state."""
        if self.due(force=force):
            return self.apply(self.train(self.snapshot()))
        return self.state()

    def state(self) -> ModelState:
        """The state of the latest adopted fit (counts only and not ready before the first)."""
        if self._state is not None:
            return self._state
        ratings, pairs, _ = self._examples()
        likes = sum(label for _, label in pairs)
        return ModelState(len(ratings), likes, len(pairs) - likes, False, None, None, False, False)

    def predict(self, shas: Sequence[str]):
        """Probability of like per stored image (numpy array); 0.5 while not ready or without an embedding."""
        shas = list(shas)
        probabilities = np.full(len(shas), 0.5, dtype=np.float64)
        if self._fit is not None and self._fit.classifier is not None:
            blobs = self.store.embeddings(self.model_id, shas)
            indices = [i for i, sha in enumerate(shas) if sha in blobs]
            if indices:
                probabilities[indices] = self.predict_vectors(_vectors(blobs[shas[i]] for i in indices))
        return probabilities

    def predict_vectors(self, vectors):
        """Probability of like per row of unit vectors from the same embedder; 0.5 while not ready."""
        if self._fit is None or self._fit.classifier is None or len(vectors) == 0:
            return np.full(len(vectors), 0.5, dtype=np.float64)
        return np.asarray(self._fit.classifier.predict_proba(np.asarray(vectors, dtype=np.float64))[:, 1],
                          dtype=np.float64)

    def direction(self):
        """The unit preference direction in embedding space, or None while not ready."""
        if self._fit is None or self._fit.classifier is None:
            return None
        coefficient = np.asarray(self._fit.classifier.coef_[0], dtype=np.float64)
        norm = np.linalg.norm(coefficient)
        return coefficient / norm if norm else None


class Selector:
    def __init__(self, store, model: TasteModel, plan, *, seed: int | None = None):
        self.store = store
        self.model = model
        self.plan = plan
        self._rng = np.random.default_rng(seed)
        self._reserved = set()
        self._shown = set()
        self._recent = []
        self._seed = seed
        self._signature = None
        self._load()

    def _load(self):
        self._images = {m.sha256: m for m in self.store.images() if not self.store.is_blocked(m.sha256)}
        blobs = self.store.embeddings(self.model.model_id)
        signature = tuple((sha, m.tier, m.category, blobs[sha]) for sha, m in self._images.items() if sha in blobs)
        if signature == self._signature:
            return
        self._signature = signature
        self._shas = [sha for sha in self._images if sha in blobs]
        self._indices = {sha: i for i, sha in enumerate(self._shas)}
        self._X = _vectors(blobs[sha] for sha in self._shas)
        self._cold = []
        training = [sha for sha in self._shas if self._images[sha].tier == "A"]
        if training:
            X = self._X[[self._indices[sha] for sha in training]]
            clusters = KMeans(n_clusters=min(COLD_START, len(training)), n_init=1, random_state=self._seed).fit(X)
            sizes = np.bincount(clusters.labels_, minlength=len(clusters.cluster_centers_))
            distances = clusters.transform(X)
            self._cold = [training[int(np.argmin(distances[:, i]))] for i in np.argsort(-sizes, kind="stable")]

    def next(self, n: int = 1, exclude: Sequence[str] = ()) -> list[str]:
        """Up to `n` images to show next, by the rules above."""
        self._load()
        rated = self.store.rated().intersection(self._images)
        self._reserved.difference_update(rated)
        self._reserved.intersection_update(self._indices)
        blocked = rated | self._reserved | set(exclude)
        candidates = [sha for sha in self._shas if sha not in blocked]
        shown = self._shown.intersection(self._images)
        recent = self._recent[-RECENT:]
        picks = []
        ready = self.model.state().ready
        for _ in range(max(0, n)):
            if not candidates:
                break
            pool = candidates
            if len(rated) < QUOTA_UNTIL:
                seen = rated | shown
                target = QUOTA * (len(seen) + 1)
                counts = {category: sum(self._images[sha].category == category for sha in seen)
                          for category in self.plan.focus()}
                behind = [category for category in self.plan.focus() if counts[category] < target
                          and any(self._images[sha].category == category for sha in pool)]
                if behind:
                    category = min(behind, key=lambda c: counts[c])
                    pool = [sha for sha in pool if self._images[sha].category == category]
            pick = None
            if not ready and len(rated | shown) < COLD_START:
                pick = next((sha for sha in self._cold if sha in pool and sha not in shown), None)
            if pick is None:
                if ready and self._rng.random() < EXPLORE:
                    pick = pool[int(self._rng.integers(len(pool)))]
                else:
                    indices = [self._indices[sha] for sha in pool]
                    previous = [self._indices[sha] for sha in recent[-RECENT:] if sha in self._indices]
                    similarity = np.max(self._X[indices] @ self._X[previous].T, axis=1) if previous else np.zeros(len(pool))
                    if ready:
                        uncertainty = 1 - 2 * np.abs(self.model.predict(pool) - 0.5)
                        values = uncertainty * np.maximum(0, 1 - similarity)
                    else:
                        values = -similarity
                    pick = pool[int(np.argmax(values))]
            picks.append(pick)
            candidates.remove(pick)
            shown.add(pick)
            recent = [*recent[-(RECENT - 1):], pick]
        self._reserved.update(picks)
        return picks

    def shown(self, sha: str) -> None:
        """Record that an image was shown (diversity and category quota)."""
        self._reserved.discard(sha)
        self._shown.add(sha)
        self._recent = [*self._recent[-(RECENT - 1):], sha]


def proposals_due(store) -> bool:
    """True when class A ratings passed a multiple of PROPOSE_EVERY since the last proposal round."""
    eligible = {m.sha256 for m in store.images(tier="A") if not store.is_blocked(m.sha256)}
    total = sum(r.sha256 in eligible for r in store.ratings())
    return total >= PROPOSE_EVERY and total // PROPOSE_EVERY > store.last_proposal_rating() // PROPOSE_EVERY


def propose_terms(store, model: TasteModel, plan, probe_vectors: dict) -> list[tuple]:
    """Make one proposal round: up to PROPOSE_MAX (term, category or None, origin) tuples.

    `probe_vectors` maps probe text to its unit vector. The new terms are stored
    with store.add_proposals and the round with store.mark_proposal_round, both at
    the current effective class A rating count."""
    images = {m.sha256: m for m in store.images(tier="A") if not store.is_blocked(m.sha256)}
    ratings = [r for r in store.ratings() if r.sha256 in images]
    rates = {}
    for category in plan.categories:
        labelled = [r for r in ratings if r.verdict in ("like", "dislike")
                    and images[r.sha256].category == category]
        if labelled:
            rates[category] = sum(r.verdict == "like" for r in labelled) / len(labelled)
    adjacent = [(term, category, "adjacent") for category in sorted(rates, key=lambda c: -rates[c])
                for term in plan.categories[category].adjacent]
    probes = []
    blobs = store.embeddings(model.model_id, [r.sha256 for r in ratings if r.verdict == "like"])
    if len(blobs) >= 5 and probe_vectors:
        X = _vectors(blobs.values())
        clusters = KMeans(n_clusters=min(5, len(blobs) // 5), n_init=1, random_state=0).fit(X)
        sizes = np.bincount(clusters.labels_, minlength=len(clusters.cluster_centers_))
        terms = list(probe_vectors)
        vectors = np.asarray([probe_vectors[t] for t in terms])
        for i in np.argsort(-sizes, kind="stable"):
            centre = embed.normalise(clusters.cluster_centers_[i])
            probes.append((terms[int(np.argmax(vectors @ centre))], None, "probe"))
    known = {t.text.casefold() for c in plan.categories.values() for t in c.terms}
    known.update(p["term"].casefold() for p in store.proposals())
    pools = [iter(adjacent), iter(probes)]
    picked = []
    while len(picked) < PROPOSE_MAX:
        added = False
        for pool in pools:
            for item in pool:
                if item[0].casefold() not in known:
                    picked.append(item)
                    known.add(item[0].casefold())
                    added = True
                    break
            if len(picked) == PROPOSE_MAX:
                break
        if not added:
            break
    total = len(ratings)
    store.add_proposals(picked, at_rating=total)
    store.mark_proposal_round(total)
    return picked
