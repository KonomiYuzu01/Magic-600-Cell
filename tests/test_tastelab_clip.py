"""Taste Lab's NumPy CLIP (tools/tastelab/clip.py) and the embedder (tastelab environment).

A tiny random checkpoint, written to a temporary safetensors file, is checked
against a plain loop-by-loop reference of the same model. Model checksum and tokenizer checks use synthetic files only. Nothing is downloaded. Without the tastelab environment every test is
skipped.
"""
from __future__ import annotations

import io
import json
import math
import struct
import sys
import tempfile
import shutil
import hashlib
from uuid import uuid4
from unittest import mock
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools"))

try:
    import numpy as np
    from PIL import Image, ImageDraw, ImageFont
    from tastelab import clip, embed, common
    HAVE_ENV = True
except ImportError:
    HAVE_ENV = False


def write_safetensors(path: Path, tensors: dict, dtype: str = "F32", shift: int = 0) -> None:
    header, blobs, offset = {}, [], 0
    for name, array in tensors.items():
        data = np.asarray(array, dtype="<f4").tobytes()
        header[name] = {"dtype": dtype, "shape": list(np.shape(array)), "data_offsets": [offset + shift, offset + len(data) + shift]}
        blobs.append(data)
        offset += len(data)
    raw = json.dumps(header).encode("utf-8")
    path.write_bytes(struct.pack("<Q", len(raw)) + raw + b"".join(blobs))


def tiny_checkpoint(seed: int = 7) -> dict:
    """An OpenCLIP-shaped ViT: image 8 px in 4 px patches, width 128 (two heads), one block per tower."""
    rng = np.random.default_rng(seed)

    def r(*shape, scale=0.2):
        return (rng.standard_normal(shape) * scale).astype(np.float32)

    def block(prefix: str, width: int) -> dict:
        return {f"{prefix}ln_1.weight": 1 + r(width), f"{prefix}ln_1.bias": r(width),
                f"{prefix}attn.in_proj_weight": r(3 * width, width, scale=0.1), f"{prefix}attn.in_proj_bias": r(3 * width),
                f"{prefix}attn.out_proj.weight": r(width, width, scale=0.1), f"{prefix}attn.out_proj.bias": r(width),
                f"{prefix}ln_2.weight": 1 + r(width), f"{prefix}ln_2.bias": r(width),
                f"{prefix}mlp.c_fc.weight": r(4 * width, width, scale=0.1), f"{prefix}mlp.c_fc.bias": r(4 * width),
                f"{prefix}mlp.c_proj.weight": r(width, 4 * width, scale=0.1), f"{prefix}mlp.c_proj.bias": r(width)}

    t = {"logit_scale": np.float32(math.log(100.0)), "visual.conv1.weight": r(128, 3, 4, 4),
         "visual.class_embedding": r(128), "visual.positional_embedding": r(5, 128),
         "visual.ln_pre.weight": 1 + r(128), "visual.ln_pre.bias": r(128),
         "visual.ln_post.weight": 1 + r(128), "visual.ln_post.bias": r(128), "visual.proj": r(128, 32),
         "token_embedding.weight": r(50, 128), "positional_embedding": r(77, 128),
         "ln_final.weight": 1 + r(128), "ln_final.bias": r(128), "text_projection": r(128, 32)}
    t.update(block("visual.transformer.resblocks.0.", 128))
    t.update(block("transformer.resblocks.0.", 128))
    return t


def ref_layer_norm(v, w, b):
    mu = sum(v) / len(v)
    var = sum((a - mu) ** 2 for a in v) / len(v)
    return [(a - mu) / math.sqrt(var + 1e-5) * w[i] + b[i] for i, a in enumerate(v)]


def ref_linear(v, weight, bias):
    """PyTorch layout: weight is (out, in)."""
    return [sum(weight[o][i] * v[i] for i in range(len(v))) + bias[o] for o in range(len(weight))]


def ref_block(xs, t, prefix, causal):
    """One pre-norm residual block over a list of token vectors, one head and one position at a time."""
    g = lambda name: t[prefix + name].tolist()  # noqa: E731
    width = len(xs[0])
    heads = width // 64
    normed = [ref_layer_norm(x, g("ln_1.weight"), g("ln_1.bias")) for x in xs]
    qkv = [ref_linear(x, g("attn.in_proj_weight"), g("attn.in_proj_bias")) for x in normed]
    out = []
    for i in range(len(xs)):
        concat = []
        for h in range(heads):
            sl = slice(h * 64, (h + 1) * 64)
            q = qkv[i][0:width][sl]
            visible = range(i + 1) if causal else range(len(xs))
            scores = [sum(a * b for a, b in zip(q, qkv[j][width:2 * width][sl])) / 8.0 for j in visible]
            top = max(scores)
            weights = [math.exp(s - top) for s in scores]
            total = sum(weights)
            mixed = [0.0] * 64
            for wgt, j in zip(weights, visible):
                for d, value in enumerate(qkv[j][2 * width:][sl]):
                    mixed[d] += wgt / total * value
            concat.extend(mixed)
        out.append(ref_linear(concat, g("attn.out_proj.weight"), g("attn.out_proj.bias")))
    xs = [[a + b for a, b in zip(x, o)] for x, o in zip(xs, out)]
    result = []
    for x in xs:
        hidden = ref_linear(ref_layer_norm(x, g("ln_2.weight"), g("ln_2.bias")), g("mlp.c_fc.weight"), g("mlp.c_fc.bias"))
        hidden = [0.5 * a * (1 + math.erf(a / math.sqrt(2))) for a in hidden]
        result.append([a + b for a, b in zip(x, ref_linear(hidden, g("mlp.c_proj.weight"), g("mlp.c_proj.bias")))])
    return result


def ref_image(t, pixels):
    """pixels: (3, 8, 8). Patch embedding as an explicit convolution."""
    conv = t["visual.conv1.weight"]
    xs = [t["visual.class_embedding"].tolist()]
    for gy in range(2):
        for gx in range(2):
            xs.append([float(sum(conv[o, c, ky, kx] * pixels[c, gy * 4 + ky, gx * 4 + kx]
                                 for c in range(3) for ky in range(4) for kx in range(4))) for o in range(128)])
    pos = t["visual.positional_embedding"].tolist()
    xs = [[a + b for a, b in zip(x, p)] for x, p in zip(xs, pos)]
    xs = [ref_layer_norm(x, t["visual.ln_pre.weight"].tolist(), t["visual.ln_pre.bias"].tolist()) for x in xs]
    xs = ref_block(xs, t, "visual.transformer.resblocks.0.", causal=False)
    cls = ref_layer_norm(xs[0], t["visual.ln_post.weight"].tolist(), t["visual.ln_post.bias"].tolist())
    return [sum(cls[i] * t["visual.proj"][i, o] for i in range(128)) for o in range(32)]


def ref_text(t, ids):
    """ids: the full 77 token ids. The end token is the largest id; all 77 positions are computed."""
    end = int(np.argmax(ids))
    xs = [[a + b for a, b in zip(t["token_embedding.weight"][i].tolist(), t["positional_embedding"][p].tolist())]
          for p, i in enumerate(ids)]
    xs = ref_block(xs, t, "transformer.resblocks.0.", causal=True)
    final = ref_layer_norm(xs[end], t["ln_final.weight"].tolist(), t["ln_final.bias"].tolist())
    return [sum(final[i] * t["text_projection"][i, o] for i in range(128)) for o in range(32)]


@unittest.skipUnless(HAVE_ENV, "needs the tastelab environment (numpy, scipy, Pillow, regex, ftfy)")
class TinyModelTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.gettempdir()) / f"tastelab-{uuid4().hex}"
        self.tmp.mkdir()
        self.tensors = tiny_checkpoint()
        self.path = self.tmp / "tiny.safetensors"
        write_safetensors(self.path, self.tensors)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_safetensors_round_trip(self):
        loaded = clip.read_safetensors(self.path)
        self.assertEqual(set(loaded), set(self.tensors))
        for name, array in self.tensors.items():
            np.testing.assert_array_equal(loaded[name], array)

    def test_image_tower_matches_the_reference(self):
        model = clip.load(self.path)
        self.assertEqual((model.image_size, model.dim), (8, 32))
        self.assertAlmostEqual(model.logit_scale, 100.0, places=3)
        pixels = np.random.default_rng(1).standard_normal((2, 3, 8, 8)).astype(np.float32)
        got = model.encode_images(pixels)
        self.assertEqual(got.shape, (2, 32))
        for row, image in zip(got, pixels):
            np.testing.assert_allclose(row, ref_image(self.tensors, image), rtol=2e-4, atol=2e-4)

    def test_text_tower_matches_the_reference_and_ignores_padding(self):
        model = clip.load(self.path)
        tokens = np.zeros((2, 77), dtype=np.int64)
        tokens[0, :5] = [47, 3, 9, 12, 49]           # 49 is the largest id: the end token
        tokens[1, :3] = [47, 20, 49]
        got = model.encode_tokens(tokens)
        for row, ids in zip(got, tokens):
            np.testing.assert_allclose(row, ref_text(self.tensors, ids.tolist()), rtol=2e-4, atol=2e-4)
        np.testing.assert_allclose(model.encode_tokens(tokens[1:]), got[1:], rtol=1e-5, atol=1e-6)

    def test_refuses_files_that_are_not_plain_float32_checkpoints(self):
        cases = {"half precision": dict(dtype="F16"), "offset outside the file": dict(shift=10 ** 6)}
        for name, options in cases.items():
            bad = self.tmp / f"{name}.safetensors"
            write_safetensors(bad, self.tensors, **options)
            with self.subTest(case=name), self.assertRaises(ValueError):
                clip.read_safetensors(bad)
        huge = self.tmp / "header.safetensors"
        huge.write_bytes(struct.pack("<Q", 2 ** 40) + b"{}")
        with self.assertRaises(ValueError):
            clip.read_safetensors(huge)
        missing = dict(self.tensors)
        del missing["visual.proj"]
        partial = self.tmp / "partial.safetensors"
        write_safetensors(partial, missing)
        with self.assertRaises(ValueError):
            clip.load(partial)
        extra = dict(self.tensors, **{"visual.extra": np.zeros(3, dtype=np.float32)})
        write_safetensors(partial, extra)
        with self.assertRaises(ValueError):
            clip.load(partial)

    def test_preprocess_resizes_the_short_side_and_crops_the_centre(self):
        image = Image.new("RGB", (448, 224), (255, 0, 0))
        ImageDraw.Draw(image).rectangle((0, 0, 111, 223), fill=(0, 0, 255))  # left quarter: cut away by the crop
        x = clip.preprocess(image)
        self.assertEqual((x.shape, x.dtype), ((3, 224, 224), np.float32))
        red = (np.array([1.0, 0.0, 0.0], dtype=np.float32) - clip.MEAN) / clip.STD
        np.testing.assert_allclose(x[:, 112, 112], red, rtol=1e-5)
        self.assertTrue(np.allclose(x[:, :, 5:].mean(axis=(1, 2)), red, atol=1e-3))
        for size in ((10, 640), (640, 10), (1, 1), (224, 224)):
            with self.subTest(size=size):
                self.assertEqual(clip.preprocess(Image.new("L", size, 128)).shape, (3, 224, 224))
        rgba = Image.new("RGBA", (300, 300), (0, 255, 0, 0))
        self.assertEqual(clip.preprocess(rgba).shape, (3, 224, 224))

    def test_preprocess_equals_full_resize_then_crop(self):
        rng = np.random.default_rng(7)
        maximum, differing, total = 0, 0, 0
        for w, h in ((65, 321), (321, 65), (50, 1000), (223, 225), (224, 224), (448, 7), (64, 4096), (4096, 64),
                     (65, 4160), (97, 5003)):
            y, x = np.indices((h, w))
            pattern = np.stack(((17 * x + 31 * y) % 256, ((x // 3 + y // 5) % 2) * 255,
                                (x * y) % 256), axis=-1).astype(np.uint8)
            for name, pixels in (("random", rng.integers(0, 256, (h, w, 3), dtype=np.uint8)), ("pattern", pattern)):
                with self.subTest(size=(w, h), image=name), Image.fromarray(pixels) as image:
                    size = (224, int(224 * h / w)) if w <= h else (int(224 * w / h), 224)
                    left, top = (int(round((edge - 224) / 2.0)) for edge in size)
                    with image.resize(size, Image.Resampling.BICUBIC) as resized:
                        with resized.crop((left, top, left + 224, top + 224)) as crop:
                            expected = np.asarray(crop, dtype=np.int16)
                    normalized = clip.preprocess(image)
                    actual = np.rint((normalized.transpose(1, 2, 0) * clip.STD + clip.MEAN) * 255).astype(np.int16)
                    difference = np.abs(actual - expected)
                    observed = int(difference.max())
                    changed = int(np.count_nonzero(difference))
                    maximum, differing, total = max(maximum, observed), differing + changed, total + difference.size
                    print(f"preprocess {w}x{h} {name}: max={observed} uint8, differing={changed}/{difference.size}")
                    self.assertEqual(observed, 0)
        print(f"preprocess total: max={maximum} uint8, differing={differing}/{total} ({differing / total:.8%})")

    def test_extreme_aspect_ratios_are_refused_and_the_full_resize_stays_bounded(self):
        from tastelab import images
        for size in ((64, 20001), (20001, 64), (65, 10003), (10, 641)):
            with self.subTest(size=size), Image.new("RGB", size) as image:
                with self.assertRaisesRegex(ValueError, "aspect"):
                    clip.preprocess(image)
                encoded = io.BytesIO()
                image.save(encoded, format="PNG")
                with self.assertRaisesRegex(images.BadImage, "aspect"):
                    images.decode(encoded.getvalue())
        resize = Image.Image.resize
        def bounded(image, size, *args, **kwargs):
            self.assertLessEqual(size[0] * size[1], 224 * 224 * images.MAX_ASPECT)
            return resize(image, size, *args, **kwargs)
        with mock.patch.object(Image.Image, "resize", bounded):
            for size in ((64 * 20, 20), (20, 64 * 20), (7, 7 * 64)):
                with self.subTest(size=size), Image.new("RGB", size) as image:
                    self.assertEqual(clip.preprocess(image).shape, (3, 224, 224))
        for size in ((64, 4096), (4096, 64), (65, 4159)):    # the re-embed path reads thumbnails of decoded images
            with self.subTest(thumbnail=size), Image.new("RGB", size) as image:
                with Image.open(io.BytesIO(images.thumbnail_jpeg(image))) as thumbnail:
                    self.assertEqual(clip.preprocess(thumbnail).shape, (3, 224, 224))

    def test_byte_map_is_a_bijection_onto_printable_characters(self):
        table = clip.bytes_to_unicode()
        self.assertEqual(len(table), 256)
        self.assertEqual(len(set(table.values())), 256)
        self.assertTrue(all(ch.isprintable() and not ch.isspace() for ch in table.values()))




@unittest.skipUnless(HAVE_ENV, "needs the Taste Lab environment")
class PinTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.gettempdir()) / f"tastelab-{uuid4().hex}"
        self.tmp.mkdir()
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.path = self.tmp / "tiny.safetensors"
        write_safetensors(self.path, tiny_checkpoint())
        self.merges = self.tmp / "merges.txt"
        self.merges.write_bytes(b"# synthetic merges\na b\n")
        self.lock = self.tmp / "lock.json"
        self.pins = {"tools": [{"id": name, "size": path.stat().st_size,
                                "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                               for name, path in (("clip-vit-b-16", self.path), ("clip-vit-b-16-merges", self.merges))]}
        self.lock.write_text(json.dumps(self.pins), encoding="utf-8")
        for name, value in (("MODEL_FILE", self.path), ("MERGES_FILE", self.merges), ("LOCK_FILE", self.lock)):
            patch = mock.patch.object(embed, name, value)
            patch.start()
            self.addCleanup(patch.stop)

    def test_changed_tensor_byte_refused_before_model_load(self):
        raw = bytearray(self.path.read_bytes())
        raw[-1] ^= 1
        self.path.write_bytes(raw)
        with mock.patch.object(clip, "load") as load, mock.patch.object(clip, "Tokenizer") as tokenizer:
            with self.assertRaisesRegex(common.Refused, "SHA-256"):
                embed.load_embedder()
        load.assert_not_called()
        tokenizer.assert_not_called()

    def test_changed_merges_byte_refused_before_either_reader(self):
        raw = bytearray(self.merges.read_bytes())
        raw[-2] ^= 1
        self.merges.write_bytes(raw)
        with mock.patch.object(clip, "load") as load, mock.patch.object(clip, "Tokenizer") as tokenizer:
            with self.assertRaisesRegex(common.Refused, "merges.txt: SHA-256"):
                embed.load_embedder()
        load.assert_not_called()
        tokenizer.assert_not_called()

    def test_sizes_and_missing_lock_entries_refused(self):
        for name, path in (("clip-vit-b-16", self.path), ("clip-vit-b-16-merges", self.merges)):
            with self.subTest(name=name), mock.patch.object(clip, "load") as load:
                previous = path.read_bytes()
                path.write_bytes(previous + b"x")
                with self.assertRaisesRegex(common.Refused, "size"):
                    embed.load_embedder()
                path.write_bytes(previous)
                load.assert_not_called()
                self.lock.write_text(json.dumps({"tools": [r for r in self.pins["tools"] if r["id"] != name]}), encoding="utf-8")
                with self.assertRaisesRegex(common.Refused, name):
                    embed.load_embedder()
                load.assert_not_called()
                self.lock.write_text(json.dumps(self.pins), encoding="utf-8")

    def test_both_verified_before_load_and_tokenizer(self):
        model = mock.Mock(dim=512, logit_scale=100.0)
        with mock.patch.object(clip, "load", return_value=model) as load, mock.patch.object(clip, "Tokenizer") as tokenizer:
            instance = embed.load_embedder(batch=3)
        load.assert_called_once_with(self.path)
        tokenizer.assert_called_once_with(self.merges)
        self.assertEqual(instance.weights_sha256, self.pins["tools"][0]["sha256"])
        self.assertEqual(instance.batch, 3)


@unittest.skipUnless(HAVE_ENV, "needs the Taste Lab environment")
class TokenizerTests(unittest.TestCase):
    def test_synthetic_merges_unicode_padding_and_truncation(self):
        folder = Path(tempfile.gettempdir()) / f"tastelab-{uuid4().hex}"
        folder.mkdir()
        self.addCleanup(shutil.rmtree, folder, ignore_errors=True)
        path = folder / "merges.txt"
        lines = ["h e", "he l", "hel l", "hell o</w>"] + [f"left{i} right{i}" for i in range(clip.MERGES - 4)]
        path.write_text("# synthetic\n" + "\n".join(lines) + "\n", encoding="utf-8")
        tokenizer = clip.Tokenizer(path)
        self.assertEqual(tokenizer.encode("  HELLO  "), [515])
        self.assertEqual(tokenizer.encode("&amp;"), tokenizer.encode("&"))
        values = tokenizer.tokenize(["hello", "é 中文 🧭", "hello " * 100])
        self.assertEqual(values.shape, (3, 77))
        self.assertEqual(values[0, :4].tolist(), [49406, 515, 49407, 0])
        self.assertEqual(values[2, -1], 49407)
        np.testing.assert_array_equal(values[1], tokenizer.tokenize(["é 中文 🧭"])[0])


if __name__ == "__main__":
    unittest.main()
