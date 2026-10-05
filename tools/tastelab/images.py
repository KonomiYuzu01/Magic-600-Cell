"""Decode fetched image bytes safely and make the stored thumbnail (Pillow, tastelab environment).

Only JPEG, PNG, WebP and GIF are accepted. The pixel count is checked from the
header before any pixel is decoded, so a decompression bomb is refused cheaply.
An animated image (GIF, APNG, animated WebP) is reduced to its first frame: that
frame is what the content screen sees and the only thing Taste Lab keeps, so the
original file of an animated image is never stored. Thumbnails carry no EXIF or
other metadata.
"""
from __future__ import annotations

import io
import warnings
from dataclasses import dataclass

from PIL import Image, ImageOps

from tastelab import common

FORMATS = {"JPEG": "jpg", "PNG": "png", "WEBP": "webp", "GIF": "gif"}
MAX_PIXELS = 40_000_000
MAX_ASPECT = 64          # longer side / shorter side; bounds the model's full-size resize at 224 x 14,336
MIN_EDGE = 64
JPEG_QUALITY = 85


class BadImage(ValueError):
    """Bytes that are not a usable image."""


@dataclass
class Decoded:
    image: Image.Image      # RGB, orientation applied, first frame only
    width: int
    height: int
    ext: str                # extension of the original format
    animated: bool


def decode(data: bytes) -> Decoded:
    if not data:
        raise BadImage("empty")
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("error", Image.DecompressionBombWarning)
            im = Image.open(io.BytesIO(data))
            fmt = im.format
            if fmt not in FORMATS:
                raise BadImage(f"format {fmt} is not accepted")
            width, height = im.size
            if width * height > MAX_PIXELS:
                raise BadImage("too many pixels")
            if max(width, height) > MAX_ASPECT * min(width, height):
                raise BadImage("aspect ratio above 64")
            animated = bool(getattr(im, "is_animated", False)) or getattr(im, "n_frames", 1) > 1
            im.seek(0)
            im.load()
            im = ImageOps.exif_transpose(im)
            if im.mode in ("RGBA", "LA", "P", "PA") or "transparency" in im.info:
                rgba = im.convert("RGBA")
                flat = Image.new("RGB", rgba.size, (255, 255, 255))
                flat.paste(rgba, mask=rgba.getchannel("A"))
                im = flat
            else:
                im = im.convert("RGB")
    except BadImage:
        raise
    except (OSError, ValueError, SyntaxError, EOFError, MemoryError, Image.DecompressionBombError,
            Image.DecompressionBombWarning) as exc:
        raise BadImage(type(exc).__name__) from None
    if min(im.size) < MIN_EDGE:
        raise BadImage("too small")
    return Decoded(im, im.size[0], im.size[1], FORMATS[fmt], animated)


def thumbnail_jpeg(image: Image.Image, edge: int = common.THUMB_EDGE) -> bytes:
    """A JPEG no larger than `edge` on its longer side, with no metadata."""
    thumb = image.copy()
    thumb.thumbnail((edge, edge), Image.Resampling.LANCZOS)
    thumb = Image.frombytes(thumb.mode, thumb.size, thumb.tobytes())
    out = io.BytesIO()
    thumb.save(out, format="JPEG", quality=JPEG_QUALITY, optimize=True)
    return out.getvalue()
