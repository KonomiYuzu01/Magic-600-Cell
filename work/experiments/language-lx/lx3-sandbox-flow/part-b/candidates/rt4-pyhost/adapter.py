"""Trusted bench glue: byte approval, AST checker, then actual Python execution."""

import ast
import json
from pathlib import Path
import sys

FOLDER = Path(__file__).resolve().parent
sys.path.insert(0, str(FOLDER))

from library import Library
from render import render, symbol
from subset import Analysis, SubsetError, approve, closure, has_preview, method


def execute(ctx):
    ctx.layer = "rt4_library"
    index = json.loads((FOLDER / "program-index.json").read_bytes())[ctx.case["id"]]
    bundle = (FOLDER / "programs.py").read_bytes()
    approved = bundle[index["start"]:index["end"]]
    # Render the actual stimulus, including any dependency substitution, without
    # looking at its case kind or approved neutral program. Never execute old
    # bytes in place of the mutated dependency, or refresh its approval digest.
    actual, mapping = render(ctx.program)
    if actual != approved:
        ctx.refuse("LG5:input.module_digest_mismatch", operation="approval", layer="byte_approval")
    try:
        functions = approve(actual)
    except (SubsetError, SyntaxError):
        ctx.refuse("LG3:capability.not_granted", operation="approval", layer="approved_subset")
    entry = symbol("r_", ctx.run["id"])
    grants = set(ctx.grants)
    if ctx.form == "capability_separation" and ctx.run["profile"] == "solving":
        grants.discard("read" if has_preview(functions, entry) else "preview")
    ctx.bind_grants(grants)
    ctx.admit()
    # Dynamic code is refused by shared policy during subset approval, even in
    # an unexecuted arm of a reachable function.
    for name in sorted(closure(functions, entry)):
        for node in ast.walk(functions[name]):
            if isinstance(node, ast.Call) and method(node) == "dynamic":
                ctx.position = ast.literal_eval(node.args[0])
                ctx.check("dynamic", labels=(), read_state=False)
    analysis = None
    if ctx.form == "static_labels":
        analysis = Analysis(functions, ctx.program["inputs"], ctx.run["profile"], ctx.policy.data["transparent"])
        analysis.function(entry)
        for path, (operation, info, facts) in analysis.gates.items():
            ctx.position = path
            ctx.check(operation, labels=info.labels, read_state=info.reads, **facts)
    fragments = {path: actual[item["start"]:item["end"]] for path, item in mapping.items()}
    library = Library(ctx, fragments, functions, analysis)
    namespace = {"__builtins__": {}}
    exec(compile(actual, "candidates/rt4-pyhost/programs.py", "exec"), namespace)
    value = namespace[entry](library)
    return None if value is None else value.value
