"""Mechanical neutral-program -> restricted Python. No case or answer inputs.

Experiment only: these host-language programs propose no product syntax.
"""

import hashlib
import json


def identifier(prefix, name):
    return prefix + name.encode("utf-8").hex()


def literal(value):
    if isinstance(value, dict):
        return {key: literal(value[key]) for key in sorted(value)}
    if isinstance(value, list):
        return [literal(item) for item in value]
    return value


def render(program):
    """Return UTF-8 source and byte spans for *every* neutral instruction."""
    output, mapping = bytearray(), {}

    def line(text, indent=0):
        output.extend(("    " * indent + text + "\n").encode("utf-8"))

    def expr(value, variables, entry):
        if "ref" in value:
            return variables[value["ref"]]
        data = json.dumps(literal(value["value"]), ensure_ascii=False,
                          separators=(",", ":"), allow_nan=False)
        return "h_literal(" + repr(value["type"]) + ", " + repr(data) + ", " + repr(entry) + ")"

    def block(body, prefix, indent, variables, control=(), scope=""):
        if not body:
            line("pass", indent)
        for index, op in enumerate(body):
            path = prefix + "/" + str(index)
            start = len(output)
            kind = op["op"]
            controls = "[" + ", ".join(control) + "]"
            args = [repr(path)]
            def expression(value):
                return expr(value, variables, path + "/literal")
            bound = variables.get(op.get("as"), identifier("v_", scope + op.get("as", "")))
            target = bound + " = " if "as" in op else ""
            if kind in ("kernel", "calc", "call"):
                args += [repr(op["type"])]
                args += [identifier("m_", op["module"]) if kind == "call" else
                         repr(op["row"] if kind == "kernel" else op["fn"])]
                args += ["[" + ", ".join(expression(v) for v in op["args"]) + "]", controls]
            elif kind == "if":
                condition = identifier("c_", path)
                line(condition + " = h_condition(" + ", ".join([repr(path), expression(op["test"]), controls]) + ")", indent)
                line("if h_truth(" + condition + "):", indent)
                yes, no = dict(variables), dict(variables)
                block(op["then"], path + "/then", indent + 1, yes, (*control, condition), scope)
                line("else:", indent)
                block(op["else"], path + "/else", indent + 1, no, (*control, condition), scope)
                variables.update({name: yes[name] for name in set(yes) & set(no)})
                line("h_done(" + repr(path) + ")", indent)
            elif kind == "for":
                args += [repr(op["type"]), expression(op["items"]), controls]
                local = dict(variables)
                local[op["item"]] = identifier("v_", path + "/item/" + op["item"])
                line("for " + local[op["item"]] + " in h_iter(" + ", ".join(args) + "):", indent)
                block(op["body"], path + "/body", indent + 1, local, control, path + "/local/")
                line("h_done(" + repr(path) + ")", indent)
            elif kind == "workspace_get":
                args += [repr(op["type"]), repr(op["key"]), controls]
            elif kind == "workspace_put":
                args += [repr(op["key"]), expression(op["value"]), controls]
            elif kind == "preview":
                args += [expression(op["word"]), expression(op["target"]), controls]
            elif kind == "pick":
                args += [repr(op["type"]), expression(op["options"]), repr(op["event"]), controls]
            elif kind in ("emit", "return"):
                args += [expression(op["value"]), controls]
            elif kind == "publish":
                args += [expression(op["exact"]), expression(op["coordinates"]), controls]
            elif kind == "retain":
                args += [repr(op["bytes"]), controls]
            elif kind == "ambient":
                args += [repr(op["type"]), repr(op["resource"]), repr(op["logical_resource"]), controls]
            elif kind == "dynamic":
                args += [expression(op["source"]), controls]
            elif kind in ("tick", "stop"):
                args += [controls]
            else:
                raise ValueError("unsupported neutral instruction")
            if kind not in ("if", "for"):
                text = "h_" + kind + "(" + ", ".join(args) + ")"
                line(("return " if kind == "return" else target) + text, indent)
            if "as" in op:
                variables[op["as"]] = bound
            mapping[path] = {"start": start, "end": len(output),
                             "sha256": hashlib.sha256(output[start:]).hexdigest()}

    def function(name, body, prefix, parameters=()):
        variables = {key: identifier("v_", key) for key in set(program["inputs"]) | set(parameters)}
        line("def " + name + "(" + ", ".join(identifier("v_", p) for p in parameters) + "):")
        for key in sorted(set(program["inputs"]) - set(parameters)):
            line(identifier("v_", key) + " = h_input(" + repr(key) + ")", 1)
        block(body, prefix, 1, variables)
        line("")

    for name, module in sorted(program["modules"].items()):
        function(identifier("m_", name), module["body"], "modules/" + name + "/body", sorted(module["parameters"]))
    for run in program["runs"]:
        function(identifier("r_", run["id"]), run["body"], run["id"] + "/body")
    return bytes(output), mapping
