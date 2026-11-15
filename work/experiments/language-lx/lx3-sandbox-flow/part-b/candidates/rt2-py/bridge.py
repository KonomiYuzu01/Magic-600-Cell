"""Trusted, metered domain bindings for native CPython control flow.

The checker supplies fixed annotations. This is not a second interpreter or a
VM modification; the VM executes functions, branches, loops and calls itself.
"""

import ast
import json

from harness.calc import calculate
from harness.common import digest, thaw
from harness.meter import Stop
from harness.policy import STATE_ROWS


class Bridge:
    def __init__(self, ctx, source, spans, analysis):
        self.ctx, self.source, self.spans, self.analysis = ctx, source, spans, analysis
        self.host, self.meter = ctx.host, ctx.meter
        self.loop_index, self.frames, self.constituents = None, 0, 0
        self.call_controls, self.loop_controls = [], []
        self.read_state = False
        self.kinds = {node.args[0].value: node.func.id for node in ast.walk(analysis.tree)
                      if isinstance(node, ast.Call) and node.func.id not in ("h_literal", "h_input", "h_truth", "h_done")}
        self.inputs = {name: self.host.input(name) for name in ctx.program["inputs"]}

    def namespace(self):
        names = ("input", "literal", "truth", "done", "kernel", "calc", "call", "condition", "iter",
                 "workspace_get", "workspace_put", "preview", "pick", "emit", "publish", "return",
                 "retain", "ambient", "stop", "tick")
        return {"__builtins__": {}, **{"h_" + name: getattr(self, "h_" + name) for name in names}}

    def controls(self, explicit):
        return self.call_controls + self.loop_controls + list(explicit)

    def begin(self, path, operation, values=(), controls=(), **facts):
        ctx = self.ctx
        ctx.position = path
        self.meter.continuation(path + ":before_step", loop_index=self.loop_index)
        span = self.spans[path]
        ctx.executed(path, self.source[span["start"]:span["end"]])
        knowledge = self.analysis.marks[path][0]
        ctx.check(operation, list(values) + self.controls(controls), labels=knowledge.labels,
                  read_state=knowledge.read or self.read_state, **facts)

    def make(self, path, kind, data, dependencies=(), controls=(), *, entry=None, **extra):
        info = self.analysis.marks[path][1]
        return self.host.make(kind, data, entry or path, list(dependencies) + self.controls(controls),
                              labels=info.labels, read_state=info.read, **extra)

    def bind(self, path, value, **charges):
        self.meter.bind(value.data, path, **charges)
        self.h_done(path)
        return value

    def h_done(self, path):
        parts = path.split("/")
        if len(parts) == 3 and parts[:2] == [self.ctx.run["id"], "body"]:
            if len(self.ctx.run["body"]) == 1 and self.kinds[path] in ("h_iter", "h_call"):
                return
            self.ctx.coverage["examined"].append(int(parts[2]))

    def h_input(self, name):
        return self.inputs[name]

    def h_literal(self, kind, data, entry):
        # JSON literal bytes were validated by the AST checker. No eval here.
        return self.host.make(kind, json.loads(data), entry, origin="fixture")

    def h_kernel(self, path, kind, row, values, controls):
        facts = {}
        if row == "setup":
            facts = {"manual_target": bool(values) and values[0].block["origin"] == "manual",
                     "fixed_entry": True}
        self.begin(path, row, values, controls, **facts)
        data = self.ctx.kernel_call(row, values, path)
        extra = {}
        if row in STATE_ROWS:
            self.read_state = True
        if row == "setup":
            extra = {"origin": "fixed_entry", "declassifiers": [{
                "id": "named_setup", "manual_target": thaw(values[0].data),
                "entry_digest": digest(self.ctx.kernel.fixture["rows"]["setup"]),
                "table_digest": digest(self.ctx.kernel.fixture["setups"]), "output": thaw(data)}]}
        elif row == "caps":
            extra = {"origin": "fixed_object_set"}
        value = self.make(path, kind, data, values, controls, **extra)
        if row == "batch_check":
            return value  # The host already charged the retained buffer and coverage.
        return self.bind(path, value, snapshot=row == "snapshot")

    def h_calc(self, path, kind, function, values, controls):
        self.begin(path, "calc", values, controls)

        def charge(name, weight):
            self.meter.continuation(path + ":calc/" + str(self.constituents) + "/" + name,
                                    weight, self.loop_index)
            self.constituents += 1
        data = calculate(function, [thaw(v.data) for v in values], charge, self.ctx.kernel.fixture["inverses"])
        return self.bind(path, self.make(path, kind, data, values, controls))

    def h_condition(self, path, test, controls):
        self.begin(path, "if", [test], controls)
        return test

    def h_truth(self, test):
        return bool(test.data)

    def h_iter(self, path, kind, items, controls):
        self.begin(path, "for", [items], controls)
        track = path == self.ctx.run["id"] + "/body/0" and len(self.ctx.run["body"]) == 1
        if track:
            self.ctx.coverage = {"examined": [], "total": list(range(len(items.data))), "complete": False}
        old_index, old_controls = self.loop_index, self.loop_controls
        try:
            for index in range(len(items.data) + 1):
                self.meter.continuation(path + ":loop_continuation/" + str(index), loop_index=index)
                if index == len(items.data):
                    break
                self.loop_index = index
                self.loop_controls = old_controls + [items]
                item = self.make(path, kind, items.data[index], [items], controls, entry=path + "/item")
                self.meter.bind(item.data, path + "/item")
                yield item
                if track:
                    self.ctx.coverage["examined"].append(index)
        finally:
            self.loop_index, self.loop_controls = old_index, old_controls

    def h_call(self, path, kind, function, values, controls):
        self.begin(path, "call", values, controls)
        name = bytes.fromhex(function.__name__[2:]).decode("utf-8")
        if function.__name__ in self.analysis.recursive:
            self.frames += 1
            self.ctx.coverage = {"examined": list(range(self.frames - 1)),
                                 "total": list(range(self.frames)), "complete": False}
            self.meter.charge("memory", 16, self.ctx.run["id"] + "/" + name + ":before_frame_" + str(self.frames))
        for parameter, value in zip(function.__code__.co_varnames, values):
            parameter = bytes.fromhex(parameter[2:]).decode("utf-8")
            self.meter.bind(value.data, path + "/parameter/" + parameter)
        old_call, old_loop, old_index = self.call_controls, self.loop_controls, self.loop_index
        self.call_controls, self.loop_controls, self.loop_index = self.controls(controls), [], None
        try:
            value = function(*values)
        finally:
            self.call_controls, self.loop_controls, self.loop_index = old_call, old_loop, old_index
        if value is None:
            raise ValueError("module did not return")
        return self.bind(path, self.make(path, kind, value.data, [value] + values, controls))

    def h_workspace_get(self, path, kind, key, controls):
        self.begin(path, "workspace_get", controls=controls)
        value = self.host.workspace_get(key)
        self.read_state |= value.read_state
        return self.bind(path, value)

    def h_workspace_put(self, path, key, value, controls):
        self.begin(path, "workspace_put", [value], controls)
        stored = self.make(path, value.type, value.data, [value], controls)
        self.meter.output(stored.data, path)
        self.host.workspace_put(key, stored)
        self.h_done(path)

    def h_preview(self, path, word, target, controls):
        self.begin(path, "preview", [word, target], controls)
        capture = self.ctx.run["profile"] != "solving"
        check = self.ctx.kernel.word_check(word.data)
        public = {"acknowledged": True} if self.ctx.form == "capability_separation" else {
            "code": None if capture or check["protected"] else "KN2:protection.piece_would_move",
            "protected": bool(check["protected"])}
        self.meter.bind(public, path)
        if capture:
            self.meter.output({"word": word.data, "target": target.data, "destination": "capture"}, path)
        info = self.analysis.marks[path][1]
        value = self.host.preview(word, target, path, labels=info.labels, read_state=info.read,
                                  control=self.controls(controls))
        self.h_done(path)
        return value

    def h_pick(self, path, kind, options, event, controls):
        self.begin(path, "pick", [options], controls)
        selected = options.data[self.ctx.program["inputs"][event]["value"]]
        self.meter.bind(selected, path)
        info = self.analysis.marks[path][1]
        if self.controls(controls):
            options = self.make(path, options.type, options.data, [options], controls,
                                entry=path + "/options", origin=options.block["origin"],
                                declassifiers=options.block["declassifiers"])
        # The host seals the fixed-entry test and the actual captured pick record.
        value = self.host.pick(options, event, kind, path, labels=info.labels, read_state=info.read)
        self.h_done(path)
        return value

    def h_emit(self, path, value, controls):
        self.begin(path, "emit", [value], controls)
        emitted = self.make(path, value.type, value.data, [value], controls)
        self.meter.output(emitted.data, path)
        self.host.event("emit", {"value": emitted.data}, emitted)
        self.h_done(path)

    def h_publish(self, path, exact, coordinates, controls):
        self.begin(path, "publish", [exact, coordinates], controls)
        value = self.make(path, "Record", exact.data, [exact, coordinates], controls)
        plan = {"exact": {**thaw(exact.data), "source_digest": digest(self.ctx.program),
                           "input_digest": digest(self.ctx.kernel.fixture["session"])},
                "coordinates": coordinates.data}
        self.meter.output(plan, path)
        self.ctx.display = self.host.publish(exact.data, coordinates.data, value)
        self.h_done(path)

    def h_return(self, path, value, controls):
        self.begin(path, "return", [value], controls)
        result = self.make(path, value.type, value.data, [value], controls)
        if not path.startswith("modules/"):
            self.meter.output(result.data, path)
        return result

    def h_retain(self, path, size, controls):
        self.begin(path, "retain", controls=controls)
        value = self.make(path, "Bytes", {"logical_bytes": size}, controls=controls)
        return self.bind(path, value, logical_bytes=size)

    def h_ambient(self, path, kind, resource, logical_resource, controls):
        self.begin(path, "ambient", controls=controls, resource=resource)
        # Under the frozen policy this line is unreachable. Traps are synthetic.
        data = self.host.trap(resource, logical_resource)
        return self.bind(path, self.make(path, kind, data, controls=controls))

    def h_stop(self, path, controls):
        self.begin(path, "stop", controls=controls)
        raise Stop("completed", None, "requested", path, self.loop_index)

    def h_tick(self, path, controls):
        self.begin(path, "tick", controls=controls)
        self.h_done(path)
