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
        expected = 20 / (20 + np.exp(5) + 19 * np.exp(-5))
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
        cases = [valid[:-1], np.zeros((len(valid), 15)), valid.reshape(2, 20, 16), valid * 1.002,
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


if __name__ == "__main__":
    unittest.main()
