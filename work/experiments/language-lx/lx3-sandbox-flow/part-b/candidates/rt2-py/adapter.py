"""Bench glue: verify all program bytes, check their AST, execute that same tree."""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from harness.common import ROOT, digest
from bridge import Bridge
from render import identifier, render
from subset import Analysis, approved_tree


def execute(ctx):
    ctx.layer = "rt2_py"
    index = json.loads((HERE / "programs.json").read_bytes())
    approved = (ROOT / index[digest(ctx.case["program"])]).read_bytes()
    actual, spans = render(ctx.program)
    # Apply every supplied substitution mechanically. Never restore old bytes or
    # change the approval digest to bless a mutated dependency.
    if actual != approved:
        ctx.refuse("LG5:input.module_digest_mismatch", operation="approval", layer="rt2_byte_binding")
    tree, functions, paths = approved_tree(actual)
    if paths != set(spans):
        raise ValueError("instruction/AST mapping mismatch")
    analysis = Analysis(tree, functions, ctx.program, ctx.policy.data["transparent"])
    grants = list(ctx.grants)
    if ctx.form == "capability_separation" and ctx.run["profile"] == "solving" and not analysis.has_preview(ctx.run["id"]):
        grants.remove("preview")
    ctx.bind_grants(grants)
    ctx.admit()
    # Dynamic code is a recognized attempted authority, never an executed helper.
    # The shared policy supplies its refusal code; no answer data are consulted.
    if analysis.dynamic:
        ctx.position = sorted(set(analysis.dynamic))[0]
        ctx.check("dynamic", labels=(), read_state=False)
        raise ValueError("dynamic evaluation cannot be approved")
    bridge = Bridge(ctx, actual, spans, analysis)
    namespace = bridge.namespace()
    # No re-read, source rewrite, bytecode substitution or recompilation occurs
    # between checking this AST and compiling it into the code object executed.
    code = compile(tree, "rt2-approved.py", "exec", dont_inherit=True, optimize=0)
    exec(code, namespace, namespace)
    return namespace[identifier("r_", ctx.run["id"])]()
