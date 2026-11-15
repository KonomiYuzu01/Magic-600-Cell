"""Explicit preparation only; acceptance never refreshes a pin or a result."""

import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(1, str(ROOT))

from harness.common import canonical, cases, digest, load, sha
from harness.policy import GRANTS
from frontend import compile_program
from render import bundle, render

FORMS = ("capability_separation", "static_labels", "static_runtime_labels")
OWN_SOURCES = ("adapter.py", "bridge.js", "render.py", "frontend.py", "prepare.py", "probe.py", "check.py", "report.py")


def artifacts():
    surfaces = {digest(case["program"]): render(case["program"]) for _, case in cases()}
    # JSON serialization sorts entry keys. Compile that same parsed source order.
    surfaces = json.loads(canonical(surfaces))
    source, spans = bundle(surfaces)
    transparent = load("policy.json")["transparent"]
    compiled = {key: {form: {run_id: compile_program(surface, run_id, form, transparent)
                            for run_id in surface["entries"]} for form in FORMS}
                for key, surface in sorted(surfaces.items())}
    return {"programs.rt3": source, "modules.json": canonical(compiled) + b"\n"}, spans


def declaration():
    return {"version": 1, "id": "rt3-wasm", "adapter": "adapter.py", "cells": [
        {"form": form, "variant": variant,
         "status": "supported" if variant == "source_bound" else "not_run",
         "reason": None if variant == "source_bound" else "own_front_end_has_no_host_subset"}
        for form in FORMS for variant in ("source_bound", "approved_subset_byte_bound")]}


def bindings(rendered, spans, manifest):
    prefix = HERE.relative_to(ROOT).as_posix() + "/"
    source_files = {p.relative_to(ROOT).as_posix(): sha(p.read_bytes())
                    for p in sorted((ROOT / "harness").glob("*.py"))}
    source_files["run_matrix.py"] = sha((ROOT / "run_matrix.py").read_bytes())
    source_files.update({prefix + name: sha((HERE / name).read_bytes()) for name in OWN_SOURCES})
    source_files.update({prefix + name: sha(data) for name, data in rendered.items()})
    source_files[prefix + "candidate.json"] = sha(canonical(manifest) + b"\n")
    mapping = {}
    for _, case in cases():
        key = digest(case["program"])
        mapping[case["id"]] = {path: {"file": prefix + "programs.rt3", "start": start, "end": end,
                                     "sha256": sha(rendered["programs.rt3"][start:end])}
                               for path, (start, end) in spans[key].items()}
    return {"version": 1, "candidate_id": "rt3-wasm", "source_files": source_files,
            "edition": "rt3-array-1; throwaway experiment surface, no proposed syntax",
            "execution_contract": {
                "notation": sha((ROOT.parent / "corpus/notation.md").read_bytes()),
                "profile": sha((ROOT.parent / "corpus/profile.json").read_bytes()),
                "target": "WebAssembly 1 binary with multi-value function results, Node built-in engine",
                "versions": {"Python": "3.15.1", "Node": "v24.21.0"},
                "values": "Python immutable host handles; i32 control/labels; i64 exact step fuel",
                "source_binding": "whole surface and every compiled module regenerated before admission"},
            "charge_mapping": {
                "instruction": "Wasm step import before each source instruction",
                "loop": "Wasm loop_test import before each test, including final false test",
                "constituents": "shared calc invokes the emitted charge export before each constituent; nested meter import",
                "resources": "precharged kernel/memory/output host intrinsics; batch growth once; aliases retained in full",
                "recursion": "16 logical bytes before each direct recursive call; compiler call_enter import",
                "calc_sha256": source_files["harness/calc.py"],
                "meter_sha256": source_files["harness/meter.py"]},
            "kernel_implementation": {"harness/kernel.py": source_files["harness/kernel.py"]},
            "checkers": {path: source_files[path] for path in (
                "harness/policy.py", "harness/classify.py", "harness/result.py",
                prefix + "frontend.py", prefix + "adapter.py")},
            "effective_grants": {profile: list(GRANTS) for profile in load("policy.json")["profiles"]},
            "provenance_contract": {
                "host": source_files["harness/host.py"], "version": 1,
                "labels": "state=1, computed=2, research=4, unknown=8, read_state=16",
                "static": "both branches; monotone loop/module/workspace summaries",
                "runtime": "Wasm ORs operand and pc fields; control handles preserve parts; metadata ingress only",
                "separation": "remove preview for a solving entry with a reachable state row; shared policy enforces exclusivity"},
            "consumed_inputs": "host ledger: workspace value/block/parts, pick, previous exact output; included in per-run binding digest",
            "source_map": mapping, "rendered_file": prefix + "programs.rt3"}


def main():
    rendered, spans = artifacts()
    manifest = declaration()
    pinned = bindings(rendered, spans, manifest)
    for name, data in rendered.items():
        (HERE / name).write_bytes(data)
    (HERE / "candidate.json").write_bytes(canonical(manifest) + b"\n")
    (HERE / "bindings.json").write_bytes(canonical(pinned) + b"\n")
    print("Prepared RT3 surface, compiled modules, six declarations and source pins.")


if __name__ == "__main__":
    main()
