"""Pure neutral-program -> throwaway RT1 text. No case or outcome input."""

import json


def render(program):
    out, spans = bytearray(), {}

    def emit(text):
        out.extend(text.encode("utf-8"))

    def literal(value):
        return json.dumps(value, ensure_ascii=False, sort_keys=True,
                          separators=(",", ":"), allow_nan=False)

    def expr(value):
        if "ref" in value:
            return "(ref " + literal(value["ref"]) + ")"
        return "(lit " + value["type"] + " " + literal(value["value"]) + ")"

    def body(ops, prefix, indent):
        for index, op in enumerate(ops):
            path = prefix + "/" + str(index)
            emit(" " * indent)
            start = len(out)
            kind = op["op"]
            emit("(" + kind)
            if kind in ("kernel", "calc", "call"):
                callee = op[{"kernel": "row", "calc": "fn", "call": "module"}[kind]]
                emit(" " + literal(op["as"]) + " " + op["type"] + " " + literal(callee))
                for arg in op["args"]:
                    emit(" " + expr(arg))
            elif kind == "if":
                emit(" " + expr(op["test"]) + "\n")
                for branch in ("then", "else"):
                    emit(" " * (indent + 2) + "(" + branch + "\n")
                    body(op[branch], path + "/" + branch, indent + 4)
                    emit(" " * (indent + 2) + ")\n")
                emit(" " * indent)
            elif kind == "for":
                emit(" " + literal(op["item"]) + " " + op["type"] + " " + expr(op["items"]) + "\n")
                body(op["body"], path + "/body", indent + 2)
                emit(" " * indent)
            elif kind == "workspace_get":
                emit(" " + literal(op["as"]) + " " + op["type"] + " " + literal(op["key"]))
            elif kind == "workspace_put":
                emit(" " + literal(op["key"]) + " " + expr(op["value"]))
            elif kind == "preview":
                emit(" " + literal(op["as"]) + " " + op["type"] + " " + expr(op["word"]) + " " + expr(op["target"]))
            elif kind == "pick":
                emit(" " + literal(op["as"]) + " " + op["type"] + " " + expr(op["options"]) + " " + literal(op["event"]))
            elif kind in ("emit", "return", "dynamic"):
                emit(" " + expr(op["source" if kind == "dynamic" else "value"]))
            elif kind == "publish":
                emit(" " + expr(op["exact"]) + " " + expr(op["coordinates"]))
            elif kind == "retain":
                emit(" " + literal(op["as"]) + " " + op["type"] + " " + str(op["bytes"]))
            elif kind == "ambient":
                emit(" " + literal(op["as"]) + " " + op["type"] + " " + literal(op["resource"]) + " " + literal(op["logical_resource"]))
            elif kind == "stop":
                emit(" " + literal(op["reason"]))
            elif kind != "tick":
                raise ValueError("unknown instruction")
            emit(")")
            spans[path] = (start, len(out))
            emit("\n")

    emit("(rt1\n")
    for name, item in sorted(program["inputs"].items()):
        emit("  (input " + literal(name) + " " + item["type"] + ")\n")
    for name, module in sorted(program["modules"].items()):
        emit("  (module " + literal(name) + " (params")
        for parameter, kind in sorted(module["parameters"].items()):
            emit(" (" + literal(parameter) + " " + kind + ")")
        emit(") " + module["result"] + "\n")
        body(module["body"], "modules/" + name + "/body", 4)
        emit("  )\n")
    for run in program["runs"]:
        emit("  (run " + literal(run["id"]) + "\n")
        body(run["body"], run["id"] + "/body", 4)
        emit("  )\n")
    emit(")\n")
    return bytes(out), spans
