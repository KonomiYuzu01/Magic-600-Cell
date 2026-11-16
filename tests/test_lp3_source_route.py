"""Replay the frozen oracle cases O01 to O07 on the LP3 source route (`rust/c600-tools`).

The check builds `c600-lp3-oracle` (`rust/c600-command`) and `c600-lp3-source` (`rust/c600-tools`) with
`cargo build --release --locked --offline` in rust/, checks the seven tracked case files against MANIFEST-oracle.json
and runs every case on the command route (`c600-lp3-oracle --route command`) and on the source route. On the source
route the runner expands each step's recipe to its literal word through the kernel, outside any program. The built-in
600-cell-Full tool, a 600 Cell Script program, then runs on `c600-run` in the solving profile with that word as its
pinned absolute argument, and its one proposal reaches the command layer through the host bridge as the runtime's
`operation.preview` invocation. The shell presents the built preview and commits it with the key route's commit chord;
undo and redo follow the command route's schedule.

`tools/oracle/compare.py` must report every source-route trace equal with no omitted fields (3,201 steps), and each
source-route trace must be byte-identical to the command route's. The receipts must agree step by step on status, job,
error code and revision after; they may differ only in command ID, caller and request digest. The preview-ready result
digest covers the draft's digest, so it must equal the command route's once rebased onto the case recipe's draft. A
recipe with no literal word (six steps of O07) makes no invocation on the source route, and the command route's receipt
for it is the kernel's refusal of the recipe. The literal digest the tool proposed must be the trace's expansion digest,
and a single word's literal digest is the digest of its moves. A negative control negates one letter of one literal word
before the tool runs: compare.py must then fail at that step.

Standard library only. A missing Rust toolchain fails the check: a test that did not run is no pass (PHS.63). A pass is
headless source and fixture evidence for the source route's draft, not Windows/DirectX, input, long-session or
performance evidence; the timings it prints are informational.
"""
from __future__ import annotations

import gzip
import hashlib
import json
import shutil
import struct
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
ORACLE = ("c600-command", "c600-lp3-oracle")
SOURCE = ("c600-tools", "c600-lp3-source")
CANDIDATE = "c600-command-lp3-loop"
STEPS = {"O01": 2400, "O02": 500, "O03": 210, "O04": 25, "O05": 50, "O06": 6, "O07": 10}
INDEPENDENT = {"O01", "O07"}
TOTAL_STEPS = 3201
# The steps whose recipe has no literal word: the kernel refuses the recipe, and it is not one word of integer moves.
NO_LITERAL = {("O07", index) for index in (3, 4, 5, 6, 7, 9)}
TAMPERED_CASE, TAMPERED_INDEX = "O06", 3
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


def read_recipes(cases: list[Path]) -> dict[str, dict[int, object]]:
    """Each case's recipes by step index."""
    recipes = {}
    for case in cases:
        document = json.loads(gzip.decompress(case.read_bytes()))
        recipes[document["case_id"]] = {step["index"]: step["recipe"] for step in document["steps"]}
    return recipes


def build(cargo: str, package: str, binary: str) -> Path:
    """Builds one runner in release and returns the executable cargo reports."""
    command = [cargo, "build", "--release", "--locked", "--offline", "-p", package, "--bin", binary,
               "--message-format=json-render-diagnostics"]
    start = time.monotonic()
    try:
        out = subprocess.run(command, cwd=WORKSPACE, stdout=subprocess.PIPE, text=True,
                             timeout=BUILD_LIMIT_SECONDS)
    except subprocess.TimeoutExpired:
        raise Failed(f"cargo build ran over {BUILD_LIMIT_SECONDS} s") from None
    require(out.returncode == 0, f"cargo build --release --locked --offline -p {package} exited with {out.returncode}")
    executable = None
    for line in out.stdout.splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if (message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == binary
                and message.get("executable")):
            executable = Path(message["executable"])
    require(executable is not None and executable.is_file(), f"cargo reported no {binary} executable")
    print(f"build: cargo build --release --locked --offline -p {package} --bin {binary} "
          f"({time.monotonic() - start:.1f} s)")
    return executable


def compare(case: Path, trace: Path) -> tuple[int, dict, str]:
    out = run([sys.executable, str(COMPARE), str(case), str(trace)])
    line = out.stdout.strip()
    return out.returncode, one_line(out, "compare.py"), line


def trace_route(runner: Path, cases: list[Path], folder: Path, extra: list[str]) -> tuple[dict[str, Path], Path, dict]:
    """Runs every case through one runner, writing a trace per case and the receipts file."""
    folder.mkdir()
    traces = {case.name[:3]: folder / f"{case.name[:3]}.trace.json" for case in cases}
    receipts = folder / "receipts.json"
    command = [str(runner), "trace", "--assets", str(ASSETS), *extra, "--receipts", str(receipts)]
    for case in cases:
        command += [str(case), str(traces[case.name[:3]])]
    start = time.monotonic()
    out = run(command)
    wall = time.monotonic() - start
    require(out.returncode == 0, f"{runner.stem} exited with {out.returncode}: {out.stderr.strip()}")
    summary = one_line(out, runner.stem)
    require(summary.get("candidate") == CANDIDATE, f"{runner.stem}'s summary names another candidate")
    print(f"{runner.stem}: model load {summary['load_seconds']} s, all seven cases {summary['total_seconds']} s "
          f"in the runner, {wall:.1f} s wall")
    return traces, receipts, summary


def check_source_traces(cases: list[Path], traces: dict[str, Path], summary: dict) -> dict[str, dict]:
    """compare.py reports every source-route trace equal, and the runner's counts match the traces."""
    require(summary.get("route") == "source" and summary.get("host") == "in-process",
            "the source route's summary names another route or host")
    require(str(summary.get("tools", "")).startswith("c600-tools "), "the source route's summary has no tools build")
    build_string = summary.get("build", "")
    counts = {c["case_id"]: c for c in summary["cases"]}
    loaded = {}
    steps = 0
    for case in cases:
        name = case.name[:3]
        trace = json.loads(traces[name].read_text(encoding="utf-8"))
        require(trace["candidate"] == {"name": CANDIDATE, "build": build_string}, f"{name}: trace candidate differs")
        code, report, line = compare(case, traces[name])
        require(code == 0, f"compare.py exited with {code} for {name} on the source route: {line}")
        require(report.get("status") == "equal" and report.get("omitted_fields") == []
                and report.get("case_id") == name and report.get("steps") == STEPS[name],
                f"compare.py did not report {name} equal with every field on the source route")
        steps += report["steps"]
        accepted = sum(1 for step in trace["steps"] if step["accepted"])
        expected = (accepted, accepted, 0) if name in INDEPENDENT else (accepted, accepted, accepted)
        got = (counts[name]["committed"], counts[name]["undone"], counts[name]["redone"])
        require(got == expected, f"{name} on the source route: committed, undone, redone {got}, expected {expected}")
        runs = STEPS[name] - sum(1 for case_id, _ in NO_LITERAL if case_id == name)
        require(counts[name]["runs"] == runs, f"{name}: the tool ran {counts[name]['runs']} times instead of {runs}")
        print(f"  {name}: equal, {report['steps']} steps; {runs} tool runs; committed {got[0]}, undone {got[1]}, "
              f"redone {got[2]}; {counts[name]['receipts']} receipts")
        loaded[name] = trace
    require(steps == TOTAL_STEPS, f"{steps} steps compared on the source route instead of {TOTAL_STEPS}")
    print(f"PASS: source route, all seven cases equal, {steps} steps, no omitted fields")
    return loaded


def check_traces_identical(command: dict[str, Path], source: dict[str, Path], command_summary: dict,
                           source_summary: dict) -> None:
    require(command_summary.get("build") == source_summary.get("build"),
            "the two runners report different command-layer builds")
    for name in STEPS:
        require(command[name].read_bytes() == source[name].read_bytes(),
                f"{name}: the source route's trace differs from the command route's")
    print("PASS: the source route's traces are byte-identical to the command route's")


def single_word_literal(recipe: object) -> str | None:
    """The digest of a single word's moves packed as little-endian 32-bit integers; None for any other recipe."""
    if not (isinstance(recipe, list) and len(recipe) == 1 and isinstance(recipe[0], dict)
            and recipe[0].get("kind") == "word"):
        return None
    moves = recipe[0].get("moves")
    if not (isinstance(moves, list) and moves and all(type(m) is int and -2**31 <= m < 2**31 for m in moves)):
        return None
    return hashlib.sha256(struct.pack(f"<{len(moves)}i", *moves)).hexdigest()


def draft_digest(recipe: object) -> str:
    """The digest of the draft `{"recipe": recipe}` in canonical JSON, for a recipe of integers and strings."""
    text = json.dumps({"recipe": recipe}, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def check_receipts(command_file: Path, source_file: Path, traces: dict[str, dict],
                   recipes: dict[str, dict[int, object]]) -> None:
    """The two routes' receipts agree step by step but for command ID, caller and request digest."""
    command = {c["case_id"]: c["receipts"] for c in json.loads(command_file.read_text(encoding="utf-8"))["cases"]}
    source = {c["case_id"]: c for c in json.loads(source_file.read_text(encoding="utf-8"))["cases"]}
    require(set(command) == set(source) == set(STEPS), "the receipts files name other cases")
    pairs = rebased = withheld = requests = 0
    for name in STEPS:
        a, record_case = command[name], source[name]
        b, callers = record_case["receipts"], record_case["callers"]
        require(len(b) == len(callers), f"{name}: the source route lists {len(b)} receipts and {len(callers)} callers")
        steps = {step["index"]: step for step in traces[name]["steps"]}
        records = record_case["steps"]
        require([r["index"] for r in records] == sorted(steps), f"{name}: the step records differ from the trace")
        ia = ib = 0
        for record in records:
            index = record["index"]
            step, recipe, run = steps[index], recipes[name][index], f"{name}-{index}"
            where = f"{name} step {index}"
            if step["accepted"]:
                count = 4 if name in INDEPENDENT else 3
            elif step["error"] == "protected":
                count = 3
            else:
                require(step["error"] == "invalid-input", f"{where}: unexpected error {step['error']}")
                count = 1
            if (name, index) in NO_LITERAL:
                require(record["literal"] is None and record["receipts"] == 0 and count == 1,
                        f"{where}: a recipe with no literal word reached the command layer on the source route")
                refusal = record["expansion_refusal"]
                require(isinstance(refusal, dict) and str(refusal.get("code", "")).startswith("KN2:"),
                        f"{where}: no kernel refusal recorded for the recipe")
                require(a[ia]["status"] == {"name": "refused", "error": refusal} and a[ia]["job"] is None,
                        f"{where}: the command route's refusal is not the kernel's refusal of the recipe")
                ia += 1
                withheld += 1
                continue
            require(record["literal"] is not None and record["expansion_refusal"] is None,
                    f"{where}: the tool proposed no literal word")
            require(record["receipts"] == count, f"{where}: {record['receipts']} receipts instead of {count}")
            literal, single = record["literal"], single_word_literal(recipe)
            require(step["expansion_sha256"] is not None or single is not None, f"{where}: no literal to check")
            require(step["expansion_sha256"] in (None, literal), f"{where}: the literal is not the trace's expansion")
            require(single in (None, literal), f"{where}: the literal is not the digest of the word's moves")
            letters = step["primitive_count"] if step["primitive_count"] is not None else len(recipe[0]["moves"])
            require(record["letters"] == letters, f"{where}: {record['letters']} letters instead of {letters}")
            require(record["case_draft"] == draft_digest(recipe), f"{where}: the case draft's digest differs")
            for k in range(count):
                ra, rb, caller = a[ia + k], b[ib + k], callers[ib + k]
                expected = {"caller": "runtime", "run": run} if k < 2 else {"caller": "shell"}
                require(caller == expected, f"{where}, receipt {k}: caller {caller}, expected {expected}")
                require(k >= 2 or rb["command"] == f"{run}/1", f"{where}, receipt {k}: command {rb['command']}")
                require(ra["job"] == rb["job"], f"{where}, receipt {k}: the jobs differ")
                sa, sb = ra["status"], rb["status"]
                if k == 1:
                    require(sa.get("name") == "completed" and sa.get("kind") == "preview_ready",
                            f"{where}: the second receipt is not the preview-ready receipt")
                    require({key: v for key, v in sa.items() if key != "result"}
                            == {key: v for key, v in sb.items() if key != "result"},
                            f"{where}: the preview-ready receipts differ beyond the result")
                    require(sa["result"]["job"] == sb["result"]["job"], f"{where}: the result jobs differ")
                    require(sa["result"]["digest"] == record["rebased_result"],
                            f"{where}: the rebased result digest differs from the command route's")
                    if record["draft"] == record["case_draft"]:
                        require(sb["result"]["digest"] == sa["result"]["digest"], f"{where}: the result digests differ")
                    else:
                        rebased += 1
                else:
                    require(sa == sb, f"{where}, receipt {k}: the statuses differ: {sa} and {sb}")
                requests += ra["request"] != rb["request"]
                pairs += 1
            ia += count
            ib += count
        tail = record_case["tail"]
        accepted = sum(1 for step in steps.values() if step["accepted"])
        require(tail == (0 if name in INDEPENDENT else 2 * accepted), f"{name}: {tail} undo and redo receipts")
        require(len(a) - ia == tail and len(b) - ib == tail, f"{name}: the routes' receipt counts differ")
        for k in range(tail):
            ra, rb = a[ia + k], b[ib + k]
            require(callers[ib + k] == {"caller": "shell"}, f"{name}, tail receipt {k}: not the shell's")
            require(ra["job"] == rb["job"] and ra["status"] == rb["status"], f"{name}, tail receipt {k}: differs")
            requests += ra["request"] != rb["request"]
            pairs += 1
    print(f"PASS: the routes' receipts agree on status, job, error code and revision after ({pairs} pairs; "
          f"{rebased} preview-ready digests compared after rebasing onto the case draft; {requests} request digests "
          f"differ; {withheld} recipes with no literal word refused by the kernel on both routes)")


def check_tampered_literal(runner: Path, cases: list[Path], folder: Path) -> None:
    case = next(case for case in cases if case.name.startswith(TAMPERED_CASE))
    folder.mkdir()
    trace = folder / f"{TAMPERED_CASE}.trace.json"
    out = run([str(runner), "trace", "--assets", str(ASSETS), "--tamper", f"{TAMPERED_CASE}:{TAMPERED_INDEX}",
               str(case), str(trace)])
    require(out.returncode == 0, f"the tampered run exited with {out.returncode}: {out.stderr.strip()}")
    code, report, line = compare(case, trace)
    print(f"negative control ({TAMPERED_CASE} step {TAMPERED_INDEX}: the first letter of the literal word negated "
          f"before the tool runs): exit {code} {line}")
    fields = [difference["field"] for difference in report.get("differences", [])]
    require(code == 1 and report.get("status") == "mismatch" and report.get("index") == TAMPERED_INDEX
            and "expansion_sha256" in fields, "compare.py did not report the tampered literal at its step")
    print(f"PASS: the tampered literal fails at index {TAMPERED_INDEX} ({', '.join(fields)})")


def main() -> int:
    cargo = shutil.which("cargo")
    if cargo is None:
        print("FAIL: cargo is not on PATH, so the LP3 source-route check did not run")
        return 1
    print(version(cargo))
    print(version(shutil.which("rustc") or "rustc"), flush=True)
    start = time.monotonic()
    try:
        cases = verify_inputs()
        recipes = read_recipes(cases)
        oracle = build(cargo, *ORACLE)
        source = build(cargo, *SOURCE)
        with tempfile.TemporaryDirectory(prefix="c600-lp3-source-") as name:
            folder = Path(name)
            command_traces, command_receipts, command_summary = trace_route(
                oracle, cases, folder / "command", ["--route", "command"])
            source_traces, source_receipts, source_summary = trace_route(source, cases, folder / "source", [])
            traces = check_source_traces(cases, source_traces, source_summary)
            check_traces_identical(command_traces, source_traces, command_summary, source_summary)
            check_receipts(command_receipts, source_receipts, traces, recipes)
            check_tampered_literal(source, cases, folder / "tampered")
    except (Failed, OSError, KeyError, ValueError, TypeError, IndexError) as error:
        print(f"FAIL: {error}")
        return 1
    print(f"whole check {time.monotonic() - start:.1f} s")
    print("PASS: O01 to O07 replay exactly on the source route: each proposal comes from the 600-cell-Full tool's "
          "program through the host bridge, and the traces and receipts agree with the command route")
    return 0


if __name__ == "__main__":
    sys.exit(main())
