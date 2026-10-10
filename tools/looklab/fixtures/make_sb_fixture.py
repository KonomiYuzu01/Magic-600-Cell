"""Capture 300 evenly spread samples per S-B case, with immutable provenance.

Run from any directory: python <checkout>/tools/looklab/fixtures/make_sb_fixture.py <input_folder>
The required input folder is a read-only S-B source copy.
"""
from pathlib import Path
import argparse
import hashlib
import json
import math
import struct

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input_folder", type=Path, help="read-only S-B source copy containing reference/ and workload/")
    source = parser.parse_args().input_folder.resolve()
    names = ["SOURCE_COMMIT", "SPEC.md", "reference_geometry.py", "cameras.json",
             "workload/turn.json", "reference/index.json", "reference/sample.u32"]
    names += [f"reference/c{c}_{pose}.f32" for c in range(3)
              for pose in ("start", "mid", "end")]
    inputs = {name: (source / name).read_bytes() for name in names}
    digests = {name: hashlib.sha256(raw).hexdigest() for name, raw in inputs.items()}
    index = json.loads(inputs["reference/index.json"])
    anchors = index.get("anchors")
    if not isinstance(anchors, dict) or not isinstance(anchors.get("rule"), str) or not isinstance(anchors.get("sha256"), str):
        raise ValueError("reference/index.json: anchors (rule and SHA-256 of the shrink anchors) missing")
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
    fixture = {"format": "magic600-look-sb-reference", "version": 2,
               "sourceBranch": "main", "sourceCommit": inputs["SOURCE_COMMIT"].decode().strip(),
               "anchors": {"rule": anchors["rule"], "sha256": anchors["sha256"]},
               "inputDigests": digests, "referenceInputs": index["inputs"],
               "sourceSampleCount": count, "samplePositions": positions,
               "parameters": index["parameters"], "aspect": index["aspect"], "cases": cases}
    (HERE / "sb-reference.json").write_text(json.dumps(fixture, indent=2, allow_nan=False) + "\n", encoding="utf-8", newline="\n")
    (HERE / "w3-turn.json").write_bytes(inputs["workload/turn.json"])
    # Test-only permutation: W3 is an involution and cannot expose a forward-only inverse implementation.
    even = list(range(259800))
    odd = even.copy()
    move_src, move_dst = [0, 1, 2], [1, 2, 0]
    for src, dst in zip(move_src, move_dst):
        odd[dst] = even[src]
    labels = {"revision_even_sha256": hashlib.sha256(struct.pack(f"<{len(even)}I", *even)).hexdigest(),
              "revision_odd_sha256": hashlib.sha256(struct.pack(f"<{len(odd)}I", *odd)).hexdigest()}
    cycle = {"format": "magic600-sb-turn-v1", "model_id": "synthetic-three-cycle-test-only",
             "angle": 2 * math.pi / 3, "plane_u": [1, 0, 0, 0], "plane_v": [0, 1, 0, 0],
             "moving_slots": move_src, "move_src": move_src, "move_dst": move_dst,
             "inverse_src": move_dst, "inverse_dst": move_src, "labels": labels}
    cycle_bytes = (json.dumps(cycle, indent=2, allow_nan=False) + "\n").encode("utf-8")
    (HERE / "three-cycle-turn.json").write_bytes(cycle_bytes)
    lines = ["# Fixture provenance", "", "The S-B source branch is `main`, at commit",
             f"`{fixture['sourceCommit']}`. These are synthetic model/probe inputs, not session data.", "",
             f"Shrink anchors: {anchors['rule']}. SHA-256 of the 433 x 4 little-endian float32 anchors:",
             f"`{anchors['sha256']}`.", "",
             "`sb-reference.json` retains 300 samples per camera/pose, at source positions",
             "`floor(i * (9066 - 1) / 299)` for i = 0..299. Both endpoints are included.",
             "`w3-turn.json` is the original turn file, copied byte for byte.", "",
             "Production command for both fixtures:", "", "```text",
             "python tools/looklab/fixtures/make_sb_fixture.py <input_folder>", "```", "",
             "`input_folder` is the read-only S-B source copy containing reference/ and workload/.", "",
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
    lines += ["", "## Synthetic inverse-path fixture", "",
              "`three-cycle-turn.json` is a test-only 3-cycle of slots 0, 1 and 2, with angle",
              "2*pi/3 and an orthonormal coordinate plane. It is not a legal model generator",
              "and is never used for rendering. The other 259,797 slots retain their labels.",
              "Its explicit inverse restores solved labels; applying its forward move twice does not.", "",
              "Production command (also regenerates the S-B fixtures above):", "", "```text",
              "python tools/looklab/fixtures/make_sb_fixture.py <input_folder>", "```", "",
              f"Fixture SHA-256: `{hashlib.sha256(cycle_bytes).hexdigest()}`.",
              f"Solved-label SHA-256: `{labels['revision_even_sha256']}`.",
              f"Turned-label SHA-256: `{labels['revision_odd_sha256']}`.",
              "Label digests cover all 259,800 labels encoded as u32 little-endian, like W3."]
    (HERE / "PROVENANCE.md").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print("S-B fixture: 9 cases x 300 samples; turn copied with matching input digest; synthetic 3-cycle written")


if __name__ == "__main__":
    main()
