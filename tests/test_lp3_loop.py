"""Drive the frozen oracle cases O01 to O07 through the LP3 command layer (`rust/c600-command`) on two routes.

The check builds the loop runner `c600-lp3-oracle` with `cargo build --release --locked --offline` in rust/, checks
the seven tracked case files against MANIFEST-oracle.json and runs every case twice: once invoking the command rows
with absolute arguments (`--route command`) and once pressing the draft keymap's chords (`--route key`). Every step
goes through the command layer: `operation.preview` with the recipe as an inline draft, the preview job, the shell's
presentation of the built preview and `operation.execute` with a token bound to it. Independent cases undo every
commit with `session.undo`; chain cases undo and then redo every commit with `session.undo` and `session.redo`, and
each lands on the state the trace recorded for it, or the runner fails.

`tools/oracle/compare.py` must report every trace equal with no omitted fields (3,201 steps per route). The two routes
must write byte-identical traces and receipts: the key route resolves to the same rows and absolute arguments. The
runner's counts must match the traces: every accepted step is committed once, an independent case undoes each commit,
and a chain case undoes and redoes each one. Two controls follow: a trace with one tampered state hash fails at that
step, and a case that names another model identity is refused without a trace.

Standard library only. A missing Rust toolchain fails the check: a test that did not run is no pass (PHS.63). A pass is
headless source and fixture evidence for the command layer's draft, not Windows/DirectX, input, long-session or
performance evidence; the timings it prints are informational.
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
RUNNER = "c600-lp3-oracle"
CANDIDATE = "c600-command-lp3-loop"
ROUTES = ("command", "key")
STEPS = {"O01": 2400, "O02": 500, "O03": 210, "O04": 25, "O05": 50, "O06": 6, "O07": 10}
INDEPENDENT = {"O01", "O07"}
TOTAL_STEPS = 3201
TAMPERED_CASE, TAMPERED_INDEX = "O06", 2
# Below the CI runner's limit per check (tools/ci/run_headless.py), so a slow step is reported here.
BUILD_LIMIT_SECONDS = 900
RUN_LIMIT_SECONDS = 400


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
    """Builds the runner in release and returns the executable cargo reports."""
    command = [cargo, "build", "--release", "--locked", "--offline", "-p", "c600-command", "--bin", RUNNER,
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
        if (message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == RUNNER
                and message.get("executable")):
            executable = Path(message["executable"])
    require(executable is not None and executable.is_file(), "cargo reported no runner executable")
    print(f"build: cargo build --release --locked --offline -p c600-command --bin {RUNNER} "
          f"({time.monotonic() - start:.1f} s)")
    return executable


def compare(case: Path, trace: Path) -> tuple[int, dict, str]:
    out = run([sys.executable, str(COMPARE), str(case), str(trace)])
    line = out.stdout.strip()
    return out.returncode, one_line(out, "compare.py"), line


def run_route(runner: Path, cases: list[Path], folder: Path, route: str) -> tuple[dict[str, Path], Path, dict]:
    """Runs every case on one route and checks each trace against the oracle and the runner's counts."""
    folder.mkdir()
    traces = {case.name[:3]: folder / f"{case.name[:3]}.trace.json" for case in cases}
    receipts = folder / "receipts.json"
    command = [str(runner), "trace", "--assets", str(ASSETS), "--route", route, "--receipts", str(receipts)]
    for case in cases:
        command += [str(case), str(traces[case.name[:3]])]
    start = time.monotonic()
    out = run(command)
    wall = time.monotonic() - start
    require(out.returncode == 0, f"the {route} route exited with {out.returncode}: {out.stderr.strip()}")
    summary = one_line(out, f"the {route} route")
    require(summary.get("candidate") == CANDIDATE and summary.get("route") == route,
            f"the {route} route's summary names another candidate or route")
    build_string = summary.get("build", "")
    print(f"{route} route: model load {summary['load_seconds']} s, all seven cases {summary['total_seconds']} s "
          f"in the runner, {wall:.1f} s wall")
    counts = {c["case_id"]: c for c in summary["cases"]}
    steps = 0
    for case in cases:
        name = case.name[:3]
        trace = json.loads(traces[name].read_text(encoding="utf-8"))
        require(trace["candidate"] == {"name": CANDIDATE, "build": build_string}, f"{name}: trace candidate differs")
        code, report, line = compare(case, traces[name])
        require(code == 0, f"compare.py exited with {code} for {name} on the {route} route: {line}")
        require(report.get("status") == "equal" and report.get("omitted_fields") == []
                and report.get("case_id") == name and report.get("steps") == STEPS[name],
                f"compare.py did not report {name} equal with every field on the {route} route")
        steps += report["steps"]
        accepted = sum(1 for step in trace["steps"] if step["accepted"])
        expected = (accepted, accepted, 0) if name in INDEPENDENT else (accepted, accepted, accepted)
        got = (counts[name]["committed"], counts[name]["undone"], counts[name]["redone"])
        require(got == expected, f"{name} on the {route} route: committed, undone, redone {got}, expected {expected}")
        print(f"  {name}: equal, {report['steps']} steps; committed {got[0]}, undone {got[1]}, redone {got[2]}; "
              f"{counts[name]['receipts']} receipts")
    require(steps == TOTAL_STEPS, f"{steps} steps compared on the {route} route instead of {TOTAL_STEPS}")
    print(f"PASS: {route} route, all seven cases equal, {steps} steps, no omitted fields")
    return traces, receipts, summary


def check_routes_agree(results: dict[str, tuple[dict[str, Path], Path, dict]]) -> None:
    (traces_a, receipts_a, _), (traces_b, receipts_b, _) = results["command"], results["key"]
    for name in STEPS:
        require(traces_a[name].read_bytes() == traces_b[name].read_bytes(), f"{name}: the routes' traces differ")
    require(receipts_a.read_bytes() == receipts_b.read_bytes(), "the routes' receipts differ")
    total = sum(len(case["receipts"]) for case in json.loads(receipts_a.read_text(encoding="utf-8"))["cases"])
    print(f"PASS: the command and key routes wrote identical traces and identical receipts ({total} receipts)")


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


def check_identity_refusal(runner: Path, cases: list[Path], folder: Path) -> None:
    source = next(case for case in cases if case.name.startswith(TAMPERED_CASE))
    original = json.loads(gzip.decompress(source.read_bytes()))
    for key in ("model_id", "manifest_sha256"):
        case = json.loads(json.dumps(original))
        case["model"][key] = "0" * 64
        path = folder / f"other-{key}.case.json"
        path.write_text(json.dumps(case), encoding="utf-8")
        trace = folder / f"other-{key}.trace.json"
        out = run([str(runner), "trace", "--assets", str(ASSETS), "--route", "command", str(path), str(trace)])
        print(f"identity control (case model.{key} changed): exit {out.returncode} {out.stderr.strip()}")
        require(out.returncode == 2 and not trace.exists() and not out.stdout.strip(),
                f"the runner did not refuse a case with another model.{key}")
    print("PASS: cases with another model identity are refused without a trace")


def main() -> int:
    cargo = shutil.which("cargo")
    if cargo is None:
        print("FAIL: cargo is not on PATH, so the LP3 loop check did not run")
        return 1
    print(version(cargo))
    print(version(shutil.which("rustc") or "rustc"), flush=True)
    start = time.monotonic()
    try:
        cases = verify_inputs()
        runner = build(cargo)
        with tempfile.TemporaryDirectory(prefix="c600-lp3-loop-") as name:
            folder = Path(name)
            results = {route: run_route(runner, cases, folder / route, route) for route in ROUTES}
            check_routes_agree(results)
            check_tampered_trace(cases, results["command"][0], folder)
            check_identity_refusal(runner, cases, folder)
    except (Failed, OSError, KeyError, ValueError) as error:
        print(f"FAIL: {error}")
        return 1
    print(f"whole check {time.monotonic() - start:.1f} s")
    print("PASS: O01 to O07 replay exactly through the command layer on the command and key routes, with undo and "
          "redo landing on the recorded states")
    return 0


if __name__ == "__main__":
    sys.exit(main())
