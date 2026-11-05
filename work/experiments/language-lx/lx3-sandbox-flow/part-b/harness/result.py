"""DET.6 payload validation, exact hashing and RTE.46 display comparison."""

import math
from fractions import Fraction
from itertools import combinations

from .common import canonical, digest, load

PROFILE = load("profile.json")
EXACT_FIELDS = frozenset(PROFILE["exact_payload_fields"])
EXCLUDED_FIELDS = tuple(PROFILE["excluded_fields"])


class HarnessFailure(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise HarnessFailure(message)


def validate_payload(payload):
    require(set(payload) == EXACT_FIELDS, "missing or extra exact payload field")
    canonical(payload)
    require(payload["outcome"] in PROFILE["exact_payload_shape"]["outcome"], "invalid outcome")
    require(payload["outcome"] != "completed" or payload["code"] is None, "completed with error code")
    require(set(payload["coverage"]) == {"examined", "total", "complete"}, "missing coverage")
    require(type(payload["coverage"]["complete"]) is bool, "invalid coverage flag")
    require(set(payload["budgets"]) == {"units", "granted", "used", "children"}, "missing budgets")
    require(payload["budgets"]["units"] == PROFILE["units"], "wrong budget units")
    require(set(payload["budgets"]["used"]) == set(PROFILE["units"]), "missing used counter")
    require(all(type(n) is int and n >= 0 for n in payload["budgets"]["used"].values()), "invalid counter")
    require(set(payload["input_identities"]) == {"neutral_sha256", "binding_sha256", "run_key", "fixture", "model", "menu", "level", "contract"}, "missing input identity")
    identity = payload["input_identities"]
    require(identity["run_key"] == digest({"neutral_sha256": identity["neutral_sha256"],
                                          "binding_sha256": identity["binding_sha256"]}), "run key recipe mismatch")
    for name in ("neutral_sha256", "binding_sha256", "run_key", "fixture"):
        value = identity[name]
        require(type(value) is str and len(value) == 64 and all(c in "0123456789abcdef" for c in value), "invalid digest")
    for name in ("words", "certificates", "checker_reports", "traces"):
        require(type(payload[name]) is list, "invalid " + name)
    if payload["value"] is not None:
        require(set(payload["value"]) == {"type", "value"}, "untyped returned value")
    if payload["stop"] is not None:
        require(set(payload["stop"]) == {"reason", "run_id", "path", "loop_index"}, "missing stop position")
    if payload["outcome"] == "not_run":
        require(bool(payload["context"].get("reason")), "not_run without reason")
    return payload


def payload_hash(payload):
    validate_payload(payload)
    return digest(payload)


def number(value):
    if type(value) is int:
        return Fraction(value)
    if type(value) is float and math.isfinite(value):
        return Fraction(str(value))
    if type(value) is dict and set(value) == {"numerator", "denominator"}:
        canonical(value)
        return Fraction(value["numerator"], value["denominator"])
    raise HarnessFailure("invalid approximate coordinate")


def display_equal(left, right):
    if left is None or right is None:
        return left is right
    require(set(left) == set(right) == {"exact", "coordinates"}, "missing display field")
    require(set(left["exact"]) == set(right["exact"]) == set(PROFILE["display"]["exact_fields"]), "missing exact display field")
    if canonical(left["exact"]) != canonical(right["exact"]):
        return False
    a, b = left["coordinates"], right["coordinates"]
    if not isinstance(a, list) or not isinstance(b, list) or len(a) != len(b):
        return False
    tolerance = number(PROFILE["display"]["approximate_fields"]["coordinates"]["absolute_tolerance"])
    for xs, ys in zip(a, b):
        if not isinstance(xs, list) or not isinstance(ys, list) or len(xs) != len(ys):
            return False
        for x, y in zip(xs, ys):
            x, y = number(x), number(y)
            if abs(x) > 16 or abs(y) > 16 or abs(x - y) > tolerance:
                return False
    return True


def compare(records, reference_display=None, reproducible=True):
    """Compare only one full key; return concrete counterexample pairs."""
    failures = []
    for record in records:
        require(record["sha256"] == payload_hash(record["exact"]), "wrong payload hash")
        if not display_equal(record["display"], reference_display):
            failures.append({"kind": "display_reference", "run": record["repeat"]})
    keys = {r["exact"]["input_identities"]["run_key"] for r in records}
    require(len(keys) <= 1, "cannot compare different full keys")
    for a, b in combinations(records, 2):
        if reproducible and a["sha256"] != b["sha256"]:
            failures.append({"kind": "exact", "runs": [a["repeat"], b["repeat"]],
                             "hashes": [a["sha256"], b["sha256"]]})
        if not display_equal(a["display"], b["display"]):
            failures.append({"kind": "display_pair", "runs": [a["repeat"], b["repeat"]]})
    return failures


def self_test():
    exact = {"kind": "points", "object_ids": [], "order": [], "source_digest": "s",
             "input_digest": "i", "exact_properties": {"n": 9007199254740993}}
    a = {"exact": exact, "coordinates": [[0, {"numerator": 1, "denominator": 3}]]}
    b = {"exact": exact, "coordinates": [[0.000001, 0.333333333333]]}
    assert display_equal(a, b)
    b["coordinates"][0][0] = 0.0000011
    assert not display_equal(a, b)
    b["coordinates"][0][0] = float("inf")
    try:
        display_equal(a, b)
    except HarnessFailure:
        pass
    else:
        raise AssertionError("nonfinite coordinate accepted")
    assert "stop" in EXACT_FIELDS and "cache_diagnostics" in EXCLUDED_FIELDS
    reference = {"exact": exact, "coordinates": [[0, 0]]}
    low = {"exact": exact, "coordinates": [[-0.0000009, 0]]}
    high = {"exact": exact, "coordinates": [[0.0000009, 0]]}
    assert display_equal(low, reference) and display_equal(high, reference)
    assert not display_equal(low, high)  # Both near reference is insufficient.
    try:
        validate_payload({"outcome": "completed"})
    except HarnessFailure:
        pass
    else:
        raise AssertionError("omitted payload fields accepted")
    from copy import deepcopy
    from .host import Host
    from .runtime import RunContext
    case = load("allowed-tools.json")["cases"][0]
    bindings = {"effective_grants": {"solving": []}, "source_map": {}}
    host = Host(case["program"], "static_labels", {})
    context = RunContext(case, "allowed-tools", case["program"]["runs"][0], host, bindings, "static_labels", "source_bound")
    a = context.finish()
    b = deepcopy(a)
    b["repeat"] = 1
    b["exact"]["budgets"]["used"]["steps"] = 1
    b["sha256"] = payload_hash(b["exact"])
    assert compare([a, b]) == [{"kind": "exact", "runs": [0, 1], "hashes": [a["sha256"], b["sha256"]]}]
