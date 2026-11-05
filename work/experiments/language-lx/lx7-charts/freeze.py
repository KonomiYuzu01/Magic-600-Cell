"""Explicit fixture/candidate freeze; acceptance never updates its own expected bytes."""
import sys
sys.dont_write_bytecode = True
from hashlib import sha256
import json
from pathlib import Path
import shutil
from generate_hlsl import generate
from make_fixtures import make, ROOT
from reference import HERE, cases, cpu, expectations, json_bytes, programs, record, summarize, input_bytes

SOURCES = (
    "CARD.md", "W3-PLAN.md", "subset.md", "subset.py", "generate_hlsl.py", "builtin.hlsl", "generated.hlsl",
    "reference.py", "make_fixtures.py", "freeze.py", "check.py", "device/main.cpp", "device/build.cmd",
    "device/run.py", "device/compare_device.py", "device/README.md",
    "charts/perspective.chart", "charts/stereographic.chart", "charts/gnomonic.chart", "charts/bounded.chart",
    "charts/rejected/recursion.chart", "charts/rejected/unbounded.chart", "charts/rejected/float64.chart",
    "charts/rejected/impure.chart")
INPUTS = ("fixtures.json", "expectations.json")
SYNTHETIC = ("cpu-records.json", "cpu-summary.json")
UPSTREAM = ("assets/mesh.json", "assets/mesh_vertices.f32", "research/jumbling/render-contract.md",
            "work/experiments/renderer-sb/probe/shaders/geometry.hlsl",
            "docs/architecture/1.0/language/architecture/10-renderer-platform.md",
            "docs/architecture/1.0/language/architecture/11-route-and-spikes.md",
            "docs/architecture/1.0/language/adr/ADR-L11-display-plans.md",
            "docs/wiki/decisions/owner-decisions-2026-11-05.md")


def inventory(root, names):
    return {name: sha256((root / name).read_bytes()).hexdigest() for name in sorted(names)}


def aggregate(files):
    return sha256("".join(name + "\0" + digest + "\n" for name, digest in sorted(files.items())).encode("utf-8")).hexdigest()


def verify_manifest():
    manifest = json.loads((HERE / "manifest.json").read_bytes())
    if manifest["format"] != "lx7-manifest-v1" or manifest["candidate"] != "lx7-r1":
        raise ValueError("manifest identity mismatch")
    for group, root, names in (("candidate", HERE, SOURCES), ("inputs", HERE, INPUTS),
                               ("synthetic", HERE, SYNTHETIC), ("upstream", ROOT, UPSTREAM)):
        expected = manifest[group + "_files"]
        if expected != inventory(root, names) or manifest[group + "_sha256"] != aggregate(expected):
            raise ValueError(group + " digest mismatch")
    if manifest["device_input_sha256"] != sha256(input_bytes(json.loads((HERE / "fixtures.json").read_bytes()))).hexdigest():
        raise ValueError("device input digest mismatch")
    return manifest


def clean_build():
    # Never enumerate targets in one shell and delete in another. Only this owned path.
    build = HERE / "build"
    if build.exists():
        if build.resolve() != HERE.resolve() / "build" or build.is_symlink():
            raise ValueError("unsafe build cleanup target")
        shutil.rmtree(build)


def freeze():
    fixtures = make()
    (HERE / "fixtures.json").write_bytes(json_bytes(fixtures))
    (HERE / "expectations.json").write_bytes(json_bytes(expectations(fixtures)))
    (HERE / "generated.hlsl").write_bytes(generate())
    checked = programs()
    records = [record(cpu(case, checked)) for case in cases(fixtures)]
    (HERE / "cpu-records.json").write_bytes(json_bytes(records))
    (HERE / "cpu-summary.json").write_bytes(json_bytes(summarize(fixtures, records)))
    manifest = {"format": "lx7-manifest-v1", "candidate": "lx7-r1", "experiment_id": "E-2.4-L7",
                "decision": "open", "decider": "owner", "evidence_class": ["source", "synthetic"],
                "device_input_sha256": sha256(input_bytes(fixtures)).hexdigest()}
    for group, root, names in (("candidate", HERE, SOURCES), ("inputs", HERE, INPUTS),
                               ("synthetic", HERE, SYNTHETIC), ("upstream", ROOT, UPSTREAM)):
        manifest[group + "_files"] = inventory(root, names)
        manifest[group + "_sha256"] = aggregate(manifest[group + "_files"])
    (HERE / "manifest.json").write_bytes(json_bytes(manifest))
    print("Frozen lx7-r1; candidate " + manifest["candidate_sha256"])


if __name__ == "__main__":
    if sys.argv[1:] != ["--freeze"]:
        raise SystemExit("Usage: python freeze.py --freeze (deliberate pre-device freeze only)")
    freeze()
