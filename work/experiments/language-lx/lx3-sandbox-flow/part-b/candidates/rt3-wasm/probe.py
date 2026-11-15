"""Focused synthetic proofs of lowering hazards, separate from frozen results."""

from copy import deepcopy
from pathlib import Path
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(1, str(HERE.parents[1]))

from harness.common import digest
from harness.host import Host
from harness.meter import Stop
from harness.policy import GRANTS, Policy
from harness.runtime import RunContext
from adapter import execute, run_compiled
from frontend import compile_program
from render import instructions, render

FORMS = ("capability_separation", "static_labels", "static_runtime_labels")


def lit(kind, value):
    return {"type": kind, "value": value}


def ref(name):
    return {"ref": name}


def run(body, run_id="r1"):
    return {"id": run_id, "profile": "solving", "author": "built_in",
            "boundary_class": "analysis", "invocation": "command", "body": body}


def program(body, modules=None):
    return {"inputs": {"word": {**lit("Word", ["a"]), "origin": "manual"},
                       "target": {**lit("Target", "c0"), "origin": "manual"}},
            "modules": modules or {}, "runs": [run(body)]}


def calculation(name, kind, fn, args):
    return {"op": "calc", "as": name, "type": kind, "fn": fn, "args": args}


COUNTS = {"op": "kernel", "as": "n", "type": "Ints", "row": "counts", "args": []}
TEST = [COUNTS, calculation("v", "Int", "index", [ref("n"), lit("Int", 0)]),
        calculation("test", "Bool", "gt", [ref("v"), lit("Int", 0)])]
PREVIEW = {"op": "preview", "as": "receipt", "type": "Receipt", "word": ref("w"), "target": ref("target")}


def context(source, form, invocation=None, host=None):
    surface = render(source)
    invocation = invocation or source["runs"][0]
    pins = {"effective_grants": {p: list(GRANTS) for p in ("solving", "research", "tool_authoring")},
            "source_map": {"synthetic": {path: {"sha256": digest(node)} for path, node in instructions(surface).items()}}}
    host = host or Host(source, form, {"synthetic_source": digest(source)})
    case = {"id": "synthetic", "program": source}
    return RunContext(case, "synthetic", invocation, host, pins, form, "source_bound"), surface


def perform(source, form, invocation=None, host=None):
    ctx, surface = context(source, form, invocation, host)
    compiled = compile_program(surface, ctx.run["id"], form, Policy().data["transparent"])
    return run_compiled(ctx, surface, compiled), ctx


def refused(source, form):
    try:
        perform(source, form)
    except Stop as stop:
        assert stop.code == "LG3:capability.not_granted"
        return
    raise AssertionError("lost branch dependence")


def main():
    choice = {"op": "if", "test": ref("test"),
              "then": [calculation("w", "Word", "identity", [lit("Word", ["a"])])],
              "else": [calculation("w", "Word", "identity", [lit("Word", ["b"])])]}
    for form in FORMS:
        value, _ = perform(program(TEST + [choice, {"op": "return", "value": ref("w")}]), form)
        assert "state" in value.labels and value.read_state
        refused(program(TEST + [choice, PREVIEW]), form)

    # If the lowerer drops the unexecuted arm at a phi, static evidence loses
    # state; if it unions both arms dynamically, path precision is lost.
    branch = {"op": "if", "test": lit("Bool", True),
              "then": [calculation("w", "Word", "identity", [ref("word")])],
              "else": TEST + [calculation("w", "Word", "index", [lit("Words", [["a"], ["b"], ["c"]]), ref("v")])]}
    source = program([branch, {"op": "return", "value": ref("w")}])
    value, _ = perform(source, "static_labels")
    assert "state" in value.labels
    value, _ = perform(source, "static_runtime_labels")
    assert "state" not in value.labels

    # Branch control must cross an actual compiled module call, even when its
    # callee returns a literal and has no data parameter.
    module = {"constant": {"parameters": {}, "result": "Word",
                            "body": [{"op": "return", "value": lit("Word", ["a"])}]}}
    call = {"op": "call", "as": "w", "type": "Word", "module": "constant", "args": []}
    source = program(TEST + [{"op": "if", "test": ref("test"),
                             "then": [call, PREVIEW], "else": []}], module)
    for form in ("static_labels", "static_runtime_labels"):
        refused(source, form)

    # Persisted labels and parts are inputs to the next separately invoked run.
    source = program(TEST + [choice, {"op": "workspace_put", "key": "saved", "value": ref("w")}])
    source["runs"].append(run([{"op": "workspace_get", "as": "w", "type": "Word", "key": "saved"}, PREVIEW], "r2"))
    for form in FORMS:
        _, first = perform(source, form)
        try:
            perform(source, form, source["runs"][1], first.host)
        except Stop as stop:
            assert stop.code == "LG3:capability.not_granted"
        else:
            raise AssertionError("persisted flow was lost")

    # A source-bound route also rejects dynamic evaluation. S01/S02 themselves
    # are variant-inapplicable in the frozen matrix; these are extra probes.
    refused(program([{"op": "dynamic", "source": lit("Text", "arithmetic")}]), "static_runtime_labels")
    original = {"inputs": {}, "modules": {"constant": {"parameters": {}, "result": "Int",
                "body": [{"op": "return", "value": lit("Int", 1)}]}},
                "runs": [{"id": "r1", "profile": "tool_authoring", "author": "own", "boundary_class": None,
                          "invocation": "command", "body": [
                              {"op": "call", "as": "n", "type": "Int", "module": "constant", "args": []},
                              {"op": "return", "value": ref("n")}]}]}
    ctx, _ = context(original, "static_runtime_labels")
    actual = deepcopy(original)
    actual["modules"]["constant"]["body"][0]["value"]["value"] = 2
    ctx.program = ctx.host.program = actual
    try:
        execute(ctx)
    except Stop as stop:
        assert stop.code == "LG5:input.module_digest_mismatch" and not ctx.executions
    else:
        raise AssertionError("dependency substitution executed")
    print("PASS: Wasm branch/phi/module/workspace labels, dynamic refusal, source substitution, exact fuel.")


if __name__ == "__main__":
    main()
