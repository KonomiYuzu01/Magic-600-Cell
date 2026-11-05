"""One isolated LX-1 worker. Counts and timings use separate invocations."""

import argparse
import builtins
from collections import Counter
from copy import deepcopy
import json
import sys
import time
from types import CodeType

sys.dont_write_bytecode = True

from support import (CANDIDATES, CORPUS, FORMS, HERE, MODES, PART_B, canonical,
                     digest, read, require)

sys.path.insert(0, str(PART_B))
from harness.common import canonical as value_bytes, thaw
from harness.classify import classify
from harness.host import Host
from harness.meter import Stop
from harness.policy import STATE_ROWS
from harness.runtime import RunContext
import variants


EXECUTED_FORMS = {
    "rt1-own": "typed IR instructions dispatched by rt1_vm.Machine",
    "rt2-py": "CPython 3.15.1 opcode events in rendered code objects",
    "rt2-node": "JavaScript on unmodified V8; instruction count not available here",
    "rt3-wasm": "generated WebAssembly; instruction count not available here",
    "rt4-pyhost": "CPython 3.15.1 opcode events in rendered code objects",
}
COARSE_UNITS = {
    "rt2-node": "executed JavaScript domain-helper callbacks",
    "rt3-wasm": "executed Wasm-to-host import calls",
}


class Totals:
    def __init__(self, candidate):
        self.candidate = candidate
        self.steps = 0 if candidate in ("rt1-own", "rt2-py", "rt4-pyhost") else None
        self.continuation_calls = self.continuation_weight = 0
        self.rows = {r: dict(calls=0, arguments_bytes=0, results_bytes=0)
                     for r in read(CORPUS / "fixture.json")["rows"]}
        self.sent = self.received = 0
        self.callbacks = Counter()

    def attach(self, ctx):
        kernel_call, continuation, executed = ctx.kernel_call, ctx.meter.continuation, ctx.executed

        def call(row, values, path):
            args = [v.data for v in values]
            if row == "preview_outcome":
                args = [ctx.host.preview_outcome(values[0])]
            result = kernel_call(row, values, path)
            counts = self.rows[row]
            counts["calls"] += 1
            counts["arguments_bytes"] += len(value_bytes(args))
            counts["results_bytes"] += len(value_bytes(result))
            return result

        def meter(path, weight=1, loop_index=None):
            self.continuation_calls += 1
            self.continuation_weight += weight
            return continuation(path, weight, loop_index)

        def dispatch(path, data):
            if self.candidate == "rt1-own":
                self.steps += 1
            return executed(path, data)

        ctx.kernel_call, ctx.meter.continuation, ctx.executed = call, meter, dispatch

    def payload(self):
        node = self.candidate in COARSE_UNITS
        return {
            "steps": self.steps,
            "step_counter": ("ctx.executed at typed IR dispatch" if self.candidate == "rt1-own" else
                             "sys.monitoring INSTRUCTION events; rendered objects only; excludes RESUME/CACHE" if self.steps is not None else
                             "not available here: built-in unmodified V8 exposes no exact instruction hook"),
            "coarse_unit": {"name": COARSE_UNITS[self.candidate], "count": sum(self.callbacks.values()),
                            "by_name": dict(sorted(self.callbacks.items()))} if node else None,
            "kernel_calls_by_row": self.rows,
            "kernel_calls": sum(r["calls"] for r in self.rows.values()),
            "canonical_arguments_bytes": sum(r["arguments_bytes"] for r in self.rows.values()),
            "canonical_results_bytes": sum(r["results_bytes"] for r in self.rows.values()),
            "actual_bridge": {"python_to_node_bytes": self.sent, "node_to_python_bytes": self.received,
                              "total_bytes": self.sent + self.received} if node else None,
            "bench_continuation_calls": self.continuation_calls,
            "bench_continuation_weight": self.continuation_weight,
            "projection": {"kind": "projection, never measurement", "snapshot_calls": self.rows["snapshot"]["calls"],
                           "label_bytes_each": 1039200, "label_bytes": self.rows["snapshot"]["calls"] * 1039200,
                           "pose_bundle_bytes_each": 869952, "pose_bundle_calls": None, "pose_bundle_bytes": None,
                           "pose_reason": "No pose-transfer row in the frozen corpus; no row-to-pose mapping assumed"}}


class PipeTap:
    """Count bytes actually accepted/read by the existing binary stdio streams."""

    def __init__(self, stream, active, outgoing):
        self.stream, self.active, self.outgoing = stream, active, outgoing

    def __getattr__(self, name):
        return getattr(self.stream, name)

    def write(self, data):
        n = self.stream.write(data)
        self.active[0].sent += n
        return n

    def readline(self, *args):
        data = self.stream.readline(*args)
        self.active[0].received += len(data)
        if data:
            message = json.loads(data)
            if message.get("event") == "call":
                self.active[0].callbacks[message["name"]] += 1
            elif "method" in message:
                self.active[0].callbacks[message["method"]] += 1
        return data

    def __iter__(self):
        return self

    def __next__(self):
        line = self.readline()
        if not line:
            raise StopIteration
        return line


def tap_bridge(adapter, active):
    original = adapter.subprocess.Popen

    def popen(*args, **kwargs):
        process = original(*args, **kwargs)
        process.stdin = PipeTap(process.stdin, active, True)
        process.stdout = PipeTap(process.stdout, active, False)
        return process

    adapter.subprocess.Popen = popen


def monitor_bytecode(adapter, active):
    tool = 5  # A fresh isolated worker owns this otherwise unused local tool ID.
    sys.monitoring.use_tool_id(tool, "lx1-rendered-bytecode")

    def instruction(code, offset):
        active[0].steps += 1

    def watch(code):
        sys.monitoring.set_local_events(tool, code, sys.monitoring.events.INSTRUCTION)
        for item in code.co_consts:
            if isinstance(item, CodeType):
                watch(item)

    def compile_rendered(*args, **kwargs):
        code = builtins.compile(*args, **kwargs)
        require(code.co_filename in ("rt2-approved.py", "candidates/rt4-pyhost/programs.py"),
                "unexpected rendered code object")
        watch(code)
        return code

    sys.monitoring.register_callback(tool, sys.monitoring.events.INSTRUCTION, instruction)
    adapter.compile = compile_rendered
    return tool


class LabelFreeContext(RunContext):
    def kernel_call(self, row, values, path):
        if row in STATE_ROWS:
            self.host.read_seen = True
        return super().kernel_call(row, values, path)

    def check(self, operation, values=(), *, labels, read_state, **facts):
        return super().check(operation, values, labels=(), read_state=read_state, **facts)


def result_payload(record):
    exact, observer = record["exact"], record["observer"]
    effects = []
    for original in exact["traces"]:
        event = {k: v for k, v in original.items() if k != "provenance"}
        if event["kind"] == "pick":
            event["record"] = {k: v for k, v in event["record"].items() if k != "producer_digest"}
        effects.append(event)
    after = observer["after"]
    return {**{k: exact[k] for k in ("outcome", "code", "value", "words", "certificates",
                                    "checker_reports", "coverage", "stop")},
            "display": record["display"], "effects": effects, "session": after["session"],
            "pending": {k: v for k, v in after["pending_preview"].items() if k != "provenance"},
            "workspace": {k: {"type": v["type"], "value": v["value"]} for k, v in after["workspace"].items()}}


class Experiment:
    def __init__(self, candidate, mode, counting):
        self.candidate, self.mode, self.counting = candidate, mode, counting
        self.folder = PART_B / "candidates" / candidate
        self.form = FORMS[mode]
        self.variant = "source_bound" if candidate in ("rt1-own", "rt3-wasm") else "approved_subset_byte_bound"
        declaration = read(self.folder / "candidate.json")
        cell = next(c for c in declaration["cells"] if c["form"] == self.form and c["variant"] == self.variant)
        self.not_run = cell["reason"] if cell["status"] == "not_run" else None
        self.bindings = read(self.folder / "bindings.json")
        self.local_identity = None
        if mode == "labels_off":
            self.local_identity = digest(read(HERE / "INPUTS.json")["local_files"])
            self.bindings["execution_contract"]["lx1_labels_off_variant"] = self.local_identity
        self.cases = {c["id"]: c for c in read(CORPUS / "allowed-tools.json")["cases"]}
        self.workloads = read(HERE / "workloads.json")["workloads"]
        self.summary = next(c for c in read(self.folder / "summary.json")["cells"]
                            if c["form"] == self.form and c["variant"] == self.variant)
        self.adapter = None if self.not_run else variants.load(candidate, mode)
        self.host_class = variants.no_label_host() if mode == "labels_off" else Host
        self.context_class = LabelFreeContext if mode == "labels_off" else RunContext
        self.active = [None]
        self.monitor = None
        if counting and not self.not_run and candidate in COARSE_UNITS:
            tap_bridge(self.adapter, self.active)
        if counting and not self.not_run and candidate in ("rt2-py", "rt4-pyhost"):
            self.monitor = monitor_bytecode(self.adapter, self.active)

    def base(self, workload):
        return {"candidate": self.candidate, "mode": self.mode, "workload": workload,
                "executed_form": EXECUTED_FORMS[self.candidate],
                "status": "not_run" if self.not_run else
                          "incomplete_wrapper_only_attempt" if self.candidate == "rt4-pyhost" and self.mode == "run_time" else "ran",
                "reason": self.not_run, "local_variant_sha256": self.local_identity}

    def execute_workload(self, workload, timed=False):
        totals = Totals(self.candidate) if self.counting else None
        self.active[0] = totals
        contexts = []
        started = time.perf_counter_ns() if timed else None
        try:
            for item in self.workloads[workload]:
                case = self.cases[item["case"]]
                program = deepcopy(case["program"])
                # The selected corpus programs each have one invocation. Keeping
                # that frozen condition avoids moving result hashing inside timing.
                require(len(program["runs"]) == 1, "selected workload gained a sequential invocation")
                host = self.host_class(program, self.form, self.bindings["source_files"])
                ctx = self.context_class(case, "allowed-tools", program["runs"][0], host,
                                         self.bindings, self.form, self.variant)
                if totals:
                    totals.attach(ctx)
                try:
                    value, stop = self.adapter.execute(ctx), None
                except Stop as stopped:
                    value, stop = None, stopped
                contexts.append((case, ctx, value, stop))
        finally:
            elapsed = time.perf_counter_ns() - started if timed else None
        records, result_hashes = [], []
        for case, ctx, value, stop in contexts:
            record = ctx.finish(value, stop)
            classification = classify(case, "allowed-tools", record)
            require(classification["status"] == "allowed_ok", self.candidate + "/" + self.mode + "/" + case["id"] + " outcome differs")
            require(all(e["inside_approval"] for e in ctx.executions), "execution outside approved source")
            key = case["id"] + "/" + ctx.run["id"] + "/default"
            if self.mode != "labels_off":
                expected = self.summary["hashes"][key]
                require(expected and all(v["sha256"] == record["sha256"] and
                        v["run_key"] == record["exact"]["input_identities"]["run_key"] for v in expected),
                        self.candidate + "/" + self.mode + "/" + key + " stored summary differs")
                require(self.summary["outcomes"]["allowed-tools"][case["id"]] == ["allowed_ok"], "stored outcome differs")
            else:
                require(all(not block["labels"] for block in ctx.host.blocks.values()), "labels-off retained a label")
            hashed = digest(result_payload(record))
            result_hashes.append({"case": case["id"], "sha256": hashed})
            records.append({"case": case["id"], "outcome": classification["status"],
                            "result_payload_sha256": hashed, "exact_sha256": record["sha256"],
                            "run_key": record["exact"]["input_identities"]["run_key"],
                            "local_wasm_sha256": getattr(ctx, "local_lowering_sha256", None),
                            "summary_comparison": "not comparable: local variant; compare result via static cell" if self.mode == "labels_off"
                                                  else "exact hash and run key match every stored repeat"})
        if totals:
            require(totals.continuation_weight == sum(c.meter.used["steps"] for _, c, _, _ in contexts),
                    "bench continuation accounting differs")
            if self.candidate == "rt1-own":
                require(totals.steps == sum(len(c.executions) for _, c, _, _ in contexts), "typed IR accounting differs")
            if self.candidate in ("rt2-py", "rt4-pyhost"):
                require(totals.steps > 0, "no rendered bytecode events counted")
        return {**self.base(workload), "result_payload_sha256": digest(result_hashes),
                "programs": records, **(totals.payload() if totals else {})}, elapsed

    def close(self):
        if self.monitor is not None:
            sys.monitoring.register_callback(self.monitor, sys.monitoring.events.INSTRUCTION, None)
            sys.monitoring.free_tool_id(self.monitor)
        if self.adapter is not None and self.candidate == "rt2-node" and self.adapter.CONNECTION is not None:
            self.adapter.CONNECTION.close()
            self.adapter.atexit.unregister(self.adapter.CONNECTION.close)
            self.adapter.CONNECTION = None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", choices=CANDIDATES)
    parser.add_argument("mode", choices=MODES)
    parser.add_argument("action", choices=("counts", "timings"))
    args = parser.parse_args()
    experiment = Experiment(args.candidate, args.mode, args.action == "counts")
    output = []
    try:
        for workload in experiment.workloads:
            if experiment.not_run:
                output.append({**experiment.base(workload), "steps": None, "kernel_calls": None,
                               "result_payload_sha256": None})
            elif args.action == "counts":
                output.append(experiment.execute_workload(workload)[0])
            else:
                smoke, elapsed = experiment.execute_workload(workload, timed=True)
                cell = {**experiment.base(workload), "smoke_ns": elapsed, "series_ns": []}
                for series in range(3):
                    samples = []
                    for _ in range(100):
                        current, elapsed = experiment.execute_workload(workload, timed=True)
                        require(current["result_payload_sha256"] == smoke["result_payload_sha256"], "timed outcome differs")
                        samples.append(elapsed)
                    cell["series_ns"].append(samples)
                    print(args.candidate, args.mode, workload, "series", series + 1, "recorded", file=sys.stderr, flush=True)
                output.append(cell)
    finally:
        experiment.close()
    sys.stdout.buffer.write(canonical(output) + b"\n")


if __name__ == "__main__":
    main()
