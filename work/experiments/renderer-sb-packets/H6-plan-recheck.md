# Scoped plan re-check packet: the H-06 plan after findings H06-01 to H06-03

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: a scoped re-check of the changed H-06 plan. The plan check `20261003T085715Z-0c924f46` found three `major` findings. All three were adopted, and both packets were revised:
  - `work/experiments/renderer-sb-packets/H6-T-cost-table.md`;
  - `work/experiments/renderer-sb-packets/H6-P-probe-features.md`.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. It answers only these questions:
  1. Is each of H06-01, H06-02 and H06-03 fixed by the revised packets?
  2. Do the changes introduce a new `blocker` or `major`?

  Report only `blocker` or `major` findings, each with a concrete counterexample. Verdict `pass` if there is none.
- Out of scope:
  - every part of the plan that did not change, which the first check already covered;
  - code quality;
  - `minor` and `nit` findings.

## 2. Actual problem and reproduction
The findings (full text: `work/reviews/20261003T085715Z-0c924f46/review.json`) and the changes made:

- **H06-01, the gaps row reported the disabled feature's gate result.**
  - Change: H6-T section 1, item 3, "Rows".
    - The `gaps` row keeps the `no-gaps` group's verdict as `variant_verdict`.
    - `verdict` and `breaks_gate` come from the group in which the feature is on, which for gaps is `none`.
    - The cost sign stays: baseline minus `no-gaps`.
  - Tests: both counterexamples, in item 4, "H06-01".
- **H06-02, matching builds did not establish comparable measurements.**
  - Change: H6-T item 3, "Controls" and "Comparability".
    - Every valid run has control values: the build, `turn_ms`, `camera`, presentation interval, `vsync`, `tearing`, `warp`, the presenting adapter with its driver, power source and mode, display and backbuffer size and refresh, the declared frame generation, upscaling, driver V-Sync and vendor mode, and `msaa`.
    - Controls must be equal within each group and between each feature group and the baseline. The only exception is `msaa`: 4 for `msaa4`, and 1 everywhere else.
    - Any difference gives `not-comparable`, null costs and `differing_controls`.
  - `b412_summary` also publishes `msaa`, `vsync`, `tearing` and `warp`.
  - Tests: item 4, "H06-02", covers a mismatched presentation interval and resolution, msaa, a within-group difference and `turn_ms`.
- **H06-03, frame-driven camera rotation confounded the comparison.**
  - The rotation stays frame-driven. The renderer experiment plan defines W2 and W3 that way: "one fixed rotation step per frame" (`docs/progress/1.0/renderer-experiment-plan.md`, lines 23 and 27). Changing it would change the gated scene.
  - Change in H6-P item 6a: every trace entry records its `camera` step count, and `run.json` records the camera definition.
  - Change in H6-T, "Pose balancing":
    - each frame is assigned to one of 64 bins of the camera angle modulo 2π;
    - a group needs at least 20 frames in every bin;
    - costs come from pose-balanced statistics: the mean of the per-bin means, and a nearest-rank p99 weighted per bin.
    - The turn animation follows the QPC clock, so with equal `turn_ms`, which is a control, its coverage is the same in every run.
  - Formal mode now reads run directories, the same ones the gate reads, instead of gate summaries. Per-frame poses are needed for that.
  - `renderer_gate.py` gains a trace timing for callers only (`run_frames`, `summarize(..., timing='trace')`); its command line stays PresentMon-only.
  - Tests: item 4, "H06-03", makes the Astra simulation an attribution acceptance check in both timings, and adds coverage and missing-pose cases.

## 3. Environment and versions
- Same as the first check. The review sandbox is read-only, with no GPU. Evidence kind: source and plan.

## 4. Necessary source and evidence
- The two revised packets.
- `work/reviews/20261003T085715Z-0c924f46/review.json`.
- `tools/perf/renderer_gate.py`, `tools/perf/b412_summary.py`, and `work/experiments/renderer-sb/probe/src/gpu.cpp` (`runGpu`, `environment`).
- A CPU check of the H06-03 scenario, made before the revision. It used:
  - a camera step of 0.002 rad per frame from the first frame;
  - a 4 s preroll and the gate interval;
  - a baseline of 1.3 ms, and a feature adding 30 ms over half the rotation.

  Results:
  - the plain difference was 16.230 to 16.232 ms for three placements of the slow half;
  - the pose-balanced difference was 15.000 ms in every placement;
  - the smallest bin held 147 frames;
  - a feature at 70 ms per frame left empty bins.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | gate summaries plus matching builds suffice | first plan | Astra plan check | three majors |
| 2 | the gaps verdict from `none`, controls, and pose balancing fix them | revised packets | this re-check | pending |

## 6. Constraints and owned files
- Read-only scoped re-check. The decisions listed in the first packet's section 6 stay out of scope. The frame-driven W2/W3 camera is part of the scene definition and stays.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
