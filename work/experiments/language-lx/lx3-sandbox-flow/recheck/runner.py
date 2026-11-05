"""One strict pass per frozen key, in fresh children with the original worker."""

import sys
sys.dont_write_bytecode = True

import argparse
from concurrent.futures import ThreadPoolExecutor
import copy
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
PART = HERE.parent / "part-b"
sys.path.insert(0, str(PART))

from harness.bindings import candidate
from harness.classify import classify as frozen_classify
from harness.common import canonical, cases, digest, write_json
from harness import matrix
from harness.report import summarize
import strict

CANDIDATES = ("rt1-own", "rt2-py", "rt2-node", "rt3-wasm", "rt4-pyhost",
              "control-open", "control-closed")


def child(request):
    strict.install()
    if request.get("mode") == "control":
        from control import execute
        from harness.meter import Stop
        from harness.runtime import RunContext
        case = next(c for g, c in cases() if c["id"] == "MR07")
        _, bindings = candidate(PART / "candidates/control-open")
        retained = None
        for peer in range(2 if request["control"] == "peer_corruption" else 1):
            host = strict.StrictHost(case["program"], "static_labels", bindings["source_files"])
            records = []
            for run in case["program"]["runs"]:
                ctx = RunContext(case, "must-refuse", run, host, bindings, "static_labels", "source_bound",
                                 records[-1]["sha256"] if records else None)
                try:
                    value = execute(ctx, "workspace_value" if peer else request["control"])
                    record = ctx.finish(value)
                except Stop as stop:
                    if request["control"] == "after_refusal":
                        host.pending = None
                    record = ctx.finish(stop=stop)
                records.append(record)
            if retained is None:
                retained = records
        return {"event": "finished", "records": retained}
    return matrix.worker(request)


def launch(request):
    return matrix.launch([sys.executable, "-B", str(HERE / "runner.py"), "--worker"], request, 30)


def controls(build):
    from control import MODES
    case = next(c for g, c in cases() if c["id"] == "MR07")
    results = {}
    for mode in MODES:
        response = launch({"mode": "control", "control": mode})
        assert "failure" not in response, "synthetic control child failed: " + mode
        record = response["records"][-1]
        baseline = frozen_classify(case, "must-refuse", record)
        checked = strict.classify(case, "must-refuse", record)
        results[mode] = {"frozen": baseline, "strict": checked,
                         "audits": record["host"]["strict_peer_preservation"]}
        expected_frozen = "state_changed" if mode in ("session", "after_refusal") else "refused_expected"
        assert baseline["status"] == expected_frozen, "frozen control disagrees: " + mode
        assert checked["status"] == ("refused_expected" if mode == "legitimate_write" else "state_changed"), mode
    write_json(build / "controls.json", results)
    return results


def normalized_cell(cell):
    result = copy.deepcopy(cell)
    result["hashes"] = {k: sorted({(e["run_key"], e["sha256"]) for e in entries})
                        for k, entries in cell["hashes"].items()}
    for field in ("not_run", "budget_cache_interruption"):
        result[field] = sorted({canonical({k: v for k, v in row.items() if k not in ("repeat", "leg")})
                                for row in cell[field]})
    return result


def baseline_differences(name, rows, declaration):
    path = PART / ("summaries/" + name + ".json" if name.startswith("control-")
                   else "candidates/" + name + "/summary.json")
    stored = json.loads(path.read_bytes())
    fresh = summarize(rows, declaration, 1, stored["legs"])
    differences = []
    for a, b in zip(fresh["cells"], stored["cells"], strict=True):
        left, right = normalized_cell(a), normalized_cell(b)
        for field in left:
            if left[field] != right[field]:
                differences.append({"cell": a["form"] + "/" + a["variant"], "field": field})
    outcome_file = {"rt2-node": "key-outcomes.json", "rt3-wasm": "outcomes.json"}.get(name)
    expected = None
    if outcome_file:
        expected = json.loads((PART / "candidates" / name / outcome_file).read_bytes())
        if name == "rt2-node":
            expected = expected["keys"]
    elif name == "rt4-pyhost":
        observations = json.loads((PART / "candidates" / name / "evidence.json").read_bytes())["observations"]
        expected = {"/".join((o["form"], "approved_subset_byte_bound", o["case_id"], o["run_id"], o["host_variant"])):
                    {k: o[k] for k in ("classification", "run_key", "sha256")} for o in observations}
    for row in rows:
        key = "/".join(row[k] for k in ("form", "variant", "case_id", "run_id", "host_variant"))
        if expected is not None and key in expected and projection(row) != expected[key]:
            differences.append({"key": key, "field": "stored_per_key"})
    return differences


def projection(row):
    record = row["record"]
    return {"classification": row["classification"],
            "run_key": record["exact"]["input_identities"]["run_key"] if record else None,
            "sha256": record["sha256"] if record else None}


def frozen_pass(build):
    results = {}
    for name in CANDIDATES:
        declaration, _ = candidate(PART / "candidates" / name)
        jobs = []
        for cell in declaration["cells"]:
            for group, case in cases():
                for host_variant in ([v["id"] for v in case.get("host", {}).get("variants", [])] or ["default"]):
                    request = {"candidate": "candidates/" + name, "case_id": case["id"],
                               "form": cell["form"], "variant": cell["variant"], "host_variant": host_variant}
                    jobs.append((request, group, case))
        rows, outcomes = [], {}
        with ThreadPoolExecutor(max_workers=8) as pool:
            responses = pool.map(launch, (job[0] for job in jobs))
            for (request, group, case), response in zip(jobs, responses, strict=True):
                if "failure" not in response:
                    assert len(response["records"]) == len(case["program"]["runs"]), "missing run"
                for i, run in enumerate(case["program"]["runs"]):
                    record = None if "failure" in response else response["records"][i]
                    options = {"failure": response.get("failure"), "host_variant": request["host_variant"],
                               "terminal": i == len(case["program"]["runs"]) - 1 or run["profile"] == "solving"}
                    baseline = frozen_classify(case, group, record, **options)
                    checked = strict.classify(case, group, record, **options)
                    row = {**request, "run_id": run["id"], "repeat": 0,
                           "leg": "control" if name.startswith("control-") else "Windows",
                           "classification": baseline, "record": record}
                    rows.append(row)
                    key = "/".join(row[k] for k in ("form", "variant", "case_id", "run_id", "host_variant"))
                    if record is not None:
                        peers = record["host"]["strict_peer_preservation"]
                        assert len(peers) == (2 if request["host_variant"] == "shared" else 1), "missing peer audit"
                    outcomes[key] = {**projection(row), "strict": checked,
                        "violations": strict.violations(record) if record else None}
        differences = baseline_differences(name, rows, declaration)
        cells = []
        for cell in declaration["cells"]:
            prefix = cell["form"] + "/" + cell["variant"] + "/"
            selected = {k: v for k, v in outcomes.items() if k.startswith(prefix)}
            cells.append({**cell, "keys": len(selected),
                          "strict_failures": [k for k, v in selected.items() if v["violations"]],
                          "not_run": sum(v["classification"]["status"] == "not_run" for v in selected.values()),
                          "unobserved": [k for k, v in selected.items() if v["violations"] is None]})
        results[name] = {"cells": cells, "baseline_differences": differences, "outcomes": outcomes}
        write_json(build / (name + ".json"), results[name])
        print(name + ": " + str(len(outcomes)) + " keys; " + str(len(differences)) + " baseline differences; "
              + str(sum(len(c["strict_failures"]) for c in cells)) + " strict failures", flush=True)
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", action="store_true")
    args = parser.parse_args()
    if args.worker:
        try:
            response = child(json.loads(sys.stdin.buffer.readline()))
        except Exception as error:
            response = {"event": "crash", "exception": type(error).__name__}
        sys.stdout.buffer.write(matrix.wire(response))
        return 0
    parser.error("use check.py or prepare.py")


if __name__ == "__main__":
    raise SystemExit(main())
