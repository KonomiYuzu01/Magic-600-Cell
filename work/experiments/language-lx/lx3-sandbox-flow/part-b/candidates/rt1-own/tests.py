"""Small independent mechanism probes; not additions to the frozen corpus."""

import copy
from pathlib import Path
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.append(str(HERE.parents[1]))

from harness.common import cases, sha
from harness.host import Host
from harness.meter import Stop
from harness.policy import GRANTS, Policy
from harness.runtime import RunContext
from render import render
from rt1_check import Checker, walk
from rt1_syntax import SourceError, parse
from rt1_vm import Machine


def lit(kind, value):
    return {"type": kind, "value": value}


def ref(name):
    return {"ref": name}


def run(body, name="r1", profile="solving"):
    return {"id": name, "profile": profile, "author": "built_in",
            "boundary_class": "analysis", "invocation": "command", "body": body}


def calc(name, kind, fn, *args):
    return {"op": "calc", "as": name, "type": kind, "fn": fn, "args": list(args)}


def kernel(name, kind, row, *args):
    return {"op": "kernel", "as": name, "type": kind, "row": row, "args": list(args)}


def preview(word):
    return {"op": "preview", "as": "receipt", "type": "Receipt", "word": word, "target": ref("target")}


def base(body):
    return {"inputs": {"target": {**lit("Target", "c0"), "origin": "manual"}},
            "modules": {}, "runs": [run(body)]}


def execute(program, form):
    source, spans = render(program)
    tree = parse(source)
    host = Host(program, form, {"synthetic": sha(source)})
    checked = Checker(tree, program, host.kernel.fixture["rows"], Policy().data["transparent"]).check(form)
    mapping = {p: {"sha256": sha(source[a:b])} for p, (a, b) in spans.items()}
    bindings = {"effective_grants": {p: list(GRANTS) for p in ("solving", "research", "tool_authoring")},
                "source_map": {"synthetic": mapping}}
    outputs = []
    for invocation in program["runs"]:
        ctx = RunContext({"id": "synthetic"}, "synthetic", invocation, host, bindings, form, "source_bound")
        try:
            result = Machine(ctx, checked).execute()
            outputs.append((None, result, ctx))
        except Stop as stop:
            outputs.append((stop.code, None, ctx))
    return outputs


def self_test():
    # Every branch, including unreachable modules, has the exact UTF-8 span that
    # the parser hands to the executor. Unicode is measured in bytes, not chars.
    for _, case in cases():
        source, spans = render(case["program"])
        program = parse(source)
        parsed = {n.path: n.span for body in list(program.runs.values()) + [m.body for m in program.modules.values()]
                  for n in walk(body)}
        assert parsed == spans
        host = Host(case["program"], "static_labels", {})
        for form in ("capability_separation", "static_labels", "static_runtime_labels"):
            Checker(program, case["program"], host.kernel.fixture["rows"], Policy().data["transparent"]).check(form)

    simple = base([{"op": "return", "value": lit("Text", "e\u0301 / \u00e9 / \u03bd / (ambient)")}])
    assert execute(simple, "static_labels")[0][1].data == simple["runs"][0]["body"][0]["value"]["value"]
    exact = base([calc("n", "Int", "add", lit("Int", 2**130), lit("Int", 3)), {"op": "return", "value": ref("n")}])
    assert execute(exact, "static_runtime_labels")[0][1].data == 2**130 + 3
    bad = [base([{"op": "return", "value": lit("Bool", 1)}]),
           base([{"op": "return", "value": ref("missing")}]),
           base([calc("x", "Bool", "add", lit("Int", 1), lit("Int", 2))])]
    for program in bad:
        try:
            execute(program, "static_labels")
        except SourceError:
            pass
        else:
            raise AssertionError("ill-typed source accepted")
    for source in (b'(rt1 (run "r1" (python "anything")))', b'(rt1) trailing', b'(rt1 (input "x" Int) (input "x" Int))'):
        try:
            parse(source)
        except SourceError:
            pass
        else:
            raise AssertionError("invalid source accepted")

    # Unexecuted tainted arm: static join refuses; own runtime labels can retain
    # the public selected arm. Both analyze the same source and type-check both.
    alternative = base([
        {"op": "if", "test": lit("Bool", True),
         "then": [calc("w", "Word", "identity", lit("Word", ["a"]))],
         "else": [kernel("nu", "Ints", "nu2"), calc("w", "Word", "combine_corrections", lit("Words", [["a"], ["b"]]), ref("nu"))]},
        preview(ref("w")),
    ])
    assert execute(alternative, "static_labels")[0][0] == "LG3:capability.not_granted"
    code, _, ctx = execute(alternative, "static_runtime_labels")[0]
    assert code is None and ctx.host.pending["word"] == ["a"]
    assert execute(alternative, "capability_separation")[0][0] == "LG3:capability.not_granted"

    implicit = base([kernel("bits", "Ints", "bits"), calc("bit", "Int", "index", ref("bits"), lit("Int", 0)),
                     calc("test", "Bool", "eq", ref("bit"), lit("Int", 1)),
                     {"op": "if", "test": ref("test"), "then": [preview(lit("Word", ["a"]))], "else": []}])
    for form in ("static_labels", "static_runtime_labels"):
        code, _, ctx = execute(implicit, form)[0]
        assert code == "LG3:capability.not_granted" and not ctx.host.shell

    # Captures, parameter aliases and context-insensitive call-site joins.
    captured = base([{"op": "call", "as": "w", "type": "Word", "module": "capture", "args": []}, preview(ref("w"))])
    captured["inputs"]["legacy"] = {**lit("Word", ["a"]), "origin": "unknown"}
    captured["modules"]["capture"] = {"parameters": {}, "result": "Word", "body": [{"op": "return", "value": ref("legacy")}]}
    for form in ("static_labels", "static_runtime_labels", "capability_separation"):
        assert execute(captured, form)[0][0] == "PS1:capability.provenance_incompatible"
    calls = base([{"op": "call", "as": "safe", "type": "Int", "module": "copy", "args": [lit("Int", 1)]},
                  kernel("bits", "Ints", "bits"), calc("bit", "Int", "index", ref("bits"), lit("Int", 0)),
                  {"op": "call", "as": "state", "type": "Int", "module": "copy", "args": [ref("bit")]},
                  {"op": "return", "value": ref("safe")}])
    calls["modules"]["copy"] = {"parameters": {"x": "Int"}, "result": "Int", "body": [{"op": "return", "value": ref("x")}]}
    assert execute(calls, "static_labels")[0][1].data == 1
    assert "state" in execute(calls, "static_labels")[0][1].labels
    assert "state" not in execute(calls, "static_runtime_labels")[0][1].labels

    stored = base([])
    stored["runs"] = [run([{"op": "workspace_put", "key": "w", "value": lit("Word", ["a"])}], profile="research"),
                      run([{"op": "workspace_get", "as": "w", "type": "Word", "key": "w"}, preview(ref("w"))], "r2")]
    for form in ("static_labels", "static_runtime_labels", "capability_separation"):
        outputs = execute(copy.deepcopy(stored), form)
        assert outputs[0][0] is None and outputs[1][0] == "LG3:capability.not_granted"

    # Unreachable later writes must not erase the earlier storage label summary.
    bypassed = base([])
    bypassed["inputs"]["legacy"] = {**lit("Word", ["a"]), "origin": "unknown"}
    bypassed["runs"] = [run([{"op": "workspace_put", "key": "w", "value": ref("legacy")},
                             {"op": "stop", "reason": "requested"},
                             {"op": "workspace_put", "key": "w", "value": lit("Word", ["a"])}]),
                        run([{"op": "workspace_get", "as": "w", "type": "Word", "key": "w"}, preview(ref("w"))], "r2")]
    assert execute(bypassed, "static_labels")[1][0] == "PS1:capability.provenance_incompatible"

    dynamic = base([{"op": "dynamic", "source": lit("Text", "constant arithmetic")}])
    code, _, ctx = execute(dynamic, "static_runtime_labels")[0]
    assert code == "LG3:capability.not_granted" and not ctx.dynamic_attempts

    # The matrix deliberately excludes S02 from source_bound. Independently
    # exercise the real adapter's byte guard without claiming a matrix pass.
    from adapter import execute as adapter_execute
    import json
    original = next(c for _, c in cases() if c["id"] == "S02")
    changed = copy.deepcopy(original["program"])
    changed["modules"]["constant"]["body"][0]["value"]["value"] = 2
    bindings = json.loads((HERE / "bindings.json").read_bytes())
    host = Host(changed, "static_labels", bindings["source_files"])
    ctx = RunContext(original, "approval", changed["runs"][0], host, bindings, "static_labels", "source_bound")
    try:
        adapter_execute(ctx)
    except Stop as stop:
        assert stop.code == "LG5:input.module_digest_mismatch" and not ctx.executions and not ctx.admitted
    else:
        raise AssertionError("changed dependency accepted")
    print("PASS: lexer/parser spans, exact types, branch joins, runtime PC, closures, calls and storage", flush=True)


if __name__ == "__main__":
    self_test()
