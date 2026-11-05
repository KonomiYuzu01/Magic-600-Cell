"""Export frozen model arrays without NumPy; no mechanics are computed here."""
from __future__ import annotations

import ast
import gzip
import hashlib
import json
import struct
import sys
import zipfile
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
BUILD = HERE / "build"
INPUTS = ROOT / "work/experiments/language-lx/inputs"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def write_json(path, value):
    path.write_bytes((json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n").encode())


def verify_inputs():
    frozen = json.loads((HERE / "INPUTS.json").read_bytes())
    for name, expected in frozen.items():
        if sha((ROOT / name).read_bytes()) != expected:
            raise ValueError("Frozen input digest mismatch: " + name)
    manifest = json.loads((INPUTS / "MANIFEST-oracle.json").read_bytes())
    if set(manifest["files"]) != {f"O{i:02}.json.gz" for i in range(1, 8)}:
        raise ValueError("Frozen case set differs")
    for name, row in manifest["files"].items():
        data = (INPUTS / "oracle" / name).read_bytes()
        if sha(data) != row["sha256"] or len(data) != row["bytes"]:
            raise ValueError("Frozen case digest mismatch: " + name)
    assets = json.loads((ROOT / "assets/manifest.json").read_bytes())
    for name, digest in assets["files"].items():
        if sha((ROOT / name).read_bytes()) != digest:
            raise ValueError("Model asset digest mismatch: " + name)
    return manifest


def npy_i32(archive, name):
    """The pinned one-dimensional little-endian signed integer NPY subset."""
    with archive.open(name + ".npy") as stream:
        magic = stream.read(8)
        if magic[:6] != b"\x93NUMPY" or magic[6:] not in (b"\x01\x00", b"\x02\x00"):
            raise ValueError("Unsupported NPY version")
        size = int.from_bytes(stream.read(2 if magic[6] == 1 else 4), "little")
        header = ast.literal_eval(stream.read(size).decode("latin1").strip())
        if header["fortran_order"] or len(header["shape"]) != 1:
            raise ValueError("Expected a flat C-order array")
        data = stream.read()
    count = header["shape"][0]
    if header["descr"] == "<i4" and len(data) == count * 4:
        return data
    if header["descr"] == "<i2" and len(data) == count * 2:
        return b"".join(struct.pack("<i", value[0]) for value in struct.iter_unpack("<h", data))
    raise ValueError("Unexpected model dtype or byte length")


def export():
    verify_inputs()
    BUILD.mkdir(exist_ok=True)
    manifest_bytes = (ROOT / "assets/manifest.json").read_bytes()
    manifest = json.loads(manifest_bytes)
    census = json.loads((ROOT / "assets/census.json").read_bytes())
    trees = json.loads(gzip.decompress((ROOT / "assets/execution_trees.json.gz").read_bytes()))
    atlas = json.loads((ROOT / "assets/seed_atlas.json").read_bytes())
    if len(trees) != 35 or any(tree["seed"] != atlas[i]["word"] for i, tree in enumerate(trees)):
        raise ValueError("Tree/atlas seed definitions differ")
    meta = canonical(dict(model_id=manifest["model_id"], manifest_sha256=sha(manifest_bytes),
                          census=census["orbits"])).encode()
    target = BUILD / "model.bin"
    with target.open("wb") as out:
        out.write(b"LX6MOD1\0")
        out.write(struct.pack("<I", len(meta)))
        out.write(meta)

        def vector(data):
            out.write(struct.pack("<I", len(data) // 4))
            out.write(data)

        for archive_name, arrays in (("model.npz", ("slot_piece", "orbit_id")),
                                     ("primitives.npz", ("offsets", "src", "dst"))):
            with zipfile.ZipFile(ROOT / "assets" / archive_name) as archive:
                for name in arrays:
                    vector(npy_i32(archive, name))
        out.write(struct.pack("<I", len(trees)))
        for tree in trees:
            for key in ("seed", "relocation", "parent", "move"):
                vector(b"".join(struct.pack("<i", value) for value in tree[key]))
    return sha(target.read_bytes())


if __name__ == "__main__":
    print(export())
