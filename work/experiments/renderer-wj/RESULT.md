# W-J result card

Status: measured in the owner's attended GPU session of 10 October 2026, build identity `74b22fec1a969f8d595c2a5a6b3158cd632a12b8556e02ffbe65e83cd43e9daa`. Acceptance items 1 to 6 are met for that build. Item 7, the overlay costs, is unmeasured, so **W-J is not yet accepted as a whole**. The figures below are actual Windows/Direct3D 12 results on one machine, for that build identity only.

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
- The shrink anchor of every sticker is now the area-weighted centroid of its triangles, computed from the unchanged `mesh_vertices.f32` and `mesh.json` offsets by the rule in `work/experiments/renderer-sb/SPEC.md` section 3 and rounded once to float32. The 433 x 4 float32 anchors have SHA-256 `0b6ead284d3f62719e3e6ec893d6c199d49698d246a59164bc362446e39faca6`; `probe.cpp` (when it loads the assets), `reference_wj.py` and `lattice_wj.py` compute them and refuse a different digest; `lattice_wj.py` also refuses any asset whose SHA-256 differs from the manifest. No asset and no model identity changed.
- `tools/.venv/engine/Scripts/python.exe work/experiments/renderer-wj/check_lattice.py` now exits 0 (engine Python, same assets):
  - lattice-start: all 259,800 labels pass, maximum bound and centre error 0;
  - lattice-retained: all 259,800 labels pass, maximum bound error 8.17e-8, maximum centre error 9.80e-8; the refusal threshold is 2e-6.
- The geometry references (`ref_*.f32` and the three indexes) were regenerated with the new anchors. The changed probe has a new build identity; none of its GPU runs has happened.
- This is source evidence. The GPU lattice check, the W-J runs and three cold W3 runs of the changed W-J executable (`run_scene.ps1 -Scene w3 -Runs 3`) wait for the owner's GPU session.

GPU session, 10 October 2026 (owner attended, build `74b22fec…`):
- Conditions:
  - RTX 4070 Laptop GPU, NVIDIA driver 32.0.16.1692, driving the window's display;
  - mains power, Windows power mode `max_performance`;
  - 2560 x 1600 at 60 Hz, exclusive full screen, swap interval 0, no MSAA, no WARP;
  - declared: no frame generation, no upscaling, no overlays.
- The owner watched every timed run and confirmed it afterwards. The judge is `tools/perf/renderer_gate.py`, and its summaries are in [results/](results/). Raw PresentMon output, traces and exact-check records stay private.
- GPU geometry check against the regenerated references: S4, I_a and I_b pass 18 of 18 comparisons each (six states, three cameras), and W3 passes 9 of 9. The maximum error is 1.91e-6. Every one of the 600 cells ran 30,480 vertex invocations, with no count failure.
- GPU lattice check: lattice-start and lattice-retained both pass, each with 259,800 slots and 0 label, centre, bound or lattice failures.
- The first attempt at the timed series stopped after run 1 of each series because of a runner defect: the confirmation text went to the `-Overlay` parameter. Those four runs, and one I_a series whose run 2 the runner refused because stray console input reached the prompt, are private records. They count for nothing. The runner was fixed, and every series below ran again from run 1.

Filled from those runs, using the build identity, the judge summaries and the exact-check records. A passing timing gate cannot close a failed lattice item; the lattice item passed separately.

| Fixture | Build identity | Three-run / pooled fps | p99 ms (three runs / pooled) | Peak VRAM MB (`vram_peak_mb`) | Exact pose check every run | W-J verdict |
|---|---|---|---|---|---|---|
| wj-s4 | `74b22fec…` | 615.33, 614.23, 615.31 / 614.96 | 1.928, 1.932, 1.916 / 1.926 | 86.85 | pass, every drawn revision | `met` |
| wj-i-a | `74b22fec…` | 615.31, 613.45, 611.07 / 613.28 | 1.906, 1.904, 1.922 / 1.911 | 86.80 | pass, every drawn revision | `met` |
| wj-i-b | `74b22fec…` | 612.85, 611.18, 611.72 / 611.92 | 1.921, 1.913, 1.928 / 1.920 | 86.79 | pass, every drawn revision | `met` |

- W3 on the same executable (item 6), three cold runs:
  - fps 610.48, 610.40 and 609.95, pooled 610.28;
  - p99 1.874, 1.853 and 1.855 ms, pooled 1.861 ms;
  - maximum frame time 3.244 ms; peak VRAM 86.52 MB;
  - labels exact in every run; verdict `met`.
- Every timed run had 0 label mismatches, hash failures, late adoptions, binding mismatches or missing revisions. The judge's 180 s interval had no dropped present and no second without a displayed frame.

| GPU fault (turn 20; README: clean control first, injection applied, the fault's own failure) | Expected result | Actual result |
|---|---|---|
| clean control (S4, 10 s) | pass / exit 0 | pass, 53 revisions, all exact |
| corrupt-index | fail / exit 2 | fail / exit 2, injection applied; 1 element mismatch, 1 hash failure |
| swap-same-colour | fail / exit 2 | fail / exit 2, injection applied; 2 element mismatches, 1 hash failure |
| stale-pose | fail / exit 2 | fail / exit 2, injection applied; 15 element mismatches in the matrix table, 1 hash failure |
| delay-adoption | fail / exit 2 | fail / exit 2, injection applied; 1 late adoption, 2 binding mismatches, 0 element mismatches |
| stale-binding (capture only) | fail / exit 2 | fail / exit 2, injection applied; 3 binding mismatches, 1 late adoption, 1 missing revision, 0 element mismatches |

| Overlay alone | S4 mean / p99 cost ms | I_a mean / p99 cost ms | I_b mean / p99 cost ms |
|---|---|---|---|
| admissible/blocked grip poles | unmeasured | unmeasured | unmeasured |
| two certificate points | unmeasured | unmeasured | unmeasured |
| straddling pieces + points | unmeasured | unmeasured | unmeasured |
| preview angle gauge | unmeasured | unmeasured | unmeasured |

| Acceptance item | Status |
|---|---|
| 1, stdlib geometry reference | three fixtures, six states, three cameras committed; GPU comparison passes (maximum error 1.91e-6) |
| 2, lattice agreement | source check passes with the area-centroid anchors (retained bound error 8.17e-8); GPU lattice check passes for both states |
| 3, exact revision check every timed run | passes in all nine timed W-J runs; W3's three runs pass the label check |
| 4, five negative tests | all five fail with exit 2 and their own failure, after a passing clean control |
| 5, three cold W-J runs per fixture | `met` for S4, I_a and I_b |
| 6, W3 re-acceptance on changed build | `met` (pooled 610.28 fps, p99 1.861 ms) |
| 7, overlays and measured costs | drawings implemented; all costs unmeasured |

The mandatory sandbox acceptance command is `python work/experiments/renderer-wj/check_wj.py`: it proves assets/frame identity, one replay per fixture with exact stage/array digests, six altered-fixture refusals per menu and byte-identical reference outputs. Its pass is source/fixture evidence only. The independent candidate review belongs to the integrator; no review finding or passing review is fabricated here.
