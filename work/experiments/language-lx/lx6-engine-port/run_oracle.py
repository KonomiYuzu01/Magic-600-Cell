"""Build the std-only Rust port and compare all frozen cases, stopping at first difference."""
from __future__ import annotations

import gzip
import json
import os
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
from export_model import BUILD, HERE, INPUTS, ROOT, canonical, export, sha, verify_inputs, write_json

SOURCE_NAMES = ("export_model.py", "run_oracle.py", "check.py", "measure.py",
                "main.rs", "engine.rs", "json.rs", "sha256.rs", "INPUTS.json")
FLAGS = ["--edition=2021", "-O", "-D", "warnings", "-C", "debuginfo=0"]
EXE = BUILD / ("lx6-port.exe" if os.name == "nt" else "lx6-port")


def environment():
    temporary = BUILD / "tmp"
    temporary.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.update(TMP=str(temporary), TEMP=str(temporary), TMPDIR=str(temporary),
               PYTHONDONTWRITEBYTECODE="1")
    return env


def cleanup():
    # The sole recursive deletion target is this experiment's resolved build directory.
    if BUILD.exists():
        if BUILD.is_symlink() or BUILD.is_junction() or BUILD.resolve() != HERE / "build":
            raise ValueError("Refusing unexpected build path")
        shutil.rmtree(BUILD)


def prepare():
    model_sha = export()
    version = subprocess.run(["rustc", "--version", "--verbose"], check=True,
                             capture_output=True, text=True, env=environment()).stdout.strip()
    flags = FLAGS + (["-C", "link-arg=/Brepro"] if os.name == "nt" else [])
    candidate = dict(base_commit="1e7d0074df96cbeb05f55282cf8f6c03fbf4a4ff",
                     sources={name: sha((HERE / name).read_bytes()) for name in SOURCE_NAMES},
                     rustc=version, flags=flags)
    candidate_sha = sha(canonical(candidate).encode())
    env = environment()
    env["LX6_BUILD"] = candidate_sha
    print("Building Rust candidate " + candidate_sha, flush=True)
    subprocess.run(["rustc", "main.rs", *flags, "-o", str(EXE)], cwd=HERE,
                   env=env, check=True, timeout=240)
    test = subprocess.run([str(EXE), "self-test", str(BUILD / "model.bin")],
                          capture_output=True, check=True, env=env, timeout=120)
    report = json.loads(test.stdout)
    if report != {"status": "pass", "model_sha256": model_sha}:
        raise ValueError("Rust self-test or exported-model digest differs")
    candidate.update(candidate_sha256=candidate_sha, exported_model_sha256=model_sha,
                     executable_sha256=sha(EXE.read_bytes()), self_tests=report["status"],
                     driver_python=sys.version.split()[0])
    write_json(HERE / "CANDIDATE.json", candidate)
    return candidate


def compare(case, trace):
    process = subprocess.run([sys.executable, "-B", str(ROOT / "tools/oracle/compare.py"),
                              str(case), str(trace)], capture_output=True, env=environment(), timeout=120)
    try:
        report = json.loads(process.stdout)
    except ValueError:
        report = dict(status="error", error="Comparator produced no JSON report")
    return process.returncode, report


def record_difference(report):
    # Keep the first failure visible even if the operator stops before finishing the narrative.
    path = HERE / "RESULT.md"
    text = path.read_text(encoding="utf-8")
    marker = "<!-- first-oracle-difference -->"
    if marker not in text:
        text += "\n" + marker + "\nFirst oracle difference (spike stopped):\n\n```json\n"
        text += json.dumps(report, indent=2, sort_keys=True) + "\n```\n"
        path.write_bytes(text.encode())


def run_cases(candidate):
    verify_inputs()
    results = dict(candidate_sha256=candidate["candidate_sha256"],
                   exported_model_sha256=candidate["exported_model_sha256"],
                   status="running", cases=[], first_difference=None)
    output = HERE / "ORACLE_RESULTS.json"
    for number in range(1, 8):
        case_id = f"O{number:02}"
        case_path = INPUTS / "oracle" / (case_id + ".json.gz")
        raw = gzip.decompress(case_path.read_bytes())
        trace_path = BUILD / (case_id + ".trace.json")
        print("Replaying " + case_id, flush=True)
        with trace_path.open("wb") as trace:
            process = subprocess.run([str(EXE), "trace", str(BUILD / "model.bin")], input=raw,
                                     stdout=trace, stderr=subprocess.PIPE, env=environment(), timeout=300)
        if process.returncode:
            raise RuntimeError("Rust replay failed for " + case_id + ": " + process.stderr.decode().strip())
        trace_bytes = trace_path.read_bytes()
        # Fail on every omission even if an optional_fields extension is present in a future input.
        code, report = compare(case_path, trace_path)
        if code == 0 and (report.get("status") != "equal" or report.get("omitted_fields") != []):
            code, report = 1, dict(status="error", error="Omitted trace fields", comparison=report)
        row = dict(case=case_id, case_sha256=sha(case_path.read_bytes()), trace_sha256=sha(trace_bytes),
                   trace_bytes=len(trace_bytes), compare_exit=code, comparison=report)
        results["cases"].append(row)
        if code:
            results["status"] = "failed"
            results["first_difference"] = row
            write_json(output, results)
            record_difference(row)
            return code
        print(f"{case_id}: equal, {report['steps']} steps, no omissions", flush=True)
        write_json(output, results)

    # Deliberate corruption of a disposable copy proves comparator failure at an interior step.
    negative = json.loads((BUILD / "O07.trace.json").read_bytes())
    negative["steps"][8]["state_hash"] = "0" * 64
    negative_path = BUILD / "negative.trace.json"
    write_json(negative_path, negative)
    code, report = compare(INPUTS / "oracle/O07.json.gz", negative_path)
    if code != 1 or report.get("index") != 8:
        raise ValueError("Comparator failed to detect the deliberate negative control")
    results.update(status="equal", comparator_negative_control=dict(compare_exit=code, index=8,
                   field="state_hash", kind="deliberately corrupted trace copy"))
    verify_inputs()
    write_json(output, results)
    return 0


def main():
    try:
        return run_cases(prepare())
    finally:
        cleanup()


if __name__ == "__main__":
    raise SystemExit(main())
