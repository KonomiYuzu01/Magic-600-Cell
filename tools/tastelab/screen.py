"""Zero-shot content screen with the Taste Lab embedder; no extra model (tastelab environment).

`Screen.check` marks every image to discard: sexual or suggestive content,
nudity, underwear or swimwear, anything that sexualises a character who looks
like a minor, and gore. The probes are fixed here, not owner-editable.

Rule: softmax over UNSAFE_PROBES + SAFE_PROBES with the embedder's logit scale.
An image's unsafe score is the probability mass of the unsafe probes, raised to
1.0 when the single most likely probe is unsafe. Discard when the score reaches
the threshold (STRICT_THRESHOLD for tier B and anime-style sources). The screen
fails closed: a non-finite vector or score, a wrong shape or any embedder error
discards the image. The fetch pipeline keeps only counts of discarded images.

Calibration gate (`fetch.py --calibrate`, run once with the real model before
the bulk fetch, owner approved on 1 October 2026): lawful public-domain or CC0
images from Met, AIC, NASA and Wikimedia Commons, adult subjects only, held in
memory and never stored; only aggregate counts are kept. Each positive group
passes separately: it has at least MIN_GROUP images, the normal threshold misses
at most one of them and the strict threshold misses none. If the gate fails at
(THRESHOLD, STRICT_THRESHOLD) it is judged once more at LOWERED; if it fails
again, nothing is fetched and the owner gets the counts. Anime-style suggestive
content is never used as a test case.

CALIBRATION_GROUPS, NEGATIVE_QUERIES, MIN_GROUP, GROUP_SIZE and LOWERED are the
owner-approved calibration design; change them only with the owner. A change to
the probe texts needs a new calibration (`fetch.py --calibrate`).
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Sequence

import numpy as np

THRESHOLD = 0.15
STRICT_THRESHOLD = 0.08
LOWERED = (0.10, 0.05)          # (threshold, strict threshold) for the single retry of the gate

# Fixed probe texts, read by the embedder as written (no template).
UNSAFE_PROBES: tuple = (
    "a sexually explicit image",
    "a nude person",
    "a naked body",
    "a topless person",
    "a person in underwear",
    "a person in lingerie",
    "a person in a bikini or a swimsuit",
    "a vintage photo of women in bathing costumes",    # owner decision, 9 October 2026: period swimwear is caught too
    "a sexually suggestive pose",
    "a pin-up photo",
    "an erotic illustration",
    "an anime girl in a revealing outfit",
    "a sexualized anime character",
    "fan service anime art",
    "a sexualized drawing of a young-looking character",
    "gore with blood and open wounds",
    "a dead body",
    "a bloody injury",
    "a graphic violent scene",
    "a severed body part",
    "an anatomical dissection",
)
SAFE_PROBES: tuple = (
    "a photo of an object",
    "a landscape",
    "a building",
    "a machine or a device",
    "a user interface",
    "a poster with text",
    "an abstract pattern",
    "a painting of a landscape",
    "a portrait of a fully clothed person",
    "a group of people in everyday clothes",
    "an anime character in ordinary clothes",
    "an anime background scenery",
    "a video game screenshot",
    "pixel art",
    "a diagram",
    "a city at night",
    "a product photo",
    "a scientific instrument",
    "an image of outer space",
    "an album cover",
)

# Positive calibration groups. A candidate counts for a group only when its licence is
# public domain or CC0 (sources.PUBLIC_DOMAIN_LICENCES) and its title or keywords match
# the group's pattern (case-insensitive regular expression).
CALIBRATION_GROUPS: dict = {
    "nudity": {
        "queries": (("met", "female nude"), ("met", "male nude"), ("aic", "nude"), ("wikimedia", "nude painting")),
        "match": r"\bnudes?\b|\bnaked\b|\bnudity\b",
    },
    "suggestive": {
        "queries": (("wikimedia", "pin-up girl"), ("wikimedia", "burlesque dancer photograph"),
                    ("met", "pin-up"), ("aic", "pin-up")),
        "match": r"pin-?ups?\b|burlesque|risqu|boudoir|lingerie|garter",
    },
    "underwear_swimwear": {
        "queries": (("wikimedia", "bathing beauties"), ("wikimedia", "vintage swimsuit photograph"),
                    ("wikimedia", "corset advertisement"), ("met", "bathing suit")),
        "match": r"swim ?suit|swimwear|bathing (suit|beaut|costume)|bikini|lingerie|underwear|corset|brassi[eè]re",
    },
    "gore": {
        "queries": (("wikimedia", "anatomical dissection illustration"), ("met", "beheading"),
                    ("aic", "martyrdom"), ("wikimedia", "Judith beheading Holofernes")),
        "match": r"dissect|anatom|behead|decapitat|martyr|massacre|wound|corpse|blood|severed|holofernes|[ée]corch[ée]",
    },
}
# Ordinary images for the false-discard rate (reported, not a pass criterion).
NEGATIVE_QUERIES: tuple = (("met", "teapot"), ("met", "astrolabe"), ("aic", "landscape"), ("aic", "still life"),
                           ("wikimedia", "botanical illustration"), ("wikimedia", "vintage computer"),
                           ("nasa", "nebula"), ("nasa", "mission control"))
GROUP_SIZE = 30         # positive images collected per group
NEGATIVE_SIZE = 25      # negative images collected per negative query
MIN_GROUP = 15          # a group with fewer images fails as insufficient


@dataclass
class GateResult:
    passed: bool
    threshold: float
    strict_threshold: float
    groups: dict = field(default_factory=dict)      # group -> {"n", "missed_normal", "missed_strict", "passed"}
    negatives: dict = field(default_factory=dict)   # {"n", "discarded_normal", "discarded_strict"}


def _validate_thresholds(threshold, strict_threshold):
    if not 0 < strict_threshold <= threshold <= 1:
        raise ValueError("thresholds must satisfy 0 < strict_threshold <= threshold <= 1")


def gate(scores: dict, negative_scores: Sequence[float], *, threshold: float = THRESHOLD,
         strict_threshold: float = STRICT_THRESHOLD) -> GateResult:
    """Unscorable cases are reported separately and never count as caught."""
    _validate_thresholds(threshold, strict_threshold)
    groups = {}
    for name in dict.fromkeys([*CALIBRATION_GROUPS, *scores]):
        values = np.asarray(scores.get(name, ()), dtype=np.float64)
        finite = np.isfinite(values) & (values >= 0) & (values <= 1)
        scorable = int(np.count_nonzero(finite))
        normal = int(np.count_nonzero(finite & (values < threshold)))
        strict = int(np.count_nonzero(finite & (values < strict_threshold)))
        groups[name] = {"n": len(values), "scorable": scorable, "unscorable": len(values) - scorable,
                        "caught_normal": scorable - normal, "caught_strict": scorable - strict,
                        "missed_normal": normal, "missed_strict": strict,
                        "passed": scorable >= MIN_GROUP and normal <= 1 and strict == 0}
    negatives = np.asarray(negative_scores, dtype=np.float64)
    finite = np.isfinite(negatives) & (negatives >= 0) & (negatives <= 1)
    counts = {"n": len(negatives),
              "scorable": int(np.count_nonzero(finite)), "unscorable": int(np.count_nonzero(~finite)),
              "discarded_normal": int(np.count_nonzero(finite & (negatives >= threshold))),
              "discarded_strict": int(np.count_nonzero(finite & (negatives >= strict_threshold)))}
    return GateResult(all(g["passed"] for g in groups.values()), threshold, strict_threshold, groups, counts)


class Screen:
    def __init__(self, embedder, *, threshold: float = THRESHOLD, strict_threshold: float = STRICT_THRESHOLD):
        """Embeds the probe texts once; an embedder error propagates, so a fetch stops before any download."""
        _validate_thresholds(threshold, strict_threshold)
        self.embedder = embedder
        self.threshold = threshold
        self.strict_threshold = strict_threshold
        self._probes = np.asarray(embedder.embed_texts(UNSAFE_PROBES + SAFE_PROBES), dtype=np.float64)
        if not self._valid_probes():
            raise ValueError("screen probes: expected one finite unit vector per probe, with the model's dimension")

    def _valid_probes(self) -> bool:
        return (self._probes.shape == (len(UNSAFE_PROBES) + len(SAFE_PROBES), self.embedder.dim)
                and np.isfinite(self._probes).all()
                and np.all(np.abs(np.linalg.norm(self._probes, axis=1) - 1) <= 1e-3)
                and np.isfinite(self.embedder.logit_scale) and self.embedder.logit_scale > 0)

    def check(self, vectors, *, strict: bool) -> list[bool]:
        """One flag per row of `vectors` (unit image vectors from the same embedder): True means discard."""
        scores = self.unsafe_score(vectors)
        threshold = self.strict_threshold if strict else self.threshold
        return (~np.isfinite(scores) | (scores >= threshold)).tolist()

    def unsafe_score(self, vectors) -> Sequence[float]:
        """The unsafe score per image (NaN where it cannot be computed), for the calibration gate."""
        try:
            n = len(vectors)
        except TypeError:
            raise ValueError("vectors must have countable rows") from None
        if isinstance(vectors, np.ndarray) and vectors.ndim < 2:
            raise ValueError("vectors must be a two-dimensional array")
        try:
            values = np.asarray(vectors, dtype=np.float64)
        except (TypeError, ValueError):
            if all(np.ndim(row) == 0 for row in vectors):
                raise ValueError("vectors must be a two-dimensional array") from None
            # Ragged rows retain their individual discard flags.
            values = np.full((n, self.embedder.dim), np.nan, dtype=np.float64)
            for i, row in enumerate(vectors):
                try:
                    row = np.asarray(row, dtype=np.float64)
                    if row.shape == (self.embedder.dim,):
                        values[i] = row
                except (TypeError, ValueError):
                    pass
        if values.ndim < 2:
            raise ValueError("vectors must be a two-dimensional array")
        scores = np.full(n, np.nan, dtype=np.float64)
        if not self._valid_probes() or values.ndim != 2 or values.shape[1] != self.embedder.dim:
            return scores
        with np.errstate(over="ignore", invalid="ignore", divide="ignore"):
            valid = np.isfinite(values).all(axis=1) & (np.abs(np.linalg.norm(values, axis=1) - 1) <= 1e-3)
            logits = self.embedder.logit_scale * values[valid] @ self._probes.T
            probabilities = np.exp(logits - np.max(logits, axis=1, keepdims=True))
            probabilities /= probabilities.sum(axis=1, keepdims=True)
            mass = probabilities[:, :len(UNSAFE_PROBES)].sum(axis=1)
            mass[np.isfinite(logits).all(axis=1) & (np.argmax(logits, axis=1) < len(UNSAFE_PROBES))] = 1.0
        mass[~np.isfinite(logits).all(axis=1)] = np.nan
        scores[valid] = mass
        return scores
