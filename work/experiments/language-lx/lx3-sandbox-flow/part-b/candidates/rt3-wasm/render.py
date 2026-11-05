"""Pure neutral-program -> throwaway RT3 surface. No corpus or answer access.

The surface is JSON arrays with positional operands, not a syntax proposal.
Paths identify instructions, never cases. Both sides of every branch survive.
"""

import json


def encode(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":"), allow_nan=False).encode("utf-8")


def expression(value):
    return ["use", value["ref"]] if "ref" in value else ["lit", value["type"], value["value"]]


def render(program):
    """Only the neutral program is an input; no case identity or expectations."""
    def block(body, prefix):
        result = []
        for index, op in enumerate(body):
            path = prefix + "/" + str(index)
            kind = op["op"]
            if kind in ("kernel", "calc", "call"):
                key = {"kernel": "row", "calc": "fn", "call": "module"}[kind]
                args = [op["as"], op["type"], op[key], [expression(v) for v in op["args"]]]
            elif kind == "if":
                args = [expression(op["test"]), block(op["then"], path + "/then"),
                        block(op["else"], path + "/else")]
            elif kind == "for":
                args = [op["item"], op["type"], expression(op["items"]), block(op["body"], path + "/body")]
            elif kind == "workspace_get":
                args = [op["as"], op["type"], op["key"]]
            elif kind == "workspace_put":
                args = [op["key"], expression(op["value"])]
            elif kind == "preview":
                args = [op["as"], op["type"], expression(op["word"]), expression(op["target"])]
            elif kind == "pick":
                args = [op["as"], op["type"], expression(op["options"]), op["event"]]
            elif kind in ("return", "emit"):
                args = [expression(op["value"])]
            elif kind == "publish":
                args = [expression(op["exact"]), expression(op["coordinates"])]
            elif kind == "retain":
                args = [op["as"], op["type"], op["bytes"]]
            elif kind == "ambient":
                args = [op["as"], op["type"], op["resource"], op["logical_resource"]]
            elif kind == "dynamic":
                args = [expression(op["source"])]
            elif kind == "stop":
                args = [op["reason"]]
            elif kind == "tick":
                args = []
            else:
                raise ValueError("unsupported neutral operation")
            result.append([kind, path, *args])
        return result

    return {"edition": "rt3-array-1", "entry_order": [run["id"] for run in program["runs"]], "inputs": {
        name: [item["type"], item["value"], item["origin"]]
        for name, item in program["inputs"].items()},
        "modules": {name: [module["parameters"], module["result"],
                           block(module["body"], "modules/" + name + "/body")]
                    for name, module in program["modules"].items()},
        "entries": {run["id"]: [run["profile"], run["author"], run["boundary_class"],
                                run["invocation"], block(run["body"], run["id"] + "/body")]
                    for run in program["runs"]}}


def instructions(surface):
    result = {}
    def block(body):
        for node in body:
            result[node[1]] = node
            if node[0] == "if":
                block(node[3])
                block(node[4])
            elif node[0] == "for":
                block(node[5])
    for entry in surface["entries"].values():
        block(entry[4])
    for module in surface["modules"].values():
        block(module[2])
    return result


def bundle(surfaces):
    """Serialize surfaces and retain UTF-8 byte spans of every source node."""
    watched = {id(node): (key, path) for key, surface in surfaces.items()
               for path, node in instructions(surface).items()}
    spans = {key: {} for key in surfaces}
    output = bytearray()
    def emit(value):
        start = len(output)
        if type(value) is dict:
            output.extend(b"{")
            for i, key in enumerate(sorted(value)):
                if i:
                    output.extend(b",")
                output.extend(encode(key) + b":")
                emit(value[key])
            output.extend(b"}")
        elif type(value) is list:
            output.extend(b"[")
            for i, item in enumerate(value):
                if i:
                    output.extend(b",")
                emit(item)
            output.extend(b"]")
        else:
            output.extend(encode(value))
        if id(value) in watched:
            key, path = watched[id(value)]
            spans[key][path] = (start, len(output))
    emit(surfaces)
    assert bytes(output) == encode(surfaces)
    return bytes(output) + b"\n", spans
