"""Four additional two-run programs, outside lx3-neutral-v2 and its counts."""

from copy import deepcopy


def programs():
    inputs = {"word": {"type": "Word", "value": ["a"], "origin": "manual"},
              "target": {"type": "Target", "value": "c0", "origin": "manual"},
              "corrections": {"type": "Words", "value": [["a"], ["b"]], "origin": "fixture"}}
    clean = {"op": "workspace_put", "key": "saved", "value": {"ref": "word"}}
    taint = [{"op": "kernel", "as": "coefficients", "type": "Ints", "row": "nu2", "args": []},
             {"op": "calc", "as": "derived", "type": "Word", "fn": "combine_corrections",
              "args": [{"ref": "corrections"}, {"ref": "coefficients"}]},
             {"op": "workspace_put", "key": "saved", "value": {"ref": "derived"}}]
    bodies = {
        "unreachable_clean": taint + [{"op": "stop", "reason": "requested"}, clean],
        "untaken_clean": taint + [{"op": "if", "test": {"type": "Bool", "value": False},
                                    "then": [clean], "else": []}],
        "clean_before_taint": [clean] + taint,
        "executed_clean": taint + [clean],
    }
    result = {}
    for name, body in bodies.items():
        result[name] = {"inputs": deepcopy(inputs), "modules": {}, "runs": [
            {"id": "r1", "profile": "tool_authoring", "author": "built_in",
             "boundary_class": "analysis", "invocation": "command", "body": deepcopy(body)},
            {"id": "r2", "profile": "solving", "author": "built_in",
             "boundary_class": "analysis", "invocation": "command", "body": [
                 {"op": "workspace_get", "as": "loaded", "type": "Word", "key": "saved"},
                 {"op": "preview", "as": "receipt", "type": "Receipt",
                  "word": {"ref": "loaded"}, "target": {"ref": "target"}}]}]}
    return result
