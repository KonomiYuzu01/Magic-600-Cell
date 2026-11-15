"""Trusted Python glue. Node executes the checked JS branches/loops/modules.

Opaque integer handles keep exact values and provenance inside the host. Only
the shared policy decides permissions; this candidate supplies its knowledge.
"""

import atexit
import json
from pathlib import Path
import queue
import subprocess
import sys
import threading

sys.path.insert(0, str(Path(__file__).resolve().parent))

from checker import Rejected, STATE, analyze, recognize
from render import render
from harness.bindings import neutral_paths
from harness.calc import calculate
from harness.common import digest, thaw
from harness.meter import Stop

FOLDER = Path(__file__).resolve().parent
LOCK = threading.Lock()
CONNECTION = None


class Bridge:
    def __init__(self):
        self.process = subprocess.Popen(["node", str(FOLDER / "runtime.cjs")], cwd=FOLDER,
                                        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=sys.stderr)
        self.lines = queue.Queue()

        def read():
            for line in self.process.stdout:
                self.lines.put(line)
            self.lines.put(None)

        self.reader = threading.Thread(target=read, daemon=True)
        self.reader.start()
        atexit.register(self.close)

    def close(self):
        if self.process.stdin and not self.process.stdin.closed:
            try:
                self.process.stdin.close()
            except OSError:
                pass
        try:
            self.process.wait(timeout=1)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        self.reader.join(timeout=1)
        self.process.stdout.close()

    def send(self, message):
        self.process.stdin.write(json.dumps(message, ensure_ascii=True, separators=(",", ":")).encode() + b"\n")
        self.process.stdin.flush()

    def receive(self, path):
        try:
            line = self.lines.get(timeout=6)
        except queue.Empty:
            self.process.kill()
            self.process.wait()
            raise Stop("cut_off", "KN7:resource_limit.safety_stop", "safety_stop", path) from None
        if line is None:
            raise Stop("error", "KN7:resource_limit.runtime_lost", "runtime_loss", path)
        return json.loads(line)

    def run(self, request, executor):
        self.send(request)
        while True:
            response = self.receive(executor.ctx.position)
            if response["event"] == "call":
                try:
                    value = executor.handle(response["name"], response["args"])
                except Stop:
                    self.send({"stop": True})
                    if self.receive(executor.ctx.position)["event"] != "stopped":
                        raise ValueError("Node failed to acknowledge a host stop")
                    raise
                self.send({"value": value})
            elif response["event"] == "done":
                return None if response["value"] is None else executor.values[response["value"]]
            elif response["event"] == "timeout":
                raise Stop("cut_off", "KN7:resource_limit.safety_stop", "safety_stop", executor.ctx.position)
            else:
                raise Stop("error", "KN7:resource_limit.runtime_lost", "runtime_loss", executor.ctx.position)


class Executor:
    def __init__(self, ctx, source, program, mapping, plan):
        self.ctx, self.source, self.program = ctx, source, program
        self.mapping, self.plan = mapping, plan
        self.ops = neutral_paths(program)
        self.values, self.inputs = {}, {}
        self.constituents = self.frames = 0
        self.read_state = False
        for name in program["inputs"]:
            self.inputs[name] = self.keep(ctx.host.input(name))

    def keep(self, value):
        handle = len(self.values) + 1
        self.values[handle] = value
        return handle

    def expression(self, expr, env, path):
        if "ref" in expr:
            return self.values[env[expr["ref"]]]
        return self.ctx.host.make(expr["type"], expr["value"], path + "/literal", origin="fixture")

    def observe(self, kind, data, path, dependencies=(), control=(), **extra):
        knowledge = self.plan["knowledge"][path][1]
        read = knowledge.read or (self.ctx.form == "capability_separation" and self.read_state)
        return self.ctx.host.make(kind, data, path, list(dependencies) + list(control),
                                  labels=knowledge.labels, read_state=read, **extra)

    def bind(self, value, path, **size):
        self.ctx.meter.bind(value.data, path, **size)
        return self.keep(value)

    def handle(self, name, args):
        ctx, meter, host = self.ctx, self.ctx.meter, self.ctx.host
        path = args[0]
        op = self.ops[path]
        kind = op["op"]
        if name == "complete":
            depth = args[1]
            if depth == 0 and path.count("/") == 2:
                body = ctx.run["body"]
                if not (len(body) == 1 and kind in ("for", "call")) and not (kind == "kernel" and op["row"] == "batch_check"):
                    ctx.coverage["examined"].append(int(path.rsplit("/", 1)[1]))
            return None
        if name == "covered":
            if len(ctx.run["body"]) == 1 and path == ctx.run["id"] + "/body/0":
                ctx.coverage["examined"].append(args[1])
            return None
        if name == "loop":
            _, handle, index = args
            meter.continuation(path + ":loop_continuation/" + str(index), loop_index=index)
            return index < len(self.values[handle].data)
        if name == "item":
            _, handle, controls, index = args
            seq, control = self.values[handle], [self.values[h] for h in controls]
            knowledge = self.plan["knowledge"][path][1]
            value = host.make(op["type"], seq.data[index], path + "/item", [seq] + control,
                              labels=knowledge.labels, read_state=knowledge.read)
            return self.bind(value, path + "/item")
        if name == "leave":
            _, handle, env, controls = args
            value = self.values[handle]
            values = [self.expression(e, env, path + "/args/" + str(i)) for i, e in enumerate(op["args"])]
            return self.bind(self.observe(op["type"], value.data, path, [value] + values,
                                          [self.values[h] for h in controls]), path)

        _, env, controls, depth, index = args
        control = [self.values[h] for h in controls]
        if name != {"if": "branch", "for": "sequence", "call": "enter", "return": "finish"}.get(kind, kind):
            raise ValueError("bridge operation does not match approved instruction")
        ctx.position = path
        meter.continuation(path + ":before_step", loop_index=index)
        span = self.mapping[path]
        ctx.executed(path, self.source[span["start"]:span["end"]])
        values = [self.expression(e, env, path + "/args/" + str(i)) for i, e in enumerate(op.get("args", []))]
        fields = {key: self.expression(op[key], env, path + ("/" + key if key in ("word", "target", "exact", "coordinates") else ""))
                  for key in ("test", "items", "value", "word", "target", "options", "exact", "coordinates", "source") if key in op}
        used = self.plan["knowledge"][path][0]
        facts = {}
        if kind == "ambient":
            facts["resource"] = op["resource"]
        if kind == "kernel" and op["row"] == "setup":
            facts.update(manual_target=values[0].block["origin"] == "manual", fixed_entry=True)
        ctx.check(op["row"] if kind == "kernel" else kind,
                  values + list(fields.values()) + control, labels=used.labels,
                  read_state=used.read or (ctx.form == "capability_separation" and self.read_state), **facts)

        if kind == "kernel":
            row = op["row"]
            data = ctx.kernel_call(row, values, path)
            self.read_state |= row in STATE
            extra = {}
            if row == "setup":
                extra = {"origin": "fixed_entry", "declassifiers": [{"id": "named_setup",
                         "manual_target": thaw(values[0].data), "entry_digest": digest(ctx.kernel.fixture["rows"]["setup"]),
                         "table_digest": digest(ctx.kernel.fixture["setups"]), "output": thaw(data)}]}
            elif row == "caps":
                extra = {"origin": "fixed_object_set"}
            value = self.observe(op["type"], data, path, values, control, **extra)
            return self.keep(value) if row == "batch_check" else self.bind(value, path, snapshot=row == "snapshot")
        if kind == "calc":
            def charge(operation, weight):
                meter.continuation(path + ":calc/" + str(self.constituents) + "/" + operation, weight, index)
                self.constituents += 1
            data = calculate(op["fn"], [thaw(v.data) for v in values], charge, ctx.kernel.fixture["inverses"])
            return self.bind(self.observe(op["type"], data, path, values, control), path)
        if kind == "if":
            test = fields["test"]
            if type(test.data) is not bool:
                raise ValueError("non-Boolean branch")
            return {"id": self.keep(test), "truth": test.data}
        if kind == "for":
            items = fields["items"]
            if depth == 0 and len(ctx.run["body"]) == 1:
                ctx.coverage = {"examined": [], "total": list(range(len(items.data))), "complete": False}
            return self.keep(items)
        if kind == "call":
            module = self.program["modules"][op["module"]]
            if any(x.get("module") == op["module"] for x in module["body"]):
                self.frames += 1
                ctx.coverage = {"examined": list(range(self.frames - 1)), "total": list(range(self.frames)), "complete": False}
                meter.charge("memory", 16, ctx.run["id"] + "/" + op["module"] + ":before_frame_" + str(self.frames))
            local = dict(self.inputs)
            for parameter, value in zip(sorted(module["parameters"]), values):
                local[parameter] = self.bind(value, path + "/parameter/" + parameter)
            return local
        if kind == "workspace_get":
            value = host.workspace_get(op["key"])
            self.read_state |= value.read_state
            return self.bind(value, path)
        if kind == "workspace_put":
            value = fields["value"]
            value = self.observe(value.type, value.data, path, [value], control)
            meter.output(value.data, path)
            host.workspace_put(op["key"], value)
        elif kind == "preview":
            word, target = fields["word"], fields["target"]
            check = ctx.kernel.word_check(word.data)
            capture = ctx.run["profile"] != "solving"
            public = {"acknowledged": True} if ctx.form == "capability_separation" else {
                "code": None if capture or check["protected"] else "KN2:protection.piece_would_move",
                "protected": bool(check["protected"])}
            meter.bind(public, path)
            if capture:
                meter.output({"word": word.data, "target": target.data, "destination": "capture"}, path)
            knowledge = self.plan["knowledge"][path][1]
            return self.keep(host.preview(word, target, path, labels=knowledge.labels,
                                          read_state=knowledge.read, control=control))
        elif kind == "pick":
            options = fields["options"]
            selected = options.data[self.program["inputs"][op["event"]]["value"]]
            meter.bind(selected, path)
            knowledge = self.plan["knowledge"][path][1]
            return self.keep(host.pick(options, op["event"], op["type"], path,
                                       labels=knowledge.labels, read_state=knowledge.read))
        elif kind == "emit":
            value = fields["value"]
            value = self.observe(value.type, value.data, path, [value], control)
            meter.output(value.data, path)
            host.event("emit", {"value": value.data}, value)
        elif kind == "publish":
            exact, coordinates = fields["exact"], fields["coordinates"]
            value = self.observe("Record", exact.data, path, [exact, coordinates], control)
            plan = {"exact": {**thaw(exact.data), "source_digest": digest(ctx.program),
                               "input_digest": digest(ctx.kernel.fixture["session"])}, "coordinates": coordinates.data}
            meter.output(plan, path)
            ctx.display = host.publish(exact.data, coordinates.data, value)
        elif kind == "retain":
            value = self.observe("Bytes", {"logical_bytes": op["bytes"]}, path, control=control)
            return self.bind(value, path, logical_bytes=op["bytes"])
        elif kind == "return":
            value = fields["value"]
            value = self.observe(value.type, value.data, path, [value], control)
            if depth == 0:
                meter.output(value.data, path)
            return self.keep(value)
        elif kind == "stop":
            raise Stop("completed", None, "requested", path, index)
        elif kind not in ("tick",):
            raise ValueError("disallowed effect passed policy")
        return None


def execute(ctx):
    global CONNECTION
    ctx.layer = "rt2_node"
    index = json.loads((FOLDER / "program-index.json").read_bytes())[ctx.case["id"]]
    pinned = (FOLDER / "programs.js").read_bytes()[index["start"]:index["end"]]
    # The current host program includes any dependency substitution. Never load
    # the originally approved program in its place or silently renew approval.
    actual, _ = render(ctx.program)
    if actual != pinned:
        ctx.refuse("LG5:input.module_digest_mismatch", layer="rt2_byte_approval")
    try:
        program, mapping = recognize(actual)
    except Rejected:
        ctx.refuse("LG3:capability.not_granted", layer="rt2_subset_checker")
    plan = analyze(program, ctx.policy.data["transparent"])[ctx.run["id"]]
    grants = list(ctx.grants)
    if ctx.form == "capability_separation" and ctx.run["profile"] == "solving" and not plan["previews"]:
        grants.remove("preview")
    ctx.bind_grants(grants)
    ctx.admit()
    for path, op in neutral_paths(program).items():
        if path in plan["knowledge"] and op["op"] in ("dynamic", "ambient"):
            # These are attempted authorities, not functions granted to the VM.
            # The checker identifies them; the shared policy supplies the code.
            ctx.position = path
            facts = {"resource": op["resource"]} if op["op"] == "ambient" else {}
            ctx.check(op["op"], labels=(), read_state=False, **facts)
    executor = Executor(ctx, actual, program, mapping, plan)
    functions = {"if": "branch", "for": "sequence", "call": "enter", "return": "finish"}
    names = {functions.get(name, name) for name in plan["operations"]} | {"complete", "loop", "item", "covered", "leave"}
    request = {"source": actual.decode("utf-8"), "run": ctx.run["id"], "inputs": executor.inputs,
               "functions": sorted(names), "items": {p: op["item"] for p, op in executor.ops.items() if op["op"] == "for"}}
    # B12's overlapping Python invocations share immutable host cache values;
    # one Node child serves this worker, serializing its two independent VMs.
    with LOCK:
        if CONNECTION is None:
            CONNECTION = Bridge()
        return CONNECTION.run(request, executor)
