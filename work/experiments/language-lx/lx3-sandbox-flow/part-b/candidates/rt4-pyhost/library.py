"""Domain library for actual Python programs; no neutral-op execution loop.

Wrapped values carry explicit dependencies. __bool__ deliberately cannot tell
the library which native branch runs. That limitation is the run-time attempt.
"""

from dataclasses import dataclass

from harness.calc import calculate
from harness.common import digest, thaw
from harness.meter import Stop
from harness.policy import STATE_ROWS


@dataclass(frozen=True)
class Wrapped:
    value: object

    def __bool__(self):
        return bool(self.value.data)


class Library:
    def __init__(self, ctx, fragments, functions, analysis=None):
        self.ctx, self.fragments = ctx, fragments
        self.functions, self.analysis = functions, analysis
        self.created, self.inputs = {}, {}
        self.frames, self.constituents = 0, 0
        self.read_seen = False

    def input(self, name):
        if name not in self.inputs:
            self.inputs[name] = self.remember("input/" + name, self.ctx.host.input(name))
        return self.inputs[name]

    def literal(self, kind, data, entry):
        return self.remember(entry, self.ctx.host.make(kind, data, entry, origin="fixture"))

    def remember(self, path, value):
        self.created[path] = value
        self.read_seen |= value.read_state
        return Wrapped(value)

    def knowledge(self, path, values, *, output=False):
        dependencies = [value.value for value in values]
        labels = {label for value in dependencies for label in value.labels}
        reads = any(value.read_state for value in dependencies)
        if self.analysis is not None:
            info = (self.analysis.outputs.get(path) if output else
                    self.analysis.gates.get(path, (None, None, None))[1])
            if info is not None:
                labels.update(info.labels)
                reads |= info.reads
                seen = {digest(value.block) for value in dependencies}
                for reference in sorted(info.references):
                    value = self.created.get(reference)
                    if value is not None and digest(value.block) not in seen:
                        dependencies.append(value)
                        seen.add(digest(value.block))
                        labels.update(value.labels)
                        reads |= value.read_state
        return dependencies, labels, reads

    def gate(self, operation, path, values=(), **facts):
        self.ctx.position = path
        dependencies, labels, reads = self.knowledge(path, values)
        if operation == "preview" and self.ctx.form == "capability_separation":
            reads |= self.read_seen
        self.ctx.check(operation, dependencies, labels=labels, read_state=reads, **facts)

    def make(self, kind, data, path, values=(), *, extra_labels=(), state=False, **metadata):
        dependencies, labels, reads = self.knowledge(path, values, output=True)
        labels.update(extra_labels)
        if self.ctx.run["profile"] == "research":
            labels.add("research")
        value = self.ctx.host.make(kind, data, path, dependencies, labels=labels,
                                   read_state=reads or state, **metadata)
        return self.remember(path, value)

    def bind(self, value, path, **size):
        self.ctx.meter.bind(value.value.data, path, **size)
        return value

    def step(self, path, loop_index):
        self.ctx.position = path
        self.ctx.meter.continuation(path + ":before_step", loop_index=loop_index)
        self.ctx.executed(path, self.fragments[path])

    def done(self, index):
        self.ctx.coverage["examined"].append(index)

    def kernel(self, path, kind, row, values, loop_index):
        facts = {}
        if row == "setup":
            facts = {"manual_target": values[0].value.block["origin"] == "manual", "fixed_entry": True}
        self.gate(row, path, values, **facts)
        data = self.ctx.kernel_call(row, [value.value for value in values], path)
        metadata = {}
        if row == "setup":
            metadata = {"origin": "fixed_entry", "declassifiers": [{
                "id": "named_setup", "manual_target": thaw(values[0].value.data),
                "entry_digest": digest(self.ctx.kernel.fixture["rows"]["setup"]),
                "table_digest": digest(self.ctx.kernel.fixture["setups"]), "output": thaw(data)}]}
        elif row == "caps":
            metadata = {"origin": "fixed_object_set"}
        value = self.make(kind, data, path, values, state=row in STATE_ROWS,
                          extra_labels=["state"] if row in STATE_ROWS else [], **metadata)
        return value if row == "batch_check" else self.bind(value, path, snapshot=row == "snapshot")

    def calc(self, path, kind, function, values, loop_index):
        self.gate("calc", path, values)

        def charge(name, weight):
            self.ctx.meter.continuation(path + ":calc/" + str(self.constituents) + "/" + name, weight, loop_index)
            self.constituents += 1

        data = calculate(function, [thaw(value.value.data) for value in values], charge,
                         self.ctx.kernel.fixture["inverses"])
        computed = kind in ("Word", "Words", "Target", "Targets") and function not in self.ctx.policy.data["transparent"]
        return self.bind(self.make(kind, data, path, values,
                                  extra_labels=["computed"] if computed else []), path)

    def iterate(self, path, items, kind, track):
        self.gate("for", path, [items])
        self.created[path] = items.value
        if track:
            self.ctx.coverage = {"examined": [], "total": list(range(len(items.value.data))), "complete": False}
        for index in range(len(items.value.data) + 1):
            self.ctx.meter.continuation(path + ":loop_continuation/" + str(index), loop_index=index)
            if index == len(items.value.data):
                break
            value = self.bind(self.make(kind, items.value.data[index], path + "/item", [items]), path + "/item")
            yield index, value
            if track:
                self.ctx.coverage["examined"].append(index)

    def call(self, path, kind, function, values):
        self.gate("call", path, values)
        description = self.functions[function.__name__]
        # The frozen recursion tariff retains each direct recursive frame.
        import ast
        recursive = any(isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
                        and node.func.attr == "call" and node.args[2].id == function.__name__
                        for node in ast.walk(description))
        if recursive:
            self.frames += 1
            name = bytes.fromhex(function.__name__[2:]).decode("utf-8")
            self.ctx.coverage = {"examined": list(range(self.frames - 1)),
                                 "total": list(range(self.frames)), "complete": False}
            self.ctx.meter.charge("memory", 16, self.ctx.run["id"] + "/" + name + ":before_frame_" + str(self.frames))
        for parameter, value in zip(description.args.args[1:], values):
            name = bytes.fromhex(parameter.arg[2:]).decode("utf-8")
            self.bind(value, path + "/parameter/" + name)
        returned = function(self, *values)
        if not isinstance(returned, Wrapped):
            raise ValueError("module did not return a domain value")
        return self.bind(self.make(kind, returned.value.data, path, [returned] + list(values)), path)

    def workspace_get(self, path, key):
        value = self.remember(path, self.ctx.host.workspace_get(key))
        self.gate("workspace_get", path, [value])
        return self.bind(value, path)

    def workspace_put(self, path, key, value):
        self.gate("workspace_put", path, [value])
        value = self.make(value.value.type, value.value.data, path, [value])
        self.ctx.meter.output(value.value.data, path)
        self.ctx.host.workspace_put(key, value.value)

    def preview(self, path, word, target):
        self.gate("preview", path, [word, target])
        dependencies, labels, reads = self.knowledge(path, [word, target])
        control = dependencies[2:]
        check = self.ctx.kernel.word_check(word.value.data)
        capture = self.ctx.run["profile"] != "solving"
        public = {"acknowledged": True} if self.ctx.form == "capability_separation" else {
            "code": None if capture or check["protected"] else "KN2:protection.piece_would_move",
            "protected": bool(check["protected"])}
        self.ctx.meter.bind(public, path)
        if capture:
            self.ctx.meter.output({"word": word.value.data, "target": target.value.data, "destination": "capture"}, path)
        return self.remember(path, self.ctx.host.preview(word.value, target.value, path,
                                                        labels=labels, read_state=reads, control=control))

    def pick(self, path, kind, options, event):
        self.gate("pick", path, [options])
        dependencies, labels, reads = self.knowledge(path, [options], output=True)
        if len(dependencies) > 1:
            options = Wrapped(self.ctx.host.make(options.value.type, options.value.data, path + "/options",
                                                 dependencies, labels=labels, read_state=reads,
                                                 origin=options.value.block["origin"]))
        chosen = options.value.data[self.ctx.program["inputs"][event]["value"]]
        self.ctx.meter.bind(chosen, path)
        return self.remember(path, self.ctx.host.pick(options.value, event, kind, path, labels=labels, read_state=reads))

    def emit(self, path, value):
        self.gate("emit", path, [value])
        value = self.make(value.value.type, value.value.data, path, [value])
        self.ctx.meter.output(value.value.data, path)
        self.ctx.host.event("emit", {"value": value.value.data}, value.value)

    def publish(self, path, exact, coordinates):
        self.gate("publish", path, [exact, coordinates])
        value = self.make("Record", exact.value.data, path, [exact, coordinates])
        plan = {"exact": {**thaw(exact.value.data), "source_digest": digest(self.ctx.program),
                           "input_digest": digest(self.ctx.kernel.fixture["session"])},
                "coordinates": coordinates.value.data}
        self.ctx.meter.output(plan, path)
        self.ctx.display = self.ctx.host.publish(exact.value.data, coordinates.value.data, value.value)

    def retain(self, path, size):
        self.gate("retain", path)
        return self.bind(self.make("Bytes", {"logical_bytes": size}, path), path, logical_bytes=size)

    def ambient(self, path, kind, resource, logical_resource):
        self.gate("ambient", path, resource=resource)
        return self.bind(self.make(kind, self.ctx.host.trap(resource, logical_resource), path), path)

    def dynamic(self, path, source):
        self.gate("dynamic", path, [source])
        raise ValueError("dynamic evaluation has no library implementation")

    def result(self, path, value, top):
        self.gate("return", path, [value])
        value = self.make(value.value.type, value.value.data, path, [value])
        if top:
            self.ctx.meter.output(value.value.data, path)
        return value

    def stop(self, path, loop_index):
        self.gate("stop", path)
        raise Stop("completed", None, "requested", path, loop_index)
