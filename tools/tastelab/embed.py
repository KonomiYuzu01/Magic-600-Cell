"""Image and text embeddings behind one small interface (tastelab environment).

`ClipEmbedder` runs OpenCLIP ViT-B/16 trained on LAION-2B (s34B b88K) from the
hash-pinned weights and merges that `bootstrap.py install --profile tastelab`
places under tools/.models/, with the NumPy implementation in clip.py on the CPU
(Windows Smart App Control blocks PyTorch on the owner's machine). It never
downloads anything. `FakeEmbedder` is the deterministic stand-in every test
uses: no model, no network.

Vectors are float32 and L2-normalised, shape (n, dim). The content screen, the
taste model and the axis names all use the same embedder, so no second model is
ever needed.

Command line: embed the stored images and probe texts that have no vector yet.
  python tools/tastelab/embed.py [--data DIR] [--batch N]
"""
from __future__ import annotations

import hashlib
import json
import argparse
import sys
from pathlib import Path
from typing import Protocol, Sequence

if not __package__:
    sys.path[0] = str(Path(__file__).resolve().parents[1])

import numpy as np

from tastelab import common

MODEL_DIR = common.ROOT / "tools" / ".models" / "clip-vit-b-16-laion2b"
MODEL_FILE = MODEL_DIR / "open_clip_model.safetensors"
MERGES_FILE = MODEL_DIR / "merges.txt"
LOCK_FILE = common.ROOT / "tools" / "toolchain.lock.json"
MODEL_ID = "clip-vit-b-16-laion2b-numpy1"   # weights and implementation: stored vectors are valid for this id only


class Embedder(Protocol):
    model_id: str
    dim: int
    logit_scale: float      # the model's learned temperature for zero-shot softmax

    def embed_images(self, images: Sequence) -> np.ndarray:
        """PIL images (RGB) to an (n, dim) float32 array of unit vectors."""

    def embed_texts(self, texts: Sequence[str]) -> np.ndarray:
        """Texts to an (n, dim) float32 array of unit vectors."""


def normalise(m) -> np.ndarray:
    m = np.asarray(m, dtype=np.float32)
    norms = np.linalg.norm(m, axis=-1, keepdims=True)
    return m / np.maximum(norms, 1e-12)


def to_blob(vector) -> bytes:
    return np.asarray(vector, dtype="<f4").tobytes()


def from_blobs(blobs: Sequence[bytes], dim: int) -> np.ndarray:
    if not blobs:
        return np.zeros((0, dim), dtype=np.float32)
    return np.frombuffer(b"".join(blobs), dtype="<f4").reshape(-1, dim).astype(np.float32)


class FakeEmbedder:
    """Deterministic stand-in for tests.

    An image vector is a fixed random projection of its 4 x 4 colour layout, so
    images of similar colour get similar vectors. A text vector is a direction
    seeded by the text's hash, unless `texts` pins it, which lets a test place a
    probe exactly where it needs it."""

    model_id = "fake-16"
    dim = 16
    logit_scale = 100.0

    def __init__(self, texts: dict | None = None):
        rng = np.random.default_rng(600)
        self._projection = rng.standard_normal((49, self.dim)).astype(np.float32)
        self._texts = {t: normalise(np.asarray(v, dtype=np.float32)) for t, v in (texts or {}).items()}

    def embed_images(self, images: Sequence) -> np.ndarray:
        rows = []
        for image in images:
            small = np.asarray(image.convert("RGB").resize((4, 4)), dtype=np.float32).reshape(-1) / 255.0 - 0.5
            rows.append(np.append(small, 1.0))
        if not rows:
            return np.zeros((0, self.dim), dtype=np.float32)
        return normalise(np.stack(rows) @ self._projection)

    def embed_texts(self, texts: Sequence[str]) -> np.ndarray:
        rows = []
        for text in texts:
            if text in self._texts:
                rows.append(self._texts[text])
            else:
                seed = int.from_bytes(hashlib.sha256(text.encode("utf-8")).digest()[:8], "little")
                rows.append(np.random.default_rng(seed).standard_normal(self.dim).astype(np.float32))
        if not rows:
            return np.zeros((0, self.dim), dtype=np.float32)
        return normalise(np.stack(rows))


def model_pins() -> dict:
    """Read the two entries by id; never substitute guessed pins or another checkout."""
    try:
        entries = json.loads(LOCK_FILE.read_text(encoding="utf-8"))["tools"]
    except (OSError, ValueError, KeyError, TypeError) as exc:
        raise common.Refused("cannot read model pins from tools/toolchain.lock.json") from exc
    pins = {}
    for name in ("clip-vit-b-16", "clip-vit-b-16-merges"):
        matches = [row for row in entries if isinstance(row, dict) and row.get("id") == name]
        if len(matches) != 1:
            raise common.Refused(f"tools/toolchain.lock.json: missing or duplicate entry {name}")
        row = matches[0]
        if not common.valid_sha(row.get("sha256")) or type(row.get("size")) is not int or row["size"] <= 0:
            raise common.Refused(f"tools/toolchain.lock.json: invalid SHA-256 or size for {name}")
        pins[name] = row
    return pins


def verify_file(path: Path, pin: dict) -> None:
    try:
        if path.stat().st_size != pin["size"]:
            raise common.Refused(f"{path.name}: size does not match the pinned file")
        with path.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
    except OSError as exc:
        raise common.Refused(f"{path.name}: model file is missing or unreadable") from exc
    if digest != pin["sha256"]:
        raise common.Refused(f"{path.name}: SHA-256 does not match the pinned file")


class ClipEmbedder:
    """CLIP ViT-B/16 (LAION-2B s34B b88K) from MODEL_FILE and MERGES_FILE, offline, on the CPU.

    Refuses to start when a model file is missing or has the wrong size; run
    `python tools/toolchain/bootstrap.py install --profile tastelab`."""

    model_id = MODEL_ID
    dim = 512

    def __init__(self, batch: int = 16):
        from tastelab import clip
        pins = model_pins()
        verify_file(MODEL_FILE, pins["clip-vit-b-16"])
        verify_file(MERGES_FILE, pins["clip-vit-b-16-merges"])
        self.weights_sha256 = pins["clip-vit-b-16"]["sha256"]
        self._clip = clip
        self._model = clip.load(MODEL_FILE)
        self._tokenizer = clip.Tokenizer(MERGES_FILE)
        if self._model.dim != self.dim:
            raise ValueError(f"{MODEL_FILE.name} has {self._model.dim}-dimensional embeddings, not {self.dim}")
        self.logit_scale = self._model.logit_scale
        self.batch = max(1, int(batch))

    def embed_images(self, images: Sequence) -> np.ndarray:
        images = list(images)
        rows = [self._model.encode_images(np.stack([self._clip.preprocess(im) for im in images[i:i + self.batch]]))
                for i in range(0, len(images), self.batch)]
        return normalise(np.concatenate(rows)) if rows else np.zeros((0, self.dim), dtype=np.float32)

    def embed_texts(self, texts: Sequence[str]) -> np.ndarray:
        texts = list(texts)
        rows = [self._model.encode_tokens(self._tokenizer.tokenize(texts[i:i + 64])) for i in range(0, len(texts), 64)]
        return normalise(np.concatenate(rows)) if rows else np.zeros((0, self.dim), dtype=np.float32)


def load_embedder(batch: int = 16) -> Embedder:
    return ClipEmbedder(batch=batch)


def main(argv=None) -> int:
    from PIL import Image
    from tastelab import seeds, store

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data")
    parser.add_argument("--batch", type=int, default=16)
    args = parser.parse_args(argv)
    if not 1 <= args.batch <= 64:
        parser.error("--batch must be from 1 to 64")
    try:
        root = common.data_root(args.data)
        with store.Store(root) as source:
            embedder = load_embedder(args.batch)
            missing = source.missing_embeddings(embedder.model_id)
            count, progress = 0, 500
            for start in range(0, len(missing), 64):
                shas, images = [], []
                for sha in missing[start:start + 64]:
                    try:
                        with Image.open(source.thumb_path(sha)) as image:
                            images.append(image.convert("RGB"))
                        shas.append(sha)
                    except (OSError, ValueError) as exc:
                        print(f"skipped {sha[:12]}: {exc}", file=sys.stderr)
                if images:
                    try:
                        vectors = embedder.embed_images(images)
                        source.put_embeddings(embedder.model_id, embedder.dim,
                                              [(sha, to_blob(v)) for sha, v in zip(shas, vectors)])
                    finally:
                        for image in images:
                            image.close()
                    count += len(shas)
                    while count >= progress:
                        print(f"embedded {progress} images")
                        progress += 500
            probes = seeds.load_probes(seeds.ensure(root, seeds.PROBES_FILE, seeds.PROBES_DEFAULT))
            texts = [probes.phrase(t) for t in probes.texts()]
            known = source.text_embeddings(embedder.model_id, texts)
            texts = [text for text in texts if text not in known]
            for start in range(0, len(texts), 64):
                batch = texts[start:start + 64]
                vectors = embedder.embed_texts(batch)
                source.put_text_embeddings(embedder.model_id, embedder.dim,
                                           [(text, to_blob(v)) for text, v in zip(batch, vectors)])
        print(f"embedded {count} images and {len(texts)} probe texts")
        return 0
    except common.Refused as exc:
        print("refused: " + " ".join(str(exc).split()), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
