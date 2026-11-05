"""Explicit artifact production; never refresh evidence during acceptance."""

import argparse
import json
from pathlib import Path
import platform
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
FOLDER = Path(__file__).resolve().parent
ROOT = FOLDER.parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(FOLDER))

from harness.common import canonical, cases, digest, load, sha, write_json
from harness.policy import GRANTS
from render import render


def artifacts():
    bundle, index, mapping = bytearray(), {}, {}
    for _, case in cases():
        source, spans = render(case["program"])
        offset = len(bundle)
        bundle.extend(source)
        index[case["id"]] = {"start": offset, "end": len(bundle), "sha256": sha(source)}
        mapping[case["id"]] = {
            path: {"file": "candidates/rt4-pyhost/programs.py", "start": span["start"] + offset,
                   "end": span["end"] + offset, "sha256": span["sha256"]}
            for path, span in spans.items()}
    return bytes(bundle), index, mapping


def prepare():
    policy = load("policy.json")
    manifest = {"version": 1, "id": "rt4-pyhost", "adapter": "adapter.py", "cells": []}
    for form in policy["forms"]:
        for variant in policy["variants"]:
            supported = variant["id"] == "approved_subset_byte_bound"
            manifest["cells"].append({"form": form, "variant": variant["id"],
                                      "status": "supported" if supported else "not_run",
                                      "reason": None if supported else "RT4 attempt requires approved host subset and executed-byte binding; source_bound is not implemented"})
    write_json(FOLDER / "candidate.json", manifest)
    bundle, index, mapping = artifacts()
    (FOLDER / "programs.py").write_bytes(bundle)
    write_json(FOLDER / "program-index.json", index)
    files = list((ROOT / "harness").glob("*.py")) + [ROOT / "run_matrix.py"]
    files += [FOLDER / name for name in ("candidate.json", "adapter.py", "render.py", "subset.py", "library.py",
                                        "programs.py", "program-index.json", "prepare.py", "check.py")]
    sources = {path.relative_to(ROOT).as_posix(): sha(path.read_bytes()) for path in sorted(files)}
    own = "candidates/rt4-pyhost/"
    bindings = {
        "version": 1, "candidate_id": "rt4-pyhost", "source_files": sources,
        "edition": "rt4-pyhost-v1; Python approved subset, experiment only; no LQ8 host selection",
        "execution_contract": {
            "notation": sha((ROOT.parent / "corpus/notation.md").read_bytes()),
            "profile": sha((ROOT.parent / "corpus/profile.json").read_bytes()),
            "host": "CPython native if/for/functions/returns; domain operations only through Library",
            "approval": "compare complete rendered executed bytes including every module with pinned program slice before compile; no dynamic execution",
            "static_labels": "Python AST abstract labels, both branches, loop joins, module flow; symbolic persisted inputs instantiated at host boundary; no stop-sensitive repair",
            "runtime_attempt": "structural subset approval plus value wrappers only; no static flow analysis or native branch instrumentation; deliberately incomplete form",
            "capabilities": "library holds exclusive read-or-preview grant in solving, using transitive AST call closure; stored read_state survives"},
        "charge_mapping": {
            "instruction": "lib.step before each neutral instruction, with executed fragment bytes",
            "loop": "lib.iterate precharges each test, including final false, and each item binding",
            "module": "native Python function through lib.call; parameters, result and recursive frames retain neutral charges",
            "constituents": sources["harness/calc.py"], "meter": sources["harness/meter.py"],
            "effects": "kernel precharge, then memory, then output; batch buffer charged only by shared runtime",
            "library": sources[own + "library.py"]},
        "kernel_implementation": {"harness/kernel.py": sources["harness/kernel.py"]},
        "checkers": {path: sources[path] for path in (own + "subset.py", own + "adapter.py", "harness/policy.py", "harness/classify.py", "harness/result.py")},
        "effective_grants": {profile: list(GRANTS) for profile in policy["profiles"]},
        "provenance_contract": {"host": sources["harness/host.py"], "version": 1,
                                "mechanism": "candidate explicit dependencies and static joins; wrapper attempt has no implicit branch dependencies"},
        "consumed_inputs": "host ledger: workspace value/block/parts, fixed picks and previous exact output; included in per-run binding digest",
        "source_map": mapping, "rendered_file": own + "programs.py"}
    write_json(FOLDER / "bindings.json", bindings)


def observations(path):
    """Small observer evidence, separate from the mechanism and exact hashes."""
    output = []
    with path.open("rb") as stream:
        for line in stream:
            row = json.loads(line)
            if row["repeat"] != 0 or row["variant"] != "approved_subset_byte_bound":
                continue
            record = row["record"]
            item = {key: row[key] for key in ("form", "case_id", "run_id", "host_variant", "classification")}
            if record is not None:
                observer, exact = record["observer"], record["exact"]
                def proposal(state):
                    pending = state["pending_preview"]
                    return {key: pending[key] for key in ("word", "target")} if pending else None
                item.update({"run_key": exact["input_identities"]["run_key"], "sha256": record["sha256"],
                             "outcome": exact["outcome"], "code": exact["code"], "stop": exact["stop"],
                             "used": exact["budgets"]["used"], "coverage": exact["coverage"],
                             "session_unchanged": observer["before"]["session"] == observer["after"]["session"],
                             "pending_before": proposal(observer["before"]), "pending_after": proposal(observer["after"]),
                             "traps": observer["trap_calls"], "runtime_loss_observed": observer["runtime_loss_observed"],
                             "executed_fragments": len(observer["approval"]["executions"]),
                             "unapproved_fragments": sum(not entry["inside_approval"] for entry in observer["approval"]["executions"]),
                             "dynamic_executions": observer["approval"]["dynamic_attempts"],
                             "cache_diagnostics": record["host"]["cache_diagnostics"],
                             "gates": [{key: denial[key] for key in ("operation", "code", "layer", "path")}
                                       for denial in observer["denials"]]})
            output.append(item)
    return output


def seal():
    if (FOLDER / "build").exists():
        raise ValueError("build must be absent before sealing")
    files = {path.relative_to(FOLDER).as_posix(): sha(path.read_bytes()) for path in sorted(FOLDER.rglob("*"))
             if path.is_file() and path.name != "MANIFEST.json"}
    write_json(FOLDER / "MANIFEST.json", {"version": 1, "decision": "open", "files": files,
                                        "content_sha256": digest(files)})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seal", action="store_true")
    parser.add_argument("--pins-only", action="store_true")
    parser.add_argument("--runs", type=int, default=10)
    args = parser.parse_args()
    if args.seal:
        seal()
        return 0
    build = (FOLDER / "build").resolve()
    if build.exists():
        raise ValueError("build must be absent before preparation")
    prepare()
    if args.pins_only:
        return 0
    try:
        command = [sys.executable, "-B", str(ROOT / "run_matrix.py"), "--candidate", "candidates/rt4-pyhost",
                   "--out", "candidates/rt4-pyhost/build/Windows", "--runs", str(args.runs), "--leg", "Windows"]
        subprocess.run(command, check=True)
        output = build / "Windows"
        (FOLDER / "summary.json").write_bytes((output / "summary.json").read_bytes())
        import hashlib
        with (output / "results.jsonl").open("rb") as stream:
            raw_digest = hashlib.file_digest(stream, "sha256").hexdigest()
        write_json(FOLDER / "evidence.json", {
            "decision": "open", "decider": "owner", "python": platform.python_version(), "leg": "Windows",
            "runs": args.runs, "bindings_sha256": sha((FOLDER / "bindings.json").read_bytes()),
            "raw_results_sha256": raw_digest, "raw_results_bytes": (output / "results.jsonl").stat().st_size,
            "observations": observations(output / "results.jsonl")})
    finally:
        if build.exists():
            assert build.resolve().parent == FOLDER and build.name == "build"
            shutil.rmtree(build)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
