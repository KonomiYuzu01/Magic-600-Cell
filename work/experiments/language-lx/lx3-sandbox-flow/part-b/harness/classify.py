"""Classify observations against frozen expectations. Missing data fails closed."""

from .common import canonical, load
from .result import HarnessFailure, display_equal, payload_hash, require

SESSION_FIELDS = frozenset(load("fixture.json")["session"])


def validate_record(record):
    require(set(record) == {"exact", "sha256", "display", "observer", "host", "repeat"}, "missing result envelope field")
    require(record["sha256"] == payload_hash(record["exact"]), "payload hash mismatch")
    observer = record["observer"]
    require(set(observer) == {"before", "after", "shell", "trap_calls", "denials", "approval", "consumed", "provenance_blocks", "runtime_loss_observed", "kernel_calls"}, "missing observer field")
    for state in (observer["before"], observer["after"]):
        require(set(state) == {"session", "pending_preview", "workspace"}, "missing session observation")
        require(set(state["session"]) == SESSION_FIELDS, "missing protected session field")
    for name in ("shell", "trap_calls", "denials", "consumed", "kernel_calls"):
        require(type(observer[name]) is list, "invalid observer list")
    require(set(observer["approval"]) == {"executions", "dynamic_attempts"}, "missing approval evidence")
    for execution in observer["approval"]["executions"]:
        require(set(execution) == {"path", "approved_sha256", "executed_sha256", "inside_approval"}, "missing executed-byte observation")
        require(execution["inside_approval"] == (execution["approved_sha256"] == execution["executed_sha256"]), "inconsistent approval observation")
    for denial in observer["denials"]:
        require(set(denial) == {"operation", "code", "layer", "path", "before", "after"}, "missing refusal checkpoint")
    require("cache_diagnostics" in record["host"], "missing cache diagnostics")
    require(set(record["host"]["diagnostics"]) == {"sha256", "planted_data"}, "missing diagnostic observation")
    return record


def observations(record):
    traces = record["exact"]["traces"]
    emitted, previews, writes = [], [], []
    for event in traces:
        require("kind" in event and "provenance" in event, "unattributed host event")
        if event["kind"] == "emit":
            emitted.append(event["value"])
        elif event["kind"] == "preview":
            previews.append({k: event[k] for k in ("word", "target", "destination")})
        elif event["kind"] == "workspace_put":
            writes.append({k: event[k] for k in ("key", "value", "profile")})
        elif event["kind"] not in ("publish", "pick"):
            raise HarnessFailure("unknown host event")
    return emitted, previews, writes


def leaked(record):
    markers = [c["plant"]["data"] for c in load("planted.json")["cases"]]
    # Include all sinks and the excluded diagnostic envelope. Approximate display
    # floats are allowed there, so use a representation rather than exact JSON.
    text = repr(record)
    return (bool(record["observer"]["trap_calls"]) or record["host"]["diagnostics"]["planted_data"]
            or any(marker in text for marker in markers))


def state_changed(record, case):
    observer = record["observer"]
    before, after = observer["before"], observer["after"]
    if canonical(before["session"]) != canonical(after["session"]):
        return True
    for field in ("pending_preview", "workspace"):
        if field in case.get("unchanged", []) and canonical(before[field]) != canonical(after[field]):
            return True
    for denial in observer["denials"]:
        if canonical(denial["before"]) != canonical(denial["after"]):
            return True
        if canonical(denial["after"]) != canonical(after):
            return True
        # A refusal after the very effect it purports to gate is not a refusal.
        if denial["operation"] == "preview" and any(
                event["provenance"]["entry"] == denial["path"] and event["outcome"]["code"] is None
                for event in observer["shell"]):
            return True
    if "family" in case and record["exact"]["outcome"] == "refused":
        installed = [event for event in observer["shell"] if event["destination"] == "solving_preview" and event["outcome"]["code"] is None]
        if installed:
            if case["family"] not in ("stop_selected_preview", "accumulation_across_runs"):
                return True
            from .policy import STATE_ROWS
            if any(call["row"] in STATE_ROWS for call in observer["kernel_calls"]):
                return True
    return False


def stop_matches(case, actual):
    expected = case["expected"]["stop"]
    if expected is None:
        return actual is None
    if actual is None:
        return False
    if expected == "before admission":
        return actual["path"] == "r1/admission"
    event = case["host"].get("event")
    if event == "cancel":
        return actual["path"] == case["host"]["at"]
    if event == "runtime_loss":
        return actual["reason"] == "runtime_loss"
    if event == "deadline":
        return actual["reason"] == "safety_stop"
    return actual["path"] == expected


def classify(case, group, record=None, *, failure=None, host_variant="default", terminal=True):
    """One observed neutral key. terminal=False retains multi-run preparation."""
    if failure is not None:
        require(failure in ("crash", "hang"), "unknown process failure")
        return {"status": failure, "code": None}
    validate_record(record)
    exact = record["exact"]
    outcome, code = exact["outcome"], exact["code"]
    if outcome == "not_run":
        return {"status": "not_run", "code": code, "reason": exact["context"]["reason"]}
    if leaked(record):
        return {"status": "planted_read" if group in ("must-refuse", "planted") else "wrong_value", "code": code}
    if state_changed(record, case):
        return {"status": "state_changed" if group in ("must-refuse", "planted", "approval") else "wrong_value", "code": code}
    if group in ("must-refuse", "planted", "approval"):
        if group == "approval":
            approval = record["observer"]["approval"]
            if approval["dynamic_attempts"] or any(not e["inside_approval"] for e in approval["executions"]):
                return {"status": "unrefused", "code": code, "reason": "execution_outside_approval"}
        if outcome == "refused":
            require(bool(record["observer"]["denials"]), "refusal without observed gate")
            require(record["observer"]["denials"][-1]["code"] == code, "refusal code differs from gate evidence")
            return {"status": "refused_expected" if code == case["expected_code"] else "wrong_code", "code": code}
        if not terminal and outcome == "completed":
            return {"status": "preparation_completed", "code": code}
        return {"status": "unrefused", "code": code}
    expected = case["expected"]
    value = None if exact["value"] is None else exact["value"]["value"]
    if group == "allowed-tools":
        if outcome == "refused":
            return {"status": "false_refusal", "code": code}
        emitted, previews, writes = observations(record)
        okay = outcome == expected["outcome"] and code == expected["code"]
        if exact["value"] is not None:
            from .bindings import neutral_paths
            types = {name: item["type"] for name, item in case["program"]["inputs"].items()}
            types.update({op["as"]: op["type"] for op in neutral_paths(case["program"]).values() if "as" in op})
            returned = case["program"]["runs"][-1]["body"][-1]["value"]
            kind = types[returned["ref"]] if "ref" in returned else returned["type"]
            okay = okay and exact["value"]["type"] == kind
        for actual, frozen in ((value, expected["value"]), (emitted, expected["emitted"]),
                               (previews, expected["previews"]), (writes, expected["workspace_writes"])):
            okay = okay and canonical(actual) == canonical(frozen)
        okay = okay and display_equal(record["display"], expected["display"])
        return {"status": "allowed_ok" if okay else "wrong_value", "code": code}
    require(group == "budgets", "unknown corpus group")
    okay = outcome == expected["outcome"] and code == expected["code"] and canonical(value) == canonical(expected["value"])
    if expected["used"] is not None:
        okay = okay and exact["budgets"]["used"] == expected["used"]
    coverage = exact["coverage"]
    okay = okay and coverage["total"] == expected["coverage"]["total"] and coverage["complete"] == expected["coverage"]["complete"]
    if expected["coverage"]["examined"] is not None:
        okay = okay and coverage["examined"] == expected["coverage"]["examined"]
    okay = okay and stop_matches(case, exact["stop"])
    if case["kind"] == "runtime_loss":
        okay = okay and record["observer"]["runtime_loss_observed"]
    if case["kind"] == "cache_changes":
        variant = next(v for v in case["host"]["variants"] if v["id"] == host_variant)
        diagnostics = record["host"]["cache_diagnostics"]
        okay = okay and diagnostics == ([] if variant["expected_host_code"] is None else [variant["expected_host_code"]])
    return {"status": "budget_ok" if okay else "wrong_value", "code": code,
            "used": exact["budgets"]["used"], "stop": exact["stop"], "coverage": coverage}


def self_test():
    from .common import digest
    from .host import Host
    from .runtime import RunContext
    from .meter import Stop
    case = load("must-refuse.json")["cases"][0]
    bindings = {"effective_grants": {"solving": []}, "source_map": {}}
    host = Host(case["program"], "static_labels", {})
    ctx = RunContext(case, "must-refuse", case["program"]["runs"][0], host, bindings, "static_labels", "source_bound")
    record = ctx.finish()
    assert classify(case, "must-refuse", record)["status"] == "unrefused"
    try:
        ctx.refuse(case["expected_code"])
    except Stop as stop:
        refused = ctx.finish(stop=stop)
    assert classify(case, "must-refuse", refused)["status"] == "refused_expected"
    refused["exact"]["code"] = "KN7:budget.steps"
    refused["observer"]["denials"][-1]["code"] = "KN7:budget.steps"
    refused["sha256"] = digest(refused["exact"])
    assert classify(case, "must-refuse", refused)["status"] == "wrong_code"
    for key in list(record):
        broken = dict(record)
        del broken[key]
        try:
            classify(case, "must-refuse", broken)
        except (HarnessFailure, KeyError):
            pass
        else:
            raise AssertionError("omitted field accepted: " + key)
    record["observer"]["trap_calls"].append({"resource": "clock"})
    assert classify(case, "must-refuse", record)["status"] == "planted_read"
    assert classify(case, "must-refuse", failure="hang")["status"] == "hang"
    # A correct code supplied after installing the selected proposal is a fail.
    host = Host(case["program"], "static_labels", {})
    ctx = RunContext(case, "must-refuse", case["program"]["runs"][0], host, bindings, "static_labels", "source_bound")
    host.preview(host.input("word"), host.input("target"), "r1/body/3", labels=["state"], read_state=True)
    try:
        ctx.refuse(case["expected_code"], operation="later_gate")
    except Stop as stop:
        late = ctx.finish(stop=stop)
    assert classify(case, "must-refuse", late)["status"] == "state_changed"
    # Excluded transport diagnostics remain in the planted-data observation.
    record["observer"]["trap_calls"].clear()
    record["host"]["diagnostics"]["planted_data"] = True
    assert classify(case, "must-refuse", record)["status"] == "planted_read"
