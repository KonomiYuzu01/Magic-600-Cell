"""RT1 type/effect checking and conservative labels over parsed source bytes.

No policy decisions live here. The executor submits these facts to ctx.check.
The fixture has named modules closing over immutable inputs, not first-class
functions. Recursive signatures and effects are closed before interpretation.
"""

from collections.abc import Mapping
from dataclasses import dataclass, replace

from rt1_syntax import SourceError


STATE_READS = frozenset("counts floors blocked bits nu2 nu5 snapshot word_check score preview_outcome batch_check".split())
ELEMENT = {"Ints": "Int", "Bools": "Bool", "Words": "Word", "Targets": "Target",
           "Word": "Text", "State": "Int", "Matrix": "Ints"}
COLLECTIONS = frozenset(ELEMENT) | {"Rows"}


def demand(condition, message):
    if not condition:
        raise SourceError(message)


def exact_data(data):
    if data is None or type(data) in (int, bool):
        return True
    if isinstance(data, str):
        data.encode("utf-8", "strict")
        return True
    if isinstance(data, (tuple, list)):
        return all(exact_data(x) for x in data)
    if isinstance(data, Mapping):
        return all(type(k) is str and exact_data(k) and exact_data(v) for k, v in data.items())
    return False


def value_matches(kind, data):
    if kind in ("Int", "Bool"):
        return type(data) is {"Int": int, "Bool": bool}[kind]
    if kind in ("Text", "Target"):
        return isinstance(data, str) and exact_data(data)
    if kind in ELEMENT:
        return isinstance(data, (tuple, list)) and all(value_matches(ELEMENT[kind], x) for x in data)
    if kind == "Rows":
        return isinstance(data, (tuple, list)) and exact_data(data)
    if kind in ("Record", "Receipt"):
        return isinstance(data, Mapping) and exact_data(data)
    if kind == "Bytes":
        return (isinstance(data, Mapping) and set(data) == {"logical_bytes"}
                and type(data["logical_bytes"]) is int and data["logical_bytes"] >= 0)
    return False


@dataclass(frozen=True)
class Flow:
    labels: frozenset = frozenset()
    read_state: bool = False

    def contains(self, other):
        return other.labels <= self.labels and (self.read_state or not other.read_state)


def join(*flows):
    return Flow(frozenset().union(*(f.labels for f in flows)), any(f.read_state for f in flows))


@dataclass(frozen=True)
class Info:
    type: str
    flow: Flow = Flow()


@dataclass(frozen=True)
class ExprIR:
    name: str | None
    type: str
    data: object
    flow: Flow


@dataclass(frozen=True)
class Instruction:
    op: str
    path: str
    span: tuple
    name: str | None
    type: str | None
    args: tuple
    details: tuple
    branches: tuple
    gate: Flow
    flow: Flow


@dataclass(frozen=True)
class Checked:
    program: object
    instructions: dict
    effects: dict
    recursive: frozenset


def walk(body):
    for node in body:
        yield node
        for branch in node.branches:
            yield from walk(branch)


def calculation_type(fn, types, result):
    signature = {
        "gt": (("Int", "Int"), "Bool"), "add": (("Int", "Int"), "Int"),
        "argmax_first": (("Ints",), "Int"), "argmin_first": (("Ints",), "Int"),
        "rank_desc_indices": (("Ints",), "Ints"), "sum": (("Ints",), "Int"),
        "mod_dot": (("Ints", "Ints", "Int"), "Int"),
        "linear_solve_mod": (("Matrix", "Ints", "Int"), "Ints"),
        "combine_corrections": (("Words", "Ints"), "Word"),
        "inverse": (("Word",), "Word"), "indexed_points": (("Ints",), "Rows"),
        "point": (("Int", "Int"), "Rows"),
    }
    if fn in signature:
        demand((types, result) == signature[fn], "calculation signature mismatch: " + fn)
    elif fn == "identity":
        demand(types == (result,), "identity type mismatch")
    elif fn == "index":
        demand(len(types) == 2 and types[1] == "Int" and types[0] in COLLECTIONS,
               "index needs a collection and integer")
        demand(types[0] == "Rows" or ELEMENT[types[0]] == result, "index element type mismatch")
    elif fn == "field":
        demand(types in (("Record", "Text"), ("Receipt", "Text")), "field needs a record and name")
        # Open JSON records/Rows have a checked dynamic projection. Its declared
        # result type is tested before a value enters the interpreter environment.
    elif fn == "eq":
        demand(len(types) == 2 and types[0] == types[1] and result == "Bool", "equality type mismatch")
    elif fn == "length":
        demand(len(types) == 1 and types[0] in COLLECTIONS | {"Text"} and result == "Int", "length type mismatch")
    elif fn == "count_equal":
        demand(len(types) == 2 and ELEMENT.get(types[0]) == types[1] and result == "Int", "count type mismatch")
    elif fn == "singleton":
        demand(len(types) == 1 and ELEMENT.get(result) == types[0], "singleton type mismatch")
    elif fn == "append":
        demand(len(types) == 2 and types[0] == result and ELEMENT.get(result) == types[1], "append type mismatch")
    elif fn == "concat":
        demand(types == (result, result) and result in COLLECTIONS, "concat type mismatch")
    elif fn == "zip":
        demand(len(types) == 2 and all(t in COLLECTIONS for t in types) and result == "Rows", "zip type mismatch")
    else:
        raise SourceError("unknown calculation " + fn)


class Checker:
    def __init__(self, program, host_program, rows, transparent):
        self.program, self.rows, self.transparent = program, rows, frozenset(transparent)
        demand(program.inputs == {n: v["type"] for n, v in host_program["inputs"].items()}, "input declarations differ from host")
        demand(list(program.runs) == [r["id"] for r in host_program["runs"]], "run declarations differ from host")
        self.inputs = {n: Info(v["type"], Flow(frozenset({"unknown"}) if v["origin"] == "unknown" else frozenset()))
                       for n, v in host_program["inputs"].items()}
        self.contexts = host_program["runs"]
        self.tables, self.effects, self.module_effects = {}, {}, {}
        for name in program.modules:
            self.module_effects[name] = self.effect_closure(program.modules[name].body, {name})
        self.recursive = frozenset(n for n, effects in self.module_effects.items() if ("call", n) in effects)

    def effect_closure(self, body, seen):
        effects = set()
        for node in walk(body):
            target = node.details[0] if node.op in ("kernel", "call") else node.op
            effects.add((node.op, target))
            if node.op == "call":
                demand(target in self.program.modules, "unknown module " + target)
                if target not in seen:
                    effects |= self.effect_closure(self.program.modules[target].body, seen | {target})
        return effects

    def expression(self, expr, env):
        if expr.name is not None:
            demand(expr.name in env, "unbound name " + expr.name)
            info = env[expr.name]
        else:
            demand(value_matches(expr.type, expr.data), "literal type mismatch")
            info = Info(expr.type)
        return ExprIR(expr.name, info.type, expr.data, info.flow)

    def merge_env(self, left, right, common=True):
        keys = left.keys() & right.keys() if common else left.keys() | right.keys()
        result = {}
        for name in keys:
            a, b = left.get(name), right.get(name)
            if a is None or b is None:
                result[name] = a or b
            else:
                demand(a.type == b.type, "inconsistent joined type " + name)
                result[name] = Info(a.type, join(a.flow, b.flow))
        return result

    def record(self, node, args, gate, flow):
        ir = Instruction(node.op, node.path, node.span, node.name, node.type, args,
                         node.details, tuple(tuple(n.path for n in branch) for branch in node.branches), gate, flow)
        old = self.table.get(node.path)
        if old is not None:
            demand(tuple(a.type for a in old.args) == tuple(a.type for a in args), "inconsistent call-site types")
            ir = replace(ir, gate=join(old.gate, gate), flow=join(old.flow, flow),
                         args=tuple(replace(a, flow=join(a.flow, b.flow)) for a, b in zip(args, old.args)))
        self.table[node.path] = ir

    def module(self, name, arguments, pc, store, active):
        module = self.program.modules[name]
        demand(tuple(a.type for a in arguments) == tuple(t for _, t in module.parameters), "module argument mismatch")
        if name in active:
            effects = self.module_effects[name]
            labels = {"state"} if any(op == "kernel" and row in STATE_READS for op, row in effects) else set()
            if any(n.op == "calc" and n.type in ("Word", "Words", "Target", "Targets")
                   and n.details[0] not in self.transparent for n in walk(module.body)):
                labels.add("computed")
            flow = join(pc, self.profile_flow, *(a.flow for a in arguments), *(v.flow for v in self.inputs.values()),
                        *(v.flow for v in store.values()), Flow(frozenset(labels), "state" in labels))
            return Info(module.result, flow)
        env = dict(self.inputs)
        env.update({n: Info(a.type, a.flow) for (n, _), a in zip(module.parameters, arguments)})
        returns = self.block(module.body, env, pc, store, active | {name})
        demand(bool(returns), "module has no typed return")
        demand(all(v.type == module.result for v in returns), "module return mismatch")
        return Info(module.result, join(*(v.flow for v in returns)))

    def block(self, body, env, pc, store, active=frozenset()):
        returns = []
        for node in body:
            op = node.op
            args = tuple(self.expression(a, env) for a in node.args)
            gate = join(pc, *(a.flow for a in args))
            flow = join(gate, self.profile_flow)
            kinds = tuple(a.type for a in args)
            if op == "kernel":
                row = node.details[0]
                demand(row in self.rows, "unknown kernel row")
                demand(kinds == tuple(self.rows[row]["args"]) and node.type == self.rows[row]["result"], "kernel type mismatch")
                if row in STATE_READS:
                    flow = join(flow, Flow(frozenset({"state"}), True))
            elif op == "calc":
                calculation_type(node.details[0], kinds, node.type)
                if node.type in ("Word", "Words", "Target", "Targets") and node.details[0] not in self.transparent:
                    flow = join(flow, Flow(frozenset({"computed"})))
            elif op == "call":
                result = self.module(node.details[0], args, pc, store, active)
                demand(result.type == node.type, "call result mismatch")
                flow = join(flow, result.flow)
            elif op == "if":
                demand(kinds == ("Bool",), "branch test must be Bool")
                left, right, ls, rs = dict(env), dict(env), dict(store), dict(store)
                returns += self.block(node.branches[0], left, gate, ls, active)
                returns += self.block(node.branches[1], right, gate, rs, active)
                env.clear()
                env.update(self.merge_env(left, right))
                store.clear()
                store.update(self.merge_env(ls, rs, common=False))
                # Deliberately ordinary branch labels: no stop-sensitive repair.
            elif op == "for":
                demand(len(kinds) == 1 and ELEMENT.get(kinds[0]) == node.type, "loop element mismatch")
                initial, initial_store = dict(env), dict(store)
                while True:
                    previous, previous_store = dict(env), dict(store)
                    local = dict(env)
                    local[node.name] = Info(node.type, flow)
                    returns += self.block(node.branches[0], local, gate, store, active)
                    updated = {n: local[n] for n in initial if n != node.name}
                    env.update(self.merge_env(initial, updated))
                    store.update(self.merge_env(initial_store, store, common=False))
                    if env == previous and store == previous_store:
                        break
            elif op == "workspace_get":
                stored = store.get(node.details[0], Info(node.type, Flow(frozenset({"unknown"}), True)))
                demand(stored.type == node.type, "stored type mismatch")
                flow = join(pc, stored.flow)
            elif op == "preview":
                demand(kinds == ("Word", "Target") and node.type == "Receipt", "preview type mismatch")
                if self.form != "capability_separation":
                    flow = join(flow, Flow(frozenset({"state"}), True))
            elif op == "pick":
                demand(len(kinds) == 1 and ELEMENT.get(kinds[0]) == node.type, "pick element mismatch")
                demand(node.details[0] in self.inputs and self.inputs[node.details[0]].type == "Int", "pick index declaration mismatch")
                flow = join(flow, self.inputs[node.details[0]].flow)
            elif op == "publish":
                demand(kinds == ("Record", "Rows"), "display type mismatch")
            elif op == "retain":
                demand(node.type == "Bytes", "retain needs Bytes")
            elif op == "dynamic":
                demand(kinds == ("Text",), "dynamic stimulus needs Text")
            elif op == "ambient":
                demand(node.type == "Text", "trap stimulus needs Text")
                demand(node.details[0] in ("clock", "file", "network", "process"), "unknown resource")
            self.record(node, args, gate, flow)
            flow = self.table[node.path].flow
            if node.name is not None and op != "for":
                demand(node.name not in env or env[node.name].type == node.type, "rebinding changes type")
                env[node.name] = Info(node.type, flow)
            if op == "return":
                returns.append(Info(args[0].type, flow))
            if op == "workspace_put":
                key = node.details[0]
                previous = store.get(key, Info(args[0].type))
                demand(previous.type == args[0].type, "stored key changes type")
                # A later syntactic write might be bypassed by stop/return.
                # Joining all possible writes keeps the storage summary sound
                # without claiming a stop-sensitive information-flow repair.
                store[key] = Info(args[0].type, join(previous.flow, flow))
        return returns

    def check(self, form):
        self.form, store = form, {}
        for run in self.contexts:
            self.table = self.tables[run["id"]] = {}
            self.profile_flow = Flow(frozenset({"research"}) if run["profile"] == "research" else frozenset())
            # Validate even unused modules; discard their analysis facts. Real
            # call sites below specialize labels to parameters and captured inputs.
            for name, module in self.program.modules.items():
                args = tuple(Info(t) for _, t in module.parameters)
                self.module(name, args, Flow(), dict(store), frozenset())
            self.table.clear()
            body = self.program.runs[run["id"]]
            self.effects[run["id"]] = self.effect_closure(body, set())
            initial_store = dict(store)
            while True:
                before = dict(self.table)
                store = dict(initial_store)
                self.block(body, dict(self.inputs), Flow(), store)
                if self.table == before:
                    break
        return Checked(self.program, self.tables, self.effects, self.recursive)
