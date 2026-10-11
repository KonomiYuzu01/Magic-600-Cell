"""Mirror proven class A bundles into a local public Taste Lab checkout."""
from __future__ import annotations

import argparse
import base64
import json
import os
import sqlite3
import stat
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

if not __package__:
    sys.path[0] = str(Path(__file__).resolve().parents[1])

import numpy as np

from tastelab import bundle, common, embed, licences, sources, store, sync_public

ROOT = Path(__file__).resolve().parents[2]


@dataclass(frozen=True)
class Plan:
    writes: tuple[str, ...]
    removals: tuple[str, ...]
    directories: tuple[str, ...]
    lines: tuple[str, ...]


def _item_failures(library, model, item, thumbnail, vector):
    row = library.image(item["imageId"])
    if row is None:
        return {"missing"}
    reasons = set()
    if row.tier != "A":
        reasons.add("tier")
    if not sources.admit(row, "A"):
        reasons.add("admission")
    if not licences.licence_ok(row.source, row.licence, row.licence_url):
        reasons.add("licence")
    attribution = {"source": row.source, "sourceId": row.source_id, "title": row.title or "",
                   "credit": row.attribution, "pageUrl": row.page_url, "licence": row.licence,
                   "licenceUrl": row.licence_url}
    if any(item[key] != value for key, value in attribution.items()):
        reasons.add("attribution")
    try:
        data, width, height = bundle._thumbnail(library.thumb_path(row.sha256))
        if (thumbnail, item["width"], item["height"]) != (data, width, height):
            reasons.add("thumbnail")
    except (common.Refused, OSError, ValueError):
        reasons.add("thumbnail")
    if model != embed.MODEL_ID:
        reasons.add("model")
    blob = library.embeddings(embed.MODEL_ID, [row.sha256]).get(row.sha256)
    if (blob is None or len(blob) != bundle.DIM * 4
            or vector != np.frombuffer(blob, "<f4").astype("<f2").tobytes()):
        reasons.add("embedding")
    return reasons


def _unique_keys(pairs):
    names = [name for name, _ in pairs]
    if len(set(names)) != len(names):
        raise ValueError("duplicate key")
    return dict(pairs)


def read_source(library):
    """Capture only complete bundles and prove every item's library provenance."""
    folder = common.data_root(common.PRIVATE_BASE / "bundles")
    complete = []
    if folder.exists():
        for path in sorted(folder.iterdir()):
            if not bundle.ID_RE.fullmatch(path.name):
                continue
            common.check_input(path, kind="bundle")
            manifest = bundle._complete(path)
            if manifest is None:
                print(f"skipped {path.name} incomplete")
            else:
                complete.append((path, manifest))
    if len(complete) > 100:
        raise common.Refused("more than 100 complete bundles")
    desired, refused, images = {}, [], 0
    for path, manifest in complete:
        manifest_bytes = common.check_input(path / "manifest.json").read_bytes()
        # These bytes are published as they are, so they may hold only what is
        # checked: an ordinary parse ignores the earlier value of a duplicate key.
        try:
            captured = json.loads(manifest_bytes.decode("utf-8"), object_pairs_hook=_unique_keys)
        except ValueError:
            raise common.Refused(f"refused {path.name}: manifest has duplicate keys or is not UTF-8 JSON") from None
        if captured != manifest:
            raise common.Refused("bundle changed during validation")
        desired[path.name + "/manifest.json"] = manifest_bytes
        vectors = base64.b64decode(manifest["embeddings"]["data"], validate=True)
        failed, reasons = 0, set()
        for index, item in enumerate(manifest["items"]):
            name = item["thumbSha256"] + ".jpg"
            data = common.check_input(path / name).read_bytes()
            vector = vectors[index * bundle.DIM * 2:(index + 1) * bundle.DIM * 2]
            failures = _item_failures(library, manifest["model"]["id"], item, data, vector)
            if failures:
                failed += 1
                reasons.update(failures)
            desired[path.name + "/" + name] = data
        if failed:
            refused.append(f"refused {path.name}: {failed} items fail the class A check ({', '.join(sorted(reasons))})")
        images += len(manifest["items"])
    if refused:
        raise common.Refused("\n".join(refused))
    ids = [path.name for path, _ in complete]
    desired["index.json"] = (json.dumps({"format": "tastelab-bundle-index", "version": 1, "bundles": ids}, indent=2)
                             + "\n").encode("utf-8")
    return desired, dict(bundles=len(ids), images=images, bytes=sum(map(len, desired.values())))


def _lstat(path):
    try:
        info = os.lstat(path)
    except FileNotFoundError:
        return None
    if stat.S_ISLNK(info.st_mode) or path.is_junction():
        raise common.Refused("target contains a symbolic link or junction")
    return info


def _target_tree(target):
    managed = target / "bundles"
    files, directories = set(), set()
    for path in (managed, *managed.parents):
        _lstat(path)

    def visit(directory):
        info = _lstat(directory)
        if info is None:
            return
        if not stat.S_ISDIR(info.st_mode):
            raise common.Refused("expected a managed directory")
        if directory != managed:
            directories.add(directory.relative_to(managed).as_posix())
        for path in sorted(directory.iterdir()):
            info = _lstat(path)
            if stat.S_ISDIR(info.st_mode):
                visit(path)
            elif stat.S_ISREG(info.st_mode):
                files.add(path.relative_to(managed).as_posix())
            else:
                raise common.Refused("expected a managed file")

    visit(managed)
    return files, directories


def compute_plan(desired, target):
    """Compare captured bytes and identify stale files and directories."""
    files, directories = _target_tree(target)
    managed = target / "bundles"
    writes = {name for name, data in desired.items() if name not in files or (managed / name).read_bytes() != data}
    removals = files - desired.keys()
    needed = {parent.as_posix() for name in desired for parent in Path(name).parents if parent != Path(".")}
    prunes = directories - needed
    ids = {name.split("/")[0] for name in desired if "/" in name}
    old_ids = {name for name in directories if bundle.ID_RE.fullmatch(name)}
    changed = {name.split("/")[0] for name in writes | removals}
    lines = []
    for ident in sorted(ids | old_ids):
        if ident not in ids:
            lines.append("removed " + ident)
        elif ident not in old_ids:
            lines.append("added " + ident)
        elif ident in changed:
            lines.append("changed " + ident)
    lines.extend("removed file " + name for name in sorted(removals) if name.split("/")[0] not in ids | old_ids)
    if "index.json" in writes:
        lines.append("index changed")
    return Plan(tuple(sorted(writes, key=lambda name: (name == "index.json", name))), tuple(sorted(removals)),
                tuple(sorted(prunes, key=lambda name: (-name.count("/"), name))), tuple(lines))


def apply_plan(desired, target, plan):
    """Remove stale paths, then replace bundle files atomically and the index last."""
    managed = target / "bundles"
    for name in plan.removals:
        (managed / name).unlink()
    for name in plan.directories:
        (managed / name).rmdir()
    for name in plan.writes:
        path = managed / name
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(dir=path.parent, prefix=".publish-", delete=False) as stream:
                temporary = Path(stream.name)
                stream.write(desired[name])
            os.replace(temporary, path)
        finally:
            if temporary is not None:
                temporary.unlink(missing_ok=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--target", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    try:
        target = Path(os.path.abspath(args.target.expanduser()))
        for path in (target, *target.parents):
            _lstat(path)
        target = sync_public.validate_target(target, ROOT)
        _target_tree(target)
        root = common.data_root()
        if not (root / store.DB_NAME).is_file():
            raise common.Refused("bundle publication needs an existing library")
        with store.Store(root) as library:
            desired, counts = read_source(library)
        plan = compute_plan(desired, target)
        if not args.check:
            apply_plan(desired, target, plan)
        for line in plan.lines:
            print(line)
        print(f"bundles {counts['bundles']} images {counts['images']} bytes {counts['bytes']}")
        return int(args.check and bool(plan.writes or plan.removals or plan.directories))
    except (common.Refused, sync_public.Refused) as error:
        message = str(error)
        print(message if message.startswith("refused ") else "refused: " + message, file=sys.stderr)
        return 2
    except (OSError, ValueError, sqlite3.Error):
        print("refused: bundle files or library could not be read or updated", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
