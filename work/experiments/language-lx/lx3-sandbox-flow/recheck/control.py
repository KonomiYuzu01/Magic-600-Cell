"""Synthetic corrupting candidate. Never replaces a frozen candidate's code."""

from harness.common import freeze, thaw
from harness.host import Value

MODES = ("workspace_value", "workspace_provenance", "pending_replace", "pending_clear",
         "legitimate_write", "corrupt_then_write", "session", "after_refusal", "peer_corruption")


def execute(ctx, mode):
    host = ctx.host
    if ctx.run["id"] == "r1":
        host.workspace_put("saved", host.input("word"))
        return None
    old = host.workspace["saved"]
    if mode in ("workspace_value", "corrupt_then_write"):
        host.workspace["saved"] = Value(old.type, freeze(["b"]), old.block)
    elif mode == "workspace_provenance":
        block = thaw(old.block)
        block["author"] = "synthetic_corruption"
        host.workspace["saved"] = Value(old.type, old.data, freeze(block))
    elif mode == "pending_replace":
        host.pending = {"word": ["a"], "target": "c0"}
    elif mode == "pending_clear":
        host.pending = None
    elif mode == "session":
        host.session["revision"] += 1
    if mode in ("legitimate_write", "corrupt_then_write"):
        host.workspace_put("legitimate", host.input("word"))
    ctx.refuse(ctx.case["expected_code"], operation="preview", path="r2/body/3")
