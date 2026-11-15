"""Explicit control artifact preparation. Never called by the acceptance check."""

import json
import argparse
import shutil
import sys

sys.dont_write_bytecode = True

from harness.bindings import render_controls
from harness.common import ROOT, digest, load, sha, write_json
from harness.matrix import run
from harness.policy import GRANTS


def prepare():
    policy = load("policy.json")
    data, source_map = render_controls()
    for name in ("control-open", "control-closed"):
        folder = ROOT / "candidates" / name
        manifest = {"version": 1, "id": name, "adapter": "adapter.py",
                    "cells": [{"form": form, "variant": variant["id"], "status": "supported", "reason": None}
                              for form in policy["forms"] for variant in policy["variants"]]}
        write_json(folder / "candidate.json", manifest)
        (folder / "programs.json").write_bytes(data)
        files = sorted((ROOT / "harness").glob("*.py")) + [ROOT / "run_matrix.py", folder / "adapter.py",
                                                           folder / "candidate.json", folder / "programs.json"]
        sources = {path.relative_to(ROOT).as_posix(): sha(path.read_bytes()) for path in files}
        rendering = (folder / "programs.json").relative_to(ROOT).as_posix()
        mapping = {case_id: {path: {"file": rendering, **item} for path, item in entries.items()}
                   for case_id, entries in source_map.items()}
        bindings = {"version": 1, "candidate_id": name, "source_files": sources,
                    "edition": "neutral-control-v1; not a language edition",
                    "execution_contract": {"notation": sha((ROOT.parent / "corpus/notation.md").read_bytes()),
                                           "profile": sha((ROOT.parent / "corpus/profile.json").read_bytes())},
                    "charge_mapping": {"instruction": "one pre-step", "constituents": sources["harness/calc.py"],
                                       "executor": sources["harness/reference.py"], "meter": sources["harness/meter.py"]},
                    "kernel_implementation": {"harness/kernel.py": sources["harness/kernel.py"]},
                    "checkers": {p: sources[p] for p in ("harness/policy.py", "harness/classify.py", "harness/result.py")},
                    "effective_grants": {profile: list(GRANTS) for profile in policy["profiles"]},
                    "provenance_contract": {"host": sources["harness/host.py"], "version": 1},
                    "consumed_inputs": "host ledger: workspace value/block/parts, pick, previous exact output; included in per-run binding digest",
                    "source_map": mapping, "rendered_file": rendering}
        write_json(folder / "bindings.json", bindings)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seal", action="store_true", help="pin current artifacts without running or rewriting controls")
    args = parser.parse_args()
    if args.seal:
        if (ROOT / "build").exists():
            raise ValueError("remove generated build products before sealing")
        files = {path.relative_to(ROOT).as_posix(): sha(path.read_bytes()) for path in sorted(ROOT.rglob("*"))
                 if path.is_file() and path.name != "MANIFEST.json"
                 and (not path.relative_to(ROOT).as_posix().startswith("candidates/")
                      or path.parent.name in ("control-open", "control-closed"))}
        write_json(ROOT / "MANIFEST.json", {"version": 1, "decision": "open", "files": files,
                                           "content_sha256": digest(files), "rule": "raw SHA-256 of every file except this manifest; check.py additionally snapshots every file, including this one"})
        return 0
    prepare()
    build = (ROOT / "build").resolve()
    if build.exists():
        raise ValueError("preparation requires an absent build folder")
    assert build.parent == ROOT and build.name == "build"
    try:
        for name in ("control-open", "control-closed"):
            summary = run(ROOT / "candidates" / name, build / name)
            write_json(ROOT / "summaries" / (name + ".json"), summary)
            print(name, summary["row_count"], "observations", flush=True)
    finally:
        if build.exists():
            shutil.rmtree(build)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
