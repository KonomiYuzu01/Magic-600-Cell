"""Pure neutral-program to Python rendering; no case or outcome input.

Python is a stand-in host for this experiment, not a host or syntax proposal.
Branches, loops, functions and returns execute in Python, not a neutral evaluator.
"""

import hashlib


def symbol(prefix, name):
    return prefix + name.encode("utf-8").hex()


def literal(value):
    if isinstance(value, dict):
        return "{" + ", ".join(repr(k) + ": " + literal(value[k]) for k in sorted(value)) + "}"
    if isinstance(value, list):
        return "[" + ", ".join(map(literal, value)) + "]"
    return repr(value)


def render(program):
    """Return executable UTF-8 bytes and local neutral-instruction byte spans."""
    output, mapping = bytearray(), {}

    def line(indent, text):
        output.extend(("    " * indent + text + "\n").encode("utf-8"))

    def expression(expr, env, path):
        if "ref" in expr:
            return env[expr["ref"]]
        return f"lib.literal({expr['type']!r}, {literal(expr['value'])}, {path + '/literal'!r})"

    def assignments(body):
        names = set()
        for op in body:
            if "as" in op:
                names.add(op["as"])
            if op["op"] == "if":
                names.update(assignments(op["then"]))
                names.update(assignments(op["else"]))
        return names

    def block(body, env, prefix, indent, *, top=False, loop_index="None", module=False):
        for name in sorted(assignments(body)):
            env.setdefault(name, symbol("v_", name + "@" + prefix))
        for index, op in enumerate(body):
            path = prefix + "/" + str(index)
            start = len(output)
            line(indent, f"lib.step({path!r}, {loop_index})")
            kind = op["op"]
            args = [expression(e, env, path + "/args/" + str(i)) for i, e in enumerate(op.get("args", []))]
            values = "[" + ", ".join(args) + "]"
            if kind in ("kernel", "calc"):
                name = op["row"] if kind == "kernel" else op["fn"]
                line(indent, f"{env[op['as']]} = lib.{kind}({path!r}, {op['type']!r}, {name!r}, {values}, {loop_index})")
            elif kind == "if":
                line(indent, f"if {expression(op['test'], env, path)}:")
                for branch in ("then", "else"):
                    if branch == "else":
                        line(indent, "else:")
                    if op[branch]:
                        block(op[branch], env, path + "/" + branch, indent + 1,
                              loop_index=loop_index, module=module)
                    else:
                        line(indent + 1, "pass")
            elif kind == "for":
                local = dict(env)
                item = symbol("v_", op["item"] + "@" + path)
                local[op["item"]] = item
                counter = symbol("v_", "index@" + path)
                items = expression(op["items"], env, path)
                track = top and len(body) == 1
                line(indent, f"for {counter}, {item} in lib.iterate({path!r}, {items}, {op['type']!r}, {track!r}):")
                if op["body"]:
                    block(op["body"], local, path + "/body", indent + 1,
                          loop_index=counter, module=module)
                else:
                    line(indent + 1, "pass")
            elif kind == "call":
                function = symbol("m_", op["module"])
                line(indent, f"{env[op['as']]} = lib.call({path!r}, {op['type']!r}, {function}, {values})")
            elif kind == "workspace_get":
                line(indent, f"{env[op['as']]} = lib.workspace_get({path!r}, {op['key']!r})")
            elif kind in ("workspace_put", "emit", "return"):
                value = expression(op["value"], env, path)
                if kind == "workspace_put":
                    line(indent, f"lib.workspace_put({path!r}, {op['key']!r}, {value})")
                elif kind == "emit":
                    line(indent, f"lib.emit({path!r}, {value})")
                else:
                    line(indent, f"return lib.result({path!r}, {value}, {not module!r})")
            elif kind == "preview":
                word = expression(op["word"], env, path + "/word")
                target = expression(op["target"], env, path + "/target")
                line(indent, f"{env[op['as']]} = lib.preview({path!r}, {word}, {target})")
            elif kind == "pick":
                options = expression(op["options"], env, path)
                line(indent, f"{env[op['as']]} = lib.pick({path!r}, {op['type']!r}, {options}, {op['event']!r})")
            elif kind == "publish":
                exact = expression(op["exact"], env, path + "/exact")
                coordinates = expression(op["coordinates"], env, path + "/coordinates")
                line(indent, f"lib.publish({path!r}, {exact}, {coordinates})")
            elif kind == "retain":
                line(indent, f"{env[op['as']]} = lib.retain({path!r}, {op['bytes']!r})")
            elif kind == "ambient":
                line(indent, f"{env[op['as']]} = lib.ambient({path!r}, {op['type']!r}, {op['resource']!r}, {op['logical_resource']!r})")
            elif kind == "dynamic":
                line(indent, f"lib.dynamic({path!r}, {expression(op['source'], env, path)})")
            elif kind == "stop":
                line(indent, f"lib.stop({path!r}, {loop_index})")
            elif kind != "tick":
                raise ValueError("unsupported neutral instruction")
            if top and kind != "return" and not (len(body) == 1 and kind in ("for", "call")):
                if not (kind == "kernel" and op["row"] == "batch_check"):
                    line(indent, f"lib.done({index})")
            end = len(output)
            mapping[path] = {"start": start, "end": end,
                             "sha256": hashlib.sha256(output[start:end]).hexdigest()}

    line(0, "# Mechanically rendered experiment code; no host or syntax selection.")
    functions = [(symbol("m_", name), module["parameters"], module["body"], "modules/" + name, True)
                 for name, module in sorted(program["modules"].items())]
    functions += [(symbol("r_", run["id"]), {}, run["body"], run["id"], False) for run in program["runs"]]
    for name, parameters, body, prefix, module in functions:
        env = {key: symbol("v_", key) for key in program["inputs"]}
        env.update({key: symbol("v_", key) for key in parameters})
        signature = ", ".join(["lib"] + [env[key] for key in sorted(parameters)])
        line(0, f"def {name}({signature}):")
        for key in sorted(program["inputs"]):
            if key not in parameters:
                line(1, f"{env[key]} = lib.input({key!r})")
        block(body, env, prefix + "/body", 1, top=not module, module=module)
        if not body and not program["inputs"]:
            line(1, "pass")
        line(0, "")
    return bytes(output), mapping
