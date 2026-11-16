"""Replay the frozen oracle cases O01 to O07 through the Rust kernel (LP2, M4; CT-KN2-01, the 600-cell-Full half).

The check builds the oracle adapter of `rust/c600-kernel` with `cargo build --release --locked --offline` in rust/,
checks the seven tracked case files against MANIFEST-oracle.json, lets the adapter write one trace per case into a
temporary folder, and requires `tools/oracle/compare.py` to report "equal" with no omitted fields for every case
(3,201 steps). Two controls follow: a trace with one tampered state hash must fail at that step (exit 1), and a case
that names another model identity must be refused by the adapter (exit 2) without a trace.

Last, the adapter's `agree` command runs the KN2.9 agreement test: the star cache's net maps against per-primitive
replay for the first, middle and last frame node of every orbit and both signs (210 stars), and the word [1, -1]. It
runs here, in the release binary, because replaying its 385,836 primitives one by one takes too long in the
unoptimized profile of `cargo test`, which runs a short version of the same comparison on one orbit.

Standard library only: the frozen case files are the inputs, so no step needs NumPy. A missing Rust toolchain fails
the check: a test that did not run is no pass (PHS.63). A pass is headless source and fixture evidence, not
Windows/DirectX, input, long-session or performance evidence; the timings it prints are informational.
"""
from __future__ import annotations

import gzip
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKSPACE = ROOT / "rust"
ASSETS = ROOT / "assets"
INPUTS = ROOT / "work" / "experiments" / "language-lx" / "inputs"
COMPARE = ROOT / "tools" / "oracle" / "compare.py"
ADAPTER = "c600-oracle-adapter"
CANDIDATE = "c600-kernel-full-candidate"
STEPS = {"O01": 2400, "O02": 500, "O03": 210, "O04": 25, "O05": 50, "O06": 6, "O07": 10}
TOTAL_STEPS = 3201
AGREEMENT_STARS = 210
TAMPERED_CASE, TAMPERED_INDEX = "O07", 8
# Below the CI runner's limit per check (tools/ci/run_headless.py), so a slow step is reported here.
BUILD_LIMIT_SECONDS = 900
RUN_LIMIT_SECONDS = 300


class Failed(Exception):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise Failed(message)


def version(tool: str) -> str:
    try:
        out = subprocess.run([tool, "--version"], capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.TimeoutExpired) as error:
        return f"{Path(tool).name}: unavailable ({error})"
    return out.stdout.strip() or f"{Path(tool).name}: no version (exit {out.returncode})"


def run(command: list[str], limit: int = RUN_LIMIT_SECONDS) -> subprocess.CompletedProcess:
    try:
        return subprocess.run(command, capture_output=True, text=True, timeout=limit)
    except subprocess.TimeoutExpired:
        raise Failed(f"{Path(command[1]).name if len(command) > 1 else command[0]} ran over {limit} s") from None


def one_line(out: subprocess.CompletedProcess, what: str) -> dict:
    lines = out.stdout.strip().splitlines()
    require(len(lines) == 1, f"{what} printed {len(lines)} lines instead of one JSON line")
    try:
        return json.loads(lines[0])
    except ValueError:
        raise Failed(f"{what} printed no JSON") from None


def verify_inputs() -> list[Path]:
    """The seven frozen case files, checked against their manifest before use."""
    manifest = json.loads((INPUTS / "MANIFEST-oracle.json").read_text(encoding="utf-8"))
    paths = []
    for name in STEPS:
        path = INPUTS / "oracle" / f"{name}.json.gz"
        entry = manifest["files"].get(path.name)
        require(entry is not None, f"{path.name} is not in MANIFEST-oracle.json")
        data = path.read_bytes()
        require(len(data) == entry["bytes"] and hashlib.sha256(data).hexdigest() == entry["sha256"],
                f"{path.name} differs from MANIFEST-oracle.json")
        paths.append(path)
    require(len(manifest["files"]) == len(STEPS), "MANIFEST-oracle.json lists other case files")
    print(f"inputs: {len(paths)} case files match MANIFEST-oracle.json")
    return paths


def build(cargo: str) -> Path:
    """Builds the adapter in release and returns the executable cargo reports."""
    command = [cargo, "build", "--release", "--locked", "--offline", "-p", "c600-kernel", "--bin", ADAPTER,
               "--message-format=json-render-diagnostics"]
    start = time.monotonic()
    try:
        out = subprocess.run(command, cwd=WORKSPACE, stdout=subprocess.PIPE, text=True,
                             timeout=BUILD_LIMIT_SECONDS)
    except subprocess.TimeoutExpired:
        raise Failed(f"cargo build ran over {BUILD_LIMIT_SECONDS} s") from None
    require(out.returncode == 0, f"cargo build --release --locked --offline exited with {out.returncode}")
    executable = None
    for line in out.stdout.splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if (message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == ADAPTER
                and message.get("executable")):
            executable = Path(message["executable"])
    require(executable is not None and executable.is_file(), "cargo reported no adapter executable")
    print(f"build: cargo build --release --locked --offline -p c600-kernel --bin {ADAPTER} "
          f"({time.monotonic() - start:.1f} s)")
    return executable


def compare(case: Path, trace: Path) -> tuple[int, dict, str]:
    out = run([sys.executable, str(COMPARE), str(case), str(trace)])
    line = out.stdout.strip()
    return out.returncode, one_line(out, "compare.py"), line


def check_traces(adapter: Path, cases: list[Path], folder: Path) -> tuple[dict[str, Path], dict]:
    traces = {case.name[:3]: folder / f"{case.name[:3]}.trace.json" for case in cases}
    command = [str(adapter), "trace", "--assets", str(ASSETS)]
    for case in cases:
        command += [str(case), str(traces[case.name[:3]])]
    start = time.monotonic()
    out = run(command)
    wall = time.monotonic() - start
    require(out.returncode == 0, f"the adapter's trace command exited with {out.returncode}: {out.stderr.strip()}")
    summary = one_line(out, "the adapter's trace command")
    require(summary.get("candidate") == CANDIDATE, "the trace summary names another candidate")
    build_string = summary.get("build", "")
    print(f"adapter build: {build_string}")
    print(f"trace: model load {summary['load_seconds']} s, all seven cases {summary['total_seconds']} s "
          f"in the adapter, {wall:.1f} s wall; per case: "
          + ", ".join(f"{c['case_id']} {c['steps']} steps {c['seconds']} s" for c in summary["cases"]))
    steps = 0
    for case in cases:
        name = case.name[:3]
        trace = json.loads(traces[name].read_text(encoding="utf-8"))
        require(trace["candidate"] == {"name": CANDIDATE, "build": build_string}, f"{name}: trace candidate differs")
        code, report, line = compare(case, traces[name])
        print(f"compare {name}: exit {code} {line}")
        require(code == 0, f"compare.py exited with {code} for {name}")
        require(report.get("status") == "equal" and report.get("omitted_fields") == []
                and report.get("case_id") == name and report.get("steps") == STEPS[name],
                f"compare.py did not report {name} equal with every field")
        steps += report["steps"]
    require(steps == TOTAL_STEPS, f"{steps} steps compared instead of {TOTAL_STEPS}")
    print(f"PASS: all seven cases equal, {steps} steps, no omitted fields")
    return traces, summary


def check_tampered_trace(cases: list[Path], traces: dict[str, Path], folder: Path) -> None:
    trace = json.loads(traces[TAMPERED_CASE].read_text(encoding="utf-8"))
    step = next(step for step in trace["steps"] if step["index"] == TAMPERED_INDEX)
    original = step["state_hash"]
    step["state_hash"] = ("1" if original[0] == "0" else "0") + original[1:]
    tampered = folder / "tampered.trace.json"
    tampered.write_text(json.dumps(trace, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")
    case = next(case for case in cases if case.name.startswith(TAMPERED_CASE))
    code, report, line = compare(case, tampered)
    print(f"negative control ({TAMPERED_CASE} step {TAMPERED_INDEX} state_hash changed): exit {code} {line}")
    require(code == 1 and report.get("status") == "mismatch" and report.get("index") == TAMPERED_INDEX
            and [d["field"] for d in report.get("differences", [])] == ["state_hash"],
            "compare.py did not report the tampered state hash at its step")
    print(f"PASS: the tampered trace fails at index {TAMPERED_INDEX}, field state_hash")


def check_identity_refusal(adapter: Path, cases: list[Path], folder: Path) -> None:
    source = next(case for case in cases if case.name.startswith(TAMPERED_CASE))
    original = json.loads(gzip.decompress(source.read_bytes()))
    for key in ("model_id", "manifest_sha256"):
        case = json.loads(json.dumps(original))
        case["model"][key] = "0" * 64
        path = folder / f"other-{key}.case.json"
        path.write_text(json.dumps(case), encoding="utf-8")
        trace = folder / f"other-{key}.trace.json"
        out = run([str(adapter), "trace", "--assets", str(ASSETS), str(path), str(trace)])
        print(f"identity control (case model.{key} changed): exit {out.returncode} {out.stderr.strip()}")
        require(out.returncode == 2 and not trace.exists() and not out.stdout.strip(),
                f"the adapter did not refuse a case with another model.{key}")
    print("PASS: cases with another model identity are refused without a trace")


def check_agreement(adapter: Path) -> dict:
    out = run([str(adapter), "agree", "--assets", str(ASSETS)])
    require(out.returncode == 0, f"the adapter's agree command exited with {out.returncode}: {out.stderr.strip()}")
    summary = one_line(out, "the adapter's agree command")
    print(f"agree: {out.stdout.strip()}")
    require(summary.get("stars") == AGREEMENT_STARS, f"agree compared {summary.get('stars')} stars")
    print(f"PASS: KN2.9 agreement, {summary['stars']} stars and {summary['primitives']} primitives "
          f"in {summary['seconds']} s")
    return summary


def main() -> int:
    cargo = shutil.which("cargo")
    if cargo is None:
        print("FAIL: cargo is not on PATH, so the kernel oracle check did not run")
        return 1
    print(version(cargo))
    print(version(shutil.which("rustc") or "rustc"), flush=True)
    start = time.monotonic()
    try:
        cases = verify_inputs()
        adapter = build(cargo)
        with tempfile.TemporaryDirectory(prefix="c600-kernel-oracle-") as name:
            folder = Path(name)
            traces, summary = check_traces(adapter, cases, folder)
            check_tampered_trace(cases, traces, folder)
            check_identity_refusal(adapter, cases, folder)
        agreement = check_agreement(adapter)
    except (Failed, OSError, KeyError, ValueError) as error:
        print(f"FAIL: {error}")
        return 1
    print(f"timings (release): model load {summary['load_seconds']} s, oracle run {summary['total_seconds']} s, "
          f"agreement {agreement['seconds']} s; whole check {time.monotonic() - start:.1f} s")
    print("PASS: the kernel replays O01 to O07 exactly through its API, and the star cache agrees with replay")
    return 0


if __name__ == "__main__":
    sys.exit(main())
