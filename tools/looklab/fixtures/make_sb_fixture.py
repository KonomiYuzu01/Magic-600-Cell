"""Capture 300 evenly spread samples per S-B case, with immutable provenance.

Run from any directory: python tools/looklab/fixtures/make_sb_fixture.py
The read-only input copy is ../../reviews/inputs/looklab-sb from the repo root.
"""
from pathlib import Path
import hashlib
import json
import struct

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SOURCE = ROOT / "../../reviews/inputs/looklab-sb"


def main():
    names = ["SOURCE_COMMIT", "SPEC.md", "reference_geometry.py", "cameras.json",
             "workload/turn.json", "reference/index.json", "reference/sample.u32"]
    names += [f"reference/c{c}_{pose}.f32" for c in range(3)
              for pose in ("start", "mid", "end")]
    inputs = {name: (SOURCE / name).read_bytes() for name in names}
    digests = {name: hashlib.sha256(raw).hexdigest() for name, raw in inputs.items()}
    index = json.loads(inputs["reference/index.json"])
    for name, expected in index["files"].items():
        if digests[f"reference/{name}"] != expected:
            raise ValueError(f"reference/{name}: digest mismatch with index.json")
    for name, local in [("cameras.json", "cameras.json"), ("workload/turn.json", "workload/turn.json")]:
        if digests[local] != index["inputs"][f"work/experiments/renderer-sb/{name}"]:
            raise ValueError(f"{name}: digest mismatch with index.json inputs")
    count = index["sample_count"]
    if count != 9066:
        raise ValueError("expected 9,066 S-B source samples")
    sample = struct.unpack(f"<{count}I", inputs["reference/sample.u32"])
    positions = [i * (count - 1) // 299 for i in range(300)]
    cameras = json.loads(inputs["cameras.json"])["cameras"]
    cases = []
    for camera in cameras:
        for pose in ("start", "mid", "end"):
            data = inputs[f"reference/{camera['name']}_{pose}.f32"]
            if len(data) != count * 12:
                raise ValueError("wrong reference float count")
            cases.append({"camera": camera["name"], "rotations": camera["rotations"],
                          "pose": pose, "theta": index["poses"][pose],
                          "samples": [{"vertex": sample[i], "reference": list(struct.unpack_from("<3f", data, i * 12))}
                                      for i in positions]})
    fixture = {"format": "magic600-look-sb-reference", "version": 1,
               "sourceBranch": "claude/renderer-sb", "sourceCommit": inputs["SOURCE_COMMIT"].decode().strip(),
               "inputDigests": digests, "referenceInputs": index["inputs"],
               "sourceSampleCount": count, "samplePositions": positions,
               "parameters": index["parameters"], "aspect": index["aspect"], "cases": cases}
    (HERE / "sb-reference.json").write_text(json.dumps(fixture, indent=2, allow_nan=False) + "\n", encoding="utf-8", newline="\n")
    (HERE / "w3-turn.json").write_bytes(inputs["workload/turn.json"])
    lines = ["# Fixture provenance", "", "The S-B source branch is `claude/renderer-sb`, at commit",
             f"`{fixture['sourceCommit']}`. These are synthetic model/probe inputs, not session data.", "",
             "`sb-reference.json` retains 300 samples per camera/pose, at source positions",
             "`floor(i * (9066 - 1) / 299)` for i = 0..299. Both endpoints are included.",
             "`w3-turn.json` is the original turn file, copied byte for byte.", "",
             "Production command for both fixtures:", "", "```text",
             "python tools/looklab/fixtures/make_sb_fixture.py", "```", "",
             "Inputs (paths relative to the S-B source copy):", "", "| File | SHA-256 |", "|---|---|"]
    lines += [f"| `{name}` | `{digest}` |" for name, digest in digests.items()]
    lines += ["", "Asset input digests used by the original reference:", "", "| File | SHA-256 |", "|---|---|"]
    lines += [f"| `{name}` | `{digest}` |" for name, digest in index["inputs"].items() if name.startswith("assets/")]
    model_digest = hashlib.sha256((ROOT / "assets/model.npz").read_bytes()).hexdigest()
    lines += ["", "`rings.json` is a separate deterministic engine-order fixture. Its command is:", "", "```text",
              "python tools/looklab/fixtures/make_rings_fixture.py", "```", "",
              f"Input `assets/model.npz`: `{model_digest}` (verified against the manifest).",
              "Only normals.npy, orbit_id.npy, face_offsets.npy and face_values.npy are read.",
              "The helix enumeration, tuple sorting, lowest-uncovered-cell search and 7-regular",
              "acceptance rule port tools/tastelab/page/geometry.js and sim/geometry_fixture.py.",
              "Vertex ids retain core.py's ascending orbit-34 piece-position order; cells retain engine ids.", "",
              "Tools: Python standard library (the packet environment is CPython 3.14.7).",
              "The generators contain their editable production source. Rerunning them requires",
              "the scoped read-only S-B source copy for the S-B fixtures; acceptance only needs committed fixtures."]
    (HERE / "PROVENANCE.md").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print("S-B fixture: 9 cases x 300 samples; turn copied with matching input digest")


if __name__ == "__main__":
    main()
