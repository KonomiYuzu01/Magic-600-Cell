"""Explicit artifact production, never invoked by acceptance. Experiment only."""

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT))

from harness.common import cases, digest, load, sha, write_json
from harness.policy import GRANTS
from probes import observations
from render import render


def prepare():
    policy = load("policy.json")
    cells = []
    for form in policy["forms"]:
        for variant in policy["variants"]:
            supported = variant["id"] == "approved_subset_byte_bound" and form != "static_runtime_labels"
            reason = None if supported else ("RT2 requires approved_subset_byte_bound; source_bound not implemented"
                      if variant["id"] == "source_bound" else "Run-time labels require a modified VM (RTE.12/RTE.24); CPython is unmodified")
            cells.append({"form": form, "variant": variant["id"],
                          "status": "supported" if supported else "not_run", "reason": reason})
    write_json(HERE / "candidate.json", {"version": 1, "id": "rt2-py", "adapter": "adapter.py", "cells": cells})
    index, mapping, generated = {}, {}, set()
    for _, case in cases():
        data, spans = render(case["program"])
        file = HERE / ("program-" + sha(data)[:24] + ".py")
        file.write_bytes(data)
        generated.add(file.name)
        relative = file.relative_to(ROOT).as_posix()
        index[digest(case["program"])] = relative
        mapping[case["id"]] = {path: {"file": relative, **span} for path, span in spans.items()}
    for path in HERE.glob("program-*.py"):
        if path.name not in generated:
            path.unlink()
    write_json(HERE / "programs.json", index)
    write_json(HERE / "probes.json", observations())
    files = (list((ROOT / "harness").glob("*.py")) + [ROOT / "run_matrix.py"]
             + list(HERE.glob("*.py")) + [HERE / "candidate.json", HERE / "programs.json"])
    sources = {p.relative_to(ROOT).as_posix(): sha(p.read_bytes()) for p in sorted(files)}
    def local(name):
        return (HERE / name).relative_to(ROOT).as_posix()
    checks = [local("subset.py"), local("adapter.py"), "harness/policy.py", "harness/classify.py", "harness/result.py"]
    bindings = {
        "version": 1, "candidate_id": "rt2-py", "source_files": sources,
        "edition": "rt2-approved-python-v1; native CPython control flow, empty builtins; Windows observation CPython 3.15.1",
        "execution_contract": {"notation": sha((ROOT.parent / "corpus/notation.md").read_bytes()),
                               "profile": sha((ROOT.parent / "corpus/profile.json").read_bytes()),
                               "approval": "render actual neutral stimulus; byte-compare complete program and modules; check and compile same AST",
                               "vm": "unmodified host CPython and its standard library; not an OS sandbox or an instrumented second interpreter"},
        "charge_mapping": {"instructions": "bridge helper pre-step before policy and effects; for tests include terminal test; native module calls charge parameters and recursive frames",
                           "constituents": sources["harness/calc.py"], "meter": sources["harness/meter.py"],
                           "executor": sources[local("bridge.py")],
                           "memory": "full canonical binding sizes, aliases and loop items; 32-byte snapshots; batch retained growth only once",
                           "output": "canonical exact output precharged before each sink; display approximation stays separate"},
        "kernel_implementation": {"harness/kernel.py": sources["harness/kernel.py"]},
        "checkers": {path: sources[path] for path in checks},
        "effective_grants": {profile: list(GRANTS) for profile in policy["profiles"]},
        "provenance_contract": {"version": 1, "host": sources["harness/host.py"],
                                "knowledge": "AST joins branches, loop fixed points, module returns and sequential workspace parts; immutable host records; no VM run-time labels",
                                "separation": "AST call-graph selects preview-capable or read-capable solving run; read-capable removes preview; shared policy denies state rows in preview-capable runs"},
        "consumed_inputs": "host ledger: workspace value/block/parts, manual picks and previous exact output; expanded binding hashes per run",
        "source_map": mapping, "rendered_file": local("programs.json")}
    write_json(HERE / "bindings.json", bindings)


def seal():
    if (HERE / "build").exists():
        raise ValueError("build must be absent before sealing")
    files = {p.relative_to(HERE).as_posix(): sha(p.read_bytes()) for p in sorted(HERE.rglob("*"))
             if p.is_file() and p.name != "MANIFEST.json"}
    write_json(HERE / "MANIFEST.json", {"version": 1, "decision": "open", "decider": "owner",
                                       "files": files, "content_sha256": digest(files)})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seal", action="store_true")
    parser.add_argument("--render-only", action="store_true")
    parser.add_argument("--runs", type=int, default=10)
    args = parser.parse_args()
    if args.seal:
        seal()
        return 0
    prepare()
    if args.render_only:
        return 0
    build = HERE / "build"
    if build.exists():
        raise ValueError("production requires absent build folder")
    try:
        output = (build / "Windows").relative_to(ROOT).as_posix()
        subprocess.run([sys.executable, "-B", str(ROOT / "run_matrix.py"), "--candidate", "candidates/rt2-py",
                        "--out", output, "--runs", str(args.runs), "--leg", "Windows"], check=True, timeout=3600)
        (HERE / "summary.json").write_bytes((build / "Windows/summary.json").read_bytes())
        # Large raw observations are not retained, but their exact identity is.
        raw = build / "Windows/results.jsonl"
        with raw.open("rb") as stream:
            raw_digest = hashlib.file_digest(stream, "sha256").hexdigest()
        write_json(HERE / "series.json", {"decision": "open", "decider": "owner", "runs": args.runs,
            "leg": "Windows", "python": sys.version.split()[0], "evidence": "source and synthetic",
            "results_jsonl_sha256": raw_digest, "results_jsonl_bytes": raw.stat().st_size,
            "run": json.loads((build / "Windows/run.json").read_bytes())})
    finally:
        if build.exists():
            assert build.resolve().parent == HERE and build.name == "build"
            shutil.rmtree(build)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
