"""LX-1 input identity and read-only verification; no product dependencies."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
LX3 = HERE.parent / "lx3-sandbox-flow"
PART_B = LX3 / "part-b"
CORPUS = LX3 / "corpus"
BUILD = HERE / "build"
CANDIDATES = ("rt1-own", "rt2-py", "rt2-node", "rt3-wasm", "rt4-pyhost")
MODES = ("labels_off", "static_only", "run_time")
FORMS = dict(zip(MODES, ("capability_separation", "static_labels", "static_runtime_labels")))
SOURCES = ("CARD.md", "support.py", "variants.py", "runner.py", "measure.py", "check.py")
FREEZE = "d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":"), allow_nan=False).encode("utf-8")


def sha(data):
    return hashlib.sha256(data).hexdigest()


def digest(value):
    return sha(canonical(value))


def read(path):
    return json.loads(path.read_bytes())


def write(path, value):
    require(path.resolve().is_relative_to(HERE), "output outside LX-1")
    path.write_bytes(canonical(value) + b"\n")


def selection():
    cases = read(CORPUS / "allowed-tools.json")["cases"]

    def rows(value):
        if isinstance(value, dict):
            if value.get("op") == "kernel":
                yield value["row"]
            for item in value.values():
                yield from rows(item)
        elif isinstance(value, list):
            for item in value:
                yield from rows(item)

    categories = {"reading_state", "exact_counting", "own_word_checking", "named_target_setup"}
    selected = {"full": [], "jumbling": [], "representations": []}
    for case in sorted(cases, key=lambda c: c["id"]):
        if case["id"] in ("A25", "A30"):
            continue
        names = []
        if case["category"] in categories:
            names.append("full")
        if {"nu2", "nu5", "bits"} & set(rows(case["program"])):
            names.append("jumbling")
        if case["category"] in ("charts", "explaining"):
            names.append("representations")
        for name in names:
            selected[name].append({"case": case["id"], "program_sha256": digest(case["program"]),
                                   "inputs_sha256": digest(case["program"]["inputs"])})
    return {"version": 1, "decision": "open", "decider": "owner", "workloads": selected}


def versions():
    node = json.loads(subprocess.check_output(["node", "-p", "JSON.stringify(process.versions)"], cwd=HERE))
    return {"python": sys.version.split()[0], "node": node["node"], "v8": node["v8"]}


def freeze_inputs():
    write(HERE / "workloads.json", selection())
    paths = list(CORPUS.glob("*")) + list((PART_B / "harness").glob("*.py")) + [PART_B / "run_matrix.py"]
    for candidate in CANDIDATES:
        paths += list((PART_B / "candidates" / candidate).glob("*"))
    external = {p.relative_to(REPO).as_posix(): sha(p.read_bytes())
                for p in paths if p.is_file()}
    write(HERE / "INPUTS.json", {
        "version": 1, "decision": "open", "decider": "owner", "versions": versions(),
        "corpus_freeze_sha256": FREEZE, "external_files": dict(sorted(external.items())),
        "local_files": {n: sha((HERE / n).read_bytes()) for n in (*SOURCES, "workloads.json")}})


def verify_pins():
    manifest = read(HERE / "INPUTS.json")
    require(set(manifest["local_files"]) == set((*SOURCES, "workloads.json")), "local pin set changed")
    for name, expected in manifest["local_files"].items():
        require(sha((HERE / name).read_bytes()) == expected, "local pin changed: " + name)
    for name, expected in manifest["external_files"].items():
        path = (REPO / name).resolve()
        require(path.is_relative_to(REPO / "work/experiments"), "pin outside experiment sources")
        require(sha(path.read_bytes()) == expected, "external pin changed: " + name)
    raw = (CORPUS / "FREEZE.json").read_bytes()
    require(sha(raw) == FREEZE == manifest["corpus_freeze_sha256"], "corpus freeze changed")
    frozen = json.loads(raw)
    require(set(frozen["files"]) == {p.name for p in CORPUS.iterdir() if p.is_file()}, "corpus file set changed")
    for name, expected in frozen["files"].items():
        if name != "FREEZE.json":
            require(sha((CORPUS / name).read_bytes()) == expected, "corpus pin changed: " + name)
    normalized = json.loads(raw)
    normalized["files"]["FREEZE.json"] = None
    require(digest(normalized) == frozen["files"]["FREEZE.json"], "corpus self pin changed")
    require(digest({n: h for n, h in frozen["files"].items() if n != "FREEZE.json"}) == frozen["content_sha256"],
            "corpus content identity changed")
    require(selection() == read(HERE / "workloads.json"), "workload selection changed")
    require(versions() == manifest["versions"], "tool versions changed")
    sys.path.insert(0, str(PART_B))
    from harness.bindings import candidate as verify_candidate
    for name in CANDIDATES:
        folder = PART_B / "candidates" / name
        verify_candidate(folder)  # Includes every source map and executed dependency.
        stored = read(folder / ("evidence.json" if name == "rt3-wasm" else "MANIFEST.json"))
        for item, expected in stored["files"].items():
            require(sha((folder / item).read_bytes()) == expected, name + " stored pin changed: " + item)
        if "content_sha256" in stored:
            require(digest(stored["files"]) == stored["content_sha256"], name + " manifest content pin changed")
    tools = read(PART_B / "candidates/rt2-node/tools.json")
    require(sha(Path(sys.executable).read_bytes()) == tools["python_executable_sha256"], "stored Python executable pin changed")
    require(sha(Path(shutil.which("node")).read_bytes()) == tools["node_executable_sha256"], "stored Node executable pin changed")
    return manifest


def snapshot():
    return {p.relative_to(HERE).as_posix(): sha(p.read_bytes()) for p in HERE.rglob("*")
            if p.is_file() and not p.is_relative_to(BUILD)}


def clean_build():
    require(BUILD.resolve() == HERE.resolve() / "build" and not BUILD.is_symlink(), "unsafe build cleanup")
    if BUILD.exists():
        shutil.rmtree(BUILD)


def worker(candidate, mode, action):
    output = BUILD / (candidate + "-" + mode + "-" + action + ".json")
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", PYTHONHASHSEED="0",
               TMP=str(BUILD / "tmp"), TEMP=str(BUILD / "tmp"))
    (BUILD / "tmp").mkdir(exist_ok=True)
    with output.open("wb") as stream:
        subprocess.run([sys.executable, "-B", str(HERE / "runner.py"), candidate, mode, action],
                       cwd=HERE, env=env, stdout=stream, check=True,
                       timeout=840 if action == "counts" else 3600)
    return read(output)


def compare_modes(cells):
    for candidate in CANDIDATES:
        for workload in read(HERE / "workloads.json")["workloads"]:
            group = [c for c in cells if c["candidate"] == candidate and c["workload"] == workload
                     and c["status"] != "not_run"]
            require(len({c["result_payload_sha256"] for c in group}) == 1,
                    "result differs across modes: " + candidate + "/" + workload)


def count_all():
    cells = []
    for candidate in CANDIDATES:
        for mode in MODES:
            cells.extend(worker(candidate, mode, "counts"))
    require(len(cells) == 45, "missing workload/prototype/mode cell")
    compare_modes(cells)
    return {"version": 1, "decision": "open", "decider": "owner", "cells": cells}
