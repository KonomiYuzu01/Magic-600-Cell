"""Neutral control evaluator. Policy and approval enforcement are OFF.

The common resource meter remains active so the infinite recursion stimulus is
bounded. Observational dependency blocks are evidence, never flow decisions.
"""

from .calc import calculate
import json

from .common import ROOT, canonical, digest, thaw
from .meter import Stop
from .policy import STATE_ROWS


class Returned(Exception):
    def __init__(self, value):
        self.value = value


def execute(ctx):
    ctx.layer = "control_open"
    ctx.admit()
    # Execute the mechanically rendered, pinned bytes. The mutation probe changes
    # the loaded dependency after approval, keeping its original approval digest.
    rendered = json.loads((ROOT / ctx.bindings["rendered_file"]).read_bytes())[ctx.case["id"]] if "rendered_file" in ctx.bindings else ctx.program
    if ctx.case.get("kind") == "dependency_substitution":
        rendered["modules"]["constant"]["body"][0]["value"]["value"] = 2
    ctx.program = ctx.host.program = rendered
    ctx.run = next(run for run in rendered["runs"] if run["id"] == ctx.run["id"])
    host, meter = ctx.host, ctx.meter
    inputs = {name: host.input(name) for name in ctx.program["inputs"]}
    frames = 0
    constituents = 0

    def observe(kind, data, entry, dependencies=(), control=(), **extra):
        dependencies = list(dependencies) + list(control)
        labels = {label for value in dependencies for label in value.labels}
        labels.update(extra.pop("add_labels", ()))
        if ctx.run["profile"] == "research":
            labels.add("research")
        state = extra.pop("state", False)
        read = any(v.read_state for v in dependencies) or state
        return host.make(kind, data, entry, dependencies, labels=labels, read_state=read, **extra)

    def expression(expr, env, path):
        if "ref" in expr:
            return env[expr["ref"]]
        return host.make(expr["type"], expr["value"], path + "/literal", origin="fixture")

    def bind(env, name, value, path, **size):
        meter.bind(value.data, path, **size)
        env[name] = value

    def block(body, env, prefix, control=(), depth=0, top=False, loop_index=None):
        nonlocal frames, constituents
        for index, op in enumerate(body):
            path = prefix + "/" + str(index)
            ctx.position = path
            meter.continuation(path + ":before_step", loop_index=loop_index)
            ctx.executed(path, canonical(op))
            kind = op["op"]
            if kind in ("kernel", "calc", "call"):
                values = [expression(e, env, path + "/args/" + str(i)) for i, e in enumerate(op["args"])]
            if kind == "kernel":
                row = op["row"]
                data = ctx.kernel_call(row, values, path)
                extra = {"state": row in STATE_ROWS, "add_labels": ["state"] if row in STATE_ROWS else []}
                if row == "setup":
                    extra.update(origin="fixed_entry", declassifiers=[{
                        "id": "named_setup", "manual_target": thaw(values[0].data),
                        "entry_digest": digest(ctx.kernel.fixture["rows"]["setup"]),
                        "table_digest": digest(ctx.kernel.fixture["setups"]), "output": thaw(data)}])
                elif row == "caps":
                    extra["origin"] = "fixed_object_set"
                value = observe(op["type"], data, path, values, control, **extra)
                if row == "batch_check":
                    env[op["as"]] = value  # Buffer growth was charged chunk by chunk.
                else:
                    bind(env, op["as"], value, path, snapshot=row == "snapshot")
            elif kind == "calc":
                def charge(name, weight):
                    nonlocal constituents
                    meter.continuation(path + ":calc/" + str(constituents) + "/" + name, weight, loop_index)
                    constituents += 1
                data = calculate(op["fn"], [thaw(v.data) for v in values], charge, ctx.kernel.fixture["inverses"])
                computed = op["type"] in ("Word", "Words", "Target", "Targets") and op["fn"] not in ctx.policy.data["transparent"]
                value = observe(op["type"], data, path, values, control, add_labels=["computed"] if computed else [])
                bind(env, op["as"], value, path)
            elif kind == "if":
                test = expression(op["test"], env, path)
                branch = "then" if test.data else "else"
                block(op[branch], env, path + "/" + branch, tuple(control) + (test,), depth, loop_index=loop_index)
            elif kind == "for":
                items = expression(op["items"], env, path)
                track = top and len(body) == 1
                if track:
                    ctx.coverage = {"examined": [], "total": list(range(len(items.data))), "complete": False}
                for i in range(len(items.data) + 1):
                    meter.continuation(path + ":loop_continuation/" + str(i), loop_index=i)
                    if i == len(items.data):
                        break
                    local = dict(env)
                    item = observe(op["type"], items.data[i], path + "/item", [items], control)
                    bind(local, op["item"], item, path + "/item")
                    block(op["body"], local, path + "/body", control, depth, loop_index=i)
                    for name in env:
                        if name != op["item"]:
                            env[name] = local[name]
                    if track:
                        ctx.coverage["examined"].append(i)
            elif kind == "call":
                module = ctx.program["modules"][op["module"]]
                recursive = any(x.get("module") == op["module"] for x in module["body"])
                if recursive:
                    frames += 1
                    ctx.coverage = {"examined": list(range(frames - 1)), "total": list(range(frames)), "complete": False}
                    meter.charge("memory", 16, ctx.run["id"] + "/" + op["module"] + ":before_frame_" + str(frames))
                local = dict(inputs)
                for name, value in zip(sorted(module["parameters"]), values):
                    bind(local, name, value, path + "/parameter/" + name)
                try:
                    block(module["body"], local, "modules/" + op["module"] + "/body", control, depth + 1)
                except Returned as returned:
                    value = observe(op["type"], returned.value.data, path, [returned.value] + values, control)
                    bind(env, op["as"], value, path)
                else:
                    raise ValueError("module did not return")
            elif kind == "workspace_get":
                bind(env, op["as"], host.workspace_get(op["key"]), path)
            elif kind == "workspace_put":
                value = expression(op["value"], env, path)
                value = observe(value.type, value.data, path, [value], control)
                meter.output(value.data, path)
                host.workspace_put(op["key"], value)
            elif kind == "preview":
                word = expression(op["word"], env, path + "/word")
                target = expression(op["target"], env, path + "/target")
                dependencies = [word, target] + list(control)
                labels = {label for value in dependencies for label in value.labels}
                labels.add("state")
                if ctx.run["profile"] == "research":
                    labels.add("research")
                # Preflight all charges before a host side effect. Receipt bytes
                # depend on form; an acknowledgement never exposes refusal.
                check = ctx.kernel.word_check(word.data)
                capture = ctx.run["profile"] != "solving"
                public = {"acknowledged": True} if ctx.form == "capability_separation" else {
                    "code": None if capture or check["protected"] else "KN2:protection.piece_would_move",
                    "protected": bool(check["protected"])}
                meter.bind(public, path)
                if capture:
                    meter.output({"word": word.data, "target": target.data, "destination": "capture"}, path)
                value = host.preview(word, target, path, labels=labels, read_state=True, control=control)
                env[op["as"]] = value
            elif kind == "pick":
                options = expression(op["options"], env, path)
                chosen = options.data[ctx.program["inputs"][op["event"]]["value"]]
                meter.bind(chosen, path)
                env[op["as"]] = host.pick(options, op["event"], op["type"], path,
                                         labels=options.labels, read_state=options.read_state)
            elif kind == "emit":
                value = expression(op["value"], env, path)
                meter.output(value.data, path)
                host.event("emit", {"value": value.data}, value)
            elif kind == "publish":
                exact = expression(op["exact"], env, path + "/exact")
                coordinates = expression(op["coordinates"], env, path + "/coordinates")
                value = observe("Record", exact.data, path, [exact, coordinates], control)
                plan = {"exact": {**thaw(exact.data), "source_digest": digest(ctx.program),
                                   "input_digest": digest(ctx.kernel.fixture["session"])},
                        "coordinates": coordinates.data}
                meter.output(plan, path)
                ctx.display = host.publish(exact.data, coordinates.data, value)
            elif kind == "retain":
                value = observe("Bytes", {"logical_bytes": op["bytes"]}, path, control=control)
                bind(env, op["as"], value, path, logical_bytes=op["bytes"])
            elif kind == "ambient":
                data = host.trap(op["resource"], op["logical_resource"])
                bind(env, op["as"], observe(op["type"], data, path, control=control), path)
            elif kind == "dynamic":
                source = expression(op["source"], env, path)
                # The notation specifies an attempt, not executable Python text.
                ctx.dynamic_attempts.append({"path": path, "source_digest": digest(source.data)})
            elif kind == "return":
                value = expression(op["value"], env, path)
                value = observe(value.type, value.data, path, [value], control)
                if depth == 0:
                    meter.output(value.data, path)
                raise Returned(value)
            elif kind == "stop":
                raise Stop("completed", None, "requested", path, loop_index)
            elif kind != "tick":
                raise ValueError("unknown neutral instruction")
            if top and kind != "return" and not (len(body) == 1 and kind in ("for", "call")):
                if not (kind == "kernel" and op["row"] == "batch_check"):
                    ctx.coverage["examined"].append(index)

    try:
        block(ctx.run["body"], dict(inputs), ctx.run["id"] + "/body", top=True)
    except Returned as returned:
        return returned.value
    return None


def self_test():
    from .common import cases
    from .bindings import neutral_paths
    from .classify import classify
    from .host import Host
    from .runtime import RunContext
    from .policy import GRANTS
    for group, case in cases():
        if group == "approval" or case.get("kind") == "runtime_loss":
            continue
        mapping = {path: {"sha256": digest(op)} for path, op in neutral_paths(case["program"]).items()}
        bindings = {"effective_grants": {p: list(GRANTS) for p in ("solving", "research", "tool_authoring")}, "source_map": {case["id"]: mapping}}
        host = Host(case["program"], "static_runtime_labels", {})
        previous = None
        for i, run in enumerate(case["program"]["runs"]):
            hv = "cold" if case.get("kind") == "cache_changes" else "default"
            ctx = RunContext(case, group, run, host, bindings, "static_runtime_labels", "source_bound", previous, hv)
            try:
                value = execute(ctx)
                record = ctx.finish(value)
            except Stop as stop:
                record = ctx.finish(stop=stop)
            previous = record["sha256"]
            terminal = i == len(case["program"]["runs"]) - 1 or run["profile"] == "solving"
            result = classify(case, group, record, host_variant=hv, terminal=terminal)
            expected = {"allowed-tools": "allowed_ok", "planted": "planted_read", "budgets": "budget_ok",
                        "must-refuse": "unrefused" if terminal else "preparation_completed"}[group]
            if case["id"] == "B10":
                expected = "not_run"
            assert result["status"] == expected, (case["id"], result)
            if case["id"] == "B08":
                assert ctx.meter.used == dict(steps=3, kernel_cost=0, memory=1, output=0)
            if case["id"] == "B09":
                assert ctx.meter.used == dict(steps=1, kernel_cost=2, memory=117, output=0)
