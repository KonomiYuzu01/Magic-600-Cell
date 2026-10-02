"""Compare a draft oracle case with a candidate trace (exit codes 0, 1, 2)."""
from __future__ import annotations

import argparse
import gzip
import json
import re
import sys
import zlib
from pathlib import Path

sys.dont_write_bytecode = True

CASE_FORMAT = "C600-ORACLE-CASE-draft"
TRACE_FORMAT = "C600-ORACLE-TRACE-draft"
FIELDS = (
    "accepted", "error", "state_hash", "net_sha256", "net_moved_stickers",
    "net_moved_pieces", "expansion_sha256", "primitive_count", "support",
    "conflicts", "progress_sha256",
)
EFFECT_FIELDS = FIELDS[3:-1]
HASH_FIELDS = {"state_hash", "net_sha256", "expansion_sha256", "progress_sha256"}
COUNT_FIELDS = {"net_moved_stickers", "net_moved_pieces", "primitive_count"}


def canonical_json(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Duplicate JSON key: " + key)
        result[key] = value
    return result


def _bad_constant(value):
    raise ValueError("Non-finite JSON number")


def read_document(path):
    """Read JSON or gzip JSON without including input paths in diagnostics."""
    try:
        data = Path(path).read_bytes()
        if data.startswith(b"\x1f\x8b"):
            data = gzip.decompress(data)
        return json.loads(data.decode("utf-8"), object_pairs_hook=_object,
                          parse_constant=_bad_constant)
    except (OSError, EOFError, UnicodeError, ValueError, zlib.error) as exc:
        raise ValueError("Unreadable or malformed oracle JSON file") from exc


def _require(condition, message):
    if not condition:
        raise ValueError(message)


def _text(value):
    return isinstance(value, str) and bool(value)


def _hash(value):
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def _model(document):
    model = document.get("model")
    _require(isinstance(model, dict), "Missing model identity")
    _require(_text(model.get("model_id")), "Missing or malformed model.model_id")
    _require(_hash(model.get("manifest_sha256")),
             "Missing or malformed model.manifest_sha256")


def _field(field, value):
    if field == "accepted":
        valid = type(value) is bool
    elif field == "error":
        valid = value is None or value in ("invalid-input", "protected")
    elif field in HASH_FIELDS:
        valid = _hash(value) or (value is None and field in EFFECT_FIELDS)
    elif field in COUNT_FIELDS:
        valid = value is None or (type(value) is int and value >= 0)
    else:
        valid = value is None or isinstance(value, list)
        if isinstance(value, list):
            valid = all(
                isinstance(row, dict) and set(row) == {"orbit", "stickers", "pieces"}
                and type(row["orbit"]) is int and -1 <= row["orbit"] < 35
                and all(type(row[key]) is int and row[key] >= 0
                        for key in ("stickers", "pieces"))
                for row in value
            )
    _require(valid, "Malformed step field: " + field)


def _steps(document, *, case):
    steps = document.get("steps")
    _require(isinstance(steps, list), "Missing or malformed steps")
    _require(not case or bool(steps), "A case must contain steps")
    seen = set()
    for step in steps:
        _require(isinstance(step, dict), "Malformed step")
        index = step.get("index")
        _require(type(index) is int and index >= 0 and index not in seen,
                 "Missing, duplicate or malformed step index")
        seen.add(index)
        if case:
            # Invalid recipes deliberately remain unvalidated here.
            _require("recipe" in step, "Missing step recipe")
            values = step.get("expect")
            _require(isinstance(values, dict) and all(key in values for key in FIELDS),
                     "Missing or malformed step expectations")
        else:
            values = step
        for field in FIELDS:
            if field in values:
                _field(field, values[field])
        if case:
            invalid = values["error"] == "invalid-input"
            _require(values["error"] is None if values["accepted"]
                     else values["error"] in ("invalid-input", "protected"),
                     "Inconsistent step acceptance and error")
            _require(all((values[field] is None) == invalid for field in EFFECT_FIELDS),
                     "Invalid-input effects must be null; valid effects must be present")


def validate_case(case):
    _require(isinstance(case, dict), "Malformed case")
    _require(case.get("format") == CASE_FORMAT, "Wrong case format")
    _require(_text(case.get("case_id")), "Missing case_id")
    _require(_text(case.get("description")), "Missing description")
    _model(case)
    _require(case.get("mode") in ("chain", "independent"), "Malformed case mode")
    initial = case.get("initial")
    _require(isinstance(initial, dict), "Missing initial state")
    moves = initial.get("moves")
    _require(isinstance(moves, list) and all(type(m) is int and 1 <= abs(m) <= 1200
                                           for m in moves), "Malformed initial moves")
    _require(initial.get("kind") in ("solved", "word")
             and (not moves if initial["kind"] == "solved" else bool(moves)),
             "Malformed initial kind")
    _require(_hash(initial.get("state_hash")), "Missing initial state_hash")
    protected = case.get("protected")
    _require(isinstance(protected, list)
             and all(type(o) is int and 0 <= o < 35 for o in protected),
             "Malformed protected orbits")
    generator = case.get("generator")
    _require(isinstance(generator, dict), "Missing generator provenance")
    _require(all(_text(generator.get(key)) for key in ("script", "python", "numpy"))
             and all(_hash(generator.get(key))
                     for key in ("script_sha256", "core_sha256", "session_sha256"))
             and type(generator.get("seed")) is int, "Malformed generator provenance")
    # Draft extension: per-component omissions, keyed by candidate.name.
    optional = case.get("optional_fields", {})
    _require(isinstance(optional, dict) and all(
        _text(component) and isinstance(fields, list)
        and all(isinstance(field, str) and field in FIELDS for field in fields)
        for component, fields in optional.items()), "Malformed optional_fields")
    _steps(case, case=True)


def validate_trace(trace):
    _require(isinstance(trace, dict), "Malformed trace")
    _require(trace.get("format") == TRACE_FORMAT, "Wrong trace format")
    _require(_text(trace.get("case_id")), "Missing trace case_id")
    _model(trace)
    candidate = trace.get("candidate")
    _require(isinstance(candidate, dict)
             and all(_text(candidate.get(key)) for key in ("name", "build")),
             "Missing or malformed candidate identity")
    _steps(trace, case=False)


def _rows(value):
    return None if value is None else {canonical_json(row) for row in value}


def compare_documents(case, trace):
    """Return (exit code, report); validate identities and step coverage first."""
    try:
        validate_case(case)
        validate_trace(trace)
        _require(trace["case_id"] == case["case_id"], "Different case_id")
        for key in ("model_id", "manifest_sha256"):
            _require(trace["model"][key] == case["model"][key],
                     "Different model." + key)
        actual_steps = {step["index"]: step for step in trace["steps"]}
        expected_indices = {step["index"] for step in case["steps"]}
        _require(expected_indices <= actual_steps.keys(), "Missing step")
        _require(actual_steps.keys() <= expected_indices, "Unexpected step")
    except ValueError as exc:
        return 2, {"status": "error", "error": str(exc)}

    omissions = []
    optional = set(case.get("optional_fields", {}).get(trace["candidate"]["name"], []))
    for step in sorted(case["steps"], key=lambda item: item["index"]):
        actual = actual_steps[step["index"]]
        differences = []
        for field in FIELDS:
            expected = step["expect"][field]
            if field not in actual:
                if field in optional:
                    omissions.append({"index": step["index"], "field": field})
                    continue
                differences.append({"field": field, "expected": expected,
                                    "actual": {"missing": True}})
                continue
            equal = (_rows(expected) == _rows(actual[field])
                     if field in ("support", "conflicts") else expected == actual[field])
            if not equal:
                differences.append({"field": field, "expected": expected,
                                    "actual": actual[field]})
        if differences:
            return 1, {"status": "mismatch", "case_id": case["case_id"],
                       "index": step["index"], "recipe": step["recipe"],
                       "differences": differences, "omitted_fields": omissions}
    return 0, {"status": "match-with-omissions" if omissions else "equal",
               "case_id": case["case_id"], "steps": len(case["steps"]),
               "omitted_fields": omissions}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case")
    parser.add_argument("trace")
    args = parser.parse_args(argv)
    try:
        code, report = compare_documents(read_document(args.case), read_document(args.trace))
    except ValueError as exc:
        code, report = 2, {"status": "error", "error": str(exc)}
    print(canonical_json(report))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
