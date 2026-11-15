"""Check a reconciliation packet against reconciliation-findings.json. Writes nothing.

Usage:
  python docs/architecture/1.0/language/closure/check_p7.py --packet p7-table
  python docs/architecture/1.0/language/closure/check_p7.py --all

The check is mechanical. It shows that each owned finding cites anchored clause or requirement definitions and a
concrete vector, and that the forms of HANDOFF H15 are in the owned files. It cannot show that the text is right:
that is the review's job.
"""
import sys

sys.dont_write_bytecode = True

import argparse
import importlib.util
import json
import pathlib
import re

HERE = pathlib.Path(__file__).resolve().parent


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


closure = load("check_closure", HERE / "check_closure.py")
ROOT, LANGUAGE, PROGRESS = closure.ROOT, closure.LANGUAGE, closure.PROGRESS
CHAPTERS = LANGUAGE / "architecture"
fail = closure.fail
DRAFT_JI = re.compile(r"\bJI:(?:input|unverified_reference)\.[a-z_]+")
SERVICES = {"kernel.snapshot.open", "kernel.snapshot.close", "kernel.stream.next", "kernel.value.retain", "kernel.value.release",
            "kernel.outcome", "kernel.job_result", "kernel.undo_hint", "language.publish_view"}
NEW_CODES = {
    "EV1:input.module_not_checked", "EV1:input.source_range_invalid", "EV1:stale.activation_base", "EV2:input.word_range_invalid",
    "EV2:stale.debug_stop", "KN1:input.unsupported_dimension", "KN2:input.exposed_geometry_unavailable", "KN2:input.named_object_invalid",
    "KN2:stale.value_unavailable", "KN4:input.branch_path_unproved", "KN5:input.confirmation_required", "KN5:input.exit_record_voided",
    "KN5:input.overlay_dependency_missing", "KN5:input.scramble_policy_unset", "KN5:resource_limit.engine_lost", "KN5:stale.filter_preview",
    "KN5:stale.result_gone", "LB1:input.key_contract_mismatch", "LB6:evidence.decomposition_mismatch", "LG4:input.generator_input",
    "LG4:input.pointwise_failed", "LG4:input.text_scalar_invalid", "LG4:resource_limit.generator_exhausted"}
NEW_REQUIREMENTS = ([f"CT-JI-{n:02d}" for n in range(1, 12)] + [f"CT-LI-{n:02d}" for n in range(1, 15)] + [f"CT-FP-{n:02d}" for n in range(1, 10)]
                    + [f"CT-DET-{n:02d}" for n in range(25, 33)] + ["CT-LB5-10", "CT-LB6-05", "CT-EV1-08", "CT-EV2-11", "CT-EVD-24", "CT-EVD-25"]
                    + [f"CT-RM-{n:02d}" for n in range(1, 8)])


def read(path):
    return path.read_bytes().decode("utf-8")


def must_hold(where, path, words):
    text = read(path)
    for word in words:
        if word not in text:
            fail(f"{where}: {path.name} lacks {word!r}")


def must_lack(where, path, words):
    text = read(path)
    for word in words:
        if word in text:
            fail(f"{where}: {path.name} still has {word!r}")


def check_table():
    builder = load("build_command_table", PROGRESS / "build_command_table.py")
    stored = (PROGRESS / "command-table.json").read_bytes()
    try:
        built = builder.encoded(builder.build_table(delta=builder.default_deltas()))
    except Exception as error:
        fail(f"p7-table: the table does not build with the delta: {error}")
        return
    if built != stored:
        fail("p7-table: command-table.json differs from a fresh build")
    table = json.loads(stored)
    rows = {row["id"]: row for row in table["commands"]}
    if table.get("final") is not True:
        fail("p7-table: the table is not final")
    for name in ("workspace.undo", "workspace.redo", "jumble.preview.scrub"):
        if name in rows:
            fail(f"p7-table: {name} is still a row")
    for name in ("session.undo", "session.redo"):
        row = rows.get(name, {})
        if row.get("arguments") != {"hint": "ActionHint", "token": "UserActionToken"}:
            fail(f"p7-table: {name} arguments: {row.get('arguments')!r}")
        if row.get("action_guard") != "action_hint" or row.get("slot_map") != "by_target":
            fail(f"p7-table: {name} guard or slot map")
    dispatch = table.get("undo_dispatch", {})
    if set(dispatch.get("classes", [])) != {"journal", "workspace"} or dispatch.get("entry") != {"undo": "session.undo", "redo": "session.redo"}:
        fail(f"p7-table: undo_dispatch: {dispatch!r}")
    if "protection" in rows["operation.preview"]["gates"] or "protection" not in rows["operation.execute"]["gates"]:
        fail("p7-table: operation.preview or operation.execute gates")
    applied = rows["filter.apply"]
    if "fresh_filter_preview" not in applied["gates"] or "fresh_preview" in applied["gates"] or applied["preview"] != "filter":
        fail("p7-table: filter.apply gate or preview")
    for row in rows.values():
        if row["lane"] in ("view", "latest_wins"):
            target = row.get("lane_target")
            if not isinstance(target, dict) or len(target) != 1 or next(iter(target)) not in ("arg", "view", "control"):
                fail(f"p7-table: {row['id']}: lane_target {target!r}")
            elif "arg" in target and target["arg"] not in row.get("arguments", {}):
                fail(f"p7-table: {row['id']}: lane_target names no argument")
    services = {item.get("id") for item in table.get("services", [])}
    if services != SERVICES:
        fail(f"p7-table: services differ: {sorted(services ^ SERVICES)}")
    if services & set(rows):
        fail("p7-table: a service ID is also a command")
    if "jumble.observe" not in rows:
        fail("p7-table: jumble.observe is missing")
    must_lack("p7-table", PROGRESS / "command-table.md", ["workspace.undo", "workspace.redo", "jumble.preview.scrub"])


def check_layers():
    ji, li = PROGRESS / "jumbling-interfaces.md", PROGRESS / "layer-interfaces.md"
    for path in (ji, li, CHAPTERS / "01-kernel.md", CHAPTERS / "08-one-state.md"):
        if DRAFT_JI.search(read(path)):
            fail(f"p7-layers: {path.name} still has a draft JI code")
        must_lack("p7-layers", path, ["workspace.undo", "workspace.redo", "jumble.preview.scrub"])
    must_hold("p7-layers", ji, ["exit_root", "solved_witness", "KN4:input.branch_path_unproved", "KN5:input.exit_record_voided",
                                "KN2:input.exposed_geometry_unavailable", "view.preview.scrub"])
    must_hold("p7-layers", li, ["ActionHint", "fresh_filter_preview", "lane_target", "KN5:stale.filter_preview"])


def check_data():
    readme = LANGUAGE / "format-packet" / "README.md"
    must_hold("p7-data", readme, ["human_review", "formal_proof", "context_before", "exit_root", "solved_witness", "admission", "workspace"])
    must_lack("p7-data", readme, ["to be pinned"])


def check_lang():
    environment = CHAPTERS / "04-environment.md"
    must_hold("p7-lang", environment, ["LG2:type.exactness_violation", "EV3:stale.display_pick"])
    for line in read(environment).splitlines():
        if re.match(r"\|\s*`?CT-EV(1-08|2-11)\b", line) and "LP4" not in line:
            fail("p7-lang: CT-EV1-08 or CT-EV2-11 is not in LP4")


def check_render():
    matrix = LANGUAGE / "renderer-feature-matrix.md"
    text = read(matrix)
    if DRAFT_JI.search(text):
        fail("p7-render: the matrix still has a draft JI code")
    for number in range(1, 11):
        if f"J{number:02d}" not in text:
            fail(f"p7-render: J{number:02d} has no row")
    must_hold("p7-render", matrix, ["EV3:stale.display_pick", "EV3:busy.renderer_unavailable"])


def check_registry():
    errors = read(CHAPTERS / "13-errors.md")
    for code in sorted(NEW_CODES):
        if f"`{code}`" not in errors:
            fail(f"p7-reg: {code} is not registered")
    if DRAFT_JI.search(errors):
        fail("p7-reg: chapter 13 has a draft JI code")
    phases = read(CHAPTERS / "12-phases-and-conformance.md")
    for requirement in NEW_REQUIREMENTS:
        if requirement not in phases:
            fail(f"p7-reg: {requirement} is not in chapter 12")


STRUCTURE = {"p7-table": check_table, "p7-layers": check_layers, "p7-data": check_data, "p7-lang": check_lang,
             "p7-render": check_render, "p7-reg": check_registry}


def main():
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--packet")
    group.add_argument("--all", action="store_true")
    args = parser.parse_args()
    data = json.loads((HERE / "reconciliation-findings.json").read_bytes())
    if args.packet and args.packet not in data["packets"]:
        sys.exit(f"unknown packet {args.packet}")
    ids = closure.defined_ids()  # Shared definition scan; citations cannot establish existence.
    for name in (sorted(data["packets"]) if args.all else [args.packet]):
        closure.check_packet(name, data, ids)
        STRUCTURE[name]()
    for line in closure.duplicate_definitions():
        fail(f"duplicate clause definition: {line}")
    if closure.errors:
        print("\n".join(closure.errors))
        sys.exit(f"FAIL: {len(closure.errors)} problems")
    print("reconciliation check: PASS (mechanical only; it cannot show the text is right)")


if __name__ == "__main__":
    main()
