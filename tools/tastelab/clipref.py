"""Checksum-bound synthetic OpenCLIP reference, with no torch dependency.

Version 1 stores floating-point arrays as base64 little-endian float32: channel
means/stds have 3 values, samples have 1,200, and features are row-major. Feature
width is inferred from the first single image and must agree everywhere. The
producer's transform mean/std use the same encoding. JSON keys are written in
the order validated below; duplicate, missing and unknown keys are refused.

Samples use flat CHW indices c*224*224 + y*224 + x, for every channel and the
Cartesian grid: coordinates 0 and 223, and floor((2*i+1)*224/40), i=1..18.
This covers each of 20 equal row/column bands and all four corners of each
channel. Only these 1,200 samples per image decide the preprocessing check, a
deliberate narrowing of "each preprocessed value"; channel statistics and full
tensor hashes are informational.

Each synthetic source image is converted with images.to_rgb, exactly as
production converts a decoded image, and both sides preprocess that RGB image;
the L, LA, P, RGBA and CMYK sources cover the conversion. Image input hashes
cover mode + NUL, little-endian uint32 width/height, and tobytes() of that RGB
image (palette images would also cover their palette and transparent entry).
The cases digest hashes ordered [kind, id, input_sha256] JSON rows and must
equal CASES_DIGEST before any download or comparison.

A real fixture must also come from OpenCLIP 3.3.0 on Linux with real torch and
the transform of clip.py (resize 224, bicubic, crop 224, the CLIP mean/std).

Offline CLI (from the repository root, with the Taste Lab interpreter):
  PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools python -m tastelab.clipref check
"""
from __future__ import annotations

import argparse
import base64
import binascii
import hashlib
import json
import struct
import sys
from pathlib import Path

try:
    import numpy as np
    from PIL import Image, __version__ as PILLOW_VERSION

    from tastelab import common, embed, images
except ImportError as exc:
    if __name__ != "__main__":
        raise
    # Exit 1 means a mismatch; a missing package is a setup problem.
    print(f"setup: {type(exc).__name__}: {exc}")
    print("summary: setup problem")
    sys.exit(2)

FORMAT = "tastelab-openclip-reference"
VERSION = 1
MAX_FIXTURE_BYTES = 512 * 1024
PRE_ABSOLUTE = 1e-4
FEATURE_COSINE = 0.9999
FIXTURE = common.ROOT / "tests/fixtures/tastelab/openclip-reference.json"
# cases_digest(cases()) with the pinned Pillow and NumPy; a platform difference is a setup problem.
CASES_DIGEST = "730023a5d2575c95e8eecd6b6575cff2eae887c01da498e8444410edf9d752d3"
UPSTREAM_OPEN_CLIP = "3.3.0"
GRID = (0, *( (2 * i + 1) * 224 // 40 for i in range(1, 19)), 223)
SAMPLE_INDICES = tuple(c * 224 * 224 + y * 224 + x for c in range(3) for y in GRID for x in GRID)

# All source pixels are integer formulas; no RNG, fonts, or decoded files.
IMAGE_SPECS = (
    ("rgb-224-gradient", 224, 224, "RGB", "gradient"),
    ("rgb-odd-wide", 225, 223, "RGB", "gradient"),
    ("rgb-odd-tall", 333, 517, "RGB", "rings"),
    ("rgb-small", 17, 31, "RGB", "hash"),
    ("rgb-max-wide", images.MAX_ASPECT * 64, 64, "RGB", "stripes"),
    ("rgb-max-tall", 64, images.MAX_ASPECT * 64, "RGB", "checker"),
    ("rgb-review-stripes", 1025, 100, "RGB", "stripes"),
    ("rgb-checker-1px", 224, 224, "RGB", "checker"),
    ("rgb-stripes-1px-x", 225, 223, "RGB", "stripe-x"),
    ("rgb-stripes-1px-y", 223, 225, "RGB", "stripe-y"),
    ("rgb-rings", 511, 257, "RGB", "rings"),
    ("rgb-hash", 257, 255, "RGB", "hash"),
    ("l-gradient", 226, 221, "L", "gradient"),
    ("la-alpha", 31, 17, "LA", "gradient"),
    ("p-transparent", 255, 257, "P", "gradient"),
    ("rgba-alpha", 223, 333, "RGBA", "gradient"),
    ("cmyk-hash", 225, 221, "CMYK", "hash"),
    ("rgb-wide-gradient", 720, 181, "RGB", "gradient"),
)
CAPTION_SPECS = (
    ("empty", ""),
    ("ascii", "a photo of a striped geometric puzzle."),
    ("case-whitespace", " \t MiXeD   CASE\r\n with\t whitespace  "),
    ("html", "Tom &amp; Jerry &lt;blue&gt; &#169; &#x1F9ED;"),
    ("html-double", "&amp;amp; &amp;lt;blue&amp;gt; &amp;#233;"),
    ("ftfy", "Fran\u00c3\u00a7ais caf\u00c3\u00a9 isn\u00e2\u20ac\u2122t broken"),
    ("latin", "Cr\u00e8me br\u00fbl\u00e9e, na\u00efve fa\u00e7ade, \u00e5ngstr\u00f6m."),
    ("cjk", "\u4e2d\u6587\u56fe\u6848 \u65e5\u672c\u8a9e\u306e\u5f62 \ud55c\uad6d\uc5b4"),
    ("emoji", "\U0001f9ed \U0001f308 \U0001f469\u200d\U0001f52c \U0001f44d\U0001f3fd"),
    ("non-bmp", "\U0001d11e music \U00020000 ideograph \U00010400 letter"),
    ("digits", "0123456789 9999999999 1234.5678"),
    ("punctuation", "!!!??? ... --- ___ () [] {} <> /\\ :; +*= @#$%"),
    ("contractions", "cat's can't we're they've I'm she'll you'd"),
    # clip.py tokenizes special-token text as plain text; this row shows what upstream does.
    ("literal-special", "<start_of_text>"),
    # 'a' is one text token in CLIP BPE, regardless of the merges table.
    ("tokens-75", " ".join(["a"] * 75)),
    ("tokens-long", " ".join(["a"] * 100)),
    ("combining", "cafe\u0301 A\u030a o\u0308 versus caf\u00e9 \u00c5 \u00f6"),
    ("rtl", "\u0645\u0631\u062d\u0628\u0627 \u0628\u0627\u0644\u0639\u0627\u0644\u0645 \u05e9\u05dc\u05d5\u05dd"),
)
BATCH_SPECS = (
    ("image", ("rgb-224-gradient", "rgb-small", "rgb-odd-tall", "rgb-review-stripes", "p-transparent", "rgba-alpha")),
    ("text", ("empty", "ascii", "latin", "emoji", "tokens-75", "tokens-long")),
)


class FixtureError(ValueError):
    """A missing or invalid reference is a setup problem, never a pass."""


def _pixels(w, h, mode, pattern):
    y, x = np.indices((h, w), dtype=np.uint32)
    planes = []
    for c in range(len(mode) if mode not in ("L", "P") else 1):
        if pattern == "gradient":
            v = x * 255 // max(w - 1, 1) + y * 255 // max(h - 1, 1) + c * 73
        elif pattern == "rings":
            dx, dy = 2 * x.astype(np.int64) - (w - 1), 2 * y.astype(np.int64) - (h - 1)
            v = ((dx * dx + dy * dy) // 29 + c * 53) % 256
        elif pattern == "hash":
            # uint32 wraparound is intentional, identical on both platforms.
            v = (x * np.uint32(0x45D9F3B)) ^ (y * np.uint32(0x27D4EB2D)) ^ np.uint32(c * 0x9E3779B9 & 0xFFFFFFFF)
            v ^= v >> 16
            v *= np.uint32(0x85EBCA6B)
            v ^= v >> 13
        elif pattern == "stripes":
            v = ((x // 7) % 2) * 255 + y * 3 + c * 61
        else:
            v = ((x + y if pattern == "checker" else x if pattern == "stripe-x" else y) % 2) * 255
        if mode in ("LA", "RGBA") and c == len(mode) - 1:
            v = x * 17 + y * 29
        planes.append((v & 255).astype(np.uint8))
    return np.stack(planes, axis=-1).tobytes()


def cases():
    """Return ordered image/caption dictionaries and two batches of their ids.

    Each image case holds its synthetic `source` and the production RGB `image` that both sides see.
    """
    image_cases = []
    for name, w, h, mode, pattern in IMAGE_SPECS:
        source = Image.frombytes(mode, (w, h), _pixels(w, h, mode, pattern))
        if mode == "P":
            source.putpalette(bytes((i * (c * 2 + 1) + c * 37) & 255 for i in range(256) for c in range(3)))
            source.info["transparency"] = 0
        image_cases.append({"id": name, "source": source, "image": images.to_rgb(source)})
    return {
        "images": image_cases,
        "texts": [{"id": name, "text": text} for name, text in CAPTION_SPECS],
        "batches": [{"kind": kind, "ids": list(ids)} for kind, ids in BATCH_SPECS],
    }


def input_sha256(case):
    if "text" in case:
        return hashlib.sha256(case["text"].encode("utf-8")).hexdigest()
    image = case["image"]
    digest = hashlib.sha256(image.mode.encode("ascii") + b"\0" + struct.pack("<II", *image.size) + image.tobytes())
    if image.mode == "P":
        digest.update(bytes(image.getpalette()))
        digest.update(struct.pack("<I", image.info["transparency"]))
    return digest.hexdigest()


def cases_digest(inputs):
    rows = [[kind, case["id"], input_sha256(case)] for kind in ("images", "texts") for case in inputs[kind]]
    return hashlib.sha256(json.dumps(rows, separators=(",", ":"), ensure_ascii=True).encode("ascii")).hexdigest()


def encode_floats(values):
    return base64.b64encode(np.asarray(values, dtype="<f4").tobytes()).decode("ascii")


def decode_floats(value, length=None):
    try:
        raw = base64.b64decode(value, validate=True) if isinstance(value, str) else b""
    except (ValueError, binascii.Error):
        raise FixtureError("invalid float32 base64") from None
    if not raw or len(raw) % 4 or (length is not None and len(raw) != length * 4):
        raise FixtureError("wrong float32 array length")
    array = np.frombuffer(raw, dtype="<f4")
    if not np.isfinite(array).all():
        raise FixtureError("non-finite float32 value")
    return array


def _keys(value, keys, label):
    if not isinstance(value, dict) or set(value) != set(keys):
        raise FixtureError(f"{label}: unknown or missing keys")
    return {key: value[key] for key in keys}


def validate_fixture(fixture):
    """Validate all lengths/values and return dictionaries in canonical key order."""
    f = _keys(fixture, ("format", "version", "producer", "pins", "cases_digest", "images", "texts", "batches"), "fixture")
    if f["format"] != FORMAT or type(f["version"]) is not int or f["version"] != VERSION:
        raise FixtureError("unsupported fixture format or version")
    p = _keys(f["producer"], ("python", "platform", "torch", "torchvision", "open_clip", "pillow", "numpy", "transform"), "producer")
    if any(not isinstance(p[key], str) or not p[key] for key in p if key != "transform"):
        raise FixtureError("invalid producer version")
    t = _keys(p["transform"], ("resize_size", "interpolation", "crop_size", "mean", "std"), "transform")
    for key, lengths in (("resize_size", (1, 2)), ("crop_size", (2,))):
        if not isinstance(t[key], list) or len(t[key]) not in lengths or any(type(v) is not int or v <= 0 for v in t[key]):
            raise FixtureError("invalid transform size")
    if not isinstance(t["interpolation"], str) or not t["interpolation"]:
        raise FixtureError("invalid interpolation")
    decode_floats(t["mean"], 3)
    if (decode_floats(t["std"], 3) <= 0).any():
        raise FixtureError("invalid transform std")
    p["transform"], f["producer"] = t, p
    pins = _keys(f["pins"], ("weights_sha256", "weights_size", "merges_sha256"), "pins")
    if not common.valid_sha(pins["weights_sha256"]) or not common.valid_sha(pins["merges_sha256"]) or type(pins["weights_size"]) is not int or pins["weights_size"] <= 0:
        raise FixtureError("invalid model pins")
    if not common.valid_sha(f["cases_digest"]):
        raise FixtureError("invalid cases digest")
    f["pins"] = pins
    dim = None
    for kind, specs in (("images", IMAGE_SPECS), ("texts", CAPTION_SPECS)):
        rows = f[kind]
        if not isinstance(rows, list) or len(rows) != len(specs):
            raise FixtureError(f"{kind}: wrong case count")
        canonical = []
        for row, spec in zip(rows, specs):
            row = _keys(row, ("id", "input_sha256", "pre" if kind == "images" else "tokens", "features"), kind)
            if row["id"] != spec[0] or not common.valid_sha(row["input_sha256"]):
                raise FixtureError(f"{kind}: invalid case id or input digest")
            if kind == "images":
                pre = _keys(row["pre"], ("mean", "std", "samples", "tensor_sha256"), "pre")
                decode_floats(pre["mean"], 3)
                if (decode_floats(pre["std"], 3) < 0).any():
                    raise FixtureError("negative channel std")
                decode_floats(pre["samples"], len(SAMPLE_INDICES))
                if not common.valid_sha(pre["tensor_sha256"]):
                    raise FixtureError("invalid tensor digest")
                row["pre"] = pre
            else:
                tokens = row["tokens"]
                if not isinstance(tokens, list) or len(tokens) != 77 or any(type(v) is not int or not 0 <= v < 49408 for v in tokens):
                    raise FixtureError("invalid token ids or length")
            features = decode_floats(row["features"], dim)
            if len(features) < 2 or not np.any(features):
                raise FixtureError("invalid feature row")
            dim = len(features)
            canonical.append(row)
        f[kind] = canonical
    if not isinstance(f["batches"], list) or len(f["batches"]) != len(BATCH_SPECS):
        raise FixtureError("wrong batch count")
    batches = []
    for row, (kind, ids) in zip(f["batches"], BATCH_SPECS):
        row = _keys(row, ("kind", "ids", "features"), "batch")
        if row["kind"] != kind or row["ids"] != list(ids):
            raise FixtureError("invalid batch kind or ids")
        values = decode_floats(row["features"], len(ids) * dim).reshape(len(ids), dim)
        if not np.any(values, axis=1).all():
            raise FixtureError("invalid batch feature row")
        batches.append(row)
    f["batches"] = batches
    return f


def fixture_bytes(fixture):
    raw = json.dumps(validate_fixture(fixture), separators=(",", ":"), ensure_ascii=True, allow_nan=False).encode("ascii")
    if len(raw) > MAX_FIXTURE_BYTES:
        raise FixtureError("fixture exceeds 512 KiB")
    return raw


def _unique_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise FixtureError("duplicate JSON key")
        result[key] = value
    return result


def read_fixture(path):
    try:
        with Path(path).open("rb") as stream:
            raw = stream.read(MAX_FIXTURE_BYTES + 1)
    except OSError:
        raise FixtureError("fixture is missing or unreadable") from None
    if len(raw) > MAX_FIXTURE_BYTES:
        raise FixtureError("fixture exceeds 512 KiB")
    try:
        fixture = json.loads(raw, object_pairs_hook=_unique_keys)
    except (ValueError, UnicodeError, RecursionError) as exc:
        raise FixtureError("invalid fixture JSON") from exc
    return validate_fixture(fixture)


def reference_pins(pins):
    return {"weights_sha256": pins["clip-vit-b-16"]["sha256"],
            "weights_size": pins["clip-vit-b-16"]["size"],
            "merges_sha256": pins["clip-vit-b-16-merges"]["sha256"]}


def _pre_summary(pixels):
    pixels = np.asarray(pixels, dtype=np.float32)
    if pixels.shape != (3, 224, 224) or not np.isfinite(pixels).all():
        raise FixtureError("preprocess must return finite (3, 224, 224) pixels")
    return {"mean": encode_floats(pixels.mean(axis=(1, 2), dtype=np.float64)),
            "std": encode_floats(pixels.astype(np.float64).std(axis=(1, 2))),
            "samples": encode_floats(pixels.ravel()[list(SAMPLE_INDICES)]),
            "tensor_sha256": hashlib.sha256(np.asarray(pixels, dtype="<f4").tobytes()).hexdigest()}


def build_fixture(upstream, producer):
    """Adapter methods return unnormalised features; its pins describe verified files."""
    inputs = cases()
    fixture = {"format": FORMAT, "version": VERSION, "producer": producer,
               "pins": dict(upstream.pins), "cases_digest": cases_digest(inputs),
               "images": [], "texts": [], "batches": []}
    pixels, token_rows = {}, {}
    for case in inputs["images"]:
        value = np.asarray(upstream.preprocess(case["image"]), dtype=np.float32)
        pre = _pre_summary(value)
        features = np.asarray(upstream.encode_images(value[None]), dtype=np.float32)
        if features.ndim != 2 or features.shape[0] != 1:
            raise FixtureError("encode_images must return (n, dim) features")
        pixels[case["id"]] = value
        fixture["images"].append({"id": case["id"], "input_sha256": input_sha256(case), "pre": pre, "features": encode_floats(features)})
    texts = {case["id"]: case["text"] for case in inputs["texts"]}
    for case in inputs["texts"]:
        tokens = np.asarray(upstream.tokenize([case["text"]]))
        if tokens.shape != (1, 77) or not np.issubdtype(tokens.dtype, np.integer):
            raise FixtureError("tokenize must return (n, 77) integer ids")
        token_rows[case["id"]] = tokens[0]
        features = np.asarray(upstream.encode_texts(tokens), dtype=np.float32)
        if features.ndim != 2 or features.shape[0] != 1:
            raise FixtureError("encode_texts must return (n, dim) features")
        fixture["texts"].append({"id": case["id"], "input_sha256": input_sha256(case), "tokens": tokens[0].tolist(), "features": encode_floats(features)})
    for batch in inputs["batches"]:
        if batch["kind"] == "image":
            features = upstream.encode_images(np.stack([pixels[name] for name in batch["ids"]]))
        else:
            tokens = np.asarray(upstream.tokenize([texts[name] for name in batch["ids"]]))
            if not np.array_equal(tokens, np.stack([token_rows[name] for name in batch["ids"]])):
                raise FixtureError("upstream batch tokens differ from single tokens")
            features = upstream.encode_texts(tokens)
        features = np.asarray(features, dtype=np.float32)
        if features.ndim != 2 or features.shape[0] != len(batch["ids"]):
            raise FixtureError("batch encoder must return (n, dim) features")
        fixture["batches"].append({**batch, "features": encode_floats(features)})
    # Enforce the serialized cap even when the caller keeps the fixture in memory.
    fixture_bytes(fixture)
    return validate_fixture(fixture)


class NumpyAdapter:
    def __init__(self, model, tokenizer, pins):
        self.model, self.tokenizer, self.pins = model, tokenizer, pins

    def tokenize(self, texts):
        return self.tokenizer.tokenize(texts)

    def preprocess(self, image):
        from tastelab import clip
        return clip.preprocess(image)

    def encode_images(self, pixels):
        return self.model.encode_images(pixels)

    def encode_texts(self, tokens):
        return self.model.encode_tokens(tokens)


def load_ours():
    """Both files are verified before either reader runs; nothing is written."""
    from tastelab import clip
    pins = embed.model_pins()
    embed.verify_file(embed.MODEL_FILE, pins["clip-vit-b-16"])
    embed.verify_file(embed.MERGES_FILE, pins["clip-vit-b-16-merges"])
    return NumpyAdapter(clip.load(embed.MODEL_FILE), clip.Tokenizer(embed.MERGES_FILE), reference_pins(pins))


def provenance_problems(producer):
    """Names of producer fields that do not describe OpenCLIP 3.3.0 on Linux with clip.py's transform."""
    from tastelab import clip

    t = producer["transform"]
    expected = {"open_clip": producer["open_clip"] == UPSTREAM_OPEN_CLIP,
                "platform": producer["platform"].startswith("Linux "),
                "torch": producer["torch"] not in ("absent", "synthetic"),
                "resize_size": t["resize_size"] == [clip.IMAGE_SIZE],
                "interpolation": t["interpolation"] == "bicubic",
                "crop_size": t["crop_size"] == [clip.IMAGE_SIZE, clip.IMAGE_SIZE],
                "mean": np.array_equal(decode_floats(t["mean"], 3), clip.MEAN),
                "std": np.array_equal(decode_floats(t["std"], 3), clip.STD)}
    return [name for name, ok in expected.items() if not ok]


def compare(fixture, ours, *, provenance=True):
    """Report named maxima. Setup problems stop before inference; info never fails.

    provenance=False is only for unit tests whose fixture comes from a synthetic adapter.
    """
    checks = []

    def check(name, maximum, passed=True, *, setup=False, info=False, detail=""):
        status = "info" if info else "pass" if passed else "setup problem" if setup else "mismatch"
        checks.append({"name": name, "maximum": maximum, "status": status, "detail": detail})

    def report():
        status = "setup problem" if any(c["status"] == "setup problem" for c in checks) else "mismatch" if any(c["status"] == "mismatch" for c in checks) else "pass"
        return {"status": status, "checks": checks}

    try:
        fixture = validate_fixture(fixture)
        fixture_bytes(fixture)
    except FixtureError as exc:
        check("setup:fixture", 1, False, setup=True, detail=str(exc))
        return report()
    inputs = cases()
    if provenance:
        for name in provenance_problems(fixture["producer"]):
            check("setup:provenance:" + name, 1, False, setup=True)
        equal = cases_digest(inputs) == CASES_DIGEST
        check("setup:cases_digest:committed", int(not equal), equal, setup=True)
    equal = set(ours.pins) == set(fixture["pins"])
    check("setup:pins:keys", int(not equal), equal, setup=True)
    for key, value in fixture["pins"].items():
        equal = ours.pins.get(key) == value
        check("setup:pins:" + key, int(not equal), equal, setup=True)
    for key, value in (("pillow", PILLOW_VERSION), ("numpy", np.__version__)):
        equal = fixture["producer"][key] == value
        check("setup:version:" + key, int(not equal), equal, setup=True)
    equal = fixture["cases_digest"] == cases_digest(inputs)
    check("setup:cases_digest", int(not equal), equal, setup=True)
    for kind in ("images", "texts"):
        for case, ref in zip(inputs[kind], fixture[kind]):
            equal = input_sha256(case) == ref["input_sha256"]
            check(f"setup:{kind}:{case['id']}:input_sha256", int(not equal), equal, setup=True)
    if report()["status"] == "setup problem":
        return report()

    def feature_check(name, actual, reference, info=False):
        ref = np.asarray(reference, dtype=np.float64)
        actual = np.asarray(actual, dtype=np.float64)
        if actual.shape != ref.shape or not np.isfinite(actual).all():
            check(name + ":cosine", float("inf"), False, detail="feature shape or values invalid")
            return
        ref_norm, our_norm = np.linalg.norm(ref, axis=1), np.linalg.norm(actual, axis=1)
        cosine = np.sum(actual * ref, axis=1) / (ref_norm * np.maximum(our_norm, np.finfo(float).tiny))
        cosine = np.clip(cosine, -1, 1)
        error = np.linalg.norm(actual - ref, axis=1) / ref_norm
        check(name + ":cosine", float(np.max(1 - cosine)), bool(np.all(cosine >= FEATURE_COSINE)), info=info,
              detail=f"minimum cosine={cosine.min():.9g}; required >= {FEATURE_COSINE}")
        check(name + ":relative_l2", float(error.max()), info=True)

    dim = len(decode_floats(fixture["images"][0]["features"]))
    pixels, tokens_by_id = {}, {}
    for case, ref in zip(inputs["images"], fixture["images"]):
        name = "image:" + case["id"]
        value = np.asarray(ours.preprocess(case["image"]), dtype=np.float32)
        pre = _pre_summary(value)
        pixels[case["id"]] = value
        for key in ("samples", "mean", "std"):
            error = float(np.max(np.abs(decode_floats(pre[key]).astype(np.float64) - decode_floats(ref["pre"][key]))))
            detail = ""
            if key != "samples":
                detail = f"ours={decode_floats(pre[key]).tolist()}; reference={decode_floats(ref['pre'][key]).tolist()}"
            check(name + ":pre:" + key, error, error <= PRE_ABSOLUTE, info=key != "samples", detail=detail)
        equal = pre["tensor_sha256"] == ref["pre"]["tensor_sha256"]
        check(name + ":pre:tensor_sha256", int(not equal), info=True, detail=f"equal={equal}")
        feature_check(name + ":features", ours.encode_images(value[None]), decode_floats(ref["features"], dim)[None])
    texts = {case["id"]: case["text"] for case in inputs["texts"]}
    for case, ref in zip(inputs["texts"], fixture["texts"]):
        name = "text:" + case["id"]
        tokens = np.asarray(ours.tokenize([case["text"]]))
        expected = np.asarray(ref["tokens"], dtype=np.int64)[None]
        equal = tokens.shape == expected.shape and np.issubdtype(tokens.dtype, np.integer) and np.array_equal(tokens, expected)
        maximum = int(np.max(np.abs(tokens.astype(np.int64) - expected))) if tokens.shape == expected.shape else 1
        check(name + ":tokens", maximum, equal)
        tokens_by_id[case["id"]] = expected[0]
        feature_check(name + ":features", ours.encode_texts(tokens), decode_floats(ref["features"], dim)[None])
    for batch in fixture["batches"]:
        name = "batch:" + batch["kind"]
        if batch["kind"] == "image":
            actual = ours.encode_images(np.stack([pixels[key] for key in batch["ids"]]))
            single = {row["id"]: decode_floats(row["features"], dim) for row in fixture["images"]}
        else:
            tokens = np.asarray(ours.tokenize([texts[key] for key in batch["ids"]]))
            expected = np.stack([tokens_by_id[key] for key in batch["ids"]])
            equal = tokens.shape == expected.shape and np.issubdtype(tokens.dtype, np.integer) and np.array_equal(tokens, expected)
            maximum = int(np.max(np.abs(tokens.astype(np.int64) - expected))) if tokens.shape == expected.shape else 1
            check(name + ":tokens", maximum, equal)
            actual = ours.encode_texts(tokens)
            single = {row["id"]: decode_floats(row["features"], dim) for row in fixture["texts"]}
        reference = decode_floats(batch["features"], len(batch["ids"]) * dim).reshape(-1, dim)
        feature_check(name + ":features", actual, reference)
        feature_check(name + ":reference_vs_single", reference, np.stack([single[key] for key in batch["ids"]]), info=True)
    return report()


def main(argv=None):
    parser = argparse.ArgumentParser(description="Check NumPy CLIP against the upstream fixture, offline.")
    parser.add_argument("command", choices=("check",))
    parser.add_argument("--fixture", type=Path, default=FIXTURE)
    args = parser.parse_args(argv)
    try:
        fixture = read_fixture(args.fixture)
        result = compare(fixture, load_ours())
    except (FixtureError, common.Refused) as exc:
        print(f"setup: {exc}")
        print("summary: setup problem")
        return 2
    except Exception as exc:  # exit 1 means a mismatch, so any other failure is a setup problem
        print(f"setup: unexpected {type(exc).__name__}")
        print("summary: setup problem")
        return 2
    for row in result["checks"]:
        print(f"{row['status']}: {row['name']} maximum={row['maximum']:.9g} {row['detail']}".rstrip())
    print("summary: " + result["status"])
    return {"pass": 0, "mismatch": 1, "setup problem": 2}[result["status"]]


if __name__ == "__main__":
    sys.exit(main())
