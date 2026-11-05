"""Mechanical control rendering and mandatory candidate binding verification."""

import json
from pathlib import Path

from .common import ROOT, canonical, cases, digest, load, sha
from .result import require


def within_root(path):
    path = Path(path).resolve()
    require(path.is_relative_to(ROOT), "path outside part-b")
    return path


def neutral_paths(program):
    result = {}
    def block(body, prefix):
        for i, instruction in enumerate(body):
            path = prefix + "/" + str(i)
            result[path] = instruction
            for branch in ("body", "then", "else"):
                if branch in instruction:
                    block(instruction[branch], path + "/" + branch)
    for run in program["runs"]:
        block(run["body"], run["id"] + "/body")
    for name, module in program["modules"].items():
        block(module["body"], "modules/" + name + "/body")
    return result


def render_controls():
    """Canonical neutral JSON is the executed notation for the controls only."""
    programs = {case["id"]: case["program"] for _, case in cases()}
    watched = {id(op): (case_id, path) for case_id, program in programs.items()
               for path, op in neutral_paths(program).items()}
    source_map = {case_id: {} for case_id in programs}
    output = bytearray()
    def emit(value):
        start = len(output)
        if type(value) is dict:
            output.extend(b"{")
            for i, key in enumerate(sorted(value)):
                if i:
                    output.extend(b",")
                output.extend(canonical(key) + b":")
                emit(value[key])
            output.extend(b"}")
        elif type(value) is list:
            output.extend(b"[")
            for i, item in enumerate(value):
                if i:
                    output.extend(b",")
                emit(item)
            output.extend(b"]")
        else:
            output.extend(canonical(value))
        if id(value) in watched:
            case_id, path = watched[id(value)]
            source_map[case_id][path] = {"start": start, "end": len(output),
                                        "sha256": sha(bytes(output[start:]))}
    emit(programs)
    assert bytes(output) == canonical(programs)
    return bytes(output), source_map


def validate(manifest, bindings):
    required = {"version", "candidate_id", "source_files", "edition", "execution_contract",
                "charge_mapping", "kernel_implementation", "checkers", "effective_grants",
                "provenance_contract", "consumed_inputs", "source_map", "rendered_file"}
    require(set(bindings) == required, "missing candidate binding")
    require(set(manifest) == {"version", "id", "adapter", "cells"}, "missing candidate declaration")
    require(manifest["id"] == bindings["candidate_id"], "candidate identity mismatch")
    require(manifest["version"] == bindings["version"] == 1, "unsupported interface version")
    policy = load("policy.json")
    cells = manifest["cells"]
    require(len(cells) == len(policy["forms"]) * len(policy["variants"]), "omitted form or variant")
    require({(c["form"], c["variant"]) for c in cells} == {
        (f, v["id"]) for f in policy["forms"] for v in policy["variants"]}, "omitted or duplicate cell")
    for cell in cells:
        require(set(cell) == {"form", "variant", "status", "reason"}, "missing cell field")
        require(cell["status"] in ("supported", "not_run"), "invalid cell status")
        require(cell["reason"] is None if cell["status"] == "supported" else bool(cell["reason"]), "missing not_run reason")
    for field in ("edition", "execution_contract", "charge_mapping", "provenance_contract", "consumed_inputs"):
        require(bool(bindings[field]), "empty binding: " + field)
    require(set(bindings["effective_grants"]) == set(policy["profiles"]), "missing effective grants")
    from .policy import GRANTS
    require(all(type(grants) is list and len(grants) == len(set(grants)) and set(grants) <= set(GRANTS)
                for grants in bindings["effective_grants"].values()), "invalid effective grant declaration")
    shared = {p.relative_to(ROOT).as_posix() for p in (ROOT / "harness").glob("*.py")} | {"run_matrix.py"}
    require(shared <= set(bindings["source_files"]), "missing shared implementation dependency")
    require(bindings["rendered_file"] in bindings["source_files"], "unbound rendering")
    for field in ("kernel_implementation", "checkers"):
        require(bool(bindings[field]), "empty implementation identity")
        for path, expected in bindings[field].items():
            require(bindings["source_files"].get(path) == expected, "unresolved implementation identity")
    source_bytes = {}
    for path, expected in bindings["source_files"].items():
        require(not Path(path).is_absolute() and "\\" not in path and ".." not in Path(path).parts, "nonportable source path")
        source = within_root(ROOT / path)
        require(source.is_file(), "missing source: " + path)
        source_bytes[path] = source.read_bytes()
        require(sha(source_bytes[path]) == expected, "source digest mismatch: " + path)
    require(set(bindings["source_map"]) == {case["id"] for _, case in cases()}, "omitted rendered case")
    for _, case in cases():
        mapping = bindings["source_map"][case["id"]]
        require(set(mapping) == set(neutral_paths(case["program"])), "omitted neutral instruction mapping")
        for item in mapping.values():
            require(set(item) == {"file", "start", "end", "sha256"}, "incomplete executed source map")
            require(item["file"] in bindings["source_files"], "unbound source map bytes")
            data = source_bytes[item["file"]]
            require(type(item["start"]) is int and type(item["end"]) is int and 0 <= item["start"] < item["end"] <= len(data), "invalid source range")
            require(sha(data[item["start"]:item["end"]]) == item["sha256"], "source map mismatch")


def candidate(folder):
    folder = within_root(folder)
    manifest = json.loads((folder / "candidate.json").read_bytes())
    bindings = json.loads((folder / "bindings.json").read_bytes())
    validate(manifest, bindings)
    for name in ("candidate.json", manifest["adapter"]):
        require((folder / name).relative_to(ROOT).as_posix() in bindings["source_files"], "unbound adapter/declaration")
    return manifest, bindings


def self_test():
    data, mapping = render_controls()
    programs = json.loads(data)
    for _, case in cases():
        assert programs[case["id"]] == case["program"]
        for path, op in neutral_paths(case["program"]).items():
            item = mapping[case["id"]][path]
            assert data[item["start"]:item["end"]] == canonical(op)
