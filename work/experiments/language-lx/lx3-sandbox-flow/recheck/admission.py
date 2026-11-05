"""Optional RT3 admission-preflight variant, generated only inside build/."""

import sys
sys.dont_write_bytecode = True

import importlib.util
import json
from pathlib import Path
import shutil
import subprocess

HERE = Path(__file__).resolve().parent
PART = HERE.parent / "part-b"

PREFLIGHT = '''
def admission_preflight(ctx, surface):
    if ctx.form != "capability_separation" or ctx.run["profile"] != "solving":
        return
    from harness.policy import STATE_ROWS
    seen, effects = set(), set()
    def visit(body):
        for node in body:
            effects.add(node[4] if node[0] == "kernel" else node[0])
            if node[0] == "call" and node[4] not in seen:
                seen.add(node[4])
                visit(surface["modules"][node[4]][2])
            elif node[0] == "if":
                visit(node[3])
                visit(node[4])
            elif node[0] == "for":
                visit(node[5])
    visit(surface["entries"][ctx.run["id"]][4])
    # The compiled, byte-checked closure already selected the grant subset.
    # Ask the unchanged policy about those effects before entering Wasm.
    for operation in sorted(effects & (set(STATE_ROWS) | {"preview"})):
        ctx.check(operation, labels=(), read_state=False)

'''


def worker():
    stage = HERE / "build/admission"
    root, folder = stage / "b", stage / "b/candidates/rt3-wasm"
    shutil.copytree(PART / "harness", root / "harness")
    shutil.copytree(PART.parent / "corpus", stage / "corpus")
    shutil.copytree(PART / "candidates/rt3-wasm", folder)
    path = folder / "adapter.py"
    source = path.read_bytes()
    marker = b"    return Bridge(ctx, surface, compiled).run()"
    assert source.count(marker) == 1, "variant splice no longer identifies the checked boundary"
    patched = source.replace(marker, b"    admission_preflight(ctx, surface)\n" + marker)
    patched += PREFLIGHT.encode("utf-8")
    path.write_bytes(patched)
    sys.path.insert(0, str(root))
    sys.path.insert(0, str(folder))
    from harness.common import cases, digest, sha
    from harness.meter import Stop
    from harness.runtime import RunContext
    from strict import StrictHost
    bindings = json.loads((folder / "bindings.json").read_bytes())
    bindings["source_files"]["candidates/rt3-wasm/adapter.py"] = sha(patched)
    spec = importlib.util.spec_from_file_location("admission_variant", path)
    adapter = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(adapter)
    observations = {}
    for group, case in cases():
        if case["id"] not in ("MR14", "MR20"):
            continue
        host = StrictHost(case["program"], "capability_separation", bindings["source_files"])
        for run in case["program"]["runs"]:
            ctx = RunContext(case, group, run, host, bindings, "capability_separation", "source_bound")
            code, outcome = None, "completed"
            try:
                adapter.execute(ctx)
            except Stop as stop:
                code, outcome = stop.code, stop.outcome
            host.inspect("finish")
            observations[case["id"] + "/" + run["id"]] = {
                "outcome": outcome, "code": code, "expected_code": case["expected_code"],
                "matches_expectation": outcome == "refused" and code == case["expected_code"] and not host.shell,
                "executed_paths": [e["path"] for e in ctx.executions],
                "preview_count": len(host.shell), "strict_violations": host.audit["violations"],
                "denials": [{k: d[k] for k in ("operation", "code", "layer", "path")} for d in host.denials]}
    return {"name": "rt3-wasm+admission-preflight", "kind": "probe", "decision": "open",
            "adapter_before_sha256": sha(source), "adapter_after_sha256": sha(patched),
            "bindings_sha256": digest(bindings), "observations": observations}


def collect(build):
    process = subprocess.run([sys.executable, "-B", str(HERE / "admission.py"), "--worker"],
                             capture_output=True, timeout=60, cwd=HERE)
    assert process.returncode == 0, "admission variant could not run"
    value = json.loads(process.stdout)
    (build / "admission.json").write_bytes(process.stdout)
    return value


if __name__ == "__main__":
    assert sys.argv[1:] == ["--worker"]
    print(json.dumps(worker(), sort_keys=True, separators=(",", ":")))
