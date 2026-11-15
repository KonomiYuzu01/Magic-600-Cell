"""Four fixed independent edits, each replayed against a frozen baseline."""
import sys
sys.dont_write_bytecode = True

import random
from common import sha

SEED = 2026110551


def make_script(modules):
    rng = random.Random(SEED)
    rows = []
    for name in ("S", "M", "L", "W00"):
        data = modules[name]
        number = rng.randrange(2, 10000)
        signature = b"fn exported() -> Int"
        body = b"    return 1;"
        changes = {
            "body": [(data.index(body), len(body), f"    return exact_add(1, {number});")],
            "signature": [(data.index(signature), len(signature), "fn exported() -> Text"),
                          (data.index(body), len(body), '    return "edited";')],
            "completion": [(len(data), 0, "fn completion_point() -> Int effects [] charge [1, 0, 0, 0] {\n    return exact_")],
            "syntax_break": [(data.index(body) + len(body) - 1, 1, "")],
        }
        for kind, replacements in changes.items():
            row = {"module": name, "kind": kind, "base_sha256": sha(data),
                   "replacements": [{"offset": offset, "delete_hex": data[offset:offset + count].hex(),
                                     "insert": text} for offset, count, text in replacements]}
            row["result_sha256"] = sha(apply(data, row))
            rows.append(row)
    return {"schema": "lx4-edits-0", "seed": SEED, "reset_between_edits": True, "edits": rows}


def apply(data, row):
    if sha(data) != row["base_sha256"]:
        raise ValueError("edit refuses a stale baseline")
    for change in sorted(row["replacements"], key=lambda item: item["offset"], reverse=True):
        at, old = change["offset"], bytes.fromhex(change["delete_hex"])
        if data[at:at + len(old)] != old:
            raise ValueError("edit preimage differs")
        data = data[:at] + change["insert"].encode("utf-8") + data[at + len(old):]
    if "result_sha256" in row and sha(data) != row["result_sha256"]:
        raise ValueError("edit result differs")
    return data


def dependents(edges, changed):
    affected = set()
    for name, imports in edges.items():  # generator emits a topological order
        if changed in imports or affected.intersection(imports):
            affected.add(name)
    return sorted(affected)
