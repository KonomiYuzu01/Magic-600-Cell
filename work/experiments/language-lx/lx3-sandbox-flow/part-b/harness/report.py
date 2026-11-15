"""Complete per-form reports, with no false-refusal threshold or route decision."""

import argparse
import json
from collections import defaultdict

from .bindings import within_root
from .common import ROOT, canonical, cases, write_json
from .result import compare, require

FAILURES = {"unrefused", "wrong_code", "planted_read", "state_changed", "crash", "hang", "wrong_value"}


def expected_rows(manifest, runs, legs):
    return {(cell["form"], cell["variant"], case["id"], invocation["id"], host_variant, repeat, leg)
            for cell in manifest["cells"] for _, case in cases()
            for invocation in case["program"]["runs"]
            for host_variant in ([v["id"] for v in case.get("host", {}).get("variants", [])] or ["default"])
            for repeat in range(runs) for leg in legs}


def row_identity(row):
    return tuple(row[k] for k in ("form", "variant", "case_id", "run_id", "host_variant", "repeat", "leg"))


def summarize(rows, manifest, runs, legs):
    identities = [row_identity(row) for row in rows]
    require(len(set(identities)) == len(identities), "duplicate matrix observation")
    require(set(identities) == expected_rows(manifest, runs, legs), "omitted matrix key; harness failure")
    corpus = {case["id"]: (group, case) for group, case in cases()}
    cells = []
    for declared in manifest["cells"]:
        selected = [row for row in rows if row["form"] == declared["form"] and row["variant"] == declared["variant"]]
        refusal_set, false_set = set(), set()
        outcomes, hashes, budget_outcomes, not_runs = defaultdict(list), defaultdict(list), [], []
        comparisons = defaultdict(list)
        for row in selected:
            classification = row["classification"]
            status, code = classification["status"], classification["code"]
            key = row["case_id"] + "/" + row["run_id"] + "/" + row["host_variant"]
            outcomes[row["case_id"]].append(status)
            if status == "false_refusal":
                false_set.add((row["case_id"], code))
            if row["record"] is not None:
                record = row["record"]
                if record["exact"]["outcome"] == "refused":
                    refusal_set.add((row["case_id"], row["run_id"], code, record["exact"]["context"]["layer"]))
                hashes[key].append({"leg": row["leg"], "repeat": row["repeat"], "sha256": record["sha256"],
                                    "run_key": record["exact"]["input_identities"]["run_key"]})
                if status != "not_run":
                    label = row["leg"] + "/" + str(row["repeat"]) + "/" + row["host_variant"]
                    comparisons[(row["case_id"], row["run_id"])].append({**record, "repeat": label})
            if status == "not_run":
                require(bool(classification.get("reason")), "not_run without reason")
                not_runs.append({"key": key, "reason": classification["reason"], "leg": row["leg"], "repeat": row["repeat"]})
            if corpus[row["case_id"]][0] == "budgets":
                budget_outcomes.append({"key": key, "leg": row["leg"], "repeat": row["repeat"], **classification})
        counterexamples = []
        for (case_id, run_id), records in comparisons.items():
            group, case = corpus[case_id]
            reference_display = case["expected"]["display"] if group == "allowed-tools" else None
            # Legitimate refusals have no display; allowed-value checking already
            # counts them separately. Compare displays only for completed runs.
            completed = [record for record in records if record["exact"]["outcome"] == "completed"]
            reproducible = case.get("host", {}).get("event") not in ("cancel", "deadline", "runtime_loss")
            keys = {record["exact"]["input_identities"]["run_key"] for record in records}
            if len(keys) != 1:
                counterexamples.append({"case_id": case_id, "run_id": run_id, "kind": "binding_changed", "keys": sorted(keys)})
                continue
            for failure in compare(records, None, reproducible):
                if failure["kind"] != "display_reference":
                    counterexamples.append({"case_id": case_id, "run_id": run_id, **failure})
            if reference_display is not None:
                for failure in compare(completed, reference_display, reproducible):
                    if failure["kind"] == "display_reference":
                        counterexamples.append({"case_id": case_id, "run_id": run_id, **failure})
        by_group = {group: {case_id: sorted(set(statuses)) for case_id, statuses in sorted(outcomes.items())
                            if corpus[case_id][0] == group} for group in ("must-refuse", "allowed-tools", "planted", "approval")}
        cells.append({"form": declared["form"], "variant": declared["variant"],
                      "declaration": declared["status"], "reason": declared["reason"],
                      "allowed_denominator": 35,
                      "false_refusal_count": len({case_id for case_id, _ in false_set}),
                      "false_refusals": [{"case_id": case_id, "code": code} for case_id, code in sorted(false_set)],
                      "refusals": [{"case_id": case_id, "run_id": run_id, "code": code, "layer": layer}
                                   for case_id, run_id, code, layer in sorted(refusal_set)],
                      "must_refuse_failures": sum(bool(set(statuses) & FAILURES) for statuses in by_group["must-refuse"].values()),
                      "outcomes": by_group, "budget_cache_interruption": budget_outcomes,
                      "not_run": not_runs, "reproducibility_counterexamples": counterexamples,
                      "hashes": dict(sorted(hashes.items()))})
    return {"candidate": manifest["id"], "decision": "open", "decider": "owner",
            "row_count": len(rows), "runs_per_key_per_leg": runs, "legs": legs,
            "twenty_run_series": runs == 10 and set(legs) == {"Windows", "headless_Linux"},
            "cells": cells}


def main(argv=None):
    parser = argparse.ArgumentParser(description="Combine retained matrix folders under part-b.")
    parser.add_argument("--inputs", nargs="+", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    rows, legs, meta = [], [], None
    for name in args.inputs:
        folder = within_root(ROOT / name)
        run = json.loads((folder / "run.json").read_bytes())
        if meta is not None:
            require(canonical(run["candidate"]) == canonical(meta["candidate"]), "different candidates")
            require(run["bindings_sha256"] == meta["bindings_sha256"] and run["runs"] == meta["runs"], "different binding or repetition count")
        meta = run
        legs.extend(run["legs"])
        rows.extend(json.loads(line) for line in (folder / "results.jsonl").read_bytes().splitlines())
    require(len(legs) == len(set(legs)), "duplicate leg")
    output = within_root(ROOT / args.out)
    require(not output.exists(), "report output already exists")
    write_json(output, summarize(rows, meta["candidate"], meta["runs"], legs))
    return 0


def self_test():
    from .result import HarnessFailure
    manifest = {"cells": [{"form": "static_labels", "variant": "source_bound"}]}
    assert len(expected_rows(manifest, 1, ["control"])) == 87
    try:
        summarize([], manifest, 1, ["control"])
    except HarnessFailure:
        pass
    else:
        raise AssertionError("omitted corpus passed")


if __name__ == "__main__":
    raise SystemExit(main())
