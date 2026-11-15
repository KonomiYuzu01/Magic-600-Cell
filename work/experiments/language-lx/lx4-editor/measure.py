"""Function timers only: no key message, window, presented frame or bar judgement."""
import sys
sys.dont_write_bytecode = True

import argparse
import json
import math
import platform
import statistics
import subprocess
import time

from common import (ROOT, BUILD, PARSERS, LoweringError, compile_rust, encoded, load,
                    lower, put, rust_run, scratch, sha)
from edit_script import apply
from generate import generate, write_build

SERIES = 3
SAMPLES = 100
BARS = [
    ("Typed echo", "50 / 67", "50 / 67", "50 / 67"),
    ("Syntax feedback", "67 / 100", "67 / 100", "100 / 150"),
    ("Completion list shown", "100 / 150", "100 / 150", "150 / 250"),
    ("Type/effect diagnostics from last burst key", "150 / 250", "200 / 300", "500 / 750"),
    ("Stale state shown on view", "100 / 150", "100 / 150", "100 / 150"),
    ("Reference view re-run presented", "250 / 400", "250 / 400", "500 / 750"),
]


def jobs_for(modules, interfaces, checks):
    script = json.loads((ROOT / "EDITS.json").read_bytes())
    dependent, updated = checks.dependent_inputs(modules, interfaces, script)
    decoded = {k: json.loads(v) for k, v in updated.items()}
    jobs = []

    def job(size, metric, sources, mode):
        paths = []
        failures = 0
        for index, data in enumerate(sources):
            path = BUILD / "timing-inputs" / f"{size}-{metric}-{index:02}.lxa8"
            put(path, data)
            paths.append(path.relative_to(ROOT).as_posix())
            parsed = PARSERS["a"](data)
            if mode == "parse":
                failures += bool(parsed["diagnostics"])
            else:
                try:
                    lower(parsed, decoded)
                except LoweringError:
                    failures += 1
        jobs.append({"id": size + "/" + metric, "size": size, "metric": metric,
                     "sources": paths, "source_sha256": [sha(s) for s in sources],
                     "mode": mode, "expected_errors": failures,
                     "series": SERIES, "samples": SAMPLES})

    for size in "SMLW":
        sources = [data for name, data in modules.items() if name.startswith(size)]
        job(size, "cold_parse", sources, "parse")
        job(size, "cold_check", sources, "check")
        for row in script["edits"]:
            if row["module"].startswith(size):
                data = apply(modules[row["module"]], row)
                kind = row["kind"]
                job(size, "completion_parse" if kind == "completion" else kind + "_parse_check",
                    [data], "parse" if kind == "completion" else "both")
    job("W", "dependents_check", list(dependent.values()), "check")
    return jobs, updated


def python_series(job, interfaces):
    sources = [(ROOT / path).read_bytes() for path in job["sources"]]
    mode = job["mode"]
    parsed = [PARSERS["a"](data) for data in sources] if mode == "check" else []

    def run():
        failures = 0
        if mode == "check":
            for tree in parsed:
                try:
                    lower(tree, interfaces)
                except LoweringError:
                    failures += 1
        else:
            for data in sources:
                tree = PARSERS["a"](data)
                if mode == "parse":
                    failures += bool(tree["diagnostics"])
                else:
                    try:
                        lower(tree, interfaces)
                    except LoweringError:
                        failures += 1
        return failures

    invalid = []

    def sample(series, index):
        start = time.perf_counter_ns()
        errors = run()
        elapsed = time.perf_counter_ns() - start
        if errors != job["expected_errors"]:
            invalid.append({"series": series, "sample": index, "errors": errors})
        return elapsed

    smoke = sample(0, 0)
    series = [[sample(s + 1, n) for n in range(SAMPLES)] for s in range(SERIES)]
    return {"id": job["id"], "smoke_ns": smoke, "series_ns": series, "invalid": invalid}


def summary(samples):
    ordered = sorted(samples)
    # Nearest rank p95/p99; median is the midpoint of the two middle samples.
    return [statistics.median(ordered) / 1e6,
            ordered[math.ceil(0.95 * len(ordered)) - 1] / 1e6,
            ordered[math.ceil(0.99 * len(ordered)) - 1] / 1e6,
            ordered[-1] / 1e6]


def table(record):
    lines = ["# LX-4 headless function-timer series", "",
             "Experiment E-2.4-L4. Decision: **open**; decider: **owner**.", "",
             "**Function timers, a lead only. The proposed bars below are context and are not compared.**",
             "No key-to-frame interval, editor, window, PresentMon, renderer gate or E2 was measured.", "",
             "RTE.49 proposed context only, p95 / p99 milliseconds:", "",
             "| Interval | S | M | L |", "| --- | --- | --- | --- |"]
    lines += ["| " + " | ".join(row) + " |" for row in BARS]
    lines += ["", "W proposed dependent re-check context: 1,000 / 1,500 ms. **Not compared.**", "",
              "Function times below use milliseconds. All three 100-sample series are shown separately.",
              "One preceding smoke sample per row is retained in `timings.json`; it is excluded from quantiles.",
              "p95/p99 use nearest rank; median averages the middle pair. `cold` means fresh parser/checker work",
              "without a retained result cache, in an already running process; it does not mean cold OS caches",
              "or application startup. `cold_check` and `dependents_check` receive already parsed trees.",
              "Cold W covers all 50 modules; W edits cover W00 (400 lines); W dependent checks cover 49 modules.",
              "Syntax-break checks refuse the draft. Completion measures parsing the broken tail, not constructing",
              "or displaying a completion list. See RESULT.md for host/build and exclusions.", "",
              "| Host | Size / function | Series | n | median ms | p95 ms | p99 ms | largest ms |",
              "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |"]
    for host, results in record["hosts"].items():
        for row in results:
            for series, samples in enumerate(row["series_ns"], 1):
                values = " | ".join(f"{x:.3f}" for x in summary(samples))
                lines.append(f"| {host} | {row['id']} | {series} | {len(samples)} | {values} |")
    lines += ["", "These are synthetic source workloads in a shared sandboxed machine run. No bar has a pass/fail result.",
              "The two-puzzle 10,800-sample key-to-presented-frame series and all foreground work are **not run here**.",
              "Raw nanoseconds and source/build/input identities: `timings.json`.", ""]
    return "\n".join(lines).encode("utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", action="store_true", help="replace timings.json and SERIES.md with this new observation")
    args = parser.parse_args()
    checks = load("lx4_acceptance", ROOT / "check.py")
    manifest = checks.frozen()
    record = {"schema": "lx4-function-timers-0", "experiment": "E-2.4-L4", "decision": "open", "decider": "owner",
              "evidence": "source and synthetic; function timing lead only", "bars_compared": False,
              "candidate_sha256": manifest["candidate_sha256"], "input_sha256": manifest["input_sha256"],
              "module_set_sha256": manifest["module_set_sha256"], "edit_sha256": manifest["edit_sha256"],
              "python": platform.python_version(), "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
              "os": platform.system(), "os_release": platform.release(), "rust_flags": "--edition=2021 -C opt-level=3 -C debuginfo=0",
              "clock_python": "perf_counter_ns", "clock_rust": "Instant", "series": SERIES, "samples": SAMPLES,
              "machine_conditions": "shared machine; D6 conditions not controlled or recorded; no decisive timing",
              "hosts": {}}
    with scratch():
        exe = compile_rust()
        record["rust_binary_sha256"] = sha(exe.read_bytes())
        checks.corpus(exe)
        checks.generated(exe, manifest)
        modules, interfaces, _ = generate()
        write_build(modules)
        jobs, interfaces = jobs_for(modules, interfaces, checks)
        record["jobs"] = jobs
        decoded = {key: json.loads(data) for key, data in interfaces.items()}
        for host in ("CPython/LX-8 candidate-a", "Rust/std candidate-a"):
            rows = []
            record["hosts"][host] = rows
            for job in jobs:
                print(host, job["id"], "smoke + 3 x 100", flush=True)
                if host.startswith("CPython"):
                    row = python_series(job, decoded)
                else:
                    row = rust_run(exe, "measure", [job], interfaces)[0]
                rows.append(row)
                print("  series p99 ms:", ", ".join(f"{summary(s)[2]:.3f}" for s in row["series_ns"]), flush=True)
    record["status"] = "invalid" if any(row["invalid"] for rows in record["hosts"].values() for row in rows) else "valid function run"
    output = table(record)
    if args.record:
        put(ROOT / "timings.json", record)
        put(ROOT / "SERIES.md", output)
        print("Recorded timings.json and SERIES.md; build removed.")
    else:
        print(output.decode("utf-8"))
    print("Raw observation SHA-256:", sha(encoded(record)))
    print("Function timer lead only. Proposed bars are not compared. Decision: open; decider: owner.")
    if record["status"] == "invalid":
        raise SystemExit("Invalid correctness run retained; fix and repeat the whole measurement.")


if __name__ == "__main__":
    main()
