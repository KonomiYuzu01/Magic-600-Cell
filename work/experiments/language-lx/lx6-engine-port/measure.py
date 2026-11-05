"""B4-12-shaped harness. Default full mode: smoke, then 3 x 100 samples per metric/backend.

Only the explicitly selected pinned reference worker imports NumPy. The driver and exporter
are Python standard library only. No process startup, model load, resets or hashes are timed.
"""
from __future__ import annotations

import argparse
import gzip
import json
import math
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
from export_model import BUILD, HERE, INPUTS, ROOT, canonical, export, sha, verify_inputs, write_json
from run_oracle import EXE, SOURCE_NAMES, cleanup, environment, prepare

METRICS = ("apply_generator", "apply_long_word", "compose_macros", "net_effect")


def prepare_frozen():
    """Rebuild the recorded Rust candidate without replacing any frozen evidence."""
    candidate = json.loads((HERE / "CANDIDATE.json").read_bytes())
    identity = {key: candidate[key] for key in ("base_commit", "sources", "rustc", "flags")}
    if sha(canonical(identity).encode()) != candidate["candidate_sha256"]:
        raise ValueError("Frozen candidate identity differs")
    if set(candidate["sources"]) != set(SOURCE_NAMES):
        raise ValueError("Frozen candidate source set differs")
    for name, digest in candidate["sources"].items():
        # This packet changes only these two harness scripts; the Rust sources stay frozen.
        if name not in ("check.py", "measure.py") and sha((HERE / name).read_bytes()) != digest:
            raise ValueError("Frozen candidate source differs: " + name)
    model_sha = export()
    if model_sha != candidate["exported_model_sha256"]:
        raise ValueError("Frozen exported-model digest differs")
    env = environment()
    version = subprocess.run(["rustc", "--version", "--verbose"], check=True,
                             capture_output=True, text=True, env=env).stdout.strip()
    if version != candidate["rustc"]:
        raise ValueError("Frozen Rust compiler version differs")
    env["LX6_BUILD"] = candidate["candidate_sha256"]
    print("Building frozen Rust candidate " + candidate["candidate_sha256"], flush=True)
    subprocess.run(["rustc", "main.rs", *candidate["flags"], "-o", str(EXE)], cwd=HERE,
                   env=env, check=True, timeout=240)
    test = subprocess.run([str(EXE), "self-test", str(BUILD / "model.bin")],
                          capture_output=True, check=True, env=env, timeout=120)
    if json.loads(test.stdout) != {"status": "pass", "model_sha256": model_sha}:
        raise ValueError("Rust self-test or exported-model digest differs")
    return candidate


def request(samples):
    def case(number):
        return json.loads(gzip.decompress((INPUTS / "oracle" / f"O{number:02}.json.gz").read_bytes()))

    o1, o2, o3, o4 = (case(i) for i in range(1, 5))
    return dict(samples=samples, generator=o1["steps"][0]["recipe"][0]["moves"][0],
                initial=[dict(kind="word", moves=o3["initial"]["moves"])],
                long_word=[dict(kind="word", moves=[step["recipe"][0]["moves"][0] for step in o2["steps"]])],
                macros=[step["recipe"] for step in o4["steps"]],
                net_recipe=o4["steps"][0]["recipe"])


def numpy_worker():
    import numpy as np
    sys.path.insert(0, str(ROOT))
    from core import Model, canonical as core_canonical, state_hash

    if sys.version_info[:3] != (3, 14, 7) or np.__version__ != "2.3.5":
        raise ValueError("Reference worker requires CPython 3.14.7 and NumPy 2.3.5")
    data = json.load(sys.stdin)
    model = Model()
    initial = model.ids.copy()
    for move in model.expand(data["initial"]):
        s, d = model.move(move)
        initial[d] = initial[s]
    word = list(model.expand(data["long_word"]))
    macros = [model.net(recipe)[:2] for recipe in data["macros"]]
    # The reference uses its retained warm star cache. Report that asymmetry explicitly.
    model.net(data["net_recipe"])
    rows = []
    for metric in METRICS:
        times, digest = [], None
        for _ in range(data["samples"]):
            labels = initial.copy()
            start = time.perf_counter_ns()
            if metric == "apply_generator":
                s, d = model.move(data["generator"])
                labels[d] = labels[s]
                elapsed = time.perf_counter_ns() - start
                current = state_hash(labels)
            elif metric == "apply_long_word":
                for move in word:
                    s, d = model.move(move)
                    labels[d] = labels[s]
                elapsed = time.perf_counter_ns() - start
                current = state_hash(labels)
            elif metric == "compose_macros":
                mapping = model.ids.copy()
                for s, d in macros:
                    mapping[d] = mapping[s]
                d = np.flatnonzero(mapping != model.ids).astype(np.int32)
                s = mapping[d]
                order = np.argsort(s)
                s, d = s[order], d[order]
                elapsed = time.perf_counter_ns() - start
                current = sha(s.astype("<i4").tobytes() + d.astype("<i4").tobytes())
            else:
                s, d, _, _ = model.net(data["net_recipe"])
                elapsed = time.perf_counter_ns() - start
                current = sha(s.astype("<i4").tobytes() + d.astype("<i4").tobytes())
            if digest is not None and current != digest:
                raise ValueError("NumPy output changed between samples")
            digest = current
            times.append(elapsed)
        rows.append(dict(metric=metric, ns=times, output_sha256=digest))
    print(core_canonical(dict(backend="numpy", rows=rows, python=sys.version.split()[0], numpy=np.__version__)))


def engine_python():
    # Local root environment after integration; the packet's relative location in this worktree.
    candidates = [ROOT / "tools/.venv/engine/Scripts/python.exe",
                  ROOT / "../../../../../../../../tools/.venv/engine/Scripts/python.exe"]
    if os.environ.get("LX6_ENGINE_PYTHON"):
        candidates.insert(0, Path(os.environ["LX6_ENGINE_PYTHON"]))
    for path in candidates:
        if path.is_file():
            return path
    raise ValueError("Pinned reference Python not found; set LX6_ENGINE_PYTHON to its executable")


def run_pair(samples, series):
    data = canonical(request(samples)).encode()
    commands = {
        "rust": [str(EXE), "bench", str(BUILD / "model.bin")],
        "numpy": [str(engine_python()), "-B", str(HERE / "measure.py"), "--numpy-worker"],
    }
    results = {}
    # Alternate backend order by series; both are sequential on the same machine.
    for backend in (("rust", "numpy") if series % 2 == 0 else ("numpy", "rust")):
        process = subprocess.run(commands[backend], input=data, capture_output=True,
                                 env=environment(), check=True, timeout=600)
        result = json.loads(process.stdout)
        if result["backend"] != backend or [r["metric"] for r in result["rows"]] != list(METRICS):
            raise ValueError("Unexpected benchmark rows")
        for row in result["rows"]:
            if len(row["ns"]) != samples or any(type(v) is not int or v < 0 for v in row["ns"]):
                raise ValueError("Missing or invalid benchmark samples")
        results[backend] = result
    for rust, numpy in zip(results["rust"]["rows"], results["numpy"]["rows"]):
        if rust["output_sha256"] != numpy["output_sha256"]:
            raise ValueError("Timing output differs: " + rust["metric"])
    return dict(series=series, input_sha256=sha(data), backends=results)


def summary(series):
    rows = []
    for run in series:
        for backend, result in run["backends"].items():
            for row in result["rows"]:
                values = sorted(row["ns"])
                rows.append(dict(series=run["series"], backend=backend, metric=row["metric"],
                                 samples=len(values), median_ns=statistics.median(values),
                                 p99_ns=values[math.ceil(0.99 * len(values)) - 1]))
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--smoke", action="store_true", help="One sample per metric; no speed judgement")
    parser.add_argument("--output", type=Path, help="Write a frozen-candidate verification report to this JSON path")
    parser.add_argument("--conditions", help="Repo-relative JSON file recording D6 conditions, required in full mode")
    parser.add_argument("--numpy-worker", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.numpy_worker:
        numpy_worker()
        return 0
    conditions = dict(machine_activity="shared sandbox; concurrent runs possible", evidence="smoke only")
    if not args.smoke:
        if not args.conditions or Path(args.conditions).is_absolute():
            parser.error("Full mode requires --conditions with a repo-relative D6 conditions JSON file")
        conditions = json.loads((ROOT / args.conditions).read_bytes())
        required = ("power_source", "windows_power_mode", "vendor_performance_mode", "driver_vsync", "overlays",
                    "frame_generation", "cpu", "gpu", "quiet_machine", "evidence_class")
        if any(key not in conditions for key in required):
            parser.error("Conditions file must record: " + ", ".join(required))
    output = args.output.resolve() if args.output else HERE / ("SMOKE.json" if args.smoke else "MEASUREMENTS.json")
    report = dict(mode="smoke" if args.smoke else "full", status="running", conditions=conditions,
                  p99_method="nearest rank: sorted[ceil(0.99*n)-1]", smoke=None, series=[])
    try:
        candidate = prepare_frozen() if args.output else prepare()
        report["candidate_sha256"] = candidate["candidate_sha256"]
        report["exported_model_sha256"] = candidate["exported_model_sha256"]
        write_json(output, report)
        report["smoke"] = run_pair(1, 0)
        write_json(output, report)
        print("SMOKE RUN: one sample per metric/backend; all four output digests equal; no performance conclusion.", flush=True)
        if not args.smoke:
            for number in range(1, 4):
                report["series"].append(run_pair(100, number))
                write_json(output, report)
                print(f"Series {number}: 100 samples per metric/backend, outputs equal", flush=True)
            report["summary"] = summary(report["series"])
        report["status"] = "pass"
        verify_inputs()
        write_json(output, report)
        return 0
    except Exception as error:
        report.update(status="failed", error_type=type(error).__name__)
        write_json(output, report)
        raise
    finally:
        cleanup()
        if output.is_relative_to(BUILD):
            # Keep only the requested report for check.py to compare, then remove with build/.
            output.parent.mkdir(parents=True, exist_ok=True)
            write_json(output, report)


if __name__ == "__main__":
    raise SystemExit(main())
