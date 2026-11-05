"""Fresh child per case, observable crashes/timeouts, complete key enumeration."""

import argparse
import importlib.util
import hashlib
import json
import queue
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor

from . import bindings as binding_api
from .classify import classify, validate_record
from .common import ROOT, canonical, cases, digest, freeze, load, write_json
from .host import Host
from .meter import Stop
from .result import HarnessFailure, require
from .runtime import RunContext

# The frozen short loop has no specified way to remain live until a 1 ms wall
# deadline. Inventing delay or deterministic deadline injection changes the case.
CORPUS_ISSUES = {"B10": "corpus_issue"}


def wire(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":"), allow_nan=False).encode("utf-8") + b"\n"


def not_run(ctx, reason):
    return ctx.finish(stop=Stop("not_run", None, reason, ctx.run["id"] + "/admission"), reason=reason)


def worker(request):
    folder = binding_api.within_root(ROOT / request["candidate"])
    manifest, bindings = binding_api.candidate(folder)
    group, case = next((g, c) for g, c in cases() if c["id"] == request["case_id"])
    cell = next(c for c in manifest["cells"] if c["form"] == request["form"] and c["variant"] == request["variant"])
    spec = importlib.util.spec_from_file_location("lx3_candidate", folder / manifest["adapter"])
    adapter = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(adapter)
    program = json.loads(canonical(case["program"]))
    if case.get("kind") == "dependency_substitution":
        program["modules"]["constant"]["body"][0]["value"]["value"] = 2
    host = Host(program, request["form"], bindings["source_files"])
    results = []

    def loss_callback(ctx):
        ctx.coverage = {"examined": None,
                        "total": list(range(len(ctx.run["body"][0]["items"]["value"]))), "complete": False}
        stop = Stop("error", "KN7:resource_limit.runtime_lost", "runtime_loss", ctx.run["id"] + "/after_admission")
        record = ctx.finish(stop=stop)
        sys.stdout.buffer.write(wire({"event": "runtime_loss_ready", "records": results + [record]}))
        sys.stdout.buffer.flush()
        threading.Event().wait()  # The parent kills this actual child after admission.

    shared_cache = {}
    barrier = threading.Barrier(2) if request["host_variant"] == "shared" else None

    def execute_run(run, active_host, previous):
        ctx = RunContext(case, group, run, active_host, bindings, request["form"], request["variant"],
                         previous, request["host_variant"], loss_callback)
        if cell["status"] == "not_run":
            return not_run(ctx, cell["reason"])
        if group == "approval" and request["variant"] not in case["variants"]:
            return not_run(ctx, "variant_not_applicable")
        if request["host_variant"] != "default":
            key = digest({"fixture": digest(ctx.kernel.fixture), "row": "counts", "args": []})
            if request["host_variant"] in ("warm", "shared", "relocated"):
                if request["host_variant"] == "shared":
                    # Both independently executing contexts read this same
                    # immutable value object while their invocations overlap.
                    shared_cache.setdefault(key, ctx.kernel.call("counts"))
                    ctx.cache = dict(shared_cache)
                else:
                    value = ctx.kernel.call("counts")
                    ctx.cache[key] = freeze(json.loads(canonical(value))) if request["host_variant"] == "relocated" else value
            elif request["host_variant"] == "poisoned":
                ctx.cache[key] = freeze([99, 99, 99])
        if barrier:
            barrier.wait()
        try:
            value = adapter.execute(ctx)
            record = ctx.finish(value)
        except Stop as stop:
            record = ctx.finish(stop=stop)
        if barrier:
            record["host"]["shared_value_identity"] = ctx.cache[key] is shared_cache[key]
        return record

    for run in program["runs"]:
        previous = results[-1]["sha256"] if results else None
        if barrier:
            with ThreadPoolExecutor(max_workers=2) as pool:
                a = pool.submit(execute_run, run, host, previous)
                other = Host(program, request["form"], bindings["source_files"])
                b = pool.submit(execute_run, run, other, previous)
                record, concurrent = a.result(), b.result()
            require(record["sha256"] == concurrent["sha256"], "shared cache changed an exact result")
            require(record["host"]["shared_value_identity"] and concurrent["host"]["shared_value_identity"], "shared cache was not shared")
        else:
            record = execute_run(run, host, previous)
        results.append(record)
    return {"event": "finished", "records": results}


def launch(command, request, timeout):
    """Read one bounded protocol response; kill and reap on every failure path."""
    started = time.monotonic()
    process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, cwd=ROOT)
    first_line = queue.Queue()
    reader = threading.Thread(target=lambda: first_line.put(process.stdout.readline()), daemon=True)
    diagnostics = {"sha256": None, "planted_data": False}
    markers = [case["plant"]["data"].encode("utf-8") for case in load("planted.json")["cases"]]
    def drain_diagnostics():
        hasher, tail = hashlib.sha256(), b""
        while chunk := process.stderr.read(4096):
            hasher.update(chunk)
            text = tail + chunk
            diagnostics["planted_data"] |= any(marker in text for marker in markers)
            tail = text[-max(map(len, markers)):]
        diagnostics["sha256"] = hasher.hexdigest()
    errors = threading.Thread(target=drain_diagnostics, daemon=True)
    def attach_diagnostics(response):
        errors.join(timeout=1)
        require(diagnostics["sha256"] is not None, "missing diagnostic stream completion")
        for record in response.get("records", []):
            record["host"]["diagnostics"] = dict(diagnostics)
        return response
    try:
        process.stdin.write(wire(request))
        process.stdin.close()
        process.stdin = None
        reader.start()
        errors.start()
        try:
            line = first_line.get(timeout=timeout)
        except queue.Empty:
            return {"failure": "hang"}
        if not line:
            return {"failure": "crash"}
        try:
            response = json.loads(line)
        except (ValueError, UnicodeError):
            raise HarnessFailure("child returned malformed JSON") from None
        if response.get("event") == "runtime_loss_ready":
            process.kill()
            process.wait(timeout=timeout)
            for record in response["records"]:
                if record["exact"]["code"] == "KN7:resource_limit.runtime_lost":
                    record["observer"]["runtime_loss_observed"] = True
            return attach_diagnostics(response)
        try:
            remaining = max(0.001, timeout - (time.monotonic() - started))
            process.wait(timeout=remaining)
        except subprocess.TimeoutExpired:
            return {"failure": "hang"}
        require(not process.stdout.read(1), "child returned extra output")
        if process.returncode != 0 or response.get("event") == "crash":
            return {"failure": "crash"}
        require(response.get("event") == "finished", "child omitted result event")
        return attach_diagnostics(response)
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()
        if reader.is_alive():
            reader.join(timeout=1)
        if errors.is_alive():
            errors.join(timeout=1)
        process.stdout.close()
        process.stderr.close()


def run(candidate, out, runs=1, timeout=10, leg="control"):
    from .report import summarize
    require(type(runs) is int and runs >= 1 and timeout > 0, "invalid run count or timeout")
    folder = binding_api.within_root(candidate)
    out = binding_api.within_root(out)
    require(not out.exists(), "output folder already exists")
    manifest, bindings = binding_api.candidate(folder)
    out.mkdir(parents=True)
    rows = []
    command = [sys.executable, "-B", str(ROOT / "run_matrix.py"), "--worker"]
    with (out / "results.jsonl").open("wb") as stream:
        for cell in manifest["cells"]:
            for group, case in cases():
                host_variants = [v["id"] for v in case.get("host", {}).get("variants", [])] or ["default"]
                for host_variant in host_variants:
                    for repeat in range(runs):
                        request = {"candidate": folder.relative_to(ROOT).as_posix(), "case_id": case["id"],
                                   "form": cell["form"], "variant": cell["variant"], "host_variant": host_variant}
                        response = launch(command, request, timeout)
                        if "failure" not in response:
                            require(len(response["records"]) == len(case["program"]["runs"]), "child omitted a sequential key")
                        for i, invocation in enumerate(case["program"]["runs"]):
                            record = None if "failure" in response else response["records"][i]
                            if record is not None:
                                record["repeat"] = repeat
                                validate_record(record)
                            terminal = i == len(case["program"]["runs"]) - 1 or invocation["profile"] == "solving"
                            classification = classify(case, group, record, failure=response.get("failure"),
                                                      host_variant=host_variant, terminal=terminal)
                            row = {**request, "run_id": invocation["id"], "repeat": repeat, "leg": leg,
                                   "classification": classification, "record": record}
                            rows.append(row)
                            stream.write(wire(row))
    summary = summarize(rows, manifest, runs, [leg])
    write_json(out / "summary.json", summary)
    write_json(out / "run.json", {"candidate": manifest, "bindings_sha256": digest(bindings),
                                  "runs": runs, "legs": [leg], "decision": "open"})
    return summary


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--candidate")
    parser.add_argument("--out")
    parser.add_argument("--runs", type=int, default=1)
    parser.add_argument("--timeout", type=float, default=10)
    parser.add_argument("--leg", choices=("control", "Windows", "headless_Linux"), default="control")
    args = parser.parse_args(argv)
    if args.worker:
        try:
            response = worker(json.loads(sys.stdin.buffer.readline()))
        except HarnessFailure:
            raise
        except Exception as error:
            response = {"event": "crash", "exception": type(error).__name__}
        sys.stdout.buffer.write(wire(response))
        return 0
    if not args.candidate or not args.out:
        parser.error("--candidate and --out are required")
    summary = run(ROOT / args.candidate, ROOT / args.out, args.runs, args.timeout, args.leg)
    print("Recorded", summary["row_count"], "key observations; decision open.")
    return 0


def self_test():
    hanging = [sys.executable, "-B", "-c", "import threading; threading.Event().wait()"]
    crashing = [sys.executable, "-B", "-c", "import sys; sys.stdin.readline(); raise SystemExit(7)"]
    assert launch(hanging, {}, 0.15) == {"failure": "hang"}
    assert launch(crashing, {}, 2) == {"failure": "crash"}
    loss = [sys.executable, "-B", "-c", "import sys,threading; sys.stdin.readline(); print('{\"event\":\"runtime_loss_ready\",\"records\":[]}',flush=True); threading.Event().wait()"]
    assert launch(loss, {}, 2)["event"] == "runtime_loss_ready"
    omitted = [sys.executable, "-B", "-c", "print('{}')"]
    try:
        launch(omitted, {}, 2)
    except HarnessFailure:
        pass
    else:
        raise AssertionError("omitted child result accepted")
