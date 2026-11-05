"""Trusted interface-v1 glue; no expected outcomes or case-family handling."""

import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from harness.common import ROOT, digest, sha
from render import render
from rt1_syntax import parse
from rt1_check import Checker
from rt1_vm import Machine


def execute(ctx):
    index = json.loads((HERE / "programs.json").read_bytes())
    artifact = index[ctx.case["id"]]
    path = (ROOT / artifact["file"]).resolve()
    if path.parent != HERE:
        raise ValueError("rendered program outside candidate")
    source = path.read_bytes()[artifact["start"]:artifact["end"]]
    # Render the actual stimulus, including any changed dependency. Never repair
    # it from ctx.case['program']; retain the original approval bytes and pin.
    actual_source, _ = render(ctx.program)
    if (source != actual_source or sha(source) != artifact["sha256"]
            or digest(ctx.program) != artifact["neutral_sha256"]):
        ctx.refuse("LG5:input.module_digest_mismatch", layer="rt1_source_binding")
    program = parse(source)
    checked = Checker(program, ctx.program, ctx.kernel.fixture["rows"], ctx.policy.data["transparent"]).check(ctx.form)
    return Machine(ctx, checked).execute()
