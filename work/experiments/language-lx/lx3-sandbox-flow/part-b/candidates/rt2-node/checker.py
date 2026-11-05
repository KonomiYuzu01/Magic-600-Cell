"""Closed generated-JavaScript subset and termination-insensitive static labels.

The handwritten recognizer parses the single JSON header, checks its closed
instruction vocabulary, and reconstructs every permitted JavaScript token.
Byte equality rejects ANY extra JS, property access, expression or code injection.
The checker sees executed bytes, never case IDs, families or expected outcomes.
It deliberately accepts only the renderer's canonical image, not general JS.
"""

import json
from dataclasses import dataclass

from render import HEADER, render


SHAPES = {
    "kernel": "as type row args", "calc": "as type fn args", "call": "as type module args",
    "if": "test then else", "for": "item type items body", "workspace_get": "as type key",
    "workspace_put": "key value", "preview": "as type word target", "pick": "as type options event",
    "stop": "reason", "return": "value", "emit": "value", "publish": "exact coordinates",
    "retain": "as type bytes", "tick": "", "ambient": "as type resource logical_resource",
    "dynamic": "source",
}
CALCS = frozenset("identity index field eq gt argmax_first argmin_first rank_desc_indices append singleton sum count_equal length add mod_dot linear_solve_mod combine_corrections concat inverse zip indexed_points point".split())
# This is the mechanism's classification of sources, not a second policy.
STATE = frozenset("counts floors blocked bits nu2 nu5 snapshot word_check score preview_outcome batch_check".split())
ROWS = STATE | {"caps", "setup"}
TYPES = frozenset("Int Bool Text Word Target Ints Bools Words Targets Matrix Record Rows State Receipt Bytes".split())


class Rejected(ValueError):
    pass


def require(condition):
    if not condition:
        raise Rejected("outside generated JavaScript subset")


def expression(value):
    require(type(value) is dict)
    require(set(value) == {"ref"} or set(value) == {"type", "value"})
    if "ref" in value:
        require(type(value["ref"]) is str)
    else:
        require(value["type"] in TYPES)


def recognize(data):
    try:
        text = data.decode("utf-8")
        require(text.startswith(HEADER))
        end = text.index("\n", len(HEADER))
        program = json.loads(text[len(HEADER):end])
        require(set(program) == {"inputs", "modules", "runs"})
        require(type(program["inputs"]) is dict and type(program["modules"]) is dict)
        for item in program["inputs"].values():
            require(set(item) == {"type", "value", "origin"})
            require(item["type"] in TYPES and item["origin"] in ("manual", "fixture", "unknown"))

        def block(body):
            require(type(body) is list)
            for op in body:
                require(type(op) is dict and op.get("op") in SHAPES)
                kind = op["op"]
                require(set(op) == {"op"} | set(SHAPES[kind].split()))
                if "as" in op:
                    require(type(op["as"]) is str and op["type"] in TYPES)
                if kind == "kernel":
                    require(op["row"] in ROWS)
                if kind == "calc":
                    require(op["fn"] in CALCS)
                if kind == "call":
                    require(op["module"] in program["modules"])
                for key in ("test", "items", "value", "word", "target", "options", "exact", "coordinates", "source"):
                    if key in op:
                        expression(op[key])
                for value in op.get("args", []):
                    expression(value)
                for key in ("then", "else", "body"):
                    if key in op:
                        block(op[key])

        for module in program["modules"].values():
            require(set(module) == {"parameters", "result", "body"})
            require(module["result"] in TYPES and set(module["parameters"].values()) <= TYPES)
            block(module["body"])
        require(type(program["runs"]) is list and bool(program["runs"]))
        for run in program["runs"]:
            require(set(run) == {"id", "profile", "author", "boundary_class", "invocation", "body"})
            require(type(run["id"]) is str)
            block(run["body"])
        regenerated, mapping = render(program)
        require(regenerated == data)
        return program, mapping
    except (ValueError, KeyError, TypeError, UnicodeError, RecursionError) as error:
        raise Rejected("outside generated JavaScript subset") from error


@dataclass(frozen=True)
class Knowledge:
    labels: frozenset = frozenset()
    read: bool = False


EMPTY = Knowledge()
TOP = Knowledge(frozenset(("state", "research", "unknown", "computed")), True)


def join(*values):
    return Knowledge(frozenset(label for value in values for label in value.labels), any(v.read for v in values))


def analyze(program, transparent):
    """May-label analysis of both arms, loop fixed points and transitive modules.

Sequential host runs retain abstract workspace knowledge. Recursive module
cycles use TOP, conservatively; they still execute under the ordinary meter.
Stop/return do not taint following statements: no stop-sensitive repair.
"""
    plans, workspace = {}, {}
    for run in program["runs"]:
        plan, reads, previews, operations = {}, set(), set(), set()
        research = Knowledge(frozenset(("research",))) if run["profile"] == "research" else EMPTY
        inputs = {name: Knowledge(frozenset(("unknown",))) if item["origin"] == "unknown" else EMPTY
                  for name, item in program["inputs"].items()}

        def expr(value, env):
            return env.get(value["ref"], TOP) if "ref" in value else EMPTY

        def note(path, used, result):
            old = plan.get(path, (EMPTY, EMPTY))
            plan[path] = (join(old[0], used), join(old[1], result))

        def merge_env(a, b):
            return {name: join(a.get(name, EMPTY), b.get(name, EMPTY)) for name in a.keys() | b.keys()}

        def block(body, env, prefix, control=EMPTY, stack=()):
            nonlocal workspace
            returned = EMPTY
            for i, op in enumerate(body):
                path, kind = prefix + "/" + str(i), op["op"]
                operations.add(kind)
                args = [expr(e, env) for e in op.get("args", [])]
                args += [expr(op[k], env) for k in ("test", "items", "value", "word", "target", "options", "exact", "coordinates", "source") if k in op]
                used = join(control, *args)
                result = join(used, research)
                if kind == "kernel":
                    if op["row"] in STATE:
                        reads.add(path)
                        result = join(result, Knowledge(frozenset(("state",)), True))
                elif kind == "calc" and op["type"] in ("Word", "Words", "Target", "Targets") and op["fn"] not in transparent:
                    result = join(result, Knowledge(frozenset(("computed",))))
                elif kind == "workspace_get":
                    result = join(result, workspace.get(op["key"], TOP))
                elif kind == "workspace_put":
                    workspace[op["key"]] = join(workspace.get(op["key"], EMPTY), result)
                elif kind == "preview":
                    previews.add(path)
                    result = join(result, Knowledge(frozenset(("state",)), True))
                elif kind == "if":
                    a, b = dict(env), dict(env)
                    wa = dict(workspace)
                    ra = block(op["then"], a, path + "/then", used, stack)
                    after_a = dict(workspace)
                    workspace = wa
                    rb = block(op["else"], b, path + "/else", used, stack)
                    workspace = merge_env(after_a, workspace)
                    env.update(merge_env(a, b))
                    returned = join(returned, ra, rb)
                elif kind == "for":
                    # Only pre-existing variables escape; finite lattice grows
                    # monotonically even for a zero-iteration concrete list.
                    before = dict(env)
                    while True:
                        previous = (dict(env), dict(workspace))
                        local = dict(env)
                        local[op["item"]] = result
                        returned = join(returned, block(op["body"], local, path + "/body", used, stack))
                        for name in before:
                            if name != op["item"]:
                                env[name] = join(env[name], local[name])
                        if previous == (env, workspace):
                            break
                elif kind == "call":
                    name = op["module"]
                    if name in stack:
                        result = join(result, TOP)
                    else:
                        local = dict(inputs)
                        local.update(zip(sorted(program["modules"][name]["parameters"]), args))
                        result = join(result, block(program["modules"][name]["body"], local,
                                                   "modules/" + name + "/body", control, stack + (name,)))
                elif kind == "return":
                    returned = join(returned, result)
                if "as" in op:
                    env[op["as"]] = result
                note(path, used, result)
            return returned

        block(run["body"], dict(inputs), run["id"] + "/body")
        plans[run["id"]] = {"knowledge": plan, "reads": reads, "previews": previews, "operations": operations}
    return plans
