"""CLIP ViT-B/16 image and text encoders in NumPy (tastelab environment).

Taste Lab runs the hash-pinned OpenCLIP weights (LAION-2B s34B b88K) on the CPU
with NumPy and SciPy alone: Windows Smart App Control blocks the unsigned
PyTorch DLLs on the owner's machine. The computation is OpenCLIP's ViT-B-16
(open_clip 3.3.0, standard GELU, no QuickGELU):

  image  bicubic resize of the shorter side to 224, centre crop, RGB, OpenAI mean and std;
         16 x 16 patches (conv1 without bias) + class token + positional embedding, ln_pre,
         12 pre-norm residual blocks, ln_post on the class token, @ visual.proj
  text   CLIP byte-pair tokens (77: start token, text, end token, zero padding), token and
         positional embedding, 12 causal pre-norm blocks, ln_final, the end token's row,
         @ text_projection
  block  x + attn(ln_1(x)), then x + c_proj(gelu(c_fc(ln_2(x)))); heads 64 wide; exact
         (erf) GELU; LayerNorm eps 1e-5

`load` reads the safetensors file directly (float32 tensors only) and `Tokenizer`
builds the vocabulary from merges.txt as OpenAI CLIP does. Nothing here
downloads anything or runs code from the model files.
"""
from __future__ import annotations

import functools
import html
import json
import math
import struct
from pathlib import Path

import ftfy
import numpy as np
import regex
from PIL import Image
from scipy.special import erf

IMAGE_SIZE = 224
CONTEXT = 77
MEAN = np.array([0.48145466, 0.4578275, 0.40821073], dtype=np.float32)
STD = np.array([0.26862954, 0.26130258, 0.27577711], dtype=np.float32)
HEAD_WIDTH = 64
EPS = 1e-5
MERGES = 49152 - 256 - 2        # merges used by CLIP (lines 1..48894 of merges.txt)
MAX_HEADER = 16 * 1024 * 1024   # safetensors header bytes
MAX_ASPECT = 4                  # longer images are cut to this aspect around the centre before resizing

# Special tokens are not matched in the text: a caption that contains them is tokenized as plain text.
_PATTERN = regex.compile(r"""'s|'t|'re|'ve|'m|'ll|'d|[\p{L}]+|[\p{N}]|[^\s\p{L}\p{N}]+""", regex.IGNORECASE)


@functools.lru_cache(maxsize=None)
def bytes_to_unicode() -> dict:
    """OpenAI CLIP's reversible map from bytes to printable characters."""
    bs = list(range(ord("!"), ord("~") + 1)) + list(range(ord("¡"), ord("¬") + 1)) + list(range(ord("®"), ord("ÿ") + 1))
    cs = bs[:]
    n = 0
    for b in range(256):
        if b not in bs:
            bs.append(b)
            cs.append(256 + n)
            n += 1
    return dict(zip(bs, (chr(c) for c in cs)))


def _pairs(word: tuple) -> set:
    return {(word[i], word[i + 1]) for i in range(len(word) - 1)}


class Tokenizer:
    """OpenAI CLIP's byte-pair tokenizer with lower-case cleaning, built from merges.txt."""

    def __init__(self, merges_path):
        lines = Path(merges_path).read_text(encoding="utf-8").split("\n")
        merges = [tuple(line.split()) for line in lines[1:MERGES + 1]]
        if len(merges) != MERGES or any(len(m) != 2 for m in merges):
            raise ValueError(f"{merges_path}: not a CLIP merges file")
        chars = list(bytes_to_unicode().values())
        vocab = chars + [c + "</w>" for c in chars] + ["".join(m) for m in merges] + ["<start_of_text>", "<end_of_text>"]
        self.encoder = dict(zip(vocab, range(len(vocab))))
        self.ranks = dict(zip(merges, range(len(merges))))
        self.sot = self.encoder["<start_of_text>"]
        self.eot = self.encoder["<end_of_text>"]
        self._cache: dict = {}

    def _bpe(self, token: str) -> list:
        cached = self._cache.get(token)
        if cached is not None:
            return cached
        word = tuple(token[:-1]) + (token[-1] + "</w>",)
        pairs = _pairs(word)
        while pairs:
            first, second = min(pairs, key=lambda pair: self.ranks.get(pair, math.inf))
            if (first, second) not in self.ranks:
                break
            merged, i = [], 0
            while i < len(word):
                try:
                    j = word.index(first, i)
                except ValueError:
                    merged.extend(word[i:])
                    break
                merged.extend(word[i:j])
                i = j
                if i < len(word) - 1 and word[i + 1] == second:
                    merged.append(first + second)
                    i += 2
                else:
                    merged.append(word[i])
                    i += 1
            word = tuple(merged)
            pairs = _pairs(word) if len(word) > 1 else set()
        result = [self.encoder[part] for part in word]
        if len(self._cache) < 100_000:
            self._cache[token] = result
        return result

    def encode(self, text: str) -> list:
        text = html.unescape(html.unescape(ftfy.fix_text(text))).strip()
        text = " ".join(text.split()).lower()
        byte_chars = bytes_to_unicode()
        ids = []
        for token in _PATTERN.findall(text):
            ids.extend(self._bpe("".join(byte_chars[b] for b in token.encode("utf-8"))))
        return ids

    def tokenize(self, texts) -> np.ndarray:
        """(n, 77) token ids; a long text is cut and keeps its end token."""
        out = np.zeros((len(texts), CONTEXT), dtype=np.int64)
        for row, text in enumerate(texts):
            ids = [self.sot] + self.encode(text) + [self.eot]
            if len(ids) > CONTEXT:
                ids = ids[:CONTEXT]
                ids[-1] = self.eot
            out[row, :len(ids)] = ids
        return out


def preprocess(image: Image.Image) -> np.ndarray:
    """A PIL image to the model's (3, 224, 224) float32 input."""
    image = image.convert("RGB")
    w, h = image.size
    if max(w, h) > MAX_ASPECT * min(w, h):     # only the centre survives the crop; keep a margin around it
        if w > h:
            keep = MAX_ASPECT * h
            image = image.crop(((w - keep) // 2, 0, (w - keep) // 2 + keep, h))
        else:
            keep = MAX_ASPECT * w
            image = image.crop((0, (h - keep) // 2, w, (h - keep) // 2 + keep))
        w, h = image.size
    if w <= h:
        size = (IMAGE_SIZE, int(IMAGE_SIZE * h / w))
    else:
        size = (int(IMAGE_SIZE * w / h), IMAGE_SIZE)
    image = image.resize(size, Image.Resampling.BICUBIC)
    left = int(round((size[0] - IMAGE_SIZE) / 2.0))
    top = int(round((size[1] - IMAGE_SIZE) / 2.0))
    image = image.crop((left, top, left + IMAGE_SIZE, top + IMAGE_SIZE))
    pixels = np.asarray(image, dtype=np.float32) / np.float32(255.0)
    return np.ascontiguousarray(((pixels - MEAN) / STD).transpose(2, 0, 1))


def read_safetensors(path) -> dict:
    """All tensors of a safetensors file as float32 arrays; anything but plain F32 tensors is refused."""
    path = Path(path)
    size = path.stat().st_size
    with path.open("rb") as f:
        (header_len,) = struct.unpack("<Q", f.read(8))
        if not 2 <= header_len <= min(MAX_HEADER, size - 8):
            raise ValueError(f"{path.name}: bad safetensors header")
        header = json.loads(f.read(header_len).decode("utf-8"))
        base = 8 + header_len
        tensors = {}
        for name, spec in header.items():
            if name == "__metadata__":
                continue
            begin, end = spec["data_offsets"]
            count = math.prod(spec["shape"])
            if spec["dtype"] != "F32" or end - begin != 4 * count or not 0 <= begin <= end <= size - base:
                raise ValueError(f"{path.name}: tensor {name} is not a plain float32 tensor inside the file")
            f.seek(base + begin)
            tensors[name] = np.fromfile(f, dtype="<f4", count=count).reshape(spec["shape"])
    return tensors


def _layer_norm(x, weight, bias):
    centred = x - x.mean(axis=-1, keepdims=True)
    var = np.mean(centred * centred, axis=-1, keepdims=True)
    return centred / np.sqrt(var + np.float32(EPS)) * weight + bias


def _gelu(x):
    return np.float32(0.5) * x * (np.float32(1.0) + erf(x * np.float32(1.0 / math.sqrt(2.0))))


def _attention(x, p, causal: bool):
    n, length, width = x.shape
    heads = width // HEAD_WIDTH
    qkv = (x.reshape(n * length, width) @ p["qkv_w"] + p["qkv_b"]).reshape(n, length, 3, heads, HEAD_WIDTH)
    qkv = qkv.transpose(2, 0, 3, 1, 4)                     # (3, n, heads, length, 64)
    q = np.ascontiguousarray(qkv[0]) * np.float32(1.0 / math.sqrt(HEAD_WIDTH))
    k = np.ascontiguousarray(qkv[1])
    v = np.ascontiguousarray(qkv[2])
    scores = q @ k.transpose(0, 1, 3, 2)                   # (n, heads, length, length)
    if causal:
        scores += np.triu(np.full((length, length), -np.inf, dtype=np.float32), 1)
    scores -= scores.max(axis=-1, keepdims=True)
    np.exp(scores, out=scores)
    scores /= scores.sum(axis=-1, keepdims=True)
    out = (scores @ v).transpose(0, 2, 1, 3).reshape(n * length, width)
    return (out @ p["out_w"] + p["out_b"]).reshape(n, length, width)


def _block(x, p, causal: bool):
    x = x + _attention(_layer_norm(x, p["ln1_w"], p["ln1_b"]), p, causal)
    n, length, width = x.shape
    hidden = _layer_norm(x, p["ln2_w"], p["ln2_b"]).reshape(n * length, width) @ p["fc_w"] + p["fc_b"]
    return x + (_gelu(hidden) @ p["proj_w"] + p["proj_b"]).reshape(n, length, width)


def _blocks(t: dict, prefix: str) -> list:
    """Pop the residual blocks under `prefix`, with linear weights transposed to (in, out)."""
    blocks, i = [], 0
    while f"{prefix}{i}.ln_1.weight" in t:
        b = f"{prefix}{i}."
        blocks.append({
            "ln1_w": t.pop(b + "ln_1.weight"), "ln1_b": t.pop(b + "ln_1.bias"),
            "qkv_w": np.ascontiguousarray(t.pop(b + "attn.in_proj_weight").T), "qkv_b": t.pop(b + "attn.in_proj_bias"),
            "out_w": np.ascontiguousarray(t.pop(b + "attn.out_proj.weight").T), "out_b": t.pop(b + "attn.out_proj.bias"),
            "ln2_w": t.pop(b + "ln_2.weight"), "ln2_b": t.pop(b + "ln_2.bias"),
            "fc_w": np.ascontiguousarray(t.pop(b + "mlp.c_fc.weight").T), "fc_b": t.pop(b + "mlp.c_fc.bias"),
            "proj_w": np.ascontiguousarray(t.pop(b + "mlp.c_proj.weight").T), "proj_b": t.pop(b + "mlp.c_proj.bias"),
        })
        i += 1
    return blocks


class ClipModel:
    """The two towers of an OpenCLIP ViT model from its tensors (sizes are read from the shapes)."""

    def __init__(self, tensors: dict):
        t = dict(tensors)
        try:
            self.logit_scale = float(np.exp(t.pop("logit_scale")))
            conv = t.pop("visual.conv1.weight")                         # (width, 3, patch, patch)
            self.patch = conv.shape[2]
            self.width = conv.shape[0]
            self.patch_w = np.ascontiguousarray(conv.reshape(self.width, -1).T)
            self.class_emb = t.pop("visual.class_embedding")
            self.v_pos = t.pop("visual.positional_embedding")
            self.grid = math.isqrt(self.v_pos.shape[0] - 1)
            self.ln_pre = (t.pop("visual.ln_pre.weight"), t.pop("visual.ln_pre.bias"))
            self.ln_post = (t.pop("visual.ln_post.weight"), t.pop("visual.ln_post.bias"))
            self.v_proj = t.pop("visual.proj")                          # (width, dim), used as x @ proj
            self.v_blocks = _blocks(t, "visual.transformer.resblocks.")
            self.tok_emb = t.pop("token_embedding.weight")
            self.t_pos = t.pop("positional_embedding")
            self.ln_final = (t.pop("ln_final.weight"), t.pop("ln_final.bias"))
            self.t_proj = t.pop("text_projection")                      # (width, dim), used as x @ projection
            self.t_blocks = _blocks(t, "transformer.resblocks.")
        except KeyError as exc:
            raise ValueError(f"not an OpenCLIP ViT checkpoint: {exc} is missing") from None
        if t or self.grid * self.grid + 1 != self.v_pos.shape[0] or not self.v_blocks or not self.t_blocks:
            raise ValueError(f"not an OpenCLIP ViT checkpoint (unexpected tensors: {sorted(t)[:3]})")
        self.image_size = self.grid * self.patch
        self.dim = self.v_proj.shape[1]

    def encode_images(self, pixels: np.ndarray) -> np.ndarray:
        """(n, 3, size, size) model inputs to (n, dim) image features (not normalised)."""
        n = pixels.shape[0]
        g, p = self.grid, self.patch
        patches = pixels.reshape(n, 3, g, p, g, p).transpose(0, 2, 4, 1, 3, 5).reshape(n * g * g, 3 * p * p)
        x = (patches @ self.patch_w).reshape(n, g * g, self.width)
        x = np.concatenate([np.broadcast_to(self.class_emb, (n, 1, self.width)), x], axis=1) + self.v_pos
        x = _layer_norm(x, *self.ln_pre)
        for block in self.v_blocks:
            x = _block(x, block, causal=False)
        return _layer_norm(x[:, 0], *self.ln_post) @ self.v_proj

    def encode_tokens(self, tokens: np.ndarray) -> np.ndarray:
        """(n, 77) token ids to (n, dim) text features (not normalised).

        Positions after the last end token never reach an end token through the causal
        mask, so the towers run only up to it."""
        end = tokens.argmax(axis=-1)
        length = int(end.max()) + 1
        x = self.tok_emb[tokens[:, :length]] + self.t_pos[:length]
        for block in self.t_blocks:
            x = _block(x, block, causal=True)
        return _layer_norm(x[np.arange(len(tokens)), end], *self.ln_final) @ self.t_proj


def load(model_file) -> ClipModel:
    return ClipModel(read_safetensors(model_file))
