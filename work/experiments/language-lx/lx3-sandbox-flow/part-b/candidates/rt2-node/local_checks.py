"""Candidate-local checker and process probes; never corpus denominator entries."""

import subprocess
import json
from pathlib import Path
from types import SimpleNamespace

from checker import Rejected, analyze, recognize
from render import render

FOLDER = Path(__file__).resolve().parent


def checks():
    # An unexecuted branch still taints the join. Changing the predicate's value
    # never changes this static fact; this is not executed-path propagation.
    body = [
        {"op": "if", "test": {"type": "Bool", "value": True},
         "then": [{"op": "calc", "as": "choice", "type": "Word", "fn": "identity",
                   "args": [{"type": "Word", "value": ["a"]}]}],
         "else": [{"op": "kernel", "as": "n", "type": "Ints", "row": "nu2", "args": []},
                  {"op": "calc", "as": "choice", "type": "Word", "fn": "combine_corrections",
                   "args": [{"type": "Words", "value": [["a"], ["b"]]}, {"ref": "n"}]}]},
        {"op": "preview", "as": "receipt", "type": "Receipt", "word": {"ref": "choice"},
         "target": {"type": "Target", "value": "c0"}},
    ]
    program = {"inputs": {}, "modules": {}, "runs": [{"id": "r1", "profile": "solving",
               "author": "built_in", "boundary_class": "analysis", "invocation": "command", "body": body}]}
    source, mapping = render(program)
    assert recognize(source)[0] == program and mapping
    plan = analyze(program, ("identity", "concat", "inverse", "index", "field"))["r1"]
    assert plan["knowledge"]["r1/body/1"][0].labels == {"state", "computed"}
    for mutation in (source + b"tick.constructor;", source.replace(b"if (t.truth)", b"if (true)"),
                     source.replace(b"return finish", b"return eval") if b"return finish" in source else b"eval('2')"):
        try:
            recognize(mutation)
        except Rejected:
            pass
        else:
            raise AssertionError("changed JavaScript passed the subset checker")

    observed = subprocess.run(["node", str(FOLDER / "probes.cjs")], cwd=FOLDER,
                              capture_output=True, check=True, timeout=15)
    assert not observed.stderr
    probes = json.loads(observed.stdout)
    for probe in probes:
        try:
            recognize(probe["source"].encode())
        except Rejected:
            probe["checker"] = "rejected_before_execution"
        else:
            raise AssertionError("escape source passed the subset checker")

    # Exercise the actual adapter's typed outcome conversion independently of
    # B10 (corpus issue) and B11 (the bench kills Python before Node startup).
    from adapter import Bridge
    from harness.meter import Stop
    executor = SimpleNamespace(ctx=SimpleNamespace(position="local/no_host_call"))
    bridge = Bridge()
    try:
        try:
            bridge.run({"source": "while (true) {}", "functions": [], "items": {}, "inputs": {}, "run": "r1"}, executor)
        except Stop as stop:
            assert (stop.outcome, stop.code) == ("cut_off", "KN7:resource_limit.safety_stop")
            timeout = {"outcome": stop.outcome, "code": stop.code, "reason": stop.stop["reason"]}
        else:
            raise AssertionError("no-host-call loop was not stopped")
        bridge.process.kill()
        bridge.process.wait(timeout=5)
        try:
            bridge.receive("local/killed_node")
        except Stop as stop:
            assert (stop.outcome, stop.code) == ("error", "KN7:resource_limit.runtime_lost")
            loss = {"outcome": stop.outcome, "code": stop.code, "reason": stop.stop["reason"]}
        else:
            raise AssertionError("Node loss was not observed")
    finally:
        bridge.close()
    return {"decision": "open", "scope": "candidate-local; outside corpus counts", "probes": probes,
            "adapter_no_host_call": timeout, "adapter_node_kill": loss,
            "static_unexecuted_branch_join": "passed", "changed_bytes": "rejected"}
