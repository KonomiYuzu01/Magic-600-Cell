"""Explicit artifact production. Acceptance never invokes this script."""

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

from harness.common import cases, digest, load, sha, write_json
from harness.policy import GRANTS
from render import render


def tool_identity():
    node = shutil.which("node")
    version = subprocess.run([node, "-p", "JSON.stringify(process.versions)"], capture_output=True, check=True, timeout=10)
    return {"python": platform.python_version(), "python_executable_sha256": sha(Path(sys.executable).read_bytes()),
            "node": json.loads(version.stdout), "node_executable_sha256": sha(Path(node).read_bytes()),
            "dependencies": "Python standard library and Node built-ins only; no installed package",
            "local_node_documentation": "not known here", "local_node_licence": "not known here"}


def prepare():
    policy = load("policy.json")
    cells = []
    for form in policy["forms"]:
        for variant in policy["variants"]:
            reason = None
            if form == "static_runtime_labels":
                reason = "RTE.12/RTE.24: borrowed VM is unmodified; no native run-time labels"
            elif variant["id"] == "source_bound":
                reason = "RT2 requires the approved_subset_byte_bound variant; source_bound not implemented"
            cells.append({"form": form, "variant": variant["id"], "status": "not_run" if reason else "supported", "reason": reason})
    write_json(FOLDER / "candidate.json", {"version": 1, "id": "rt2-node", "adapter": "adapter.py", "cells": cells})
    data, index, mapping = bytearray(), {}, {}
    artifact = "candidates/rt2-node/programs.js"
    for _, case in cases():
        program, spans = render(case["program"])
        offset = len(data)
        data.extend(program)
        index[case["id"]] = {"start": offset, "end": len(data), "sha256": sha(program)}
        mapping[case["id"]] = {path: {"file": artifact, "start": offset + span["start"],
                                      "end": offset + span["end"], "sha256": sha(program[span["start"]:span["end"]])}
                                   for path, span in spans.items()}
    (FOLDER / "programs.js").write_bytes(bytes(data))
    write_json(FOLDER / "program-index.json", index)
    write_json(FOLDER / "tools.json", tool_identity())
    from local_checks import checks
    write_json(FOLDER / "adversarial.json", checks())
    # Execution identity covers the actual runtime closure. Acceptance and
    # production helpers are pinned by MANIFEST.json, outside the run key.
    local = [FOLDER / name for name in ("adapter.py", "checker.py", "render.py", "runtime.cjs")]
    files = sorted((ROOT / "harness").glob("*.py")) + [ROOT / "run_matrix.py"] + local
    files += [FOLDER / name for name in ("candidate.json", "programs.js", "program-index.json", "tools.json")]
    sources = {p.relative_to(ROOT).as_posix(): sha(p.read_bytes()) for p in files}
    bindings = {"version": 1, "candidate_id": "rt2-node", "source_files": sources,
        "edition": "rt2-node-generated-js-v1; experiment-only canonical subset, not a language edition",
        "execution_contract": {"notation": sha((ROOT.parent / "corpus/notation.md").read_bytes()),
            "profile": sha((ROOT.parent / "corpus/profile.json").read_bytes()),
            "corpus_freeze": sha((ROOT.parent / "corpus/FREEZE.json").read_bytes()),
            "tools": sources["candidates/rt2-node/tools.json"], "values": "opaque handles; exact Python integers; JS controls branching, loops and module calls",
            "timeout": "vm.Script 2000 ms; Python bridge 6 s between messages; matrix watchdog 10 s"},
        "charge_mapping": {"instruction": "one continuation before each executed instruction, including branch/call/return",
            "loops": "one continuation per test including final false; item full canonical memory; body charged normally",
            "constituents": sources["harness/calc.py"], "executor": sources["candidates/rt2-node/adapter.py"],
            "meter": sources["harness/meter.py"], "kernel": "ctx.kernel_call precharges; batch growth retained once",
            "memory_output": "canonical host values; frame 16 logical bytes; precharge every bind, output and effect"},
        "kernel_implementation": {"harness/kernel.py": sources["harness/kernel.py"]},
        "checkers": {p: sources[p] for p in ("candidates/rt2-node/checker.py", "harness/policy.py", "harness/classify.py", "harness/result.py")},
        "effective_grants": {profile: list(GRANTS) for profile in policy["profiles"]},
        "provenance_contract": {"host": sources["harness/host.py"], "version": 1,
            "labels": "static may-join of both arms and loop fixed points; workspace retained across runs; recursive cycle TOP; no stop-sensitive repair",
            "separation": "transitive preview presence fixes grants before admission; read-state persists through stored values"},
        "consumed_inputs": "host ledger: workspace value/block/parts, pick record, previous exact output; included in per-run binding digest",
        "source_map": mapping, "rendered_file": artifact}
    write_json(FOLDER / "bindings.json", bindings)


def clean_build(build):
    assert build.resolve().parent == FOLDER and build.name == "build"
    if build.exists():
        shutil.rmtree(build)


def collect_outcomes(folder, manifest):
    """Keep every invocation's outcome even if a crash has no exact payload."""
    from harness.report import expected_rows, row_identity
    meta = json.loads((folder / "run.json").read_bytes())
    outcomes, seen = {}, set()
    with (folder / "results.jsonl").open("rb") as stream:
        for line in stream:
            row = json.loads(line)
            identity = row_identity(row)
            assert identity not in seen, "duplicate outcome"
            seen.add(identity)
            key = "/".join(row[k] for k in ("form", "variant", "case_id", "run_id", "host_variant"))
            record = row["record"]
            value = {"classification": row["classification"],
                     "run_key": record["exact"]["input_identities"]["run_key"] if record else None,
                     "sha256": record["sha256"] if record else None}
            if key in outcomes:
                assert outcomes[key] == value, "outcome or exact payload changed across repetitions: " + key
            outcomes[key] = value
    assert seen == expected_rows(manifest, meta["runs"], meta["legs"]), "missing outcome"
    return {"decision": "open", "row_count": len(seen), "runs": meta["runs"], "keys": outcomes}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--series", type=int)
    parser.add_argument("--seal", action="store_true")
    args = parser.parse_args()
    build = FOLDER / "build"
    assert not build.exists(), "build must be absent before preparation"
    if args.seal:
        files = {p.relative_to(FOLDER).as_posix(): sha(p.read_bytes()) for p in sorted(FOLDER.rglob("*"))
                 if p.is_file() and p.name != "MANIFEST.json"}
        write_json(FOLDER / "MANIFEST.json", {"decision": "open", "files": files, "content_sha256": digest(files)})
    elif args.series:
        try:
            command = [sys.executable, "-B", str(ROOT / "run_matrix.py"), "--candidate", "candidates/rt2-node",
                       "--out", "candidates/rt2-node/build/Windows", "--runs", str(args.series), "--leg", "Windows"]
            subprocess.run(command, check=True, timeout=1800)
            summary = (build / "Windows/summary.json").read_bytes()
            (FOLDER / "summary.json").write_bytes(summary)
            manifest = json.loads((FOLDER / "candidate.json").read_bytes())
            write_json(FOLDER / "key-outcomes.json", collect_outcomes(build / "Windows", manifest))
            write_json(FOLDER / "series-evidence.json", {"decision": "open", "runs": args.series,
                "raw_sha256": sha((build / "Windows/results.jsonl").read_bytes()),
                "run_manifest_sha256": sha((build / "Windows/run.json").read_bytes()),
                "summary_sha256": sha(summary), "raw_retained": False})
        finally:
            clean_build(build)
    else:
        prepare()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
