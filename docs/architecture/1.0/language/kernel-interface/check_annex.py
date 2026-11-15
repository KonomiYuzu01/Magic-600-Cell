"""Check the draft FR-B03 annex; no route, language, syntax or LQ is selected.

Not an accepted/frozen interface or a product conformance test. Read-only standard
library check, including in-memory rejection controls; no refreshed artifacts.
"""

import sys

sys.dont_write_bytecode = True

import copy
import json
import re
from pathlib import Path, PurePosixPath


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
OWN_FILES = ("README.md", "operations.json", "walks.json", "check_annex.py")
CLAUSE = re.compile(r"\b[A-Z][A-Z0-9]*\.\d+\b")
DEFINITION = re.compile(r"^\s*- \*\*([A-Z][A-Z0-9]*\.\d+) \[(?:B|D|O|P LQ\d+(?:,\s*LQ\d+)*)\]\*\*", re.M)
PLACEMENTS = {"engine_isolate", "runtime_process", "shell_process"}
EXACTNESS = {"exact", "exact_partial", "host", "display"}
ROW_USES = {"dispatch", "service", "consumer", "continuation", "host_gate", "batch_component", "library_component"}
OWNER_PATH = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)(?:\.[A-Za-z_][A-Za-z0-9_]*|\[\d+\])*")
# A word a solver's program computed, as the walk's resolved provenance input (parts are resolved blocks here).
TOOL_BLOCK = {"author": "own", "producer": {"kind": "program", "module": "fixture", "entry": "main"},
              "profile": "solving", "parts": []}


def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for item in value.values():
            yield from strings(item)
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)


def definitions_in(text):
    return set(DEFINITION.findall(text))


def tool_computed(block):
    """BND.84, BND.85: the block says a solver's tool computed the word, directly or in a part. Parts here are resolved blocks."""
    producer = block.get("producer", {})
    if producer.get("kind") == "program" and block.get("author") in {"own", "forked_built_in", "imported", "unknown"}:
        return True
    return any(tool_computed(part) for part in block.get("parts", []))


def validate(catalogue, walks, readme, row_ids, services, definitions):
    errors = []

    def need(condition, code, detail):
        if not condition:
            errors.append(f"{code}: {detail}")

    def text_fields(value, fields, where):
        for field in fields:
            need(isinstance(value.get(field), str) and bool(value[field].strip()), "field", f"{where}.{field}")

    def unique(values, where):
        need(len(values) == len(set(values)), "duplicate", where)

    need(catalogue.get("format") == "c600-kernel-interface-annex-draft-1", "format", "catalogue")
    need(walks.get("format") == "c600-kernel-interface-walks-draft-1", "format", "walks")
    for document in (catalogue, walks):
        status = document.get("status", "")
        need("draft" in status and "not accepted or frozen" in status, "status", "draft boundary")

    citations = set(CLAUSE.findall(readme))
    for text in strings([catalogue, walks, services]):
        citations.update(CLAUSE.findall(text))
    for clause in sorted(citations - definitions):
        need(False, "clause", f"undefined {clause}")

    required = catalogue.get("requires", [])
    required_rows = [entry["row"] for entry in required]
    unique(required_rows, "requires rows")
    service_ids = [service["id"] for service in services]
    unique(service_ids, "service IDs")
    service_by_id = {service["id"]: service for service in services}
    need(not row_ids & set(service_ids), "registration", "command/service ID collision")
    for service in services:
        need(set(service) == {"id", "kind", "caller", "lane", "clauses", "note"}, "registration", service["id"])
        need(service.get("kind") in {"service", "continuation"}, "registration", service["id"])
        need(service.get("caller") in {"runtime", "host", "both"}, "registration", service["id"])
    for entry in required:
        text_fields(entry, ("row", "owner", "kind", "why"), "requires")
        need(bool(entry.get("clauses")), "field", f"requires {entry['row']} clauses")
        if entry.get("kind") == "command":
            need(entry["row"] in row_ids, "registration", f"missing command {entry['row']}")
        else:
            service = service_by_id.get(entry["row"])
            need(entry.get("kind") in {"service", "continuation"} and service is not None
                 and service["kind"] == entry["kind"], "registration", f"missing or mismatched service {entry['row']}")
    allowed_rows = row_ids | set(service_ids)

    profiles = catalogue.get("transfer_profiles", {})
    for name, profile in profiles.items():
        need(set(profile) == PLACEMENTS, "transfer", name)
        text_fields(profile, PLACEMENTS, f"transfer {name}")

    operations = catalogue.get("operations", [])
    unique([op["id"] for op in operations], "operation IDs")
    by_id = {op["id"]: op for op in operations}
    used_rows = set()
    for op in operations:
        name = op["id"]
        text_fields(op, ("id", "row_use", "caller", "effect", "lane", "snapshot", "cancellation"), name)
        need(op.get("row_use") in ROW_USES, "row_use", name)
        need(bool(op.get("rows")), "row", f"{name} needs a row dependency")
        for row in op.get("rows", []):
            need(row in allowed_rows, "row", f"{name}: unknown {row}")
            if op.get("row_use") in {"service", "continuation"}:
                need(row in service_by_id and service_by_id[row]["kind"] == op["row_use"], "registration", f"{name}: service kind for {row}")
            else:
                need(row in row_ids, "registration", f"{name}: command dependency {row}")
            used_rows.add(row)
        for group, fields in (("inputs", ("name", "type", "ownership")),
                              ("result_fields", ("name", "type", "exactness", "meaning"))):
            entries = op.get(group, [])
            need(bool(entries), "field", f"{name}.{group}")
            unique([entry["name"] for entry in entries], f"{name}.{group}")
            for entry in entries:
                text_fields(entry, fields, f"{name}.{group}")
                if group == "result_fields":
                    need(entry.get("exactness") in EXACTNESS, "exactness", f"{name}.{entry['name']}")
        if name == "undo_hint":
            need({p["name"]: p["type"] for p in op["inputs"]} ==
                 {"session": "SessionId + EngineEpoch", "direction": "undo | redo"},
                 "walk_inputs", "undo_hint must acquire the LI.9 direction-specific hint")
        need(bool(op.get("buffers")), "buffer", name)
        for buffer in op.get("buffers", []):
            text_fields(buffer, ("name", "owner", "lifetime", "immutable", "release"), f"{name}.buffer")
        transfer = op.get("transfer", {})
        need(transfer.get("profile") in profiles, "transfer", f"{name} profile")
        text_fields(transfer, ("payload",), f"{name}.transfer")
        account = op.get("accounting", {})
        text_fields(account, ("steps", "kernel_cost", "memory", "output"), f"{name}.accounting")
        refs = account.get("clauses", [])
        need(any(c.startswith("DET.") for c in refs) and any(c.startswith("LG4.") for c in refs), "meter", name)
        need(bool(op.get("clauses")), "field", f"{name}.clauses")
    need(set(required_rows) <= used_rows, "row", "unused requires entry")

    observations = catalogue.get("observations", [])
    names = [item["name"] for item in observations]
    unique(names, "observations")
    need(set(names) == {"total_N", "seams", "D", "units", "F", "phi", "prefix"}, "observations", "LB3 packaging incomplete")
    for item in observations:
        text_fields(item, ("name", "operation", "field", "choice", "why"), "observation")
        op = by_id.get(item["operation"])
        need(op is not None, "observation_operation", item["name"])
        if op:
            need(item["field"] in {field["name"] for field in op["result_fields"]}, "observation_field", item["name"])
        need(item["choice"] in {"explicit_batch", "library_aggregation"}, "observations", item["name"])

    holes = catalogue.get("binding_holes", [])
    unique([hole["route"] for hole in holes], "route holes")
    need({hole["route"] for hole in holes} == {"RT1", "RT2", "RT3", "RT4"}, "holes", "all four routes required")
    for hole in holes:
        need(hole.get("status") == "open", "holes", hole["route"])
        text_fields(hole, ("needs",), hole["route"])

    def trace(document):
        name = document["id"]
        need(document.get("completion") == "specified_complete", "walk", f"{name} incomplete")
        text_fields(document, ("given", "postcondition"), name)
        initial = document.get("initial_resources", [])
        unique(initial, f"{name} initial owners")
        live = set(initial)
        ever_owned = set(initial)
        steps = document.get("steps", [])
        need(bool(steps), "walk", f"{name} has no steps")
        unique([step["id"] for step in steps], f"{name} step IDs")
        for step in steps:
            where = f"{name}/{step['id']}"
            need(step.get("status") == "specified" and not step.get("invented"), "walk_status", where)
            need(not any(isinstance(v, str) and v.casefold() == "invented" for v in step.values()), "walk_status", where)
            op = by_id.get(step.get("operation"))
            need(op is not None, "walk_operation", where)
            text_fields(step, ("result", "snapshot", "cancellation"), where)
            need(bool(step.get("clauses")), "field", f"{where}.clauses")
            if op:
                need(set(step.get("inputs", {})) == {p["name"] for p in op["inputs"]}, "walk_inputs", where)
                if op["id"] == "undo_hint":
                    need(step.get("inputs", {}).get("direction") in {"undo", "redo"}, "walk_inputs", where)
                if op["id"] == "propose":
                    # BND.84, BND.85: a word a solver's tool computed never reaches a preview in a solving session.
                    block = step.get("inputs", {}).get("provenance")
                    need(isinstance(block, dict), "provenance", where)
                    tool_word = isinstance(block, dict) and tool_computed(block)
                    need(not tool_word or "LG3:capability.not_granted" in step.get("result", ""), "provenance", where)
            for field in ("uses", "creates", "releases"):
                need(isinstance(step.get(field), list), "resource", f"{where}.{field}")
                unique(step.get(field, []), f"{where}.{field}")
            uses, creates, releases = (set(step.get(field, [])) for field in ("uses", "creates", "releases"))
            if op:
                # KI.4: resolve actual resource inputs before this step creates or
                # releases owners. The separate uses list cannot conceal a borrow.
                for param in op["inputs"]:
                    if not param["type"].startswith(("Owner<", "Borrow<", "StateRef", "PreviewId")):
                        continue
                    value = step.get("inputs", {}).get(param["name"])
                    path = OWNER_PATH.fullmatch(value) if isinstance(value, str) else None
                    owner = path.group(1) if path else None
                    need(owner in live, "resource_use", f"{where}.{param['name']}: unknown/released owner for {value!r}")
                    need(owner in uses, "resource_use", f"{where}.{param['name']}: input owner absent from uses")
            need(uses <= live, "resource_use", f"{where}: unknown/released {sorted(uses - live)}")
            need(releases <= live, "resource_release", f"{where}: not owned {sorted(releases - live)}")
            need(not creates & ever_owned, "resource_create", f"{where}: reused owner identity")
            if step.get("operation") in {"release_value", "close_snapshot"}:
                param = "owner" if step["operation"] == "release_value" else "snapshot"
                target = step.get("inputs", {}).get(param)
                need(releases == {target} and target in uses, "resource_drop", where)
            live -= releases
            live |= creates
            ever_owned |= creates
        need(document.get("final_resources") == [], "resource_final", name)
        need(not live, "resource_leak", f"{name}: {sorted(live)}")
        for branch in document.get("interruptions", []):
            trace(branch)

    main_walks = walks.get("walks", [])
    unique([walk["id"] for walk in main_walks], "walk IDs")
    need({walk["id"] for walk in main_walks} == {"retained_macro", "jumbling_research"}, "walk", "both required walks")
    required_ops = {
        "retained_macro": {"open_snapshot", "word_open", "stream_next", "propose", "read_presented", "commit_preview", "outcome", "undo_hint", "undo", "close_snapshot"},
        "jumbling_research": {"open_snapshot", "seam_facts", "prefix_observations", "stream_next", "retain_value", "release_value", "close_snapshot"},
    }
    for walk in main_walks:
        trace(walk)
        invoked = {step["operation"] for step in walk.get("steps", [])}
        need(required_ops.get(walk["id"], set()) <= invoked, "walk_coverage", walk["id"])
        if walk["id"] == "retained_macro":
            directions = {step.get("inputs", {}).get("direction") for step in walk["steps"] if step["operation"] == "undo_hint"}
            need(directions == {"undo", "redo"}, "walk_coverage", "retained_macro hint directions")
        need(bool(walk.get("interruptions")), "walk_coverage", f"{walk['id']} interruption trace")
        if walk["id"] == "jumbling_research":
            fixture = walk.get("seam_fixture", {})
            need(fixture.get("complete") is True and fixture.get("scope") == "all", "seams", "fixture must cover all cells")
            cells = fixture.get("cells", [])
            unique([(cell["domain"], cell["cell"]) for cell in cells], "seam cell identities")
            for cell in cells:
                need(type(cell.get("mismatched")) is bool and type(cell.get("on_grip")) is bool, "seams", "typed cell flags")
            on = sum(cell["mismatched"] and cell["on_grip"] for cell in cells)
            off = sum(cell["mismatched"] and not cell["on_grip"] for cell in cells)
            need(off > 0, "seams", "counterexample must include off-grip seams")
            need(fixture.get("expected") == {"total_N": on + off, "on_grip_mismatches": on, "off_grip_mismatches": off}, "seams", "all-scope totals disagree")
    return errors


def rejection_controls(catalogue, walks, readme, row_ids, services, definitions):
    # Mutants live only in memory. They must fail for the intended reason.
    def step(walks, walk_id, step_id):
        return next(s for w in walks["walks"] if w["id"] == walk_id for s in w["steps"] if s["id"] == step_id)

    controls = [
        ("row", lambda c, w: c["operations"][0]["rows"].append("absent.row")),
        ("clause", lambda c, w: c["operations"][0]["clauses"].append("KI.999999")),
        ("walk_operation", lambda c, w: w["walks"][0]["steps"][0].update(operation="absent_operation")),
        ("walk_status", lambda c, w: w["walks"][0]["interruptions"][0]["steps"][0].update(status="invented")),
        ("buffer", lambda c, w: c["operations"][0].update(buffers=[])),
        ("field", lambda c, w: c["operations"][0]["accounting"].pop("memory")),
        ("transfer", lambda c, w: c["transfer_profiles"]["kernel"].pop("shell_process")),
        ("observation_field", lambda c, w: c["observations"][0].update(field="missing_field")),
        ("observations", lambda c, w: c["observations"].pop()),
        ("duplicate", lambda c, w: c["requires"].append(c["requires"][0])),
        ("registration", lambda c, w: c["requires"][0].update(kind="command")),
        ("registration", lambda c, w: next(e for e in c["requires"] if e["row"] == "jumble.observe").update(kind="service")),
        ("registration", lambda c, w: c["requires"].append({"row": "absent.service", "owner": "EN4", "kind": "service", "why": "must be registered", "clauses": ["KI.1"]})),
        ("resource_leak", lambda c, w: w["walks"][0]["steps"][-1].update(releases=[])),
        ("resource_use", lambda c, w: w["walks"][0]["steps"][6]["uses"].append("recipe")),
        # MC-06: change only the actual input, leaving the formerly trusted uses.
        ("resource_use", lambda c, w: step(w, "retained_macro", "r7")["inputs"].update(word="letters")),
        ("resource_use", lambda c, w: step(w, "retained_macro", "r7")["inputs"].update(word="absent_owner")),
        ("resource_use", lambda c, w: step(w, "retained_macro", "r7")["uses"].remove("w")),
        ("resource_use", lambda c, w: step(w, "jumbling_research", "j6")["inputs"].update(state="cells")),
        ("resource_use", lambda c, w: step(w, "jumbling_research", "j12")["inputs"].update(value="cells.items[0].prefix_state")),
        ("resource_use", lambda c, w: step(w, "jumbling_research", "j12")["inputs"].update(value="d1.items[0].prefix_state")),
        ("resource_use", lambda c, w: step(w, "jumbling_research", "j12")["uses"].remove("pc")),
        ("walk_inputs", lambda c, w: step(w, "retained_macro", "r12")["inputs"].pop("direction")),
        ("walk_inputs", lambda c, w: step(w, "retained_macro", "r12")["inputs"].update(direction="both")),
        ("walk_coverage", lambda c, w: step(w, "retained_macro", "r13_redo_hint")["inputs"].update(direction="undo")),
        ("seams", lambda c, w: w["walks"][1]["seam_fixture"]["expected"].update(total_N=1)),
        ("holes", lambda c, w: c["binding_holes"].pop()),
        ("walk_inputs", lambda c, w: w["walks"][0]["steps"][0]["inputs"].pop("at")),
        # BND.84, BND.85: the retained walk's manual word passes; a tool block there, direct or in a part, must refuse.
        ("provenance", lambda c, w: step(w, "retained_macro", "r7")["inputs"].update(provenance=copy.deepcopy(TOOL_BLOCK))),
        ("provenance", lambda c, w: step(w, "retained_macro", "r7")["inputs"]["provenance"]["parts"].append(copy.deepcopy(TOOL_BLOCK))),
        ("provenance", lambda c, w: step(w, "retained_macro", "r7")["inputs"].update(provenance="program(own module); solving session")),
    ]
    errors = []
    for code, mutate in controls:
        changed_catalogue, changed_walks = copy.deepcopy((catalogue, walks))
        mutate(changed_catalogue, changed_walks)
        found = validate(changed_catalogue, changed_walks, readme, row_ids, services, definitions)
        if not any(error.startswith(code + ":") for error in found):
            errors.append(f"rejection control failed: {code}")
    registration_controls = [
        (row_ids, services[1:]),
        (row_ids - {"jumble.observe"}, services),
        (row_ids | {services[0]["id"]}, services),
    ]
    for commands, registrations in registration_controls:
        found = validate(catalogue, walks, readme, commands, registrations, definitions)
        if not any(error.startswith("registration:") for error in found):
            errors.append("rejection control failed: missing or colliding registration")
    return len(controls) + len(registration_controls), errors


def main():
    own_text = {}
    for name in OWN_FILES:
        data = (HERE / name).read_bytes()
        if b"\r" in data or not data.endswith(b"\n"):
            raise ValueError(f"{name} must use LF with a final newline")
        own_text[name] = data.decode("utf-8")
    catalogue = json.loads(own_text["operations.json"])
    walks = json.loads(own_text["walks.json"])
    readme = own_text["README.md"]
    definitions = definitions_in(readme)
    sources = catalogue["sources"]
    if len(sources) != len(set(sources)):
        raise ValueError("duplicate source path")
    for source in sources:
        relative = PurePosixPath(source)
        if relative.is_absolute() or ".." in relative.parts or "\\" in source or ":" in source:
            raise ValueError("sources must be repository-relative")
        path = ROOT / source
        if not path.resolve().is_relative_to(ROOT):
            raise ValueError("source escapes repository")
        text = path.read_text(encoding="utf-8")
        if path.suffix == ".md":
            definitions.update(definitions_in(text))
    table = json.loads((ROOT / "docs/progress/1.0/command-table.json").read_text(encoding="utf-8"))
    row_ids = {row["id"] for row in table["commands"]}
    services = table.get("services", [])
    errors = validate(catalogue, walks, readme, row_ids, services, definitions)
    if errors:
        print("FAIL: annex reference/structure check")
        print("\n".join(errors[:30]))
        return 1
    count, errors = rejection_controls(catalogue, walks, readme, row_ids, services, definitions)
    if errors:
        print("FAIL: " + "; ".join(errors))
        return 1
    step_count = sum(len(walk["steps"]) + sum(len(branch["steps"]) for branch in walk["interruptions"]) for walk in walks["walks"])
    print(f"PASS: {len(catalogue['operations'])} operations, {len(catalogue['requires'])} requires, "
          f"2 complete specified walks ({step_count} steps), {count} rejection controls. Draft structure only.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, UnicodeError, ValueError, KeyError, TypeError) as error:
        # Do not print machine paths or diagnostics into a public check report.
        print(f"FAIL: invalid or unavailable annex input ({type(error).__name__})")
        raise SystemExit(1)
