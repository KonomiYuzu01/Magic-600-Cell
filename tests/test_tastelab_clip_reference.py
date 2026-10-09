"""C-02 fixture plumbing and refusal tests: no torch, real weights, or network.

The round trip uses test_tastelab_clip's tiny checkpoint. Its token embeddings
are repeated to fit the real Tokenizer's vocabulary. TinyAdapter.preprocess
shrinks to the checkpoint's 8 x 8 size, then repeats those pixels to 224 x 224
only to exercise the fixed fixture format; encode_images undoes the repeat.
This round trip is synthetic evidence, not an upstream-equivalence claim.
"""
from __future__ import annotations

import copy
import hashlib
import io
import json
import math
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock
from uuid import uuid4

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

try:
    import ftfy
    import numpy as np
    from PIL import __version__ as PILLOW_VERSION
    from tastelab import clip, clipref, common, embed, images, openclip_reference
    from test_tastelab_clip import tiny_checkpoint, write_safetensors
    HAVE_ENV = True
except ImportError:
    HAVE_ENV = False


if HAVE_ENV:
    class TinyAdapter(clipref.NumpyAdapter):
        def preprocess(self, image):
            with mock.patch.object(clip, "IMAGE_SIZE", self.model.image_size):
                value = super().preprocess(image)
            scale = 224 // self.model.image_size
            return np.repeat(np.repeat(value, scale, axis=1), scale, axis=2)

        def encode_images(self, pixels):
            scale = 224 // self.model.image_size
            return super().encode_images(pixels[:, :, ::scale, ::scale])


def rotate_row(value, cosine):
    """Keep the norm and give a row a controlled, orthogonal component."""
    row = np.asarray(value, dtype=np.float64)
    unit = row / np.linalg.norm(row)
    other = np.zeros_like(unit)
    other[np.argmin(np.abs(unit))] = 1
    other -= np.dot(other, unit) * unit
    other /= np.linalg.norm(other)
    return ((cosine * unit + math.sqrt(1 - cosine * cosine) * other) * np.linalg.norm(row)).astype(np.float32)


@unittest.skipUnless(HAVE_ENV, "needs the Taste Lab environment")
class ReferenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.folder = Path(tempfile.gettempdir()) / f"tastelab-test-{uuid4().hex}"
        cls.folder.mkdir()
        cls.addClassCleanup(shutil.rmtree, cls.folder, ignore_errors=True)
        checkpoint = tiny_checkpoint()
        checkpoint["token_embedding.weight"] = np.tile(checkpoint["token_embedding.weight"], (989, 1))[:49408].copy()
        weights = cls.folder / "tiny.safetensors"
        write_safetensors(weights, checkpoint)
        merges = cls.folder / "merges.txt"
        merges.write_text("# synthetic\n" + "\n".join(f"left{i} right{i}" for i in range(clip.MERGES)) + "\n", encoding="utf-8")
        pins = {"weights_sha256": hashlib.sha256(weights.read_bytes()).hexdigest(), "weights_size": weights.stat().st_size,
                "merges_sha256": hashlib.sha256(merges.read_bytes()).hexdigest()}
        cls.adapter = TinyAdapter(clip.load(weights), clip.Tokenizer(merges), pins)
        cls.producer = {"python": "synthetic", "platform": "test", "torch": "absent", "torchvision": "absent", "open_clip": "absent",
                        "pillow": PILLOW_VERSION, "numpy": np.__version__,
                        "transform": {"resize_size": [8], "interpolation": "bicubic", "crop_size": [8, 8],
                                      "mean": clipref.encode_floats(clip.MEAN), "std": clipref.encode_floats(clip.STD)}}
        cls.fixture = clipref.build_fixture(cls.adapter, cls.producer)

    def assert_named_failure(self, fixture, name, status="mismatch"):
        report = clipref.compare(fixture, self.adapter, provenance=False)
        self.assertEqual(report["status"], status, report)
        checks = {row["name"]: row for row in report["checks"]}
        self.assertIn(name, checks)
        self.assertEqual(checks[name]["status"], status, checks[name])
        self.assertGreater(checks[name]["maximum"], 0)
        return checks[name]

    def test_cases_are_deterministic_and_cover_image_categories(self):
        first, second = clipref.cases(), clipref.cases()
        self.assertEqual(clipref.cases_digest(first), clipref.cases_digest(second))
        self.assertEqual([clipref.input_sha256(c) for c in first["images"] + first["texts"]],
                         [clipref.input_sha256(c) for c in second["images"] + second["texts"]])
        images_by_id = {case["id"]: case["image"] for case in first["images"]}
        sources = {case["id"]: case["source"] for case in first["images"]}
        self.assertGreaterEqual(len(images_by_id), 16)
        self.assertEqual({im.mode for im in sources.values()}, {"RGB", "L", "LA", "P", "RGBA", "CMYK"})
        for name, image in images_by_id.items():
            self.assertEqual(image.mode, "RGB", name)
            self.assertEqual(image.tobytes(), images.to_rgb(sources[name]).tobytes(), name)
        for name, size in (("rgb-224-gradient", (224, 224)), ("rgb-odd-wide", (225, 223)), ("rgb-odd-tall", (333, 517)),
                           ("rgb-small", (17, 31)), ("rgb-review-stripes", (1025, 100))):
            self.assertEqual(images_by_id[name].size, size)
        for name in ("rgb-max-wide", "rgb-max-tall"):
            w, h = images_by_id[name].size
            self.assertEqual(max(w, h), images.MAX_ASPECT * min(w, h))
        self.assertGreater(images_by_id["rgb-max-wide"].width, images_by_id["rgb-max-wide"].height)
        self.assertGreater(images_by_id["rgb-max-tall"].height, images_by_id["rgb-max-tall"].width)
        checker = np.asarray(images_by_id["rgb-checker-1px"])
        self.assertEqual(checker[:2, :2, 0].tolist(), [[0, 255], [255, 0]])
        self.assertEqual(np.asarray(images_by_id["rgb-stripes-1px-x"])[0, :4, 0].tolist(), [0, 255, 0, 255])
        self.assertEqual(np.asarray(images_by_id["rgb-stripes-1px-y"])[:4, 0, 0].tolist(), [0, 255, 0, 255])
        for name in ("la-alpha", "rgba-alpha"):
            self.assertGreater(len(set(sources[name].getchannel("A").tobytes())), 2)
        self.assertEqual(sources["p-transparent"].info["transparency"], 0)
        self.assertIn(0, sources["p-transparent"].tobytes())
        self.assertEqual(sources["p-transparent"].getpalette(), {c["id"]: c["source"] for c in second["images"]}["p-transparent"].getpalette())
        # Transparent pixels reach both sides flattened onto white, as production stores them.
        self.assertTrue((np.asarray(images_by_id["p-transparent"]) == 255).all(axis=-1).any())
        self.assertNotEqual(images_by_id["rgb-hash"].tobytes()[:48], bytes(48))
        self.assertGreater(len(set(images_by_id["rgb-rings"].tobytes())), 2)
        self.assertEqual(len(first["batches"]), 2)
        image_batch = first["batches"][0]
        self.assertGreater(len({images_by_id[name].size for name in image_batch["ids"]}), 1)

    def test_captions_cover_unicode_cleaning_boundaries_and_mixed_batch(self):
        texts = {case["id"]: case["text"] for case in clipref.cases()["texts"]}
        self.assertGreaterEqual(len(texts), 16)
        self.assertEqual(texts["empty"], "")
        self.assertTrue(texts["ascii"].isascii())
        self.assertIn("\t", texts["case-whitespace"])
        self.assertIn("\r\n", texts["case-whitespace"])
        for name in ("html", "html-double"):
            self.assertIn("&amp;", texts[name])
        self.assertNotEqual(ftfy.fix_text(texts["ftfy"]), texts["ftfy"])
        for name in ("latin", "cjk", "emoji", "non-bmp", "combining", "rtl"):
            self.assertFalse(texts[name].isascii())
        for name in ("emoji", "non-bmp"):
            self.assertTrue(any(ord(c) > 0xFFFF for c in texts[name]))
        self.assertTrue(any(c.isdigit() for c in texts["digits"]))
        self.assertIn("!!!", texts["punctuation"])
        for suffix in ("'s", "'t", "'re", "'ve", "'m", "'ll", "'d"):
            self.assertIn(suffix, texts["contractions"].lower())
        self.assertEqual(texts["literal-special"], "<start_of_text>")
        tokenizer = self.adapter.tokenizer
        self.assertEqual(len(tokenizer.encode(texts["tokens-75"])), 75)
        self.assertGreater(len(tokenizer.encode(texts["tokens-long"])), 77)
        values = tokenizer.tokenize([texts["tokens-75"], texts["tokens-long"]])
        self.assertEqual(values.shape, (2, 77))
        self.assertEqual(values[:, -1].tolist(), [tokenizer.eot, tokenizer.eot])
        batch_ids = clipref.cases()["batches"][1]["ids"]
        self.assertIn("empty", batch_ids)
        self.assertIn("tokens-long", batch_ids)
        self.assertGreater(len({len(texts[name]) for name in batch_ids}), 1)

    def test_cases_digest_equals_the_committed_digest(self):
        self.assertEqual(clipref.cases_digest(clipref.cases()), clipref.CASES_DIGEST)

    def test_production_decode_of_a_png_source_gives_the_case_image(self):
        sources = {case["id"]: case for case in clipref.cases()["images"]}
        for name in ("l-gradient", "p-transparent", "rgba-alpha", "rgb-hash"):
            case = sources[name]
            data = io.BytesIO()
            case["source"].save(data, format="PNG", **({"transparency": 0} if case["source"].mode == "P" else {}))
            with self.subTest(name=name):
                self.assertEqual(images.decode(data.getvalue()).image.tobytes(), case["image"].tobytes())

    def test_image_digest_covers_mode_size_pixels_and_palette(self):
        from PIL import Image
        original = {"image": Image.frombytes("L", (2, 1), b"\x00\xff")}
        variants = ({"image": Image.frombytes("L", (1, 2), b"\x00\xff")},
                    {"image": Image.frombytes("L", (2, 1), b"\x01\xff")},
                    {"image": Image.frombytes("P", (2, 1), b"\x00\xff")})
        for case in variants:
            if case["image"].mode == "P":
                case["image"].putpalette(bytes(range(256)) * 3)
                case["image"].info["transparency"] = 0
            self.assertNotEqual(clipref.input_sha256(original), clipref.input_sha256(case))
        pcase = variants[2]
        before = clipref.input_sha256(pcase)
        palette = pcase["image"].getpalette()
        palette[0] ^= 1
        pcase["image"].putpalette(palette)
        self.assertNotEqual(before, clipref.input_sha256(pcase))
        self.assertEqual(clipref.input_sha256({"text": "\U0001f9ed"}), hashlib.sha256("\U0001f9ed".encode("utf-8")).hexdigest())

    def test_sample_grid_covers_channels_bands_and_corners(self):
        indices = clipref.SAMPLE_INDICES
        self.assertGreaterEqual(len(indices), 1024)
        self.assertEqual(len(set(indices)), len(indices))
        for c in range(3):
            pixels = [(i % (224 * 224) // 224, i % 224) for i in indices if i // (224 * 224) == c]
            self.assertEqual({y * 20 // 224 for y, x in pixels}, set(range(20)))
            self.assertEqual({x * 20 // 224 for y, x in pixels}, set(range(20)))
            self.assertTrue({(0, 0), (0, 223), (223, 0), (223, 223)} <= set(pixels))

    def test_round_trip_and_canonical_format(self):
        path = self.folder / "reference.json"
        raw = clipref.fixture_bytes(self.fixture)
        path.write_bytes(raw)
        loaded = clipref.read_fixture(path)
        self.assertEqual(raw, clipref.fixture_bytes(loaded))
        self.assertLessEqual(len(raw), 512 * 1024)
        self.assertEqual(list(loaded), ["format", "version", "producer", "pins", "cases_digest", "images", "texts", "batches"])
        self.assertEqual(base64_bytes(clipref.encode_floats([1.0])), b"\x00\x00\x80\x3f")
        report = clipref.compare(loaded, self.adapter, provenance=False)
        self.assertEqual(report["status"], "pass", report)
        self.assertTrue(all("maximum" in row for row in report["checks"]))
        self.assertTrue(any(row["name"] == "batch:text:tokens" for row in report["checks"]))
        self.assertTrue(any(row["name"] == "batch:image:reference_vs_single:relative_l2" and row["status"] == "info" for row in report["checks"]))

    def test_builder_encodes_every_case_alone_and_each_batch_once(self):
        adapter = mock.Mock(wraps=self.adapter)
        adapter.pins = self.adapter.pins
        clipref.build_fixture(adapter, self.producer)
        self.assertEqual([c.args[0].shape[0] for c in adapter.encode_images.call_args_list], [1] * 18 + [6])
        self.assertEqual([c.args[0].shape[0] for c in adapter.encode_texts.call_args_list], [1] * 18 + [6])

    def test_one_token_id_fails_exact_equality(self):
        fixture = copy.deepcopy(self.fixture)
        fixture["texts"][1]["tokens"][1] += 1
        check = self.assert_named_failure(fixture, "text:ascii:tokens")
        self.assertEqual(check["maximum"], 1)

    def test_preprocessed_sample_threshold(self):
        for delta, passes in ((9.9e-5, True), (1.01e-4, False), (2e-4, False)):
            with self.subTest(delta=delta):
                fixture = copy.deepcopy(self.fixture)
                row = fixture["images"][0]["pre"]
                values = clipref.decode_floats(row["samples"]).copy()
                values[0] += np.float32(delta)
                row["samples"] = clipref.encode_floats(values)
                if passes:
                    self.assertEqual(clipref.compare(fixture, self.adapter, provenance=False)["status"], "pass")
                else:
                    self.assertGreater(self.assert_named_failure(fixture, "image:rgb-224-gradient:pre:samples")["maximum"], 1e-4)

    def test_single_feature_cosine_threshold(self):
        for kind, name in (("images", "image:rgb-224-gradient"), ("texts", "text:empty")):
            for cosine, passes in ((0.99995, True), (0.999899, False), (0.9998, False)):
                with self.subTest(kind=kind, cosine=cosine):
                    fixture = copy.deepcopy(self.fixture)
                    row = fixture[kind][0]
                    row["features"] = clipref.encode_floats(rotate_row(clipref.decode_floats(row["features"]), cosine))
                    if passes:
                        self.assertEqual(clipref.compare(fixture, self.adapter, provenance=False)["status"], "pass")
                    else:
                        self.assert_named_failure(fixture, name + ":features:cosine")

    def test_batch_feature_cosine_threshold(self):
        for index, kind in ((0, "image"), (1, "text")):
            for cosine, passes in ((0.99995, True), (0.999899, False), (0.9998, False)):
                with self.subTest(kind=kind, cosine=cosine):
                    fixture = copy.deepcopy(self.fixture)
                    row = fixture["batches"][index]
                    values = clipref.decode_floats(row["features"]).reshape(6, -1).copy()
                    values[1] = rotate_row(values[1], cosine)
                    row["features"] = clipref.encode_floats(values)
                    if passes:
                        self.assertEqual(clipref.compare(fixture, self.adapter, provenance=False)["status"], "pass")
                    else:
                        self.assert_named_failure(fixture, f"batch:{kind}:features:cosine")

    def test_information_never_sets_pass_criteria(self):
        fixture = copy.deepcopy(self.fixture)
        pre = fixture["images"][0]["pre"]
        pre["mean"], pre["std"], pre["tensor_sha256"] = clipref.encode_floats([99] * 3), clipref.encode_floats([99] * 3), "0" * 64
        fixture["images"][0]["features"] = clipref.encode_floats(clipref.decode_floats(fixture["images"][0]["features"]) * 2)
        report = clipref.compare(fixture, self.adapter, provenance=False)
        self.assertEqual(report["status"], "pass", report)
        self.assertTrue(any(row["status"] == "info" and row["maximum"] > 0.1 for row in report["checks"]))

    def test_input_and_cases_digest_mismatches_are_setup(self):
        for kind in ("images", "texts"):
            fixture = copy.deepcopy(self.fixture)
            fixture[kind][0]["input_sha256"] = "0" * 64
            self.assert_named_failure(fixture, f"setup:{kind}:{fixture[kind][0]['id']}:input_sha256", "setup problem")
        fixture = copy.deepcopy(self.fixture)
        fixture["cases_digest"] = "0" * 64
        self.assert_named_failure(fixture, "setup:cases_digest", "setup problem")

    def test_model_pins_mismatches_are_setup_before_inference(self):
        for key in ("weights_sha256", "weights_size", "merges_sha256"):
            fixture = copy.deepcopy(self.fixture)
            fixture["pins"][key] = fixture["pins"][key] + 1 if key == "weights_size" else "0" * 64
            with mock.patch.object(self.adapter, "encode_images", side_effect=AssertionError("inference must not start")):
                self.assert_named_failure(fixture, "setup:pins:" + key, "setup problem")

    def test_pillow_and_numpy_version_mismatches_are_setup(self):
        for key in ("pillow", "numpy"):
            fixture = copy.deepcopy(self.fixture)
            fixture["producer"][key] = "different"
            self.assert_named_failure(fixture, "setup:version:" + key, "setup problem")

    def test_reader_refuses_oversize_unknown_missing_and_duplicate_keys(self):
        path = self.folder / "invalid.json"
        raw = clipref.fixture_bytes(self.fixture)
        path.write_bytes(raw + b" " * (clipref.MAX_FIXTURE_BYTES + 1 - len(raw)))
        with self.assertRaisesRegex(clipref.FixtureError, "512 KiB"):
            clipref.read_fixture(path)
        for mutate in (lambda f: f.update(unknown=1), lambda f: f.pop("pins"), lambda f: f["images"][0]["pre"].update(unknown=1)):
            fixture = copy.deepcopy(self.fixture)
            mutate(fixture)
            path.write_text(json.dumps(fixture), encoding="utf-8")
            with self.assertRaisesRegex(clipref.FixtureError, "keys"):
                clipref.read_fixture(path)
        path.write_bytes(b'{"format":"duplicate",' + raw[1:])
        with self.assertRaises(clipref.FixtureError):
            clipref.read_fixture(path)

    def test_reader_refuses_other_format_version_lengths_and_nonfinite_values(self):
        mutations = (
            lambda f: f.update(format="other"), lambda f: f.update(version=2),
            lambda f: f["texts"][0]["tokens"].pop(), lambda f: f["images"].pop(),
            lambda f: f["batches"][0]["ids"].pop(),
            lambda f: f["images"][0]["pre"].update(samples=clipref.encode_floats([0])),
            lambda f: f["images"][0].update(features=clipref.encode_floats([0, 1])),
            lambda f: f["batches"][0].update(features=clipref.encode_floats([0, 1])),
            lambda f: f["images"][0]["pre"].update(mean=clipref.encode_floats([0, float("nan"), 0])),
            lambda f: f["texts"][0].update(features=clipref.encode_floats([float("inf")] * 32)),
        )
        path = self.folder / "invalid-values.json"
        for mutate in mutations:
            fixture = copy.deepcopy(self.fixture)
            mutate(fixture)
            path.write_text(json.dumps(fixture), encoding="utf-8")
            with self.subTest(mutate=mutate), self.assertRaises(clipref.FixtureError):
                clipref.read_fixture(path)

    def upstream_producer(self, fixture):
        fixture["producer"].update(platform="Linux x86_64", torch="2.9.1+cpu", torchvision="0.24.1", open_clip="3.3.0")
        fixture["producer"]["transform"].update(resize_size=[224], crop_size=[224, 224])

    def test_provenance_requires_openclip_on_linux_with_the_clip_transform(self):
        report = clipref.compare(self.fixture, self.adapter)
        self.assertEqual(report["status"], "setup problem")
        failed = {row["name"] for row in report["checks"] if row["status"] == "setup problem"}
        self.assertEqual(failed, {"setup:provenance:" + name for name in ("open_clip", "platform", "torch", "resize_size", "crop_size")})
        fixture = copy.deepcopy(self.fixture)
        self.upstream_producer(fixture)
        self.assertEqual(clipref.compare(fixture, self.adapter)["status"], "pass")
        for change in (lambda p: p.update(open_clip="3.2.0"), lambda p: p.update(platform="Windows AMD64"),
                       lambda p: p["transform"].update(interpolation="bilinear"),
                       lambda p: p["transform"].update(mean=clipref.encode_floats([0.5, 0.5, 0.5]))):
            changed = copy.deepcopy(fixture)
            change(changed["producer"])
            with self.subTest(change=change):
                self.assertEqual(clipref.compare(changed, self.adapter)["status"], "setup problem")
        with mock.patch.object(clipref, "CASES_DIGEST", "0" * 64):
            report = clipref.compare(fixture, self.adapter)
        self.assertEqual(report["status"], "setup problem")
        self.assertEqual([row["name"] for row in report["checks"] if row["status"] == "setup problem"], ["setup:cases_digest:committed"])

    def test_unexpected_cli_errors_and_deep_json_are_setup_problems(self):
        path = self.folder / "deep.json"
        path.write_bytes(b"[" * 100_000)
        with self.assertRaises(clipref.FixtureError):
            clipref.read_fixture(path)
        path.write_bytes(clipref.fixture_bytes(self.fixture))
        with mock.patch.object(clipref, "load_ours", side_effect=KeyError("x")), mock.patch("sys.stdout", new_callable=io.StringIO) as output:
            self.assertEqual(clipref.main(["check", "--fixture", str(path)]), 2)
            self.assertIn("unexpected KeyError", output.getvalue())

    def test_cli_exit_codes_and_missing_fixture(self):
        path = self.folder / "cli.json"
        path.write_bytes(clipref.fixture_bytes(self.fixture))
        with mock.patch.object(clipref, "load_ours", return_value=self.adapter), mock.patch("sys.stdout", new_callable=io.StringIO):
            # The synthetic producer is not OpenCLIP on Linux.
            self.assertEqual(clipref.main(["check", "--fixture", str(path)]), 2)
        with mock.patch.object(clipref, "load_ours", return_value=self.adapter), mock.patch.object(clipref, "provenance_problems", return_value=[]), \
                mock.patch("sys.stdout", new_callable=io.StringIO) as output:
            self.assertEqual(clipref.main(["check", "--fixture", str(path)]), 0)
            self.assertIn("summary: pass", output.getvalue())
            fixture = copy.deepcopy(self.fixture)
            fixture["texts"][1]["tokens"][1] += 1
            path.write_bytes(clipref.fixture_bytes(fixture))
            self.assertEqual(clipref.main(["check", "--fixture", str(path)]), 1)
            fixture["producer"]["pillow"] = "different"
            path.write_bytes(clipref.fixture_bytes(fixture))
            self.assertEqual(clipref.main(["check", "--fixture", str(path)]), 2)
        with mock.patch.object(clipref, "load_ours") as loader, mock.patch("sys.stdout", new_callable=io.StringIO) as output:
            self.assertEqual(clipref.main(["check", "--fixture", str(self.folder / "absent.json")]), 2)
            self.assertIn("setup problem", output.getvalue())
            loader.assert_not_called()

    def test_loader_verifies_both_files_before_reading_either(self):
        weights, merges = self.folder / "tiny.safetensors", self.folder / "merges.txt"
        pins = {"clip-vit-b-16": {"size": weights.stat().st_size, "sha256": self.adapter.pins["weights_sha256"]},
                "clip-vit-b-16-merges": {"size": merges.stat().st_size, "sha256": self.adapter.pins["merges_sha256"]}}
        with mock.patch.object(embed, "MODEL_FILE", weights), mock.patch.object(embed, "MERGES_FILE", merges), mock.patch.object(embed, "model_pins", return_value=pins):
            with mock.patch.object(clip, "load", return_value=self.adapter.model), mock.patch.object(clip, "Tokenizer", return_value=self.adapter.tokenizer):
                self.assertEqual(clipref.load_ours().pins, self.adapter.pins)
            pins["clip-vit-b-16-merges"]["sha256"] = "0" * 64
            with mock.patch.object(clip, "load") as load, mock.patch.object(clip, "Tokenizer") as tokenizer:
                with self.assertRaises(common.Refused):
                    clipref.load_ours()
                load.assert_not_called()
                tokenizer.assert_not_called()


def base64_bytes(value):
    import base64
    return base64.b64decode(value)


@unittest.skipUnless(HAVE_ENV, "needs the Taste Lab environment")
class CloudScriptTests(unittest.TestCase):
    def setUp(self):
        self.folder = Path(tempfile.gettempdir()) / f"tastelab-test-{uuid4().hex}"
        self.folder.mkdir()
        self.addCleanup(shutil.rmtree, self.folder, ignore_errors=True)
        self.requirements = openclip_reference.REQUIREMENTS.read_text(encoding="utf-8")
        self.versions = {"pillow": "12.3.0", "numpy": "2.5.3", "scipy": "1.18.1", "ftfy": "6.3.1", "regex": "2026.9.10", "open_clip_torch": "3.3.0"}

    def test_pure_environment_check_accepts_only_matching_pins_without_torch(self):
        self.assertNotIn("torch", sys.modules)
        self.assertEqual(openclip_reference.check_environment(self.versions, self.requirements), self.versions)
        for key in self.versions:
            with self.subTest(key=key), self.assertRaisesRegex(common.Refused, key):
                openclip_reference.check_environment({**self.versions, key: "wrong"}, self.requirements)
        with self.assertRaises(common.Refused):
            openclip_reference.check_environment({}, self.requirements)
        self.assertNotIn("torch", sys.modules)

    def test_cloud_requirement_pins_equal_shared_pins(self):
        reference = (ROOT / "tools/python/tastelab-reference.txt").read_text(encoding="utf-8")
        expected = openclip_reference.check_environment(self.versions, self.requirements)
        for name, version in expected.items():
            self.assertIn(f"{name}=={version}\n", reference)

    def test_environment_refusal_precedes_platform_download_and_torch(self):
        with mock.patch.object(openclip_reference.importlib.metadata, "version", return_value="wrong"), mock.patch.object(openclip_reference, "download_verified") as download:
            with mock.patch("sys.stderr", new_callable=io.StringIO) as output:
                self.assertEqual(openclip_reference.main(["--out", str(self.folder / "unused.json")]), 2)
                self.assertIn("environment:", output.getvalue())
            download.assert_not_called()
        self.assertNotIn("torch", sys.modules)
        with mock.patch.object(openclip_reference.importlib.metadata, "version", side_effect=lambda name: self.versions[name]), mock.patch.object(openclip_reference.platform, "system", return_value="Windows"):
            with mock.patch.object(openclip_reference, "download_verified") as download, mock.patch("sys.stderr", new_callable=io.StringIO):
                self.assertEqual(openclip_reference.main(["--out", str(self.folder / "unused.json")]), 2)
                download.assert_not_called()

    def test_changed_cases_refuse_before_download(self):
        with mock.patch.object(openclip_reference.importlib.metadata, "version", side_effect=lambda name: self.versions[name]), \
                mock.patch.object(openclip_reference.platform, "system", return_value="Linux"), mock.patch.object(clipref, "CASES_DIGEST", "0" * 64), \
                mock.patch.object(openclip_reference, "download_verified") as download, mock.patch("sys.stderr", new_callable=io.StringIO) as output:
            self.assertEqual(openclip_reference.main(["--out", str(self.folder / "unused.json")]), 2)
            self.assertIn("committed digest", output.getvalue())
            download.assert_not_called()

    def test_failures_are_described_without_paths_or_urls(self):
        import urllib.error
        signed = "https://cdn.example/signed?token=abc"
        cases = ((urllib.error.HTTPError(signed, 403, "Forbidden", {}, None), "HTTPError 403"),
                 (urllib.error.URLError(ConnectionRefusedError(111, "Connection refused")), "URLError ConnectionRefusedError"),
                 (FileNotFoundError(2, "No such file or directory", "/private/path"), "FileNotFoundError errno=2"),
                 (urllib.error.URLError(FileNotFoundError(2, "No such file", "/private/path/model")), "URLError FileNotFoundError: errno=2"),
                 (urllib.error.URLError("tunnel to " + signed + " failed"), "URLError str: tunnel to <redacted> failed"),
                 (RuntimeError("cannot open /private/path/model"), "RuntimeError: cannot open <redacted>"),
                 (ImportError("cannot load C:\\private\\path\\x.dll"), "ImportError: cannot load <redacted>"),
                 (clipref.FixtureError("wrong case count"), "FixtureError: wrong case count"),
                 (common.Refused("model download: destination must be inside tools/.models/"), "inside tools/.models/"))
        for exc, expected in cases:
            with self.subTest(expected=expected):
                text = openclip_reference.describe(exc)
                if hasattr(exc, "close"):
                    exc.close()
                self.assertIn(expected, text)
                self.assertNotIn("private", text)
                self.assertNotIn("token=", text)
        with mock.patch.object(openclip_reference.importlib.metadata, "version", side_effect=lambda name: self.versions[name]), \
                mock.patch.object(openclip_reference.platform, "system", return_value="Linux"), \
                mock.patch.object(openclip_reference, "download_verified", side_effect=RuntimeError("/private/path " + signed)), \
                mock.patch("sys.stderr", new_callable=io.StringIO) as output:
            self.assertEqual(openclip_reference.main(["--out", str(self.folder / "unused.json")]), 2)
        self.assertIn("RuntimeError", output.getvalue())
        self.assertNotIn("private", output.getvalue())
        self.assertNotIn("token=", output.getvalue())

    def test_python_before_3_12_is_refused_before_any_lookup(self):
        with mock.patch.object(openclip_reference.sys, "version_info", (3, 11, 9)), \
                mock.patch.object(openclip_reference.importlib.metadata, "version") as version, \
                mock.patch("sys.stderr", new_callable=io.StringIO) as output:
            self.assertEqual(openclip_reference.main(["--out", str(self.folder / "unused.json")]), 2)
        self.assertIn("3.12", output.getvalue())
        version.assert_not_called()

    def test_default_opener_uses_environment_proxies_and_the_checked_redirect_only(self):
        import urllib.error
        pin, _ = self.fake_download(b"weights")
        handlers = []

        def build(*args):
            handlers.extend(args)
            opener = mock.Mock()
            opener.open.side_effect = urllib.error.URLError("offline")
            return opener

        with mock.patch.object(openclip_reference.urllib.request, "build_opener", side_effect=build):
            with self.assertRaises(urllib.error.URLError):
                openclip_reference.download_verified(pin, self.folder / "weights")
        self.assertEqual([type(handler) for handler in handlers], [openclip_reference.AllowedRedirect])
        self.assertEqual(list(self.folder.iterdir()), [])

    def fake_download(self, data, pin_changes=None, response_url="https://huggingface.co/model"):
        response = io.BytesIO(data)
        response.geturl = lambda: response_url
        opener = mock.Mock()
        opener.open.return_value = response
        pin = {"url": "https://huggingface.co/model", "network_hosts": ["huggingface.co"], "size": len(data), "sha256": hashlib.sha256(data).hexdigest()}
        pin.update(pin_changes or {})
        return pin, opener

    def test_wrong_size_or_hash_download_is_refused_and_removed(self):
        for changes, message in (({"size": 6}, "size"), ({"size": 4}, "size"), ({"sha256": "0" * 64}, "SHA-256")):
            pin, opener = self.fake_download(b"model", changes)
            with self.subTest(changes=changes), self.assertRaisesRegex(common.Refused, message):
                openclip_reference.download_verified(pin, self.folder / "weights", opener=opener)
            self.assertFalse((self.folder / "weights").exists())
            self.assertEqual(list(self.folder.glob("*.part-*")), [])
        self.assertNotIn("torch", sys.modules)

    def test_verified_download_and_existing_file_need_no_network(self):
        pin, opener = self.fake_download(b"model")
        dest = openclip_reference.download_verified(pin, self.folder / "weights", opener=opener)
        self.assertEqual(dest.read_bytes(), b"model")
        opener.open.assert_called_once()
        with mock.patch.object(openclip_reference.urllib.request, "build_opener", side_effect=AssertionError("network must not start")):
            self.assertEqual(openclip_reference.download_verified(pin, dest), dest)
            dest.write_bytes(b"wrong")
            with self.assertRaisesRegex(common.Refused, "SHA-256"):
                openclip_reference.download_verified(pin, dest)

    def test_redirects_and_response_hosts_are_bounded(self):
        handler = openclip_reference.AllowedRedirect(["huggingface.co"])
        request = openclip_reference.urllib.request.Request("https://huggingface.co/model")
        redirected = handler.redirect_request(request, None, 302, "", {}, "https://huggingface.co/allowed")
        self.assertEqual(redirected.full_url, "https://huggingface.co/allowed")
        for url in ("https://evil.example/model", "https://huggingface.co.evil.example/", "http://huggingface.co/", "https://user@huggingface.co/", "https://huggingface.co:444/"):
            with self.subTest(url=url), self.assertRaises(common.Refused):
                handler.redirect_request(request, None, 302, "", {}, url)
        pin, opener = self.fake_download(b"model", response_url="https://evil.example/model")
        with self.assertRaises(common.Refused):
            openclip_reference.download_verified(pin, self.folder / "weights", opener=opener)
        self.assertEqual(list(self.folder.iterdir()), [])


@unittest.skipUnless(HAVE_ENV, "needs the Taste Lab environment")
class RealReferenceTests(unittest.TestCase):
    def test_real_upstream_equivalence_when_reference_and_verified_model_exist(self):
        if not clipref.FIXTURE.is_file():
            self.skipTest("C-02: OpenCLIP reference fixture is not yet produced")
        if not (embed.MODEL_FILE.is_file() and embed.MERGES_FILE.is_file()):
            self.skipTest("C-02: pinned model files are absent")
        ours = clipref.load_ours()      # a present but unverified file fails here
        report = clipref.compare(clipref.read_fixture(clipref.FIXTURE), ours)
        self.assertEqual(report["status"], "pass", report)

    def test_present_but_unverified_model_fails_instead_of_skipping(self):
        folder = Path(tempfile.gettempdir()) / f"tastelab-test-{uuid4().hex}"
        folder.mkdir()
        self.addCleanup(shutil.rmtree, folder, ignore_errors=True)
        for name in ("fixture.json", "model.safetensors", "merges.txt"):
            (folder / name).write_bytes(b"not the pinned file")
        with mock.patch.object(clipref, "FIXTURE", folder / "fixture.json"), mock.patch.object(embed, "MODEL_FILE", folder / "model.safetensors"), \
                mock.patch.object(embed, "MERGES_FILE", folder / "merges.txt"):
            with self.assertRaises(common.Refused):
                try:
                    self.test_real_upstream_equivalence_when_reference_and_verified_model_exist()
                except unittest.SkipTest:
                    self.fail("a present but unverified model file must fail, not skip")


if __name__ == "__main__":
    unittest.main()
