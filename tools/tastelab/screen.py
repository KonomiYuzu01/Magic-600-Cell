"""Zero-shot content guard and screen with the Taste Lab embedder; no extra model (tastelab environment).

`Screen.guard` is what the fetch applies (owner decisions, 9 October 2026: class A
images from the approved sources are all acceptable, nudity, swimwear, suggestive
content and gore included; class B gets the minor-protection checks only; the ban
on sexualized minors and young-looking characters always stays). It discards an
image when the probe for sexualized young-looking characters is the most likely
probe or reaches GUARD_THRESHOLD; when its label names a minor and either the label
names nudity or sexual content or the sexual probes together reach GUARD_THRESHOLD;
and, for class A only (its thumbnails may go to the claude.ai page), when "a
sexually explicit image" is the most likely probe. GUARD_THRESHOLD and the
argmax rule are heuristics: no lawful test set exists for them, and none is built.

`Screen.check`, the full screen measured by the calibration tool and no longer
applied by the fetch, marks every image to discard: sexual or suggestive content,
nudity, underwear or swimwear, anything that sexualises a character who looks
like a minor, and gore. The probes are fixed here, not owner-editable.

Rule: softmax over UNSAFE_PROBES + SAFE_PROBES with the embedder's logit scale.
An image's unsafe score is the probability mass of the unsafe probes, raised to
1.0 when the single most likely probe is unsafe. Discard when the score reaches
the threshold (STRICT_THRESHOLD for tier B and anime-style sources). The screen
fails closed: a non-finite vector or score, a wrong shape or any embedder error
discards the image. The fetch pipeline keeps only counts of discarded images.

Calibration gate (`fetch.py --calibrate`, owner approved on 1 October 2026; since
9 October 2026 a measurement of the full screen, not a precondition of the fetch): lawful public-domain or CC0
images from Met, AIC, NASA and Wikimedia Commons, adult subjects only, held in
memory and never stored; only aggregate counts are kept. Each positive group
passes separately: it has at least MIN_GROUP images, the normal threshold misses
at most one of them and the strict threshold misses none. If the gate fails at
(THRESHOLD, STRICT_THRESHOLD) it is judged once more at LOWERED. The report
keeps the counts only. Anime-style suggestive content is never used as a test case.

CALIBRATION_GROUPS, NEGATIVE_QUERIES, MIN_GROUP, GROUP_SIZE and LOWERED are the
owner-approved calibration design; change them only with the owner.
"""
from __future__ import annotations

import re
import unicodedata
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
    "an artwork depicting a beheading",               # owner decision, 9 October 2026: depicted beheadings are caught too
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

# The guard (owner decisions, 9 October 2026). GUARD_THRESHOLD is a heuristic bar, not a calibrated one.
GUARD_THRESHOLD = STRICT_THRESHOLD
MINOR_PROBE = "a sexualized drawing of a young-looking character"
EXPLICIT_PROBE = "a sexually explicit image"
GORE_PROBES: tuple = ("gore with blood and open wounds", "a dead body", "a bloody injury", "a graphic violent scene",
                      "a severed body part", "an artwork depicting a beheading", "an anatomical dissection")
# Label words, matched after guard_text(). They supplement the minor probe; they are not a complete vocabulary. Booru
# counts such as "1girl" or "2boys" name characters too. Words that name sexual content involving minors by themselves
# ("lolicon", "jailbait", "paedophilia") are in both lists, so that they discard alone. A few common French, German,
# Italian, Spanish and Japanese words are included; Japanese has no word boundaries.
MINOR_WORDS = re.compile(
    r"\b(?:\d*girls?|\d*boys?|girlhood|boyhood|child(?:ren|hood|ish|like)?|kid(?:s|die|dies|do|dy)?|infant(?:s|ile)?|"
    r"bab(?:y|ies)|baby[- ]?fac\w*|young[- ]looking|barely[- ]legal|toddlers?|newborns?|youngsters?|preschool\w*|"
    r"(?:pre-?)?teen(?:age|aged|ager)?s?|tweens?|(?:pre-?)?pubescen\w*|pubert\w*|pa?ederast\w*|"
    r"(?:pre-?)?adolescen\w*|juveniles?|minors?|youths?|school[- ]?(?:girl|boy|child(?:ren)?|kid)s?|school[- ]uniforms?|"
    r"serafuku|randoseru|kindergarten\w*|(?:elementary|primary|middle|nursery|grade|high)[- ]?school\w*|junior high\w*|"
    r"(?:first|second|third|fourth|fifth|sixth|seventh|eighth|1st|2nd|3rd|[4-8]th)[- ]graders?|\d+[- ]?months?[- ]olds?|"
    r"(?:[1-9]|1[0-7]|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fourteen|fifteen|"
    r"sixteen|seventeen)(?: ?yos?| ?y/o|[- ]?(?:years?|yrs?)[- ]?olds?|[- ]?years? of age)|aged? (?:[1-9]|1[0-7])|"
    r"under[- ]?1[0-8]s?|"
    r"lolis?|lolita\w*|lolicon|shotas?|shotacon|nymphets?|putt[oi]|cherubs?|cupids?|amor(?:ino|ini|etto|etti)|"
    r"under[- ]?age(?:d|s)?|jailbait|flat[- ]?chest\w*|pa?edo(?:s|phil\w*)?|m[äa]dchen|knaben?|kinder|enfants?|fillettes?|"
    r"gar[çc]ons?|ni[ñn][oa]s?|bambin[oaie]|ragazz[oaie]|fanciull[oaie])\b"
    r"|少女|少年|幼女|幼児|児童|子供|子ども|ロリ|ショタ|小学生|中学生|女子高生", re.I)
SEXUAL_WORDS = re.compile("|".join([*(f"(?:{CALIBRATION_GROUPS[name]['match']})"
                                      for name in ("nudity", "suggestive", "underwear_swimwear")),
                                    r"\b(?:sex(?:es|ed|y|ier|iest|ing|ts?|ting|ual\w*|ism|ist)?|porn\w*|xxx|x-?rated|"
                                    r"explicit|nsfw|r-?18g?|hentai|ecchi|eroge|lewd\w*|obscen\w*|indecen\w*|eroti[cs]\w*|"
                                    r"sensual\w*|seduc\w*|suggestive|fetish\w*|kinky|kinks?|bdsm|bondage|shibari|"
                                    r"aroused?|arousal|horny|voyeur\w*|exhibitionis\w*|naked\w*|nud(?:e|es|ity|ist\w*|ism)|"
                                    r"naturis\w*|skinny[- ]?dip\w*|undress\w*|unclothed|disrob\w*|topless|bottomless|"
                                    r"nipples?|areolae?|pubic|breasts?|boob\w*|tits|titty|titties|ass|asses|butt|"
                                    r"buttocks?|anus|anal|crotch|groin|genital\w*|penis\w*|phall(?:us|i|ic)|vagina\w*|"
                                    r"vulva\w*|pussy|pussies|cunts?|twats?|clit(?:s|oris|oral)?|testicl\w*|scrot\w*|semen|cum(?:s|med|[- ]?shot\w*|ming)?|"
    r"ejaculat\w*|orgasm\w*|sodom\w*|fornicat\w*|adulter(?:y|ous|er|ers|ess)|debauch\w*|lecher\w*|lust|lustful|"
    r"carnal\w*|libid\w*|nymphomani\w*|concubin\w*|harems?|pa?ederast\w*|barely[- ]legal|"
                                    r"masturbat\w*|intercourse|coitus|copulat\w*|fuck\w*|fellatio|cunnilingus|"
                                    r"(?:blow|hand|foot|rim)[- ]?jobs?|deep[- ]?throat\w*|orgy|orgies|gang[- ]?bang\w*|"
                                    r"threesomes?|bukkake|creampies?|dildos?|vibrators?|condoms?|prostitut\w*|brothels?|"
                                    r"whor(?:e|es|ing)|harlots?|sluts?|slutty|hookers?|strippers?|striptease\w*|"
                                    r"lap[- ]?danc\w*|panties|panty|pantsu|pantyshot\w*|panchira|upskirt\w*|downblouse|"
                                    r"cameltoe\w*|knickers|underpants|thongs?|g-strings?|bras?|cleavage|"
                                    r"spread(?:ing)?[- ]legs|legs[- ]spread|ahegao|grop(?:e|ed|es|ing)|molest\w*|"
                                    r"rap(?:e|ed|es|ing|ist|ists)|incest\w*|lolicon|shotacon|jailbait|pa?edo(?:s|phil\w*)?|"
                                    r"nackt\w*|akt|desnud[oa]s?|nud[oaie]|nue?s?)\b",
                                    r"\b18 ?\+",
                                    r"裸|ヌード|エロ|性的|性交|セックス|18禁|猥褻|わいせつ|ポルノ|売春|援交|援助交際|痴漢|レイプ|"
                                    r"自慰|オナニー|射精|フェラ|パンチラ|水着|下着|おっぱい|乳首|ブルマ"]), re.I)


def guard_text(label) -> str:
    """NFKC, then underscores as spaces, so that source tags such as "nude_teen" split into words."""
    return unicodedata.normalize("NFKC", label or "").replace("_", " ")


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

    def guard(self, vectors, *, labels, explicit: bool) -> list[bool]:
        """One flag per row of `vectors` and its label (title and keywords): True means discard. The rules are in the
        module docstring; `explicit` adds the class A check."""
        probabilities, tops = self._softmax(vectors)
        if isinstance(labels, (str, bytes)) or len(labels := list(labels)) != len(probabilities):
            raise ValueError("guard needs one label per row")
        names = UNSAFE_PROBES + SAFE_PROBES
        minor, explicit_probe = names.index(MINOR_PROBE), names.index(EXPLICIT_PROBE)
        sexual = [i for i, name in enumerate(UNSAFE_PROBES) if name not in GORE_PROBES]
        flags = []
        for row, top, label in zip(probabilities, tops, labels):
            if not np.isfinite(row).all():
                flags.append(True)
                continue
            text = guard_text(label)
            flags.append(bool(top == minor or row[minor] >= GUARD_THRESHOLD
                              or MINOR_WORDS.search(text) and (SEXUAL_WORDS.search(text)
                                                               or row[sexual].sum() >= GUARD_THRESHOLD)
                              or explicit and top == explicit_probe))
        return flags

    def unsafe_score(self, vectors) -> Sequence[float]:
        """The unsafe score per image (NaN where it cannot be computed), for the calibration gate."""
        probabilities, tops = self._softmax(vectors)
        scores = probabilities[:, :len(UNSAFE_PROBES)].sum(axis=1)     # NaN rows stay NaN
        scores[(tops >= 0) & (tops < len(UNSAFE_PROBES))] = 1.0
        return scores

    def _softmax(self, vectors):
        """Softmax over UNSAFE_PROBES + SAFE_PROBES per row, and the index of each row's largest logit (not of its
        rounded probability). A row whose vector or logits are invalid is NaN, with index -1."""
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
        probabilities = np.full((n, len(UNSAFE_PROBES) + len(SAFE_PROBES)), np.nan, dtype=np.float64)
        tops = np.full(n, -1, dtype=np.int64)
        if not self._valid_probes() or values.ndim != 2 or values.shape[1] != self.embedder.dim:
            return probabilities, tops
        with np.errstate(over="ignore", invalid="ignore", divide="ignore"):
            valid = np.isfinite(values).all(axis=1) & (np.abs(np.linalg.norm(values, axis=1) - 1) <= 1e-3)
            logits = self.embedder.logit_scale * values[valid] @ self._probes.T
            finite = np.isfinite(logits).all(axis=1)
            rows = np.exp(logits - np.max(logits, axis=1, keepdims=True))
            rows /= rows.sum(axis=1, keepdims=True)
        scored = np.flatnonzero(valid)[finite]
        probabilities[scored] = rows[finite]
        tops[scored] = np.argmax(logits[finite], axis=1)
        return probabilities, tops
