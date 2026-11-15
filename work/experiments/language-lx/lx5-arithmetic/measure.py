"""LX-5 per-operation measurements. Smoke is a harness check, never decisive timing."""
import argparse
from datetime import datetime, timezone
import json
import math
from pathlib import Path
import statistics
import subprocess
import sys

sys.dont_write_bytecode = True
from common import BUILD, CANDIDATES, ROOT, cleanup, compile_candidates, json_bytes, reference_bytes, run, sha, versions, write
from compare import compare
from make_fixtures import freeze
from reference import load


def source_identity():
    names = ("candidate.mjs", "candidate.cs", "candidate.rs", "reference.py", "make_fixtures.py",
             "compare.py", "common.py", "check.py", "measure.py", "licences.py", "CARD.md", "LICENCES.md", "LICENCE-EVIDENCE.json")
    return {name: sha((ROOT / name).read_bytes()) for name in names}


def stats(samples):
    ordered = sorted(samples)
    return {"n": len(samples), "median_ns": statistics.median(ordered),
            "p99_ns": ordered[math.ceil(0.99 * len(ordered)) - 1],
            "min_ns": ordered[0], "max_ns": ordered[-1]}


def measure(smoke, conditions=None, *, cleanup_build=True):
    identity = source_identity()
    count, series_count = (3, 1) if smoke else (100, 3)
    result = {"format": "lx5-measure/1", "mode": "smoke" if smoke else "full",
              "started_utc": datetime.now(timezone.utc).isoformat(), "source_digests": identity,
              "samples_per_series": count, "series": series_count, "warmups_per_case_per_series": 1,
              "timing_scope": "operation on already parsed operands, including canonical output formatting; excludes startup and file input except word stream I/O",
              "evidence_class": ["source", "synthetic"], "timing_status": "lead only; no decisive timing claim",
              "conditions": conditions or {key: "not recorded" for key in
                  ("power_source", "windows_power_mode", "vendor_performance_mode", "driver_vsync", "overlays", "frame_generation")},
              "decision": "open", "decider": "owner", "candidates": {}}
    try:
        freeze()
        result["fixture_manifest_sha256"] = sha((ROOT / "fixtures/MANIFEST.json").read_bytes())
        result["tools"] = versions()
        commands = compile_candidates()
        values, cases = load(ROOT / "fixtures/operands.tsv")
        selected = [c for c in cases if c["measured"]]
        selected_ids = {c["id"] for c in selected}
        expected = b"".join(line for line in reference_bytes().splitlines(keepends=True)
                            if line.split(b"\t", 1)[0].decode("ascii") in selected_ids)
        for name in CANDIDATES:
            output = BUILD / (name + "-bench.tsv")
            try:
                run(commands[name] + [ROOT / "fixtures/operands.tsv", BUILD / "words", output,
                                     "bench", str(count), str(series_count)], timeout=600 if smoke else 7200)
                comparison = compare(expected, output.read_bytes(), selected, values)
                raw = output.with_name(output.name + ".timings.tsv").read_bytes()
                buckets = {(c["id"], series): {} for c in selected for series in range(series_count)}
                for line in raw.decode("ascii").splitlines():
                    key, series, sample, ns = line.split("\t")
                    series, sample, ns = int(series), int(sample), int(ns)
                    assert 0 <= sample < count and ns >= 0
                    assert sample not in buckets[key, series], "duplicate timing sample"
                    buckets[key, series][sample] = ns
                rows = []
                for c in selected:
                    per_series = []
                    for series in range(series_count):
                        bucket = buckets[c["id"], series]
                        assert set(bucket) == set(range(count)), "missing timing sample"
                        samples = [bucket[i] for i in range(count)]
                        row = dict(series=series, **stats(samples))
                        if smoke:
                            row["raw_ns"] = samples
                        per_series.append(row)
                    rows.append(dict(case=c["id"], size=c["size"], operation=c["op"], series=per_series))
                result["candidates"][name] = {"status": comparison["status"], "comparison": comparison,
                    "rows": rows, "raw_timing_sha256": sha(raw), "raw_timing_bytes": len(raw),
                    "raw_retained": "inline samples" if smoke else "digest; regenerate with measure.py"}
                print(f"{name}: smoke exact={comparison['status']} metrics={len(rows)}" if smoke
                      else f"{name}: full exact={comparison['status']} metrics={len(rows)}", flush=True)
            except subprocess.TimeoutExpired:
                result["candidates"][name] = {"status": "not run to completion", "reason": "timeout; no passing timing result"}
                print(name + ": measurement timeout", flush=True)
            except subprocess.CalledProcessError as error:
                result["candidates"][name] = {"status": "not run to completion", "reason": "candidate exit " + str(error.returncode)}
                print(name + ": measurement failed", flush=True)
        assert source_identity() == identity, "source changed during measurement"
        result["complete"] = all(name in result["candidates"] for name in CANDIDATES)
        return result
    except Exception as error:
        result["complete"] = False
        result["failure"] = type(error).__name__ + "; measurement incomplete (diagnostics not published)"
        for name in CANDIDATES:
            result["candidates"].setdefault(name, {"status": "not run to completion"})
        print(result["failure"], flush=True)
        return result
    finally:
        if cleanup_build:
            cleanup()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--conditions", type=Path, help="optional public D6 condition record, never machine diagnostics")
    parser.add_argument("--output", type=Path, help="explicit result path instead of the stored measurement file")
    parser.add_argument("--keep-build", action="store_true", help="leave build cleanup to the calling check")
    args = parser.parse_args()
    conditions = json.loads(args.conditions.read_bytes()) if args.conditions else None
    result = measure(args.smoke, conditions, cleanup_build=not args.keep_build)
    write(args.output or ROOT / ("smoke.json" if args.smoke else "full-measurement.json"), json_bytes(result))
    # Numeric mismatches are negative results, not a missing report. Infrastructure
    # failures cannot establish that the measurement harness ran successfully.
    raise SystemExit(0 if result["complete"] and all(c["status"] in ("exact", "removed")
                    for c in result["candidates"].values()) else 1)
