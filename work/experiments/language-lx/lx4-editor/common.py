"""Local-only helpers for the LX-4 experiment. No product API."""
import sys
sys.dont_write_bytecode = True

from contextlib import contextmanager
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent
LX8 = ROOT.parent / "lx8-surfaces"
BUILD = ROOT / "build"
sys.path.insert(0, str(LX8))


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PARSERS = {s: load("lx4_" + s, LX8 / ("candidate-" + s) / "parser.py").parse
           for s in "abc"}
from lower import lower
from semantics import LoweringError, semantic


def sha(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def put(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data if isinstance(data, bytes) else encoded(data))


def snapshot():
    return {p.relative_to(ROOT).as_posix(): sha(p.read_bytes())
            for p in ROOT.rglob("*") if p.is_file()}


@contextmanager
def scratch():
    if BUILD.exists():
        raise ValueError("build already exists; inspect and remove the experiment build before running")
    before = snapshot()
    BUILD.mkdir()
    try:
        yield BUILD
    finally:
        # Fixed, resolved child only; never delete a caller-supplied path.
        if BUILD.is_symlink() or BUILD.resolve().parent != ROOT:
            raise ValueError("build escaped the experiment")
        shutil.rmtree(BUILD)
        if snapshot() != before:
            raise ValueError("experiment folder changed during read-only run")


def compile_rust():
    tmp = BUILD / "tmp"
    tmp.mkdir()
    env = dict(os.environ, TMP=str(tmp), TEMP=str(tmp))
    exe = BUILD / ("frontend.exe" if os.name == "nt" else "frontend")
    command = ["rustc", "--edition=2021", "-C", "opt-level=3", "-C", "debuginfo=0",
               "frontend.rs", "-o", str(exe.relative_to(ROOT))]
    result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, timeout=180)
    if result.returncode:
        # Compiler output is transient, never a committed machine diagnostic.
        sys.stderr.buffer.write(result.stderr)
        raise ValueError("Rust compilation failed")
    return exe


def python_front(data, interfaces, surface="a"):
    parsed = PARSERS[surface](data)
    try:
        ir, error = lower(parsed, interfaces), None
    except LoweringError as problem:
        ir, error = None, str(problem)
    return {"parsed": parsed, "ir": ir, "error": error}


def rust_run(exe, mode, jobs, interfaces):
    interface_paths = {}
    for digest, data in interfaces.items():
        assert sha(data) == digest
        path = BUILD / "interfaces" / (digest + ".json")
        put(path, data)
        interface_paths[digest] = path.relative_to(ROOT).as_posix()
    request = BUILD / "request.json"
    response = BUILD / "response.json"
    put(request, {"jobs": jobs, "interfaces": interface_paths})
    subprocess.run([str(exe), mode, "build/request.json", "build/response.json"],
                   cwd=ROOT, check=True, timeout=900)
    return json.loads(response.read_bytes())
