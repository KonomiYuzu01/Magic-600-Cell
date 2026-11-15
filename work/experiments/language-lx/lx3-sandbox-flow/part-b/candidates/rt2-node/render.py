"""Mechanical JavaScript rendering of neutral programs, never a syntax proposal.

No case IDs, expected results, policy or corpus files enter render(). Exact data
are JSON inside string literals: JavaScript only handles opaque value IDs.
"""

import json


HEADER = '"use strict";\n// neutral-json '


def quoted(value):
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)


def render(program):
    output, mapping = bytearray(), {}
    modules = {name: "m" + str(i) for i, name in enumerate(sorted(program["modules"]))}

    def emit(text):
        output.extend(text.encode("utf-8"))

    def block(body, prefix, indent="  "):
        for index, op in enumerate(body):
            path = prefix + "/" + str(index)
            start = len(output)
            kind = op["op"]
            call = quoted(path) + ", e, c, d, i"
            emit(indent + "{\n")
            if kind == "if":
                emit(indent + "  const t = branch(" + call + ");\n")
                emit(indent + "  const parent = c;\n" + indent + "  c = c.concat([t.id]);\n")
                emit(indent + "  if (t.truth) {\n")
                block(op["then"], path + "/then", indent + "    ")
                emit(indent + "  } else {\n")
                block(op["else"], path + "/else", indent + "    ")
                emit(indent + "  }\n" + indent + "  c = parent;\n")
            elif kind == "for":
                emit(indent + "  const seq = sequence(" + call + ");\n")
                emit(indent + "  const outer = e;\n" + indent + "  const previous = i;\n")
                emit(indent + "  const parent = c;\n" + indent + "  c = c.concat([seq]);\n")
                emit(indent + "  for (i = 0; loop(" + quoted(path) + ", seq, i); i++) {\n")
                emit(indent + "    e = Object.assign(Object.create(null), outer);\n")
                emit(indent + "    e[" + quoted(op["item"]) + "] = item(" + quoted(path) + ", seq, c, i);\n")
                block(op["body"], path + "/body", indent + "    ")
                emit(indent + "    merge(" + quoted(path) + ", outer, e);\n")
                emit(indent + "    covered(" + quoted(path) + ", i);\n" + indent + "  }\n")
                emit(indent + "  e = outer;\n" + indent + "  i = previous;\n")
                emit(indent + "  c = parent;\n")
            elif kind == "call":
                emit(indent + "  const local = enter(" + call + ");\n")
                emit(indent + "  const value = " + modules[op["module"]] + "(local, c, d + 1, null);\n")
                emit(indent + "  e[" + quoted(op["as"]) + "] = leave(" + quoted(path) + ", value, e, c);\n")
            elif kind == "return":
                emit(indent + "  return finish(" + call + ");\n")
            else:
                assign = "e[" + quoted(op["as"]) + "] = " if "as" in op else ""
                emit(indent + "  " + assign + kind + "(" + call + ");\n")
            if kind != "return":
                emit(indent + "  complete(" + quoted(path) + ", d);\n")
            emit(indent + "}\n")
            mapping[path] = {"start": start, "end": len(output)}

    emit(HEADER + quoted(program) + "\n")
    for name, function in modules.items():
        emit("function " + function + "(e, c, d, i) {\n")
        block(program["modules"][name]["body"], "modules/" + name + "/body")
        emit("}\n")
    for index, run in enumerate(program["runs"]):
        emit("function r" + str(index) + "(e, c, d, i) {\n")
        block(run["body"], run["id"] + "/body")
        emit("}\n")
    emit("dispatch({" + ",".join(quoted(run["id"]) + ":r" + str(i)
                                 for i, run in enumerate(program["runs"])) + "});\n")
    return bytes(output), mapping
