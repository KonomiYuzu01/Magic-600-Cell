"""Throwaway RT3 front end, finite label analysis, and handwritten Wasm encoder.

No policy decisions live here. STATE_ROWS and transparent-operation semantics
come from the shared bench; the generated code supplies knowledge to ctx.check.
"""

from collections import Counter

from harness.policy import STATE_ROWS
from render import instructions

STATE, COMPUTED, RESEARCH, UNKNOWN, READ = 1, 2, 4, 8, 16
LABELS = {"state": STATE, "computed": COMPUTED, "research": RESEARCH, "unknown": UNKNOWN}
TOP = STATE | COMPUTED | RESEARCH | UNKNOWN | READ


def bits(value):
    return sum(LABELS[label] for label in value.labels) | (READ if value.read_state else 0)


def labels(mask):
    return [name for name, bit in LABELS.items() if mask & bit]


def join(values):
    mask = 0
    for value in values:
        mask |= value
    return mask


def operands(node):
    kind = node[0]
    if kind in ("kernel", "calc", "call"):
        return node[5]
    if kind in ("if", "return", "emit", "dynamic"):
        return [node[2]]
    if kind == "for":
        return [node[4]]
    if kind == "workspace_put":
        return [node[3]]
    if kind == "preview":
        return node[4:6]
    if kind == "pick":
        return [node[4]]
    if kind == "publish":
        return node[2:4]
    return []


class Analysis:
    """Monotone branch/loop joins, module summaries, and persistent workspace."""
    def __init__(self, surface, transparent):
        self.surface, self.transparent = surface, set(transparent)
        self.input_flags = {name: UNKNOWN if value[2] == "unknown" else 0
                            for name, value in surface["inputs"].items()}
        self.info, self.workspace = {}, {}
        self.profile = 0

    def expression(self, expr, env):
        if expr[0] == "use":
            return env[expr[1]]
        if expr[0] != "lit" or len(expr) != 3:
            raise ValueError("invalid RT3 expression")
        return 0

    def block(self, body, env, pc=0, stack=()):
        returned = 0
        for node in body:
            kind, path = node[:2]
            args = [self.expression(expr, env) for expr in operands(node)]
            incoming = join(args) | pc | self.profile
            outgoing = incoming
            if kind == "kernel" and node[4] in STATE_ROWS:
                outgoing |= STATE | READ
            elif kind == "calc" and node[3] in ("Word", "Words", "Target", "Targets") and node[4] not in self.transparent:
                outgoing |= COMPUTED
            elif kind == "preview":
                outgoing |= STATE | READ
            elif kind == "workspace_get":
                outgoing |= self.workspace.get(node[4], UNKNOWN)
            elif kind == "workspace_put":
                self.workspace[node[2]] = outgoing
            elif kind == "call":
                name = node[4]
                module = self.surface["modules"][name]
                if name in stack:
                    result = TOP  # A conservative finite recursion summary.
                else:
                    local = dict(self.input_flags)
                    if len(module[0]) != len(args):
                        raise ValueError("RT3 module arity")
                    local.update(zip(sorted(module[0]), args))
                    result = self.block(module[2], local, pc, stack + (name,))
                outgoing |= result
            elif kind == "if":
                before = dict(self.workspace)
                left, right = dict(env), dict(env)
                returned |= self.block(node[3], left, pc | args[0], stack)
                left_workspace = dict(self.workspace)
                self.workspace = dict(before)
                returned |= self.block(node[4], right, pc | args[0], stack)
                self.workspace = {k: left_workspace.get(k, before.get(k, UNKNOWN)) |
                                  self.workspace.get(k, before.get(k, UNKNOWN))
                                  for k in left_workspace.keys() | self.workspace.keys()}
                for name in left.keys() & right.keys():
                    env[name] = left[name] | right[name]
            elif kind == "for":
                # Include zero iterations and converge all loop-carried masks.
                while True:
                    before, before_workspace = dict(env), dict(self.workspace)
                    local = dict(env)
                    local[node[2]] = args[0] | pc | self.profile
                    returned |= self.block(node[5], local, pc | args[0], stack)
                    for name in env:
                        if name != node[2]:
                            env[name] |= local[name]
                    self.workspace = {k: before_workspace.get(k, UNKNOWN) | self.workspace.get(k, UNKNOWN)
                                      for k in before_workspace.keys() | self.workspace.keys()}
                    if env == before and self.workspace == before_workspace:
                        break
            elif kind == "return":
                returned |= outgoing
            if kind in ("kernel", "calc", "call", "workspace_get", "preview", "pick", "retain", "ambient"):
                env[node[2]] = outgoing
            old = self.info.get(path, (0, 0))
            self.info[path] = (old[0] | incoming, old[1] | outgoing)
        return returned

    def run(self, run_id):
        for name in self.surface["entry_order"]:
            entry = self.surface["entries"][name]
            self.info = {}
            self.profile = RESEARCH if entry[0] == "research" else 0
            self.block(entry[4], dict(self.input_flags))
            if name == run_id:
                return self.info
        raise ValueError("unknown RT3 entry")


def uleb(value):
    out = bytearray()
    while True:
        part = value & 127
        value >>= 7
        out.append(part | (128 if value else 0))
        if not value:
            return bytes(out)


def sleb(value):
    out = bytearray()
    while True:
        part = value & 127
        value >>= 7
        done = (value == 0 and not part & 64) or (value == -1 and part & 64)
        out.append(part | (0 if done else 128))
        if done:
            return bytes(out)


def vector(items):
    return uleb(len(items)) + b"".join(items)


def name(value):
    raw = value.encode("utf-8")
    return uleb(len(raw)) + raw


def section(number, data):
    return bytes([number]) + uleb(len(data)) + data


# Imported functions are the entire authority of the module: no WASI, memory,
# table, eval, filesystem, network, clock, process, or arbitrary import name.
# i32 carries only handles, indexes and label bits; user integers stay in Python.
I32, I64 = 0x7F, 0x7E
IMPORTS = [
    ("input", [I32], [I32]), ("literal", [I32], [I32]),
    ("meta", [I32], [I32]), ("arg", [I32], []),
    ("step", [I32, I32], [I64]), ("meter", [I32], [I64]),
    ("perform", [I32, I32, I32, I32, I32], [I32]),
    ("truth", [I32], [I32]), ("length", [I32], [I32]),
    ("control", [I32, I32], [I32]),
    ("loop_begin", [I32, I32], []), ("loop_test", [I32, I32], [I64]),
    ("item", [I32, I32, I32, I32, I32], [I32]),
    ("loop_done", [I32, I32], []), ("done", [I32], []),
    ("load", [I32], [I32]), ("call_enter", [I32, I32, I32], []),
    ("call_exit", [I32, I32, I32, I32], [I32]),
]


class Function:
    def __init__(self, owner, params):
        self.owner, self.params = owner, params
        self.locals, self.code = [], bytearray()

    def local(self):
        index = len(self.params) + len(self.locals)
        self.locals.append(I32)
        return index

    def pair(self):
        return self.local(), self.local()

    def op(self, opcode):
        self.code.append(opcode)

    def const(self, value):
        self.code.extend(b"\x41" + sleb(value))

    def get(self, local):
        self.code.extend(b"\x20" + uleb(local))

    def put(self, local):
        self.code.extend(b"\x21" + uleb(local))

    def call(self, target):
        self.owner.calls[target] += 1
        index = self.owner.indices[target]
        self.code.extend(b"\x10" + uleb(index))

    def fuel(self):
        self.code.extend(b"\x24\x00")  # Exact cumulative steps, mutable i64 global.

    def flags(self, values, pc, extra=0):
        self.get(pc)
        if extra:
            self.const(extra)
            self.op(0x72)  # i32.or
            self.owner.label_or_sites += 1
        for value in values:
            self.get(value[1])
            self.op(0x72)
            self.owner.label_or_sites += 1

    def body_bytes(self):
        groups = [uleb(len(self.locals)) + bytes([I32])] if self.locals else []
        body = vector(groups) + bytes(self.code) + b"\x0b"
        return uleb(len(body)) + body


class Compiler:
    def __init__(self, surface, run_id, form, transparent):
        if surface["edition"] != "rt3-array-1":
            raise ValueError("unknown RT3 edition")
        self.surface, self.run_id, self.form = surface, run_id, form
        self.transparent = set(transparent)
        self.static = form == "static_labels"
        self.analysis = Analysis(surface, transparent).run(run_id)
        self.nodes = instructions(surface)
        self.paths = sorted(self.nodes)
        self.opids = {path: i for i, path in enumerate(self.paths)}
        self.inputs = sorted(surface["inputs"])
        self.literals, self.calls = [], Counter()
        self.indices = {entry[0]: i for i, entry in enumerate(IMPORTS)}
        self.module_names = sorted(surface["modules"])
        self.functions = ["module/" + key for key in self.module_names] + ["entry", "charge"]
        self.indices.update({key: len(IMPORTS) + i for i, key in enumerate(self.functions)})
        self.profile = RESEARCH if surface["entries"][run_id][0] == "research" else 0
        self.value_fields = 0
        self.control_fields = self.temporary_fields = self.label_or_sites = 0

    def expression(self, fn, expr, env, path):
        if expr[0] == "use":
            return env[expr[1]]
        pair = fn.pair()
        index = len(self.literals)
        self.literals.append([expr[1], expr[2], path + "/literal"])
        fn.const(index)
        fn.call("literal")
        fn.put(pair[0])
        fn.const(0)
        fn.put(pair[1])
        self.value_fields += 1
        return pair

    def joined(self, fn, args, pc, node, output=False):
        if self.static:
            fn.const(self.analysis.get(node[1], (TOP, TOP))[int(output)])
        else:
            extra = self.profile
            if output and node[0] == "kernel" and node[4] in STATE_ROWS:
                extra |= STATE | READ
            if output and node[0] == "calc" and node[3] in ("Word", "Words", "Target", "Targets") and node[4] not in self.transparent:
                extra |= COMPUTED
            if output and node[0] == "preview" and self.form != "capability_separation":
                extra |= STATE | READ
            fn.flags(args, pc, extra)

    def arguments(self, fn, args):
        for pair in args:
            fn.get(pair[0])
            fn.call("arg")

    def bind(self, fn, env, target):
        if target not in env:
            env[target] = fn.pair()
            self.value_fields += 1
        return env[target]

    def block(self, fn, body, env, pc_h, pc_f, loop):
        for node in body:
            kind, path = node[:2]
            opid = self.opids[path]
            fn.const(opid)
            fn.get(loop)
            fn.call("step")
            fn.fuel()
            args = [self.expression(fn, expr, env, path + "/operand/" + str(i))
                    for i, expr in enumerate(operands(node))]
            if kind == "if":
                # The control handle preserves provenance parts, and the OR
                # preserves implicit flow even when both arms produce literals.
                child_h, child_f = fn.pair()
                self.control_fields += 1
                fn.get(pc_h)
                fn.get(args[0][0])
                fn.call("control")
                fn.put(child_h)
                if self.static:
                    fn.const(self.analysis.get(path, (TOP, TOP))[0])
                else:
                    fn.flags(args, pc_f)
                fn.put(child_f)
                # Preallocate names defined on BOTH arms; never invent a phi
                # value for a name defined only on the unexecuted arm.
                def assigned(branch):
                    result = set()
                    for item in branch:
                        if item[0] in ("kernel", "calc", "call", "workspace_get", "preview", "pick", "retain", "ambient"):
                            result.add(item[2])
                        elif item[0] == "if":
                            result |= assigned(item[3]) & assigned(item[4])
                    return result
                for target in sorted(assigned(node[3]) & assigned(node[4])):
                    self.bind(fn, env, target)
                fn.get(args[0][0])
                fn.call("truth")
                fn.code.extend(b"\x04\x40")
                self.block(fn, node[3], dict(env), child_h, child_f, loop)
                fn.op(0x05)
                self.block(fn, node[4], dict(env), child_h, child_f, loop)
                fn.op(0x0B)
            elif kind == "for":
                child_h, child_f = fn.pair()
                self.control_fields += 1
                index, length = fn.pair()
                fn.get(pc_h)
                fn.get(args[0][0])
                fn.call("control")
                fn.put(child_h)
                if self.static:
                    fn.const(self.analysis.get(path, (TOP, TOP))[0])
                else:
                    fn.flags(args, pc_f, self.profile)
                fn.put(child_f)
                fn.const(opid)
                fn.get(args[0][0])
                fn.call("loop_begin")
                fn.get(args[0][0])
                fn.call("length")
                fn.put(length)
                fn.const(0)
                fn.put(index)
                fn.code.extend(b"\x02\x40\x03\x40")  # block { loop {
                fn.const(opid)
                fn.get(index)
                fn.call("loop_test")
                fn.fuel()
                fn.get(index)
                fn.get(length)
                fn.op(0x4F)  # i32.ge_u
                fn.code.extend(b"\x0d\x01")  # br_if outer block
                local = dict(env)
                item = fn.pair()
                self.value_fields += 1
                local[node[2]] = item
                for value in (opid,):
                    fn.const(value)
                for value in (args[0][0], index, child_f, pc_h):
                    fn.get(value)
                fn.call("item")
                fn.put(item[0])
                fn.get(child_f)
                fn.put(item[1])
                self.block(fn, node[5], local, child_h, child_f, index)
                fn.const(opid)
                fn.get(index)
                fn.call("loop_done")
                fn.get(index)
                fn.const(1)
                fn.op(0x6A)
                fn.put(index)
                fn.code.extend(b"\x0c\x00\x0b\x0b")
            elif kind == "call":
                self.arguments(fn, args)
                fn.const(opid)
                self.joined(fn, args, pc_f, node)
                fn.get(pc_h)
                fn.call("call_enter")
                fn.get(pc_h)
                fn.get(pc_f)
                fn.const(-1)
                for pair in args:
                    fn.get(pair[0])
                    fn.get(pair[1])
                fn.call("module/" + node[4])
                result = fn.pair()
                self.value_fields += 1
                fn.put(result[1])
                fn.put(result[0])
                target = self.bind(fn, env, node[2])
                self.joined(fn, args + [result], pc_f, node, True)
                fn.put(target[1])
                self.arguments(fn, args + [result])
                fn.const(opid)
                fn.get(target[1])
                fn.get(pc_h)
                fn.get(result[0])
                fn.call("call_exit")
                fn.put(target[0])
            else:
                if kind == "workspace_get":
                    raw = fn.pair()
                    self.value_fields += 1
                    fn.const(opid)
                    fn.call("load")
                    fn.put(raw[0])
                    if self.static:
                        fn.const(0)  # The static summary, not this slot, is used.
                    else:
                        fn.get(raw[0])
                        fn.call("meta")
                    fn.put(raw[1])
                    args = [raw]
                incoming, outgoing = fn.pair()
                self.temporary_fields += 2
                self.joined(fn, args, pc_f, node)
                fn.put(incoming)
                self.joined(fn, args, pc_f, node, True)
                fn.put(outgoing)
                self.arguments(fn, args)
                fn.const(opid)
                fn.get(incoming)
                fn.get(outgoing)
                fn.get(pc_h)
                fn.get(loop)
                fn.call("perform")
                if kind == "return":
                    fn.get(outgoing)
                    fn.op(0x0F)
                elif kind in ("kernel", "calc", "workspace_get", "preview", "pick", "retain", "ambient"):
                    target = self.bind(fn, env, node[2])
                    fn.put(target[0])
                    fn.get(outgoing)
                    fn.put(target[1])
                else:
                    fn.op(0x1A)
            fn.const(opid)
            fn.call("done")

    def function(self, body, parameters):
        fn = Function(self, [I32] * (3 + 2 * len(parameters)))
        self.control_fields += 1
        env = {key: (3 + 2 * i, 4 + 2 * i) for i, key in enumerate(parameters)}
        self.value_fields += len(parameters)
        for i, key in enumerate(self.inputs):
            if key in env:
                continue
            pair = self.bind(fn, env, key)
            fn.const(i)
            fn.call("input")
            fn.put(pair[0])
            if self.static:
                fn.const(UNKNOWN if self.surface["inputs"][key][2] == "unknown" else 0)
            else:
                fn.get(pair[0])
                fn.call("meta")
            fn.put(pair[1])
        self.block(fn, body, env, 0, 1, 2)
        fn.const(0)
        fn.const(0)
        return fn

    def compile(self):
        compiled = [self.function(self.surface["modules"][key][2], sorted(self.surface["modules"][key][0]))
                    for key in self.module_names]
        compiled.append(self.function(self.surface["entries"][self.run_id][4], []))
        # Reentrant fuel trampoline: every scalar/collection constituent of the
        # shared exact primitive calls this Wasm export BEFORE that constituent.
        fuel = Function(self, [I32])
        fuel.get(0)
        fuel.call("meter")
        fuel.fuel()
        compiled.append(fuel)
        signatures = [(params, results) for _, params, results in IMPORTS]
        signatures += [(fn.params, [] if i == len(compiled) - 1 else [I32, I32]) for i, fn in enumerate(compiled)]
        types = vector([b"\x60" + vector([bytes([t]) for t in params]) + vector([bytes([t]) for t in results])
                        for params, results in signatures])
        imports = vector([name("host") + name(key) + b"\x00" + uleb(i) for i, (key, _, _) in enumerate(IMPORTS)])
        funcs = vector([uleb(len(IMPORTS) + i) for i in range(len(compiled))])
        globals_ = vector([bytes([I64, 1, 0x42, 0, 0x0B])])
        exports = vector([name("run") + b"\x00" + uleb(self.indices["entry"]),
                          name("charge") + b"\x00" + uleb(self.indices["charge"]),
                          name("fuel") + b"\x03\x00"])
        code = vector([fn.body_bytes() for fn in compiled])
        wasm = b"\x00asm\x01\x00\x00\x00" + section(1, types) + section(2, imports) + section(3, funcs) + section(6, globals_) + section(7, exports) + section(10, code)
        # Reachable effects, including either arm and transitive module calls.
        seen, state_read = set(), False
        def effects(body):
            nonlocal state_read
            for node in body:
                if node[0] == "kernel" and node[4] in STATE_ROWS:
                    state_read = True
                if node[0] == "call" and node[4] not in seen:
                    seen.add(node[4])
                    effects(self.surface["modules"][node[4]][2])
                if node[0] == "if":
                    effects(node[3])
                    effects(node[4])
                if node[0] == "for":
                    effects(node[5])
        effects(self.surface["entries"][self.run_id][4])
        return {"wasm_hex": wasm.hex(), "literals": self.literals,
                "state_read_effect": state_read,
                "counts": {"wasm_bytes": len(wasm), "value_label_fields": self.value_fields,
                           "control_label_fields": self.control_fields,
                           "temporary_label_fields": self.temporary_fields,
                           "function_label_result_fields": len(compiled) - 1,
                           "label_or_sites": self.label_or_sites,
                           "i32_locals_and_parameters": sum(len(fn.params) + len(fn.locals) for fn in compiled),
                           "fuel_i64_fields": 1, "static_call_sites": dict(sorted(self.calls.items())),
                           "static_label_joins": self.static}}


def compile_program(surface, run_id, form, transparent):
    return Compiler(surface, run_id, form, transparent).compile()
