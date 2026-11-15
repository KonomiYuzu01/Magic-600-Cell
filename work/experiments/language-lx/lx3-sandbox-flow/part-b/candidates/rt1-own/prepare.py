"""Explicit artifact production, never called by acceptance. Experiment only."""

import argparse
import json
from pathlib import Path
import shutil
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
PART = HERE.parents[1]
sys.path.append(str(PART))

from harness.common import CORPUS, ROOT, cases, digest, load, sha, write_json
from harness.matrix import run
from harness.policy import GRANTS
from render import render

PREFIX = HERE.relative_to(ROOT).as_posix()


def rendering():
    archive, index, mapping = bytearray(), {}, {}
    file = PREFIX + "/programs.rt1"
    for _, case in cases():
        source, spans = render(case["program"])
        start = len(archive)
        archive.extend(source)
        index[case["id"]] = {"file": file, "start": start, "end": len(archive),
                             "sha256": sha(source), "neutral_sha256": digest(case["program"])}
        mapping[case["id"]] = {path: {"file": file, "start": start + a, "end": start + b,
                                          "sha256": sha(source[a:b])} for path, (a, b) in spans.items()}
    return bytes(archive), index, mapping


def prepare():
    policy = load("policy.json")
    declaration = {"version": 1, "id": "rt1-own", "adapter": "adapter.py", "cells": [
        {"form": form, "variant": variant["id"],
         "status": "supported" if variant["id"] == "source_bound" else "not_run",
         "reason": None if variant["id"] == "source_bound" else "RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4"}
        for form in policy["forms"] for variant in policy["variants"]]}
    write_json(HERE / "candidate.json", declaration)
    source, index, mapping = rendering()
    (HERE / "programs.rt1").write_bytes(source)
    write_json(HERE / "programs.json", index)
    files = (sorted((ROOT / "harness").glob("*.py")) + [ROOT / "run_matrix.py"]
             + sorted(HERE.glob("*.py")) + [HERE / n for n in ("candidate.json", "programs.json", "programs.rt1")])
    sources = {p.relative_to(ROOT).as_posix(): sha(p.read_bytes()) for p in files}
    bindings = {
        "version": 1, "candidate_id": "rt1-own", "source_files": sources,
        "edition": "rt1-throwaway-prefix-v1; own typed interpreter, Python standard library exact integers",
        "execution_contract": {
            "notation_sha256": sha((CORPUS / "notation.md").read_bytes()),
            "profile_sha256": sha((CORPUS / "profile.json").read_bytes()),
            "source": "Parse pinned UTF-8 text; type/effect check and execute that same tree through typed IR. Full archive and module closure are bound; no eval or host code generation.",
            "static": "Join both branches, loop fixed point, context-insensitive call-site fixed point and sequential storage summaries. Check the inferred gate at every executed effect.",
            "runtime": "Static type/effect/upper-bound checks plus own immutable value labels and executed branch/loop context. Gates use actual labels; they must fit the static bound.",
            "separation": "Select preview authority from transitive effects; otherwise remove it. Preflight state-read effects through shared policy. Persist read_state and profile/parts; read also denotes the policy's fixed caps row.",
            "termination": "Ordinary data/control dependencies only. No stop-sensitive or accumulated-emission repair.",
        },
        "charge_mapping": {
            "instructions": "Every typed instruction maps one-to-one to its neutral path and UTF-8 span; one continuation before any effect, even on refusal.",
            "loops_calls": "Each loop test including final false: one step. Call and return: one each. Recursive logical frames: 16 bytes before entering. Parameters and loop aliases retain canonical size.",
            "constituents": sources["harness/calc.py"],
            "kernel": "ctx.kernel_call precharges tariff and batch chunks; batch buffer is never rebound/charged twice. Snapshot handle: 32 logical bytes.",
            "ordering": "Interrupt, steps, kernel cost, memory, output; unsuccessful later charge retains earlier successful charges.",
            "output": "Canonical exact plan/output before emit, publish, capture, workspace write and top-level return; internal returns have no output charge.",
            "lifetime": "Retention until run end, independent of Python GC. Pure checking/source admission/label bookkeeping has no additional neutral charge.",
        },
        "kernel_implementation": {"harness/kernel.py": sources["harness/kernel.py"]},
        "checkers": {p: sources[p] for p in (PREFIX + "/rt1_syntax.py", PREFIX + "/rt1_check.py",
                                               PREFIX + "/adapter.py", "harness/policy.py", "harness/classify.py", "harness/result.py")},
        "effective_grants": {profile: list(GRANTS) for profile in policy["profiles"]},
        "provenance_contract": {"version": 1, "host_sha256": sources["harness/host.py"],
                                "rule": "Only trusted adapter constructs blocks. All data/control/capture dependencies and aggregate parts join; stored blocks remain dependencies. Named setup and fixed picks retain host records."},
        "consumed_inputs": "Host ledger of workspace value/block/parts, pick records and previous exact output hash, expanded into the binding key. Inputs/profile/author are frozen host data, never source-minted authority.",
        "source_map": mapping, "rendered_file": PREFIX + "/programs.json",
    }
    write_json(HERE / "bindings.json", bindings)


def seal():
    if (HERE / "build").exists():
        raise ValueError("build must be absent before sealing")
    files = {p.relative_to(HERE).as_posix(): sha(p.read_bytes()) for p in sorted(HERE.rglob("*"))
             if p.is_file() and p.name != "MANIFEST.json"}
    write_json(HERE / "MANIFEST.json", {"version": 1, "decision": "open", "files": files,
                                       "content_sha256": digest(files)})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pins-only", action="store_true")
    parser.add_argument("--seal", action="store_true")
    parser.add_argument("--runs", type=int, default=10)
    args = parser.parse_args()
    if args.seal:
        seal()
        return 0
    build = (HERE / "build").resolve()
    if build.exists():
        raise ValueError("build must be absent before preparation")
    prepare()
    if args.pins_only:
        return 0
    try:
        summary = run(HERE, build / "Windows", args.runs, leg="Windows")
        write_json(HERE / "summary.json", summary)
        outputs = {p.name: {"sha256": sha(p.read_bytes()), "bytes": p.stat().st_size}
                   for p in (build / "Windows").iterdir() if p.is_file()}
        write_json(HERE / "series.json", {"decision": "open", "decider": "owner", "outputs": outputs,
                                         "bindings_sha256": digest(json.loads((HERE / "bindings.json").read_bytes())),
                                         "runs": args.runs, "leg": "Windows", "python": sys.version.split()[0],
                                         "command": "python work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt1-own --out candidates/rt1-own/build/Windows --runs " + str(args.runs) + " --leg Windows"})
        print("Recorded", summary["row_count"], "key observations; decision open.")
    finally:
        if build.exists():
            if build.parent != HERE or build.name != "build":
                raise ValueError("unexpected build location")
            shutil.rmtree(build)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
