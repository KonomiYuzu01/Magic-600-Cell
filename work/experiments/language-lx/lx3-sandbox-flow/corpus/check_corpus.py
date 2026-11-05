"""Validate/freeze LX-3 inputs. This does not execute the analysis programs."""

import argparse
import copy
import hashlib
import json
import re
import sys
from collections import Counter
from math import gcd
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
VERSION = "lx3-neutral-v1"
COMMON = {"corpus_version", "decision"}
CASE_FILES = (
    "must-refuse.json", "allowed-tools.json", "planted.json", "budgets.json",
    "approval.json",
)
INPUT_FILES = CASE_FILES + (
    "fixture.json", "policy.json", "profile.json", "notation.md", "kernel-stub.md",
)
FILES = set(INPUT_FILES) | {"keys.json", "RESULT.md", "check_corpus.py", "FREEZE.json"}
ARCH = "docs/architecture/1.0/language/"
SOURCE_FILES = (
    "AGENTS.md", "docs/development-guide/AGENT_BRIEFING.md",
    "docs/wiki/decisions/owner-decisions-2026-11-05.md",
    "work/experiments/language-lx/README.md",
    ARCH + "architecture/11-route-and-spikes.md",
    ARCH + "architecture/13-errors.md", ARCH + "architecture/07-boundary.md",
    ARCH + "architecture/09-determinism.md", ARCH + "architecture/02-language-core.md",
    ARCH + "adr/ADR-L06-effects-and-flow.md",
    ARCH + "adr/ADR-L09-boundary-mechanisms.md", ARCH + "ecosystem-partition.md",
)
FAMILIES = {
    "largest_n_c", "search_rebuild", "f2_f5_solve", "workspace_laundering",
    "state_choice", "module_flow", "stop_selected_preview",
    "refusal_driven_selection", "one_item_pick", "accumulation_across_runs",
}
CATEGORIES = {
    "reading_state", "exact_counting", "charts", "explaining",
    "own_word_checking", "named_target_setup", "research_mode",
}
FORMS = {"capability_separation", "static_labels", "static_runtime_labels"}
COUNTERS = {"steps", "kernel_cost", "memory", "output"}
TYPES = {
    "Int", "Bool", "Text", "Word", "Target", "Ints", "Bools", "Words",
    "Targets", "Matrix", "Record", "Rows", "State", "Receipt", "Bytes",
}
ELEMENT = {"Ints": "Int", "Bools": "Bool", "Words": "Word", "Targets": "Target"}
CLASSES = {
    "tracking", "analysis", "checking", "protection", "view_aid",
    "setup_search_named_target", "repair_list",
}
OP_FIELDS = {
    "kernel": "as type row args", "calc": "as type fn args",
    "if": "test then else", "for": "item type items body",
    "call": "as type module args", "workspace_get": "as type key",
    "workspace_put": "key value", "preview": "as type word target",
    "pick": "as type options event", "stop": "reason", "return": "value",
    "emit": "value", "publish": "exact coordinates", "retain": "as type bytes",
    "tick": "", "ambient": "as type resource logical_resource", "dynamic": "source",
}
FUNCTIONS = {
    "argmax_first": ("Int", ["Ints"]), "argmin_first": ("Int", ["Ints"]),
    "rank_desc_indices": ("Ints", ["Ints"]), "append": ("Ints", ["Ints", "Int"]),
    "sum": ("Int", ["Ints"]), "count_equal": ("Int", ["Ints", "Int"]),
    "length": ("Int", ["Word"]), "add": ("Int", ["Int", "Int"]),
    "mod_dot": ("Int", ["Ints", "Ints", "Int"]),
    "linear_solve_mod": ("Ints", ["Matrix", "Ints", "Int"]),
    "combine_corrections": ("Word", ["Words", "Ints"]),
    "concat": ("Word", ["Word", "Word"]), "inverse": ("Word", ["Word"]),
    "zip": ("Rows", ["Targets", "Ints"]), "indexed_points": ("Rows", ["Ints"]),
    "point": ("Rows", ["Int", "Int"]), "gt": ("Bool", ["Int", "Int"]),
}


def need(condition, message):
    if not condition:
        raise ValueError(message)


def shape(value, fields, at):
    need(type(value) is dict, f"{at}: expected object")
    need(set(value) == set(fields), f"{at}: missing/extra fields {sorted(set(value) ^ set(fields))}")


def array(value, at, minimum=0):
    need(type(value) is list and len(value) >= minimum, f"{at}: expected array of size >= {minimum}")


def strings(value, at, minimum=1):
    array(value, at, minimum)
    need(all(type(x) is str and x for x in value), f"{at}: expected nonempty strings")
    need(len(value) == len(set(value)), f"{at}: duplicate values")


def integer(value, at, minimum=0):
    need(type(value) is int and value >= minimum, f"{at}: expected integer >= {minimum}")


def name(value, at):
    need(type(value) is str and re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*", value), f"{at}: invalid name")


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")


def pretty(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2, allow_nan=False) + "\n").encode("utf-8")


def digest(value):
    return hashlib.sha256(value).hexdigest()


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        need(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def bad_number(value):
    raise ValueError(f"non-integer JSON number: {value}")


def load_json(filename):
    return json.loads(HERE.joinpath(filename).read_bytes().decode("utf-8"),
                      object_pairs_hook=unique_object, parse_float=bad_number, parse_constant=bad_number)


def common(value, extra, at):
    shape(value, COMMON | set(extra), at)
    need(value["corpus_version"] == VERSION and value["decision"] == "open", f"{at}: version/decision")


def budget_values(value, at):
    shape(value, COUNTERS, at)
    for key, count in value.items():
        integer(count, f"{at}/{key}")


def rational(value, at):
    shape(value, {"numerator", "denominator"}, at)
    need(type(value["numerator"]) is int, f"{at}: rational numerator")
    integer(value["denominator"], at, 1)
    need(gcd(value["numerator"], value["denominator"]) == 1, f"{at}: unreduced rational")


def literal(kind, value, fixture, at):
    need(kind in TYPES, f"{at}: unknown type {kind}")
    if kind == "Int":
        need(type(value) is int, f"{at}: expected exact integer")
    elif kind == "Bool":
        need(type(value) is bool, f"{at}: expected boolean")
    elif kind == "Text":
        need(type(value) is str, f"{at}: expected text")
    elif kind == "Target":
        need(value in fixture["caps"], f"{at}: unknown fake target")
    elif kind == "Word":
        array(value, at)
        need(all(x in fixture["identity"]["menu_order"] for x in value), f"{at}: unknown fake letter")
    elif kind in ELEMENT:
        array(value, at)
        for item in value:
            literal(ELEMENT[kind], item, fixture, at)
    elif kind == "Matrix":
        array(value, at, 1)
        for row in value:
            literal("Ints", row, fixture, at)
        need(len(value[0]) > 0 and len({len(row) for row in value}) == 1, f"{at}: ragged matrix")
    elif kind == "Rows":
        array(value, at)
    elif kind == "State":
        literal("Ints", value, fixture, at)
        need(sorted(value) == [0, 1, 2, 3], f"{at}: invalid fake labels")
    elif kind == "Record":
        need(type(value) is dict, f"{at}: expected record")
    else:
        raise ValueError(f"{at}: {kind} is host-created, not a literal")


def program_schema(program, fixture, at):
    shape(program, {"inputs", "modules", "runs"}, at)
    need(type(program["inputs"]) is dict and type(program["modules"]) is dict, f"{at}: inputs/modules")
    inputs = {}
    for key, value in program["inputs"].items():
        name(key, at)
        shape(value, {"type", "value", "origin"}, at)
        literal(value["type"], value["value"], fixture, at)
        need(value["origin"] in {"manual", "fixture", "unknown"}, f"{at}: input origin")
        inputs[key] = value["type"]
    modules = program["modules"]
    for key, module in modules.items():
        name(key, at)
        shape(module, {"parameters", "result", "body"}, at)
        need(type(module["parameters"]) is dict and module["result"] in TYPES, f"{at}: module signature")
        for parameter, kind in module["parameters"].items():
            name(parameter, at)
            need(kind in TYPES, f"{at}: parameter type")
    workspace = {}

    def expr(value, scope, where):
        need(type(value) is dict, f"{where}: value expression")
        if set(value) == {"ref"}:
            need(value["ref"] in scope, f"{where}: unbound reference {value['ref']}")
            return scope[value["ref"]]
        shape(value, {"type", "value"}, where)
        literal(value["type"], value["value"], fixture, where)
        return value["type"]

    def body_schema(body, scope, where, result_type=None):
        array(body, where)
        for index, step in enumerate(body):
            site = f"{where}/{index}"
            need(type(step) is dict and step.get("op") in OP_FIELDS, f"{site}: unknown instruction")
            op = step["op"]
            shape(step, {"op"} | set(OP_FIELDS[op].split()), site)
            get = lambda value: expr(value, scope, site)
            actual = None
            if op in {"kernel", "calc", "call"}:
                array(step["args"], site)
                args = [get(x) for x in step["args"]]
                if op == "kernel":
                    need(step["row"] in fixture["rows"], f"{site}: unknown row")
                    row = fixture["rows"][step["row"]]
                    actual, wanted = row["result"], row["args"]
                elif op == "call":
                    need(step["module"] in modules, f"{site}: unknown module")
                    module = modules[step["module"]]
                    actual = module["result"]
                    wanted = [module["parameters"][x] for x in sorted(module["parameters"])]
                else:
                    fn = step["fn"]
                    if fn in FUNCTIONS:
                        actual, wanted = FUNCTIONS[fn]
                    elif fn == "identity":
                        need(len(args) == 1, f"{site}: identity arity")
                        actual, wanted = args[0], args
                    elif fn == "index":
                        need(len(args) == 2 and args[0] in ELEMENT, f"{site}: index operands")
                        actual, wanted = ELEMENT[args[0]], [args[0], "Int"]
                    elif fn == "field":
                        actual, wanted = step["type"], ["Record", "Text"]
                    elif fn == "eq":
                        need(len(args) == 2 and args[0] == args[1], f"{site}: equality operands")
                        actual, wanted = "Bool", args
                    elif fn == "singleton":
                        need(len(args) == 1 and args[0] in {"Word", "Target"}, f"{site}: singleton operand")
                        actual, wanted = {"Word": "Words", "Target": "Targets"}[args[0]], args
                    else:
                        raise ValueError(f"{site}: unknown exact operation {fn}")
                need(args == wanted, f"{site}: argument types {args}, expected {wanted}")
            elif op == "if":
                need(get(step["test"]) == "Bool", f"{site}: non-boolean branch")
                yes, no = dict(scope), dict(scope)
                body_schema(step["then"], yes, site + "/then", result_type)
                body_schema(step["else"], no, site + "/else", result_type)
                for key in yes.keys() & no.keys():
                    need(yes[key] == no[key], f"{site}: inconsistent branch types")
                    scope[key] = yes[key]
            elif op == "for":
                items = get(step["items"])
                need(ELEMENT.get(items) == step["type"], f"{site}: loop item type")
                name(step["item"], site)
                child = dict(scope)
                child[step["item"]] = step["type"]
                body_schema(step["body"], child, site + "/body", result_type)
            elif op == "workspace_get":
                name(step["key"], site)
                need(step["key"] in workspace, f"{site}: missing earlier workspace record")
                actual = workspace[step["key"]]
            elif op == "workspace_put":
                name(step["key"], site)
                workspace[step["key"]] = get(step["value"])
            elif op == "preview":
                need(get(step["word"]) == "Word" and get(step["target"]) == "Target", f"{site}: preview operands")
                actual = "Receipt"
            elif op == "pick":
                actual = ELEMENT.get(get(step["options"]))
                need(actual in {"Word", "Target"}, f"{site}: pick options")
                event = program["inputs"].get(step["event"])
                need(event is not None and event["type"] == "Int" and event["origin"] == "manual", f"{site}: pick input")
                integer(event["value"], site)
            elif op in {"return", "emit"}:
                returned = get(step["value"])
                if op == "return" and result_type is not None:
                    need(returned == result_type, f"{site}: module return type")
            elif op == "publish":
                need(get(step["exact"]) == "Record" and get(step["coordinates"]) == "Rows", f"{site}: display fields")
            elif op == "retain":
                integer(step["bytes"], site)
                actual = "Bytes"
            elif op == "ambient":
                need(step["resource"] in {"clock", "file", "network", "process"}, f"{site}: ambient kind")
                name(step["logical_resource"], site)
                actual = "Text"
            elif op == "dynamic":
                need(get(step["source"]) == "Text", f"{site}: dynamic input")
            elif op == "stop":
                need(step["reason"] == "requested", f"{site}: stop reason")
            if "as" in step:
                name(step["as"], site)
                need(step["type"] in TYPES and actual == step["type"], f"{site}: result type")
                need(step["as"] not in scope or scope[step["as"]] == actual, f"{site}: rebind type")
                scope[step["as"]] = actual

    for key, module in modules.items():
        body_schema(module["body"], inputs | module["parameters"], at + "/module/" + key, module["result"])
    array(program["runs"], at, 1)
    run_ids = []
    for run in program["runs"]:
        shape(run, {"id", "profile", "author", "boundary_class", "invocation", "body"}, at)
        name(run["id"], at)
        run_ids.append(run["id"])
        need(run["profile"] in {"solving", "tool_authoring", "research"}, f"{at}: profile")
        need(run["author"] in {"built_in", "own", "forked_built_in", "imported"}, f"{at}: author")
        need(run["boundary_class"] in CLASSES or run["boundary_class"] is None, f"{at}: boundary class")
        need(run["author"] != "built_in" or run["boundary_class"] in CLASSES, f"{at}: missing built-in class")
        need(run["invocation"] == "command", f"{at}: invocation")
        body_schema(run["body"], dict(inputs), at + "/" + run["id"])
    need(len(run_ids) == len(set(run_ids)), f"{at}: duplicate run id")


def source_contracts():
    sources = {name: ROOT.joinpath(name).read_bytes() for name in SOURCE_FILES}
    text = sources[ARCH + "architecture/13-errors.md"].decode("utf-8")
    registry = text.split("## 5. Registry by source\n", 1)[1].split("## 6. Reconciliation", 1)[0]
    rows = {}
    for line in registry.splitlines():
        match = re.match(r"\| `([A-Z0-9]+:[a-z_]+\.[a-z_]+)` \|", line)
        if match:
            rows[match[1]] = not any(word in line for word in ("Reserved:", "Retired before", "pending LQ21"))
    clauses = set()
    for raw in sources.values():
        clauses.update(re.findall(r"\*\*([A-Z]+[0-9]*\.[0-9]+)\b", raw.decode("utf-8")))
    need(len(rows) > 50, "chapter 13 registry table could not be read")
    return sources, rows, clauses


def validate(data, registry, clauses):
    fixture, policy, profile = (data[x] for x in ("fixture.json", "policy.json", "profile.json"))
    common(fixture, {"id", "kind", "identity", "session", "caps", "counts", "floors", "blocked", "nu2", "nu5", "matrix2", "matrix5", "correction_words2", "correction_words5", "permutations", "inverses", "setups", "buffers", "bits", "workspace", "pending_preview", "universe", "rows"}, "fixture")
    shape(fixture["identity"], {"model", "menu", "menu_order", "contract", "level"}, "identity")
    shape(fixture["session"], {"revision", "labels", "journal", "protected_slots", "preferences", "recovery_checkpoint"}, "session")
    literal("State", fixture["session"]["labels"], fixture, "session labels")
    for key in ("counts", "floors", "nu2", "nu5", "bits", "buffers"):
        literal("Ints", fixture[key], fixture, key)
    literal("Bools", fixture["blocked"], fixture, "blocked")
    strings(fixture["caps"], "caps")
    strings(fixture["identity"]["menu_order"], "menu")
    integer(fixture["session"]["revision"], "revision")
    literal("State", fixture["session"]["recovery_checkpoint"], fixture, "checkpoint")
    literal("Ints", fixture["session"]["protected_slots"], fixture, "protection")
    need(all(0 <= x < 4 for x in fixture["session"]["protected_slots"]), "protected slot range")
    need(fixture["session"]["journal"] == [] and fixture["workspace"] == {}, "fresh fixture state")
    need(len(fixture["caps"]) == len(fixture["counts"]) == len(fixture["floors"]) == len(fixture["blocked"]), "cap fact lengths")
    for field in ("matrix2", "matrix5"):
        literal("Matrix", fixture[field], fixture, field)
    for field in ("correction_words2", "correction_words5"):
        literal("Words", fixture[field], fixture, field)
    need(set(fixture["permutations"]) == set(fixture["inverses"]) == set(fixture["identity"]["menu_order"]), "letter tables")
    for row in fixture["rows"].values():
        shape(row, {"args", "result", "cost"}, "kernel row")
        array(row["args"], "kernel args")
        need(all(x in TYPES for x in row["args"]) and row["result"] in TYPES, "kernel signature")
        need(type(row["cost"]) in {int, str}, "kernel charge")
    for perm in fixture["permutations"].values():
        need(sorted(perm) == [0, 1, 2, 3], "invalid fake permutation")
    for letter, inverse in fixture["inverses"].items():
        need(inverse in fixture["permutations"], "unknown inverse")
        need([fixture["permutations"][inverse][i] for i in fixture["permutations"][letter]] == [0, 1, 2, 3], "inverse fixture mismatch")
    need(set(fixture["setups"]) == set(fixture["caps"]), "setup targets")
    for words in fixture["setups"].values():
        literal("Words", words, fixture, "setups")
        for word in words:
            # Fixture integrity only, not an interpreter or real puzzle check.
            positions = list(fixture["buffers"])
            for letter in word:
                positions = [fixture["permutations"][letter][i] for i in positions]
            need(positions == fixture["buffers"], "setup fixture moves a fixed buffer")
    shape(fixture["pending_preview"], {"word", "target", "provenance"}, "pending preview")
    literal("Word", fixture["pending_preview"]["word"], fixture, "pending word")
    literal("Target", fixture["pending_preview"]["target"], fixture, "pending target")
    common(policy, {"id", "scope", "basis", "forms", "profiles", "admitted_author_classes", "solver_tool_solving_preview", "research_to_solving_preview", "flow_rule", "stop_rule", "display_rule", "transparent", "declassifiers", "repair_exception", "grants", "preservation", "refusal_rule", "form_prediction_rule", "variants", "open_gaps"}, "policy")
    need(set(policy["forms"]) == FORMS and policy["admitted_author_classes"] == [], "policy forms/admission")
    need(set(policy["profiles"]) == {"solving", "tool_authoring", "research"}, "policy profiles")
    need(set(policy["basis"]) <= clauses, "policy source clause")
    shape(policy["grants"], {"solving_built_in", "tool_authoring", "research", "ambient", "network", "process"}, "grants")
    for grant in policy["grants"].values():
        strings(grant, "grant descriptors", minimum=0)
    strings(policy["transparent"], "transparent entries")
    for entry in policy["declassifiers"]:
        shape(entry, {"id", "entry", "requires", "clears", "output"}, "declassifier")
        strings(entry["requires"], "declassifier preconditions")
        strings(entry["clears"], "cleared labels")
        need(set(entry["clears"]) <= {"state", "computed"}, "cannot clear research/unknown provenance")
    need({x["id"] for x in policy["variants"]} == {"source_bound", "approved_subset_byte_bound"}, "source approval variants")
    for variant in policy["variants"]:
        shape(variant, {"id", "routes", "requires"}, "variant")
        strings(variant["routes"], "routes")
        strings(variant["requires"], "variant conditions")
    need(policy["solver_tool_solving_preview"] is False and policy["research_to_solving_preview"] is False, "interim preview boundary")
    need(all(policy["grants"][x] == [] for x in ("ambient", "network", "process")), "ambient grants")
    strings(policy["preservation"], "preservation")
    gaps = {}
    for gap in policy["open_gaps"]:
        shape(gap, {"id", "status", "clauses", "nearest_code", "detail"}, "gap")
        need(gap["status"] == "open" and registry.get(gap["nearest_code"]) is True, "gap status/code")
        need(set(gap["clauses"]) <= clauses and gap["id"] not in gaps, "gap reference/identity")
        gaps[gap["id"]] = gap
    common(profile, {"id", "canonical_json", "exact_payload_fields", "exact_payload_shape", "hash_rule", "display", "excluded_fields", "default_budgets", "units", "meter", "comparison_runs", "nonreproducible", "decisions"}, "profile")
    need(set(profile["exact_payload_fields"]) == {"outcome", "code", "context", "input_identities", "coverage", "budgets", "value", "words", "certificates", "checker_reports", "traces", "stop"}, "DET.6 payload fields")
    need(set(profile["exact_payload_shape"]) == set(profile["exact_payload_fields"]), "payload shape")
    shape(profile["canonical_json"], {"encoding", "key_order", "separators", "trailing_newline", "unicode", "numbers", "exact_rational", "literal_words"}, "canonical JSON")
    need(profile["canonical_json"]["separators"] == [",", ":"] and profile["canonical_json"]["trailing_newline"] is False, "canonical separators/newline")
    shape(profile["display"], {"exact_fields", "approximate_fields", "rule"}, "display contract")
    need(set(profile["display"]["exact_fields"]) == {"kind", "object_ids", "order", "source_digest", "input_digest", "exact_properties"}, "exact display fields")
    shape(profile["display"]["approximate_fields"], {"coordinates"}, "approximate display fields")
    precision = profile["display"]["approximate_fields"]["coordinates"]
    shape(precision, {"absolute_tolerance", "relative_tolerance", "unit", "domain"}, "precision")
    for field in ("absolute_tolerance", "relative_tolerance"):
        rational(precision[field], "precision")
        need(precision[field]["numerator"] >= 0, "negative display tolerance")
    shape(profile["meter"], {"scope", "steps", "kernel_cost", "memory", "output", "admission", "lifetime", "recursion"}, "meter")
    budget_values(profile["default_budgets"], "default budgets")
    shape(profile["units"], COUNTERS, "units")
    shape(profile["decisions"], {"LQ2", "LQ9", "LQ21", "BS1", "BS2", "BS3", "BS4", "LP1_metering"}, "decisions")
    need(all(x == "open" for x in profile["decisions"].values()), "owner decisions must stay open")
    need(profile["comparison_runs"] == {"Windows": 10, "headless_Linux": 10}, "RTE.46 repeat counts")
    need({"timings", "host", "physical_paths", "preview_id", "cache_diagnostics"} <= set(profile["excluded_fields"]), "excluded host fields")
    need(not set(profile["exact_payload_fields"]) & set(profile["excluded_fields"]), "exact fields cannot be excluded")
    common(data["must-refuse.json"], {"families", "cases"}, "must-refuse")
    common(data["allowed-tools.json"], {"denominator", "categories", "count_rule", "cases"}, "allowed-tools")
    common(data["planted.json"], {"cases"}, "planted")
    common(data["budgets.json"], {"budget_fields", "non_budget_cases", "repetition_rule", "cases"}, "budgets")
    common(data["approval.json"], {"cases"}, "approval")
    required = {"id", "clauses", "program", "unchanged"}
    fields = {
        "must-refuse.json": {"family", "flow", "expected_code", "code_gap", "forms"},
        "allowed-tools.json": {"category", "why_allowed", "expected"},
        "planted.json": {"resource", "plant", "grants", "expected_code", "forbidden_data", "forbidden_sinks", "expected_trap_calls"},
        "budgets.json": {"kind", "limits", "host", "expected"},
        "approval.json": {"kind", "host_mutation", "expected_code", "code_gap", "variants"},
    }
    ids = set()
    for filename in CASE_FILES:
        cases = data[filename]["cases"]
        array(cases, filename, 1)
        for case in cases:
            at = filename + "/" + str(case.get("id"))
            extra = {"comparison"} if case.get("kind") == "cache_changes" else set()
            shape(case, required | fields[filename] | extra, at)
            name(case["id"], at)
            need(case["id"] not in ids, f"{at}: duplicate id")
            ids.add(case["id"])
            strings(case["clauses"], at)
            need(set(case["clauses"]) <= clauses, f"{at}: unknown source clause")
            strings(case["unchanged"], at)
            need(set(policy["preservation"]) <= set(case["unchanged"]), f"{at}: incomplete unchanged state")
            program_schema(case["program"], fixture, at)
            code = case.get("expected_code", case.get("expected", {}).get("code"))
            need(code is None or registry.get(code) is True, f"{at}: unknown, reserved or retired expected code {code}")
            if "code_gap" in case:
                need(case["code_gap"] is None or case["code_gap"] in gaps, f"{at}: unknown gap")
                if case["code_gap"] is not None:
                    need(code == gaps[case["code_gap"]]["nearest_code"], f"{at}: gap/code mismatch")
            if filename == "must-refuse.json":
                need(case["family"] in FAMILIES and code is not None, f"{at}: family/code")
                shape(case["forms"], FORMS, at)
                for prediction in case["forms"].values():
                    shape(prediction, {"expected_to_catch", "clauses", "reason"}, at)
                    need(type(prediction["expected_to_catch"]) is bool and bool(prediction["reason"]), f"{at}: form prediction")
                    need(set(prediction["clauses"]) <= clauses, f"{at}: prediction clause")
            elif filename == "allowed-tools.json":
                need(case["category"] in CATEGORIES and bool(case["why_allowed"]), f"{at}: harmless category/reason")
                result = case["expected"]
                shape(result, {"outcome", "code", "value", "emitted", "display", "previews", "workspace_writes"}, at)
                need(result["outcome"] == "completed" and result["code"] is None, f"{at}: harmless expected outcome")
                for field in ("emitted", "previews", "workspace_writes"):
                    array(result[field], at)
                for preview in result["previews"]:
                    shape(preview, {"word", "target", "destination"}, at)
                    literal("Word", preview["word"], fixture, at)
                    literal("Target", preview["target"], fixture, at)
                    need(preview["destination"] in {"capture", "solving_preview"}, f"{at}: preview destination")
                if result["display"] is not None:
                    shape(result["display"], {"exact", "coordinates"}, at)
                    literal("Record", result["display"]["exact"], fixture, at)
                    literal("Rows", result["display"]["coordinates"], fixture, at)
                    exact = result["display"]["exact"]
                    shape(exact, set(profile["display"]["exact_fields"]), at)
                    need(exact["source_digest"] == digest(canonical(case["program"])), f"{at}: display source binding")
                    need(exact["input_digest"] == digest(canonical(fixture["session"])), f"{at}: display input binding")
                    for point in result["display"]["coordinates"]:
                        array(point, at, 2)
                        need(len(point) == 2, f"{at}: fixture display dimension")
                        for coordinate in point:
                            if type(coordinate) is dict:
                                rational(coordinate, at)
                                need(abs(coordinate["numerator"]) <= 16 * coordinate["denominator"], f"{at}: coordinate domain")
                            else:
                                need(type(coordinate) is int and -16 <= coordinate <= 16, f"{at}: exact display reference")
                for write in result["workspace_writes"]:
                    shape(write, {"key", "value", "profile"}, at)
                    name(write["key"], at)
                    need(write["profile"] in policy["profiles"], f"{at}: workspace profile")
            elif filename == "planted.json":
                shape(case["plant"], {"logical_resource", "data", "mechanism"}, at)
                strings(case["forbidden_data"], at)
                need(case["plant"]["data"] in case["forbidden_data"], f"{at}: missing planted marker")
                need(case["grants"] == [] and case["expected_trap_calls"] == 0, f"{at}: planted access must be denied")
                strings(case["forbidden_sinks"], at)
                need(case["resource"] in {"clock", "file", "network", "process"} and code is not None, f"{at}: planted kind/code")
                need({"value", "context", "diagnostics", "display", "workspace", "file_output", "traces"} == set(case["forbidden_sinks"]), f"{at}: leakage sinks")
            elif filename == "budgets.json":
                if case["limits"] is not None:
                    budget_values(case["limits"], at)
                result = case["expected"]
                shape(result, {"outcome", "code", "value", "used", "stop", "coverage"}, at)
                need(result["outcome"] in {"completed", "cut_off", "cancelled", "error", "refused"}, f"{at}: outcome")
                if result["used"] is not None:
                    budget_values(result["used"], at)
                    if case["limits"] is not None:
                        need(all(result["used"][x] <= case["limits"][x] for x in COUNTERS), f"{at}: counters exceed grant")
                shape(result["coverage"], {"examined", "total", "complete"}, at)
                need(type(result["coverage"]["complete"]) is bool, f"{at}: coverage boolean")
                coverage = result["coverage"]
                literal("Ints", coverage["total"], fixture, at)
                if coverage["examined"] is not None:
                    literal("Ints", coverage["examined"], fixture, at)
                    need(coverage["total"][:len(coverage["examined"])] == coverage["examined"], f"{at}: coverage prefix")
                if coverage["complete"]:
                    need(result["outcome"] == "completed" and coverage["examined"] == coverage["total"], f"{at}: false completion")
                if result["outcome"] != "completed":
                    need(code is not None and type(result["stop"]) is str and bool(result["stop"]), f"{at}: missing stop")
                else:
                    need(code is None and result["stop"] is None, f"{at}: completed stop")
                for variant in case["host"].get("variants", []):
                    shape(variant, {"id", "cache", "expected_host_code"}, at)
                    host_code = variant["expected_host_code"]
                    need(host_code is None or registry.get(host_code) is True, f"{at}: cache code")
            else:
                shape(case["host_mutation"], {"operation", "approval_scope"}, at)
                need(case["variants"] == ["approved_subset_byte_bound"], f"{at}: approval variant")
    family_counts = Counter(x["family"] for x in data["must-refuse.json"]["cases"])
    need(set(data["must-refuse.json"]["families"]) == FAMILIES, "RTE.45 family declaration")
    need(set(family_counts) == FAMILIES and min(family_counts.values()) >= 2, "RTE.45 requires two cases per family")
    allowed = data["allowed-tools.json"]
    need(type(allowed["denominator"]) is int and allowed["denominator"] == len(allowed["cases"]) >= 30, "allowed denominator")
    need(set(allowed["categories"]) == CATEGORIES == {x["category"] for x in allowed["cases"]}, "harmless category coverage")
    need({x["resource"] for x in data["planted.json"]["cases"]} == {"clock", "file", "network", "process"}, "planted resource coverage")
    budgets = data["budgets.json"]
    need(set(budgets["budget_fields"]) == COUNTERS, "budget field coverage")
    need(COUNTERS | {"interacting_limits", "interruption", "safety_stop", "runtime_loss", "cache_changes"} <= {x["kind"] for x in budgets["cases"]}, "budget scenario coverage")
    for field in COUNTERS:
        probes = [x for x in budgets["cases"] if x["kind"] == field]
        need(any(x["expected"]["code"] == "KN7:budget." + field and x["expected"]["used"] is not None for x in probes), "missing independent budget expectation")


def make_keys(data, sources):
    fixture, policy, profile = (data[x] for x in ("fixture.json", "policy.json", "profile.json"))
    keys = []
    for filename in CASE_FILES:
        for case in data[filename]["cases"]:
            program = case["program"]
            previous = None
            for run in program["runs"]:
                # Host cache state, deadlines, actual cancellation positions and expected
                # answers are deliberately absent. Their fixture specs are file-pinned.
                expanded = {
                    "source": digest(canonical(program)),
                    "modules": {key: digest(canonical(value)) for key, value in program["modules"].items()},
                    "edition": "analysis-notation-v1-not-a-language-edition",
                    "ir_version": "neutral-stimulus-v1-not-product-IR",
                    "identity": fixture["identity"], "input": digest(canonical(fixture["session"])),
                    "parameters": {"inputs": program["inputs"], "run": run,
                                   "fixture": digest(canonical(fixture)), "policy": digest(canonical(policy)),
                                   "payload_profile": digest(canonical(profile)),
                                   "initial_workspace": digest(canonical(fixture["workspace"])),
                                   "admission_constraints": case.get("host", {}).get("ceiling", {}),
                                   "approval_attempt": case.get("host_mutation"),
                                   "prior_run_input_key": previous},
                    "seeds": [], "budgets": case.get("limits", profile["default_budgets"]),
                    "kernel_cost_table": digest(canonical(fixture["rows"])),
                    "kernel_api": digest(HERE.joinpath("kernel-stub.md").read_bytes()),
                    "notation": digest(HERE.joinpath("notation.md").read_bytes()),
                    "checkers": [],
                }
                previous = digest(canonical(expanded))
                keys.append({"case_id": case["id"], "run_id": run["id"], "neutral_sha256": previous})
    return {
        "corpus_version": VERSION, "decision": "open",
        "kind": "frozen-neutral-input-keys; not executable DET.1 ReproKeys",
        "recipe": "check_corpus.py make_keys: SHA-256 of compact canonical expanded JSON; ordered case runs link predecessor input keys, not yet-observed output bytes",
        "input_files": {name: digest(HERE.joinpath(name).read_bytes()) for name in INPUT_FILES},
        "source_documents": {name: digest(raw) for name, raw in sources.items()},
        "seeds": [], "randomness": "No random operation or unsealed random input is present.",
        "candidate_bindings_required": [
            "candidate build/source and executed dependency bytes by digest",
            "language edition, validated execution contract and abstract-charge mapping",
            "exact kernel implementation and checker identities",
            "effective grants, flow form, variant and host-verified provenance",
            "each consumed workspace record, parts, solver picks and previous-run output by digest",
            "source map from neutral instruction paths to executed bytes",
        ],
        "candidate_identity": None, "keys": keys,
    }


def freeze_bytes():
    hashes = {name: digest(HERE.joinpath(name).read_bytes()) for name in sorted(FILES - {"FREEZE.json"})}
    manifest = {
        "corpus_version": VERSION, "decision": "open", "algorithm": "SHA-256",
        "content_sha256": digest(canonical(hashes)),
        "change_rule": "Any later file change requires a new corpus version before a new candidate run; retain the prior freeze and evidence.",
        "self_hash_rule": "FREEZE.json entry hashes canonical compact JSON of this manifest with that entry null; all other entries hash raw bytes.",
        "files": hashes | {"FREEZE.json": None},
    }
    manifest["files"]["FREEZE.json"] = digest(canonical(manifest))
    return pretty(manifest)


def negative_checks(data, registry, clauses):
    # In-memory counterexamples: no fixtures, caches, bytecode or build products.
    changes = [
        lambda x: x["must-refuse.json"]["cases"].pop(),
        lambda x: x["must-refuse.json"]["cases"][0].update(expected_code="LG3:capability.flow_refused"),
        lambda x: x["must-refuse.json"]["cases"][0].update(expected_code="LG3:capability.invented"),
        lambda x: x["allowed-tools.json"].update(denominator=999),
        lambda x: x["allowed-tools.json"]["cases"][0]["program"]["runs"][0]["body"][0].update(row="missing"),
        lambda x: x["allowed-tools.json"]["cases"][0]["program"]["runs"][0]["body"][1].update(value={"ref": "unbound"}),
        lambda x: x["profile.json"]["decisions"].update(LQ21="selected"),
        lambda x: x["planted.json"]["cases"][0].update(grants=["clock"]),
        lambda x: x["profile.json"]["display"]["approximate_fields"]["coordinates"]["absolute_tolerance"].update(denominator=0),
        lambda x: x["fixture.json"]["setups"].update(c2=[["c"]]),
    ]
    for index, change in enumerate(changes):
        broken = copy.deepcopy(data)
        change(broken)
        try:
            validate(broken, registry, clauses)
        except ValueError:
            continue
        raise ValueError(f"validator negative check {index + 1} accepted invalid input")
    return len(changes)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepare-keys", action="store_true", help="write initial keys only, before FREEZE exists")
    parser.add_argument("--validate", action="store_true", help="validate inputs and keys without creating FREEZE")
    args = parser.parse_args()
    need(not (args.prepare_keys and args.validate), "choose at most one mode")
    actual = {path.name for path in HERE.iterdir()}
    need(actual <= FILES, "unexpected corpus entries: " + ", ".join(sorted(actual - FILES)))
    required = FILES - {"FREEZE.json"}
    if args.prepare_keys:
        required -= {"keys.json"}
        need("FREEZE.json" not in actual, "cannot prepare keys after freezing; create a new corpus version")
    need(required <= actual, "missing corpus entries: " + ", ".join(sorted(required - actual)))
    for filename in actual:
        path = HERE / filename
        need(path.is_file() and not path.is_symlink(), f"{filename}: expected ordinary file")
        raw = path.read_bytes()
        need(len(raw) < 2_000_000 and b"\r" not in raw and not raw.startswith(b"\xef\xbb\xbf"), f"{filename}: size/BOM/LF")
        text = raw.decode("utf-8")
        need(not any(0xD800 <= ord(c) <= 0xDFFF for c in text), f"{filename}: surrogate")
    sources, registry, clauses = source_contracts()
    data = {name: load_json(name) for name in INPUT_FILES if name.endswith(".json")}
    validate(data, registry, clauses)
    negatives = negative_checks(data, registry, clauses)
    keys = make_keys(data, sources)
    if args.prepare_keys:
        HERE.joinpath("keys.json").write_bytes(pretty(keys))
        print(f"Prepared {len(keys['keys'])} neutral run keys; no candidate exists; not frozen yet.")
        return
    need(load_json("keys.json") == keys, "keys/input/source digests differ; do not relabel an old corpus")
    if args.validate:
        print(f"Validated inputs and {len(keys['keys'])} keys; {negatives} invalid-input checks rejected; no programs executed.")
        return
    expected = freeze_bytes()
    freeze = HERE / "FREEZE.json"
    if freeze.exists():
        need(freeze.read_bytes() == expected, "FREEZE mismatch; any later change needs a new corpus version")
        action = "verified"
    else:
        freeze.write_bytes(expected)
        action = "created"
    need(freeze.read_bytes() == expected, "written FREEZE did not match recomputed manifest")
    print(f"PASS: 10 RTE.45 families; {len(data['must-refuse.json']['cases'])} must-refuse cases; "
          f"{len(data['allowed-tools.json']['cases'])} allowed tools; 4 planted resources; "
          f"{len(data['budgets.json']['cases'])} budget/cache cases; 2 approval cases; "
          f"{len(keys['keys'])} neutral run keys; {negatives} negative schema checks; "
          f"FREEZE {action}, {len(FILES)} files covered (normalized manifest self hash). "
          "Candidate execution: not run.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, IndexError, UnicodeError) as error:
        # Do not print machine paths or traceback diagnostics in the public record.
        if isinstance(error, OSError):
            print(f"FAIL: corpus/source file access failed ({type(error).__name__})", file=sys.stderr)
        else:
            print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
