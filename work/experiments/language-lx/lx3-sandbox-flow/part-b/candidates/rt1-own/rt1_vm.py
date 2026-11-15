"""Own typed values, closures, environments, branch context and interpreter.

The shared arithmetic primitive supplies the frozen constituent algorithm; it
does not control execution, policy, labels, bindings or host effects.
"""

from dataclasses import dataclass
from types import MappingProxyType

from harness.calc import calculate
from harness.common import digest, thaw
from harness.meter import Stop

from rt1_check import Flow, STATE_READS, demand, join, value_matches


@dataclass(frozen=True)
class Value:
    type: str
    data: object
    flow: Flow
    host: object


@dataclass(frozen=True)
class Closure:
    parameters: tuple
    body: tuple
    captured: dict


class Returned(Exception):
    def __init__(self, value):
        self.value = value


class Machine:
    def __init__(self, ctx, checked):
        self.ctx, self.checked = ctx, checked
        self.program = checked.program
        self.table = checked.instructions[ctx.run["id"]]
        self.static = ctx.form == "static_labels"
        self.profile_flow = Flow(frozenset({"research"}) if ctx.run["profile"] == "research" else frozenset())
        self.frames = self.constituents = 0
        self.read_state = False
        self.inputs, self.closures = {}, {}

    def wrap(self, value, flow=None):
        demand(value_matches(value.type, value.data), "host value type mismatch")
        if flow is None:
            flow = Flow(frozenset(value.block["labels"]), bool(value.block["read_state"]))
        return Value(value.type, value.data, flow, value)

    def expression(self, expr, env, path):
        if expr.name is not None:
            value = env[expr.name]
            demand(value.type == expr.type, "IR binding type mismatch")
            return value
        return self.wrap(self.ctx.host.make(expr.type, expr.data, path + "/literal", origin="fixture"), Flow())

    def flow(self, node, dependencies, extra=Flow(), *, stored=False):
        dynamic = join(*(v.flow for v in dependencies), extra,
                       Flow() if stored else self.profile_flow)
        if self.static:
            demand(node.flow.contains(dynamic), "stored/runtime flow exceeds static source analysis")
            return node.flow
        if self.ctx.form == "static_runtime_labels":
            demand(node.flow.contains(dynamic), "runtime flow exceeds static upper bound")
        return dynamic

    def make(self, node, kind, data, values=(), control=(), extra=Flow(), *, stored=False, **metadata):
        demand(value_matches(kind, data), "calculation result violates checked type")
        dependencies = tuple(values) + tuple(control)
        flow = self.flow(node, dependencies, extra, stored=stored)
        host = self.ctx.host.make(kind, data, node.path, [v.host for v in dependencies],
                                  labels=flow.labels, read_state=flow.read_state, **metadata)
        return self.wrap(host, flow)

    def gate(self, node, operation, values=(), control=(), **facts):
        dependencies = tuple(values) + tuple(control)
        flow = node.gate if self.static else join(*(v.flow for v in dependencies))
        read = flow.read_state
        if self.ctx.form == "capability_separation":
            read |= self.read_state
        self.ctx.check(operation, [v.host for v in dependencies], labels=flow.labels,
                       read_state=read, **facts)

    def bind(self, env, node, data, values=(), control=(), extra=Flow(), *, logical_bytes=None,
             snapshot=False, retained=False, stored=False, **metadata):
        if not retained:
            self.ctx.meter.bind(data, node.path, logical_bytes=logical_bytes, snapshot=snapshot)
        value = self.make(node, node.type, data, values, control, extra, stored=stored, **metadata)
        env[node.name] = value
        return value

    def block(self, paths, env, control=(), depth=0, top=False, loop_index=None):
        ctx, host, meter = self.ctx, self.ctx.host, self.ctx.meter
        for index, path in enumerate(paths):
            node = self.table[path]
            ctx.position = path
            meter.continuation(path + ":before_step", loop_index=loop_index)
            start, end = node.span
            ctx.executed(path, self.program.source[start:end])
            values = tuple(self.expression(arg, env, path + "/args/" + str(i)) for i, arg in enumerate(node.args))
            op = node.op
            if op == "kernel":
                row = node.details[0]
                facts, metadata = {}, {}
                if row == "setup":
                    facts = {"manual_target": values[0].host.block["origin"] == "manual",
                             "fixed_entry": values[0].data in ctx.kernel.fixture["setups"]}
                self.gate(node, row, values, control, **facts)
                data = ctx.kernel_call(row, [v.host for v in values], path)
                state = row in STATE_READS
                self.read_state |= state
                extra = Flow(frozenset({"state"}) if state else frozenset(), state)
                if row == "setup" and ctx.run["author"] == "built_in" and all(facts.values()):
                    metadata = {"origin": "fixed_entry", "declassifiers": [{
                        "id": "named_setup", "manual_target": thaw(values[0].data),
                        "entry_digest": digest(ctx.kernel.fixture["rows"]["setup"]),
                        "table_digest": digest(ctx.kernel.fixture["setups"]), "output": thaw(data)}]}
                elif row == "caps":
                    metadata = {"origin": "fixed_object_set"}
                self.bind(env, node, data, values, control, extra, snapshot=row == "snapshot",
                          retained=row == "batch_check", **metadata)
            elif op == "calc":
                self.gate(node, "calc", values, control)
                def charge(name, weight):
                    meter.continuation(path + ":calc/" + str(self.constituents) + "/" + name, weight, loop_index)
                    self.constituents += 1
                fn = node.details[0]
                data = calculate(fn, [thaw(v.data) for v in values], charge, ctx.kernel.fixture["inverses"])
                computed = node.type in ("Word", "Words", "Target", "Targets") and fn not in ctx.policy.data["transparent"]
                extra = Flow(frozenset({"computed"}) if computed else frozenset())
                origin = values[0].host.block["origin"] if fn == "identity" else "derived"
                self.bind(env, node, data, values, control, extra, origin=origin)
            elif op == "if":
                self.gate(node, "if", values, control)
                branch = 0 if values[0].data else 1
                self.block(node.branches[branch], env, tuple(control) + values, depth, loop_index=loop_index)
            elif op == "for":
                self.gate(node, "for", values, control)
                items = values[0]
                track = top and len(paths) == 1
                if track:
                    ctx.coverage = {"examined": [], "total": list(range(len(items.data))), "complete": False}
                for i in range(len(items.data) + 1):
                    meter.continuation(path + ":loop_continuation/" + str(i), loop_index=i)
                    if i == len(items.data):
                        break
                    local = dict(env)
                    meter.bind(items.data[i], path + "/item")
                    item = self.make(node, node.type, items.data[i], values, control)
                    local[node.name] = item
                    self.block(node.branches[0], local, tuple(control) + values, depth, loop_index=i)
                    for name in env:
                        if name != node.name:
                            env[name] = local[name]
                    if track:
                        ctx.coverage["examined"].append(i)
            elif op == "call":
                self.gate(node, "call", values, control)
                name = node.details[0]
                closure = self.closures[name]
                if name in self.checked.recursive:
                    self.frames += 1
                    ctx.coverage = {"examined": list(range(self.frames - 1)), "total": list(range(self.frames)), "complete": False}
                    meter.charge("memory", 16, ctx.run["id"] + "/" + name + ":before_frame_" + str(self.frames))
                local = dict(closure.captured)
                for (parameter, _), value in zip(closure.parameters, values):
                    meter.bind(value.data, path + "/parameter/" + parameter)
                    local[parameter] = value
                try:
                    self.block(closure.body, local, control, depth + 1)
                except Returned as returned:
                    self.bind(env, node, returned.value.data, (returned.value,) + values, control)
                else:
                    raise ValueError("module reached end without returning")
            elif op == "workspace_get":
                self.gate(node, op, control=control)
                stored = self.wrap(host.workspace_get(node.details[0]))
                self.read_state |= stored.flow.read_state
                self.bind(env, node, stored.data, (stored,), control, stored=True,
                          origin=stored.host.block["origin"])
            elif op == "workspace_put":
                self.gate(node, op, values, control)
                meter.output(values[0].data, path)
                value = self.make(node, values[0].type, values[0].data, values, control)
                host.workspace_put(node.details[0], value.host)
            elif op == "preview":
                self.gate(node, op, values, control)
                word, target = values
                capture = ctx.run["profile"] != "solving"
                check = ctx.kernel.word_check(word.data)
                public = {"acknowledged": True} if ctx.form == "capability_separation" else {
                    "code": None if capture or check["protected"] else "KN2:protection.piece_would_move",
                    "protected": bool(check["protected"])}
                meter.bind(public, path)
                if capture:
                    meter.output({"word": word.data, "target": target.data, "destination": "capture"}, path)
                extra = Flow() if ctx.form == "capability_separation" else Flow(frozenset({"state"}), True)
                flow = self.flow(node, values + tuple(control), extra)
                receipt = host.preview(word.host, target.host, path, labels=flow.labels,
                                       read_state=flow.read_state, control=[v.host for v in control])
                env[node.name] = self.wrap(receipt, flow)
            elif op == "pick":
                event = self.inputs[node.details[0]]
                dependencies = values + (event,)
                self.gate(node, op, dependencies, control)
                options = values[0]
                chosen = options.data[event.data]
                meter.bind(chosen, path)
                flow = self.flow(node, dependencies + tuple(control))
                # The host's pick API accepts one producer. Its producer block
                # therefore joins options, manual event and all branch controls.
                producer = host.make(options.type, options.data, path + "/options",
                                     [v.host for v in dependencies + tuple(control)],
                                     labels=flow.labels, read_state=flow.read_state,
                                     origin=options.host.block["origin"])
                picked = host.pick(producer, node.details[0], node.type, path,
                                   labels=flow.labels, read_state=flow.read_state)
                env[node.name] = self.wrap(picked, flow)
            elif op == "emit":
                self.gate(node, op, values, control)
                meter.output(values[0].data, path)
                value = self.make(node, values[0].type, values[0].data, values, control)
                host.event("emit", {"value": value.data}, value.host)
            elif op == "publish":
                self.gate(node, op, values, control)
                exact, coordinates = values
                plan = {"exact": {**thaw(exact.data), "source_digest": digest(ctx.program),
                                   "input_digest": digest(ctx.kernel.fixture["session"])},
                        "coordinates": coordinates.data}
                meter.output(plan, path)
                value = self.make(node, "Record", exact.data, values, control)
                ctx.display = host.publish(exact.data, coordinates.data, value.host)
            elif op == "retain":
                self.gate(node, op, control=control)
                size = node.details[0]
                self.bind(env, node, {"logical_bytes": size}, control=control, logical_bytes=size)
            elif op == "ambient":
                self.gate(node, op, control=control, resource=node.details[0])
                data = host.trap(*node.details)
                self.bind(env, node, data, control=control)
            elif op == "dynamic":
                self.gate(node, op, values, control)
                raise ValueError("no dynamic execution instruction exists")
            elif op == "return":
                self.gate(node, "return" if depth == 0 else "call", values, control)
                if depth == 0:
                    meter.output(values[0].data, path)
                raise Returned(self.make(node, values[0].type, values[0].data, values, control))
            elif op == "stop":
                self.gate(node, op, control=control)
                raise Stop("completed", None, "requested", path, loop_index)
            else:
                demand(op == "tick", "unknown typed instruction")
                self.gate(node, op, control=control)
            if top and op != "return" and not (len(paths) == 1 and op in ("for", "call")):
                if not (op == "kernel" and node.details[0] == "batch_check"):
                    ctx.coverage["examined"].append(index)

    def execute(self):
        ctx = self.ctx
        effects = self.checked.effects[ctx.run["id"]]
        grants = set(ctx.bindings["effective_grants"][ctx.run["profile"]])
        separation = ctx.form == "capability_separation" and ctx.run["profile"] == "solving"
        if separation and ("preview", "preview") not in effects:
            grants.discard("preview")
        ctx.bind_grants(grants)
        ctx.layer = "rt1_interpreter"
        ctx.admit()
        if separation:
            # The read grant also names caps (a fixed object set). Shared policy
            # excludes every state row when preview authority is present. Ask it
            # about the transitive read effects before any proposal can be made.
            for op, row in sorted(effects):
                if op == "kernel" and row in STATE_READS:
                    ctx.check(row, labels={"state"}, read_state=True)
        self.inputs = {name: self.wrap(ctx.host.input(name)) for name in self.program.inputs}
        self.closures = {name: Closure(module.parameters, tuple(n.path for n in module.body), MappingProxyType(dict(self.inputs)))
                         for name, module in self.program.modules.items()}
        try:
            paths = tuple(n.path for n in self.program.runs[ctx.run["id"]])
            self.block(paths, dict(self.inputs), top=True)
        except Returned as returned:
            return returned.value.host
        return None
