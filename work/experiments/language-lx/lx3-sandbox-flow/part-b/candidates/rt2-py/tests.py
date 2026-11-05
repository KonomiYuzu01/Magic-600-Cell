"""Small independent fixtures for checker joins, indirection and native scope."""

from harness.bindings import neutral_paths
from harness.common import sha
from harness.host import Host
from harness.meter import Stop
from harness.policy import GRANTS, Policy
from harness.runtime import RunContext
from bridge import Bridge
from probes import observations
from render import identifier, render
from subset import Analysis, approved_tree


def program(body, inputs=None, modules=None):
    return {"inputs": inputs or {}, "modules": modules or {}, "runs": [{
        "id": "r1", "profile": "solving", "author": "built_in", "boundary_class": "analysis",
        "invocation": "command", "body": body}]}


def literal(kind, value):
    return {"type": kind, "value": value}


def analyze(stimulus):
    source, spans = render(stimulus)
    tree, functions, paths = approved_tree(source)
    assert paths == set(neutral_paths(stimulus)) == set(spans)
    assert all(sha(source[s["start"]:s["end"]]) == s["sha256"] for s in spans.values())
    return source, spans, Analysis(tree, functions, stimulus, Policy().data["transparent"])


def run_local(stimulus, form="static_labels"):
    source, spans, analysis = analyze(stimulus)
    host = Host(stimulus, form, {})
    bindings = {"effective_grants": {"solving": list(GRANTS)}, "source_map": {"local": spans}}
    ctx = RunContext({"id": "local", "program": stimulus}, "local", stimulus["runs"][0], host,
                     bindings, form, "approved_subset_byte_bound")
    if form == "capability_separation" and not analysis.has_preview("r1"):
        ctx.bind_grants([g for g in GRANTS if g != "preview"])
    ctx.admit()
    bridge = Bridge(ctx, source, spans, analysis)
    namespace = bridge.namespace()
    exec(compile(analysis.tree, "local-fixture.py", "exec"), namespace, namespace)
    try:
        value, stop = namespace[identifier("r_", "r1")](), None
    except Stop as caught:
        value, stop = None, caught
    return value, stop, ctx


def self_test():
    rows = observations()
    assert all(row["checker"] == "rejected" and row["approved_execution"] == "not entered" for row in rows)
    assert rows[0]["runtime_without_checker"] == "returned list metadata"
    assert rows[1]["runtime_without_checker"].endswith("True")
    assert rows[2]["runtime_without_checker"] == "returned str metadata"
    preview = {"op": "preview", "as": "receipt", "type": "Receipt",
               "word": {"ref": "choice"}, "target": literal("Target", "c0")}
    branch = program([
        {"op": "if", "test": literal("Bool", True),
         "then": [{"op": "calc", "as": "choice", "type": "Word", "fn": "identity",
                   "args": [literal("Word", ["a"])]}],
         "else": [{"op": "kernel", "as": "n", "type": "Ints", "row": "counts", "args": []},
                  {"op": "calc", "as": "choice", "type": "Word", "fn": "index",
                   "args": [literal("Words", [["a"], ["b"], ["c"]]), literal("Int", 0)]},
                  {"op": "if", "test": literal("Bool", True),
                   "then": [{"op": "calc", "as": "choice", "type": "Word", "fn": "combine_corrections",
                             "args": [literal("Words", [["a"], ["b"], ["c"]]), {"ref": "n"}]}], "else": []}]}, preview])
    _, _, analysis = analyze(branch)
    assert "state" in analysis.marks["r1/body/1"][0].labels
    _, stop, ctx = run_local(branch)
    assert stop.code == "LG3:capability.not_granted" and not ctx.host.shell
    assert not ctx.kernel_calls  # Even the unexecuted branch contributes labels.

    module = {"parameters": {}, "result": "Text", "body": [
        {**preview, "word": literal("Word", ["a"])}, {"op": "return", "value": literal("Text", "done")} ]}
    indirect = program([
        {"op": "kernel", "as": "n", "type": "Ints", "row": "counts", "args": []},
        {"op": "call", "as": "result", "type": "Text", "module": "p", "args": []}], modules={"p": module})
    assert analyze(indirect)[2].has_preview("r1")
    _, stop, ctx = run_local(indirect, "capability_separation")
    assert stop.code == "LG3:capability.not_granted" and not ctx.kernel_calls and not ctx.host.shell

    # Native Python's function-wide locals must not overwrite a neutral loop's
    # outer item binding. Existing accumulator assignments do survive the loop.
    scoped = program([
        {"op": "calc", "as": "sum", "type": "Int", "fn": "identity", "args": [literal("Int", 0)]},
        {"op": "for", "item": "x", "type": "Int", "items": literal("Ints", [1, 2]), "body": [
            {"op": "calc", "as": "sum", "type": "Int", "fn": "add", "args": [{"ref": "sum"}, {"ref": "x"}]}]},
        {"op": "calc", "as": "result", "type": "Int", "fn": "add", "args": [{"ref": "sum"}, {"ref": "x"}]},
        {"op": "return", "value": {"ref": "result"}}], inputs={"x": {"type": "Int", "value": 9, "origin": "manual"}})
    value, stop, _ = run_local(scoped)
    assert stop is None and value.data == 12
    # Dict insertion order is not a hidden renderer input.
    assert render(scoped) == render({key: scoped[key] for key in reversed(scoped)})

    # Static propagation across module return and persisted parts uses AST facts.
    stored = program([
        {"op": "kernel", "as": "n", "type": "Ints", "row": "counts", "args": []},
        {"op": "workspace_put", "key": "n", "value": {"ref": "n"}},
        {"op": "workspace_get", "as": "saved", "type": "Ints", "key": "n"},
        {"op": "return", "value": {"ref": "saved"}}])
    assert analyze(stored)[2].marks["r1/body/3"][0].read
