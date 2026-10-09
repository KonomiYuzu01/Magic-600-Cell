# Packet H6-T: feature runs in the renderer gate, and the H-06 cost table

Run from the `claude/renderer-sb` checkout:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-sb-packets/H6-T-cost-table.md`

The parallel packet H6-P (`H6-P-probe-features.md`) owns the probe. It adds:
- the feature variants and a top-level `"feature"` key in every `run.json`;
- a `camera` step count in every trace entry, and a top-level `"camera"` object;
- the cost scene `w3f`, described below.

It keeps `environment.msaa`. This packet owns only the tools below and their tests.

## 1. Goal and acceptance
- Goal: the H-06 table gives two things for each feature.
  - **The cost:** the frame time the feature adds at matched W3 states.
  - **The gate verdict** of W3 with the feature on.

  Three rules hold throughout:
  - Runs of different features are never pooled.
  - Costs come from paired runs. Every variant draws the same states in the same proportions, so the comparison needs no binning and no model.
  - Costs are compared only under known, matching conditions.
- **The cost scene `w3f`.** H6-P defines it; the tools rely on these facts.
  - The scene is W3, with every animation state stepped by the frame counter instead of the clock.
  - The first trace entry has `frame` 0. Trace entry k (its `frame`) shows state S(k mod n):
    - the camera is reset to its start pose at every k with k mod n = 0, then turned one 0.002 rad step per frame, so `camera` = (k mod n) + 1;
    - the turn index is k // T, and `phase` = (k mod T) / T;
    - the direction comes from the turn's parity, as in `turnAt`.
  - T is `turn_frames` (default 157). n is `cycle_frames` (default 3140, about one camera rotation of 3141.6 steps). n is a multiple of 2T, so the turn index of entry k has the same parity as that of entry k mod n, and S(k mod n) fixes the direction too.
  - The preroll advances no state.
  - The label path is W3's: one upload per turn, with the label check.
  - `run.json` has:
    - `"scene": "w3f"`;
    - `"turn_frames"` and `"cycle_frames"`;
    - the camera object `{"plane": [0, 3], "step_rad": 0.002, "per": "frame"}`.
  - **Key property:** any n consecutive trace entries show every state of the cycle exactly once.
- Acceptance:
  1. **`tools/perf/renderer_gate.py`.**
     - **Feature.** `run.json` may carry a top-level `feature`.
       - When present, it must be a string that matches `NAME`; otherwise the run is unreadable with reason `bad-feature`.
       - When absent, the feature is `none`.
     - **Grouping.** `summarize` groups runs by (candidate, scene, feature) instead of (candidate, scene).
       - Each entry of `scenes` and of `invalid_runs` carries `"feature"`.
       - `scenes` is sorted by (candidate, scene, feature).
     - **Scene `w3f`.**
       - Add it to `SCENES` and `LABEL_SCENES`, but not to `GATE_SCENES`. Its groups therefore get the verdict `attribution`, like W1, W2 and W4, and a `w3f` run is never gate evidence.
       - `validate_run` requires, for `w3f`, an integer `turn_frames` of at least 2 and an integer `cycle_frames` that is a positive multiple of 2 × `turn_frames`. Otherwise the run is unreadable with reason `bad-cycle`.
     - **Trace entries.** A trace entry may carry `camera`, a non-negative integer. Any other value makes the trace `bad-trace`. The gate does not use it.
     - **Trace steps.**
       - Move the first rule of `trace_reasons` into a new public function `trace_step_reasons(entries, run, frame_ticks)`: `trace-empty`, and `trace-incomplete` when a step between two measured presents holds a number of trace entries other than one, or the trace does not cover the interval.
       - `trace_reasons` calls it, so the gate's behaviour does not change.
       - `summarize` also applies `trace_step_reasons` to `w3f` runs.
     - **Frame source.** A new function `run_frames(directory, run, columns, timing)` returns the same tuple as `read_frames`: values, ticks, ignored chains and reasons.
       - `timing='presentmon'` calls `read_frames`, unchanged.
       - `timing='trace'` takes the frames from the probe's own trace (`read_trace`), for runs without PresentMon. The kept entries are those whose `qpc` lies in `interval_ticks(run)`.
         - Values: for each kept entry that has a predecessor in the trace, its `qpc` minus the predecessor's, in ms.
         - Ticks: the kept entries' `qpc`.
         - Ignored chains: none.
         - Reasons, with the rules of `read_frames`:
           - `capture-short` when `trace_stop_qpc` is before the interval end;
           - `capture-not-covered` when the first kept entry lies later than `EDGE_S` after the interval start, or the last lies earlier than `EDGE_S` before its end;
           - `trace-empty` when there is no kept entry.
     - **`summarize`.** The signature becomes `summarize(directories, columns=None, timing='presentmon')`, and it gets its frames from `run_frames`.
       - With trace timing, every run also needs a `window` object in which `visible_throughout` and `foreground_throughout` are both `true`. Otherwise the run is invalid, with reason `window-missing`, `window-not-visible` or `window-not-foreground`.
       - With trace timing, the result adds `"timing": "probe-trace"` and `"preliminary": true`.
       - The command line (`main`) gets no timing option, so a gate summary file always comes from PresentMon.
     - **Unchanged.**
       - Verdict rules, thresholds, interval, the W3 trace rules, label rules and sanitising stay exactly as they are.
       - For PresentMon timing and inputs without `feature` or `w3f`, the output differs from today's only by `"feature": "none"` in those entries.
       - Update `METHOD` and the module docstring with one sentence each:
         - runs of different features are judged separately;
         - `w3f` is an attribution scene for H-06 costs and never gate evidence.
  2. **`tools/perf/b412_summary.py`.**
     - `PUBLIC_OBJECTS['declared']` adds `upscaling`. The gate requires it as a condition, but the summary never published it.
     - `PUBLIC_SCALARS` adds `msaa`, `vsync`, `tearing` and `warp`.
     - All of these keys are optional, and B4-12 outputs without them are unchanged.
  3. **New `tools/perf/feature_costs.py`.** It builds the H-06 table from run directories: `w3` runs for the gate verdicts, and `w3f` runs for the costs.
     - **Command line.**
       - Run directories are positional arguments, `w3` and `w3f` runs together.
       - `--timing presentmon|trace` (default `presentmon`) and `--candidate` (default `s-b`).
       - `--out <file>` and an optional `--markdown <file>`.
     - **Gate data.** It makes one call to `renderer_gate.summarize(directories, timing=...)`.
       - The call's `w3` groups for the candidate give the gate numbers and verdicts per feature. Those are the gate's own.
       - Its valid `w3f` runs are the candidate cost runs. They have passed the gate's run rules: conditions, label check, coverage and one trace entry per step, and the window rules in trace timing.
     - **Per-frame data.** For the valid `w3f` runs it reads `run.json`, `run_frames` and `read_trace`, all imported, never copied.
     - **State check.**
       - For each valid `w3f` run, every trace entry must follow the sequence:
         - `frame` is its 0-based position in the trace;
         - `camera` = (`frame` mod n) + 1;
         - `turn` = `frame` // T;
         - `phase` = (`frame` mod T) / T within 1e-9;
         - `revision` = `turn`.
       - The `camera` object must be the one above.
       - If any check fails, the run gives no cost frames, and it is listed in `cost_invalid_runs` with the reason `state-mismatch`.
     - **Window.**
       - Each kept frame is paired with its nominal trace entry:
         - in PresentMon timing, the last entry whose `qpc` is not later than the frame's present tick;
         - in trace timing, the entry that ends the step.
       - The nominal entries form one unbroken series of consecutive `frame` values; otherwise the reason is `state-mismatch`.
       - The run's `cycles` is c = (number of kept frames) // n.
       - Its window is the c × n kept frames in the middle of the kept series. The leftover frames are split evenly between the two ends, with the odd one at the end, so every group's window is centred on the interval.
       - A run with c = 0 gives no cost frames, and it is listed in `cost_invalid_runs` with the reason `cycle-coverage`.
     - **Why the comparison is exact.**
       - A window covers c × n consecutive entries, so it holds every state of the cycle exactly c times, in every variant and at any frame rate.
       - A measured interval may reflect a frame up to two entries earlier, because two frames are in flight. With a fixed lag the window moves by that many entries and still holds every state c times.
       - The mean does not depend on attribution at all: the summed intervals of c × n consecutive presents are the time taken by c × n consecutive frames. A change of the lag can only move the window's ends, by at most two frames.
       - Two groups' windows therefore hold the same states in the same proportions. The difference of their means is the mean cost over the cycle's states, and the difference of their p99 compares the same state mix.
       - Like the gate, this assumes the machine's speed is steady during the interval; the warm-up and the power conditions serve that. Nothing is claimed for a machine whose speed drifts.
     - **Statistics.** Per group, over the pooled window values of its runs:
       - `paired_mean_ms`, the mean;
       - `paired_p99_ms`, the nearest-rank p99, using the gate's `nearest_rank`;
       - `cycles`, the sum over the runs.
     - **Costs.**
       - `cost_mean_ms` and `cost_p99_ms` are the feature group minus the baseline.
       - For `gaps` the sign is reversed: the baseline minus `no-gaps`. A positive cost then means gaps make frames slower.
     - **Controls.** Each run used for a cost has these control values, and each must be known:

       | Control | Known when |
       |---|---|
       | build identity | a non-empty string |
       | `turn_frames`, `cycle_frames` | integers as `validate_run` requires |
       | `camera` | the object above |
       | `presentation_interval` | an integer of at least 0 |
       | `vsync`, `tearing`, `warp` | booleans |
       | `presenting_adapter` (includes the driver version) | a non-empty string that does not contain `version unavailable`. The probe writes `UMD version unavailable (0x…)` there when its driver query fails, so that text is unknown even when both groups have it. |
       | `power_source` | `mains` |
       | `power_mode` | a string other than `changed` and `unknown` |
       | `display` width, height and `refresh_hz`; `backbuffer` width and height | positive integers |
       | `declared` `frame_generation`, `upscaling` and `driver_vsync` | booleans |
       | `declared` `vendor_mode` | a non-empty string |
       | `msaa` | an integer of at least 1 |

     - **Comparability.**
       - If any control is not known in any cost run of the baseline (`none`) group or of the feature group, the row is `not-comparable`. Its reason is `controls-unknown`, with a list `unknown_controls` of the control names. Two unknown values never count as equal.
       - All cost runs of a group must have equal controls.
       - Each feature group must equal the baseline group on every control except `msaa`. For `msaa`, the `msaa4` group must have 4, and every other group, the baseline included, must have 1.
       - Any difference makes the row `not-comparable`, with reason `controls-differ` and a list `differing_controls` of the control names.
       - Both lists hold names only, never values.
     - **Rows.**
       - One row for the baseline (`none`), then one per H-06 feature, in this order: `gaps`, `outlines`, `transparency`, `fog`, `dof`, `ao`, `msaa4`.
       - The `gaps` row is measured with the `no-gaps` variant, because the gated look already has gaps.
       - **Cost fields**, from `w3f`:
         - `feature` and the measured `variant`;
         - `runs`, `build_identity` (or null) and `cycles`;
         - `paired_mean_ms` and `paired_p99_ms` of the variant group;
         - `cost_mean_ms` and `cost_p99_ms`.
       - **Gate fields**, from `w3`:
         - `gate_runs`;
         - the variant group's pooled `fps`, `mean_frame_ms` (1000 / fps) and `p99_ms`;
         - `variant_verdict`: the gate verdict of the variant's `w3` group;
         - `verdict`: the gate verdict of the `w3` group in which the feature is on. That is the variant's own group, except for `gaps`, where it is the `none` group;
         - `breaks_gate`: true when `verdict` is `not-met`, false when it is `met`, and null otherwise.

         Gate fields come only from a `w3` group whose runs all have the build of the row's cost runs; for a row without cost runs, the build of the baseline's cost runs. Otherwise they are null, and `reasons` includes `gate-other-build`. Without `w3` runs, the verdicts are `no-data` and `breaks_gate` is null.
       - `status`, `reasons`, `unknown_controls` and `differing_controls`.
     - **Status.**
       - `unmeasured`: the variant has no `w3f` runs that passed the gate's run rules. Every cost number is null, never estimated.
       - `not-comparable`, with costs null and the group's own numbers kept, in any of these cases:
         - the baseline has no cost frames;
         - a group's runs all gave no cost frames (`cycle-coverage` or `state-mismatch`);
         - a group has mixed builds;
         - the build identities of the group and the baseline differ;
         - `controls-unknown` or `controls-differ`.
       - `preliminary`: trace timing, or fewer than three runs with cost frames in either group, and none of the above.
       - `measured`: PresentMon timing, at least three runs with cost frames in both groups, and none of the above.
       - The baseline row uses the same rules for its own group, and its costs are null.
     - **Output.** `--out <file>` writes JSON, sanitised with `b412_summary.sanitize`, with:
       - `format`: `magic600-h06-feature-costs-v1`;
       - `timing_source`: `presentmon` or `probe-trace`;
       - `preliminary`: true in trace timing, false otherwise;
       - `candidate` and `method`;
       - `turn_frames` and `cycle_frames`;
       - `controls`: the baseline's control values;
       - `baseline`, `features`, `invalid_runs` (the gate's), `cost_invalid_runs` and `unreadable`.
     - **Markdown.** `--markdown <file>` writes the same table, with one line each for:
       - the timing source;
       - the short build identity;
       - "costs are paired: every variant draws the same W3 states (scene w3f), in whole cycles; gate fields come from W3 runs";
       - "unmeasured features are not estimated; not-comparable features have no cost".
     - **Refusals.** It refuses an existing output (`existing-output`), and it exits 1 when no run is readable.
  4. **Tests.**
     - **`tests/test_renderer_gate.py`.**
       - Three `none` and three `fog` runs in one call give two `met` groups.
       - A missing feature means `none`, and a bad feature value is `bad-feature`.
       - Invalid-run entries carry the feature.
       - A trace `camera` that is not a non-negative integer is `bad-trace`.
       - `w3f`:
         - three `w3f` runs get `attribution`, never `met`;
         - missing or bad cycle keys give `bad-cycle`;
         - a `w3f` run in which a present step holds two trace entries is invalid with `trace-incomplete`.
       - Trace timing:
         - a covered trace gives the expected steps;
         - an uncovered trace is `capture-not-covered`;
         - a hidden window is `window-not-visible`;
         - the result carries `timing` and `preliminary`.
       - `main` has no timing option.
       - All existing tests still pass, changed only where they compare whole entries.
     - **`tests/test_b412_summary.py`.** `upscaling`, `msaa`, `vsync`, `tearing` and `warp` are published.
     - **New `tests/test_feature_costs.py`.** Synthetic data only.
       - **H06-01:**
         - `w3` runs: three `none` runs at a constant 31 ms and three `no-gaps` runs at 34 ms. Together with `w3f` runs at the same frame times, they give:
           - a `gaps` cost of -3 ms;
           - `verdict` `met` and `breaks_gate` false;
           - `variant_verdict` `not-met`.
         - The reversed timings give +3 ms, `breaks_gate` true and `variant_verdict` `met`.
       - **H06-02, all with the same build:**
         - `fog` runs with `presentation_interval` 1 against a baseline with 0;
         - `fog` runs at 1920 x 1200 against a 2560 x 1600 baseline;
         - a `fog` group with `msaa` 4;
         - a group whose own runs differ in `power_mode`;
         - a `cycle_frames` mismatch.

         Each gives `not-comparable`, null costs and the right `differing_controls`. An `msaa4` group with `msaa` 4 against a baseline with 1 is comparable.
       - **H06-02, unknown controls:**
         - `declared.driver_vsync` absent in both groups;
         - `presentation_interval` null in both groups;
         - `power_mode` `changed`;
         - the same `presenting_adapter` in both groups, containing the probe's text `driver UMD version unavailable (0x80004005)`.

         Each gives `not-comparable` with `controls-unknown`, the right `unknown_controls` and null costs.
       - **H06-03, simulated attribution checks.** Generate the `w3f` frames in code.
         - **Model:**
           - T = 157 and n = 3140;
           - a 4 s preroll that advances no state, then the gate interval;
           - three runs per group, with seeded multiplicative jitter of ±2 % per frame.
         - **Timing:**
           - Frame k starts at trace entry k and lasts its cost.
           - In PresentMon timing, frame k is presented just before entry k+1, and a group may use a fixed lag ℓ of 0 to 2: the interval paired with entry k is frame k−ℓ's.
           - Trace timing has no extra lag.
         - **Truth:** the mean of feature cost minus baseline cost over the n states of the cycle, computed exactly.
         - **Rule checked in every case:** `cost_mean_ms` within max(0.02 ms, 0.5 % of the truth) of the truth, in both timings, and in PresentMon timing also with the lags (0, 2) and (2, 0) between the groups.
         - **Cases:**
           - **Camera:** baseline 1.3 ms, and the feature adds 30 ms whenever the camera angle lies in [0, π). The truth is 15.0 ms.
           - **Turn phase** (the re-check's counterexample): baseline 1.3 ms plus 15 ms when the phase is below 0.5, and the feature adds 5 ms everywhere. The truth is 5.0 ms.
           - **Direction:** baseline 1.3 ms plus 10 ms on odd turns, and the feature adds 5 ms everywhere. The truth is 5.0 ms.
           - **Smooth camera:** baseline 1.3 + 0.3 sin(angle) ms, and the feature adds 10 (1 + sin(angle)) ms. The truth is about 10.0 ms.
           - **Within a phase bin:** baseline 3.8 + 2.5 sin(16π × phase) ms, and the feature adds 5 ms everywhere. The truth is 5.0 ms. This case broke the binned estimator of the previous plan (section 2).
           - **Constant:** the feature adds 30 ms everywhere. The truth is 30.0 ms.
         - **Coverage:** a feature at 70 ms per frame has no whole cycle in the interval. It gives:
           - `cycle-coverage` and `not-comparable`;
           - null costs;
           - its gate fields still given when it has `w3` runs.
         - **State check:** a `w3f` trace whose camera is not reset at a cycle start, or whose phase follows the clock, gives `state-mismatch`.
         - The simulation cases may call the module's window and statistics functions directly with generated frames. The turn-phase case also runs end to end through run directories, in both timings.
       - **Further cases:**
         - all features measured;
         - an unmeasured feature, with null cost numbers;
         - mixed builds give `not-comparable`;
         - gate fields are null, with `gate-other-build`, when the `w3` runs are of another build;
         - fewer than three runs give `preliminary`;
         - trace timing with a failed label check;
         - trace timing with a window that is not visible;
         - outputs refuse to overwrite;
         - the Markdown names every feature;
         - the output contains no absolute path or user name.
     - **Fixtures.**
       - Create temporary directories the way `tests/test_renderer_gate.py` `setUp` does: inside the repository, with the `os.mkdir` mode workaround. `tempfile` directories in the system temp folder fail in the Windows sandbox.
       - Keep the whole acceptance check under three minutes. Cases other than the simulations may use short frame series.
  5. The acceptance check passes.

## 2. Actual problem and reproduction
- The S-B probe's runtime options do not change its build identity. A `--msaa 4` run, or later a feature run, of the same build is therefore pooled with baseline runs under one (candidate, scene) group, and the `mixed-builds` rule cannot catch it.
- There is no tool that turns gate results into the H-06 table.
- The Astra plan check (`20261003T085715Z-0c924f46`) and its scoped re-checks (`20261003T091431Z-022cddef`, `20261003T100312Z-35f8a900`) found these faults in earlier versions of this packet:
  - **H06-01, fixed in the second version:** the gaps row took its gate result from the `no-gaps` group.
  - **H06-02:**
    - Matching builds alone let different presentation intervals or resolutions be published as feature costs.
    - Then, controls missing in both groups compared equal.
    - Then, in the fourth version, the probe's unavailable-driver text counted as a known driver version. Two runs on different drivers whose version query failed would compare equal. This version treats that text as unknown, with a test.
  - **H06-03:**
    - With a camera driven by frames, a slower variant samples other camera poses. A simulated 30 ms feature over half the rotation gave 16.232 ms by plain subtraction, against 15.0 ms at matching poses.
    - Then, balancing over camera bins alone left the turn phase uncontrolled. A baseline with 15 ms more in the first half of each turn, and a feature adding 5 ms everywhere, gave 7.94 ms.
- **Third version: binned estimation, not submitted.**
  - Design:
    - joint bins of 32 camera angles × 2 turn directions × 4 turn phases, on the gate runs;
    - a check over pairing lags 0 to 2.
  - What it did: on the cases above it was within 0.06 ms of the truth, or `not-comparable`.
  - Why it fails: in W3 the turn phase follows the clock, so frames sample each phase in proportion to 1 / frame time. A per-bin frame mean is therefore biased whenever the frame time varies inside a bin.
  - Claude's simulation (private, `pose_sim5.py` part A): a baseline of 3.8 + 2.5 sin(16π × phase) ms, with a feature adding 5 ms everywhere.
    - The binned estimate was 5.711 ms against a truth of 5.0 ms.
    - Its pairing spread was 0.001 ms, so nothing flagged it.

  No finite binning of a clock-driven state removes this.
- **This version: paired cost runs.**
  - The cost scene `w3f` draws the same frame-indexed W3 states in every variant, and the table compares whole cycles.
  - The same simulation (private, `pose_sim5.py` part B) uses three jittered runs per group, with lags (0, 0), (0, 2) and (2, 0) in PresentMon timing and the trace timing:

    | Case | Truth | Cost, every timing and lag | Whole cycles per run (baseline / feature) |
    |---|---|---|---|
    | camera | 15.0 ms | 15.000 ms | 44 / 3 |
    | turn phase | 5.0 ms | 4.999 ms | 6 / 4 |
    | direction | 5.0 ms | 4.999 ms | 9 / 5 |
    | smooth camera | 10.0 ms | 9.999 ms | 44 / 5 |
    | within a phase bin | 5.0 ms | 5.000 ms | 15 / 6 |
    | constant 30 ms | 30.0 ms | 30.002 ms | 44 / 1 |
    | 70 ms feature | — | `cycle-coverage` | 44 / 0 |

- Without an administrator PowerShell, PresentMon cannot run. Unattended runs then have only the probe's own trace, which `renderer_gate.py` does not read today.

## 3. Environment and versions
- Windows 11, CPython 3.14.7 64-bit, standard library only.
- The sandbox is CPU only, with no PresentMon.

## 4. Necessary source and evidence
- `tools/perf/renderer_gate.py`, the whole file:
  - `validate_run`, `condition_reasons`, `interval_ticks`, `read_frames`, `read_trace`, `trace_reasons`, `judge`, `summarize`;
  - the constants `SCENES`, `GATE_SCENES`, `LABEL_SCENES`, `WARMUP_S`, `INTERVAL_S`, `EDGE_S`, `RUNS_MIN` and `NAME`.
- `tools/perf/b412_summary.py`: `PUBLIC_SCALARS`, `PUBLIC_OBJECTS`, `public_environment`, `sanitize`, `nearest_rank`, `RunError`, `require`.
- `tests/test_renderer_gate.py`, for the `run_dir` fixture, and `tests/test_b412_summary.py`.
- The probe's trace has one `trace.jsonl` line per frame, with `frame`, `qpc`, `revision`, `turn` and `phase`. H6-P adds `camera`.
  - The trace's QPC time is read after the previous Present returned and after the in-flight wait, before this frame's draw and Present.
  - The camera step is applied in the same loop iteration, before the draw.
  - In `w3`, `turnAt` (`src/probe.cpp`) sets the turn index and phase from the QPC clock, with the direction alternating by turn parity. In `w3f` they come from the frame counter.
- The probe's `run.json` (`work/experiments/renderer-sb/probe/src/probe.cpp` `runJson` and `src/gpu.cpp` `runGpu` and `environment`):
  - `turn_ms`, and `presentmon`. The probe writes `"swap_chain": "FILL-FROM-CSV"`, and the capture script fills it in.
  - `environment`, with `presentation_interval`, `vsync`, `tearing`, `warp`, `msaa`, `presenting_adapter`, `power_source`, `power_mode`, `display`, `backbuffer` and `declared`.
  - `window`, with `visible_throughout` and `foreground_throughout`.
  - H6-P adds `feature`, `camera` and, in `w3f`, `turn_frames` and `cycle_frames`.
- Gate: `docs/wiki/decisions/renderer-candidates.md`.
- W2/W3 definitions: `docs/progress/1.0/renderer-experiment-plan.md`, lines 23 to 33.
- H-06:
  - the requirement: `docs/progress/1.0/packets/renderer/E-2.4-01-sb-probe.md`, acceptance item 5;
  - how it is used: `docs/progress/1.0/packets/renderer/E-2.4-05-selection.md`, line 15.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | one group per candidate and scene suffices | current `renderer_gate.py` | W3 gate on build `2b5bf5e6...` | correct for baseline-only inputs; feature runs would be pooled |
| 2 | gate summaries plus matching builds suffice for costs | first version of this packet | Astra plan check | three majors: the gaps verdict, missing controls, and the camera-pose confound |
| 3 | the gaps verdict from `none`, equal controls, and camera bins fix them | second version | Astra scoped re-check | H06-01 fixed; unknown controls compared equal; turn phase uncontrolled |
| 4 | known controls, joint state bins and a pairing check fix them | third version, not submitted | CPU simulation | biased for frame times varying inside a phase bin, and not flagged |
| 5 | paired runs over a frame-locked state cycle make costs exact | fourth version | CPU simulation; Astra second scoped re-check | within 0.003 ms of the truth in every case and lag; H06-03 fixed; the unavailable-driver text still counted as known |
| 6 | the unavailable-driver text is an unknown control | this version | unit test in item 4 | to be shown by the acceptance check |

## 6. Constraints and owned files
- Owned files:
  - `tools/perf/renderer_gate.py`, `tools/perf/b412_summary.py` and `tools/perf/feature_costs.py`;
  - `tests/test_renderer_gate.py`, `tests/test_b412_summary.py`, `tests/test_b412_fixture.py` and `tests/test_feature_costs.py`.
- Do not change any of the following beyond what section 1 states:
  - the gate's thresholds, interval, run rules or sanitising;
  - its PresentMon-timing behaviour, beyond the `feature` key and the `w3f` scene;
  - the B4-12 behaviour, beyond the public keys above.
- Never publish paths, user names, control values outside `controls`, or raw diagnostics. Outputs go through `sanitize`.
- Standard library only. No network.

## 7. Required return format
- The changed files.
- `report.md` with:
  - the grouping change, the `w3f` scene and the trace timing, and their effect on existing outputs;
  - the state check, the window, the controls and the status rules;
  - an example table from a test fixture;
  - each simulation's truth, cost and whole cycles, per timing and lag;
  - the acceptance check output.

```implement-contract
{"allowed_files": ["tools/perf/renderer_gate.py", "tools/perf/b412_summary.py", "tools/perf/feature_costs.py", "tests/test_renderer_gate.py", "tests/test_b412_summary.py", "tests/test_b412_fixture.py", "tests/test_feature_costs.py"], "acceptance_check": ["python", "-m", "unittest", "-q", "tests/test_renderer_gate.py", "tests/test_b412_summary.py", "tests/test_b412_fixture.py", "tests/test_renderer_tools.py", "tests/test_feature_costs.py"], "stop_condition": "the acceptance check passes; renderer_gate.py groups by candidate, scene and feature, accepts w3f as an attribution scene and offers trace timing to callers only, with otherwise unchanged rules; b412_summary.py publishes the new keys; feature_costs.py builds the H-06 table from w3 and w3f run directories in both timings with the state-check, window, known-control, comparability and status rules above"}
```
