# Second scoped plan re-check packet: the H-06 plan after the re-check findings H06-02 and H06-03

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: a scoped re-check of the changed H-06 plan.
  - The first scoped re-check (`20261003T091431Z-022cddef`) confirmed that H06-01 is fixed. It kept two `major` findings open, H06-02 and H06-03.
  - Both were adopted. `work/experiments/renderer-sb-packets/H6-T-cost-table.md` was rewritten, and `H6-P-probe-features.md` gained the cost scene `w3f` (items 1, 6a, 6b, 9, 10 and 11).
  - This is the last scoped round for the plan. A blocking finding that remains goes to the owner as a decision.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. It answers only these questions:
  1. Are the re-check's H06-02 and H06-03 fixed?
  2. Do the changes introduce a new `blocker` or `major`?

  Report only `blocker` or `major` findings, each with a concrete counterexample. A counterexample is an input for which the planned tool publishes, without marking the row `not-comparable`, either:
  - a cost that is not the cost at matched states: the mean over the `w3f` cycle's states of the feature's frame time minus the baseline's, beyond noise and the stated edge bound;
  - or a gate verdict that is not the gate's own.

  Verdict `pass` if there is none.
- Out of scope:
  - every part of the plan that did not change, including the feature definitions in H6-P item 3;
  - code quality;
  - `minor` and `nit` findings.

## 2. Actual problem and reproduction
The re-check findings are in `work/reviews/20261003T091431Z-022cddef/review.json`. These are the changes:

- **H06-02, missing controls compared equal.**
  - Change: H6-T section 1, item 3, "Controls" and "Comparability".
    - Every control has a "known" rule, listed in a table. Examples:
      - booleans for `vsync`, `tearing`, `warp` and the declared `frame_generation`, `upscaling` and `driver_vsync`;
      - positive integers for sizes;
      - `power_mode` other than `changed` or `unknown`.
    - A control that is not known in any cost run of either group makes the row `not-comparable`, with `controls-unknown` and `unknown_controls`. Two unknown values never count as equal.
    - The gate's own acceptance rules stay unchanged.
  - Tests: item 4, "H06-02, unknown controls".
  - H6-P's README item tells the operator to declare `driver_vsync` and the other conditions in every H-06 run.
- **H06-03, state sampling not matched.**
  - Change: costs no longer come from W3 runs. A new attribution scene, `w3f` (H6-P item 6b), gives every variant the same states in the same proportions. This is the re-check's option of "paired timings at matching poses", and W3 stays as it is.
    - Trace entry k shows state S(k mod n): camera reset at every cycle start, then one 0.002 rad step per frame; turn k // T; phase (k mod T) / T.
    - n = 3140 is a multiple of 2T, with T = 157. The preroll advances no state.
    - Any n consecutive entries show every state once.
    - The gate treats `w3f` as an attribution scene, like W1, W2 and W4, so it is never gate evidence. W3's own trace rules would also reject its frame-stepped turns.
  - The table (H6-T item 3, "State check", "Window" and "Why the comparison is exact"):
    - checks every `w3f` trace entry against the sequence;
    - takes from each run the first c × n consecutive kept frames, with c = kept // n;
    - compares the groups' pooled means and p99.

    Why this is exact:
    - With a fixed lag of up to two in-flight frames, each window still holds every state c times.
    - The mean needs no attribution at all: the summed intervals of c × n consecutive presents are the time of c × n consecutive frames. A change of the lag moves only the window's ends, by at most two frames.
    - A run without a whole cycle gives `cycle-coverage`. A trace that leaves the sequence gives `state-mismatch`.
  - Gate fields (fps, p99, `verdict`, `breaks_gate`, `variant_verdict`) still come from the gate's own `w3` groups, with the H06-01 rule. They count only when their build matches the cost runs' build.
  - Tests: item 4, "H06-03":
    - six simulated cases, including the re-check's counterexample, in both timings and with lags (0, 2) and (2, 0) between the groups;
    - coverage and state-mismatch cases.
- **Why the binned version was dropped before submission.** It used joint bins of 32 camera angles × 2 turn directions × 4 turn phases on W3 runs, with a check over pairing lags. In W3 the phase follows the clock, so frames sample each phase in proportion to 1 / frame time, and per-bin frame means are biased whenever the frame time varies inside a bin.
  - Claude's simulation: a baseline of 3.8 + 2.5 sin(16π × phase) ms, with a feature adding 5 ms everywhere.
  - The binned estimate was 5.711 ms against a truth of 5.0 ms, with a pairing spread of 0.001 ms, so nothing flagged it.
  - H6-T section 2 has the details.

## 3. Environment and versions
- Same as the first check. The review sandbox is read-only, with no GPU.
- Evidence kind: source, plan and CPU simulation.

## 4. Necessary source and evidence
- The rewritten `H6-T-cost-table.md`. Section 2 holds the simulation results of the paired design. They come from the private script `pose_sim5.py`, which is not in the repository; its numbers are quoted there.
- The changed items of `H6-P-probe-features.md`: 1, 6a, 6b, 9, 10 and 11.
- `work/reviews/20261003T091431Z-022cddef/review.json`.
- Source:
  - `tools/perf/renderer_gate.py`;
  - `work/experiments/renderer-sb/probe/src/gpu.cpp` (`runGpu`, the render loop and the label path);
  - `src/probe.cpp` (`turnAt`).
- `docs/progress/1.0/renderer-experiment-plan.md`, lines 22 to 41, for the W2 and W3 scene definitions.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | gate summaries plus matching builds suffice | first plan | Astra plan check | three majors |
| 2 | the gaps verdict from `none`, equal controls, and camera bins fix them | second plan | Astra scoped re-check | H06-01 fixed; H06-02 and H06-03 open |
| 3 | known controls, joint state bins and a pairing check fix them | third plan, not submitted | CPU simulation | biased inside phase bins, and not flagged |
| 4 | known controls and paired runs over a frame-locked cycle fix them | this plan | CPU simulation | within 0.003 ms of the truth in every case, timing and lag; a 70 ms feature gives `cycle-coverage` |

## 6. Constraints and owned files
- Read-only scoped re-check.
- The decisions listed in the first packet's section 6 stay out of scope.
- The W2/W3 scene definitions stay unchanged. `w3f` is an additional attribution scene.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
