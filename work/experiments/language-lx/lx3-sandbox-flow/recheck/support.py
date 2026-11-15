"""Pin verification and cleanup, restricted to the experiment's build folder."""

import json
from pathlib import Path
import shutil
import subprocess
import sys

from runner import CANDIDATES, HERE, PART
from harness.bindings import candidate
from harness.common import CORPUS, FREEZE_SHA256, digest, sha

REPO = HERE.parents[4]
BUILD = HERE / "build"


def read(path):
    return json.loads(path.read_bytes())


def snapshot(folder):
    return {p.relative_to(folder).as_posix(): sha(p.read_bytes()) for p in sorted(folder.rglob("*")) if p.is_file()}


def inputs():
    return {p.relative_to(REPO).as_posix(): sha(p.read_bytes())
            for folder in (PART, CORPUS) for p in sorted(folder.rglob("*")) if p.is_file()}


def file_pins(path):
    value = read(path)
    for name, expected in value["files"].items():
        assert sha((path.parent / name).read_bytes()) == expected, "stored pin mismatch: " + path.name + "/" + name
    if "content_sha256" in value:
        assert digest(value["files"]) == value["content_sha256"], "stored manifest content mismatch"


def verify_inputs():
    assert sha((CORPUS / "FREEZE.json").read_bytes()) == FREEZE_SHA256, "raw corpus pin changed"
    frozen = read(CORPUS / "FREEZE.json")
    assert set(frozen["files"]) == {p.name for p in CORPUS.iterdir() if p.is_file()}, "corpus inventory changed"
    for name, expected in frozen["files"].items():
        if name != "FREEZE.json":
            assert sha((CORPUS / name).read_bytes()) == expected, "corpus pin changed: " + name
    assert digest({k: v for k, v in frozen["files"].items() if k != "FREEZE.json"}) == frozen["content_sha256"]
    self_pin = frozen["files"]["FREEZE.json"]
    frozen["files"]["FREEZE.json"] = None
    assert digest(frozen) == self_pin, "normalized corpus self pin changed"
    # Check the frozen bench's listed pins, never apply its inventory predicate
    # to this sibling folder or add this folder to its model of part-b.
    file_pins(PART / "MANIFEST.json")
    for name in CANDIDATES:
        folder = PART / "candidates" / name
        _, bindings = candidate(folder)
        if (folder / "MANIFEST.json").exists():
            file_pins(folder / "MANIFEST.json")
        if name == "rt3-wasm":
            file_pins(folder / "evidence.json")
        if name == "rt1-own":
            assert read(folder / "series.json")["bindings_sha256"] == digest(bindings)
        elif name == "rt2-py":
            assert read(folder / "series.json")["run"]["bindings_sha256"] == digest(bindings)
        elif name == "rt4-pyhost":
            assert read(folder / "evidence.json")["bindings_sha256"] == sha((folder / "bindings.json").read_bytes())
    tools = read(PART / "candidates/rt2-node/tools.json")
    assert sys.version.split()[0] == tools["python"], "Python version pin changed"
    assert sha(Path(sys.executable).read_bytes()) == tools["python_executable_sha256"], "Python executable pin changed"
    node = shutil.which("node")
    assert node and sha(Path(node).read_bytes()) == tools["node_executable_sha256"], "Node executable pin changed"
    versions = json.loads(subprocess.check_output([node, "-p", "JSON.stringify(process.versions)"], timeout=10))
    assert versions == tools["node"], "Node dependency versions changed"
    return {"Python": sys.version.split()[0], "Node": "v" + versions["node"], "V8": versions["v8"]}


def clean_build():
    target = BUILD.resolve()
    assert target.parent == HERE.resolve() and target.name == "build", "unsafe cleanup target"
    if target.exists():
        shutil.rmtree(target)
