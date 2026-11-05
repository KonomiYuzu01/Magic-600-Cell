"""Trusted host glue for compiled RT3. Control and flow joins execute in Wasm.

Only stimulus/host interfaces and pinned candidate files are read. Policy,
oracle, and meter remain the shared bench's; no expected outcome is consulted.
"""

import json
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
FOLDER = Path(__file__).resolve().parent
if str(FOLDER) not in sys.path:
    sys.path.insert(0, str(FOLDER))

from harness.calc import calculate
from harness.common import digest, thaw
from harness.meter import Stop
from frontend import IMPORTS, READ, bits, labels, compile_program
from render import encode, instructions, render


class Bridge:
    def __init__(self, ctx, surface, compiled):
        self.ctx, self.surface, self.compiled = ctx, surface, compiled
        self.host, self.meter = ctx.host, ctx.meter
        self.nodes = [node for _, node in sorted(instructions(surface).items())]
        self.input_names = sorted(surface["inputs"])
        self.values = [None]
        self.controls = [()]
        self.arguments = []
        self.inputs = {}
        self.frames = self.constituents = 0
        self.process = None
        self.pending_charge = None

    def handle(self, value):
        self.values.append(value)
        return len(self.values) - 1

    def controls_at(self, handle):
        return self.controls[handle]

    def make(self, kind, data, path, values, mask, control=0, **extra):
        return self.host.make(kind, data, path, list(values) + list(self.controls_at(control)),
                              labels=labels(mask), read_state=bool(mask & READ), **extra)

    def check(self, operation, values, mask, control=0, **facts):
        self.ctx.check(operation, list(values) + list(self.controls_at(control)),
                       labels=labels(mask), read_state=bool(mask & READ), **facts)

    def take_args(self):
        values, self.arguments = self.arguments, []
        return values

    def input(self, index):
        name = self.input_names[index]
        if name not in self.inputs:
            self.inputs[name] = self.handle(self.host.input(name))
        return self.inputs[name]

    def literal(self, index):
        kind, data, path = self.compiled["literals"][index]
        return self.handle(self.host.make(kind, data, path, origin="fixture"))

    def meta(self, handle):
        return bits(self.values[handle])

    def arg(self, handle):
        self.arguments.append(self.values[handle])

    def step(self, opid, loop):
        node = self.nodes[opid]
        self.ctx.position = node[1]
        self.meter.continuation(node[1] + ":before_step", loop_index=None if loop < 0 else loop)
        self.ctx.executed(node[1], encode(node))
        return self.meter.used["steps"]

    def truth(self, handle):
        value = self.values[handle]
        if value.type != "Bool" or type(value.data) is not bool:
            raise ValueError("RT3 condition must be Bool")
        return int(value.data)

    def length(self, handle):
        return len(self.values[handle].data)

    def control(self, current, condition):
        self.controls.append(self.controls_at(current) + (self.values[condition],))
        return len(self.controls) - 1

    def tracked_loop(self, node):
        return len(self.ctx.run["body"]) == 1 and node[1] == self.ctx.run["id"] + "/body/0"

    def loop_begin(self, opid, handle):
        node = self.nodes[opid]
        if self.tracked_loop(node):
            self.ctx.coverage = {"examined": [], "total": list(range(self.length(handle))), "complete": False}

    def loop_test(self, opid, index):
        path = self.nodes[opid][1] + ":loop_continuation/" + str(index)
        self.meter.continuation(path, loop_index=index)
        return self.meter.used["steps"]

    def item(self, opid, source, index, mask, control):
        node, source = self.nodes[opid], self.values[source]
        value = self.make(node[3], source.data[index], node[1] + "/item", [source], mask, control)
        self.meter.bind(value.data, node[1] + "/item")
        return self.handle(value)

    def loop_done(self, opid, index):
        if self.tracked_loop(self.nodes[opid]):
            self.ctx.coverage["examined"].append(index)

    def done(self, opid):
        node = self.nodes[opid]
        prefix = self.ctx.run["id"] + "/body/"
        if not node[1].startswith(prefix) or "/" in node[1][len(prefix):]:
            return
        if node[0] == "return" or (len(self.ctx.run["body"]) == 1 and node[0] in ("call", "for")):
            return
        if node[0] != "kernel" or node[4] != "batch_check":
            self.ctx.coverage["examined"].append(int(node[1][len(prefix):]))

    def load(self, opid):
        return self.handle(self.host.workspace_get(self.nodes[opid][4]))

    def call_enter(self, opid, mask, control):
        node, values = self.nodes[opid], self.take_args()
        self.check("call", values, mask, control)
        module = self.surface["modules"][node[4]]
        if any(child[0] == "call" and child[4] == node[4] for child in module[2]):
            self.frames += 1
            self.ctx.coverage = {"examined": list(range(self.frames - 1)),
                                 "total": list(range(self.frames)), "complete": False}
            self.meter.charge("memory", 16, self.ctx.run["id"] + "/" + node[4] + ":before_frame_" + str(self.frames))
        for name, value in zip(sorted(module[0]), values):
            self.meter.bind(value.data, node[1] + "/parameter/" + name)

    def call_exit(self, opid, mask, control, result):
        node, values = self.nodes[opid], self.take_args()
        value = self.make(node[3], self.values[result].data, node[1], values, mask, control)
        self.meter.bind(value.data, node[1])
        return self.handle(value)

    def send(self, value):
        self.process.stdin.write(encode(value) + b"\n")
        self.process.stdin.flush()

    def receive(self):
        line = self.process.stdout.readline()
        if not line:
            raise RuntimeError("Wasm bridge closed")
        return json.loads(line)

    def charge(self, path, name, weight, loop):
        # Node is waiting inside perform(). Reenter the compiler-emitted fuel
        # trampoline, which calls meter(token), before arithmetic takes place.
        token = self.constituents
        self.constituents += 1
        self.pending_charge = (token, path + ":calc/" + str(token) + "/" + name, weight, loop)
        self.send({"charge": token})
        request = self.receive()
        if request != {"method": "meter", "args": [token]}:
            raise ValueError("invalid fuel continuation")
        self.send({"value": self.meter_token(token)})
        if self.receive() != {"charged": token}:
            raise ValueError("unacknowledged fuel continuation")
        self.pending_charge = None

    def meter_token(self, token):
        current, path, weight, loop = self.pending_charge
        if token != current:
            raise ValueError("invalid fuel token")
        self.meter.continuation(path, weight, None if loop < 0 else loop)
        return self.meter.used["steps"]

    def perform(self, opid, incoming, outgoing, control, loop):
        node, values = self.nodes[opid], self.take_args()
        kind, path = node[:2]
        self.ctx.position = path
        operation = node[4] if kind == "kernel" else kind
        if kind == "return" and path.startswith("modules/"):
            operation = "call"  # An internal return is not a display sink.
        facts = {}
        if operation == "setup":
            facts = {"manual_target": values[0].block["origin"] == "manual",
                     "fixed_entry": True}
        if kind == "ambient":
            facts = {"resource": node[4]}
        self.check(operation, values, incoming, control, **facts)
        value = None
        if kind == "kernel":
            row = node[4]
            data = self.ctx.kernel_call(row, values, path)
            extra = {}
            if row == "setup":
                extra = {"origin": "fixed_entry", "declassifiers": [{
                    "id": "named_setup", "manual_target": thaw(values[0].data),
                    "entry_digest": digest(self.ctx.kernel.fixture["rows"]["setup"]),
                    "table_digest": digest(self.ctx.kernel.fixture["setups"]), "output": thaw(data)}]}
            elif row == "caps":
                extra = {"origin": "fixed_object_set"}
            value = self.make(node[3], data, path, values, outgoing, control, **extra)
            if row != "batch_check":
                self.meter.bind(data, path, snapshot=row == "snapshot")
        elif kind == "calc":
            data = calculate(node[4], [thaw(v.data) for v in values],
                             lambda name, weight: self.charge(path, name, weight, loop),
                             self.ctx.kernel.fixture["inverses"])
            value = self.make(node[3], data, path, values, outgoing, control)
            self.meter.bind(data, path)
        elif kind == "workspace_get":
            value = self.make(node[3], values[0].data, path, values, outgoing, control)
            self.meter.bind(value.data, path)
        elif kind == "workspace_put":
            value = self.make(values[0].type, values[0].data, path, values, outgoing, control)
            self.meter.output(value.data, path)
            self.host.workspace_put(node[2], value)
        elif kind == "preview":
            word, target = values
            check = self.ctx.kernel.word_check(word.data)
            capture = self.ctx.run["profile"] != "solving"
            public = {"acknowledged": True} if self.ctx.form == "capability_separation" else {
                "code": None if capture or check["protected"] else "KN2:protection.piece_would_move",
                "protected": bool(check["protected"])}
            self.meter.bind(public, path)
            if capture:
                self.meter.output({"word": word.data, "target": target.data, "destination": "capture"}, path)
            value = self.host.preview(word, target, path, labels=labels(outgoing), read_state=bool(outgoing & READ),
                                      control=self.controls_at(control))
        elif kind == "pick":
            options = values[0]
            self.meter.bind(options.data[self.ctx.program["inputs"][node[5]]["value"]], path)
            # Include control in the option block before handing it to the host;
            # a computed singleton never acquires a fixed origin here.
            if self.controls_at(control):
                options = self.make(options.type, options.data, path + "/options", values,
                                    outgoing, control, origin=options.block["origin"])
            value = self.host.pick(options, node[5], node[3], path,
                                   labels=labels(outgoing), read_state=bool(outgoing & READ))
        elif kind == "emit":
            value = self.make(values[0].type, values[0].data, path, values, outgoing, control)
            self.meter.output(value.data, path)
            self.host.event("emit", {"value": value.data}, value)
        elif kind == "publish":
            exact, coordinates = values
            value = self.make("Record", exact.data, path, values, outgoing, control)
            plan = {"exact": {**thaw(exact.data), "source_digest": digest(self.ctx.program),
                              "input_digest": digest(self.ctx.kernel.fixture["session"])},
                    "coordinates": coordinates.data}
            self.meter.output(plan, path)
            self.ctx.display = self.host.publish(exact.data, coordinates.data, value)
        elif kind == "retain":
            value = self.make("Bytes", {"logical_bytes": node[4]}, path, [], outgoing, control)
            self.meter.bind(value.data, path, logical_bytes=node[4])
        elif kind == "ambient":
            data = self.host.trap(node[4], node[5])
            value = self.make(node[3], data, path, [], outgoing, control)
            self.meter.bind(data, path)
        elif kind == "return":
            value = self.make(values[0].type, values[0].data, path, values, outgoing, control)
            if not path.startswith("modules/"):
                self.meter.output(value.data, path)
        elif kind == "stop":
            raise Stop("completed", None, "requested", path, None if loop < 0 else loop)
        elif kind != "tick":
            # Dynamic evaluation is refused by the shared policy before here.
            raise ValueError("unsupported compiled intrinsic")
        return self.handle(value) if value is not None else 0

    def run(self):
        command = ["node", str(FOLDER / "bridge.js")]
        self.process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=subprocess.DEVNULL, cwd=FOLDER)
        try:
            self.send({"wasm_hex": self.compiled["wasm_hex"]})
            while True:
                request = self.receive()
                if request.get("finished"):
                    if int(request["fuel"]) != self.meter.used["steps"]:
                        raise ValueError("compiled fuel diverged from shared meter")
                    self.process.wait(timeout=2)
                    if self.process.returncode:
                        raise RuntimeError("Wasm bridge failed")
                    return self.values[request["handle"]]
                if request.get("failed"):
                    raise RuntimeError("Wasm " + request["kind"] + " failure")
                method = request["method"]
                if method not in {entry[0] for entry in IMPORTS} or method == "meter":
                    raise ValueError("unexpected host import")
                result = getattr(self, method)(*request["args"])
                self.send({"value": result})
        finally:
            if self.process.poll() is None:
                self.process.kill()
            self.process.wait()
            self.process.stdin.close()
            self.process.stdout.close()


def run_compiled(ctx, surface, compiled):
    ctx.layer = "rt3_wasm"
    grants = list(ctx.bindings["effective_grants"][ctx.run["profile"]])
    if ctx.form == "capability_separation" and ctx.run["profile"] == "solving" and compiled["state_read_effect"]:
        grants.remove("preview")
    ctx.bind_grants(grants)
    ctx.admit()  # B10 and actual child loss occur before a Node child is started.
    return Bridge(ctx, surface, compiled).run()


def execute(ctx):
    # Approval is over the original source, while ctx.program contains the host's
    # dependency-substitution stimulus. Never silently execute the original.
    key = digest(ctx.case["program"])
    surface = render(ctx.program)
    approved = json.loads((FOLDER / "programs.rt3").read_bytes())[key]
    if encode(surface) != encode(approved):
        ctx.refuse("LG5:input.module_digest_mismatch", layer="rt3_source_binding")
    compiled = compile_program(surface, ctx.run["id"], ctx.form, ctx.policy.data["transparent"])
    pinned = json.loads((FOLDER / "modules.json").read_bytes())[key][ctx.form][ctx.run["id"]]
    if compiled != pinned:
        ctx.refuse("LG5:input.module_digest_mismatch", layer="rt3_lowering_binding")
    return run_compiled(ctx, surface, compiled)
