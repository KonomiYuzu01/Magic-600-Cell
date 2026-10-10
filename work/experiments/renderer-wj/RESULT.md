# W-J result card

Status: source/fixture implementation; **W-J is not accepted**. GPU geometry, exact GPU revision checks, GPU negative controls, W-J/W3 gate captures and overlay costs remain unmeasured. Since the anchor change of 10 October 2026 the separate lattice check passes at source level; its GPU run has not happened.

Transform method: one per-piece int32 pose index and a table of row-major 4 x 4 float32 matrices, applied after home shrink. Upload rule: the whole index, logical matrix table and lattice table together on each revision; after-draw first-use readback, SHA-256 and exact 32-bit element comparison. No NVIDIA features, geometry changes, mechanical changes or relabelling.

Source build checked with MSVC 19.51.36260 x64, SDK 10.0.26100.0, DXC 1.8.2502.11, CMake 4.4.3 and Ninja 1.13.2.git.kitware.jobserver-pipe-1. C++ compilation/link and all seven shaders passed. The CPU self-test passed the clean case and rejected all five pose faults; its stale-transform case passed the label-only check. This is compilation/CPU evidence, not Direct3D execution evidence. No executable identity from a disposable sandbox build is used for performance claims.

Lattice evidence before the anchor change (anchors from `assets/mesh_centers.f32`): `python work/experiments/renderer-wj/check_lattice.py` exited 1. J1 generator 1 has all 259,800 labels and coherent transported frames correct, 4,125 differing shrink centres (max 0.0801202387), and shrunken mesh-bound disagreement up to 0.0144216457 world units. Solved agrees. Inputs are the unchanged manifest-checked meshes/centres/frames and J1's labelled projection; see the source check for the production command and immutable reference indexes for input digests. No fix outside this packet's authority was made.

Integrator's independent check (Claude, 10 October 2026, engine Python, the same unchanged assets):
- The raw sticker meshes are congruent under J1 generator 1: each moved sticker's mesh bounds agree with its destination's to 1.07e-7, although the triangulations differ. The twist itself is not at fault.
- The shrink anchor is. `assets/mesh_centers.f32` comes from the retained toolkit's numbering centres (`slot_centers`), not from a centroid that moves with the sticker. For 4,125 of the 4,605 moved stickers the anchor does not map onto the destination's anchor.
- The same anchor gives S-B's accepted W3 a jump at the end of every turn: after the full turn, the shrunk geometry of 4,120 of the 4,600 moving slots differs from its destination slot's by up to 0.0118 world units. The jump is in S-B's SPEC as fixed, not in this packet's code.
- An anchor computed from the unchanged meshes, the area-weighted centroid of each base sticker's triangles, removes both: J1 lattice disagreement at most 7.73e-8 and W3 turn-end jump at most 6.61e-8, with no slot above 2e-6 in either.
- Changing the anchor changes S-B's SPEC section 3 step 1 and the jumbling render contract, so it needs an owner decision, new references and W3 re-acceptance (item 6). It changes no asset and no model identity. Until the decision, item 2 stays failed.

Anchor change, 10 October 2026 (owner decision: `docs/wiki/decisions/owner-decisions-2026-10-10-anchor.md`):
- The shrink anchor of every sticker is now the area-weighted centroid of its triangles, computed from the unchanged `mesh_vertices.f32` and `mesh.json` offsets by the rule in `work/experiments/renderer-sb/SPEC.md` section 3 and rounded once to float32. The 433 x 4 float32 anchors have SHA-256 `0b6ead284d3f62719e3e6ec893d6c199d49698d246a59164bc362446e39faca6`; `probe.cpp`, `reference_wj.py` and `lattice_wj.py` compute them and refuse a different digest. No asset and no model identity changed.
- `tools/.venv/engine/Scripts/python.exe work/experiments/renderer-wj/check_lattice.py` now exits 0 (engine Python, same assets):
  - lattice-start: all 259,800 labels pass, maximum bound and centre error 0;
  - lattice-retained: all 259,800 labels pass, maximum bound error 8.17e-8, maximum centre error 9.80e-8; the refusal threshold is 2e-6.
- The geometry references (`ref_*.f32` and the three indexes) were regenerated with the new anchors. The changed probe has a new build identity; none of its GPU runs has happened.
- This is source evidence. The GPU lattice check, the W-J runs and three cold W3 runs of the changed W-J executable (`run_scene.ps1 -Scene w3 -Runs 3`) wait for the owner's GPU session.

Fill the following only after the owner's runs, using the matching build identity, actual judge summaries and exact-check JSON. A passing timing gate cannot close the failed lattice item.

| Fixture | Build identity | Three-run / pooled fps | p99 ms | Peak local VRAM MiB | Exact pose check every run | W-J verdict |
|---|---|---|---|---|---|---|
| wj-s4 | unmeasured | unmeasured | unmeasured | unmeasured | GPU not run | not run |
| wj-i-a | unmeasured | unmeasured | unmeasured | unmeasured | GPU not run | not run |
| wj-i-b | unmeasured | unmeasured | unmeasured | unmeasured | GPU not run | not run |

| GPU fault (turn 20; README: clean control first, injection applied, the fault's own failure) | Expected result | Actual result |
|---|---|---|
| corrupt-index | fail / exit 2 | not run |
| swap-same-colour | fail / exit 2 | not run |
| stale-pose | fail / exit 2 | not run |
| delay-adoption | fail / exit 2 | not run |
| stale-binding (capture only) | fail / exit 2 | not run |

| Overlay alone | S4 mean / p99 cost ms | I_a mean / p99 cost ms | I_b mean / p99 cost ms |
|---|---|---|---|
| admissible/blocked grip poles | unmeasured | unmeasured | unmeasured |
| two certificate points | unmeasured | unmeasured | unmeasured |
| straddling pieces + points | unmeasured | unmeasured | unmeasured |
| preview angle gauge | unmeasured | unmeasured | unmeasured |

| Acceptance item | Status |
|---|---|
| 1, stdlib geometry reference | three fixtures, six states, three cameras committed; GPU comparison not run |
| 2, lattice agreement | source check passes with the area-centroid anchors (retained bound error 8.17e-8); GPU not run |
| 3, exact revision check every timed run | implemented; CPU functions checked; GPU not run |
| 4, five negative tests | implemented, rejected by CPU self-test; GPU not run |
| 5, three cold W-J runs per fixture | not run; separate W-J judge packet must land |
| 6, W3 re-acceptance on changed build | required for the anchor change too (three cold W3 runs of the changed W-J executable); not run |
| 7, overlays and measured costs | drawings implemented; all costs unmeasured |

The mandatory sandbox acceptance command is `python work/experiments/renderer-wj/check_wj.py`: it proves assets/frame identity, one replay per fixture with exact stage/array digests, six altered-fixture refusals per menu and byte-identical reference outputs. Its pass is source/fixture evidence only. The independent candidate review belongs to the integrator; no review finding or passing review is fabricated here.
