# Scoped verification packet: S-B probe findings SB1-R1 and SB1-R5 (round 2 of 2)

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: check only whether the two blocking findings of verification round 1 (`20261003T030940Z-7fcac351`) are fixed, and whether the fixes introduce a new `blocker` or `major`. SB1-R2, SB1-R3 and SB1-R4 were confirmed fixed in round 1; do not re-review them unless these fixes break them.
- Findings under verification:
  - SB1-R1 (major, remaining part):
    - the `Dropped` check covered the whole capture, so displayed preroll frames could admit a measured interval in which every present was dropped;
    - an opaque topmost window covering the probe was not detected, because `IsWindowVisible` stays true;
    - foreground loss did not affect the exit code.
  - SB1-R5 (major): the power-mode callback was registered before throwing initialisation steps. A failed constructor never ran `~Gpu`, so the callback stayed registered with a destroyed context.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`, one finding per unresolved item (same ID) plus any new `blocker`/`major`, each with a concrete counterexample. Verdict `pass` if both are fixed and nothing new blocks.

## 2. Actual problem and reproduction
- Fixes, uncommitted in the working tree on top of `claude/renderer-sb` HEAD:
  - SB1-R1, probe (`probe/src/gpu.cpp`):
    - New `covered()` hit-tests the window centre and four inner points (10% inset) with `WindowFromPoint`, and requires `GetAncestor(hit, GA_ROOT)` to be the probe window.
    - A covered sample counts as not visible.
    - The probe exits 3 when any 100 ms trace sample was not visible (hidden, minimised, cloaked or covered) or not in the foreground.
    - `run.json` `window` adds `samples_covered` and `foreground_throughout`.
  - SB1-R1, capture script (`probe/run_scene.ps1`):
    - The script requires the `Dropped` and `QPCTime` columns.
    - It computes the gate interval [T0+10 s, min(T0+190 s, trace stop)) from `run.json` `markers` and `qpc_frequency`, as `renderer_gate.py` `interval_ticks` does.
    - It refuses the run if any one-second bin of that interval has no present with `Dropped` = 0.
    - Rows outside the interval are ignored for this check.
  - SB1-R5: registration moved to the last statement of the `Gpu` constructor, after every throwing step. The power-mode name, sampling and environment are unchanged from round 1.
- Reproduce: `python work/experiments/renderer-sb/probe/check_probe.py`; `git diff` shows all fixes since HEAD.

## 3. Environment and versions
- As in `work/experiments/renderer-sb-packets/review-1-probe.md` section 3. Evidence kind in the review sandbox: source/fixture only.

## 4. Necessary source and evidence
- Round 1 result: `work/reviews/20261003T030940Z-7fcac351/review.json` (if readable), otherwise section 1.
- Changed files: `work/experiments/renderer-sb/probe/src/gpu.cpp`, `src/probe.cpp`, `CMakeLists.txt`, `run_scene.ps1`, `README.md`.
- Gate reader for the interval definition: `tools/perf/renderer_gate.py` (`interval_ticks`, `read_frames`, `CAPTURE_COLUMNS`).
- Integrator's local results on the owner's machine. These are actual Windows/DirectX functional results, not performance evidence; the probe was launched without PresentMon.
  - Build ok; `check_probe.py` self-test ok.

  | Run (W3) | Exit | Samples | Not visible | Covered | Not foreground | Label check |
  |---|---|---|---|---|---|---|
  | 8 s, normal | 0 | 80 | 0 | 0 | 0 | pass |
  | 10 s, minimised after 7 s | 3 | 100 | 53 | — | 53 | — |
  | 10 s, opaque topmost borderless WinForms form over the primary screen for 2 s from 6 s | 3 | 100 | 20 | 20 | 0 | — |
  | 8 s, each of four fault injections | 2 | — | — | — | — | fail |

  - Measured conditions of the normal run:
    - power `mains`, power mode `max_performance`;
    - `NVIDIA GeForce RTX 4070 Laptop GPU, driver 32.0.16.1692, drives the window's display`.
  - The displayed-present block of `run_scene.ps1`, extracted verbatim and run on synthetic CSVs (2 s preroll plus 192 s trace, one present per 10 ms):

  | Case | Result |
  |---|---|
  | all displayed | accepted |
  | every interval row dropped, preroll displayed (round 1 counterexample) | refused, 180 of 180 seconds blind |
  | one second dropped | refused, 1 of 180 |
  | one present in ten displayed | accepted |

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | per-sample visibility plus capture-wide `Dropped` check closes SB1-R1 | round 1 fixes | verification round 1 | partly: capture-wide check and coverage gap remained; SB1-R5 found |
| 2 | interval bins, hit-test coverage, foreground requirement, late registration | section 2 | section 4 runs and synthetic cases | as listed |

## 6. Constraints and owned files
- Read-only review. Out of scope:
  - `tools/perf/renderer_gate.py` and `b412_summary.py`;
  - the handoff test;
  - performance numbers;
  - style;
  - minor findings.
- Questions:
  1. Can a run whose window was hidden, minimised, cloaked, covered or not in the foreground for a material part of the gate interval still exit 0 and pass `run_scene.ps1`?
  2. Can a capture with a second of the gate interval without a displayed present still be admitted? Does the script's interval match the gate's?
  3. Can the power callback still run against a destroyed `Gpu`?
  4. Do the new checks add a hazard? Consider:
     - false refusals of a normal full-screen run (for example a system window that hit-tests above a topmost window);
     - unbounded cost in the timed loop;
     - PowerShell numeric or parsing errors on real QPC values (about 10^11 to 10^13 ticks at 10 MHz).

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
