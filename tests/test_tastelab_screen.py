"""Offline content-screen and calibration checks using pinned fake probes."""
from __future__ import annotations

import sys
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools"))

try:
    import numpy as np
    from tastelab import embed, screen
    HAVE_ENV = True
except ImportError:
    HAVE_ENV = False


@unittest.skipUnless(HAVE_ENV, "needs the tastelab environment (numpy)")
class ScreenTests(unittest.TestCase):
    def setUp(self):
        self.unit = np.eye(16, dtype=np.float32)

    def mass_screen(self):
        texts = {t: self.unit[1] for t in screen.UNSAFE_PROBES}
        texts.update({t: -self.unit[0] for t in screen.SAFE_PROBES})
        texts[screen.SAFE_PROBES[0]] = self.unit[0]
        fake = embed.FakeEmbedder(texts=texts)
        fake.logit_scale = 5.0
        return screen.Screen(fake)

    def test_embeds_exact_probes_once_and_retains_thresholds(self):
        fake = embed.FakeEmbedder()
        with mock.patch.object(fake, "embed_texts", wraps=fake.embed_texts) as texts:
            got = screen.Screen(fake, threshold=0.2, strict_threshold=0.1)
            got.check(self.unit[:2], strict=False)
            got.unsafe_score(self.unit[:1])
        texts.assert_called_once_with(screen.UNSAFE_PROBES + screen.SAFE_PROBES)
        self.assertEqual((got.threshold, got.strict_threshold), (0.2, 0.1))

    def test_unsafe_argmax_raises_probability_mass_to_one(self):
        texts = {t: self.unit[1] for t in screen.UNSAFE_PROBES + screen.SAFE_PROBES}
        texts[screen.UNSAFE_PROBES[0]] = self.unit[0]
        texts[screen.SAFE_PROBES[0]] = embed.normalise(self.unit[0] + 0.01 * self.unit[1])
        fake = embed.FakeEmbedder(texts=texts)
        fake.logit_scale = 0.1
        got = screen.Screen(fake, threshold=1.0, strict_threshold=1.0)
        self.assertEqual(got.unsafe_score(self.unit[:1]).tolist(), [1.0])
        self.assertEqual(got.check(self.unit[:1], strict=False), [True])

    def test_mass_rule_and_strict_threshold(self):
        got = self.mass_screen()
        score = got.unsafe_score(self.unit[:1])
        unsafe, safe = len(screen.UNSAFE_PROBES), len(screen.SAFE_PROBES)
        expected = unsafe / (unsafe + np.exp(5) + (safe - 1) * np.exp(-5))
        self.assertEqual(score.dtype, np.float64)
        self.assertAlmostEqual(score[0], expected)
        self.assertEqual(got.check(self.unit[:1], strict=False), [False])
        self.assertEqual(got.check(self.unit[:1], strict=True), [True])
        boundary = screen.Screen(got.embedder, threshold=float(score[0]), strict_threshold=float(score[0]))
        self.assertEqual(boundary.check(self.unit[:1], strict=False), [True])

    def test_invalid_rows_fail_closed_without_affecting_valid_rows(self):
        got = self.mass_screen()
        vectors = np.stack([self.unit[0], self.unit[0] * 1.0005, self.unit[0] * 1.002,
                            np.zeros(16), np.full(16, np.nan), np.full(16, np.inf)])
        scores = got.unsafe_score(vectors)
        self.assertTrue(np.isfinite(scores[:2]).all())
        self.assertTrue(np.isnan(scores[2:]).all())
        self.assertEqual(got.check(vectors, strict=False), [False, False, True, True, True, True])
        for vectors in (np.zeros((2, 15)), np.zeros((2, 2, 16))):
            self.assertEqual(got.check(vectors, strict=False), [True, True])
        self.assertEqual(got.check([self.unit[0], np.ones(15)], strict=False), [False, True])
        self.assertEqual(got.check(np.zeros((0, 16)), strict=False), [])

    def test_uncountable_or_one_dimensional_inputs_raise(self):
        got = self.mass_screen()
        for vectors in (self.unit[0], [], ["a", "b"], np.array(["a", "b"]), 1, None, iter([self.unit[0]])):
            with self.subTest(vectors=type(vectors).__name__):
                with self.assertRaises(ValueError):
                    got.unsafe_score(vectors)
                with self.assertRaises(ValueError):
                    got.check(vectors, strict=False)

    def test_embedder_error_propagates(self):
        fake = embed.FakeEmbedder()
        with mock.patch.object(fake, "embed_texts", side_effect=RuntimeError("offline failure")):
            with self.assertRaisesRegex(RuntimeError, "offline failure"):
                screen.Screen(fake)

    def test_invalid_probe_shape_count_norm_or_value_refuses_build(self):
        fake = embed.FakeEmbedder()
        valid = fake.embed_texts(screen.UNSAFE_PROBES + screen.SAFE_PROBES)
        cases = [valid[:-1], np.zeros((len(valid), 15)), valid[np.newaxis], valid * 1.002,
                 np.zeros_like(valid), np.full_like(valid, np.nan), np.full_like(valid, np.inf)]
        for values in cases:
            with self.subTest(shape=values.shape), mock.patch.object(fake, "embed_texts", return_value=values):
                with self.assertRaisesRegex(ValueError, "screen probes"):
                    screen.Screen(fake)

    def test_negative_infinite_unsafe_probes_never_admit(self):
        got = self.mass_screen()
        probes = got._probes.copy()
        probes[:len(screen.UNSAFE_PROBES)] = -np.inf
        with mock.patch.object(got.embedder, "embed_texts", return_value=probes):
            with self.assertRaisesRegex(ValueError, "screen probes"):
                screen.Screen(got.embedder)
        got._probes = probes
        self.assertEqual(got.check(self.unit[:2], strict=False), [True, True])
        self.assertEqual(got.check(self.unit[:2], strict=True), [True, True])


@unittest.skipUnless(HAVE_ENV, "needs the tastelab environment (numpy)")
class GateTests(unittest.TestCase):
    def groups(self):
        return {name: [1.0] * screen.MIN_GROUP for name in screen.CALIBRATION_GROUPS}

    def test_nonfinite_scores_count_as_unscorable_and_never_caught(self):
        groups = self.groups()
        groups["nudity"][:4] = [np.nan, np.inf, -np.inf, screen.STRICT_THRESHOLD]
        negatives = [screen.THRESHOLD, screen.STRICT_THRESHOLD, 0.079, np.nan, np.inf, -np.inf, 0]
        result = screen.gate(groups, negatives)
        self.assertFalse(result.passed)
        self.assertEqual(result.groups["nudity"], {"n": 15, "scorable": 12, "unscorable": 3,
                                                  "caught_normal": 11, "caught_strict": 12, "missed_normal": 1,
                                                  "missed_strict": 0, "passed": False})
        self.assertEqual(result.negatives, {"n": 7, "scorable": 4, "unscorable": 3,
                                           "discarded_normal": 1, "discarded_strict": 2})

    def test_all_nan_groups_fail_at_both_approved_thresholds(self):
        groups = {name: [np.nan] * screen.GROUP_SIZE for name in screen.CALIBRATION_GROUPS}
        for thresholds in ((screen.THRESHOLD, screen.STRICT_THRESHOLD), screen.LOWERED):
            result = screen.gate(groups, [np.nan], threshold=thresholds[0], strict_threshold=thresholds[1])
            self.assertFalse(result.passed)
            for group in result.groups.values():
                self.assertEqual((group["scorable"], group["unscorable"], group["caught_normal"], group["caught_strict"]),
                                 (0, screen.GROUP_SIZE, 0, 0))

    def test_minimum_is_scorable_count(self):
        groups = {name: [1.0] * screen.MIN_GROUP + [np.nan] * 15 for name in screen.CALIBRATION_GROUPS}
        self.assertTrue(screen.gate(groups, []).passed)
        groups["gore"][0] = np.nan
        self.assertFalse(screen.gate(groups, []).passed)

    def test_one_normal_miss_is_allowed_but_two_or_a_strict_miss_fail(self):
        groups = self.groups()
        groups["gore"][0] = 0.1
        self.assertTrue(screen.gate(groups, []).passed)
        groups["gore"][1] = 0.1
        self.assertFalse(screen.gate(groups, []).passed)
        groups["gore"] = [1.0] * screen.MIN_GROUP
        groups["gore"][0] = 0.01
        self.assertFalse(screen.gate(groups, []).passed)

    def test_minimum_missing_and_additional_groups(self):
        groups = self.groups()
        groups["suggestive"].pop()
        self.assertFalse(screen.gate(groups, []).passed)
        groups = self.groups()
        del groups["gore"]
        result = screen.gate(groups, [])
        self.assertEqual(result.groups["gore"]["n"], 0)
        self.assertFalse(result.passed)
        groups = self.groups()
        groups["extra"] = [1.0]
        result = screen.gate(groups, [])
        self.assertEqual(list(result.groups), [*screen.CALIBRATION_GROUPS, "extra"])
        self.assertFalse(result.passed)

    def test_threshold_validation_for_gate_and_screen(self):
        for threshold, strict in ((0, 0), (1.1, 0.1), (0.1, 0.2), (0.1, 0),
                                  (-0.1, -0.2), (np.nan, 0.1), (0.1, np.nan)):
            with self.subTest(threshold=threshold, strict=strict):
                with self.assertRaises(ValueError):
                    screen.gate(self.groups(), [], threshold=threshold, strict_threshold=strict)
                with self.assertRaises(ValueError):
                    screen.Screen(embed.FakeEmbedder(), threshold=threshold, strict_threshold=strict)
        self.assertTrue(screen.gate(self.groups(), [], threshold=1, strict_threshold=1).passed)


@unittest.skipUnless(HAVE_ENV, "needs the tastelab environment (numpy)")
class GuardTests(unittest.TestCase):
    """The fetch's guard (owner decisions, 9 October 2026)."""

    def setUp(self):
        self.unit = np.eye(16, dtype=np.float32)
        self.names = screen.UNSAFE_PROBES + screen.SAFE_PROBES

    def real_screen(self, top):
        """The image vector unit[0] points at `top` only, so `top` is the most likely probe by far."""
        texts = {t: -self.unit[0] for t in self.names}
        texts[top] = self.unit[0]
        fake = embed.FakeEmbedder(texts=texts)
        fake.logit_scale = 5.0
        return screen.Screen(fake)

    def decide(self, row, label="Invented landscape", explicit=True):
        """The guard's decision for one probability row: `row` maps probes to probabilities, "a landscape" gets the rest."""
        probabilities = np.zeros((1, len(self.names)))
        for name, value in row.items():
            probabilities[0, self.names.index(name)] = value
        probabilities[0, self.names.index("a landscape")] += 1 - probabilities.sum()
        got = self.real_screen("a landscape")
        with mock.patch.object(got, "_softmax", return_value=(probabilities, np.argmax(probabilities, axis=1))):
            return got.guard(self.unit[:1], labels=[label], explicit=explicit)[0]

    def test_named_probes_are_unsafe_probes(self):
        self.assertIn(screen.MINOR_PROBE, screen.UNSAFE_PROBES)
        self.assertIn(screen.EXPLICIT_PROBE, screen.UNSAFE_PROBES)
        self.assertLessEqual(set(screen.GORE_PROBES), set(screen.UNSAFE_PROBES))

    def test_adult_nudity_swimwear_suggestive_and_gore_are_kept_in_both_classes(self):
        for top in ("a nude person", "a naked body", "a person in a bikini or a swimsuit", "a sexually suggestive pose",
                    "an erotic illustration", "gore with blood and open wounds", "an artwork depicting a beheading"):
            got = self.real_screen(top)
            for explicit in (True, False):
                with self.subTest(top=top, explicit=explicit):
                    self.assertEqual(got.guard(self.unit[:1], labels=["Nude Woman Standing, hand on hip"],
                                               explicit=explicit), [False])

    def test_minor_probe_as_argmax_or_at_the_threshold_discards_in_both_classes(self):
        got = self.real_screen(screen.MINOR_PROBE)
        for explicit in (True, False):
            self.assertEqual(got.guard(self.unit[:1], labels=["Invented landscape"], explicit=explicit), [True])
            self.assertFalse(self.decide({screen.MINOR_PROBE: screen.GUARD_THRESHOLD - 1e-9}, explicit=explicit))
            self.assertTrue(self.decide({screen.MINOR_PROBE: screen.GUARD_THRESHOLD}, explicit=explicit))

    def test_minor_label_with_a_sexual_label_discards_whatever_the_image_scores(self):
        for label in ("sexualized minor", "teen sex", "child pornography", "nude_teen", "1girl erotic", "2girls bikini",
                      "schoolgirl lingerie", "school_uniform panties", "teenage girl swimsuit", "loli nsfw",
                      "Nude mother with clothed child", "ｎｕｄｅ ｔｅｅｎ", "1girl masturbation", "teen masturbating",
                      "child sexualisation", "sexualization of children", "nude preteen", "nude preteens",
                      "nude prepubescent", "preteen_nude", "12yo nude", "10-year-old topless", "flat_chest nude",
                      "lolicon", "jailbait", "paedophilia", "pedophile", "nackte Mädchen", "fillette nue",
                      "niña desnuda", "少女 ヌード", "幼女 裸", "schoolgirl fucking", "1girl ejaculation",
                      "teen blowjobs", "child prostitution", "Nude 12-year-olds", "Nude 12 year olds",
                      "Nude teenaged subjects", "twelve-year-old nude", "nude, aged 14", "under-18s nude",
                      "preadolescent nude", "kiddie porn", "1girl ass", "randoseru panties", "女子高生 パンチラ",
                      "18+ teen", "1girl cum", "child sodomy", "teen fornication", "nude preschooler",
                      "nude preschoolers", "nude under age subject", "nude 14 yrs old", "nude, 14 years of age",
                      "barely legal"):
            for explicit in (True, False):
                with self.subTest(label=label, explicit=explicit):
                    self.assertTrue(self.decide({}, label=label, explicit=explicit))

    def test_minor_label_with_sexual_probe_mass_at_the_threshold_discards(self):
        for probe in ("a person in a bikini or a swimsuit", "a nude person", "a sexualized anime character"):
            with self.subTest(probe=probe):
                self.assertFalse(self.decide({probe: screen.GUARD_THRESHOLD - 1e-9}, label="children playing"))
                self.assertTrue(self.decide({probe: screen.GUARD_THRESHOLD}, label="children playing"))
        self.assertFalse(self.decide({"a bloody injury": 0.5}, label="The Massacre of the Innocents, children"))

    def test_minor_word_or_sexual_word_alone_is_kept(self):
        for label in ("Girl with a Pearl Earring", "children playing", "1girl", "Madonna and Child", "boy scouts",
                      "Nude Woman Standing", "erotic print", "bikini beach", "girlfriend nude", "boyhood landscape",
                      "Boy with a sextant", "Sussex schoolchildren", "children in a rapeseed field", "pedometer for kids",
                      "Kinderszenen", "少女 portrait", "Akt", "nu", "Naked infantry bathing", "Analysis of children's drawings",
                      "Massachusetts schoolchildren", "Children with a kinkajou", "Aged 30, nude", "Sextet of boys",
                      "Cumulus clouds over children", "Lustre ware with putti", "nude, 30 years of age"):
            with self.subTest(label=label):
                self.assertFalse(self.decide({}, label=label))

    def test_explicit_argmax_discards_class_a_only(self):
        got = self.real_screen(screen.EXPLICIT_PROBE)
        self.assertEqual(got.guard(self.unit[:1], labels=["Invented"], explicit=True), [True])
        self.assertEqual(got.guard(self.unit[:1], labels=["Invented"], explicit=False), [False])
        # Only the most likely probe counts: adult nudity with a large explicit share but a nude argmax is kept.
        self.assertFalse(self.decide({"a nude person": 0.47, screen.EXPLICIT_PROBE: 0.41}, label="adult nude"))

    def test_the_most_likely_probe_comes_from_the_logits_not_the_rounded_probabilities(self):
        # Distinct logits that round to equal probabilities: the largest logit is a safe probe, so the score stays the
        # unsafe mass, as before the guard was added (CGV-03).
        texts = {t: -self.unit[1] for t in screen.UNSAFE_PROBES}
        texts.update({t: self.unit[0] for t in screen.SAFE_PROBES})
        texts[screen.UNSAFE_PROBES[0]] = self.unit[0]
        texts[screen.SAFE_PROBES[0]] = embed.normalise(self.unit[0] + np.float32(1e-20) * self.unit[1])
        fake = embed.FakeEmbedder(texts=texts)
        fake.logit_scale = 40.0
        got = screen.Screen(fake)
        probabilities, tops = got._softmax(self.unit[1:2])
        self.assertEqual(int(tops[0]), len(screen.UNSAFE_PROBES))
        self.assertEqual(probabilities[0, 0], probabilities[0, len(screen.UNSAFE_PROBES)])
        self.assertAlmostEqual(float(got.unsafe_score(self.unit[1:2])[0]), 1 / (1 + len(screen.SAFE_PROBES)), places=9)
        self.assertEqual(got.check(self.unit[1:2], strict=False), [False])

    def test_invalid_rows_fail_closed_and_labels_must_match_rows(self):
        got = self.real_screen("a landscape")
        vectors = np.stack([self.unit[0], np.zeros(16), np.full(16, np.nan), self.unit[0] * 1.002])
        self.assertEqual(got.guard(vectors, labels=["a", "b", "c", "d"], explicit=False), [False, True, True, True])
        self.assertEqual(got.guard(np.zeros((0, 16)), labels=[], explicit=True), [])
        for labels in (["a"], "ab", b"ab"):
            with self.subTest(labels=labels), self.assertRaisesRegex(ValueError, "one label per row"):
                got.guard(self.unit[:2], labels=labels, explicit=True)
        got._probes = np.full_like(got._probes, np.nan)
        self.assertEqual(got.guard(self.unit[:1], labels=["a"], explicit=False), [True])

    def test_guard_text_splits_tags_and_normalises_width(self):
        self.assertEqual(screen.guard_text("nude_teen ｓｃｈｏｏｌ_girl"), "nude teen school girl")
        self.assertEqual(screen.guard_text(None), "")


if __name__ == "__main__":
    unittest.main()
