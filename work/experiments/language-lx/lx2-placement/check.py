"""Replay stored correctness, never compare timing, and leave every byte intact."""
from __future__ import annotations

import json
import sys

sys.dont_write_bytecode = True
from prepare import BUILD, HERE, ROOT, canonical, cleanup, prepare, sha
from run import collect


def snapshot():
    return {p.relative_to(HERE).as_posix(): sha(p.read_bytes())
            for p in HERE.rglob("*") if p.is_file()}


def verify_manifest():
    manifest = json.loads((HERE / "MANIFEST.json").read_bytes())
    actual = snapshot()
    del actual["MANIFEST.json"]
    if actual != manifest["files"]:
        raise ValueError("Experiment file set or frozen digest differs")
    for name, digest in manifest["reused"].items():
        if sha((ROOT / name).read_bytes()) != digest:
            raise ValueError("Reused experiment source differs: " + name)


def main():
    if BUILD.exists():
        raise ValueError("build/ must be absent before acceptance")
    before = snapshot()
    try:
        verify_manifest()
        candidate = prepare()
        expected_candidate = json.loads((HERE / "CANDIDATE.json").read_bytes())
        if candidate != expected_candidate:
            raise ValueError("Candidate source, compiler or executable identity differs")
        observed = collect(candidate)
        expected = json.loads((HERE / "RESULTS.json").read_bytes())
        if canonical(observed["correctness"]) != canonical(expected):
            raise ValueError("A correctness field differs from RESULTS.json")
    finally:
        cleanup()
        if snapshot() != before:
            raise ValueError("Acceptance changed experiment bytes")
    verify_manifest()
    print("PASS: all correctness fields equal; placement failures preserved; folder byte-identical")


if __name__ == "__main__":
    main()
