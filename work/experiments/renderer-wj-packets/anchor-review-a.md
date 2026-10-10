# Review packet, shard A: area-centroid shrink anchors in S-B and its consumers

Review only; do not perform follow-up work. Read only.

## 1. Goal and acceptance
- **Goal.** One full review of the S-B side of the anchor candidate on branch `claude/wj-area-centroid`, which carries out the owner's decision `docs/wiki/decisions/owner-decisions-2026-10-10-anchor.md`. The plan is `work/experiments/renderer-wj-packets/anchor-plan.md` (plan check `20261010T053301Z-f2cab142`, both findings adopted; scoped re-check `20261010T053858Z-f5bbb789` passed). The parts:
  1. Written by Claude: the rule in `work/experiments/renderer-sb/SPEC.md` sections 2 and 3, `sticker_anchors` in `work/experiments/renderer-sb/reference_geometry.py`, and the records `work/experiments/renderer-sb/RESULT.md` (section "Shrink anchor change, 10 October 2026"), `docs/wiki/evidence/s-b-probe-results.md` and the decision page.
  2. Written by Codex in implement call `20261010T053815Z-bfd5455d` (packet `anchor-sb.md`), reviewed and applied by Claude with no change: the rest of `reference_geometry.py`, `check_assets.py`, `reference/*`, `probe/src/probe.cpp`, `probe/src/probe.h` and `probe/README.md`.
  - Diff: `git diff afe5f85 HEAD -- work/experiments/renderer-sb docs/wiki`. `afe5f85` is `main` before this branch.
- **Acceptance.** One JSON result matching `schemas/review-result.schema.json`.
  - Report only `blocker` or `major` findings, each with a concrete counterexample: an input, an asset byte, a sticker, a slot or a run that gives the wrong outcome.
  - The verdict is `pass` if there is no such finding.
- **Questions.**
  1. Is the SPEC rule complete and unambiguous? Could two correct implementations that follow it produce different anchor bytes? Could it refuse the unchanged assets, or accept a broken mesh?
  2. Do `sticker_anchors` (Python floats) and the C++ `Assets::Assets` follow the SPEC order exactly (triangle order, component order 0 to 3 in each dot product, one rounding to float32)? Does each refuse a digest other than the pin, and is the digest taken over the same 433 x 4 little-endian float32 bytes?
  3. Can `check_assets.py` pass when the anchors are wrong? Consider its pin check, the continuity check over the 4,605 animated slots (the `move_src`/`move_dst` pairs plus the self-mapping `moving_slots`), the world-axis bound comparison within 2e-6 and the `mesh_centers.f32` control.
  4. Are the regenerated references consistent with the rule: `index.json` `files` and `inputs`, the `anchors` entry, unchanged `sample.u32`? Does `mesh_centers.f32` still leave the pipeline only where the SPEC says?
  5. Consumers outside the diff. The S-A2 library compiles the S-B `probe.cpp` (`work/experiments/renderer-sa2/native/CMakeLists.txt`), uploads `assets.centers` (`src/scene.cpp`) and checks geometry against `renderer-sb/reference` (`geometry_check`; `src/selftest.cpp`). The Qt S-D scene and the level 2 builds use that library. Does any of them still read `mesh_centers.f32` for the shrink, pin an old reference digest, or parse the index `format` so that the `v2` reference breaks it? The S-B GPU path is `probe/src/gpu.cpp`, `probe/shaders/geometry.hlsl` and `sort.hlsl`.
  6. Do the records claim more than was measured? In particular, do they keep the 3 October W3 result tied to build `2b5bf5e6...` and the old anchor, and leave the GPU re-acceptance as not run?
- **Out of scope.**
  - The owner's choice of the rule, and the W-J and J2 viewer files (shard B reviews them);
  - Look Lab (`tools/looklab/`, another session's area), and the 0.4 code (`magic600-04/`, `web/`, `server.py`, `build_assets.py`);
  - `tools/perf/renderer_gate.py` and the level 2 harness;
  - performance claims and wording;
  - findings below `major`.

## 2. Actual problem and reproduction
- With `assets/mesh_centers.f32` as the anchor, each W3 turn ended with a jump of up to 0.0118 world units in 4,120 of the 4,600 moved slots, and W-J's lattice check failed (0.0144). The raw sticker meshes are congruent under the turn (bounds agree to 1.07e-7); the anchors are not.
- This candidate implements the approved fix. No failure of the candidate is known.

## 3. Environment and versions
- Branch `claude/wj-area-centroid`, at the commit that adds this packet. Windows 11 (build 26200).
- The system CPython 3.14 (standard library only) for `check_assets.py`; the engine environment `tools/.venv/engine` (CPython 3.14.7, NumPy 2.3.5) for the W-J checks; MSVC, the Windows SDK with DXC, CMake and Ninja for the probe.
- Source evidence, run by Claude on the candidate:
  - `python -B work/experiments/renderer-sb/check_assets.py`: ok; the anchor pin; continuity maximum 7.09e-8 over 4,605 slots; control maximum 0.0144 > 2e-6; geometry bytes and digests match;
  - `python -B work/experiments/renderer-sb/probe/check_probe.py`: ok, including `selftest: shrink anchors SHA-256: ok`;
  - `python -B work/experiments/renderer-sa2/check_project.py`: PASS; `python -B work/experiments/renderer-sd/check_project.py`: ok; `python -B tests/test_renderer_gate.py`: OK;
  - the wiki lint and its test: ok.
- No GPU run, no `--geometry-check` and no W3 capture has used this candidate.

## 4. Necessary source and evidence
- Rule: `work/experiments/renderer-sb/SPEC.md` sections 2 and 3; contract: `research/jumbling/render-contract.md` section 2.
- Packets: `work/experiments/renderer-wj-packets/anchor-plan.md`, `anchor-sb.md`.
- Workload: `work/experiments/renderer-sb/workload/turn.json`.
- Consumers: listed in question 5.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | The anchor, not the twist, causes the turn-end jump | integrator check of raw mesh bounds | NumPy | raw meshes agree to 1.07e-7; the anchors do not |
| 2 | The area-weighted triangle centroid moves with the sticker | `sticker_anchors` | stdlib, NumPy, JavaScript and both C++ probes give the same digest `0b6ead28...faca6` | owner chose it |
| 3 | `anchor-sb.md` as written | implement call `20261010T053815Z-bfd5455d` | wrapper acceptance check; Claude's review; the checks in section 3 | valid, applied with no change |

## 6. Constraints and owned files
- Read only. No file changes.
- Invariants: synthetic labels only; claims only for the measured build identity; no asset and no model identity change; never commit raw PresentMon output or machine diagnostics.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`, with an answer to each of questions 1 to 6 in the summary.
- Review only; do not perform follow-up work.
