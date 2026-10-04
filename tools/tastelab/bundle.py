"""Export up to 300 new class A thumbnails and vectors for the Taste Lab page."""
from __future__ import annotations

import argparse
import base64
import hashlib
import io
import json
import re
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path
from uuid import uuid4

if not __package__:
    sys.path[0] = str(Path(__file__).resolve().parents[1])

import numpy as np
from PIL import Image

from tastelab import common, embed, images, licences, sources, store

FORMAT = "tastelab-images"
VERSION = 1
MAX_ITEMS = 300
THUMB_MAX_BYTES = 204_800
DIM = 512
ID_RE = re.compile(r"[0-9]{8}T[0-9]{6}Z-[0-9a-f]{8}")
MANIFEST_KEYS = {"format", "version", "id", "createdAt", "model", "items", "embeddings"}
ITEM_KEYS = {"imageId", "thumbSha256", "width", "height", "bytes", "source", "sourceId", "title", "credit", "pageUrl", "licence", "licenceUrl"}


def _text(value, minimum, maximum):
    return licences.printable(value) and minimum <= len(value) <= maximum


def _item_ok(item):
    return (isinstance(item, dict) and item.keys() == ITEM_KEYS
            and common.valid_sha(item["imageId"]) and common.valid_sha(item["thumbSha256"])
            and all(type(item[k]) is int and 1 <= item[k] <= common.THUMB_EDGE for k in ("width", "height"))
            and type(item["bytes"]) is int and 1 <= item["bytes"] <= THUMB_MAX_BYTES
            and _text(item["sourceId"], 1, 200) and _text(item["title"], 0, 300)
            and _text(item["credit"], 1, 300) and bool(item["credit"].strip())
            and licences.licence_ok(item["source"], item["licence"], item["licenceUrl"])
            and licences.page_url_ok(item["source"], item["pageUrl"]))


def _vectors_ok(vectors):
    return (vectors.ndim == 2 and vectors.shape[1] == DIM and np.isfinite(vectors).all()
            and np.all(np.abs(np.linalg.norm(vectors.astype(np.float64), axis=1) - 1) <= 1e-2))


def _complete(folder):
    """A complete, valid manifest and its bytes are the record of prior exports."""
    try:
        manifest = json.loads(common.check_input(folder / "manifest.json").read_text(encoding="utf-8"))
        if (manifest.keys() != MANIFEST_KEYS or manifest["format"] != FORMAT or type(manifest["version"]) is not int
                or manifest["version"] != VERSION or manifest["id"] != folder.name or not ID_RE.fullmatch(folder.name)
                or not re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z", manifest["createdAt"])
                or manifest["model"].keys() != {"id", "weightsSha256"} or not _text(manifest["model"]["id"], 1, 200)
                or not common.valid_sha(manifest["model"]["weightsSha256"])):
            return None
        items, embeddings = manifest["items"], manifest["embeddings"]
        if (not isinstance(items, list) or not 1 <= len(items) <= MAX_ITEMS or not all(_item_ok(item) for item in items)
                or len({item["imageId"] for item in items}) != len(items)
                or embeddings.keys() != {"dtype", "dim", "count", "data"} or embeddings["dtype"] != "float16"
                or type(embeddings["dim"]) is not int or embeddings["dim"] != DIM
                or type(embeddings["count"]) is not int or embeddings["count"] != len(items)):
            return None
        raw = base64.b64decode(embeddings["data"], validate=True)
        if len(raw) != len(items) * DIM * 2 or not _vectors_ok(np.frombuffer(raw, dtype="<f2").reshape(-1, DIM)):
            return None
        for item in items:
            data = common.check_input(folder / (item["thumbSha256"] + ".jpg")).read_bytes()
            if (not data.startswith(b"\xff\xd8\xff") or len(data) != item["bytes"]
                    or hashlib.sha256(data).hexdigest() != item["thumbSha256"]):
                return None
            with Image.open(io.BytesIO(data)) as image:
                if image.format != "JPEG" or image.size != (item["width"], item["height"]):
                    return None
                image.verify()
        return manifest
    except (OSError, ValueError, TypeError, KeyError, AttributeError):
        return None


def _thumbnail(path):
    data = common.check_input(path).read_bytes()
    with Image.open(io.BytesIO(data)) as original:
        if original.width * original.height > images.MAX_PIXELS:
            raise ValueError("thumbnail exceeds the pixel cap")
        thumb = original.convert("RGB")
    try:
        thumb.thumbnail((common.THUMB_EDGE, common.THUMB_EDGE), Image.Resampling.LANCZOS)
        for quality in (85, 75, 65, 50, 35, 20):
            output = io.BytesIO()
            thumb.save(output, format="JPEG", quality=quality, optimize=True)
            data = output.getvalue()
            if len(data) <= THUMB_MAX_BYTES:
                return data, thumb.width, thumb.height
        raise ValueError("thumbnail cannot fit the byte cap")
    finally:
        thumb.close()


def export(library, *, maximum=MAX_ITEMS):
    if type(maximum) is not int or not 1 <= maximum <= MAX_ITEMS:
        raise ValueError("--max must be from 1 to 300")
    folder = common.data_root(common.PRIVATE_BASE / "bundles")
    counts = dict(exported=0, already_exported=0, invalid_metadata=0, missing_embedding=0, invalid_embedding=0, invalid_thumbnail=0)
    # The store lock also serializes exporters: a second run cannot clean the
    # first run's live temporary folder or race its manifest record.
    with library._immediate():
        folder.mkdir(parents=True, exist_ok=True)
        for path in folder.glob(".tmp-*"):
            common.check_input(path, kind="temporary bundle")
            if path.is_dir():
                shutil.rmtree(path)
        exported = set()
        for path in folder.iterdir():
            if not ID_RE.fullmatch(path.name):
                continue
            common.check_input(path, kind="bundle")
            if path.is_dir() and (manifest := _complete(path)) is not None:
                exported.update(item["imageId"] for item in manifest["items"])
        weights_sha = embed.model_pins()["clip-vit-b-16"]["sha256"]
        items, vectors, thumbnails = [], [], {}
        for row in library.images(tier="A"):
            if len(items) >= maximum:
                break
            if row.sha256 in exported:
                counts["already_exported"] += 1
                continue
            if not sources.admit(row, "A"):
                counts["invalid_metadata"] += 1
                continue
            blob = library.embeddings(embed.MODEL_ID, [row.sha256]).get(row.sha256)
            if blob is None:
                counts["missing_embedding"] += 1
                continue
            if len(blob) != DIM * 4:
                counts["invalid_embedding"] += 1
                continue
            vector = np.frombuffer(blob, dtype="<f4").reshape(1, DIM)
            if not _vectors_ok(vector):
                counts["invalid_embedding"] += 1
                continue
            half = vector.astype("<f2")
            if not _vectors_ok(half):
                counts["invalid_embedding"] += 1
                continue
            try:
                data, width, height = _thumbnail(library.thumb_path(row.sha256))
            except (OSError, ValueError):
                counts["invalid_thumbnail"] += 1
                continue
            thumb_sha = hashlib.sha256(data).hexdigest()
            item = {"imageId": row.sha256, "thumbSha256": thumb_sha, "width": width, "height": height, "bytes": len(data),
                    "source": row.source, "sourceId": row.source_id, "title": row.title or "", "credit": row.attribution,
                    "pageUrl": row.page_url, "licence": row.licence, "licenceUrl": row.licence_url}
            if not _item_ok(item):
                counts["invalid_metadata"] += 1
                continue
            items.append(item)
            vectors.append(half)
            thumbnails[thumb_sha] = data
        if not items:
            return None, counts
        moment = datetime.now(timezone.utc)
        ident = moment.strftime("%Y%m%dT%H%M%SZ-") + uuid4().hex[:8]
        temporary, destination = folder / (".tmp-" + ident), folder / ident
        if destination.exists():
            raise common.Refused("bundle id already exists; refusing overwrite")
        temporary.mkdir()
        for sha, data in thumbnails.items():
            (temporary / (sha + ".jpg")).write_bytes(data)
        manifest = {"format": FORMAT, "version": VERSION, "id": ident, "createdAt": moment.strftime("%Y-%m-%dT%H:%M:%SZ"),
                    "model": {"id": embed.MODEL_ID, "weightsSha256": weights_sha}, "items": items,
                    "embeddings": {"dtype": "float16", "dim": DIM, "count": len(items),
                                   "data": base64.b64encode(b"".join(vector.tobytes() for vector in vectors)).decode("ascii")}}
        (temporary / "manifest.json").write_text(json.dumps(manifest, allow_nan=False, indent=2) + "\n", encoding="utf-8")
        if destination.exists():
            raise common.Refused("bundle id already exists; refusing overwrite")
        temporary.rename(destination)
        counts["exported"] = len(items)
        return destination, counts


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--max", type=int, default=MAX_ITEMS)
    args = parser.parse_args(argv)
    if not 1 <= args.max <= MAX_ITEMS:
        parser.error("--max must be from 1 to 300")
    try:
        root = common.data_root()
        if not (root / store.DB_NAME).is_file():
            raise common.Refused("bundle export needs an existing library")
        with store.Store(root) as library:
            folder, counts = export(library, maximum=args.max)
        print(str(folder) if folder else "No new bundle")
        print("Bundle counts: " + json.dumps(counts, sort_keys=True))
        return 0
    except (common.Refused, OSError, ValueError) as exc:
        print("refused: " + " ".join(str(exc).split()), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
