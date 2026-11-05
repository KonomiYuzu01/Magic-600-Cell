"""Render new inputs and run unchanged adapters in disposable copied trees."""

import sys
sys.dont_write_bytecode = True

from concurrent.futures import ThreadPoolExecutor
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess

HERE = Path(__file__).resolve().parent
PART = HERE.parent / "part-b"
IDS = ("rt1-own", "rt2-py", "rt2-node", "rt3-wasm", "rt4-pyhost")


def load(path):
    return json.loads(path.read_bytes())


def prepare_copy(name, probe, program):
    # CreateProcess's cwd limit is shorter than Python's long-path file support.
    index = IDS.index(name) * 4 + sorted(load(HERE / "probes.json")).index(probe)
    stage = HERE / "build" / ("p" + str(index))
    root = stage / "b"
    folder = root / "candidates" / name
    shutil.copytree(PART / "harness", root / "harness")
    shutil.copyfile(PART / "run_matrix.py", root / "run_matrix.py")
    shutil.copytree(PART.parent / "corpus", stage / "corpus")
    shutil.copytree(PART / "candidates" / name, folder)
    sys.path.insert(0, str(root))
    sys.path.insert(0, str(folder))
    from harness.common import canonical, digest, load as corpus, sha
    import render

    case_id = "recheck-" + probe
    key = digest(program)
    bindings = load(folder / "bindings.json")
    prefix = "candidates/" + name + "/"
    if name == "rt3-wasm":
        from frontend import compile_program
        surface = render.render(program)
        data, all_spans = render.bundle({key: surface})
        source_name = "programs.rt3"
        spans = {p: {"start": a, "end": b, "sha256": sha(data[a:b])}
                 for p, (a, b) in all_spans[key].items()}
        modules = {form: {r["id"]: compile_program(surface, r["id"], form, corpus("policy.json")["transparent"])
                          for r in program["runs"]}
                   for form in corpus("policy.json")["forms"]}
        artifacts = {source_name: data, "modules.json": canonical({key: modules}) + b"\n"}
    else:
        data, spans = render.render(program)
        if name == "rt1-own":
            source_name = "programs.rt1"
            spans = {p: {"start": a, "end": b, "sha256": sha(data[a:b])} for p, (a, b) in spans.items()}
            index = {case_id: {"file": prefix + source_name, "start": 0, "end": len(data),
                               "sha256": sha(data), "neutral_sha256": key}}
            index_name = "programs.json"
        elif name == "rt2-py":
            source_name, index_name = "recheck_program.py", "programs.json"
            index = {key: prefix + source_name}
        else:
            source_name = "programs.js" if name == "rt2-node" else "programs.py"
            index_name = "program-index.json"
            index = {case_id: {"start": 0, "end": len(data), "sha256": sha(data)}}
        artifacts = {source_name: data, index_name: canonical(index) + b"\n"}
    for span in spans.values():
        span["sha256"] = sha(data[span["start"]:span["end"]])
    for filename, raw in artifacts.items():
        (folder / filename).write_bytes(raw)
        bindings["source_files"][prefix + filename] = sha(raw)
    bindings["source_map"] = {case_id: {p: {"file": prefix + source_name, **span} for p, span in spans.items()}}
    bindings["rendered_file"] = prefix + source_name
    for relative, expected in bindings["source_files"].items():
        assert sha((root / relative).read_bytes()) == expected, "copied probe source pin mismatch: " + relative
    (folder / "bindings.json").write_bytes(canonical(bindings) + b"\n")
    # These are new probe approvals, never refreshed approvals for corpus keys.
    spec = importlib.util.spec_from_file_location("probe_adapter", folder / "adapter.py")
    adapter = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(adapter)
    return adapter, bindings, case_id, {"program_sha256": key, "bindings_sha256": digest(bindings),
        "artifacts": {prefix + filename: sha(raw) for filename, raw in artifacts.items()}}


def worker(name, probe):
    program = load(HERE / "probes.json")[probe]
    adapter, bindings, case_id, pins = prepare_copy(name, probe, program)
    from harness.common import digest, thaw
    from harness.meter import Stop
    from harness.runtime import RunContext
    from strict import StrictHost
    declaration = load(PART / "candidates" / name / "candidate.json")
    results = {}
    for cell in declaration["cells"]:
        cell_key = cell["form"] + "/" + cell["variant"] + "/" + probe
        if cell["status"] == "not_run":
            results[cell_key] = {"status": "not_expressible", "reason": cell["reason"]}
            continue
        host = StrictHost(program, cell["form"], bindings["source_files"])
        observations, installed = [], []
        for run in program["runs"]:
            case = {"id": case_id, "program": program}
            ctx = RunContext(case, "recheck", run, host, bindings, cell["form"], cell["variant"])
            before = host.snapshot()
            stop = None
            try:
                adapter.execute(ctx)
            except Stop as caught:
                stop = caught
            # No fabricated corpus key or call to the corpus-specific finish().
            host.inspect("finish")
            shell = thaw(host.shell)
            installed.extend(event for event in shell if event["destination"] == "solving_preview" and event["outcome"]["code"] is None)
            saved = host.workspace.get("saved")
            observations.append({"run": run["id"], "outcome": "completed" if stop is None else stop.outcome,
                "code": stop.code if stop else None, "stop": stop.stop if stop else None,
                "saved": None if saved is None else {"value": thaw(saved.data), "labels": sorted(saved.labels),
                    "read_state": saved.read_state, "provenance_sha256": digest(saved.block)},
                "loaded_blocks": [{"labels": thaw(b["labels"]), "read_state": b["read_state"],
                                   "provenance_sha256": digest(b)} for b in host.blocks.values() if b["entry"] == "r2/body/0"],
                "session_unchanged": host.snapshot()["session"] == before["session"],
                "pending_before_sha256": digest(before["pending_preview"]),
                "pending_after_sha256": digest(host.snapshot()["pending_preview"]),
                "workspace_writes": [e["key"] for e in host.events if e["kind"] == "workspace_put"],
                "executed_paths": [e["path"] for e in ctx.executions],
                "approval_ok": all(e["inside_approval"] for e in ctx.executions),
                "denials": [{k: d[k] for k in ("operation", "code", "layer", "path")} for d in host.denials],
                "previews": [{k: e[k] for k in ("word", "target", "destination", "outcome")} for e in shell],
                "strict_violations": host.audit["violations"]})
        code = observations[-1]["code"]
        if installed:
            status = "installed_preview"
        elif observations[-1]["outcome"] == "refused":
            status = "refused_before_preview"
        else:
            raise AssertionError("expressible probe neither refused nor installed: " + name + "/" + cell_key)
        expected = "installed_preview" if probe == "executed_clean" else "refused_before_preview"
        matches = status == expected and code == (None if probe == "executed_clean" else "LG3:capability.not_granted")
        results[cell_key] = {"status": status, "code": code, "expected": expected,
                            "matches_expectation": matches, "pins": pins, "runs": observations}
    return results


def collect(build):
    from probe_inputs import programs
    from harness.common import write_json
    frozen = load(HERE / "probes.json")
    assert programs() == frozen, "probe inputs differ from generator"
    jobs = [(name, probe) for name in IDS for probe in frozen]

    def run(job):
        name, probe = job
        process = subprocess.run([sys.executable, "-B", str(HERE / "probes.py"), "--worker", name, probe],
                                 capture_output=True, timeout=60, cwd=HERE)
        if process.returncode:
            (build / (name + "-" + probe + ".stderr")).write_bytes(process.stderr)
        assert process.returncode == 0, "probe child failed: " + name + "/" + probe + " (see build diagnostics)"
        return json.loads(process.stdout)

    results = {name: {} for name in IDS}
    with ThreadPoolExecutor(max_workers=8) as pool:
        for (name, probe), result in zip(jobs, pool.map(run, jobs), strict=True):
            results[name].update(result)
    for name, observations in results.items():
        cells = load(PART / "candidates" / name / "candidate.json")["cells"]
        assert set(observations) == {c["form"] + "/" + c["variant"] + "/" + p for c in cells for p in frozen}, "missing probe cell"
        assert all(len(v["runs"]) == 2 and all(r["approval_ok"] for r in v["runs"])
                   for v in observations.values() if v["status"] != "not_expressible"), "probe approval or run missing"
    write_json(build / "taint.json", results)
    return results


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--worker":
        print(json.dumps(worker(sys.argv[2], sys.argv[3]), sort_keys=True, separators=(",", ":")))
    else:
        raise SystemExit("use prepare.py or check.py")
